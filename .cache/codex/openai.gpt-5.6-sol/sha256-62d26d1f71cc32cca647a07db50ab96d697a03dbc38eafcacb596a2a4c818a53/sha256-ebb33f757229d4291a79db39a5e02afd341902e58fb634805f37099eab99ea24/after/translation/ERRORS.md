# Error surface

Mechanically extracted from every compiled C source file. Rows include explicit error macros/returns/assertions and null/range conditions; file and line preserve the exact source location.

| # | function | trigger (exact C condition/statement) | expected C result | tested |
|---:|---|---|---|:---:|
| 1 | `FSE_readNCount_body` (src/common/entropy_common.c:57) | `if (hbSize < 8) {` | branch-specific rejection/error | [ ] |
| 2 | `FSE_readNCount_body` (src/common/entropy_common.c:64) | `if (countSize > hbSize) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3 | `FSE_readNCount_body` (src/common/entropy_common.c:67) | `assert(hbSize >= 8);` | assertion failure | [ ] |
| 4 | `FSE_readNCount_body` (src/common/entropy_common.c:73) | `if (nbBits > FSE_TABLELOG_ABSOLUTE_MAX) return ERROR(tableLog_tooLarge);` | `ERROR(tableLog_tooLarge)` | [ ] |
| 5 | `FSE_readNCount_body` (src/common/entropy_common.c:91) | `if (LIKELY(ip <= iend-7)) {` | branch-specific rejection/error | [ ] |
| 6 | `FSE_readNCount_body` (src/common/entropy_common.c:106) | `assert((bitStream & 3) < 3);` | assertion failure | [ ] |
| 7 | `FSE_readNCount_body` (src/common/entropy_common.c:114) | `if (charnum >= maxSV1) break;` | branch-specific rejection/error | [ ] |
| 8 | `FSE_readNCount_body` (src/common/entropy_common.c:120) | `if (LIKELY(ip <= iend-7) \|\| (ip + (bitCount>>3) <= iend-4)) {` | branch-specific rejection/error | [ ] |
| 9 | `FSE_readNCount_body` (src/common/entropy_common.c:121) | `assert((bitCount >> 3) <= 3); /* For first condition to work */` | assertion failure | [ ] |
| 10 | `FSE_readNCount_body` (src/common/entropy_common.c:135) | `if ((bitStream & (threshold-1)) < (U32)max) {` | branch-specific rejection/error | [ ] |
| 11 | `FSE_readNCount_body` (src/common/entropy_common.c:140) | `if (count >= threshold) count -= max;` | branch-specific rejection/error | [ ] |
| 12 | `FSE_readNCount_body` (src/common/entropy_common.c:148) | `if (count >= 0) {` | branch-specific rejection/error | [ ] |
| 13 | `FSE_readNCount_body` (src/common/entropy_common.c:151) | `assert(count == -1);` | assertion failure | [ ] |
| 14 | `FSE_readNCount_body` (src/common/entropy_common.c:157) | `assert(threshold > 1);` | assertion failure | [ ] |
| 15 | `FSE_readNCount_body` (src/common/entropy_common.c:158) | `if (remaining < threshold) {` | branch-specific rejection/error | [ ] |
| 16 | `FSE_readNCount_body` (src/common/entropy_common.c:163) | `if (remaining <= 1) break;` | branch-specific rejection/error | [ ] |
| 17 | `FSE_readNCount_body` (src/common/entropy_common.c:167) | `if (charnum >= maxSV1) break;` | branch-specific rejection/error | [ ] |
| 18 | `FSE_readNCount_body` (src/common/entropy_common.c:169) | `if (LIKELY(ip <= iend-7) \|\| (ip + (bitCount>>3) <= iend-4)) {` | branch-specific rejection/error | [ ] |
| 19 | `FSE_readNCount_body` (src/common/entropy_common.c:179) | `if (remaining != 1) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 20 | `FSE_readNCount_body` (src/common/entropy_common.c:181) | `if (charnum > maxSV1) return ERROR(maxSymbolValue_tooSmall);` | `ERROR(maxSymbolValue_tooSmall)` | [ ] |
| 21 | `FSE_readNCount_body` (src/common/entropy_common.c:182) | `if (bitCount > 32) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 22 | `HUF_readStats_body` (src/common/entropy_common.c:254) | `if (!srcSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 23 | `HUF_readStats_body` (src/common/entropy_common.c:258) | `if (iSize >= 128) {  /* special header */` | `ERROR(srcSize_wrong)` | [ ] |
| 24 | `HUF_readStats_body` (src/common/entropy_common.c:261) | `if (iSize+1 > srcSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 25 | `HUF_readStats_body` (src/common/entropy_common.c:262) | `if (oSize >= hwSize) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 26 | `HUF_readStats_body` (src/common/entropy_common.c:270) | `if (iSize+1 > srcSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 27 | `HUF_readStats_body` (src/common/entropy_common.c:280) | `if (huffWeight[n] > HUF_TABLELOG_MAX) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 28 | `HUF_readStats_body` (src/common/entropy_common.c:284) | `if (weightTotal == 0) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 29 | `HUF_readStats_body` (src/common/entropy_common.c:288) | `if (tableLog > HUF_TABLELOG_MAX) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 30 | `HUF_readStats_body` (src/common/entropy_common.c:295) | `if (verif != rest) return ERROR(corruption_detected);    /* last value must be a clean power of 2 */` | `ERROR(corruption_detected)` | [ ] |
| 31 | `HUF_readStats_body` (src/common/entropy_common.c:301) | `if ((rankStats[1] < 2) \|\| (rankStats[1] & 1)) return ERROR(corruption_detected);   /* by construction : at least 2 elts of rank 1, must be even */` | `ERROR(corruption_detected)` | [ ] |
| 32 | `FSE_buildDTable_internal` (src/common/fse_decompress.c:70) | `if (FSE_BUILD_DTABLE_WKSP_SIZE(tableLog, maxSymbolValue) > wkspSize) return ERROR(maxSymbolValue_tooLarge);` | `ERROR(maxSymbolValue_tooLarge)` | [ ] |
| 33 | `FSE_buildDTable_internal` (src/common/fse_decompress.c:71) | `if (maxSymbolValue > FSE_MAX_SYMBOL_VALUE) return ERROR(maxSymbolValue_tooLarge);` | `ERROR(maxSymbolValue_tooLarge)` | [ ] |
| 34 | `FSE_buildDTable_internal` (src/common/fse_decompress.c:72) | `if (tableLog > FSE_MAX_TABLELOG) return ERROR(tableLog_tooLarge);` | `ERROR(tableLog_tooLarge)` | [ ] |
| 35 | `FSE_buildDTable_internal` (src/common/fse_decompress.c:85) | `if (normalizedCounter[s] >= largeLimit) DTableH.fastMode=0;` | branch-specific rejection/error | [ ] |
| 36 | `FSE_buildDTable_internal` (src/common/fse_decompress.c:124) | `assert(tableSize % unroll == 0); /* FSE_MIN_TABLELOG is 5 */` | assertion failure | [ ] |
| 37 | `FSE_buildDTable_internal` (src/common/fse_decompress.c:133) | `assert(position == 0);` | assertion failure | [ ] |
| 38 | `FSE_buildDTable_internal` (src/common/fse_decompress.c:146) | `if (position!=0) return ERROR(GENERIC);   /* position must reach all cells once, otherwise normalizedCounter is incorrect */` | `ERROR(GENERIC)` | [ ] |
| 39 | `FSE_decompress_usingDTable_generic` (src/common/fse_decompress.c:193) | `RETURN_ERROR_IF(BIT_reloadDStream(&bitD)==BIT_DStream_overflow, corruption_detected, "");` | `ERROR(D)` | [ ] |
| 40 | `FSE_decompress_usingDTable_generic` (src/common/fse_decompress.c:201) | `if (FSE_MAX_TABLELOG*2+7 > sizeof(bitD.bitContainer)*8)    /* This test must be static */` | branch-specific rejection/error | [ ] |
| 41 | `FSE_decompress_usingDTable_generic` (src/common/fse_decompress.c:206) | `if (FSE_MAX_TABLELOG*4+7 > sizeof(bitD.bitContainer)*8)    /* This test must be static */` | branch-specific rejection/error | [ ] |
| 42 | `FSE_decompress_usingDTable_generic` (src/common/fse_decompress.c:207) | `{ if (BIT_reloadDStream(&bitD) > BIT_DStream_unfinished) { op+=2; break; } }` | branch-specific rejection/error | [ ] |
| 43 | `FSE_decompress_usingDTable_generic` (src/common/fse_decompress.c:211) | `if (FSE_MAX_TABLELOG*2+7 > sizeof(bitD.bitContainer)*8)    /* This test must be static */` | branch-specific rejection/error | [ ] |
| 44 | `FSE_decompress_usingDTable_generic` (src/common/fse_decompress.c:220) | `if (op>(omax-2)) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [ ] |
| 45 | `FSE_decompress_usingDTable_generic` (src/common/fse_decompress.c:227) | `if (op>(omax-2)) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [ ] |
| 46 | `FSE_decompress_usingDTable_generic` (src/common/fse_decompress.c:234) | `assert(op >= ostart);` | assertion failure | [ ] |
| 47 | `FSE_decompress_wksp_body` (src/common/fse_decompress.c:258) | `if (wkspSize < sizeof(*wksp)) return ERROR(GENERIC);` | `ERROR(GENERIC)` | [ ] |
| 48 | `FSE_decompress_wksp_body` (src/common/fse_decompress.c:267) | `if (tableLog > maxLog) return ERROR(tableLog_tooLarge);` | `ERROR(tableLog_tooLarge)` | [ ] |
| 49 | `FSE_decompress_wksp_body` (src/common/fse_decompress.c:268) | `assert(NCountLength <= cSrcSize);` | assertion failure | [ ] |
| 50 | `FSE_decompress_wksp_body` (src/common/fse_decompress.c:273) | `if (FSE_DECOMPRESS_WKSP_SIZE(tableLog, maxSymbolValue) > wkspSize) return ERROR(tableLog_tooLarge);` | `ERROR(tableLog_tooLarge)` | [ ] |
| 51 | `FSE_decompress_wksp_body` (src/common/fse_decompress.c:274) | `assert(sizeof(*wksp) + FSE_DTABLE_SIZE(tableLog) <= wkspSize);` | assertion failure | [ ] |
| 52 | `POOL_thread` (src/common/pool.c:69) | `if (!ctx) { return NULL; }` | `NULL` | [ ] |
| 53 | `POOL_thread` (src/common/pool.c:76) | `if (ctx->shutdown) {` | branch-specific rejection/error | [ ] |
| 54 | `POOL_thread` (src/common/pool.c:103) | `assert(0);  /* Unreachable */` | assertion failure | [ ] |
| 55 | `POOL_create_advanced` (src/common/pool.c:120) | `if (!numThreads) { return NULL; }` | `NULL` | [ ] |
| 56 | `POOL_create_advanced` (src/common/pool.c:123) | `if (!ctx) { return NULL; }` | `NULL` | [ ] |
| 57 | `POOL_create_advanced` (src/common/pool.c:139) | `if (error) { POOL_free(ctx); return NULL; }` | `NULL` | [ ] |
| 58 | `POOL_create_advanced` (src/common/pool.c:147) | `if (!ctx->threads \|\| !ctx->queue) { POOL_free(ctx); return NULL; }` | `NULL` | [ ] |
| 59 | `POOL_create_advanced` (src/common/pool.c:151) | `if (ZSTD_pthread_create(&ctx->threads[i], NULL, &POOL_thread, ctx)) {` | `NULL` | [ ] |
| 60 | `POOL_create_advanced` (src/common/pool.c:154) | `return NULL;` | `NULL` | [ ] |
| 61 | `POOL_sizeof` (src/common/pool.c:207) | `if (ctx==NULL) return 0;  /* supports sizeof NULL */` | branch-specific rejection/error | [ ] |
| 62 | `POOL_resize_internal` (src/common/pool.c:217) | `if (numThreads <= ctx->threadCapacity) {` | branch-specific rejection/error | [ ] |
| 63 | `POOL_resize_internal` (src/common/pool.c:232) | `if (ZSTD_pthread_create(&threadPool[threadId], NULL, &POOL_thread, ctx)) {` | branch-specific rejection/error | [ ] |
| 64 | `POOL_resize` (src/common/pool.c:247) | `if (ctx==NULL) return 1;` | branch-specific rejection/error | [ ] |
| 65 | `isQueueFull` (src/common/pool.c:262) | `if (ctx->queueSize > 1) {` | branch-specific rejection/error | [ ] |
| 66 | `POOL_add_internal` (src/common/pool.c:277) | `assert(ctx != NULL);` | assertion failure | [ ] |
| 67 | `POOL_add_internal` (src/common/pool.c:278) | `if (ctx->shutdown) return;` | branch-specific rejection/error | [ ] |
| 68 | `POOL_add` (src/common/pool.c:288) | `assert(ctx != NULL);` | assertion failure | [ ] |
| 69 | `POOL_tryAdd` (src/common/pool.c:301) | `assert(ctx != NULL);` | assertion failure | [ ] |
| 70 | `POOL_free` (src/common/pool.c:340) | `assert(!ctx \|\| ctx == &g_poolCtx);` | assertion failure | [ ] |
| 71 | `POOL_joinJobs` (src/common/pool.c:345) | `assert(!ctx \|\| ctx == &g_poolCtx);` | assertion failure | [ ] |
| 72 | `POOL_sizeof` (src/common/pool.c:366) | `if (ctx==NULL) return 0;  /* supports sizeof NULL */` | branch-specific rejection/error | [ ] |
| 73 | `POOL_sizeof` (src/common/pool.c:367) | `assert(ctx == &g_poolCtx);` | assertion failure | [ ] |
| 74 | `ZSTD_pthread_create` (src/common/threading.c:76) | `if (thread==NULL) return -1;` | `-1` | [ ] |
| 75 | `ZSTD_pthread_create` (src/common/threading.c:84) | `if(ZSTD_pthread_cond_init(&thread_param.initialized_cond, NULL)) {` | `-1` | [ ] |
| 76 | `ZSTD_pthread_create` (src/common/threading.c:86) | `return -1;` | `-1` | [ ] |
| 77 | `ZSTD_pthread_create` (src/common/threading.c:88) | `if(ZSTD_pthread_mutex_init(&thread_param.initialized_mutex, NULL)) {` | `-1` | [ ] |
| 78 | `ZSTD_pthread_create` (src/common/threading.c:91) | `return -1;` | `-1` | [ ] |
| 79 | `ZSTD_pthread_create` (src/common/threading.c:96) | `if (*thread==NULL) {` | branch-specific rejection/error | [ ] |
| 80 | `ZSTD_pthread_mutex_init` (src/common/threading.c:142) | `assert(mutex != NULL);` | assertion failure | [ ] |
| 81 | `ZSTD_pthread_mutex_destroy` (src/common/threading.c:151) | `assert(mutex != NULL);` | assertion failure | [ ] |
| 82 | `ZSTD_pthread_cond_init` (src/common/threading.c:163) | `assert(cond != NULL);` | assertion failure | [ ] |
| 83 | `ZSTD_pthread_cond_destroy` (src/common/threading.c:172) | `assert(cond != NULL);` | assertion failure | [ ] |
| 84 | `FSE_buildCTable_wksp` (src/compress/fse_compress.c:86) | `assert(((size_t)workSpace & 1) == 0);  /* Must be 2 bytes-aligned */` | `ERROR(tableLog_tooLarge)` | [ ] |
| 85 | `FSE_buildCTable_wksp` (src/compress/fse_compress.c:87) | `if (FSE_BUILD_CTABLE_WORKSPACE_SIZE(maxSymbolValue, tableLog) > wkspSize) return ERROR(tableLog_tooLarge);` | `ERROR(tableLog_tooLarge)` | [ ] |
| 86 | `FSE_buildCTable_wksp` (src/compress/fse_compress.c:91) | `assert(tableLog < 16);   /* required for threshold strategy to work */` | assertion failure | [ ] |
| 87 | `FSE_buildCTable_wksp` (src/compress/fse_compress.c:108) | `assert(normalizedCounter[u-1] >= 0);` | assertion failure | [ ] |
| 88 | `FSE_buildCTable_wksp` (src/compress/fse_compress.c:110) | `assert(cumul[u] >= cumul[u-1]);  /* no overflow */` | assertion failure | [ ] |
| 89 | `FSE_buildCTable_wksp` (src/compress/fse_compress.c:132) | `assert(n>=0);` | assertion failure | [ ] |
| 90 | `FSE_buildCTable_wksp` (src/compress/fse_compress.c:143) | `assert(tableSize % unroll == 0); /* FSE_MIN_TABLELOG is 5 */` | assertion failure | [ ] |
| 91 | `FSE_buildCTable_wksp` (src/compress/fse_compress.c:152) | `assert(position == 0);   /* Must have initialized all positions */` | assertion failure | [ ] |
| 92 | `FSE_buildCTable_wksp` (src/compress/fse_compress.c:166) | `assert(position==0);  /* Must have initialized all positions */` | assertion failure | [ ] |
| 93 | `FSE_buildCTable_wksp` (src/compress/fse_compress.c:189) | `assert(total <= INT_MAX);` | assertion failure | [ ] |
| 94 | `FSE_buildCTable_wksp` (src/compress/fse_compress.c:194) | `assert(normalizedCounter[s] > 1);` | assertion failure | [ ] |
| 95 | `FSE_writeNCount_generic` (src/compress/fse_compress.c:268) | `if ((!writeIsSafe) && (out > oend-2))` | `ERROR(dstSize_tooSmall)` | [ ] |
| 96 | `FSE_writeNCount_generic` (src/compress/fse_compress.c:269) | `return ERROR(dstSize_tooSmall);   /* Buffer overflow */` | `ERROR(dstSize_tooSmall)` | [ ] |
| 97 | `FSE_writeNCount_generic` (src/compress/fse_compress.c:282) | `if (bitCount>16) {` | `ERROR(dstSize_tooSmall)` | [ ] |
| 98 | `FSE_writeNCount_generic` (src/compress/fse_compress.c:283) | `if ((!writeIsSafe) && (out > oend - 2))` | `ERROR(dstSize_tooSmall)` | [ ] |
| 99 | `FSE_writeNCount_generic` (src/compress/fse_compress.c:284) | `return ERROR(dstSize_tooSmall);   /* Buffer overflow */` | `ERROR(dstSize_tooSmall)` | [ ] |
| 100 | `FSE_writeNCount_generic` (src/compress/fse_compress.c:295) | `if (count>=threshold)` | branch-specific rejection/error | [ ] |
| 101 | `FSE_writeNCount_generic` (src/compress/fse_compress.c:301) | `if (remaining<1) return ERROR(GENERIC);` | `ERROR(GENERIC)` | [ ] |
| 102 | `FSE_writeNCount_generic` (src/compress/fse_compress.c:304) | `if (bitCount>16) {` | `ERROR(dstSize_tooSmall)` | [ ] |
| 103 | `FSE_writeNCount_generic` (src/compress/fse_compress.c:305) | `if ((!writeIsSafe) && (out > oend - 2))` | `ERROR(dstSize_tooSmall)` | [ ] |
| 104 | `FSE_writeNCount_generic` (src/compress/fse_compress.c:306) | `return ERROR(dstSize_tooSmall);   /* Buffer overflow */` | `ERROR(dstSize_tooSmall)` | [ ] |
| 105 | `FSE_writeNCount_generic` (src/compress/fse_compress.c:315) | `return ERROR(GENERIC);  /* incorrect normalized distribution */` | `ERROR(GENERIC)` | [ ] |
| 106 | `FSE_writeNCount_generic` (src/compress/fse_compress.c:316) | `assert(symbol <= alphabetSize);` | `ERROR(dstSize_tooSmall)` | [ ] |
| 107 | `FSE_writeNCount_generic` (src/compress/fse_compress.c:319) | `if ((!writeIsSafe) && (out > oend - 2))` | `ERROR(dstSize_tooSmall)` | [ ] |
| 108 | `FSE_writeNCount_generic` (src/compress/fse_compress.c:320) | `return ERROR(dstSize_tooSmall);   /* Buffer overflow */` | `ERROR(dstSize_tooSmall)` | [ ] |
| 109 | `FSE_writeNCount_generic` (src/compress/fse_compress.c:325) | `assert(out >= ostart);` | assertion failure | [ ] |
| 110 | `FSE_writeNCount` (src/compress/fse_compress.c:333) | `if (tableLog > FSE_MAX_TABLELOG) return ERROR(tableLog_tooLarge);   /* Unsupported */` | `ERROR(tableLog_tooLarge)` | [ ] |
| 111 | `FSE_writeNCount` (src/compress/fse_compress.c:334) | `if (tableLog < FSE_MIN_TABLELOG) return ERROR(GENERIC);   /* Unsupported */` | `ERROR(GENERIC)` | [ ] |
| 112 | `FSE_writeNCount` (src/compress/fse_compress.c:336) | `if (bufferSize < FSE_NCountWriteBound(maxSymbolValue, tableLog))` | branch-specific rejection/error | [ ] |
| 113 | `FSE_minTableLog` (src/compress/fse_compress.c:353) | `assert(srcSize > 1); /* Not supported, RLE should be used instead */` | assertion failure | [ ] |
| 114 | `FSE_optimalTableLog_internal` (src/compress/fse_compress.c:362) | `assert(srcSize > 1); /* Not supported, RLE should be used instead */` | assertion failure | [ ] |
| 115 | `FSE_optimalTableLog_internal` (src/compress/fse_compress.c:363) | `if (tableLog==0) tableLog = FSE_DEFAULT_TABLELOG;` | branch-specific rejection/error | [ ] |
| 116 | `FSE_optimalTableLog_internal` (src/compress/fse_compress.c:364) | `if (maxBitsSrc < tableLog) tableLog = maxBitsSrc;   /* Accuracy can be reduced */` | branch-specific rejection/error | [ ] |
| 117 | `FSE_optimalTableLog_internal` (src/compress/fse_compress.c:365) | `if (minBits > tableLog) tableLog = minBits;   /* Need a minimum to safely represent all symbol values */` | branch-specific rejection/error | [ ] |
| 118 | `FSE_optimalTableLog_internal` (src/compress/fse_compress.c:366) | `if (tableLog < FSE_MIN_TABLELOG) tableLog = FSE_MIN_TABLELOG;` | branch-specific rejection/error | [ ] |
| 119 | `FSE_optimalTableLog_internal` (src/compress/fse_compress.c:367) | `if (tableLog > FSE_MAX_TABLELOG) tableLog = FSE_MAX_TABLELOG;` | branch-specific rejection/error | [ ] |
| 120 | `FSE_normalizeM2` (src/compress/fse_compress.c:391) | `if (count[s] == 0) {` | branch-specific rejection/error | [ ] |
| 121 | `FSE_normalizeM2` (src/compress/fse_compress.c:395) | `if (count[s] <= lowThreshold) {` | branch-specific rejection/error | [ ] |
| 122 | `FSE_normalizeM2` (src/compress/fse_compress.c:401) | `if (count[s] <= lowOne) {` | branch-specific rejection/error | [ ] |
| 123 | `FSE_normalizeM2` (src/compress/fse_compress.c:412) | `if (ToDistribute == 0)` | branch-specific rejection/error | [ ] |
| 124 | `FSE_normalizeM2` (src/compress/fse_compress.c:415) | `if ((total / ToDistribute) > lowOne) {` | branch-specific rejection/error | [ ] |
| 125 | `FSE_normalizeM2` (src/compress/fse_compress.c:419) | `if ((norm[s] == NOT_YET_ASSIGNED) && (count[s] <= lowOne)) {` | branch-specific rejection/error | [ ] |
| 126 | `FSE_normalizeM2` (src/compress/fse_compress.c:434) | `if (count[s] > maxC) { maxV=s; maxC=count[s]; }` | branch-specific rejection/error | [ ] |
| 127 | `FSE_normalizeM2` (src/compress/fse_compress.c:439) | `if (total == 0) {` | branch-specific rejection/error | [ ] |
| 128 | `FSE_normalizeM2` (src/compress/fse_compress.c:442) | `if (norm[s] > 0) { ToDistribute--; norm[s]++; }` | branch-specific rejection/error | [ ] |
| 129 | `FSE_normalizeM2` (src/compress/fse_compress.c:456) | `if (weight < 1)` | `ERROR(GENERIC)` | [ ] |
| 130 | `FSE_normalizeM2` (src/compress/fse_compress.c:457) | `return ERROR(GENERIC);` | `ERROR(GENERIC)` | [ ] |
| 131 | `FSE_normalizeCount` (src/compress/fse_compress.c:470) | `if (tableLog==0) tableLog = FSE_DEFAULT_TABLELOG;` | `ERROR(GENERIC)` | [ ] |
| 132 | `FSE_normalizeCount` (src/compress/fse_compress.c:471) | `if (tableLog < FSE_MIN_TABLELOG) return ERROR(GENERIC);   /* Unsupported size */` | `ERROR(GENERIC)` | [ ] |
| 133 | `FSE_normalizeCount` (src/compress/fse_compress.c:472) | `if (tableLog > FSE_MAX_TABLELOG) return ERROR(tableLog_tooLarge);   /* Unsupported size */` | `ERROR(tableLog_tooLarge)` | [ ] |
| 134 | `FSE_normalizeCount` (src/compress/fse_compress.c:473) | `if (tableLog < FSE_minTableLog(total, maxSymbolValue)) return ERROR(GENERIC);   /* Too small tableLog, compression potentially impossible */` | `ERROR(GENERIC)` | [ ] |
| 135 | `FSE_normalizeCount` (src/compress/fse_compress.c:488) | `if (count[s] == 0) { normalizedCounter[s]=0; continue; }` | branch-specific rejection/error | [ ] |
| 136 | `FSE_normalizeCount` (src/compress/fse_compress.c:489) | `if (count[s] <= lowThreshold) {` | branch-specific rejection/error | [ ] |
| 137 | `FSE_normalizeCount` (src/compress/fse_compress.c:494) | `if (proba<8) {` | branch-specific rejection/error | [ ] |
| 138 | `FSE_normalizeCount` (src/compress/fse_compress.c:498) | `if (proba > largestP) { largestP=proba; largest=s; }` | branch-specific rejection/error | [ ] |
| 139 | `FSE_normalizeCount` (src/compress/fse_compress.c:502) | `if (-stillToDistribute >= (normalizedCounter[largest] >> 1)) {` | branch-specific rejection/error | [ ] |
| 140 | `FSE_normalizeCount` (src/compress/fse_compress.c:518) | `if (nTotal != (1U<<tableLog))` | branch-specific rejection/error | [ ] |
| 141 | `FSE_compress_usingCTable_generic` (src/compress/fse_compress.c:563) | `if (srcSize <= 2) return 0;` | branch-specific rejection/error | [ ] |
| 142 | `FSE_compress_usingCTable_generic` (src/compress/fse_compress.c:581) | `if ((sizeof(bitC.bitContainer)*8 > FSE_MAX_TABLELOG*4+7 ) && (srcSize & 2)) {  /* test bit 2 */` | branch-specific rejection/error | [ ] |
| 143 | `FSE_compress_usingCTable_generic` (src/compress/fse_compress.c:592) | `if (sizeof(bitC.bitContainer)*8 < FSE_MAX_TABLELOG*2+7 )   /* this test must be static */` | branch-specific rejection/error | [ ] |
| 144 | `FSE_compress_usingCTable_generic` (src/compress/fse_compress.c:597) | `if (sizeof(bitC.bitContainer)*8 > FSE_MAX_TABLELOG*4+7 ) {  /* this test must be static */` | branch-specific rejection/error | [ ] |
| 145 | `HIST_count_simple` (src/compress/hist.c:48) | `if (srcSize==0) { *maxSymbolValuePtr = 0; return 0; }` | branch-specific rejection/error | [ ] |
| 146 | `HIST_count_simple` (src/compress/hist.c:51) | `assert(*ip <= maxSymbolValue);` | assertion failure | [ ] |
| 147 | `HIST_count_simple` (src/compress/hist.c:60) | `if (count[s] > largestCount) largestCount = count[s];` | branch-specific rejection/error | [ ] |
| 148 | `HIST_count_parallel_wksp` (src/compress/hist.c:92) | `assert(*maxSymbolValuePtr <= 255);` | assertion failure | [ ] |
| 149 | `HIST_count_parallel_wksp` (src/compress/hist.c:133) | `if (Counting1[s] > max) max = Counting1[s];` | branch-specific rejection/error | [ ] |
| 150 | `HIST_count_parallel_wksp` (src/compress/hist.c:138) | `if (check && maxSymbolValue > *maxSymbolValuePtr) return ERROR(maxSymbolValue_tooSmall);` | `ERROR(maxSymbolValue_tooSmall)` | [ ] |
| 151 | `HIST_countFast_wksp` (src/compress/hist.c:154) | `if (sourceSize < 1500) /* heuristic threshold */` | `ERROR(GENERIC)` | [ ] |
| 152 | `HIST_countFast_wksp` (src/compress/hist.c:156) | `if ((size_t)workSpace & 3) return ERROR(GENERIC);  /* must be aligned on 4-bytes boundaries */` | `ERROR(GENERIC)` | [ ] |
| 153 | `HIST_countFast_wksp` (src/compress/hist.c:157) | `if (workSpaceSize < HIST_WKSP_SIZE) return ERROR(workSpace_tooSmall);` | `ERROR(workSpace_tooSmall)` | [ ] |
| 154 | `HIST_count_wksp` (src/compress/hist.c:168) | `if ((size_t)workSpace & 3) return ERROR(GENERIC);  /* must be aligned on 4-bytes boundaries */` | `ERROR(GENERIC)` | [ ] |
| 155 | `HIST_count_wksp` (src/compress/hist.c:169) | `if (workSpaceSize < HIST_WKSP_SIZE) return ERROR(workSpace_tooSmall);` | `ERROR(workSpace_tooSmall)` | [ ] |
| 156 | `HIST_count_wksp` (src/compress/hist.c:170) | `if (*maxSymbolValuePtr < 255)` | branch-specific rejection/error | [ ] |
| 157 | `HUF_alignUpWorkspace` (src/compress/huf_compress.c:118) | `assert((align & (align - 1)) == 0); /* pow 2 */` | assertion failure | [ ] |
| 158 | `HUF_alignUpWorkspace` (src/compress/huf_compress.c:119) | `assert(align <= HUF_WORKSPACE_MAX_ALIGNMENT);` | assertion failure | [ ] |
| 159 | `HUF_alignUpWorkspace` (src/compress/huf_compress.c:120) | `if (*workspaceSizePtr >= add) {` | branch-specific rejection/error | [ ] |
| 160 | `HUF_alignUpWorkspace` (src/compress/huf_compress.c:121) | `assert(add < align);` | assertion failure | [ ] |
| 161 | `HUF_alignUpWorkspace` (src/compress/huf_compress.c:122) | `assert(((size_t)aligned & mask) == 0);` | assertion failure | [ ] |
| 162 | `HUF_alignUpWorkspace` (src/compress/huf_compress.c:127) | `return NULL;` | `NULL` | [ ] |
| 163 | `HUF_compressWeights` (src/compress/huf_compress.c:159) | `if (workspaceSize < sizeof(HUF_CompressWeightsWksp)) return ERROR(GENERIC);` | `ERROR(GENERIC)` | [ ] |
| 164 | `HUF_compressWeights` (src/compress/huf_compress.c:162) | `if (wtSize <= 1) return 0;  /* Not compressible */` | branch-specific rejection/error | [ ] |
| 165 | `HUF_compressWeights` (src/compress/huf_compress.c:181) | `if (cSize == 0) return 0;   /* not enough space for compressed data */` | branch-specific rejection/error | [ ] |
| 166 | `HUF_setNbBits` (src/compress/huf_compress.c:210) | `assert(nbBits <= HUF_TABLELOG_ABSOLUTEMAX);` | assertion failure | [ ] |
| 167 | `HUF_setValue` (src/compress/huf_compress.c:217) | `if (nbBits > 0) {` | branch-specific rejection/error | [ ] |
| 168 | `HUF_setValue` (src/compress/huf_compress.c:218) | `assert((value >> nbBits) == 0);` | assertion failure | [ ] |
| 169 | `HUF_writeCTableHeader` (src/compress/huf_compress.c:235) | `assert(tableLog < 256);` | assertion failure | [ ] |
| 170 | `HUF_writeCTableHeader` (src/compress/huf_compress.c:237) | `assert(maxSymbolValue < 256);` | assertion failure | [ ] |
| 171 | `HUF_writeCTable_wksp` (src/compress/huf_compress.c:259) | `assert(HUF_readCTableHeader(CTable).maxSymbolValue == maxSymbolValue);` | `ERROR(GENERIC)` | [ ] |
| 172 | `HUF_writeCTable_wksp` (src/compress/huf_compress.c:260) | `assert(HUF_readCTableHeader(CTable).tableLog == huffLog);` | `ERROR(GENERIC)` | [ ] |
| 173 | `HUF_writeCTable_wksp` (src/compress/huf_compress.c:263) | `if (workspaceSize < sizeof(HUF_WriteCTableWksp)) return ERROR(GENERIC);` | `ERROR(GENERIC)` | [ ] |
| 174 | `HUF_writeCTable_wksp` (src/compress/huf_compress.c:264) | `if (maxSymbolValue > HUF_SYMBOLVALUE_MAX) return ERROR(maxSymbolValue_tooLarge);` | `ERROR(maxSymbolValue_tooLarge)` | [ ] |
| 175 | `HUF_writeCTable_wksp` (src/compress/huf_compress.c:274) | `if (maxDstSize < 1) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [ ] |
| 176 | `HUF_writeCTable_wksp` (src/compress/huf_compress.c:276) | `if ((hSize>1) & (hSize < maxSymbolValue/2)) {   /* FSE compressed */` | branch-specific rejection/error | [ ] |
| 177 | `HUF_writeCTable_wksp` (src/compress/huf_compress.c:282) | `if (maxSymbolValue > (256-128)) return ERROR(GENERIC);   /* should not happen : likely means source cannot be compressed */` | `ERROR(GENERIC)` | [ ] |
| 178 | `HUF_writeCTable_wksp` (src/compress/huf_compress.c:283) | `if (((maxSymbolValue+1)/2) + 1 > maxDstSize) return ERROR(dstSize_tooSmall);   /* not enough space within dst buffer */` | `ERROR(dstSize_tooSmall)` | [ ] |
| 179 | `HUF_readCTable` (src/compress/huf_compress.c:305) | `if (tableLog > HUF_TABLELOG_MAX) return ERROR(tableLog_tooLarge);` | `ERROR(tableLog_tooLarge)` | [ ] |
| 180 | `HUF_readCTable` (src/compress/huf_compress.c:306) | `if (nbSymbols > *maxSymbolValuePtr+1) return ERROR(maxSymbolValue_tooSmall);` | `ERROR(maxSymbolValue_tooSmall)` | [ ] |
| 181 | `HUF_getNbBitsFromCTable` (src/compress/huf_compress.c:348) | `assert(symbolValue <= HUF_SYMBOLVALUE_MAX);` | assertion failure | [ ] |
| 182 | `HUF_getNbBitsFromCTable` (src/compress/huf_compress.c:349) | `if (symbolValue > HUF_readCTableHeader(CTable).maxSymbolValue)` | branch-specific rejection/error | [ ] |
| 183 | `HUF_setMaxHeight` (src/compress/huf_compress.c:380) | `if (largestBits <= targetNbBits) return largestBits;` | branch-specific rejection/error | [ ] |
| 184 | `HUF_setMaxHeight` (src/compress/huf_compress.c:399) | `assert(huffNode[n].nbBits <= targetNbBits);` | assertion failure | [ ] |
| 185 | `HUF_setMaxHeight` (src/compress/huf_compress.c:405) | `assert(((U32)totalCost & (baseCost - 1)) == 0);` | assertion failure | [ ] |
| 186 | `HUF_setMaxHeight` (src/compress/huf_compress.c:407) | `assert(totalCost > 0);` | assertion failure | [ ] |
| 187 | `HUF_setMaxHeight` (src/compress/huf_compress.c:418) | `if (huffNode[pos].nbBits >= currentNbBits) continue;` | branch-specific rejection/error | [ ] |
| 188 | `HUF_setMaxHeight` (src/compress/huf_compress.c:438) | `if (highTotal <= lowTotal) break;` | branch-specific rejection/error | [ ] |
| 189 | `HUF_setMaxHeight` (src/compress/huf_compress.c:441) | `assert(rankLast[nBitsToDecrease] != noSymbol \|\| nBitsToDecrease == 1);` | assertion failure | [ ] |
| 190 | `HUF_setMaxHeight` (src/compress/huf_compress.c:445) | `assert(rankLast[nBitsToDecrease] != noSymbol);` | assertion failure | [ ] |
| 191 | `HUF_setMaxHeight` (src/compress/huf_compress.c:463) | `if (rankLast[nBitsToDecrease] == 0)    /* special case, reached largest symbol */` | branch-specific rejection/error | [ ] |
| 192 | `HUF_setMaxHeight` (src/compress/huf_compress.c:485) | `assert(n >= 0);` | assertion failure | [ ] |
| 193 | `HUF_isSorted` (src/compress/huf_compress.c:547) | `if (huffNode[i].count > huffNode[i-1].count) {` | branch-specific rejection/error | [ ] |
| 194 | `HUF_quickSortPartition` (src/compress/huf_compress.c:579) | `if (arr[j].count > pivot) {` | branch-specific rejection/error | [ ] |
| 195 | `HUF_simpleQuickSort` (src/compress/huf_compress.c:593) | `if (high - low < kInsertionSortThreshold) {` | branch-specific rejection/error | [ ] |
| 196 | `HUF_simpleQuickSort` (src/compress/huf_compress.c:599) | `if (idx - low < high - idx) {` | branch-specific rejection/error | [ ] |
| 197 | `HUF_sort` (src/compress/huf_compress.c:633) | `assert(lowerRank < RANK_POSITION_TABLE_SIZE - 1);` | assertion failure | [ ] |
| 198 | `HUF_sort` (src/compress/huf_compress.c:637) | `assert(rankPosition[RANK_POSITION_TABLE_SIZE - 1].base == 0);` | assertion failure | [ ] |
| 199 | `HUF_sort` (src/compress/huf_compress.c:649) | `assert(pos < maxSymbolValue1);` | assertion failure | [ ] |
| 200 | `HUF_sort` (src/compress/huf_compress.c:658) | `if (bucketSize > 1) {` | branch-specific rejection/error | [ ] |
| 201 | `HUF_sort` (src/compress/huf_compress.c:659) | `assert(bucketStartIdx < maxSymbolValue1);` | assertion failure | [ ] |
| 202 | `HUF_sort` (src/compress/huf_compress.c:664) | `assert(HUF_isSorted(huffNode, maxSymbolValue1));` | assertion failure | [ ] |
| 203 | `HUF_buildCTable_wksp` (src/compress/huf_compress.c:770) | `if (wkspSize < sizeof(HUF_buildCTable_wksp_tables))` | `ERROR(workSpace_tooSmall)` | [ ] |
| 204 | `HUF_buildCTable_wksp` (src/compress/huf_compress.c:771) | `return ERROR(workSpace_tooSmall);` | `ERROR(workSpace_tooSmall)` | [ ] |
| 205 | `HUF_buildCTable_wksp` (src/compress/huf_compress.c:772) | `if (maxNbBits == 0) maxNbBits = HUF_TABLELOG_DEFAULT;` | `ERROR(maxSymbolValue_tooLarge)` | [ ] |
| 206 | `HUF_buildCTable_wksp` (src/compress/huf_compress.c:773) | `if (maxSymbolValue > HUF_SYMBOLVALUE_MAX)` | `ERROR(maxSymbolValue_tooLarge)` | [ ] |
| 207 | `HUF_buildCTable_wksp` (src/compress/huf_compress.c:774) | `return ERROR(maxSymbolValue_tooLarge);` | `ERROR(maxSymbolValue_tooLarge)` | [ ] |
| 208 | `HUF_buildCTable_wksp` (src/compress/huf_compress.c:786) | `if (maxNbBits > HUF_TABLELOG_MAX) return ERROR(GENERIC);   /* check fit into table */` | `ERROR(GENERIC)` | [ ] |
| 209 | `HUF_validateCTable` (src/compress/huf_compress.c:810) | `assert(header.tableLog <= HUF_TABLELOG_ABSOLUTEMAX);` | assertion failure | [ ] |
| 210 | `HUF_validateCTable` (src/compress/huf_compress.c:812) | `if (header.maxSymbolValue < maxSymbolValue)` | branch-specific rejection/error | [ ] |
| 211 | `HUF_initCStream` (src/compress/huf_compress.c:863) | `if (dstCapacity <= sizeof(bitC->bitContainer[0])) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [ ] |
| 212 | `HUF_addBits` (src/compress/huf_compress.c:879) | `assert(idx <= 1);` | assertion failure | [ ] |
| 213 | `HUF_addBits` (src/compress/huf_compress.c:880) | `assert(HUF_getNbBits(elt) <= HUF_TABLELOG_ABSOLUTEMAX);` | assertion failure | [ ] |
| 214 | `HUF_addBits` (src/compress/huf_compress.c:892) | `assert((bitC->bitPos[idx] & 0xFF) <= HUF_BITS_IN_CONTAINER);` | assertion failure | [ ] |
| 215 | `HUF_addBits` (src/compress/huf_compress.c:903) | `assert(((elt >> dirtyBits) << (dirtyBits + nbBits)) == 0);` | assertion failure | [ ] |
| 216 | `HUF_addBits` (src/compress/huf_compress.c:905) | `assert(!kFast \|\| (bitC->bitPos[idx] & 0xFF) <= HUF_BITS_IN_CONTAINER);` | assertion failure | [ ] |
| 217 | `HUF_mergeIndex1` (src/compress/huf_compress.c:923) | `assert((bitC->bitPos[1] & 0xFF) < HUF_BITS_IN_CONTAINER);` | assertion failure | [ ] |
| 218 | `HUF_mergeIndex1` (src/compress/huf_compress.c:927) | `assert((bitC->bitPos[0] & 0xFF) <= HUF_BITS_IN_CONTAINER);` | assertion failure | [ ] |
| 219 | `HUF_flushBits` (src/compress/huf_compress.c:946) | `assert(nbBits > 0);` | assertion failure | [ ] |
| 220 | `HUF_flushBits` (src/compress/huf_compress.c:947) | `assert(nbBits <= sizeof(bitC->bitContainer[0]) * 8);` | assertion failure | [ ] |
| 221 | `HUF_flushBits` (src/compress/huf_compress.c:948) | `assert(bitC->ptr <= bitC->endPtr);` | assertion failure | [ ] |
| 222 | `HUF_flushBits` (src/compress/huf_compress.c:951) | `assert(!kFast \|\| bitC->ptr <= bitC->endPtr);` | assertion failure | [ ] |
| 223 | `HUF_flushBits` (src/compress/huf_compress.c:952) | `if (!kFast && bitC->ptr > bitC->endPtr) bitC->ptr = bitC->endPtr;` | branch-specific rejection/error | [ ] |
| 224 | `HUF_closeCStream` (src/compress/huf_compress.c:979) | `if (bitC->ptr >= bitC->endPtr) return 0; /* overflow detected */` | branch-specific rejection/error | [ ] |
| 225 | `HUF_compress1X_usingCTable_internal_body_loop` (src/compress/huf_compress.c:999) | `if (rem > 0) {` | branch-specific rejection/error | [ ] |
| 226 | `HUF_compress1X_usingCTable_internal_body_loop` (src/compress/huf_compress.c:1005) | `assert(n % kUnroll == 0);` | assertion failure | [ ] |
| 227 | `HUF_compress1X_usingCTable_internal_body_loop` (src/compress/huf_compress.c:1017) | `assert(n % (2 * kUnroll) == 0);` | assertion failure | [ ] |
| 228 | `HUF_compress1X_usingCTable_internal_body_loop` (src/compress/huf_compress.c:1040) | `assert(n == 0);` | assertion failure | [ ] |
| 229 | `HUF_compress1X_usingCTable_internal_body` (src/compress/huf_compress.c:1068) | `if (dstSize < 8) return 0;   /* not enough space to compress */` | branch-specific rejection/error | [ ] |
| 230 | `HUF_compress1X_usingCTable_internal_body` (src/compress/huf_compress.c:1073) | `if (dstSize < HUF_tightCompressBound(srcSize, (size_t)tableLog) \|\| tableLog > 11)` | branch-specific rejection/error | [ ] |
| 231 | `HUF_compress1X_usingCTable_internal_body` (src/compress/huf_compress.c:1115) | `assert(bitC.ptr <= bitC.endPtr);` | assertion failure | [ ] |
| 232 | `HUF_compress4X_usingCTable_internal` (src/compress/huf_compress.c:1179) | `if (dstSize < 6 + 1 + 1 + 1 + 8) return 0;   /* minimum space to compress successfully */` | branch-specific rejection/error | [ ] |
| 233 | `HUF_compress4X_usingCTable_internal` (src/compress/huf_compress.c:1180) | `if (srcSize < 12) return 0;   /* no saving possible : too small input */` | branch-specific rejection/error | [ ] |
| 234 | `HUF_compress4X_usingCTable_internal` (src/compress/huf_compress.c:1183) | `assert(op <= oend);` | assertion failure | [ ] |
| 235 | `HUF_compress4X_usingCTable_internal` (src/compress/huf_compress.c:1185) | `if (cSize == 0 \|\| cSize > 65535) return 0;` | branch-specific rejection/error | [ ] |
| 236 | `HUF_compress4X_usingCTable_internal` (src/compress/huf_compress.c:1191) | `assert(op <= oend);` | assertion failure | [ ] |
| 237 | `HUF_compress4X_usingCTable_internal` (src/compress/huf_compress.c:1193) | `if (cSize == 0 \|\| cSize > 65535) return 0;` | branch-specific rejection/error | [ ] |
| 238 | `HUF_compress4X_usingCTable_internal` (src/compress/huf_compress.c:1199) | `assert(op <= oend);` | assertion failure | [ ] |
| 239 | `HUF_compress4X_usingCTable_internal` (src/compress/huf_compress.c:1201) | `if (cSize == 0 \|\| cSize > 65535) return 0;` | branch-specific rejection/error | [ ] |
| 240 | `HUF_compress4X_usingCTable_internal` (src/compress/huf_compress.c:1207) | `assert(op <= oend);` | assertion failure | [ ] |
| 241 | `HUF_compress4X_usingCTable_internal` (src/compress/huf_compress.c:1208) | `assert(ip <= iend);` | assertion failure | [ ] |
| 242 | `HUF_compress4X_usingCTable_internal` (src/compress/huf_compress.c:1210) | `if (cSize == 0 \|\| cSize > 65535) return 0;` | branch-specific rejection/error | [ ] |
| 243 | `HUF_compressCTable_internal` (src/compress/huf_compress.c:1233) | `if (cSize==0) { return 0; }   /* uncompressible */` | branch-specific rejection/error | [ ] |
| 244 | `HUF_compressCTable_internal` (src/compress/huf_compress.c:1236) | `assert(op >= ostart);` | assertion failure | [ ] |
| 245 | `HUF_compressCTable_internal` (src/compress/huf_compress.c:1237) | `if ((size_t)(op-ostart) >= srcSize-1) { return 0; }` | branch-specific rejection/error | [ ] |
| 246 | `HUF_cardinality` (src/compress/huf_compress.c:1260) | `if (count[i] != 0) cardinality += 1;` | branch-specific rejection/error | [ ] |
| 247 | `HUF_optimalTableLog` (src/compress/huf_compress.c:1281) | `assert(srcSize > 1); /* Not supported, RLE should be used instead */` | assertion failure | [ ] |
| 248 | `HUF_optimalTableLog` (src/compress/huf_compress.c:1282) | `assert(wkspSize >= sizeof(HUF_buildCTable_wksp_tables));` | assertion failure | [ ] |
| 249 | `HUF_optimalTableLog` (src/compress/huf_compress.c:1306) | `if (maxBits < optLogGuess && optLogGuess > minTableLog) break;` | branch-specific rejection/error | [ ] |
| 250 | `HUF_optimalTableLog` (src/compress/huf_compress.c:1315) | `if (newSize > optSize + 1) {` | branch-specific rejection/error | [ ] |
| 251 | `HUF_optimalTableLog` (src/compress/huf_compress.c:1319) | `if (newSize < optSize) {` | branch-specific rejection/error | [ ] |
| 252 | `HUF_optimalTableLog` (src/compress/huf_compress.c:1324) | `assert(optLog <= HUF_TABLELOG_MAX);` | assertion failure | [ ] |
| 253 | `HUF_compress_internal` (src/compress/huf_compress.c:1349) | `if (wkspSize < sizeof(*table)) return ERROR(workSpace_tooSmall);` | `ERROR(workSpace_tooSmall)` | [ ] |
| 254 | `HUF_compress_internal` (src/compress/huf_compress.c:1352) | `if (srcSize > HUF_BLOCKSIZE_MAX) return ERROR(srcSize_wrong);   /* current block size limit */` | `ERROR(srcSize_wrong)` | [ ] |
| 255 | `HUF_compress_internal` (src/compress/huf_compress.c:1353) | `if (huffLog > HUF_TABLELOG_MAX) return ERROR(tableLog_tooLarge);` | `ERROR(tableLog_tooLarge)` | [ ] |
| 256 | `HUF_compress_internal` (src/compress/huf_compress.c:1354) | `if (maxSymbolValue > HUF_SYMBOLVALUE_MAX) return ERROR(maxSymbolValue_tooLarge);` | `ERROR(maxSymbolValue_tooLarge)` | [ ] |
| 257 | `HUF_compress_internal` (src/compress/huf_compress.c:1367) | `if ((flags & HUF_flags_suspectUncompressible) && srcSize >= (SUSPECT_INCOMPRESSIBLE_SAMPLE_SIZE * SUSPECT_INCOMPRESSIBLE_SAMPLE_RATIO)) {` | branch-specific rejection/error | [ ] |
| 258 | `HUF_compress_internal` (src/compress/huf_compress.c:1378) | `if (largestTotal <= ((2 * SUSPECT_INCOMPRESSIBLE_SAMPLE_SIZE) >> 7)+4) return 0;   /* heuristic : probably not compressible enough */` | branch-specific rejection/error | [ ] |
| 259 | `HUF_compress_internal` (src/compress/huf_compress.c:1384) | `if (largest <= (srcSize >> 7)+4) return 0;   /* heuristic : probably not compressible enough */` | branch-specific rejection/error | [ ] |
| 260 | `HUF_compress_internal` (src/compress/huf_compress.c:1418) | `if (oldSize <= hSize + newSize \|\| hSize + 12 >= srcSize) {` | branch-specific rejection/error | [ ] |
| 261 | `HUF_compress_internal` (src/compress/huf_compress.c:1425) | `if (hSize + 12ul >= srcSize) { return 0; }` | branch-specific rejection/error | [ ] |
| 262 | `ZSTD_compressBound` (src/compress/zstd_compress.c:72) | `if (r==0) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 263 | `ZSTD_initCCtx` (src/compress/zstd_compress.c:104) | `assert(cctx != NULL);` | assertion failure | [ ] |
| 264 | `ZSTD_initCCtx` (src/compress/zstd_compress.c:109) | `assert(!ZSTD_isError(err));` | assertion failure | [ ] |
| 265 | `ZSTD_createCCtx_advanced` (src/compress/zstd_compress.c:118) | `if ((!customMem.customAlloc) ^ (!customMem.customFree)) return NULL;` | `NULL` | [ ] |
| 266 | `ZSTD_createCCtx_advanced` (src/compress/zstd_compress.c:120) | `if (!cctx) return NULL;` | `NULL` | [ ] |
| 267 | `ZSTD_initStaticCCtx` (src/compress/zstd_compress.c:130) | `if (workspaceSize <= sizeof(ZSTD_CCtx)) return NULL;  /* minimum size */` | `NULL` | [ ] |
| 268 | `ZSTD_initStaticCCtx` (src/compress/zstd_compress.c:131) | `if ((size_t)workspace & 7) return NULL;  /* must be 8-aligned */` | `NULL` | [ ] |
| 269 | `ZSTD_initStaticCCtx` (src/compress/zstd_compress.c:135) | `if (cctx == NULL) return NULL;` | `NULL` | [ ] |
| 270 | `ZSTD_initStaticCCtx` (src/compress/zstd_compress.c:142) | `if (!ZSTD_cwksp_check_available(&cctx->workspace, TMP_WORKSPACE_SIZE + 2 * sizeof(ZSTD_compressedBlockState_t))) return NULL;` | `NULL` | [ ] |
| 271 | `ZSTD_freeCCtxContent` (src/compress/zstd_compress.c:172) | `assert(cctx != NULL);` | assertion failure | [ ] |
| 272 | `ZSTD_freeCCtxContent` (src/compress/zstd_compress.c:173) | `assert(cctx->staticSize == 0);` | assertion failure | [ ] |
| 273 | `ZSTD_freeCCtx` (src/compress/zstd_compress.c:184) | `if (cctx==NULL) return 0;   /* support free on NULL */` | `ERROR(memory_allocation)` | [ ] |
| 274 | `ZSTD_freeCCtx` (src/compress/zstd_compress.c:185) | `RETURN_ERROR_IF(cctx->staticSize, memory_allocation,` | `ERROR(memory_allocation)` | [ ] |
| 275 | `ZSTD_freeCCtx` (src/compress/zstd_compress.c:189) | `if (!cctxInWorkspace) ZSTD_customFree(cctx, cctx->customMem);` | branch-specific rejection/error | [ ] |
| 276 | `ZSTD_sizeof_CCtx` (src/compress/zstd_compress.c:208) | `if (cctx==NULL) return 0;   /* support sizeof on NULL */` | branch-specific rejection/error | [ ] |
| 277 | `ZSTD_rowMatchFinderUsed` (src/compress/zstd_compress.c:233) | `assert(mode != ZSTD_ps_auto);` | assertion failure | [ ] |
| 278 | `ZSTD_resolveRowMatchFinderMode` (src/compress/zstd_compress.c:242) | `if (!ZSTD_rowMatchFinderSupported(cParams->strategy)) return mode;` | branch-specific rejection/error | [ ] |
| 279 | `ZSTD_resolveRowMatchFinderMode` (src/compress/zstd_compress.c:243) | `if (cParams->windowLog > 14) mode = ZSTD_ps_enable;` | branch-specific rejection/error | [ ] |
| 280 | `ZSTD_allocateChainTable` (src/compress/zstd_compress.c:258) | `assert(useRowMatchFinder != ZSTD_ps_auto);` | assertion failure | [ ] |
| 281 | `ZSTD_resolveMaxBlockSize` (src/compress/zstd_compress.c:281) | `if (maxBlockSize == 0) {` | branch-specific rejection/error | [ ] |
| 282 | `ZSTD_resolveExternalRepcodeSearch` (src/compress/zstd_compress.c:290) | `if (cLevel < 10) {` | branch-specific rejection/error | [ ] |
| 283 | `ZSTD_makeCCtxParamsFromCParams` (src/compress/zstd_compress.c:315) | `assert(cctxParams.ldmParams.hashLog >= cctxParams.ldmParams.bucketSizeLog);` | assertion failure | [ ] |
| 284 | `ZSTD_makeCCtxParamsFromCParams` (src/compress/zstd_compress.c:316) | `assert(cctxParams.ldmParams.hashRateLog < 32);` | assertion failure | [ ] |
| 285 | `ZSTD_makeCCtxParamsFromCParams` (src/compress/zstd_compress.c:324) | `assert(!ZSTD_checkCParams(cParams));` | assertion failure | [ ] |
| 286 | `ZSTD_createCCtxParams_advanced` (src/compress/zstd_compress.c:332) | `if ((!customMem.customAlloc) ^ (!customMem.customFree)) return NULL;` | `NULL` | [ ] |
| 287 | `ZSTD_createCCtxParams_advanced` (src/compress/zstd_compress.c:335) | `if (!params) { return NULL; }` | `NULL` | [ ] |
| 288 | `ZSTD_freeCCtxParams` (src/compress/zstd_compress.c:348) | `if (params == NULL) { return 0; }` | branch-specific rejection/error | [ ] |
| 289 | `ZSTD_CCtxParams_init` (src/compress/zstd_compress.c:359) | `RETURN_ERROR_IF(!cctxParams, GENERIC, "NULL pointer!");` | `ERROR(GENERIC)` | [ ] |
| 290 | `ZSTD_CCtxParams_init_internal` (src/compress/zstd_compress.c:377) | `assert(!ZSTD_checkCParams(params->cParams));` | assertion failure | [ ] |
| 291 | `ZSTD_CCtxParams_init_advanced` (src/compress/zstd_compress.c:397) | `RETURN_ERROR_IF(!cctxParams, GENERIC, "NULL pointer!");` | `ERROR(GENERIC)` | [ ] |
| 292 | `ZSTD_CCtxParams_setZstdParams` (src/compress/zstd_compress.c:410) | `assert(!ZSTD_checkCParams(params->cParams));` | assertion failure | [ ] |
| 293 | `ZSTD_cParam_getBounds` (src/compress/zstd_compress.c:634) | `bounds.error = ERROR(parameter_unsupported);` | `ERROR(parameter_unsupported)` | [ ] |
| 294 | `ZSTD_cParam_clampBounds` (src/compress/zstd_compress.c:646) | `if (*value < bounds.lowerBound) *value = bounds.lowerBound;` | branch-specific rejection/error | [ ] |
| 295 | `ZSTD_cParam_clampBounds` (src/compress/zstd_compress.c:647) | `if (*value > bounds.upperBound) *value = bounds.upperBound;` | branch-specific rejection/error | [ ] |
| 296 | `ZSTD_cParam_clampBounds` (src/compress/zstd_compress.c:653) | `RETURN_ERROR_IF(!ZSTD_cParam_withinBounds(cParam,val),        \` | `ERROR(val)` | [ ] |
| 297 | `ZSTD_CCtx_setParameter` (src/compress/zstd_compress.c:711) | `if (cctx->streamStage != zcss_init) {` | `ERROR(g)` | [ ] |
| 298 | `ZSTD_CCtx_setParameter` (src/compress/zstd_compress.c:715) | `RETURN_ERROR(stage_wrong, "can only set params in cctx init stage");` | `ERROR(g)` | [ ] |
| 299 | `ZSTD_CCtx_setParameter` (src/compress/zstd_compress.c:721) | `RETURN_ERROR_IF((value!=0) && cctx->staticSize, parameter_unsupported,` | `ERROR(0)` | [ ] |
| 300 | `ZSTD_CCtx_setParameter` (src/compress/zstd_compress.c:765) | `default: RETURN_ERROR(parameter_unsupported, "unknown parameter");` | `ERROR(d)` | [ ] |
| 301 | `ZSTD_CCtxParams_setParameter` (src/compress/zstd_compress.c:783) | `if (value == 0)` | branch-specific rejection/error | [ ] |
| 302 | `ZSTD_CCtxParams_setParameter` (src/compress/zstd_compress.c:787) | `if (CCtxParams->compressionLevel >= 0) return (size_t)CCtxParams->compressionLevel;` | branch-specific rejection/error | [ ] |
| 303 | `ZSTD_CCtxParams_setParameter` (src/compress/zstd_compress.c:792) | `if (value!=0)   /* 0 => use default */` | branch-specific rejection/error | [ ] |
| 304 | `ZSTD_CCtxParams_setParameter` (src/compress/zstd_compress.c:798) | `if (value!=0)   /* 0 => use default */` | branch-specific rejection/error | [ ] |
| 305 | `ZSTD_CCtxParams_setParameter` (src/compress/zstd_compress.c:804) | `if (value!=0)   /* 0 => use default */` | branch-specific rejection/error | [ ] |
| 306 | `ZSTD_CCtxParams_setParameter` (src/compress/zstd_compress.c:810) | `if (value!=0)   /* 0 => use default */` | branch-specific rejection/error | [ ] |
| 307 | `ZSTD_CCtxParams_setParameter` (src/compress/zstd_compress.c:816) | `if (value!=0)   /* 0 => use default */` | branch-specific rejection/error | [ ] |
| 308 | `ZSTD_CCtxParams_setParameter` (src/compress/zstd_compress.c:827) | `if (value!=0)   /* 0 => use default */` | branch-specific rejection/error | [ ] |
| 309 | `ZSTD_CCtxParams_setParameter` (src/compress/zstd_compress.c:868) | `RETURN_ERROR_IF(value!=0, parameter_unsupported, "not compiled with multithreading");` | `ERROR(parameter_unsupported)` | [ ] |
| 310 | `ZSTD_CCtxParams_setParameter` (src/compress/zstd_compress.c:878) | `RETURN_ERROR_IF(value!=0, parameter_unsupported, "not compiled with multithreading");` | `ERROR(parameter_unsupported)` | [ ] |
| 311 | `ZSTD_CCtxParams_setParameter` (src/compress/zstd_compress.c:882) | `if (value != 0 && value < ZSTDMT_JOBSIZE_MIN)` | branch-specific rejection/error | [ ] |
| 312 | `ZSTD_CCtxParams_setParameter` (src/compress/zstd_compress.c:885) | `assert(value >= 0);` | assertion failure | [ ] |
| 313 | `ZSTD_CCtxParams_setParameter` (src/compress/zstd_compress.c:892) | `RETURN_ERROR_IF(value!=0, parameter_unsupported, "not compiled with multithreading");` | `ERROR(parameter_unsupported)` | [ ] |
| 314 | `ZSTD_CCtxParams_setParameter` (src/compress/zstd_compress.c:902) | `RETURN_ERROR_IF(value!=0, parameter_unsupported, "not compiled with multithreading");` | `ERROR(parameter_unsupported)` | [ ] |
| 315 | `ZSTD_CCtxParams_setParameter` (src/compress/zstd_compress.c:920) | `if (value!=0)   /* 0 ==> auto */` | branch-specific rejection/error | [ ] |
| 316 | `ZSTD_CCtxParams_setParameter` (src/compress/zstd_compress.c:926) | `if (value!=0)   /* 0 ==> default */` | branch-specific rejection/error | [ ] |
| 317 | `ZSTD_CCtxParams_setParameter` (src/compress/zstd_compress.c:932) | `if (value!=0)   /* 0 ==> default */` | branch-specific rejection/error | [ ] |
| 318 | `ZSTD_CCtxParams_setParameter` (src/compress/zstd_compress.c:938) | `if (value!=0)   /* 0 ==> default */` | branch-specific rejection/error | [ ] |
| 319 | `ZSTD_CCtxParams_setParameter` (src/compress/zstd_compress.c:944) | `if (value!=0) {  /* 0 ==> default */` | branch-specific rejection/error | [ ] |
| 320 | `ZSTD_CCtxParams_setParameter` (src/compress/zstd_compress.c:952) | `if (value!=0)    /* 0 ==> default */` | branch-specific rejection/error | [ ] |
| 321 | `ZSTD_CCtxParams_setParameter` (src/compress/zstd_compress.c:1008) | `if (value!=0)    /* 0 ==> default */` | branch-specific rejection/error | [ ] |
| 322 | `ZSTD_CCtxParams_setParameter` (src/compress/zstd_compress.c:1010) | `assert(value>=0);` | assertion failure | [ ] |
| 323 | `ZSTD_CCtxParams_setParameter` (src/compress/zstd_compress.c:1019) | `default: RETURN_ERROR(parameter_unsupported, "unknown parameter");` | `ERROR(d)` | [ ] |
| 324 | `ZSTD_CCtxParams_getParameter` (src/compress/zstd_compress.c:1080) | `assert(CCtxParams->nbWorkers == 0);` | assertion failure | [ ] |
| 325 | `ZSTD_CCtxParams_getParameter` (src/compress/zstd_compress.c:1086) | `RETURN_ERROR(parameter_unsupported, "not compiled with multithreading");` | `ERROR(d)` | [ ] |
| 326 | `ZSTD_CCtxParams_getParameter` (src/compress/zstd_compress.c:1088) | `assert(CCtxParams->jobSize <= INT_MAX);` | assertion failure | [ ] |
| 327 | `ZSTD_CCtxParams_getParameter` (src/compress/zstd_compress.c:1094) | `RETURN_ERROR(parameter_unsupported, "not compiled with multithreading");` | `ERROR(d)` | [ ] |
| 328 | `ZSTD_CCtxParams_getParameter` (src/compress/zstd_compress.c:1101) | `RETURN_ERROR(parameter_unsupported, "not compiled with multithreading");` | `ERROR(d)` | [ ] |
| 329 | `ZSTD_CCtxParams_getParameter` (src/compress/zstd_compress.c:1166) | `default: RETURN_ERROR(parameter_unsupported, "unknown parameter");` | `ERROR(d)` | [ ] |
| 330 | `ZSTD_CCtx_setParametersUsingCCtxParams` (src/compress/zstd_compress.c:1182) | `RETURN_ERROR_IF(cctx->streamStage != zcss_init, stage_wrong,` | `ERROR(stage_wrong)` | [ ] |
| 331 | `ZSTD_CCtx_setParametersUsingCCtxParams` (src/compress/zstd_compress.c:1184) | `RETURN_ERROR_IF(cctx->cdict, stage_wrong,` | `ERROR(stage_wrong)` | [ ] |
| 332 | `ZSTD_CCtx_setPledgedSrcSize` (src/compress/zstd_compress.c:1233) | `RETURN_ERROR_IF(cctx->streamStage != zcss_init, stage_wrong,` | `ERROR(stage_wrong)` | [ ] |
| 333 | `ZSTD_initLocalDict` (src/compress/zstd_compress.c:1255) | `if (dl->dict == NULL) {` | branch-specific rejection/error | [ ] |
| 334 | `ZSTD_initLocalDict` (src/compress/zstd_compress.c:1257) | `assert(dl->dictBuffer == NULL);` | assertion failure | [ ] |
| 335 | `ZSTD_initLocalDict` (src/compress/zstd_compress.c:1258) | `assert(dl->cdict == NULL);` | assertion failure | [ ] |
| 336 | `ZSTD_initLocalDict` (src/compress/zstd_compress.c:1259) | `assert(dl->dictSize == 0);` | assertion failure | [ ] |
| 337 | `ZSTD_initLocalDict` (src/compress/zstd_compress.c:1262) | `if (dl->cdict != NULL) {` | branch-specific rejection/error | [ ] |
| 338 | `ZSTD_initLocalDict` (src/compress/zstd_compress.c:1264) | `assert(cctx->cdict == dl->cdict);` | assertion failure | [ ] |
| 339 | `ZSTD_initLocalDict` (src/compress/zstd_compress.c:1267) | `assert(dl->dictSize > 0);` | assertion failure | [ ] |
| 340 | `ZSTD_initLocalDict` (src/compress/zstd_compress.c:1268) | `assert(cctx->cdict == NULL);` | assertion failure | [ ] |
| 341 | `ZSTD_initLocalDict` (src/compress/zstd_compress.c:1269) | `assert(cctx->prefixDict.dict == NULL);` | assertion failure | [ ] |
| 342 | `ZSTD_initLocalDict` (src/compress/zstd_compress.c:1278) | `RETURN_ERROR_IF(!dl->cdict, memory_allocation, "ZSTD_createCDict_advanced failed");` | `ERROR(memory_allocation)` | [ ] |
| 343 | `ZSTD_CCtx_loadDictionary_advanced` (src/compress/zstd_compress.c:1290) | `RETURN_ERROR_IF(cctx->streamStage != zcss_init, stage_wrong,` | `ERROR(stage_wrong)` | [ ] |
| 344 | `ZSTD_CCtx_loadDictionary_advanced` (src/compress/zstd_compress.c:1293) | `if (dict == NULL \|\| dictSize == 0)  /* no dictionary */` | branch-specific rejection/error | [ ] |
| 345 | `ZSTD_CCtx_loadDictionary_advanced` (src/compress/zstd_compress.c:1300) | `RETURN_ERROR_IF(cctx->staticSize, memory_allocation,` | `ERROR(memory_allocation)` | [ ] |
| 346 | `ZSTD_CCtx_loadDictionary_advanced` (src/compress/zstd_compress.c:1303) | `RETURN_ERROR_IF(dictBuffer==NULL, memory_allocation,` | `ERROR(memory_allocation)` | [ ] |
| 347 | `ZSTD_CCtx_refCDict` (src/compress/zstd_compress.c:1330) | `RETURN_ERROR_IF(cctx->streamStage != zcss_init, stage_wrong,` | `ERROR(stage_wrong)` | [ ] |
| 348 | `ZSTD_CCtx_refThreadPool` (src/compress/zstd_compress.c:1340) | `RETURN_ERROR_IF(cctx->streamStage != zcss_init, stage_wrong,` | `ERROR(stage_wrong)` | [ ] |
| 349 | `ZSTD_CCtx_refPrefix_advanced` (src/compress/zstd_compress.c:1354) | `RETURN_ERROR_IF(cctx->streamStage != zcss_init, stage_wrong,` | `ERROR(stage_wrong)` | [ ] |
| 350 | `ZSTD_CCtx_refPrefix_advanced` (src/compress/zstd_compress.c:1357) | `if (prefix != NULL && prefixSize > 0) {` | branch-specific rejection/error | [ ] |
| 351 | `ZSTD_CCtx_reset` (src/compress/zstd_compress.c:1376) | `RETURN_ERROR_IF(cctx->streamStage != zcss_init, stage_wrong,` | `ERROR(stage_wrong)` | [ ] |
| 352 | `ZSTD_clampCParams` (src/compress/zstd_compress.c:1409) | `if ((int)val<bounds.lowerBound) val=(type)bounds.lowerBound;      \` | branch-specific rejection/error | [ ] |
| 353 | `ZSTD_clampCParams` (src/compress/zstd_compress.c:1410) | `else if ((int)val>bounds.upperBound) val=(type)bounds.upperBound; \` | branch-specific rejection/error | [ ] |
| 354 | `ZSTD_dictAndWindowLog` (src/compress/zstd_compress.c:1443) | `if (dictSize == 0) {` | branch-specific rejection/error | [ ] |
| 355 | `ZSTD_dictAndWindowLog` (src/compress/zstd_compress.c:1446) | `assert(windowLog <= ZSTD_WINDOWLOG_MAX);` | assertion failure | [ ] |
| 356 | `ZSTD_dictAndWindowLog` (src/compress/zstd_compress.c:1447) | `assert(srcSize != ZSTD_CONTENTSIZE_UNKNOWN); /* Handled in ZSTD_adjustCParams_internal() */` | assertion failure | [ ] |
| 357 | `ZSTD_dictAndWindowLog` (src/compress/zstd_compress.c:1455) | `if (windowSize >= dictSize + srcSize) {` | branch-specific rejection/error | [ ] |
| 358 | `ZSTD_dictAndWindowLog` (src/compress/zstd_compress.c:1457) | `} else if (dictAndWindowSize >= maxWindowSize) {` | branch-specific rejection/error | [ ] |
| 359 | `ZSTD_adjustCParams_internal` (src/compress/zstd_compress.c:1481) | `assert(ZSTD_checkCParams(cPar)==0);` | assertion failure | [ ] |
| 360 | `ZSTD_adjustCParams_internal` (src/compress/zstd_compress.c:1548) | `assert(0);` | assertion failure | [ ] |
| 361 | `ZSTD_adjustCParams_internal` (src/compress/zstd_compress.c:1553) | `if ( (srcSize <= maxWindowResize)` | branch-specific rejection/error | [ ] |
| 362 | `ZSTD_adjustCParams_internal` (src/compress/zstd_compress.c:1559) | `if (cPar.windowLog > srcLog) cPar.windowLog = srcLog;` | branch-specific rejection/error | [ ] |
| 363 | `ZSTD_adjustCParams_internal` (src/compress/zstd_compress.c:1564) | `if (cPar.hashLog > dictAndWindowLog+1) cPar.hashLog = dictAndWindowLog+1;` | branch-specific rejection/error | [ ] |
| 364 | `ZSTD_adjustCParams_internal` (src/compress/zstd_compress.c:1565) | `if (cycleLog > dictAndWindowLog)` | branch-specific rejection/error | [ ] |
| 365 | `ZSTD_adjustCParams_internal` (src/compress/zstd_compress.c:1569) | `if (cPar.windowLog < ZSTD_WINDOWLOG_ABSOLUTEMIN)` | branch-specific rejection/error | [ ] |
| 366 | `ZSTD_adjustCParams_internal` (src/compress/zstd_compress.c:1577) | `if (cPar.hashLog > maxShortCacheHashLog) {` | branch-specific rejection/error | [ ] |
| 367 | `ZSTD_adjustCParams_internal` (src/compress/zstd_compress.c:1580) | `if (cPar.chainLog > maxShortCacheHashLog) {` | branch-specific rejection/error | [ ] |
| 368 | `ZSTD_adjustCParams_internal` (src/compress/zstd_compress.c:1602) | `assert(cPar.hashLog >= rowLog);` | assertion failure | [ ] |
| 369 | `ZSTD_adjustCParams_internal` (src/compress/zstd_compress.c:1603) | `if (cPar.hashLog > maxHashLog) {` | branch-specific rejection/error | [ ] |
| 370 | `ZSTD_adjustCParams` (src/compress/zstd_compress.c:1617) | `if (srcSize == 0) srcSize = ZSTD_CONTENTSIZE_UNKNOWN;` | branch-specific rejection/error | [ ] |
| 371 | `ZSTD_overrideCParams` (src/compress/zstd_compress.c:1628) | `if (overrides->windowLog)    cParams->windowLog    = overrides->windowLog;` | branch-specific rejection/error | [ ] |
| 372 | `ZSTD_overrideCParams` (src/compress/zstd_compress.c:1629) | `if (overrides->hashLog)      cParams->hashLog      = overrides->hashLog;` | branch-specific rejection/error | [ ] |
| 373 | `ZSTD_overrideCParams` (src/compress/zstd_compress.c:1630) | `if (overrides->chainLog)     cParams->chainLog     = overrides->chainLog;` | branch-specific rejection/error | [ ] |
| 374 | `ZSTD_overrideCParams` (src/compress/zstd_compress.c:1631) | `if (overrides->searchLog)    cParams->searchLog    = overrides->searchLog;` | branch-specific rejection/error | [ ] |
| 375 | `ZSTD_overrideCParams` (src/compress/zstd_compress.c:1632) | `if (overrides->minMatch)     cParams->minMatch     = overrides->minMatch;` | branch-specific rejection/error | [ ] |
| 376 | `ZSTD_overrideCParams` (src/compress/zstd_compress.c:1633) | `if (overrides->targetLength) cParams->targetLength = overrides->targetLength;` | branch-specific rejection/error | [ ] |
| 377 | `ZSTD_overrideCParams` (src/compress/zstd_compress.c:1634) | `if (overrides->strategy)     cParams->strategy     = overrides->strategy;` | branch-specific rejection/error | [ ] |
| 378 | `ZSTD_getCParamsFromCCtxParams` (src/compress/zstd_compress.c:1641) | `if (srcSizeHint == ZSTD_CONTENTSIZE_UNKNOWN && CCtxParams->srcSizeHint > 0) {` | branch-specific rejection/error | [ ] |
| 379 | `ZSTD_getCParamsFromCCtxParams` (src/compress/zstd_compress.c:1642) | `assert(CCtxParams->srcSizeHint>=0);` | assertion failure | [ ] |
| 380 | `ZSTD_getCParamsFromCCtxParams` (src/compress/zstd_compress.c:1646) | `if (CCtxParams->ldmParams.enableLdm == ZSTD_ps_enable) cParams.windowLog = ZSTD_LDM_DEFAULT_WINDOW_LOG;` | branch-specific rejection/error | [ ] |
| 381 | `ZSTD_getCParamsFromCCtxParams` (src/compress/zstd_compress.c:1648) | `assert(!ZSTD_checkCParams(cParams));` | assertion failure | [ ] |
| 382 | `ZSTD_sizeof_matchState` (src/compress/zstd_compress.c:1688) | `assert(useRowMatchFinder != ZSTD_ps_auto);` | assertion failure | [ ] |
| 383 | `ZSTD_estimateCCtxSize_usingCCtxParams` (src/compress/zstd_compress.c:1761) | `RETURN_ERROR_IF(params->nbWorkers > 0, GENERIC, "Estimate CCtx size is supported for single-threaded compression only.");` | `ERROR(GENERIC)` | [ ] |
| 384 | `ZSTD_estimateCCtxSize` (src/compress/zstd_compress.c:1806) | `if (newMB > memBudget) memBudget = newMB;` | branch-specific rejection/error | [ ] |
| 385 | `ZSTD_estimateCStreamSize_usingCCtxParams` (src/compress/zstd_compress.c:1813) | `RETURN_ERROR_IF(params->nbWorkers > 0, GENERIC, "Estimate CCtx size is supported for single-threaded compression only.");` | `ERROR(GENERIC)` | [ ] |
| 386 | `ZSTD_estimateCStreamSize` (src/compress/zstd_compress.c:1860) | `if (newMB > memBudget) memBudget = newMB;` | branch-specific rejection/error | [ ] |
| 387 | `ZSTD_getFrameProgression` (src/compress/zstd_compress.c:1872) | `if (cctx->appliedParams.nbWorkers > 0) {` | branch-specific rejection/error | [ ] |
| 388 | `ZSTD_getFrameProgression` (src/compress/zstd_compress.c:1879) | `if (buffered) assert(cctx->inBuffPos >= cctx->inToCompress);` | assertion failure | [ ] |
| 389 | `ZSTD_getFrameProgression` (src/compress/zstd_compress.c:1880) | `assert(buffered <= ZSTD_BLOCKSIZE_MAX);` | assertion failure | [ ] |
| 390 | `ZSTD_toFlushNow` (src/compress/zstd_compress.c:1896) | `if (cctx->appliedParams.nbWorkers > 0) {` | branch-specific rejection/error | [ ] |
| 391 | `ZSTD_assertEqualCParams` (src/compress/zstd_compress.c:1909) | `assert(cParams1.windowLog    == cParams2.windowLog);` | assertion failure | [ ] |
| 392 | `ZSTD_assertEqualCParams` (src/compress/zstd_compress.c:1910) | `assert(cParams1.chainLog     == cParams2.chainLog);` | assertion failure | [ ] |
| 393 | `ZSTD_assertEqualCParams` (src/compress/zstd_compress.c:1911) | `assert(cParams1.hashLog      == cParams2.hashLog);` | assertion failure | [ ] |
| 394 | `ZSTD_assertEqualCParams` (src/compress/zstd_compress.c:1912) | `assert(cParams1.searchLog    == cParams2.searchLog);` | assertion failure | [ ] |
| 395 | `ZSTD_assertEqualCParams` (src/compress/zstd_compress.c:1913) | `assert(cParams1.minMatch     == cParams2.minMatch);` | assertion failure | [ ] |
| 396 | `ZSTD_assertEqualCParams` (src/compress/zstd_compress.c:1914) | `assert(cParams1.targetLength == cParams2.targetLength);` | assertion failure | [ ] |
| 397 | `ZSTD_assertEqualCParams` (src/compress/zstd_compress.c:1915) | `assert(cParams1.strategy     == cParams2.strategy);` | assertion failure | [ ] |
| 398 | `ZSTD_reset_matchState` (src/compress/zstd_compress.c:2003) | `assert(useRowMatchFinder != ZSTD_ps_auto);` | assertion failure | [ ] |
| 399 | `ZSTD_reset_matchState` (src/compress/zstd_compress.c:2014) | `assert(!ZSTD_cwksp_reserve_failed(ws)); /* check that allocation hasn't already failed */` | assertion failure | [ ] |
| 400 | `ZSTD_reset_matchState` (src/compress/zstd_compress.c:2023) | `RETURN_ERROR_IF(ZSTD_cwksp_reserve_failed(ws), memory_allocation,` | `ERROR(s)` | [ ] |
| 401 | `ZSTD_reset_matchState` (src/compress/zstd_compress.c:2032) | `if (ZSTD_rowMatchFinderUsed(cParams->strategy, useRowMatchFinder)) {` | branch-specific rejection/error | [ ] |
| 402 | `ZSTD_reset_matchState` (src/compress/zstd_compress.c:2048) | `assert(cParams->hashLog >= rowLog);` | assertion failure | [ ] |
| 403 | `ZSTD_reset_matchState` (src/compress/zstd_compress.c:2054) | `if ((forWho == ZSTD_resetTarget_CCtx) && (cParams->strategy >= ZSTD_btopt)) {` | branch-specific rejection/error | [ ] |
| 404 | `ZSTD_reset_matchState` (src/compress/zstd_compress.c:2066) | `RETURN_ERROR_IF(ZSTD_cwksp_reserve_failed(ws), memory_allocation,` | `ERROR(s)` | [ ] |
| 405 | `ZSTD_resetCCtx_internal` (src/compress/zstd_compress.c:2110) | `assert(!ZSTD_isError(ZSTD_checkCParams(params->cParams)));` | assertion failure | [ ] |
| 406 | `ZSTD_resetCCtx_internal` (src/compress/zstd_compress.c:2120) | `assert(params->useRowMatchFinder != ZSTD_ps_auto);` | assertion failure | [ ] |
| 407 | `ZSTD_resetCCtx_internal` (src/compress/zstd_compress.c:2121) | `assert(params->postBlockSplitter != ZSTD_ps_auto);` | assertion failure | [ ] |
| 408 | `ZSTD_resetCCtx_internal` (src/compress/zstd_compress.c:2122) | `assert(params->ldmParams.enableLdm != ZSTD_ps_auto);` | assertion failure | [ ] |
| 409 | `ZSTD_resetCCtx_internal` (src/compress/zstd_compress.c:2123) | `assert(params->maxBlockSize != 0);` | assertion failure | [ ] |
| 410 | `ZSTD_resetCCtx_internal` (src/compress/zstd_compress.c:2124) | `if (params->ldmParams.enableLdm == ZSTD_ps_enable) {` | branch-specific rejection/error | [ ] |
| 411 | `ZSTD_resetCCtx_internal` (src/compress/zstd_compress.c:2127) | `assert(params->ldmParams.hashLog >= params->ldmParams.bucketSizeLog);` | assertion failure | [ ] |
| 412 | `ZSTD_resetCCtx_internal` (src/compress/zstd_compress.c:2128) | `assert(params->ldmParams.hashRateLog < 32);` | assertion failure | [ ] |
| 413 | `ZSTD_resetCCtx_internal` (src/compress/zstd_compress.c:2154) | `if (!zc->staticSize) ZSTD_cwksp_bump_oversized_duration(ws, 0);` | branch-specific rejection/error | [ ] |
| 414 | `ZSTD_resetCCtx_internal` (src/compress/zstd_compress.c:2168) | `RETURN_ERROR_IF(zc->staticSize, memory_allocation, "static cctx : no resize");` | `ERROR(memory_allocation)` | [ ] |
| 415 | `ZSTD_resetCCtx_internal` (src/compress/zstd_compress.c:2179) | `assert(ZSTD_cwksp_check_available(ws, 2 * sizeof(ZSTD_compressedBlockState_t)));` | `ERROR(memory_allocation)` | [ ] |
| 416 | `ZSTD_resetCCtx_internal` (src/compress/zstd_compress.c:2181) | `RETURN_ERROR_IF(zc->blockState.prevCBlock == NULL, memory_allocation, "couldn't allocate prevCBlock");` | `ERROR(memory_allocation)` | [ ] |
| 417 | `ZSTD_resetCCtx_internal` (src/compress/zstd_compress.c:2183) | `RETURN_ERROR_IF(zc->blockState.nextCBlock == NULL, memory_allocation, "couldn't allocate nextCBlock");` | `ERROR(memory_allocation)` | [ ] |
| 418 | `ZSTD_resetCCtx_internal` (src/compress/zstd_compress.c:2185) | `RETURN_ERROR_IF(zc->tmpWorkspace == NULL, memory_allocation, "couldn't allocate tmpWorkspace");` | `ERROR(memory_allocation)` | [ ] |
| 419 | `ZSTD_resetCCtx_internal` (src/compress/zstd_compress.c:2222) | `if (params->ldmParams.enableLdm == ZSTD_ps_enable) {` | branch-specific rejection/error | [ ] |
| 420 | `ZSTD_resetCCtx_internal` (src/compress/zstd_compress.c:2257) | `if (params->ldmParams.enableLdm == ZSTD_ps_enable) {` | branch-specific rejection/error | [ ] |
| 421 | `ZSTD_resetCCtx_internal` (src/compress/zstd_compress.c:2274) | `assert(ZSTD_cwksp_estimated_space_within_bounds(ws, neededSpace));` | assertion failure | [ ] |
| 422 | `ZSTD_invalidateRepCodes` (src/compress/zstd_compress.c:2289) | `assert(!ZSTD_window_hasExtDict(cctx->blockState.matchState.window));` | assertion failure | [ ] |
| 423 | `ZSTD_resetCCtx_byAttachingCDict` (src/compress/zstd_compress.c:2336) | `assert(windowLog != 0);` | assertion failure | [ ] |
| 424 | `ZSTD_resetCCtx_byAttachingCDict` (src/compress/zstd_compress.c:2341) | `if (cdict->matchState.dedicatedDictSearch) {` | branch-specific rejection/error | [ ] |
| 425 | `ZSTD_resetCCtx_byAttachingCDict` (src/compress/zstd_compress.c:2353) | `assert(cctx->appliedParams.cParams.strategy == adjusted_cdict_cParams.strategy);` | assertion failure | [ ] |
| 426 | `ZSTD_resetCCtx_byAttachingCDict` (src/compress/zstd_compress.c:2359) | `if (cdictLen == 0) {` | branch-specific rejection/error | [ ] |
| 427 | `ZSTD_resetCCtx_byAttachingCDict` (src/compress/zstd_compress.c:2368) | `if (cctx->blockState.matchState.window.dictLimit < cdictEnd) {` | branch-specific rejection/error | [ ] |
| 428 | `ZSTD_resetCCtx_byCopyingCDict` (src/compress/zstd_compress.c:2410) | `assert(!cdict->matchState.dedicatedDictSearch);` | assertion failure | [ ] |
| 429 | `ZSTD_resetCCtx_byCopyingCDict` (src/compress/zstd_compress.c:2415) | `assert(windowLog != 0);` | assertion failure | [ ] |
| 430 | `ZSTD_resetCCtx_byCopyingCDict` (src/compress/zstd_compress.c:2423) | `assert(cctx->appliedParams.cParams.strategy == cdict_cParams->strategy);` | assertion failure | [ ] |
| 431 | `ZSTD_resetCCtx_byCopyingCDict` (src/compress/zstd_compress.c:2424) | `assert(cctx->appliedParams.cParams.hashLog == cdict_cParams->hashLog);` | assertion failure | [ ] |
| 432 | `ZSTD_resetCCtx_byCopyingCDict` (src/compress/zstd_compress.c:2425) | `assert(cctx->appliedParams.cParams.chainLog == cdict_cParams->chainLog);` | assertion failure | [ ] |
| 433 | `ZSTD_resetCCtx_byCopyingCDict` (src/compress/zstd_compress.c:2429) | `assert(params.useRowMatchFinder != ZSTD_ps_auto);` | assertion failure | [ ] |
| 434 | `ZSTD_resetCCtx_byCopyingCDict` (src/compress/zstd_compress.c:2442) | `if (ZSTD_allocateChainTable(cctx->appliedParams.cParams.strategy, cctx->appliedParams.useRowMatchFinder, 0 /* forDDSDict */)) {` | branch-specific rejection/error | [ ] |
| 435 | `ZSTD_resetCCtx_byCopyingCDict` (src/compress/zstd_compress.c:2448) | `if (ZSTD_rowMatchFinderUsed(cdict_cParams->strategy, cdict->useRowMatchFinder)) {` | branch-specific rejection/error | [ ] |
| 436 | `ZSTD_resetCCtx_byCopyingCDict` (src/compress/zstd_compress.c:2458) | `assert(cctx->blockState.matchState.hashLog3 <= 31);` | assertion failure | [ ] |
| 437 | `ZSTD_resetCCtx_byCopyingCDict` (src/compress/zstd_compress.c:2461) | `assert(cdict->matchState.hashLog3 == 0);` | assertion failure | [ ] |
| 438 | `ZSTD_copyCCtx_internal` (src/compress/zstd_compress.c:2519) | `RETURN_ERROR_IF(srcCCtx->stage!=ZSTDcs_init, stage_wrong,` | `ERROR(stage_wrong)` | [ ] |
| 439 | `ZSTD_copyCCtx_internal` (src/compress/zstd_compress.c:2526) | `assert(srcCCtx->appliedParams.useRowMatchFinder != ZSTD_ps_auto);` | assertion failure | [ ] |
| 440 | `ZSTD_copyCCtx_internal` (src/compress/zstd_compress.c:2527) | `assert(srcCCtx->appliedParams.postBlockSplitter != ZSTD_ps_auto);` | assertion failure | [ ] |
| 441 | `ZSTD_copyCCtx_internal` (src/compress/zstd_compress.c:2528) | `assert(srcCCtx->appliedParams.ldmParams.enableLdm != ZSTD_ps_auto);` | assertion failure | [ ] |
| 442 | `ZSTD_copyCCtx_internal` (src/compress/zstd_compress.c:2537) | `assert(dstCCtx->appliedParams.cParams.windowLog == srcCCtx->appliedParams.cParams.windowLog);` | assertion failure | [ ] |
| 443 | `ZSTD_copyCCtx_internal` (src/compress/zstd_compress.c:2538) | `assert(dstCCtx->appliedParams.cParams.strategy == srcCCtx->appliedParams.cParams.strategy);` | assertion failure | [ ] |
| 444 | `ZSTD_copyCCtx_internal` (src/compress/zstd_compress.c:2539) | `assert(dstCCtx->appliedParams.cParams.hashLog == srcCCtx->appliedParams.cParams.hashLog);` | assertion failure | [ ] |
| 445 | `ZSTD_copyCCtx_internal` (src/compress/zstd_compress.c:2540) | `assert(dstCCtx->appliedParams.cParams.chainLog == srcCCtx->appliedParams.cParams.chainLog);` | assertion failure | [ ] |
| 446 | `ZSTD_copyCCtx_internal` (src/compress/zstd_compress.c:2541) | `assert(dstCCtx->blockState.matchState.hashLog3 == srcCCtx->blockState.matchState.hashLog3);` | assertion failure | [ ] |
| 447 | `ZSTD_copyCCtx` (src/compress/zstd_compress.c:2596) | `if (pledgedSrcSize==0) pledgedSrcSize = ZSTD_CONTENTSIZE_UNKNOWN;` | branch-specific rejection/error | [ ] |
| 448 | `ZSTD_reduceTable_internal` (src/compress/zstd_compress.c:2620) | `assert((size & (ZSTD_ROWSIZE-1)) == 0);  /* multiple of ZSTD_ROWSIZE */` | assertion failure | [ ] |
| 449 | `ZSTD_reduceTable_internal` (src/compress/zstd_compress.c:2621) | `assert(size < (1U<<31));   /* can be cast to int */` | assertion failure | [ ] |
| 450 | `ZSTD_reduceTable_internal` (src/compress/zstd_compress.c:2644) | `} else if (table[cellNb] < reducerThreshold) {` | branch-specific rejection/error | [ ] |
| 451 | `ZSTD_reduceIndex` (src/compress/zstd_compress.c:2672) | `if (ZSTD_allocateChainTable(params->cParams.strategy, params->useRowMatchFinder, (U32)ms->dedicatedDictSearch)) {` | branch-specific rejection/error | [ ] |
| 452 | `ZSTD_reduceIndex` (src/compress/zstd_compress.c:2674) | `if (params->cParams.strategy == ZSTD_btlazy2)` | branch-specific rejection/error | [ ] |
| 453 | `ZSTD_reduceIndex` (src/compress/zstd_compress.c:2680) | `if (ms->hashLog3) {` | branch-specific rejection/error | [ ] |
| 454 | `ZSTD_seqToCodes` (src/compress/zstd_compress.c:2702) | `assert(nbSeq <= seqStorePtr->maxNbSeq);` | assertion failure | [ ] |
| 455 | `ZSTD_seqToCodes` (src/compress/zstd_compress.c:2710) | `assert(!(MEM_64bits() && ofCode >= STREAM_ACCUMULATOR_MIN));` | assertion failure | [ ] |
| 456 | `ZSTD_seqToCodes` (src/compress/zstd_compress.c:2711) | `if (MEM_32bits() && ofCode >= STREAM_ACCUMULATOR_MIN)` | branch-specific rejection/error | [ ] |
| 457 | `ZSTD_seqToCodes` (src/compress/zstd_compress.c:2714) | `if (seqStorePtr->longLengthType==ZSTD_llt_literalLength)` | branch-specific rejection/error | [ ] |
| 458 | `ZSTD_seqToCodes` (src/compress/zstd_compress.c:2716) | `if (seqStorePtr->longLengthType==ZSTD_llt_matchLength)` | branch-specific rejection/error | [ ] |
| 459 | `ZSTD_blockSplitterEnabled` (src/compress/zstd_compress.c:2739) | `assert(cctxParams->postBlockSplitter != ZSTD_ps_auto);` | assertion failure | [ ] |
| 460 | `ZSTD_buildSequencesStatistics` (src/compress/zstd_compress.c:2784) | `assert(op <= oend);` | assertion failure | [ ] |
| 461 | `ZSTD_buildSequencesStatistics` (src/compress/zstd_compress.c:2785) | `assert(nbSeq != 0); /* ZSTD_selectEncodingType() divides by nbSeq */` | assertion failure | [ ] |
| 462 | `ZSTD_buildSequencesStatistics` (src/compress/zstd_compress.c:2796) | `assert(set_basic < set_compressed && set_rle < set_compressed);` | assertion failure | [ ] |
| 463 | `ZSTD_buildSequencesStatistics` (src/compress/zstd_compress.c:2797) | `assert(!(stats.LLtype < set_compressed && nextEntropy->litlength_repeatMode != FSE_repeat_none)); /* We don't copy tables */` | assertion failure | [ ] |
| 464 | `ZSTD_buildSequencesStatistics` (src/compress/zstd_compress.c:2814) | `assert(op <= oend);` | assertion failure | [ ] |
| 465 | `ZSTD_buildSequencesStatistics` (src/compress/zstd_compress.c:2829) | `assert(!(stats.Offtype < set_compressed && nextEntropy->offcode_repeatMode != FSE_repeat_none)); /* We don't copy tables */` | assertion failure | [ ] |
| 466 | `ZSTD_buildSequencesStatistics` (src/compress/zstd_compress.c:2846) | `assert(op <= oend);` | assertion failure | [ ] |
| 467 | `ZSTD_buildSequencesStatistics` (src/compress/zstd_compress.c:2859) | `assert(!(stats.MLtype < set_compressed && nextEntropy->matchlength_repeatMode != FSE_repeat_none)); /* We don't copy tables */` | assertion failure | [ ] |
| 468 | `ZSTD_buildSequencesStatistics` (src/compress/zstd_compress.c:2876) | `assert(op <= oend);` | assertion failure | [ ] |
| 469 | `ZSTD_entropyCompressSeqStore_internal` (src/compress/zstd_compress.c:2919) | `assert(entropyWkspSize >= HUF_WORKSPACE_SIZE);` | assertion failure | [ ] |
| 470 | `ZSTD_entropyCompressSeqStore_internal` (src/compress/zstd_compress.c:2935) | `assert(cSize <= dstCapacity);` | assertion failure | [ ] |
| 471 | `ZSTD_entropyCompressSeqStore_internal` (src/compress/zstd_compress.c:2940) | `RETURN_ERROR_IF((oend-op) < 3 /*max nbSeq Size*/ + 1 /*seqHead*/,` | `ERROR(p)` | [ ] |
| 472 | `ZSTD_entropyCompressSeqStore_internal` (src/compress/zstd_compress.c:2942) | `if (nbSeq < 128) {` | branch-specific rejection/error | [ ] |
| 473 | `ZSTD_entropyCompressSeqStore_internal` (src/compress/zstd_compress.c:2944) | `} else if (nbSeq < LONGNBSEQ) {` | branch-specific rejection/error | [ ] |
| 474 | `ZSTD_entropyCompressSeqStore_internal` (src/compress/zstd_compress.c:2953) | `assert(op <= oend);` | assertion failure | [ ] |
| 475 | `ZSTD_entropyCompressSeqStore_internal` (src/compress/zstd_compress.c:2954) | `if (nbSeq==0) {` | branch-specific rejection/error | [ ] |
| 476 | `ZSTD_entropyCompressSeqStore_internal` (src/compress/zstd_compress.c:2983) | `assert(op <= oend);` | assertion failure | [ ] |
| 477 | `ZSTD_entropyCompressSeqStore_internal` (src/compress/zstd_compress.c:2992) | `if (lastCountSize && (lastCountSize + bitstreamSize) < 4) {` | branch-specific rejection/error | [ ] |
| 478 | `ZSTD_entropyCompressSeqStore_internal` (src/compress/zstd_compress.c:2994) | `assert(lastCountSize + bitstreamSize == 3);` | assertion failure | [ ] |
| 479 | `ZSTD_entropyCompressSeqStore_wExtLitBuffer` (src/compress/zstd_compress.c:3022) | `if (cSize == 0) return 0;` | `ERROR(dstSize_tooSmall)` | [ ] |
| 480 | `ZSTD_entropyCompressSeqStore_wExtLitBuffer` (src/compress/zstd_compress.c:3026) | `if ((cSize == ERROR(dstSize_tooSmall)) & (blockSize <= dstCapacity)) {` | `ERROR(dstSize_tooSmall)` | [ ] |
| 481 | `ZSTD_entropyCompressSeqStore_wExtLitBuffer` (src/compress/zstd_compress.c:3034) | `if (cSize >= maxCSize) return 0;  /* block not compressed */` | branch-specific rejection/error | [ ] |
| 482 | `ZSTD_entropyCompressSeqStore_wExtLitBuffer` (src/compress/zstd_compress.c:3040) | `assert(cSize < ZSTD_BLOCKSIZE_MAX);` | assertion failure | [ ] |
| 483 | `ZSTD_selectBlockCompressor` (src/compress/zstd_compress.c:3119) | `assert(ZSTD_cParam_withinBounds(ZSTD_c_strategy, (int)strat));` | assertion failure | [ ] |
| 484 | `ZSTD_selectBlockCompressor` (src/compress/zstd_compress.c:3145) | `assert(useRowMatchFinder != ZSTD_ps_auto);` | assertion failure | [ ] |
| 485 | `ZSTD_selectBlockCompressor` (src/compress/zstd_compress.c:3150) | `assert(selectedCompressor != NULL);` | assertion failure | [ ] |
| 486 | `ZSTD_postProcessSequenceProducerResult` (src/compress/zstd_compress.c:3177) | `RETURN_ERROR_IF(` | `ERROR(sequenceProducer_failed)` | [ ] |
| 487 | `ZSTD_postProcessSequenceProducerResult` (src/compress/zstd_compress.c:3184) | `RETURN_ERROR_IF(` | `ERROR(sequenceProducer_failed)` | [ ] |
| 488 | `ZSTD_postProcessSequenceProducerResult` (src/compress/zstd_compress.c:3190) | `if (srcSize == 0) {` | branch-specific rejection/error | [ ] |
| 489 | `ZSTD_postProcessSequenceProducerResult` (src/compress/zstd_compress.c:3199) | `if (lastSeq.offset == 0 && lastSeq.matchLength == 0) {` | branch-specific rejection/error | [ ] |
| 490 | `ZSTD_postProcessSequenceProducerResult` (src/compress/zstd_compress.c:3205) | `RETURN_ERROR_IF(` | `ERROR(sequenceProducer_failed)` | [ ] |
| 491 | `ZSTD_validateSeqStore` (src/compress/zstd_compress.c:3245) | `assert(seqLength.matchLength >= matchLenLowerBound);` | assertion failure | [ ] |
| 492 | `ZSTD_buildSeqStore` (src/compress/zstd_compress.c:3268) | `assert(srcSize <= ZSTD_BLOCKSIZE_MAX);` | assertion failure | [ ] |
| 493 | `ZSTD_buildSeqStore` (src/compress/zstd_compress.c:3273) | `if (srcSize < MIN_CBLOCK_SIZE+ZSTD_blockHeaderSize+1+1) {` | branch-specific rejection/error | [ ] |
| 494 | `ZSTD_buildSeqStore` (src/compress/zstd_compress.c:3274) | `if (zc->appliedParams.cParams.strategy >= ZSTD_btopt) {` | branch-specific rejection/error | [ ] |
| 495 | `ZSTD_buildSeqStore` (src/compress/zstd_compress.c:3289) | `assert(ms->dictMatchState == NULL \|\| ms->loadedDictEnd == ms->window.dictLimit);` | assertion failure | [ ] |
| 496 | `ZSTD_buildSeqStore` (src/compress/zstd_compress.c:3295) | `if (sizeof(ptrdiff_t)==8) assert(istart - base < (ptrdiff_t)(U32)(-1));   /* ensure no overflow */` | assertion failure | [ ] |
| 497 | `ZSTD_buildSeqStore` (src/compress/zstd_compress.c:3296) | `if (curr > ms->nextToUpdate + 384)` | branch-specific rejection/error | [ ] |
| 498 | `ZSTD_buildSeqStore` (src/compress/zstd_compress.c:3307) | `if (zc->externSeqStore.pos < zc->externSeqStore.size) {` | branch-specific rejection/error | [ ] |
| 499 | `ZSTD_buildSeqStore` (src/compress/zstd_compress.c:3308) | `assert(zc->appliedParams.ldmParams.enableLdm == ZSTD_ps_disable);` | assertion failure | [ ] |
| 500 | `ZSTD_buildSeqStore` (src/compress/zstd_compress.c:3312) | `RETURN_ERROR_IF(` | `ERROR(s)` | [ ] |
| 501 | `ZSTD_buildSeqStore` (src/compress/zstd_compress.c:3325) | `assert(zc->externSeqStore.pos <= zc->externSeqStore.size);` | assertion failure | [ ] |
| 502 | `ZSTD_buildSeqStore` (src/compress/zstd_compress.c:3326) | `} else if (zc->appliedParams.ldmParams.enableLdm == ZSTD_ps_enable) {` | branch-specific rejection/error | [ ] |
| 503 | `ZSTD_buildSeqStore` (src/compress/zstd_compress.c:3331) | `RETURN_ERROR_IF(` | `ERROR(s)` | [ ] |
| 504 | `ZSTD_buildSeqStore` (src/compress/zstd_compress.c:3350) | `assert(ldmSeqStore.pos == ldmSeqStore.size);` | assertion failure | [ ] |
| 505 | `ZSTD_buildSeqStore` (src/compress/zstd_compress.c:3351) | `} else if (ZSTD_hasExtSeqProd(&zc->appliedParams)) {` | branch-specific rejection/error | [ ] |
| 506 | `ZSTD_buildSeqStore` (src/compress/zstd_compress.c:3352) | `assert(` | assertion failure | [ ] |
| 507 | `ZSTD_buildSeqStore` (src/compress/zstd_compress.c:3355) | `assert(zc->appliedParams.extSeqProdFunc != NULL);` | assertion failure | [ ] |
| 508 | `ZSTD_buildSeqStore` (src/compress/zstd_compress.c:3380) | `RETURN_ERROR_IF(seqLenSum > srcSize, externalSequences_invalid, "External sequences imply too large a block!");` | `ERROR(externalSequences_invalid)` | [ ] |
| 509 | `ZSTD_buildSeqStore` (src/compress/zstd_compress.c:3396) | `if (!zc->appliedParams.enableMatchFinderFallback) {` | branch-specific rejection/error | [ ] |
| 510 | `ZSTD_copyBlockSequences` (src/compress/zstd_compress.c:3444) | `assert(seqCollector->seqIndex <= seqCollector->maxSequences);` | `ERROR(t)` | [ ] |
| 511 | `ZSTD_copyBlockSequences` (src/compress/zstd_compress.c:3445) | `RETURN_ERROR_IF(` | `ERROR(t)` | [ ] |
| 512 | `ZSTD_copyBlockSequences` (src/compress/zstd_compress.c:3461) | `if (i == seqStore->longLengthPos) {` | branch-specific rejection/error | [ ] |
| 513 | `ZSTD_copyBlockSequences` (src/compress/zstd_compress.c:3462) | `if (seqStore->longLengthType == ZSTD_llt_literalLength) {` | branch-specific rejection/error | [ ] |
| 514 | `ZSTD_copyBlockSequences` (src/compress/zstd_compress.c:3464) | `} else if (seqStore->longLengthType == ZSTD_llt_matchLength) {` | branch-specific rejection/error | [ ] |
| 515 | `ZSTD_copyBlockSequences` (src/compress/zstd_compress.c:3472) | `assert(repcode > 0);` | assertion failure | [ ] |
| 516 | `ZSTD_copyBlockSequences` (src/compress/zstd_compress.c:3474) | `if (outSeqs[i].litLength != 0) {` | branch-specific rejection/error | [ ] |
| 517 | `ZSTD_copyBlockSequences` (src/compress/zstd_compress.c:3478) | `assert(repcodes.rep[0] > 1);` | assertion failure | [ ] |
| 518 | `ZSTD_copyBlockSequences` (src/compress/zstd_compress.c:3500) | `assert(nbInLiterals >= nbOutLiterals);` | assertion failure | [ ] |
| 519 | `ZSTD_copyBlockSequences` (src/compress/zstd_compress.c:3506) | `assert(nbOutSequences == nbInSequences + 1);` | assertion failure | [ ] |
| 520 | `ZSTD_copyBlockSequences` (src/compress/zstd_compress.c:3509) | `assert(seqCollector->seqIndex <= seqCollector->maxSequences);` | assertion failure | [ ] |
| 521 | `ZSTD_generateSequences` (src/compress/zstd_compress.c:3529) | `RETURN_ERROR_IF(targetCBlockSize != 0, parameter_unsupported, "targetCBlockSize != 0");` | `ERROR(parameter_unsupported)` | [ ] |
| 522 | `ZSTD_generateSequences` (src/compress/zstd_compress.c:3534) | `RETURN_ERROR_IF(nbWorkers != 0, parameter_unsupported, "nbWorkers != 0");` | `ERROR(parameter_unsupported)` | [ ] |
| 523 | `ZSTD_generateSequences` (src/compress/zstd_compress.c:3538) | `RETURN_ERROR_IF(dst == NULL, memory_allocation, "NULL pointer!");` | `ERROR(memory_allocation)` | [ ] |
| 524 | `ZSTD_generateSequences` (src/compress/zstd_compress.c:3551) | `assert(zc->seqCollector.seqIndex <= ZSTD_sequenceBound(srcSize));` | assertion failure | [ ] |
| 525 | `ZSTD_mergeBlockDelimiters` (src/compress/zstd_compress.c:3559) | `if (sequences[in].offset == 0 && sequences[in].matchLength == 0) {` | branch-specific rejection/error | [ ] |
| 526 | `ZSTD_buildBlockEntropyStats_literals` (src/compress/zstd_compress.c:3667) | `if (srcSize <= minLitSize) {` | branch-specific rejection/error | [ ] |
| 527 | `ZSTD_buildBlockEntropyStats_literals` (src/compress/zstd_compress.c:3685) | `if (largest <= (srcSize >> 7)+4) {` | branch-specific rejection/error | [ ] |
| 528 | `ZSTD_buildBlockEntropyStats_literals` (src/compress/zstd_compress.c:3701) | `assert(huffLog <= LitHufLog);` | assertion failure | [ ] |
| 529 | `ZSTD_buildBlockEntropyStats_literals` (src/compress/zstd_compress.c:3719) | `if (oldCSize < srcSize && (oldCSize <= hSize + newCSize \|\| hSize + 12 >= srcSize)) {` | branch-specific rejection/error | [ ] |
| 530 | `ZSTD_buildBlockEntropyStats_literals` (src/compress/zstd_compress.c:3725) | `if (newCSize + hSize >= srcSize) {` | branch-specific rejection/error | [ ] |
| 531 | `ZSTD_estimateBlockSize_literal` (src/compress/zstd_compress.c:3841) | `if (hufMetadata->hType == set_basic) return litSize;` | branch-specific rejection/error | [ ] |
| 532 | `ZSTD_estimateBlockSize_literal` (src/compress/zstd_compress.c:3842) | `else if (hufMetadata->hType == set_rle) return 1;` | branch-specific rejection/error | [ ] |
| 533 | `ZSTD_estimateBlockSize_literal` (src/compress/zstd_compress.c:3843) | `else if (hufMetadata->hType == set_compressed \|\| hufMetadata->hType == set_repeat) {` | branch-specific rejection/error | [ ] |
| 534 | `ZSTD_estimateBlockSize_literal` (src/compress/zstd_compress.c:3847) | `if (writeEntropy) cLitSizeEstimate += hufMetadata->hufDesSize;` | branch-specific rejection/error | [ ] |
| 535 | `ZSTD_estimateBlockSize_literal` (src/compress/zstd_compress.c:3851) | `assert(0); /* impossible */` | assertion failure | [ ] |
| 536 | `ZSTD_estimateBlockSize_symbolType` (src/compress/zstd_compress.c:3874) | `assert(max <= defaultMax);` | assertion failure | [ ] |
| 537 | `ZSTD_estimateBlockSize_sequences` (src/compress/zstd_compress.c:3918) | `if (writeEntropy) cSeqSizeEstimate += fseMetadata->fseTablesSize;` | branch-specific rejection/error | [ ] |
| 538 | `ZSTD_countSeqStoreLiteralsBytes` (src/compress/zstd_compress.c:3977) | `if (i == seqStore->longLengthPos && seqStore->longLengthType == ZSTD_llt_literalLength) {` | branch-specific rejection/error | [ ] |
| 539 | `ZSTD_countSeqStoreMatchBytes` (src/compress/zstd_compress.c:3992) | `if (i == seqStore->longLengthPos && seqStore->longLengthType == ZSTD_llt_matchLength) {` | branch-specific rejection/error | [ ] |
| 540 | `ZSTD_deriveSeqStoreChunk` (src/compress/zstd_compress.c:4006) | `if (startIdx > 0) {` | branch-specific rejection/error | [ ] |
| 541 | `ZSTD_deriveSeqStoreChunk` (src/compress/zstd_compress.c:4012) | `if (originalSeqStore->longLengthType != ZSTD_llt_none) {` | branch-specific rejection/error | [ ] |
| 542 | `ZSTD_deriveSeqStoreChunk` (src/compress/zstd_compress.c:4013) | `if (originalSeqStore->longLengthPos < startIdx \|\| originalSeqStore->longLengthPos > endIdx) {` | branch-specific rejection/error | [ ] |
| 543 | `ZSTD_deriveSeqStoreChunk` (src/compress/zstd_compress.c:4021) | `if (endIdx == (size_t)(originalSeqStore->sequences - originalSeqStore->sequencesStart)) {` | branch-specific rejection/error | [ ] |
| 544 | `ZSTD_deriveSeqStoreChunk` (src/compress/zstd_compress.c:4023) | `assert(resultSeqStore->lit == originalSeqStore->lit);` | assertion failure | [ ] |
| 545 | `ZSTD_resolveRepcodeToRawOffset` (src/compress/zstd_compress.c:4041) | `assert(OFFBASE_IS_REPCODE(offBase));` | assertion failure | [ ] |
| 546 | `ZSTD_resolveRepcodeToRawOffset` (src/compress/zstd_compress.c:4043) | `assert(ll0);` | assertion failure | [ ] |
| 547 | `ZSTD_seqStore_resolveOffCodes` (src/compress/zstd_compress.c:4079) | `assert(offBase > 0);` | assertion failure | [ ] |
| 548 | `ZSTD_compressSeqStore_singleBlock` (src/compress/zstd_compress.c:4124) | `RETURN_ERROR_IF(dstCapacity < ZSTD_blockHeaderSize, dstSize_tooSmall, "Block header doesn't fit");` | `ERROR(dstSize_tooSmall)` | [ ] |
| 549 | `ZSTD_compressSeqStore_singleBlock` (src/compress/zstd_compress.c:4134) | `if (!zc->isFirstBlock &&` | branch-specific rejection/error | [ ] |
| 550 | `ZSTD_compressSeqStore_singleBlock` (src/compress/zstd_compress.c:4145) | `if (zc->seqCollector.collectSequences) {` | branch-specific rejection/error | [ ] |
| 551 | `ZSTD_compressSeqStore_singleBlock` (src/compress/zstd_compress.c:4151) | `if (cSeqsSize == 0) {` | branch-specific rejection/error | [ ] |
| 552 | `ZSTD_compressSeqStore_singleBlock` (src/compress/zstd_compress.c:4168) | `if (zc->blockState.prevCBlock->entropy.fse.offcode_repeatMode == FSE_repeat_valid)` | branch-specific rejection/error | [ ] |
| 553 | `ZSTD_deriveBlockSplitsHelper` (src/compress/zstd_compress.c:4209) | `assert(endIdx >= startIdx);` | assertion failure | [ ] |
| 554 | `ZSTD_deriveBlockSplitsHelper` (src/compress/zstd_compress.c:4210) | `if (endIdx - startIdx < MIN_SEQUENCES_BLOCK_SPLITTING \|\| splits->idx >= ZSTD_MAX_NB_BLOCK_SPLITS) {` | branch-specific rejection/error | [ ] |
| 555 | `ZSTD_deriveBlockSplitsHelper` (src/compress/zstd_compress.c:4225) | `if (estimatedFirstHalfSize + estimatedSecondHalfSize < estimatedOriginalSize) {` | branch-specific rejection/error | [ ] |
| 556 | `ZSTD_deriveBlockSplits` (src/compress/zstd_compress.c:4244) | `if (nbSeq <= 4) {` | branch-specific rejection/error | [ ] |
| 557 | `ZSTD_compressBlock_splitBlock_internal` (src/compress/zstd_compress.c:4300) | `if (numSplits == 0) {` | branch-specific rejection/error | [ ] |
| 558 | `ZSTD_compressBlock_splitBlock_internal` (src/compress/zstd_compress.c:4309) | `assert(zc->blockSizeMax <= ZSTD_BLOCKSIZE_MAX);` | assertion failure | [ ] |
| 559 | `ZSTD_compressBlock_splitBlock_internal` (src/compress/zstd_compress.c:4310) | `assert(cSizeSingleBlock <= zc->blockSizeMax + ZSTD_blockHeaderSize);` | assertion failure | [ ] |
| 560 | `ZSTD_compressBlock_splitBlock_internal` (src/compress/zstd_compress.c:4344) | `assert(cSizeChunk <= zc->blockSizeMax + ZSTD_blockHeaderSize);` | assertion failure | [ ] |
| 561 | `ZSTD_compressBlock_splitBlock` (src/compress/zstd_compress.c:4361) | `assert(zc->appliedParams.postBlockSplitter == ZSTD_ps_enable);` | assertion failure | [ ] |
| 562 | `ZSTD_compressBlock_splitBlock` (src/compress/zstd_compress.c:4366) | `if (zc->blockState.prevCBlock->entropy.fse.offcode_repeatMode == FSE_repeat_valid)` | `ERROR(sequenceProducer_failed)` | [ ] |
| 563 | `ZSTD_compressBlock_splitBlock` (src/compress/zstd_compress.c:4368) | `RETURN_ERROR_IF(zc->seqCollector.collectSequences, sequenceProducer_failed, "Uncompressible block");` | `ERROR(sequenceProducer_failed)` | [ ] |
| 564 | `ZSTD_compressBlock_internal` (src/compress/zstd_compress.c:4402) | `RETURN_ERROR_IF(zc->seqCollector.collectSequences, sequenceProducer_failed, "Uncompressible block");` | `ERROR(sequenceProducer_failed)` | [ ] |
| 565 | `ZSTD_compressBlock_internal` (src/compress/zstd_compress.c:4408) | `if (zc->seqCollector.collectSequences) {` | branch-specific rejection/error | [ ] |
| 566 | `ZSTD_compressBlock_internal` (src/compress/zstd_compress.c:4437) | `if (!ZSTD_isError(cSize) && cSize > 1) {` | branch-specific rejection/error | [ ] |
| 567 | `ZSTD_compressBlock_internal` (src/compress/zstd_compress.c:4444) | `if (zc->blockState.prevCBlock->entropy.fse.offcode_repeatMode == FSE_repeat_valid)` | branch-specific rejection/error | [ ] |
| 568 | `ZSTD_compressBlock_targetCBlockSize_body` (src/compress/zstd_compress.c:4487) | `if (cSize != ERROR(dstSize_tooSmall)) {` | `ERROR(dstSize_tooSmall)` | [ ] |
| 569 | `ZSTD_compressBlock_targetCBlockSize_body` (src/compress/zstd_compress.c:4491) | `if (cSize != 0 && cSize < maxCSize + ZSTD_blockHeaderSize) {` | branch-specific rejection/error | [ ] |
| 570 | `ZSTD_compressBlock_targetCBlockSize` (src/compress/zstd_compress.c:4520) | `if (zc->blockState.prevCBlock->entropy.fse.offcode_repeatMode == FSE_repeat_valid)` | branch-specific rejection/error | [ ] |
| 571 | `ZSTD_overflowCorrectIfNeeded` (src/compress/zstd_compress.c:4534) | `if (ZSTD_window_needOverflowCorrection(ms->window, cycleLog, maxDist, ms->loadedDictEnd, ip, iend)) {` | branch-specific rejection/error | [ ] |
| 572 | `ZSTD_overflowCorrectIfNeeded` (src/compress/zstd_compress.c:4542) | `if (ms->nextToUpdate < correction) ms->nextToUpdate = 0;` | branch-specific rejection/error | [ ] |
| 573 | `ZSTD_optimalBlockSize` (src/compress/zstd_compress.c:4560) | `if (srcSize < 128 KB \|\| blockSizeMax < 128 KB)` | branch-specific rejection/error | [ ] |
| 574 | `ZSTD_optimalBlockSize` (src/compress/zstd_compress.c:4566) | `if (savings < 3) {` | branch-specific rejection/error | [ ] |
| 575 | `ZSTD_optimalBlockSize` (src/compress/zstd_compress.c:4574) | `if (splitLevel == 0) {` | branch-specific rejection/error | [ ] |
| 576 | `ZSTD_optimalBlockSize` (src/compress/zstd_compress.c:4575) | `assert(ZSTD_fast <= strat && strat <= ZSTD_btultra2);` | assertion failure | [ ] |
| 577 | `ZSTD_optimalBlockSize` (src/compress/zstd_compress.c:4578) | `assert(2 <= splitLevel && splitLevel <= 6);` | assertion failure | [ ] |
| 578 | `ZSTD_compress_frameChunk` (src/compress/zstd_compress.c:4604) | `assert(cctx->appliedParams.cParams.windowLog <= ZSTD_WINDOWLOG_MAX);` | assertion failure | [ ] |
| 579 | `ZSTD_compress_frameChunk` (src/compress/zstd_compress.c:4607) | `if (cctx->appliedParams.fParams.checksumFlag && srcSize)` | branch-specific rejection/error | [ ] |
| 580 | `ZSTD_compress_frameChunk` (src/compress/zstd_compress.c:4619) | `assert(blockSize <= remaining);` | `ERROR(1)` | [ ] |
| 581 | `ZSTD_compress_frameChunk` (src/compress/zstd_compress.c:4623) | `RETURN_ERROR_IF(dstCapacity < ZSTD_blockHeaderSize + MIN_CBLOCK_SIZE + 1,` | `ERROR(dstSize_tooSmall)` | [ ] |
| 582 | `ZSTD_compress_frameChunk` (src/compress/zstd_compress.c:4633) | `if (ms->nextToUpdate < ms->window.lowLimit) ms->nextToUpdate = ms->window.lowLimit;` | branch-specific rejection/error | [ ] |
| 583 | `ZSTD_compress_frameChunk` (src/compress/zstd_compress.c:4636) | `if (ZSTD_useTargetCBlockSize(&cctx->appliedParams)) {` | branch-specific rejection/error | [ ] |
| 584 | `ZSTD_compress_frameChunk` (src/compress/zstd_compress.c:4639) | `assert(cSize > 0);` | assertion failure | [ ] |
| 585 | `ZSTD_compress_frameChunk` (src/compress/zstd_compress.c:4640) | `assert(cSize <= blockSize + ZSTD_blockHeaderSize);` | assertion failure | [ ] |
| 586 | `ZSTD_compress_frameChunk` (src/compress/zstd_compress.c:4641) | `} else if (ZSTD_blockSplitterEnabled(&cctx->appliedParams)) {` | branch-specific rejection/error | [ ] |
| 587 | `ZSTD_compress_frameChunk` (src/compress/zstd_compress.c:4644) | `assert(cSize > 0 \|\| cctx->seqCollector.collectSequences == 1);` | assertion failure | [ ] |
| 588 | `ZSTD_compress_frameChunk` (src/compress/zstd_compress.c:4651) | `if (cSize == 0) {  /* block is not compressible */` | branch-specific rejection/error | [ ] |
| 589 | `ZSTD_compress_frameChunk` (src/compress/zstd_compress.c:4661) | `}  /* if (ZSTD_useTargetCBlockSize(&cctx->appliedParams))*/` | branch-specific rejection/error | [ ] |
| 590 | `ZSTD_compress_frameChunk` (src/compress/zstd_compress.c:4680) | `assert(remaining >= blockSize);` | assertion failure | [ ] |
| 591 | `ZSTD_compress_frameChunk` (src/compress/zstd_compress.c:4683) | `assert(dstCapacity >= cSize);` | assertion failure | [ ] |
| 592 | `ZSTD_compress_frameChunk` (src/compress/zstd_compress.c:4690) | `if (lastFrameChunk && (op>ostart)) cctx->stage = ZSTDcs_ending;` | branch-specific rejection/error | [ ] |
| 593 | `ZSTD_writeFrameHeader` (src/compress/zstd_compress.c:4711) | `assert(!(params->fParams.contentSizeFlag && pledgedSrcSize == ZSTD_CONTENTSIZE_UNKNOWN));` | `ERROR(dstSize_tooSmall)` | [ ] |
| 594 | `ZSTD_writeFrameHeader` (src/compress/zstd_compress.c:4712) | `RETURN_ERROR_IF(dstCapacity < ZSTD_FRAMEHEADERSIZE_MAX, dstSize_tooSmall,` | `ERROR(dstSize_tooSmall)` | [ ] |
| 595 | `ZSTD_writeFrameHeader` (src/compress/zstd_compress.c:4716) | `if (params->format == ZSTD_f_zstd1) {` | branch-specific rejection/error | [ ] |
| 596 | `ZSTD_writeFrameHeader` (src/compress/zstd_compress.c:4725) | `assert(0); /* impossible */` | assertion failure | [ ] |
| 597 | `ZSTD_writeFrameHeader` (src/compress/zstd_compress.c:4735) | `assert(0); /* impossible */` | assertion failure | [ ] |
| 598 | `ZSTD_writeSkippableFrame` (src/compress/zstd_compress.c:4754) | `RETURN_ERROR_IF(dstCapacity < srcSize + ZSTD_SKIPPABLEHEADERSIZE /* Skippable frame overhead */,` | `ERROR(dstSize_tooSmall)` | [ ] |
| 599 | `ZSTD_writeSkippableFrame` (src/compress/zstd_compress.c:4756) | `RETURN_ERROR_IF(srcSize > (unsigned)0xFFFFFFFF, srcSize_wrong, "Src size too large for skippable frame");` | `ERROR(d)` | [ ] |
| 600 | `ZSTD_writeSkippableFrame` (src/compress/zstd_compress.c:4757) | `RETURN_ERROR_IF(magicVariant > 15, parameter_outOfBound, "Skippable frame magic number variant not supported");` | `ERROR(parameter_outOfBound)` | [ ] |
| 601 | `ZSTD_writeLastEmptyBlock` (src/compress/zstd_compress.c:4772) | `RETURN_ERROR_IF(dstCapacity < ZSTD_blockHeaderSize, dstSize_tooSmall,` | `ERROR(dstSize_tooSmall)` | [ ] |
| 602 | `ZSTD_referenceExternalSequences` (src/compress/zstd_compress.c:4782) | `assert(cctx->stage == ZSTDcs_init);` | assertion failure | [ ] |
| 603 | `ZSTD_referenceExternalSequences` (src/compress/zstd_compress.c:4783) | `assert(nbSeq == 0 \|\| cctx->appliedParams.ldmParams.enableLdm != ZSTD_ps_enable);` | assertion failure | [ ] |
| 604 | `ZSTD_compressContinue_internal` (src/compress/zstd_compress.c:4802) | `RETURN_ERROR_IF(cctx->stage==ZSTDcs_created, stage_wrong,` | `ERROR(stage_wrong)` | [ ] |
| 605 | `ZSTD_compressContinue_internal` (src/compress/zstd_compress.c:4805) | `if (frame && (cctx->stage==ZSTDcs_init)) {` | branch-specific rejection/error | [ ] |
| 606 | `ZSTD_compressContinue_internal` (src/compress/zstd_compress.c:4809) | `assert(fhSize <= dstCapacity);` | assertion failure | [ ] |
| 607 | `ZSTD_compressContinue_internal` (src/compress/zstd_compress.c:4817) | `if (!ZSTD_window_update(&ms->window, src, srcSize, ms->forceNonContiguous)) {` | branch-specific rejection/error | [ ] |
| 608 | `ZSTD_compressContinue_internal` (src/compress/zstd_compress.c:4821) | `if (cctx->appliedParams.ldmParams.enableLdm == ZSTD_ps_enable) {` | branch-specific rejection/error | [ ] |
| 609 | `ZSTD_compressContinue_internal` (src/compress/zstd_compress.c:4839) | `assert(!(cctx->appliedParams.fParams.contentSizeFlag && cctx->pledgedSrcSizePlusOne == 0));` | `ERROR(e)` | [ ] |
| 610 | `ZSTD_compressContinue_internal` (src/compress/zstd_compress.c:4840) | `if (cctx->pledgedSrcSizePlusOne != 0) {  /* control src size */` | `ERROR(srcSize_wrong)` | [ ] |
| 611 | `ZSTD_compressContinue_internal` (src/compress/zstd_compress.c:4842) | `RETURN_ERROR_IF(` | `ERROR(srcSize_wrong)` | [ ] |
| 612 | `ZSTD_getBlockSize_deprecated` (src/compress/zstd_compress.c:4872) | `assert(!ZSTD_checkCParams(cParams));` | assertion failure | [ ] |
| 613 | `ZSTD_compressBlock_deprecated` (src/compress/zstd_compress.c:4887) | `RETURN_ERROR_IF(srcSize > blockSizeMax, srcSize_wrong, "input is larger than a block"); }` | `ERROR(srcSize_wrong)` | [ ] |
| 614 | `ZSTD_loadDictionaryContent` (src/compress/zstd_compress.c:4934) | `assert(!loadLdmDict);` | assertion failure | [ ] |
| 615 | `ZSTD_loadDictionaryContent` (src/compress/zstd_compress.c:4938) | `if (srcSize > maxDictSize) {` | branch-specific rejection/error | [ ] |
| 616 | `ZSTD_loadDictionaryContent` (src/compress/zstd_compress.c:4945) | `if (srcSize > ZSTD_CHUNKSIZE_MAX) {` | branch-specific rejection/error | [ ] |
| 617 | `ZSTD_loadDictionaryContent` (src/compress/zstd_compress.c:4947) | `assert(ZSTD_window_isEmpty(ms->window));` | assertion failure | [ ] |
| 618 | `ZSTD_loadDictionaryContent` (src/compress/zstd_compress.c:4948) | `if (loadLdmDict) assert(ZSTD_window_isEmpty(ls->window));` | assertion failure | [ ] |
| 619 | `ZSTD_loadDictionaryContent` (src/compress/zstd_compress.c:4964) | `if (srcSize > maxDictSize) {` | branch-specific rejection/error | [ ] |
| 620 | `ZSTD_loadDictionaryContent` (src/compress/zstd_compress.c:4975) | `if (srcSize <= HASH_READ_SIZE) return 0;` | branch-specific rejection/error | [ ] |
| 621 | `ZSTD_loadDictionaryContent` (src/compress/zstd_compress.c:4988) | `assert(0); /* shouldn't be called: cparams should've been adjusted. */` | assertion failure | [ ] |
| 622 | `ZSTD_loadDictionaryContent` (src/compress/zstd_compress.c:4998) | `assert(srcSize >= HASH_READ_SIZE);` | assertion failure | [ ] |
| 623 | `ZSTD_loadDictionaryContent` (src/compress/zstd_compress.c:4999) | `if (ms->dedicatedDictSearch) {` | branch-specific rejection/error | [ ] |
| 624 | `ZSTD_loadDictionaryContent` (src/compress/zstd_compress.c:5000) | `assert(ms->chainTable != NULL);` | assertion failure | [ ] |
| 625 | `ZSTD_loadDictionaryContent` (src/compress/zstd_compress.c:5003) | `assert(params->useRowMatchFinder != ZSTD_ps_auto);` | assertion failure | [ ] |
| 626 | `ZSTD_loadDictionaryContent` (src/compress/zstd_compress.c:5004) | `if (params->useRowMatchFinder == ZSTD_ps_enable) {` | branch-specific rejection/error | [ ] |
| 627 | `ZSTD_loadDictionaryContent` (src/compress/zstd_compress.c:5015) | `assert(0); /* shouldn't be called: cparams should've been adjusted. */` | assertion failure | [ ] |
| 628 | `ZSTD_loadDictionaryContent` (src/compress/zstd_compress.c:5026) | `assert(srcSize >= HASH_READ_SIZE);` | assertion failure | [ ] |
| 629 | `ZSTD_loadDictionaryContent` (src/compress/zstd_compress.c:5030) | `assert(0); /* shouldn't be called: cparams should've been adjusted. */` | assertion failure | [ ] |
| 630 | `ZSTD_loadDictionaryContent` (src/compress/zstd_compress.c:5035) | `assert(0);  /* not possible : not a valid strategy id */` | assertion failure | [ ] |
| 631 | `ZSTD_dictNCountRepeat` (src/compress/zstd_compress.c:5050) | `if (dictMaxSymbolValue < maxSymbolValue) {` | branch-specific rejection/error | [ ] |
| 632 | `ZSTD_dictNCountRepeat` (src/compress/zstd_compress.c:5054) | `if (normalizedCounter[s] == 0) {` | branch-specific rejection/error | [ ] |
| 633 | `ZSTD_loadCEntropy` (src/compress/zstd_compress.c:5081) | `RETURN_ERROR_IF(HUF_isError(hufHeaderSize), dictionary_corrupted, "");` | `ERROR(e)` | [ ] |
| 634 | `ZSTD_loadCEntropy` (src/compress/zstd_compress.c:5087) | `RETURN_ERROR_IF(FSE_isError(offcodeHeaderSize), dictionary_corrupted, "");` | `ERROR(e)` | [ ] |
| 635 | `ZSTD_loadCEntropy` (src/compress/zstd_compress.c:5088) | `RETURN_ERROR_IF(offcodeLog > OffFSELog, dictionary_corrupted, "");` | `ERROR(dictionary_corrupted)` | [ ] |
| 636 | `ZSTD_loadCEntropy` (src/compress/zstd_compress.c:5090) | `RETURN_ERROR_IF(FSE_isError(FSE_buildCTable_wksp(` | `ERROR(offcodeNCount)` | [ ] |
| 637 | `ZSTD_loadCEntropy` (src/compress/zstd_compress.c:5102) | `RETURN_ERROR_IF(FSE_isError(matchlengthHeaderSize), dictionary_corrupted, "");` | `ERROR(e)` | [ ] |
| 638 | `ZSTD_loadCEntropy` (src/compress/zstd_compress.c:5103) | `RETURN_ERROR_IF(matchlengthLog > MLFSELog, dictionary_corrupted, "");` | `ERROR(dictionary_corrupted)` | [ ] |
| 639 | `ZSTD_loadCEntropy` (src/compress/zstd_compress.c:5104) | `RETURN_ERROR_IF(FSE_isError(FSE_buildCTable_wksp(` | `ERROR(matchlengthNCount)` | [ ] |
| 640 | `ZSTD_loadCEntropy` (src/compress/zstd_compress.c:5116) | `RETURN_ERROR_IF(FSE_isError(litlengthHeaderSize), dictionary_corrupted, "");` | `ERROR(e)` | [ ] |
| 641 | `ZSTD_loadCEntropy` (src/compress/zstd_compress.c:5117) | `RETURN_ERROR_IF(litlengthLog > LLFSELog, dictionary_corrupted, "");` | `ERROR(dictionary_corrupted)` | [ ] |
| 642 | `ZSTD_loadCEntropy` (src/compress/zstd_compress.c:5118) | `RETURN_ERROR_IF(FSE_isError(FSE_buildCTable_wksp(` | `ERROR(litlengthNCount)` | [ ] |
| 643 | `ZSTD_loadCEntropy` (src/compress/zstd_compress.c:5127) | `RETURN_ERROR_IF(dictPtr+12 > dictEnd, dictionary_corrupted, "");` | `ERROR(dictionary_corrupted)` | [ ] |
| 644 | `ZSTD_loadCEntropy` (src/compress/zstd_compress.c:5135) | `if (dictContentSize <= ((U32)-1) - 128 KB) {` | branch-specific rejection/error | [ ] |
| 645 | `ZSTD_loadCEntropy` (src/compress/zstd_compress.c:5145) | `RETURN_ERROR_IF(bs->rep[u] == 0, dictionary_corrupted, "");` | `ERROR(dictionary_corrupted)` | [ ] |
| 646 | `ZSTD_loadCEntropy` (src/compress/zstd_compress.c:5146) | `RETURN_ERROR_IF(bs->rep[u] > dictContentSize, dictionary_corrupted, "");` | `ERROR(dictionary_corrupted)` | [ ] |
| 647 | `ZSTD_loadZstdDictionary` (src/compress/zstd_compress.c:5175) | `assert(dictSize >= 8);` | assertion failure | [ ] |
| 648 | `ZSTD_loadZstdDictionary` (src/compress/zstd_compress.c:5176) | `assert(MEM_readLE32(dictPtr) == ZSTD_MAGIC_DICTIONARY);` | assertion failure | [ ] |
| 649 | `ZSTD_compress_insertDictionary` (src/compress/zstd_compress.c:5206) | `if ((dict==NULL) \|\| (dictSize<8)) {` | `ERROR(dictionary_wrong)` | [ ] |
| 650 | `ZSTD_compress_insertDictionary` (src/compress/zstd_compress.c:5207) | `RETURN_ERROR_IF(dictContentType == ZSTD_dct_fullDict, dictionary_wrong, "");` | `ERROR(dictionary_wrong)` | [ ] |
| 651 | `ZSTD_compress_insertDictionary` (src/compress/zstd_compress.c:5223) | `RETURN_ERROR_IF(dictContentType == ZSTD_dct_fullDict, dictionary_wrong, "");` | `ERROR(dictionary_wrong)` | [ ] |
| 652 | `ZSTD_compress_insertDictionary` (src/compress/zstd_compress.c:5224) | `assert(0);   /* impossible */` | assertion failure | [ ] |
| 653 | `ZSTD_compressBegin_internal` (src/compress/zstd_compress.c:5252) | `assert(!ZSTD_isError(ZSTD_checkCParams(params->cParams)));` | assertion failure | [ ] |
| 654 | `ZSTD_compressBegin_internal` (src/compress/zstd_compress.c:5253) | `assert(!((dict) && (cdict)));  /* either dict or cdict, not both */` | assertion failure | [ ] |
| 655 | `ZSTD_compressBegin_internal` (src/compress/zstd_compress.c:5278) | `assert(dictID <= UINT_MAX);` | assertion failure | [ ] |
| 656 | `ZSTD_writeEpilogue` (src/compress/zstd_compress.c:5350) | `RETURN_ERROR_IF(cctx->stage == ZSTDcs_created, stage_wrong, "init missing");` | `ERROR(stage_wrong)` | [ ] |
| 657 | `ZSTD_writeEpilogue` (src/compress/zstd_compress.c:5353) | `if (cctx->stage == ZSTDcs_init) {` | branch-specific rejection/error | [ ] |
| 658 | `ZSTD_writeEpilogue` (src/compress/zstd_compress.c:5361) | `if (cctx->stage != ZSTDcs_ending) {` | `ERROR(dstSize_tooSmall)` | [ ] |
| 659 | `ZSTD_writeEpilogue` (src/compress/zstd_compress.c:5365) | `RETURN_ERROR_IF(dstCapacity<3, dstSize_tooSmall, "no room for epilogue");` | `ERROR(dstSize_tooSmall)` | [ ] |
| 660 | `ZSTD_writeEpilogue` (src/compress/zstd_compress.c:5371) | `if (cctx->appliedParams.fParams.checksumFlag) {` | `ERROR(dstSize_tooSmall)` | [ ] |
| 661 | `ZSTD_writeEpilogue` (src/compress/zstd_compress.c:5373) | `RETURN_ERROR_IF(dstCapacity<4, dstSize_tooSmall, "no room for checksum");` | `ERROR(dstSize_tooSmall)` | [ ] |
| 662 | `ZSTD_CCtx_trace` (src/compress/zstd_compress.c:5386) | `if (cctx->traceCtx && ZSTD_trace_compress_end != NULL) {` | branch-specific rejection/error | [ ] |
| 663 | `ZSTD_compressEnd_public` (src/compress/zstd_compress.c:5418) | `assert(!(cctx->appliedParams.fParams.contentSizeFlag && cctx->pledgedSrcSizePlusOne == 0));` | assertion failure | [ ] |
| 664 | `ZSTD_compressEnd_public` (src/compress/zstd_compress.c:5419) | `if (cctx->pledgedSrcSizePlusOne != 0) {  /* control src size */` | `ERROR(1)` | [ ] |
| 665 | `ZSTD_compressEnd_public` (src/compress/zstd_compress.c:5422) | `RETURN_ERROR_IF(` | `ERROR(srcSize_wrong)` | [ ] |
| 666 | `ZSTD_compress_usingDict` (src/compress/zstd_compress.c:5480) | `assert(params.fParams.contentSizeFlag == 1);` | assertion failure | [ ] |
| 667 | `ZSTD_compressCCtx` (src/compress/zstd_compress.c:5493) | `assert(cctx != NULL);` | assertion failure | [ ] |
| 668 | `ZSTD_compress` (src/compress/zstd_compress.c:5504) | `RETURN_ERROR_IF(!cctx, memory_allocation, "ZSTD_createCCtx failed");` | `ERROR(memory_allocation)` | [ ] |
| 669 | `ZSTD_sizeof_CDict` (src/compress/zstd_compress.c:5544) | `if (cdict==NULL) return 0;   /* support sizeof on NULL */` | branch-specific rejection/error | [ ] |
| 670 | `ZSTD_initCDict_internal` (src/compress/zstd_compress.c:5559) | `assert(!ZSTD_checkCParams(params.cParams));` | assertion failure | [ ] |
| 671 | `ZSTD_initCDict_internal` (src/compress/zstd_compress.c:5566) | `RETURN_ERROR_IF(!internalBuffer, memory_allocation, "NULL pointer!");` | `ERROR(memory_allocation)` | [ ] |
| 672 | `ZSTD_initCDict_internal` (src/compress/zstd_compress.c:5596) | `assert(dictID <= (size_t)(U32)-1);` | assertion failure | [ ] |
| 673 | `ZSTD_createCDict_advanced_internal` (src/compress/zstd_compress.c:5612) | `if ((!customMem.customAlloc) ^ (!customMem.customFree)) return NULL;` | `NULL` | [ ] |
| 674 | `ZSTD_createCDict_advanced_internal` (src/compress/zstd_compress.c:5627) | `return NULL;` | `NULL` | [ ] |
| 675 | `ZSTD_createCDict_advanced_internal` (src/compress/zstd_compress.c:5633) | `assert(cdict != NULL);` | assertion failure | [ ] |
| 676 | `ZSTD_createCDict_advanced2` (src/compress/zstd_compress.c:5672) | `if (!customMem.customAlloc ^ !customMem.customFree) return NULL;` | `NULL` | [ ] |
| 677 | `ZSTD_createCDict_advanced2` (src/compress/zstd_compress.c:5704) | `return NULL;` | `NULL` | [ ] |
| 678 | `ZSTD_freeCDict` (src/compress/zstd_compress.c:5734) | `if (cdict==NULL) return 0;   /* support free on NULL */` | branch-specific rejection/error | [ ] |
| 679 | `ZSTD_initStaticCDict` (src/compress/zstd_compress.c:5777) | `if ((size_t)workspace & 7) return NULL;  /* 8-aligned */` | `NULL` | [ ] |
| 680 | `ZSTD_initStaticCDict` (src/compress/zstd_compress.c:5783) | `if (cdict == NULL) return NULL;` | `NULL` | [ ] |
| 681 | `ZSTD_initStaticCDict` (src/compress/zstd_compress.c:5787) | `if (workspaceSize < neededSize) return NULL;` | `NULL` | [ ] |
| 682 | `ZSTD_initStaticCDict` (src/compress/zstd_compress.c:5799) | `return NULL;` | `NULL` | [ ] |
| 683 | `ZSTD_getCParamsFromCDict` (src/compress/zstd_compress.c:5806) | `assert(cdict != NULL);` | assertion failure | [ ] |
| 684 | `ZSTD_getDictID_fromCDict` (src/compress/zstd_compress.c:5816) | `if (cdict==NULL) return 0;` | branch-specific rejection/error | [ ] |
| 685 | `ZSTD_compressBegin_usingCDict_internal` (src/compress/zstd_compress.c:5829) | `RETURN_ERROR_IF(cdict==NULL, dictionary_wrong, "NULL pointer!");` | `ERROR(dictionary_wrong)` | [ ] |
| 686 | `ZSTD_getCParamMode` (src/compress/zstd_compress.c:5961) | `if (cdict != NULL && ZSTD_shouldAttachDict(cdict, params, pledgedSrcSize))` | branch-specific rejection/error | [ ] |
| 687 | `ZSTD_initCStream_internal` (src/compress/zstd_compress.c:5994) | `assert(!ZSTD_isError(ZSTD_checkCParams(params->cParams)));` | assertion failure | [ ] |
| 688 | `ZSTD_initCStream_internal` (src/compress/zstd_compress.c:5996) | `assert(!((dict) && (cdict)));  /* either dict or cdict, not both */` | assertion failure | [ ] |
| 689 | `ZSTD_nextInputSizeHint` (src/compress/zstd_compress.c:6090) | `if (cctx->appliedParams.inBufferMode == ZSTD_bm_stable) {` | branch-specific rejection/error | [ ] |
| 690 | `ZSTD_nextInputSizeHint` (src/compress/zstd_compress.c:6093) | `assert(cctx->appliedParams.inBufferMode == ZSTD_bm_buffered);` | assertion failure | [ ] |
| 691 | `ZSTD_nextInputSizeHint` (src/compress/zstd_compress.c:6095) | `if (hintInSize==0) hintInSize = cctx->blockSizeMax;` | branch-specific rejection/error | [ ] |
| 692 | `ZSTD_compressStream_generic` (src/compress/zstd_compress.c:6108) | `const char* const istart = (assert(input != NULL), (const char*)input->src);` | assertion failure | [ ] |
| 693 | `ZSTD_compressStream_generic` (src/compress/zstd_compress.c:6111) | `char* const ostart = (assert(output != NULL), (char*)output->dst);` | assertion failure | [ ] |
| 694 | `ZSTD_compressStream_generic` (src/compress/zstd_compress.c:6118) | `assert(zcs != NULL);` | assertion failure | [ ] |
| 695 | `ZSTD_compressStream_generic` (src/compress/zstd_compress.c:6119) | `if (zcs->appliedParams.inBufferMode == ZSTD_bm_stable) {` | branch-specific rejection/error | [ ] |
| 696 | `ZSTD_compressStream_generic` (src/compress/zstd_compress.c:6120) | `assert(input->pos >= zcs->stableIn_notConsumed);` | assertion failure | [ ] |
| 697 | `ZSTD_compressStream_generic` (src/compress/zstd_compress.c:6122) | `if (ip) ip -= zcs->stableIn_notConsumed;` | branch-specific rejection/error | [ ] |
| 698 | `ZSTD_compressStream_generic` (src/compress/zstd_compress.c:6125) | `if (zcs->appliedParams.inBufferMode == ZSTD_bm_buffered) {` | branch-specific rejection/error | [ ] |
| 699 | `ZSTD_compressStream_generic` (src/compress/zstd_compress.c:6126) | `assert(zcs->inBuff != NULL);` | assertion failure | [ ] |
| 700 | `ZSTD_compressStream_generic` (src/compress/zstd_compress.c:6127) | `assert(zcs->inBuffSize > 0);` | assertion failure | [ ] |
| 701 | `ZSTD_compressStream_generic` (src/compress/zstd_compress.c:6129) | `if (zcs->appliedParams.outBufferMode == ZSTD_bm_buffered) {` | branch-specific rejection/error | [ ] |
| 702 | `ZSTD_compressStream_generic` (src/compress/zstd_compress.c:6130) | `assert(zcs->outBuff !=  NULL);` | assertion failure | [ ] |
| 703 | `ZSTD_compressStream_generic` (src/compress/zstd_compress.c:6131) | `assert(zcs->outBuffSize > 0);` | assertion failure | [ ] |
| 704 | `ZSTD_compressStream_generic` (src/compress/zstd_compress.c:6133) | `if (input->src == NULL) assert(input->size == 0);` | assertion failure | [ ] |
| 705 | `ZSTD_compressStream_generic` (src/compress/zstd_compress.c:6134) | `assert(input->pos <= input->size);` | assertion failure | [ ] |
| 706 | `ZSTD_compressStream_generic` (src/compress/zstd_compress.c:6135) | `if (output->dst == NULL) assert(output->size == 0);` | assertion failure | [ ] |
| 707 | `ZSTD_compressStream_generic` (src/compress/zstd_compress.c:6136) | `assert(output->pos <= output->size);` | assertion failure | [ ] |
| 708 | `ZSTD_compressStream_generic` (src/compress/zstd_compress.c:6137) | `assert((U32)flushMode <= (U32)ZSTD_e_end);` | assertion failure | [ ] |
| 709 | `ZSTD_compressStream_generic` (src/compress/zstd_compress.c:6143) | `RETURN_ERROR(init_missing, "call ZSTD_initCStream() first!");` | `ERROR(g)` | [ ] |
| 710 | `ZSTD_compressStream_generic` (src/compress/zstd_compress.c:6163) | `if (zcs->appliedParams.inBufferMode == ZSTD_bm_buffered) {` | branch-specific rejection/error | [ ] |
| 711 | `ZSTD_compressStream_generic` (src/compress/zstd_compress.c:6181) | `assert(zcs->appliedParams.inBufferMode == ZSTD_bm_stable);` | assertion failure | [ ] |
| 712 | `ZSTD_compressStream_generic` (src/compress/zstd_compress.c:6203) | `if (oSize >= ZSTD_compressBound(iSize) \|\| zcs->appliedParams.outBufferMode == ZSTD_bm_stable)` | branch-specific rejection/error | [ ] |
| 713 | `ZSTD_compressStream_generic` (src/compress/zstd_compress.c:6218) | `if (zcs->inBuffTarget > zcs->inBuffSize)` | branch-specific rejection/error | [ ] |
| 714 | `ZSTD_compressStream_generic` (src/compress/zstd_compress.c:6223) | `assert(zcs->inBuffTarget <= zcs->inBuffSize);` | assertion failure | [ ] |
| 715 | `ZSTD_compressStream_generic` (src/compress/zstd_compress.c:6234) | `if (lastBlock) assert(ip == iend);` | assertion failure | [ ] |
| 716 | `ZSTD_compressStream_generic` (src/compress/zstd_compress.c:6238) | `if (zcs->frameEnded) {` | branch-specific rejection/error | [ ] |
| 717 | `ZSTD_compressStream_generic` (src/compress/zstd_compress.c:6252) | `assert(zcs->appliedParams.outBufferMode == ZSTD_bm_buffered);` | assertion failure | [ ] |
| 718 | `ZSTD_compressStream_generic` (src/compress/zstd_compress.c:6263) | `assert(op==oend);` | assertion failure | [ ] |
| 719 | `ZSTD_compressStream_generic` (src/compress/zstd_compress.c:6268) | `if (zcs->frameEnded) {` | branch-specific rejection/error | [ ] |
| 720 | `ZSTD_compressStream_generic` (src/compress/zstd_compress.c:6279) | `assert(0);` | assertion failure | [ ] |
| 721 | `ZSTD_compressStream_generic` (src/compress/zstd_compress.c:6285) | `if (zcs->frameEnded) return 0;` | branch-specific rejection/error | [ ] |
| 722 | `ZSTD_nextInputSizeHint_MTorST` (src/compress/zstd_compress.c:6292) | `if (cctx->appliedParams.nbWorkers >= 1) {` | branch-specific rejection/error | [ ] |
| 723 | `ZSTD_nextInputSizeHint_MTorST` (src/compress/zstd_compress.c:6293) | `assert(cctx->mtctx != NULL);` | assertion failure | [ ] |
| 724 | `ZSTD_setBufferExpectations` (src/compress/zstd_compress.c:6314) | `if (cctx->appliedParams.inBufferMode == ZSTD_bm_stable) {` | branch-specific rejection/error | [ ] |
| 725 | `ZSTD_setBufferExpectations` (src/compress/zstd_compress.c:6317) | `if (cctx->appliedParams.outBufferMode == ZSTD_bm_stable) {` | branch-specific rejection/error | [ ] |
| 726 | `ZSTD_checkBufferStability` (src/compress/zstd_compress.c:6330) | `if (cctx->appliedParams.inBufferMode == ZSTD_bm_stable) {` | `ERROR(d)` | [ ] |
| 727 | `ZSTD_checkBufferStability` (src/compress/zstd_compress.c:6332) | `if (expect.src != input->src \|\| expect.pos != input->pos)` | `ERROR(d)` | [ ] |
| 728 | `ZSTD_checkBufferStability` (src/compress/zstd_compress.c:6333) | `RETURN_ERROR(stabilityCondition_notRespected, "ZSTD_c_stableInBuffer enabled but input differs!");` | `ERROR(d)` | [ ] |
| 729 | `ZSTD_checkBufferStability` (src/compress/zstd_compress.c:6336) | `if (cctx->appliedParams.outBufferMode == ZSTD_bm_stable) {` | `ERROR(d)` | [ ] |
| 730 | `ZSTD_checkBufferStability` (src/compress/zstd_compress.c:6338) | `if (cctx->expectedOutBufferSize != outBufferSize)` | `ERROR(d)` | [ ] |
| 731 | `ZSTD_checkBufferStability` (src/compress/zstd_compress.c:6339) | `RETURN_ERROR(stabilityCondition_notRespected, "ZSTD_c_stableOutBuffer enabled but output size differs!");` | `ERROR(d)` | [ ] |
| 732 | `ZSTD_CCtx_init_compressStream2` (src/compress/zstd_compress.c:6357) | `assert(prefixDict.dict==NULL \|\| cctx->cdict==NULL);    /* only one can be set */` | assertion failure | [ ] |
| 733 | `ZSTD_CCtx_init_compressStream2` (src/compress/zstd_compress.c:6358) | `if (cctx->cdict && !cctx->localDict.cdict) {` | branch-specific rejection/error | [ ] |
| 734 | `ZSTD_CCtx_init_compressStream2` (src/compress/zstd_compress.c:6366) | `if (endOp == ZSTD_e_end) cctx->pledgedSrcSizePlusOne = inSize + 1;  /* auto-determine pledgedSrcSize */` | branch-specific rejection/error | [ ] |
| 735 | `ZSTD_CCtx_init_compressStream2` (src/compress/zstd_compress.c:6386) | `RETURN_ERROR_IF(` | `ERROR(s)` | [ ] |
| 736 | `ZSTD_CCtx_init_compressStream2` (src/compress/zstd_compress.c:6392) | `if ((cctx->pledgedSrcSizePlusOne-1) <= ZSTDMT_JOBSIZE_MIN) {` | branch-specific rejection/error | [ ] |
| 737 | `ZSTD_CCtx_init_compressStream2` (src/compress/zstd_compress.c:6395) | `if (params.nbWorkers > 0) {` | branch-specific rejection/error | [ ] |
| 738 | `ZSTD_CCtx_init_compressStream2` (src/compress/zstd_compress.c:6400) | `if (cctx->mtctx == NULL) {` | `ERROR(memory_allocation)` | [ ] |
| 739 | `ZSTD_CCtx_init_compressStream2` (src/compress/zstd_compress.c:6404) | `RETURN_ERROR_IF(cctx->mtctx == NULL, memory_allocation, "NULL pointer!");` | `ERROR(memory_allocation)` | [ ] |
| 740 | `ZSTD_CCtx_init_compressStream2` (src/compress/zstd_compress.c:6421) | `assert(!ZSTD_isError(ZSTD_checkCParams(params.cParams)));` | assertion failure | [ ] |
| 741 | `ZSTD_CCtx_init_compressStream2` (src/compress/zstd_compress.c:6427) | `assert(cctx->appliedParams.nbWorkers == 0);` | assertion failure | [ ] |
| 742 | `ZSTD_CCtx_init_compressStream2` (src/compress/zstd_compress.c:6430) | `if (cctx->appliedParams.inBufferMode == ZSTD_bm_buffered) {` | branch-specific rejection/error | [ ] |
| 743 | `ZSTD_compressStream2` (src/compress/zstd_compress.c:6454) | `RETURN_ERROR_IF(output->pos > output->size, dstSize_tooSmall, "invalid output buffer");` | `ERROR(dstSize_tooSmall)` | [ ] |
| 744 | `ZSTD_compressStream2` (src/compress/zstd_compress.c:6455) | `RETURN_ERROR_IF(input->pos  > input->size, srcSize_wrong, "invalid input buffer");` | `ERROR(srcSize_wrong)` | [ ] |
| 745 | `ZSTD_compressStream2` (src/compress/zstd_compress.c:6456) | `RETURN_ERROR_IF((U32)endOp > (U32)ZSTD_e_end, parameter_outOfBound, "invalid endDirective");` | `ERROR(2)` | [ ] |
| 746 | `ZSTD_compressStream2` (src/compress/zstd_compress.c:6457) | `assert(cctx != NULL);` | assertion failure | [ ] |
| 747 | `ZSTD_compressStream2` (src/compress/zstd_compress.c:6460) | `if (cctx->streamStage == zcss_init) {` | branch-specific rejection/error | [ ] |
| 748 | `ZSTD_compressStream2` (src/compress/zstd_compress.c:6463) | `if ( (cctx->requestedParams.inBufferMode == ZSTD_bm_stable) /* input is presumed stable, across invocations */` | branch-specific rejection/error | [ ] |
| 749 | `ZSTD_compressStream2` (src/compress/zstd_compress.c:6466) | `if (cctx->stableIn_notConsumed) {  /* not the first time */` | `ERROR(stabilityCondition_notRespected)` | [ ] |
| 750 | `ZSTD_compressStream2` (src/compress/zstd_compress.c:6468) | `RETURN_ERROR_IF(input->src != cctx->expectedInBuffer.src, stabilityCondition_notRespected, "stableInBuffer condition not respected: wrong src pointer");` | `ERROR(stabilityCondition_notRespected)` | [ ] |
| 751 | `ZSTD_compressStream2` (src/compress/zstd_compress.c:6469) | `RETURN_ERROR_IF(input->pos != cctx->expectedInBuffer.size, stabilityCondition_notRespected, "stableInBuffer condition not respected: externally modified pos");` | `ERROR(stabilityCondition_notRespected)` | [ ] |
| 752 | `ZSTD_compressStream2` (src/compress/zstd_compress.c:6488) | `if (cctx->appliedParams.nbWorkers > 0) {` | branch-specific rejection/error | [ ] |
| 753 | `ZSTD_compressStream2` (src/compress/zstd_compress.c:6490) | `if (cctx->cParamsChanged) {` | branch-specific rejection/error | [ ] |
| 754 | `ZSTD_compressStream2` (src/compress/zstd_compress.c:6494) | `if (cctx->stableIn_notConsumed) {` | branch-specific rejection/error | [ ] |
| 755 | `ZSTD_compressStream2` (src/compress/zstd_compress.c:6495) | `assert(cctx->appliedParams.inBufferMode == ZSTD_bm_stable);` | assertion failure | [ ] |
| 756 | `ZSTD_compressStream2` (src/compress/zstd_compress.c:6497) | `assert(input->pos >= cctx->stableIn_notConsumed);` | assertion failure | [ ] |
| 757 | `ZSTD_compressStream2` (src/compress/zstd_compress.c:6509) | `if (flushMin == 0)` | branch-specific rejection/error | [ ] |
| 758 | `ZSTD_compressStream2` (src/compress/zstd_compress.c:6520) | `if (input->pos != ipos \|\| output->pos != opos \|\| input->pos == input->size \|\| output->pos == output->size)` | branch-specific rejection/error | [ ] |
| 759 | `ZSTD_compressStream2` (src/compress/zstd_compress.c:6523) | `assert(endOp == ZSTD_e_flush \|\| endOp == ZSTD_e_end);` | assertion failure | [ ] |
| 760 | `ZSTD_compressStream2` (src/compress/zstd_compress.c:6527) | `if (flushMin == 0 \|\| output->pos == output->size)` | branch-specific rejection/error | [ ] |
| 761 | `ZSTD_compressStream2` (src/compress/zstd_compress.c:6535) | `assert(endOp == ZSTD_e_continue \|\| flushMin == 0 \|\| output->pos == output->size);` | assertion failure | [ ] |
| 762 | `ZSTD_compress2` (src/compress/zstd_compress.c:6590) | `if (result != 0) {  /* compression not completed, due to lack of output space */` | `ERROR(l)` | [ ] |
| 763 | `ZSTD_compress2` (src/compress/zstd_compress.c:6591) | `assert(oPos == dstCapacity);` | `ERROR(l)` | [ ] |
| 764 | `ZSTD_compress2` (src/compress/zstd_compress.c:6592) | `RETURN_ERROR(dstSize_tooSmall, "");` | `ERROR(l)` | [ ] |
| 765 | `ZSTD_compress2` (src/compress/zstd_compress.c:6594) | `assert(iPos == srcSize);   /* all input is expected consumed */` | assertion failure | [ ] |
| 766 | `ZSTD_validateSequence` (src/compress/zstd_compress.c:6615) | `RETURN_ERROR_IF(offBase > OFFSET_TO_OFFBASE(offsetBound), externalSequences_invalid, "Offset too large!");` | `ERROR(d)` | [ ] |
| 767 | `ZSTD_validateSequence` (src/compress/zstd_compress.c:6617) | `RETURN_ERROR_IF(matchLength < matchLenLowerBound, externalSequences_invalid, "Matchlength too small for the minMatch");` | `ERROR(externalSequences_invalid)` | [ ] |
| 768 | `ZSTD_transferSequences_wBlockDelim` (src/compress/zstd_compress.c:6660) | `if (cctx->cdict) {` | branch-specific rejection/error | [ ] |
| 769 | `ZSTD_transferSequences_wBlockDelim` (src/compress/zstd_compress.c:6662) | `} else if (cctx->prefixDict.dict) {` | branch-specific rejection/error | [ ] |
| 770 | `ZSTD_transferSequences_wBlockDelim` (src/compress/zstd_compress.c:6682) | `if (cctx->appliedParams.validateSequences) {` | branch-specific rejection/error | [ ] |
| 771 | `ZSTD_transferSequences_wBlockDelim` (src/compress/zstd_compress.c:6690) | `RETURN_ERROR_IF(idx - seqPos->idx >= cctx->seqStore.maxNbSeq, externalSequences_invalid,` | `ERROR(externalSequences_invalid)` | [ ] |
| 772 | `ZSTD_transferSequences_wBlockDelim` (src/compress/zstd_compress.c:6695) | `RETURN_ERROR_IF(idx == inSeqsSize, externalSequences_invalid, "Block delimiter not found.");` | `ERROR(externalSequences_invalid)` | [ ] |
| 773 | `ZSTD_transferSequences_wBlockDelim` (src/compress/zstd_compress.c:6698) | `assert(externalRepSearch != ZSTD_ps_auto);` | assertion failure | [ ] |
| 774 | `ZSTD_transferSequences_wBlockDelim` (src/compress/zstd_compress.c:6699) | `assert(idx >= startIdx);` | assertion failure | [ ] |
| 775 | `ZSTD_transferSequences_wBlockDelim` (src/compress/zstd_compress.c:6704) | `if (lastSeqIdx >= startIdx + 2) {` | branch-specific rejection/error | [ ] |
| 776 | `ZSTD_transferSequences_wBlockDelim` (src/compress/zstd_compress.c:6713) | `assert(lastSeqIdx == startIdx);` | assertion failure | [ ] |
| 777 | `ZSTD_transferSequences_wBlockDelim` (src/compress/zstd_compress.c:6728) | `RETURN_ERROR_IF(ip != iend, externalSequences_invalid, "Blocksize doesn't agree with block delimiter!");` | `ERROR(externalSequences_invalid)` | [ ] |
| 778 | `ZSTD_transferSequences_noDelim` (src/compress/zstd_compress.c:6765) | `if (cctx->cdict) {` | branch-specific rejection/error | [ ] |
| 779 | `ZSTD_transferSequences_noDelim` (src/compress/zstd_compress.c:6767) | `} else if (cctx->prefixDict.dict) {` | branch-specific rejection/error | [ ] |
| 780 | `ZSTD_transferSequences_noDelim` (src/compress/zstd_compress.c:6783) | `if (endPosInSequence >= currSeq.litLength + currSeq.matchLength) {` | branch-specific rejection/error | [ ] |
| 781 | `ZSTD_transferSequences_noDelim` (src/compress/zstd_compress.c:6784) | `if (startPosInSequence >= litLength) {` | branch-specific rejection/error | [ ] |
| 782 | `ZSTD_transferSequences_noDelim` (src/compress/zstd_compress.c:6799) | `if (endPosInSequence > litLength) {` | branch-specific rejection/error | [ ] |
| 783 | `ZSTD_transferSequences_noDelim` (src/compress/zstd_compress.c:6803) | `if (matchLength > blockSize && firstHalfMatchLength >= cctx->appliedParams.cParams.minMatch) {` | branch-specific rejection/error | [ ] |
| 784 | `ZSTD_transferSequences_noDelim` (src/compress/zstd_compress.c:6806) | `if (secondHalfMatchLength < cctx->appliedParams.cParams.minMatch) {` | branch-specific rejection/error | [ ] |
| 785 | `ZSTD_transferSequences_noDelim` (src/compress/zstd_compress.c:6837) | `if (cctx->appliedParams.validateSequences) {` | branch-specific rejection/error | [ ] |
| 786 | `ZSTD_transferSequences_noDelim` (src/compress/zstd_compress.c:6844) | `RETURN_ERROR_IF(idx - seqPos->idx >= cctx->seqStore.maxNbSeq, externalSequences_invalid,` | `ERROR(externalSequences_invalid)` | [ ] |
| 787 | `ZSTD_transferSequences_noDelim` (src/compress/zstd_compress.c:6852) | `assert(idx == inSeqsSize \|\| endPosInSequence <= inSeqs[idx].litLength + inSeqs[idx].matchLength);` | assertion failure | [ ] |
| 788 | `ZSTD_transferSequences_noDelim` (src/compress/zstd_compress.c:6861) | `assert(ip <= iend);` | assertion failure | [ ] |
| 789 | `ZSTD_selectSequenceCopier` (src/compress/zstd_compress.c:6883) | `assert(ZSTD_cParam_withinBounds(ZSTD_c_blockDelimiters, (int)mode));` | assertion failure | [ ] |
| 790 | `ZSTD_selectSequenceCopier` (src/compress/zstd_compress.c:6887) | `assert(mode == ZSTD_sf_noBlockDelimiters);` | assertion failure | [ ] |
| 791 | `blockSize_explicitDelimiter` (src/compress/zstd_compress.c:6902) | `assert(spos <= inSeqsSize);` | assertion failure | [ ] |
| 792 | `blockSize_explicitDelimiter` (src/compress/zstd_compress.c:6907) | `if (inSeqs[spos].matchLength != 0)` | `ERROR(d)` | [ ] |
| 793 | `blockSize_explicitDelimiter` (src/compress/zstd_compress.c:6908) | `RETURN_ERROR(externalSequences_invalid, "delimiter format error : both matchlength and offset must be == 0");` | `ERROR(d)` | [ ] |
| 794 | `blockSize_explicitDelimiter` (src/compress/zstd_compress.c:6914) | `RETURN_ERROR(externalSequences_invalid, "Reached end of sequences without finding a block delimiter");` | `ERROR(d)` | [ ] |
| 795 | `determine_blockSize` (src/compress/zstd_compress.c:6928) | `assert(mode == ZSTD_sf_explicitBlockDelimiters);` | `ERROR(d)` | [ ] |
| 796 | `determine_blockSize` (src/compress/zstd_compress.c:6931) | `if (explicitBlockSize > blockSize)` | `ERROR(d)` | [ ] |
| 797 | `determine_blockSize` (src/compress/zstd_compress.c:6932) | `RETURN_ERROR(externalSequences_invalid, "sequences incorrectly define a too large block");` | `ERROR(d)` | [ ] |
| 798 | `determine_blockSize` (src/compress/zstd_compress.c:6933) | `if (explicitBlockSize > remaining)` | `ERROR(d)` | [ ] |
| 799 | `determine_blockSize` (src/compress/zstd_compress.c:6934) | `RETURN_ERROR(externalSequences_invalid, "sequences define a frame longer than source");` | `ERROR(d)` | [ ] |
| 800 | `ZSTD_compressSequences_internal` (src/compress/zstd_compress.c:6960) | `if (remaining == 0) {` | `ERROR(dstSize_tooSmall)` | [ ] |
| 801 | `ZSTD_compressSequences_internal` (src/compress/zstd_compress.c:6962) | `RETURN_ERROR_IF(dstCapacity<4, dstSize_tooSmall, "No room for empty frame block header");` | `ERROR(dstSize_tooSmall)` | [ ] |
| 802 | `ZSTD_compressSequences_internal` (src/compress/zstd_compress.c:6977) | `assert(blockSize <= remaining);` | assertion failure | [ ] |
| 803 | `ZSTD_compressSequences_internal` (src/compress/zstd_compress.c:6989) | `if (blockSize < MIN_CBLOCK_SIZE+ZSTD_blockHeaderSize+1+1) {` | branch-specific rejection/error | [ ] |
| 804 | `ZSTD_compressSequences_internal` (src/compress/zstd_compress.c:7001) | `RETURN_ERROR_IF(dstCapacity < ZSTD_blockHeaderSize, dstSize_tooSmall, "not enough dstCapacity to write a new compressed block");` | `ERROR(dstSize_tooSmall)` | [ ] |
| 805 | `ZSTD_compressSequences_internal` (src/compress/zstd_compress.c:7012) | `if (!cctx->isFirstBlock &&` | branch-specific rejection/error | [ ] |
| 806 | `ZSTD_compressSequences_internal` (src/compress/zstd_compress.c:7022) | `if (compressedSeqsSize == 0) {` | branch-specific rejection/error | [ ] |
| 807 | `ZSTD_compressSequences_internal` (src/compress/zstd_compress.c:7035) | `if (cctx->blockState.prevCBlock->entropy.fse.offcode_repeatMode == FSE_repeat_valid)` | branch-specific rejection/error | [ ] |
| 808 | `ZSTD_compressSequences` (src/compress/zstd_compress.c:7073) | `assert(cctx != NULL);` | assertion failure | [ ] |
| 809 | `ZSTD_compressSequences` (src/compress/zstd_compress.c:7080) | `assert(frameHeaderSize <= dstCapacity);` | assertion failure | [ ] |
| 810 | `ZSTD_compressSequences` (src/compress/zstd_compress.c:7084) | `if (cctx->appliedParams.fParams.checksumFlag && srcSize) {` | branch-specific rejection/error | [ ] |
| 811 | `ZSTD_compressSequences` (src/compress/zstd_compress.c:7095) | `assert(cBlocksSize <= dstCapacity);` | assertion failure | [ ] |
| 812 | `ZSTD_compressSequences` (src/compress/zstd_compress.c:7100) | `if (cctx->appliedParams.fParams.checksumFlag) {` | `ERROR(dstSize_tooSmall)` | [ ] |
| 813 | `ZSTD_compressSequences` (src/compress/zstd_compress.c:7102) | `RETURN_ERROR_IF(dstCapacity<4, dstSize_tooSmall, "no room for checksum");` | `ERROR(dstSize_tooSmall)` | [ ] |
| 814 | `convertSequences_noRepcodes` (src/compress/zstd_compress.c:7237) | `if (UNLIKELY((ll_res & 0x0FF00FF0) != 0)) {` | branch-specific rejection/error | [ ] |
| 815 | `convertSequences_noRepcodes` (src/compress/zstd_compress.c:7239) | `if (inSeqs[i].matchLength > 65535+MINMATCH) {` | branch-specific rejection/error | [ ] |
| 816 | `convertSequences_noRepcodes` (src/compress/zstd_compress.c:7240) | `assert(longLen == 0);` | assertion failure | [ ] |
| 817 | `convertSequences_noRepcodes` (src/compress/zstd_compress.c:7243) | `if (inSeqs[i].litLength > 65535) {` | branch-specific rejection/error | [ ] |
| 818 | `convertSequences_noRepcodes` (src/compress/zstd_compress.c:7244) | `assert(longLen == 0);` | assertion failure | [ ] |
| 819 | `convertSequences_noRepcodes` (src/compress/zstd_compress.c:7247) | `if (inSeqs[i+1].matchLength > 65535+MINMATCH) {` | branch-specific rejection/error | [ ] |
| 820 | `convertSequences_noRepcodes` (src/compress/zstd_compress.c:7248) | `assert(longLen == 0);` | assertion failure | [ ] |
| 821 | `convertSequences_noRepcodes` (src/compress/zstd_compress.c:7251) | `if (inSeqs[i+1].litLength > 65535) {` | branch-specific rejection/error | [ ] |
| 822 | `convertSequences_noRepcodes` (src/compress/zstd_compress.c:7252) | `assert(longLen == 0);` | assertion failure | [ ] |
| 823 | `convertSequences_noRepcodes` (src/compress/zstd_compress.c:7259) | `if (i < nbSequences) {` | branch-specific rejection/error | [ ] |
| 824 | `convertSequences_noRepcodes` (src/compress/zstd_compress.c:7261) | `assert(i == nbSequences - 1);` | assertion failure | [ ] |
| 825 | `convertSequences_noRepcodes` (src/compress/zstd_compress.c:7266) | `if (UNLIKELY(inSeqs[i].matchLength > 65535+MINMATCH)) {` | branch-specific rejection/error | [ ] |
| 826 | `convertSequences_noRepcodes` (src/compress/zstd_compress.c:7267) | `assert(longLen == 0);` | assertion failure | [ ] |
| 827 | `convertSequences_noRepcodes` (src/compress/zstd_compress.c:7270) | `if (UNLIKELY(inSeqs[i].litLength > 65535)) {` | branch-specific rejection/error | [ ] |
| 828 | `convertSequences_noRepcodes` (src/compress/zstd_compress.c:7271) | `assert(longLen == 0);` | assertion failure | [ ] |
| 829 | `convertSequences_noRepcodes` (src/compress/zstd_compress.c:7297) | `if (UNLIKELY(inSeqs[n].matchLength > 65535+MINMATCH)) {` | branch-specific rejection/error | [ ] |
| 830 | `convertSequences_noRepcodes` (src/compress/zstd_compress.c:7298) | `assert(longLen == 0);` | assertion failure | [ ] |
| 831 | `convertSequences_noRepcodes` (src/compress/zstd_compress.c:7301) | `if (UNLIKELY(inSeqs[n].litLength > 65535)) {` | branch-specific rejection/error | [ ] |
| 832 | `convertSequences_noRepcodes` (src/compress/zstd_compress.c:7302) | `assert(longLen == 0);` | assertion failure | [ ] |
| 833 | `ZSTD_convertBlockSequences` (src/compress/zstd_compress.c:7327) | `RETURN_ERROR_IF(nbSequences >= cctx->seqStore.maxNbSeq, externalSequences_invalid,` | `ERROR(externalSequences_invalid)` | [ ] |
| 834 | `ZSTD_convertBlockSequences` (src/compress/zstd_compress.c:7333) | `assert(nbSequences >= 1);` | assertion failure | [ ] |
| 835 | `ZSTD_convertBlockSequences` (src/compress/zstd_compress.c:7334) | `assert(inSeqs[nbSequences-1].matchLength == 0);` | assertion failure | [ ] |
| 836 | `ZSTD_convertBlockSequences` (src/compress/zstd_compress.c:7335) | `assert(inSeqs[nbSequences-1].offset == 0);` | assertion failure | [ ] |
| 837 | `ZSTD_convertBlockSequences` (src/compress/zstd_compress.c:7343) | `assert(cctx->seqStore.longLengthType == ZSTD_llt_none);` | assertion failure | [ ] |
| 838 | `ZSTD_convertBlockSequences` (src/compress/zstd_compress.c:7344) | `if (longl <= nbSequences-1) {` | branch-specific rejection/error | [ ] |
| 839 | `ZSTD_convertBlockSequences` (src/compress/zstd_compress.c:7350) | `assert(longl <= 2* (nbSequences-1));` | assertion failure | [ ] |
| 840 | `ZSTD_convertBlockSequences` (src/compress/zstd_compress.c:7369) | `if (!repcodeResolution && nbSequences > 1) {` | branch-specific rejection/error | [ ] |
| 841 | `ZSTD_convertBlockSequences` (src/compress/zstd_compress.c:7372) | `if (nbSequences >= 4) {` | branch-specific rejection/error | [ ] |
| 842 | `ZSTD_convertBlockSequences` (src/compress/zstd_compress.c:7382) | `assert(nbSequences == 2);` | assertion failure | [ ] |
| 843 | `ZSTD_get1BlockSummary` (src/compress/zstd_compress.c:7429) | `if (seqs[i].matchLength == 0) break; /* end of block */` | branch-specific rejection/error | [ ] |
| 844 | `ZSTD_get1BlockSummary` (src/compress/zstd_compress.c:7435) | `bs.nbSequences = ERROR(externalSequences_invalid);` | `ERROR(externalSequences_invalid)` | [ ] |
| 845 | `ZSTD_get1BlockSummary` (src/compress/zstd_compress.c:7453) | `assert(seqs);` | assertion failure | [ ] |
| 846 | `ZSTD_get1BlockSummary` (src/compress/zstd_compress.c:7457) | `if (seqs[n].matchLength == 0) {` | branch-specific rejection/error | [ ] |
| 847 | `ZSTD_get1BlockSummary` (src/compress/zstd_compress.c:7458) | `assert(seqs[n].offset == 0);` | assertion failure | [ ] |
| 848 | `ZSTD_get1BlockSummary` (src/compress/zstd_compress.c:7464) | `bs.nbSequences = ERROR(externalSequences_invalid);` | `ERROR(externalSequences_invalid)` | [ ] |
| 849 | `ZSTD_compressSequencesAndLiterals_internal` (src/compress/zstd_compress.c:7487) | `assert(cctx->appliedParams.searchForExternalRepcodes != ZSTD_ps_auto);` | `ERROR(externalSequences_invalid)` | [ ] |
| 850 | `ZSTD_compressSequencesAndLiterals_internal` (src/compress/zstd_compress.c:7490) | `RETURN_ERROR_IF(nbSequences == 0, externalSequences_invalid, "Requires at least 1 end-of-block");` | `ERROR(externalSequences_invalid)` | [ ] |
| 851 | `ZSTD_compressSequencesAndLiterals_internal` (src/compress/zstd_compress.c:7493) | `if ((nbSequences == 1) && (inSeqs[0].litLength == 0)) {` | `ERROR(dstSize_tooSmall)` | [ ] |
| 852 | `ZSTD_compressSequencesAndLiterals_internal` (src/compress/zstd_compress.c:7495) | `RETURN_ERROR_IF(dstCapacity<3, dstSize_tooSmall, "No room for empty frame block header");` | `ERROR(dstSize_tooSmall)` | [ ] |
| 853 | `ZSTD_compressSequencesAndLiterals_internal` (src/compress/zstd_compress.c:7507) | `assert(block.nbSequences <= nbSequences);` | `ERROR(externalSequences_invalid)` | [ ] |
| 854 | `ZSTD_compressSequencesAndLiterals_internal` (src/compress/zstd_compress.c:7508) | `RETURN_ERROR_IF(block.litSize > litSize, externalSequences_invalid, "discrepancy: Sequences require more literals than present in buffer");` | `ERROR(externalSequences_invalid)` | [ ] |
| 855 | `ZSTD_compressSequencesAndLiterals_internal` (src/compress/zstd_compress.c:7524) | `RETURN_ERROR_IF(dstCapacity < ZSTD_blockHeaderSize, dstSize_tooSmall, "not enough dstCapacity to write a new compressed block");` | `ERROR(dstSize_tooSmall)` | [ ] |
| 856 | `ZSTD_compressSequencesAndLiterals_internal` (src/compress/zstd_compress.c:7536) | `if (compressedSeqsSize > cctx->blockSizeMax) compressedSeqsSize = 0;` | branch-specific rejection/error | [ ] |
| 857 | `ZSTD_compressSequencesAndLiterals_internal` (src/compress/zstd_compress.c:7544) | `if (compressedSeqsSize == 0) {` | branch-specific rejection/error | [ ] |
| 858 | `ZSTD_compressSequencesAndLiterals_internal` (src/compress/zstd_compress.c:7550) | `RETURN_ERROR(cannotProduce_uncompressedBlock, "ZSTD_compressSequencesAndLiterals cannot generate an uncompressed block");` | `ERROR(k)` | [ ] |
| 859 | `ZSTD_compressSequencesAndLiterals_internal` (src/compress/zstd_compress.c:7553) | `assert(compressedSeqsSize > 1); /* no RLE */` | assertion failure | [ ] |
| 860 | `ZSTD_compressSequencesAndLiterals_internal` (src/compress/zstd_compress.c:7556) | `if (cctx->blockState.prevCBlock->entropy.fse.offcode_repeatMode == FSE_repeat_valid)` | branch-specific rejection/error | [ ] |
| 861 | `ZSTD_compressSequencesAndLiterals_internal` (src/compress/zstd_compress.c:7573) | `assert(nbSequences == 0);` | assertion failure | [ ] |
| 862 | `ZSTD_compressSequencesAndLiterals_internal` (src/compress/zstd_compress.c:7578) | `RETURN_ERROR_IF(litSize != 0, externalSequences_invalid, "literals must be entirely and exactly consumed");` | `ERROR(externalSequences_invalid)` | [ ] |
| 863 | `ZSTD_compressSequencesAndLiterals_internal` (src/compress/zstd_compress.c:7579) | `RETURN_ERROR_IF(remaining != 0, externalSequences_invalid, "Sequences must represent a total of exactly srcSize=%zu", srcSize);` | `ERROR(externalSequences_invalid)` | [ ] |
| 864 | `ZSTD_compressSequencesAndLiterals` (src/compress/zstd_compress.c:7596) | `assert(cctx != NULL);` | `ERROR(l)` | [ ] |
| 865 | `ZSTD_compressSequencesAndLiterals` (src/compress/zstd_compress.c:7597) | `if (litCapacity < litSize) {` | `ERROR(l)` | [ ] |
| 866 | `ZSTD_compressSequencesAndLiterals` (src/compress/zstd_compress.c:7598) | `RETURN_ERROR(workSpace_tooSmall, "literals buffer is not large enough: must be at least 8 bytes larger than litSize (risk of read out-of-bound)");` | `ERROR(l)` | [ ] |
| 867 | `ZSTD_compressSequencesAndLiterals` (src/compress/zstd_compress.c:7602) | `if (cctx->appliedParams.blockDelimiters == ZSTD_sf_noBlockDelimiters) {` | `ERROR(d)` | [ ] |
| 868 | `ZSTD_compressSequencesAndLiterals` (src/compress/zstd_compress.c:7603) | `RETURN_ERROR(frameParameter_unsupported, "This mode is only compatible with explicit delimiters");` | `ERROR(d)` | [ ] |
| 869 | `ZSTD_compressSequencesAndLiterals` (src/compress/zstd_compress.c:7605) | `if (cctx->appliedParams.validateSequences) {` | `ERROR(d)` | [ ] |
| 870 | `ZSTD_compressSequencesAndLiterals` (src/compress/zstd_compress.c:7606) | `RETURN_ERROR(parameter_unsupported, "This mode is not compatible with Sequence validation");` | `ERROR(d)` | [ ] |
| 871 | `ZSTD_compressSequencesAndLiterals` (src/compress/zstd_compress.c:7608) | `if (cctx->appliedParams.fParams.checksumFlag) {` | `ERROR(d)` | [ ] |
| 872 | `ZSTD_compressSequencesAndLiterals` (src/compress/zstd_compress.c:7609) | `RETURN_ERROR(frameParameter_unsupported, "this mode is not compatible with frame checksum");` | `ERROR(d)` | [ ] |
| 873 | `ZSTD_compressSequencesAndLiterals` (src/compress/zstd_compress.c:7616) | `assert(frameHeaderSize <= dstCapacity);` | assertion failure | [ ] |
| 874 | `ZSTD_compressSequencesAndLiterals` (src/compress/zstd_compress.c:7628) | `assert(cBlocksSize <= dstCapacity);` | assertion failure | [ ] |
| 875 | `ZSTD_endStream` (src/compress/zstd_compress.c:7659) | `if (zcs->appliedParams.nbWorkers > 0) return remainingToFlush;   /* minimal estimation */` | branch-specific rejection/error | [ ] |
| 876 | `ZSTD_dedicatedDictSearch_revertCParams` (src/compress/zstd_compress.c:7722) | `if (cParams->hashLog < ZSTD_HASHLOG_MIN) {` | branch-specific rejection/error | [ ] |
| 877 | `ZSTD_getCParamRowSize` (src/compress/zstd_compress.c:7745) | `assert(0);` | assertion failure | [ ] |
| 878 | `ZSTD_getCParams_internal` (src/compress/zstd_compress.c:7767) | `if (compressionLevel == 0) row = ZSTD_CLEVEL_DEFAULT;   /* 0 == default */` | branch-specific rejection/error | [ ] |
| 879 | `ZSTD_getCParams_internal` (src/compress/zstd_compress.c:7768) | `else if (compressionLevel < 0) row = 0;   /* entry 0 is baseline for fast mode */` | branch-specific rejection/error | [ ] |
| 880 | `ZSTD_getCParams_internal` (src/compress/zstd_compress.c:7769) | `else if (compressionLevel > ZSTD_MAX_CLEVEL) row = ZSTD_MAX_CLEVEL;` | branch-specific rejection/error | [ ] |
| 881 | `ZSTD_getCParams_internal` (src/compress/zstd_compress.c:7775) | `if (compressionLevel < 0) {` | branch-specific rejection/error | [ ] |
| 882 | `ZSTD_getCParams` (src/compress/zstd_compress.c:7789) | `if (srcSizeHint == 0) srcSizeHint = ZSTD_CONTENTSIZE_UNKNOWN;` | branch-specific rejection/error | [ ] |
| 883 | `ZSTD_getParams` (src/compress/zstd_compress.c:7815) | `if (srcSizeHint == 0) srcSizeHint = ZSTD_CONTENTSIZE_UNKNOWN;` | branch-specific rejection/error | [ ] |
| 884 | `ZSTD_registerSequenceProducer` (src/compress/zstd_compress.c:7824) | `assert(zc != NULL);` | assertion failure | [ ] |
| 885 | `ZSTD_CCtxParams_registerSequenceProducer` (src/compress/zstd_compress.c:7835) | `assert(params != NULL);` | assertion failure | [ ] |
| 886 | `ZSTD_CCtxParams_registerSequenceProducer` (src/compress/zstd_compress.c:7836) | `if (extSeqProdFunc != NULL) {` | branch-specific rejection/error | [ ] |
| 887 | `ZSTD_noCompressLiterals` (src/compress/zstd_compress_literals.c:46) | `RETURN_ERROR_IF(srcSize + flSize > dstCapacity, dstSize_tooSmall, "");` | `ERROR(dstSize_tooSmall)` | [ ] |
| 888 | `ZSTD_noCompressLiterals` (src/compress/zstd_compress_literals.c:60) | `assert(0);` | assertion failure | [ ] |
| 889 | `allBytesIdentical` (src/compress/zstd_compress_literals.c:70) | `assert(srcSize >= 1);` | assertion failure | [ ] |
| 890 | `allBytesIdentical` (src/compress/zstd_compress_literals.c:71) | `assert(src != NULL);` | assertion failure | [ ] |
| 891 | `ZSTD_compressRleLiteralsBlock` (src/compress/zstd_compress_literals.c:86) | `assert(dstCapacity >= 4); (void)dstCapacity;` | assertion failure | [ ] |
| 892 | `ZSTD_compressRleLiteralsBlock` (src/compress/zstd_compress_literals.c:87) | `assert(allBytesIdentical(src, srcSize));` | assertion failure | [ ] |
| 893 | `ZSTD_compressRleLiteralsBlock` (src/compress/zstd_compress_literals.c:101) | `assert(0);` | assertion failure | [ ] |
| 894 | `ZSTD_minLiteralsToCompress` (src/compress/zstd_compress_literals.c:117) | `assert((int)strategy >= 0);` | assertion failure | [ ] |
| 895 | `ZSTD_minLiteralsToCompress` (src/compress/zstd_compress_literals.c:118) | `assert((int)strategy <= 9);` | assertion failure | [ ] |
| 896 | `ZSTD_compressLiterals` (src/compress/zstd_compress_literals.c:158) | `if (srcSize < ZSTD_minLiteralsToCompress(strategy, prevHuf->repeatMode))` | `ERROR(dstSize_tooSmall)` | [ ] |
| 897 | `ZSTD_compressLiterals` (src/compress/zstd_compress_literals.c:161) | `RETURN_ERROR_IF(dstCapacity < lhSize+1, dstSize_tooSmall, "not enough space for compression");` | `ERROR(dstSize_tooSmall)` | [ ] |
| 898 | `ZSTD_compressLiterals` (src/compress/zstd_compress_literals.c:188) | `if ((cLitSize==0) \|\| (cLitSize >= srcSize - minGain) \|\| ERR_isError(cLitSize)) {` | branch-specific rejection/error | [ ] |
| 899 | `ZSTD_compressLiterals` (src/compress/zstd_compress_literals.c:198) | `if ((srcSize >= 8) \|\| allBytesIdentical(src, srcSize)) {` | branch-specific rejection/error | [ ] |
| 900 | `ZSTD_compressLiterals` (src/compress/zstd_compress_literals.c:212) | `if (!singleStream) assert(srcSize >= MIN_LITERALS_FOR_4_STREAMS);` | assertion failure | [ ] |
| 901 | `ZSTD_compressLiterals` (src/compress/zstd_compress_literals.c:218) | `assert(srcSize >= MIN_LITERALS_FOR_4_STREAMS);` | assertion failure | [ ] |
| 902 | `ZSTD_compressLiterals` (src/compress/zstd_compress_literals.c:224) | `assert(srcSize >= MIN_LITERALS_FOR_4_STREAMS);` | assertion failure | [ ] |
| 903 | `ZSTD_compressLiterals` (src/compress/zstd_compress_literals.c:231) | `assert(0);` | assertion failure | [ ] |
| 904 | `ZSTD_entropyCost` (src/compress/zstd_compress_sequences.c:89) | `assert(total > 0);` | assertion failure | [ ] |
| 905 | `ZSTD_entropyCost` (src/compress/zstd_compress_sequences.c:92) | `if (count[s] != 0 && norm == 0)` | branch-specific rejection/error | [ ] |
| 906 | `ZSTD_entropyCost` (src/compress/zstd_compress_sequences.c:94) | `assert(count[s] < total);` | assertion failure | [ ] |
| 907 | `ZSTD_fseBitCost` (src/compress/zstd_compress_sequences.c:114) | `if (ZSTD_getFSEMaxSymbolValue(ctable) < max) {` | `ERROR(GENERIC)` | [ ] |
| 908 | `ZSTD_fseBitCost` (src/compress/zstd_compress_sequences.c:117) | `return ERROR(GENERIC);` | `ERROR(GENERIC)` | [ ] |
| 909 | `ZSTD_fseBitCost` (src/compress/zstd_compress_sequences.c:123) | `if (count[s] == 0)` | `ERROR(GENERIC)` | [ ] |
| 910 | `ZSTD_fseBitCost` (src/compress/zstd_compress_sequences.c:125) | `if (bitCost >= badCost) {` | `ERROR(GENERIC)` | [ ] |
| 911 | `ZSTD_fseBitCost` (src/compress/zstd_compress_sequences.c:127) | `return ERROR(GENERIC);` | `ERROR(GENERIC)` | [ ] |
| 912 | `ZSTD_crossEntropyCost` (src/compress/zstd_compress_sequences.c:145) | `assert(accuracyLog <= 8);` | assertion failure | [ ] |
| 913 | `ZSTD_crossEntropyCost` (src/compress/zstd_compress_sequences.c:149) | `assert(norm256 > 0);` | assertion failure | [ ] |
| 914 | `ZSTD_crossEntropyCost` (src/compress/zstd_compress_sequences.c:150) | `assert(norm256 < 256);` | assertion failure | [ ] |
| 915 | `ZSTD_selectEncodingType` (src/compress/zstd_compress_sequences.c:168) | `if (isDefaultAllowed && nbSeq <= 2) {` | branch-specific rejection/error | [ ] |
| 916 | `ZSTD_selectEncodingType` (src/compress/zstd_compress_sequences.c:179) | `if (strategy < ZSTD_lazy) {` | branch-specific rejection/error | [ ] |
| 917 | `ZSTD_selectEncodingType` (src/compress/zstd_compress_sequences.c:185) | `assert(defaultNormLog >= 5 && defaultNormLog <= 6);  /* xx_DEFAULTNORMLOG */` | assertion failure | [ ] |
| 918 | `ZSTD_selectEncodingType` (src/compress/zstd_compress_sequences.c:186) | `assert(mult <= 9 && mult >= 7);` | assertion failure | [ ] |
| 919 | `ZSTD_selectEncodingType` (src/compress/zstd_compress_sequences.c:192) | `if ( (nbSeq < dynamicFse_nbSeq_min)` | branch-specific rejection/error | [ ] |
| 920 | `ZSTD_selectEncodingType` (src/compress/zstd_compress_sequences.c:206) | `size_t const basicCost = isDefaultAllowed ? ZSTD_crossEntropyCost(defaultNorm, defaultNormLog, count, max) : ERROR(GENERIC);` | `ERROR(GENERIC)` | [ ] |
| 921 | `ZSTD_selectEncodingType` (src/compress/zstd_compress_sequences.c:207) | `size_t const repeatCost = *repeatMode != FSE_repeat_none ? ZSTD_fseBitCost(prevCTable, count, max) : ERROR(GENERIC);` | `ERROR(GENERIC)` | [ ] |
| 922 | `ZSTD_selectEncodingType` (src/compress/zstd_compress_sequences.c:212) | `assert(!ZSTD_isError(basicCost));` | `ERROR(maxCode)` | [ ] |
| 923 | `ZSTD_selectEncodingType` (src/compress/zstd_compress_sequences.c:213) | `assert(!(*repeatMode == FSE_repeat_valid && ZSTD_isError(repeatCost)));` | `ERROR(maxCode)` | [ ] |
| 924 | `ZSTD_selectEncodingType` (src/compress/zstd_compress_sequences.c:215) | `assert(!ZSTD_isError(NCountCost));` | `ERROR(maxCode)` | [ ] |
| 925 | `ZSTD_selectEncodingType` (src/compress/zstd_compress_sequences.c:216) | `assert(compressedCost < ERROR(maxCode));` | `ERROR(maxCode)` | [ ] |
| 926 | `ZSTD_selectEncodingType` (src/compress/zstd_compress_sequences.c:219) | `if (basicCost <= repeatCost && basicCost <= compressedCost) {` | branch-specific rejection/error | [ ] |
| 927 | `ZSTD_selectEncodingType` (src/compress/zstd_compress_sequences.c:221) | `assert(isDefaultAllowed);` | assertion failure | [ ] |
| 928 | `ZSTD_selectEncodingType` (src/compress/zstd_compress_sequences.c:225) | `if (repeatCost <= compressedCost) {` | branch-specific rejection/error | [ ] |
| 929 | `ZSTD_selectEncodingType` (src/compress/zstd_compress_sequences.c:227) | `assert(!ZSTD_isError(repeatCost));` | assertion failure | [ ] |
| 930 | `ZSTD_selectEncodingType` (src/compress/zstd_compress_sequences.c:230) | `assert(compressedCost < basicCost && compressedCost < repeatCost);` | assertion failure | [ ] |
| 931 | `ZSTD_buildCTable` (src/compress/zstd_compress_sequences.c:258) | `RETURN_ERROR_IF(dstCapacity==0, dstSize_tooSmall, "not enough space");` | `ERROR(dstSize_tooSmall)` | [ ] |
| 932 | `ZSTD_buildCTable` (src/compress/zstd_compress_sequences.c:271) | `if (count[codeTable[nbSeq-1]] > 1) {` | branch-specific rejection/error | [ ] |
| 933 | `ZSTD_buildCTable` (src/compress/zstd_compress_sequences.c:275) | `assert(nbSeq_1 > 1);` | assertion failure | [ ] |
| 934 | `ZSTD_buildCTable` (src/compress/zstd_compress_sequences.c:276) | `assert(entropyWorkspaceSize >= sizeof(ZSTD_BuildCTableWksp));` | assertion failure | [ ] |
| 935 | `ZSTD_buildCTable` (src/compress/zstd_compress_sequences.c:279) | `assert(oend >= op);` | assertion failure | [ ] |
| 936 | `ZSTD_buildCTable` (src/compress/zstd_compress_sequences.c:286) | `default: assert(0); RETURN_ERROR(GENERIC, "impossible to reach");` | `ERROR(C)` | [ ] |
| 937 | `ZSTD_encodeSequences_body` (src/compress/zstd_compress_sequences.c:303) | `RETURN_ERROR_IF(` | `ERROR(dst)` | [ ] |
| 938 | `ZSTD_encodeSequences_body` (src/compress/zstd_compress_sequences.c:350) | `if (MEM_32bits() \|\| (ofBits+mlBits+llBits >= 64-7-(LLFSELog+MLFSELog+OffFSELog)))` | branch-specific rejection/error | [ ] |
| 939 | `ZSTD_encodeSequences_body` (src/compress/zstd_compress_sequences.c:353) | `if (MEM_32bits() && ((llBits+mlBits)>24)) BIT_flushBits(&blockStream);` | branch-specific rejection/error | [ ] |
| 940 | `ZSTD_encodeSequences_body` (src/compress/zstd_compress_sequences.c:355) | `if (MEM_32bits() \|\| (ofBits+mlBits+llBits > 56)) BIT_flushBits(&blockStream);` | branch-specific rejection/error | [ ] |
| 941 | `ZSTD_encodeSequences_body` (src/compress/zstd_compress_sequences.c:379) | `RETURN_ERROR_IF(streamSize==0, dstSize_tooSmall, "not enough space");` | `ERROR(dstSize_tooSmall)` | [ ] |
| 942 | `ZSTD_compressSubBlock_literal` (src/compress/zstd_compress_superblock.c:60) | `if (litSize == 0 \|\| hufMetadata->hType == set_basic) {` | branch-specific rejection/error | [ ] |
| 943 | `ZSTD_compressSubBlock_literal` (src/compress/zstd_compress_superblock.c:63) | `} else if (hufMetadata->hType == set_rle) {` | branch-specific rejection/error | [ ] |
| 944 | `ZSTD_compressSubBlock_literal` (src/compress/zstd_compress_superblock.c:68) | `assert(litSize > 0);` | assertion failure | [ ] |
| 945 | `ZSTD_compressSubBlock_literal` (src/compress/zstd_compress_superblock.c:69) | `assert(hufMetadata->hType == set_compressed \|\| hufMetadata->hType == set_repeat);` | assertion failure | [ ] |
| 946 | `ZSTD_compressSubBlock_literal` (src/compress/zstd_compress_superblock.c:71) | `if (writeEntropy && hufMetadata->hType == set_compressed) {` | branch-specific rejection/error | [ ] |
| 947 | `ZSTD_compressSubBlock_literal` (src/compress/zstd_compress_superblock.c:83) | `if (cSize == 0 \|\| ERR_isError(cSize)) {` | branch-specific rejection/error | [ ] |
| 948 | `ZSTD_compressSubBlock_literal` (src/compress/zstd_compress_superblock.c:88) | `if (!writeEntropy && cLitSize >= litSize) {` | branch-specific rejection/error | [ ] |
| 949 | `ZSTD_compressSubBlock_literal` (src/compress/zstd_compress_superblock.c:93) | `if (lhSize < (size_t)(3 + (cLitSize >= 1 KB) + (cLitSize >= 16 KB))) {` | branch-specific rejection/error | [ ] |
| 950 | `ZSTD_compressSubBlock_literal` (src/compress/zstd_compress_superblock.c:94) | `assert(cLitSize > litSize);` | assertion failure | [ ] |
| 951 | `ZSTD_compressSubBlock_literal` (src/compress/zstd_compress_superblock.c:121) | `assert(0);` | assertion failure | [ ] |
| 952 | `ZSTD_seqDecompressedSize` (src/compress/zstd_compress_superblock.c:145) | `assert(litLengthSum == litSize);` | assertion failure | [ ] |
| 953 | `ZSTD_seqDecompressedSize` (src/compress/zstd_compress_superblock.c:147) | `assert(litLengthSum <= litSize);` | assertion failure | [ ] |
| 954 | `ZSTD_compressSubBlock_sequences` (src/compress/zstd_compress_superblock.c:181) | `RETURN_ERROR_IF((oend-op) < 3 /*max nbSeq Size*/ + 1 /*seqHead*/,` | `ERROR(p)` | [ ] |
| 955 | `ZSTD_compressSubBlock_sequences` (src/compress/zstd_compress_superblock.c:183) | `if (nbSeq < 128)` | branch-specific rejection/error | [ ] |
| 956 | `ZSTD_compressSubBlock_sequences` (src/compress/zstd_compress_superblock.c:185) | `else if (nbSeq < LONGNBSEQ)` | branch-specific rejection/error | [ ] |
| 957 | `ZSTD_compressSubBlock_sequences` (src/compress/zstd_compress_superblock.c:189) | `if (nbSeq==0) {` | branch-specific rejection/error | [ ] |
| 958 | `ZSTD_compressSubBlock_sequences` (src/compress/zstd_compress_superblock.c:229) | `if (writeEntropy && fseMetadata->lastCountSize && fseMetadata->lastCountSize + bitstreamSize < 4) {` | branch-specific rejection/error | [ ] |
| 959 | `ZSTD_compressSubBlock_sequences` (src/compress/zstd_compress_superblock.c:231) | `assert(fseMetadata->lastCountSize + bitstreamSize == 3);` | assertion failure | [ ] |
| 960 | `ZSTD_compressSubBlock_sequences` (src/compress/zstd_compress_superblock.c:248) | `if (op-seqHead < 4) {` | branch-specific rejection/error | [ ] |
| 961 | `ZSTD_compressSubBlock` (src/compress/zstd_compress_superblock.c:285) | `if (cLitSize == 0) return 0;` | branch-specific rejection/error | [ ] |
| 962 | `ZSTD_compressSubBlock` (src/compress/zstd_compress_superblock.c:296) | `if (cSeqSize == 0) return 0;` | branch-specific rejection/error | [ ] |
| 963 | `ZSTD_estimateSubBlockSize_literal` (src/compress/zstd_compress_superblock.c:317) | `if (hufMetadata->hType == set_basic) return litSize;` | branch-specific rejection/error | [ ] |
| 964 | `ZSTD_estimateSubBlockSize_literal` (src/compress/zstd_compress_superblock.c:318) | `else if (hufMetadata->hType == set_rle) return 1;` | branch-specific rejection/error | [ ] |
| 965 | `ZSTD_estimateSubBlockSize_literal` (src/compress/zstd_compress_superblock.c:319) | `else if (hufMetadata->hType == set_compressed \|\| hufMetadata->hType == set_repeat) {` | branch-specific rejection/error | [ ] |
| 966 | `ZSTD_estimateSubBlockSize_literal` (src/compress/zstd_compress_superblock.c:323) | `if (writeEntropy) cLitSizeEstimate += hufMetadata->hufDesSize;` | branch-specific rejection/error | [ ] |
| 967 | `ZSTD_estimateSubBlockSize_literal` (src/compress/zstd_compress_superblock.c:326) | `assert(0); /* impossible */` | assertion failure | [ ] |
| 968 | `ZSTD_estimateSubBlockSize_symbolType` (src/compress/zstd_compress_superblock.c:347) | `assert(max <= defaultMax);` | `ERROR(GENERIC)` | [ ] |
| 969 | `ZSTD_estimateSubBlockSize_symbolType` (src/compress/zstd_compress_superblock.c:350) | `: ERROR(GENERIC);` | `ERROR(GENERIC)` | [ ] |
| 970 | `ZSTD_estimateSubBlockSize_sequences` (src/compress/zstd_compress_superblock.c:376) | `if (nbSeq == 0) return sequencesSectionHeaderSize;` | branch-specific rejection/error | [ ] |
| 971 | `ZSTD_estimateSubBlockSize_sequences` (src/compress/zstd_compress_superblock.c:389) | `if (writeEntropy) cSeqSizeEstimate += fseMetadata->fseTablesSize;` | branch-specific rejection/error | [ ] |
| 972 | `ZSTD_needSequenceEntropyTables` (src/compress/zstd_compress_superblock.c:420) | `if (fseMetadata->llType == set_compressed \|\| fseMetadata->llType == set_rle)` | branch-specific rejection/error | [ ] |
| 973 | `ZSTD_needSequenceEntropyTables` (src/compress/zstd_compress_superblock.c:422) | `if (fseMetadata->mlType == set_compressed \|\| fseMetadata->mlType == set_rle)` | branch-specific rejection/error | [ ] |
| 974 | `ZSTD_needSequenceEntropyTables` (src/compress/zstd_compress_superblock.c:424) | `if (fseMetadata->ofType == set_compressed \|\| fseMetadata->ofType == set_rle)` | branch-specific rejection/error | [ ] |
| 975 | `countLiterals` (src/compress/zstd_compress_superblock.c:432) | `assert(sp != NULL);` | assertion failure | [ ] |
| 976 | `sizeBlockSequences` (src/compress/zstd_compress_superblock.c:449) | `assert(firstSubBlock==0 \|\| firstSubBlock==1);` | assertion failure | [ ] |
| 977 | `sizeBlockSequences` (src/compress/zstd_compress_superblock.c:454) | `if (budget > targetBudget) return 1;` | branch-specific rejection/error | [ ] |
| 978 | `sizeBlockSequences` (src/compress/zstd_compress_superblock.c:463) | `if ( (budget > targetBudget)` | branch-specific rejection/error | [ ] |
| 979 | `ZSTD_compressSubBlock_multi` (src/compress/zstd_compress_superblock.c:514) | `if (nbSeqs > 0) {` | branch-specific rejection/error | [ ] |
| 980 | `ZSTD_compressSubBlock_multi` (src/compress/zstd_compress_superblock.c:532) | `if (ebs.estBlockSize > srcSize) return 0;` | branch-specific rejection/error | [ ] |
| 981 | `ZSTD_compressSubBlock_multi` (src/compress/zstd_compress_superblock.c:535) | `assert(nbSubBlocks>0);` | assertion failure | [ ] |
| 982 | `ZSTD_compressSubBlock_multi` (src/compress/zstd_compress_superblock.c:541) | `assert(seqCount <= (size_t)(send-sp));` | assertion failure | [ ] |
| 983 | `ZSTD_compressSubBlock_multi` (src/compress/zstd_compress_superblock.c:543) | `assert(seqCount > 0);` | assertion failure | [ ] |
| 984 | `ZSTD_compressSubBlock_multi` (src/compress/zstd_compress_superblock.c:562) | `if (cSize > 0 && cSize < decompressedSize) {` | branch-specific rejection/error | [ ] |
| 985 | `ZSTD_compressSubBlock_multi` (src/compress/zstd_compress_superblock.c:565) | `assert(ip + decompressedSize <= iend);` | assertion failure | [ ] |
| 986 | `ZSTD_compressSubBlock_multi` (src/compress/zstd_compress_superblock.c:584) | `} /* if (nbSeqs > 0) */` | branch-specific rejection/error | [ ] |
| 987 | `ZSTD_compressSubBlock_multi` (src/compress/zstd_compress_superblock.c:606) | `if (cSize > 0 && cSize < decompressedSize) {` | branch-specific rejection/error | [ ] |
| 988 | `ZSTD_compressSubBlock_multi` (src/compress/zstd_compress_superblock.c:609) | `assert(ip + decompressedSize <= iend);` | assertion failure | [ ] |
| 989 | `ZSTD_compressSubBlock_multi` (src/compress/zstd_compress_superblock.c:632) | `if (writeSeqEntropy && ZSTD_needSequenceEntropyTables(&entropyMetadata->fseMetadata)) {` | branch-specific rejection/error | [ ] |
| 990 | `ZSTD_compressSubBlock_multi` (src/compress/zstd_compress_superblock.c:640) | `if (ip < iend) {` | branch-specific rejection/error | [ ] |
| 991 | `ZSTD_compressSubBlock_multi` (src/compress/zstd_compress_superblock.c:646) | `assert(cSize != 0);` | assertion failure | [ ] |
| 992 | `ZSTD_compressSubBlock_multi` (src/compress/zstd_compress_superblock.c:649) | `if (sp < send) {` | branch-specific rejection/error | [ ] |
| 993 | `ZSTD_fillDoubleHashTableForCDict` (src/compress/zstd_double_fast.c:42) | `if (i == 0) {` | branch-specific rejection/error | [ ] |
| 994 | `ZSTD_fillDoubleHashTableForCDict` (src/compress/zstd_double_fast.c:45) | `if (i == 0 \|\| hashLarge[lgHashAndTag >> ZSTD_SHORT_CACHE_TAG_BITS] == 0) {` | branch-specific rejection/error | [ ] |
| 995 | `ZSTD_fillDoubleHashTableForCCtx` (src/compress/zstd_double_fast.c:80) | `if (i == 0)` | branch-specific rejection/error | [ ] |
| 996 | `ZSTD_fillDoubleHashTableForCCtx` (src/compress/zstd_double_fast.c:82) | `if (i == 0 \|\| hashLarge[lgHash] == 0)` | branch-specific rejection/error | [ ] |
| 997 | `ZSTD_compressBlock_doubleFast_noDict_generic` (src/compress/zstd_double_fast.c:162) | `if (offset_2 > maxRep) offsetSaved2 = offset_2, offset_2 = 0;` | branch-specific rejection/error | [ ] |
| 998 | `ZSTD_compressBlock_doubleFast_noDict_generic` (src/compress/zstd_double_fast.c:163) | `if (offset_1 > maxRep) offsetSaved1 = offset_1, offset_1 = 0;` | branch-specific rejection/error | [ ] |
| 999 | `ZSTD_compressBlock_doubleFast_noDict_generic` (src/compress/zstd_double_fast.c:172) | `if (ip1 > ilimit) {` | branch-specific rejection/error | [ ] |
| 1000 | `ZSTD_compressBlock_doubleFast_noDict_generic` (src/compress/zstd_double_fast.c:190) | `if ((offset_1 > 0) & (MEM_read32(ip+1-offset_1) == MEM_read32(ip+1))) {` | branch-specific rejection/error | [ ] |
| 1001 | `ZSTD_compressBlock_doubleFast_noDict_generic` (src/compress/zstd_double_fast.c:224) | `if (ip1 >= nextStep) {` | branch-specific rejection/error | [ ] |
| 1002 | `ZSTD_compressBlock_doubleFast_noDict_generic` (src/compress/zstd_double_fast.c:260) | `if ((idxl1 > prefixLowestIndex) && (MEM_read64(matchl1) == MEM_read64(ip1))) {` | branch-specific rejection/error | [ ] |
| 1003 | `ZSTD_compressBlock_doubleFast_noDict_generic` (src/compress/zstd_double_fast.c:262) | `if (l1len > mLength) {` | branch-specific rejection/error | [ ] |
| 1004 | `ZSTD_compressBlock_doubleFast_noDict_generic` (src/compress/zstd_double_fast.c:279) | `if (step < 4) {` | branch-specific rejection/error | [ ] |
| 1005 | `ZSTD_compressBlock_doubleFast_noDict_generic` (src/compress/zstd_double_fast.c:297) | `if (ip <= ilimit) {` | branch-specific rejection/error | [ ] |
| 1006 | `ZSTD_compressBlock_doubleFast_dictMatchState_generic` (src/compress/zstd_double_fast.c:366) | `assert(ms->window.dictLimit + (1U << cParams->windowLog) >= endIndex);` | assertion failure | [ ] |
| 1007 | `ZSTD_compressBlock_doubleFast_dictMatchState_generic` (src/compress/zstd_double_fast.c:368) | `if (ms->prefetchCDictTables) {` | branch-specific rejection/error | [ ] |
| 1008 | `ZSTD_compressBlock_doubleFast_dictMatchState_generic` (src/compress/zstd_double_fast.c:380) | `assert(offset_1 <= dictAndPrefixLength);` | assertion failure | [ ] |
| 1009 | `ZSTD_compressBlock_doubleFast_dictMatchState_generic` (src/compress/zstd_double_fast.c:381) | `assert(offset_2 <= dictAndPrefixLength);` | assertion failure | [ ] |
| 1010 | `ZSTD_compressBlock_doubleFast_dictMatchState_generic` (src/compress/zstd_double_fast.c:416) | `if ((matchIndexL >= prefixLowestIndex) && (MEM_read64(matchLong) == MEM_read64(ip))) {` | branch-specific rejection/error | [ ] |
| 1011 | `ZSTD_compressBlock_doubleFast_dictMatchState_generic` (src/compress/zstd_double_fast.c:426) | `assert(dictMatchL < dictEnd);` | assertion failure | [ ] |
| 1012 | `ZSTD_compressBlock_doubleFast_dictMatchState_generic` (src/compress/zstd_double_fast.c:428) | `if (dictMatchL > dictStart && MEM_read64(dictMatchL) == MEM_read64(ip)) {` | branch-specific rejection/error | [ ] |
| 1013 | `ZSTD_compressBlock_doubleFast_dictMatchState_generic` (src/compress/zstd_double_fast.c:435) | `if (matchIndexS > prefixLowestIndex) {` | branch-specific rejection/error | [ ] |
| 1014 | `ZSTD_compressBlock_doubleFast_dictMatchState_generic` (src/compress/zstd_double_fast.c:446) | `if (match > dictStart && MEM_read32(match) == MEM_read32(ip)) {` | branch-specific rejection/error | [ ] |
| 1015 | `ZSTD_compressBlock_doubleFast_dictMatchState_generic` (src/compress/zstd_double_fast.c:466) | `if ((matchIndexL3 >= prefixLowestIndex) && (MEM_read64(matchL3) == MEM_read64(ip+1))) {` | branch-specific rejection/error | [ ] |
| 1016 | `ZSTD_compressBlock_doubleFast_dictMatchState_generic` (src/compress/zstd_double_fast.c:476) | `assert(dictMatchL3 < dictEnd);` | assertion failure | [ ] |
| 1017 | `ZSTD_compressBlock_doubleFast_dictMatchState_generic` (src/compress/zstd_double_fast.c:477) | `if (dictMatchL3 > dictStart && MEM_read64(dictMatchL3) == MEM_read64(ip+1)) {` | branch-specific rejection/error | [ ] |
| 1018 | `ZSTD_compressBlock_doubleFast_dictMatchState_generic` (src/compress/zstd_double_fast.c:486) | `if (matchIndexS < prefixLowestIndex) {` | branch-specific rejection/error | [ ] |
| 1019 | `ZSTD_compressBlock_doubleFast_dictMatchState_generic` (src/compress/zstd_double_fast.c:507) | `if (ip <= ilimit) {` | branch-specific rejection/error | [ ] |
| 1020 | `ZSTD_compressBlock_doubleFast_extDict_generic` (src/compress/zstd_double_fast.c:670) | `if ((matchLongIndex > dictStartIndex) && (MEM_read64(matchLong) == MEM_read64(ip))) {` | branch-specific rejection/error | [ ] |
| 1021 | `ZSTD_compressBlock_doubleFast_extDict_generic` (src/compress/zstd_double_fast.c:681) | `} else if ((matchIndex > dictStartIndex) && (MEM_read32(match) == MEM_read32(ip))) {` | branch-specific rejection/error | [ ] |
| 1022 | `ZSTD_compressBlock_doubleFast_extDict_generic` (src/compress/zstd_double_fast.c:688) | `if ( (matchIndex3 > dictStartIndex) && (MEM_read64(match3) == MEM_read64(ip+1)) ) {` | branch-specific rejection/error | [ ] |
| 1023 | `ZSTD_compressBlock_doubleFast_extDict_generic` (src/compress/zstd_double_fast.c:715) | `if (ip <= ilimit) {` | branch-specific rejection/error | [ ] |
| 1024 | `ZSTD_fillHashTableForCDict` (src/compress/zstd_fast.c:31) | `assert(dtlm == ZSTD_dtlm_full);` | assertion failure | [ ] |
| 1025 | `ZSTD_fillHashTableForCDict` (src/compress/zstd_fast.c:46) | `if (hashTable[hashAndTag >> ZSTD_SHORT_CACHE_TAG_BITS] == 0) {  /* not yet filled */` | branch-specific rejection/error | [ ] |
| 1026 | `ZSTD_fillHashTableForCCtx` (src/compress/zstd_fast.c:68) | `assert(dtlm == ZSTD_dtlm_fast);` | assertion failure | [ ] |
| 1027 | `ZSTD_fillHashTableForCCtx` (src/compress/zstd_fast.c:82) | `if (hashTable[hash] == 0) {  /* not yet filled */` | branch-specific rejection/error | [ ] |
| 1028 | `ZSTD_match4Found_branch` (src/compress/zstd_fast.c:134) | `if (matchIdx >= idxLowLimit) {` | branch-specific rejection/error | [ ] |
| 1029 | `ZSTD_compressBlock_fast_noDict_generic` (src/compress/zstd_fast.c:242) | `if (rep_offset2 > maxRep) offsetSaved2 = rep_offset2, rep_offset2 = 0;` | branch-specific rejection/error | [ ] |
| 1030 | `ZSTD_compressBlock_fast_noDict_generic` (src/compress/zstd_fast.c:243) | `if (rep_offset1 > maxRep) offsetSaved1 = rep_offset1, rep_offset1 = 0;` | branch-specific rejection/error | [ ] |
| 1031 | `ZSTD_compressBlock_fast_noDict_generic` (src/compress/zstd_fast.c:257) | `if (ip3 >= ilimit) {` | branch-specific rejection/error | [ ] |
| 1032 | `ZSTD_compressBlock_fast_noDict_generic` (src/compress/zstd_fast.c:275) | `if ((MEM_read32(ip2) == rval) & (rep_offset1 > 0)) {` | branch-specific rejection/error | [ ] |
| 1033 | `ZSTD_compressBlock_fast_noDict_generic` (src/compress/zstd_fast.c:319) | `if (step <= 4) {` | branch-specific rejection/error | [ ] |
| 1034 | `ZSTD_compressBlock_fast_noDict_generic` (src/compress/zstd_fast.c:342) | `if (ip2 >= nextStep) {` | branch-specific rejection/error | [ ] |
| 1035 | `ZSTD_compressBlock_fast_noDict_generic` (src/compress/zstd_fast.c:404) | `if (ip0 <= ilimit) {` | branch-specific rejection/error | [ ] |
| 1036 | `ZSTD_compressBlock_fast_noDict_generic` (src/compress/zstd_fast.c:406) | `assert(base+current0+2 > istart);  /* check base overflow */` | assertion failure | [ ] |
| 1037 | `ZSTD_compressBlock_fast_noDict_generic` (src/compress/zstd_fast.c:410) | `if (rep_offset2 > 0) { /* rep_offset2==0 means rep_offset2 is invalidated */` | branch-specific rejection/error | [ ] |
| 1038 | `ZSTD_compressBlock_fast` (src/compress/zstd_fast.c:450) | `assert(ms->dictMatchState == NULL);` | assertion failure | [ ] |
| 1039 | `ZSTD_compressBlock_fast_dictMatchState_generic` (src/compress/zstd_fast.c:518) | `assert(endIndex - prefixStartIndex <= maxDistance);` | assertion failure | [ ] |
| 1040 | `ZSTD_compressBlock_fast_dictMatchState_generic` (src/compress/zstd_fast.c:519) | `(void)maxDistance; (void)endIndex;   /* these variables are not used when assert() is disabled */` | assertion failure | [ ] |
| 1041 | `ZSTD_compressBlock_fast_dictMatchState_generic` (src/compress/zstd_fast.c:525) | `assert(prefixStartIndex >= (U32)(dictEnd - dictBase));` | assertion failure | [ ] |
| 1042 | `ZSTD_compressBlock_fast_dictMatchState_generic` (src/compress/zstd_fast.c:527) | `if (ms->prefetchCDictTables) {` | branch-specific rejection/error | [ ] |
| 1043 | `ZSTD_compressBlock_fast_dictMatchState_generic` (src/compress/zstd_fast.c:537) | `assert(offset_1 <= dictAndPrefixLength);` | assertion failure | [ ] |
| 1044 | `ZSTD_compressBlock_fast_dictMatchState_generic` (src/compress/zstd_fast.c:538) | `assert(offset_2 <= dictAndPrefixLength);` | assertion failure | [ ] |
| 1045 | `ZSTD_compressBlock_fast_dictMatchState_generic` (src/compress/zstd_fast.c:541) | `assert(stepSize >= 1);` | assertion failure | [ ] |
| 1046 | `ZSTD_compressBlock_fast_dictMatchState_generic` (src/compress/zstd_fast.c:580) | `if (dictMatchIndex > dictStartIndex &&` | branch-specific rejection/error | [ ] |
| 1047 | `ZSTD_compressBlock_fast_dictMatchState_generic` (src/compress/zstd_fast.c:583) | `if (matchIndex <= prefixStartIndex) {` | branch-specific rejection/error | [ ] |
| 1048 | `ZSTD_compressBlock_fast_dictMatchState_generic` (src/compress/zstd_fast.c:621) | `if (ip1 >= nextStep) {` | branch-specific rejection/error | [ ] |
| 1049 | `ZSTD_compressBlock_fast_dictMatchState_generic` (src/compress/zstd_fast.c:627) | `if (ip1 > ilimit) goto _cleanup;` | branch-specific rejection/error | [ ] |
| 1050 | `ZSTD_compressBlock_fast_dictMatchState_generic` (src/compress/zstd_fast.c:634) | `assert(mLength);` | assertion failure | [ ] |
| 1051 | `ZSTD_compressBlock_fast_dictMatchState_generic` (src/compress/zstd_fast.c:638) | `if (ip0 <= ilimit) {` | branch-specific rejection/error | [ ] |
| 1052 | `ZSTD_compressBlock_fast_dictMatchState_generic` (src/compress/zstd_fast.c:640) | `assert(base+curr+2 > istart);  /* check base overflow */` | assertion failure | [ ] |
| 1053 | `ZSTD_compressBlock_fast_dictMatchState_generic` (src/compress/zstd_fast.c:667) | `assert(ip0 == anchor);` | assertion failure | [ ] |
| 1054 | `ZSTD_compressBlock_fast_dictMatchState` (src/compress/zstd_fast.c:691) | `assert(ms->dictMatchState != NULL);` | assertion failure | [ ] |
| 1055 | `ZSTD_compressBlock_fast_extDict_generic` (src/compress/zstd_fast.c:766) | `if (offset_2 >= maxRep) offsetSaved2 = offset_2, offset_2 = 0;` | branch-specific rejection/error | [ ] |
| 1056 | `ZSTD_compressBlock_fast_extDict_generic` (src/compress/zstd_fast.c:767) | `if (offset_1 >= maxRep) offsetSaved1 = offset_1, offset_1 = 0;` | branch-specific rejection/error | [ ] |
| 1057 | `ZSTD_compressBlock_fast_extDict_generic` (src/compress/zstd_fast.c:781) | `if (ip3 >= ilimit) {` | branch-specific rejection/error | [ ] |
| 1058 | `ZSTD_compressBlock_fast_extDict_generic` (src/compress/zstd_fast.c:797) | `if ( ((U32)(prefixStartIndex - repIndex) >= 4) /* intentional underflow */` | branch-specific rejection/error | [ ] |
| 1059 | `ZSTD_compressBlock_fast_extDict_generic` (src/compress/zstd_fast.c:813) | `assert((match0 != prefixStart) & (match0 != dictStart));` | assertion failure | [ ] |
| 1060 | `ZSTD_compressBlock_fast_extDict_generic` (src/compress/zstd_fast.c:876) | `if (ip2 >= nextStep) {` | branch-specific rejection/error | [ ] |
| 1061 | `ZSTD_compressBlock_fast_extDict_generic` (src/compress/zstd_fast.c:922) | `assert(matchEnd != 0);` | assertion failure | [ ] |
| 1062 | `ZSTD_compressBlock_fast_extDict_generic` (src/compress/zstd_fast.c:931) | `if (ip1 < ip0) {` | branch-specific rejection/error | [ ] |
| 1063 | `ZSTD_compressBlock_fast_extDict_generic` (src/compress/zstd_fast.c:936) | `if (ip0 <= ilimit) {` | branch-specific rejection/error | [ ] |
| 1064 | `ZSTD_compressBlock_fast_extDict_generic` (src/compress/zstd_fast.c:938) | `assert(base+current0+2 > istart);  /* check base overflow */` | assertion failure | [ ] |
| 1065 | `ZSTD_compressBlock_fast_extDict_generic` (src/compress/zstd_fast.c:945) | `if ( ((ZSTD_index_overlap_check(prefixStartIndex, repIndex2)) & (offset_2 > 0))` | branch-specific rejection/error | [ ] |
| 1066 | `ZSTD_compressBlock_fast_extDict` (src/compress/zstd_fast.c:972) | `assert(ms->dictMatchState == NULL);` | assertion failure | [ ] |
| 1067 | `ZSTD_updateDUBT` (src/compress/zstd_lazy.c:48) | `assert(ip + 8 <= iend);   /* condition for ZSTD_hashPtr */` | assertion failure | [ ] |
| 1068 | `ZSTD_updateDUBT` (src/compress/zstd_lazy.c:51) | `assert(idx >= ms->window.dictLimit);   /* condition for valid base+idx */` | assertion failure | [ ] |
| 1069 | `ZSTD_insertDUBT1` (src/compress/zstd_lazy.c:103) | `assert(curr >= btLow);` | assertion failure | [ ] |
| 1070 | `ZSTD_insertDUBT1` (src/compress/zstd_lazy.c:104) | `assert(ip < iend);   /* condition for ZSTD_count */` | assertion failure | [ ] |
| 1071 | `ZSTD_insertDUBT1` (src/compress/zstd_lazy.c:109) | `assert(matchIndex < curr);` | assertion failure | [ ] |
| 1072 | `ZSTD_insertDUBT1` (src/compress/zstd_lazy.c:120) | `assert( (matchIndex+matchLength >= dictLimit)   /* might be wrong if extDict is incorrectly set to 0 */` | assertion failure | [ ] |
| 1073 | `ZSTD_insertDUBT1` (src/compress/zstd_lazy.c:127) | `if (matchIndex+matchLength >= dictLimit)` | branch-specific rejection/error | [ ] |
| 1074 | `ZSTD_insertDUBT1` (src/compress/zstd_lazy.c:138) | `if (match[matchLength] < ip[matchLength]) {  /* necessarily within buffer */` | branch-specific rejection/error | [ ] |
| 1075 | `ZSTD_insertDUBT1` (src/compress/zstd_lazy.c:142) | `if (matchIndex <= btLow) { smallerPtr=&dummy32; break; }   /* beyond tree size, stop searching */` | branch-specific rejection/error | [ ] |
| 1076 | `ZSTD_insertDUBT1` (src/compress/zstd_lazy.c:151) | `if (matchIndex <= btLow) { largerPtr=&dummy32; break; }   /* beyond tree size, stop searching */` | branch-specific rejection/error | [ ] |
| 1077 | `ZSTD_DUBT_findBetterDictMatch` (src/compress/zstd_lazy.c:197) | `assert(dictMode == ZSTD_dictMatchState);` | assertion failure | [ ] |
| 1078 | `ZSTD_DUBT_findBetterDictMatch` (src/compress/zstd_lazy.c:204) | `if (dictMatchIndex+matchLength >= dictHighLimit)` | branch-specific rejection/error | [ ] |
| 1079 | `ZSTD_DUBT_findBetterDictMatch` (src/compress/zstd_lazy.c:207) | `if (matchLength > bestLength) {` | branch-specific rejection/error | [ ] |
| 1080 | `ZSTD_DUBT_findBetterDictMatch` (src/compress/zstd_lazy.c:209) | `if ( (4*(int)(matchLength-bestLength)) > (int)(ZSTD_highbit32(curr-matchIndex+1) - ZSTD_highbit32((U32)offsetPtr[0]+1)) ) {` | branch-specific rejection/error | [ ] |
| 1081 | `ZSTD_DUBT_findBetterDictMatch` (src/compress/zstd_lazy.c:219) | `if (match[matchLength] < ip[matchLength]) {` | branch-specific rejection/error | [ ] |
| 1082 | `ZSTD_DUBT_findBetterDictMatch` (src/compress/zstd_lazy.c:220) | `if (dictMatchIndex <= btLow) { break; }   /* beyond tree size, stop the search */` | branch-specific rejection/error | [ ] |
| 1083 | `ZSTD_DUBT_findBetterDictMatch` (src/compress/zstd_lazy.c:225) | `if (dictMatchIndex <= btLow) { break; }   /* beyond tree size, stop the search */` | branch-specific rejection/error | [ ] |
| 1084 | `ZSTD_DUBT_findBetterDictMatch` (src/compress/zstd_lazy.c:231) | `if (bestLength >= MINMATCH) {` | branch-specific rejection/error | [ ] |
| 1085 | `ZSTD_DUBT_findBestMatch` (src/compress/zstd_lazy.c:272) | `assert(ip <= iend-8);   /* required for h calculation */` | assertion failure | [ ] |
| 1086 | `ZSTD_DUBT_findBestMatch` (src/compress/zstd_lazy.c:273) | `assert(dictMode != ZSTD_dedicatedDictSearch);` | assertion failure | [ ] |
| 1087 | `ZSTD_DUBT_findBestMatch` (src/compress/zstd_lazy.c:291) | `if ( (matchIndex > unsortLimit)` | branch-specific rejection/error | [ ] |
| 1088 | `ZSTD_DUBT_findBestMatch` (src/compress/zstd_lazy.c:329) | `if ((dictMode != ZSTD_extDict) \|\| (matchIndex+matchLength >= dictLimit)) {` | branch-specific rejection/error | [ ] |
| 1089 | `ZSTD_DUBT_findBestMatch` (src/compress/zstd_lazy.c:335) | `if (matchIndex+matchLength >= dictLimit)` | branch-specific rejection/error | [ ] |
| 1090 | `ZSTD_DUBT_findBestMatch` (src/compress/zstd_lazy.c:339) | `if (matchLength > bestLength) {` | branch-specific rejection/error | [ ] |
| 1091 | `ZSTD_DUBT_findBestMatch` (src/compress/zstd_lazy.c:340) | `if (matchLength > matchEndIdx - matchIndex)` | branch-specific rejection/error | [ ] |
| 1092 | `ZSTD_DUBT_findBestMatch` (src/compress/zstd_lazy.c:342) | `if ( (4*(int)(matchLength-bestLength)) > (int)(ZSTD_highbit32(curr - matchIndex + 1) - ZSTD_highbit32((U32)*offBasePtr)) )` | branch-specific rejection/error | [ ] |
| 1093 | `ZSTD_DUBT_findBestMatch` (src/compress/zstd_lazy.c:354) | `if (match[matchLength] < ip[matchLength]) {` | branch-specific rejection/error | [ ] |
| 1094 | `ZSTD_DUBT_findBestMatch` (src/compress/zstd_lazy.c:358) | `if (matchIndex <= btLow) { smallerPtr=&dummy32; break; }   /* beyond tree size, stop the search */` | branch-specific rejection/error | [ ] |
| 1095 | `ZSTD_DUBT_findBestMatch` (src/compress/zstd_lazy.c:365) | `if (matchIndex <= btLow) { largerPtr=&dummy32; break; }   /* beyond tree size, stop the search */` | branch-specific rejection/error | [ ] |
| 1096 | `ZSTD_DUBT_findBestMatch` (src/compress/zstd_lazy.c:372) | `assert(nbCompares <= (1U << ZSTD_SEARCHLOG_MAX)); /* Check we haven't underflowed. */` | assertion failure | [ ] |
| 1097 | `ZSTD_DUBT_findBestMatch` (src/compress/zstd_lazy.c:380) | `assert(matchEndIdx > curr+8); /* ensure nextToUpdate is increased */` | assertion failure | [ ] |
| 1098 | `ZSTD_DUBT_findBestMatch` (src/compress/zstd_lazy.c:382) | `if (bestLength >= MINMATCH) {` | branch-specific rejection/error | [ ] |
| 1099 | `ZSTD_BtFindBestMatch` (src/compress/zstd_lazy.c:402) | `if (ip < ms->window.base + ms->nextToUpdate) return 0;   /* skipped area */` | branch-specific rejection/error | [ ] |
| 1100 | `ZSTD_dedicatedDictSearch_lazy_loadDictionary` (src/compress/zstd_lazy.c:437) | `assert(ms->cParams.chainLog <= 24);` | assertion failure | [ ] |
| 1101 | `ZSTD_dedicatedDictSearch_lazy_loadDictionary` (src/compress/zstd_lazy.c:438) | `assert(ms->cParams.hashLog > ms->cParams.chainLog);` | assertion failure | [ ] |
| 1102 | `ZSTD_dedicatedDictSearch_lazy_loadDictionary` (src/compress/zstd_lazy.c:439) | `assert(idx != 0);` | assertion failure | [ ] |
| 1103 | `ZSTD_dedicatedDictSearch_lazy_loadDictionary` (src/compress/zstd_lazy.c:440) | `assert(tmpMinChain <= minChain);` | assertion failure | [ ] |
| 1104 | `ZSTD_dedicatedDictSearch_lazy_loadDictionary` (src/compress/zstd_lazy.c:445) | `if (idx >= tmpMinChain) {` | branch-specific rejection/error | [ ] |
| 1105 | `ZSTD_dedicatedDictSearch_lazy_loadDictionary` (src/compress/zstd_lazy.c:461) | `if (i < minChain) {` | branch-specific rejection/error | [ ] |
| 1106 | `ZSTD_dedicatedDictSearch_lazy_loadDictionary` (src/compress/zstd_lazy.c:468) | `if (i < minChain) {` | branch-specific rejection/error | [ ] |
| 1107 | `ZSTD_dedicatedDictSearch_lazy_loadDictionary` (src/compress/zstd_lazy.c:469) | `if (!i \|\| ++countBeyondMinChain > cacheSize) {` | branch-specific rejection/error | [ ] |
| 1108 | `ZSTD_dedicatedDictSearch_lazy_loadDictionary` (src/compress/zstd_lazy.c:483) | `if (i < tmpMinChain) {` | branch-specific rejection/error | [ ] |
| 1109 | `ZSTD_dedicatedDictSearch_lazy_loadDictionary` (src/compress/zstd_lazy.c:497) | `assert(chainPos <= chainSize); /* I believe this is guaranteed... */` | assertion failure | [ ] |
| 1110 | `ZSTD_dedicatedDictSearch_lazy_search` (src/compress/zstd_lazy.c:567) | `assert(matchIndex >= ddsLowestIndex);` | assertion failure | [ ] |
| 1111 | `ZSTD_dedicatedDictSearch_lazy_search` (src/compress/zstd_lazy.c:568) | `assert(match+4 <= ddsEnd);` | assertion failure | [ ] |
| 1112 | `ZSTD_dedicatedDictSearch_lazy_search` (src/compress/zstd_lazy.c:575) | `if (currentMl > ml) {` | branch-specific rejection/error | [ ] |
| 1113 | `ZSTD_dedicatedDictSearch_lazy_search` (src/compress/zstd_lazy.c:604) | `assert(matchIndex >= ddsLowestIndex);` | assertion failure | [ ] |
| 1114 | `ZSTD_dedicatedDictSearch_lazy_search` (src/compress/zstd_lazy.c:605) | `assert(match+4 <= ddsEnd);` | assertion failure | [ ] |
| 1115 | `ZSTD_dedicatedDictSearch_lazy_search` (src/compress/zstd_lazy.c:612) | `if (currentMl > ml) {` | branch-specific rejection/error | [ ] |
| 1116 | `ZSTD_HcFindBestMatch` (src/compress/zstd_lazy.c:710) | `if ((dictMode != ZSTD_extDict) \|\| matchIndex >= dictLimit) {` | branch-specific rejection/error | [ ] |
| 1117 | `ZSTD_HcFindBestMatch` (src/compress/zstd_lazy.c:712) | `assert(matchIndex >= dictLimit);   /* ensures this is true if dictMode != ZSTD_extDict */` | assertion failure | [ ] |
| 1118 | `ZSTD_HcFindBestMatch` (src/compress/zstd_lazy.c:718) | `assert(match+4 <= dictEnd);` | assertion failure | [ ] |
| 1119 | `ZSTD_HcFindBestMatch` (src/compress/zstd_lazy.c:719) | `if (MEM_read32(match) == MEM_read32(ip))   /* assumption : matchIndex <= dictLimit-4 (by table construction) */` | branch-specific rejection/error | [ ] |
| 1120 | `ZSTD_HcFindBestMatch` (src/compress/zstd_lazy.c:724) | `if (currentMl > ml) {` | branch-specific rejection/error | [ ] |
| 1121 | `ZSTD_HcFindBestMatch` (src/compress/zstd_lazy.c:730) | `if (matchIndex <= minChain) break;` | branch-specific rejection/error | [ ] |
| 1122 | `ZSTD_HcFindBestMatch` (src/compress/zstd_lazy.c:734) | `assert(nbAttempts <= (1U << ZSTD_SEARCHLOG_MAX)); /* Check we haven't underflowed. */` | assertion failure | [ ] |
| 1123 | `ZSTD_HcFindBestMatch` (src/compress/zstd_lazy.c:754) | `assert(match+4 <= dmsEnd);` | assertion failure | [ ] |
| 1124 | `ZSTD_HcFindBestMatch` (src/compress/zstd_lazy.c:755) | `if (MEM_read32(match) == MEM_read32(ip))   /* assumption : matchIndex <= dictLimit-4 (by table construction) */` | branch-specific rejection/error | [ ] |
| 1125 | `ZSTD_HcFindBestMatch` (src/compress/zstd_lazy.c:759) | `if (currentMl > ml) {` | branch-specific rejection/error | [ ] |
| 1126 | `ZSTD_HcFindBestMatch` (src/compress/zstd_lazy.c:761) | `assert(curr > matchIndex + dmsIndexDelta);` | assertion failure | [ ] |
| 1127 | `ZSTD_HcFindBestMatch` (src/compress/zstd_lazy.c:766) | `if (matchIndex <= dmsMinChain) break;` | branch-specific rejection/error | [ ] |
| 1128 | `ZSTD_isAligned` (src/compress/zstd_lazy.c:809) | `assert((align & (align - 1)) == 0);` | assertion failure | [ ] |
| 1129 | `ZSTD_row_prefetch` (src/compress/zstd_lazy.c:818) | `if (rowLog >= 5) {` | branch-specific rejection/error | [ ] |
| 1130 | `ZSTD_row_prefetch` (src/compress/zstd_lazy.c:826) | `assert(rowLog == 4 \|\| rowLog == 5 \|\| rowLog == 6);` | assertion failure | [ ] |
| 1131 | `ZSTD_row_prefetch` (src/compress/zstd_lazy.c:827) | `assert(ZSTD_isAligned(hashTable + relRow, 64));                 /* prefetched hash row always 64-byte aligned */` | assertion failure | [ ] |
| 1132 | `ZSTD_row_prefetch` (src/compress/zstd_lazy.c:828) | `assert(ZSTD_isAligned(tagTable + relRow, (size_t)1 << rowLog)); /* prefetched tagRow sits on correct multiple of bytes (32,64,128) */` | assertion failure | [ ] |
| 1133 | `ZSTD_row_update_internalImpl` (src/compress/zstd_lazy.c:904) | `assert(hash == ZSTD_hashPtrSalted(base + updateStartIdx, hashLog + ZSTD_ROW_HASH_TAG_BITS, mls, ms->hashSalt));` | assertion failure | [ ] |
| 1134 | `ZSTD_row_update_internal` (src/compress/zstd_lazy.c:933) | `if (UNLIKELY(target - idx > kSkipThreshold)) {` | branch-specific rejection/error | [ ] |
| 1135 | `ZSTD_row_update_internal` (src/compress/zstd_lazy.c:940) | `assert(target >= idx);` | assertion failure | [ ] |
| 1136 | `ZSTD_row_matchMaskGroupWidth` (src/compress/zstd_lazy.c:965) | `assert((rowEntries == 16) \|\| (rowEntries == 32) \|\| rowEntries == 64);` | assertion failure | [ ] |
| 1137 | `ZSTD_row_matchMaskGroupWidth` (src/compress/zstd_lazy.c:966) | `assert(rowEntries <= ZSTD_ROW_HASH_MAX_ENTRIES);` | assertion failure | [ ] |
| 1138 | `ZSTD_row_getSSEMask` (src/compress/zstd_lazy.c:993) | `assert(nbChunks == 1 \|\| nbChunks == 2 \|\| nbChunks == 4);` | assertion failure | [ ] |
| 1139 | `ZSTD_row_getSSEMask` (src/compress/zstd_lazy.c:1000) | `if (nbChunks == 2) return ZSTD_rotateRight_U32((U32)matches[1] << 16 \| (U32)matches[0], head);` | branch-specific rejection/error | [ ] |
| 1140 | `ZSTD_row_getSSEMask` (src/compress/zstd_lazy.c:1001) | `assert(nbChunks == 4);` | assertion failure | [ ] |
| 1141 | `ZSTD_row_getNEONMask` (src/compress/zstd_lazy.c:1010) | `assert((rowEntries == 16) \|\| (rowEntries == 32) \|\| rowEntries == 64);` | assertion failure | [ ] |
| 1142 | `ZSTD_row_getMatchMask` (src/compress/zstd_lazy.c:1064) | `assert((rowEntries == 16) \|\| (rowEntries == 32) \|\| rowEntries == 64);` | assertion failure | [ ] |
| 1143 | `ZSTD_row_getMatchMask` (src/compress/zstd_lazy.c:1065) | `assert(rowEntries <= ZSTD_ROW_HASH_MAX_ENTRIES);` | assertion failure | [ ] |
| 1144 | `ZSTD_row_getMatchMask` (src/compress/zstd_lazy.c:1066) | `assert(ZSTD_row_matchMaskGroupWidth(rowEntries) * rowEntries <= sizeof(ZSTD_VecMask) * 8);` | assertion failure | [ ] |
| 1145 | `ZSTD_row_getMatchMask` (src/compress/zstd_lazy.c:1089) | `assert((sizeof(size_t) == 4) \|\| (sizeof(size_t) == 8));` | assertion failure | [ ] |
| 1146 | `ZSTD_RowFindBestMatch` (src/compress/zstd_lazy.c:1205) | `if (!ms->lazySkipping) {` | branch-specific rejection/error | [ ] |
| 1147 | `ZSTD_RowFindBestMatch` (src/compress/zstd_lazy.c:1232) | `if(matchPos == 0) continue;` | branch-specific rejection/error | [ ] |
| 1148 | `ZSTD_RowFindBestMatch` (src/compress/zstd_lazy.c:1233) | `assert(numMatches < rowEntries);` | assertion failure | [ ] |
| 1149 | `ZSTD_RowFindBestMatch` (src/compress/zstd_lazy.c:1234) | `if (matchIndex < lowLimit)` | branch-specific rejection/error | [ ] |
| 1150 | `ZSTD_RowFindBestMatch` (src/compress/zstd_lazy.c:1236) | `if ((dictMode != ZSTD_extDict) \|\| matchIndex >= dictLimit) {` | branch-specific rejection/error | [ ] |
| 1151 | `ZSTD_RowFindBestMatch` (src/compress/zstd_lazy.c:1257) | `assert(matchIndex < curr);` | assertion failure | [ ] |
| 1152 | `ZSTD_RowFindBestMatch` (src/compress/zstd_lazy.c:1258) | `assert(matchIndex >= lowLimit);` | assertion failure | [ ] |
| 1153 | `ZSTD_RowFindBestMatch` (src/compress/zstd_lazy.c:1260) | `if ((dictMode != ZSTD_extDict) \|\| matchIndex >= dictLimit) {` | branch-specific rejection/error | [ ] |
| 1154 | `ZSTD_RowFindBestMatch` (src/compress/zstd_lazy.c:1262) | `assert(matchIndex >= dictLimit);   /* ensures this is true if dictMode != ZSTD_extDict */` | assertion failure | [ ] |
| 1155 | `ZSTD_RowFindBestMatch` (src/compress/zstd_lazy.c:1268) | `assert(match+4 <= dictEnd);` | assertion failure | [ ] |
| 1156 | `ZSTD_RowFindBestMatch` (src/compress/zstd_lazy.c:1269) | `if (MEM_read32(match) == MEM_read32(ip))   /* assumption : matchIndex <= dictLimit-4 (by table construction) */` | branch-specific rejection/error | [ ] |
| 1157 | `ZSTD_RowFindBestMatch` (src/compress/zstd_lazy.c:1274) | `if (currentMl > ml) {` | branch-specific rejection/error | [ ] |
| 1158 | `ZSTD_RowFindBestMatch` (src/compress/zstd_lazy.c:1282) | `assert(nbAttempts <= (1U << ZSTD_SEARCHLOG_MAX)); /* Check we haven't underflowed. */` | assertion failure | [ ] |
| 1159 | `ZSTD_RowFindBestMatch` (src/compress/zstd_lazy.c:1303) | `if(matchPos == 0) continue;` | branch-specific rejection/error | [ ] |
| 1160 | `ZSTD_RowFindBestMatch` (src/compress/zstd_lazy.c:1304) | `if (matchIndex < dmsLowestIndex)` | branch-specific rejection/error | [ ] |
| 1161 | `ZSTD_RowFindBestMatch` (src/compress/zstd_lazy.c:1315) | `assert(matchIndex >= dmsLowestIndex);` | assertion failure | [ ] |
| 1162 | `ZSTD_RowFindBestMatch` (src/compress/zstd_lazy.c:1316) | `assert(matchIndex < curr);` | assertion failure | [ ] |
| 1163 | `ZSTD_RowFindBestMatch` (src/compress/zstd_lazy.c:1319) | `assert(match+4 <= dmsEnd);` | assertion failure | [ ] |
| 1164 | `ZSTD_RowFindBestMatch` (src/compress/zstd_lazy.c:1324) | `if (currentMl > ml) {` | branch-specific rejection/error | [ ] |
| 1165 | `ZSTD_RowFindBestMatch` (src/compress/zstd_lazy.c:1326) | `assert(curr > matchIndex + dmsIndexDelta);` | assertion failure | [ ] |
| 1166 | `ZSTD_RowFindBestMatch` (src/compress/zstd_lazy.c:1371) | `assert(MAX(4, MIN(6, ms->cParams.minMatch)) == mls);                           \` | assertion failure | [ ] |
| 1167 | `ZSTD_RowFindBestMatch` (src/compress/zstd_lazy.c:1381) | `assert(MAX(4, MIN(6, ms->cParams.minMatch)) == mls);                          \` | assertion failure | [ ] |
| 1168 | `ZSTD_RowFindBestMatch` (src/compress/zstd_lazy.c:1391) | `assert(MAX(4, MIN(6, ms->cParams.minMatch)) == mls);                                   \` | assertion failure | [ ] |
| 1169 | `ZSTD_RowFindBestMatch` (src/compress/zstd_lazy.c:1392) | `assert(MAX(4, MIN(6, ms->cParams.searchLog)) == rowLog);                               \` | assertion failure | [ ] |
| 1170 | `ZSTD_compressBlock_lazy_generic` (src/compress/zstd_lazy.c:1556) | `if (offset_2 > maxRep) offsetSaved2 = offset_2, offset_2 = 0;` | branch-specific rejection/error | [ ] |
| 1171 | `ZSTD_compressBlock_lazy_generic` (src/compress/zstd_lazy.c:1557) | `if (offset_1 > maxRep) offsetSaved1 = offset_1, offset_1 = 0;` | branch-specific rejection/error | [ ] |
| 1172 | `ZSTD_compressBlock_lazy_generic` (src/compress/zstd_lazy.c:1562) | `assert(offset_1 <= dictAndPrefixLength);` | assertion failure | [ ] |
| 1173 | `ZSTD_compressBlock_lazy_generic` (src/compress/zstd_lazy.c:1563) | `assert(offset_2 <= dictAndPrefixLength);` | assertion failure | [ ] |
| 1174 | `ZSTD_compressBlock_lazy_generic` (src/compress/zstd_lazy.c:1597) | `if (depth==0) goto _storeSequence;` | branch-specific rejection/error | [ ] |
| 1175 | `ZSTD_compressBlock_lazy_generic` (src/compress/zstd_lazy.c:1603) | `if (depth==0) goto _storeSequence;` | branch-specific rejection/error | [ ] |
| 1176 | `ZSTD_compressBlock_lazy_generic` (src/compress/zstd_lazy.c:1609) | `if (ml2 > matchLength)` | branch-specific rejection/error | [ ] |
| 1177 | `ZSTD_compressBlock_lazy_generic` (src/compress/zstd_lazy.c:1613) | `if (matchLength < 4) {` | branch-specific rejection/error | [ ] |
| 1178 | `ZSTD_compressBlock_lazy_generic` (src/compress/zstd_lazy.c:1628) | `if (depth>=1)` | branch-specific rejection/error | [ ] |
| 1179 | `ZSTD_compressBlock_lazy_generic` (src/compress/zstd_lazy.c:1637) | `if ((mlRep >= 4) && (gain2 > gain1))` | branch-specific rejection/error | [ ] |
| 1180 | `ZSTD_compressBlock_lazy_generic` (src/compress/zstd_lazy.c:1651) | `if ((mlRep >= 4) && (gain2 > gain1))` | branch-specific rejection/error | [ ] |
| 1181 | `ZSTD_compressBlock_lazy_generic` (src/compress/zstd_lazy.c:1659) | `if ((ml2 >= 4) && (gain2 > gain1)) {` | branch-specific rejection/error | [ ] |
| 1182 | `ZSTD_compressBlock_lazy_generic` (src/compress/zstd_lazy.c:1665) | `if ((depth==2) && (ip<ilimit)) {` | branch-specific rejection/error | [ ] |
| 1183 | `ZSTD_compressBlock_lazy_generic` (src/compress/zstd_lazy.c:1673) | `if ((mlRep >= 4) && (gain2 > gain1))` | branch-specific rejection/error | [ ] |
| 1184 | `ZSTD_compressBlock_lazy_generic` (src/compress/zstd_lazy.c:1687) | `if ((mlRep >= 4) && (gain2 > gain1))` | branch-specific rejection/error | [ ] |
| 1185 | `ZSTD_compressBlock_lazy_generic` (src/compress/zstd_lazy.c:1695) | `if ((ml2 >= 4) && (gain2 > gain1)) {` | branch-specific rejection/error | [ ] |
| 1186 | `ZSTD_compressBlock_lazy_generic` (src/compress/zstd_lazy.c:1727) | `if (ms->lazySkipping) {` | branch-specific rejection/error | [ ] |
| 1187 | `ZSTD_compressBlock_lazy_extDict_generic` (src/compress/zstd_lazy.c:1995) | `if (depth==0) goto _storeSequence;` | branch-specific rejection/error | [ ] |
| 1188 | `ZSTD_compressBlock_lazy_extDict_generic` (src/compress/zstd_lazy.c:2001) | `if (ml2 > matchLength)` | branch-specific rejection/error | [ ] |
| 1189 | `ZSTD_compressBlock_lazy_extDict_generic` (src/compress/zstd_lazy.c:2005) | `if (matchLength < 4) {` | branch-specific rejection/error | [ ] |
| 1190 | `ZSTD_compressBlock_lazy_extDict_generic` (src/compress/zstd_lazy.c:2020) | `if (depth>=1)` | branch-specific rejection/error | [ ] |
| 1191 | `ZSTD_compressBlock_lazy_extDict_generic` (src/compress/zstd_lazy.c:2038) | `if ((repLength >= 4) && (gain2 > gain1))` | branch-specific rejection/error | [ ] |
| 1192 | `ZSTD_compressBlock_lazy_extDict_generic` (src/compress/zstd_lazy.c:2047) | `if ((ml2 >= 4) && (gain2 > gain1)) {` | branch-specific rejection/error | [ ] |
| 1193 | `ZSTD_compressBlock_lazy_extDict_generic` (src/compress/zstd_lazy.c:2053) | `if ((depth==2) && (ip<ilimit)) {` | branch-specific rejection/error | [ ] |
| 1194 | `ZSTD_compressBlock_lazy_extDict_generic` (src/compress/zstd_lazy.c:2070) | `if ((repLength >= 4) && (gain2 > gain1))` | branch-specific rejection/error | [ ] |
| 1195 | `ZSTD_compressBlock_lazy_extDict_generic` (src/compress/zstd_lazy.c:2079) | `if ((ml2 >= 4) && (gain2 > gain1)) {` | branch-specific rejection/error | [ ] |
| 1196 | `ZSTD_compressBlock_lazy_extDict_generic` (src/compress/zstd_lazy.c:2101) | `if (ms->lazySkipping) {` | branch-specific rejection/error | [ ] |
| 1197 | `ZSTD_ldm_gear_init` (src/compress/zstd_ldm.c:52) | `if (hashRateLog > 0 && hashRateLog <= maxBitsInMask) {` | branch-specific rejection/error | [ ] |
| 1198 | `ZSTD_ldm_gear_feed` (src/compress/zstd_ldm.c:110) | `if (UNLIKELY((hash & mask) == 0)) { \` | branch-specific rejection/error | [ ] |
| 1199 | `ZSTD_ldm_adjustParameters` (src/compress/zstd_ldm.c:141) | `if (params->hashRateLog == 0) {` | branch-specific rejection/error | [ ] |
| 1200 | `ZSTD_ldm_adjustParameters` (src/compress/zstd_ldm.c:142) | `if (params->hashLog > 0) {` | branch-specific rejection/error | [ ] |
| 1201 | `ZSTD_ldm_adjustParameters` (src/compress/zstd_ldm.c:144) | `assert(params->hashLog <= ZSTD_HASHLOG_MAX);` | assertion failure | [ ] |
| 1202 | `ZSTD_ldm_adjustParameters` (src/compress/zstd_ldm.c:145) | `if (params->windowLog > params->hashLog) {` | branch-specific rejection/error | [ ] |
| 1203 | `ZSTD_ldm_adjustParameters` (src/compress/zstd_ldm.c:149) | `assert(1 <= (int)cParams->strategy && (int)cParams->strategy <= 9);` | assertion failure | [ ] |
| 1204 | `ZSTD_ldm_adjustParameters` (src/compress/zstd_ldm.c:154) | `if (params->hashLog == 0) {` | branch-specific rejection/error | [ ] |
| 1205 | `ZSTD_ldm_adjustParameters` (src/compress/zstd_ldm.c:157) | `if (params->minMatchLength == 0) {` | branch-specific rejection/error | [ ] |
| 1206 | `ZSTD_ldm_adjustParameters` (src/compress/zstd_ldm.c:159) | `if (cParams->strategy >= ZSTD_btultra)` | branch-specific rejection/error | [ ] |
| 1207 | `ZSTD_ldm_adjustParameters` (src/compress/zstd_ldm.c:162) | `if (params->bucketSizeLog==0) {` | branch-specific rejection/error | [ ] |
| 1208 | `ZSTD_ldm_adjustParameters` (src/compress/zstd_ldm.c:163) | `assert(1 <= (int)cParams->strategy && (int)cParams->strategy <= 9);` | assertion failure | [ ] |
| 1209 | `ZSTD_ldm_fillFastTables` (src/compress/zstd_ldm.c:266) | `assert(0); /* shouldn't be called: cparams should've been adjusted. */` | assertion failure | [ ] |
| 1210 | `ZSTD_ldm_fillFastTables` (src/compress/zstd_ldm.c:279) | `assert(0);  /* not possible : not a valid strategy id */` | assertion failure | [ ] |
| 1211 | `ZSTD_ldm_fillHashTable` (src/compress/zstd_ldm.c:309) | `if (ip + splits[n] >= istart + minMatchLength) {` | branch-specific rejection/error | [ ] |
| 1212 | `ZSTD_ldm_limitTableUpdate` (src/compress/zstd_ldm.c:334) | `if (curr > ms->nextToUpdate + 1024) {` | branch-specific rejection/error | [ ] |
| 1213 | `ZSTD_ldm_generateSequences_internal` (src/compress/zstd_ldm.c:373) | `if (srcSize < minMatchLength)` | branch-specific rejection/error | [ ] |
| 1214 | `ZSTD_ldm_generateSequences_internal` (src/compress/zstd_ldm.c:419) | `if (split < anchor) {` | branch-specific rejection/error | [ ] |
| 1215 | `ZSTD_ldm_generateSequences_internal` (src/compress/zstd_ldm.c:427) | `if (cur->checksum != checksum \|\| cur->offset <= lowestIndex) {` | branch-specific rejection/error | [ ] |
| 1216 | `ZSTD_ldm_generateSequences_internal` (src/compress/zstd_ldm.c:440) | `if (curForwardMatchLength < minMatchLength) {` | branch-specific rejection/error | [ ] |
| 1217 | `ZSTD_ldm_generateSequences_internal` (src/compress/zstd_ldm.c:448) | `if (curForwardMatchLength < minMatchLength) {` | branch-specific rejection/error | [ ] |
| 1218 | `ZSTD_ldm_generateSequences_internal` (src/compress/zstd_ldm.c:456) | `if (curTotalMatchLength > bestMatchLength) {` | branch-specific rejection/error | [ ] |
| 1219 | `ZSTD_ldm_generateSequences_internal` (src/compress/zstd_ldm.c:466) | `if (bestEntry == NULL) {` | branch-specific rejection/error | [ ] |
| 1220 | `ZSTD_ldm_generateSequences_internal` (src/compress/zstd_ldm.c:478) | `if (rawSeqStore->size == rawSeqStore->capacity)` | `ERROR(dstSize_tooSmall)` | [ ] |
| 1221 | `ZSTD_ldm_generateSequences_internal` (src/compress/zstd_ldm.c:479) | `return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [ ] |
| 1222 | `ZSTD_ldm_generateSequences_internal` (src/compress/zstd_ldm.c:500) | `if (anchor > ip + hashed) {` | branch-specific rejection/error | [ ] |
| 1223 | `ZSTD_ldm_reduceTable` (src/compress/zstd_ldm.c:521) | `if (table[u].offset < reducerValue) table[u].offset = 0;` | branch-specific rejection/error | [ ] |
| 1224 | `ZSTD_ldm_generateSequences` (src/compress/zstd_ldm.c:538) | `assert(ZSTD_CHUNKSIZE_MAX >= kMaxChunkSize);` | assertion failure | [ ] |
| 1225 | `ZSTD_ldm_generateSequences` (src/compress/zstd_ldm.c:542) | `assert(ldmState->window.nextSrc >= (BYTE const*)src + srcSize);` | assertion failure | [ ] |
| 1226 | `ZSTD_ldm_generateSequences` (src/compress/zstd_ldm.c:546) | `assert(sequences->pos <= sequences->size);` | assertion failure | [ ] |
| 1227 | `ZSTD_ldm_generateSequences` (src/compress/zstd_ldm.c:547) | `assert(sequences->size <= sequences->capacity);` | assertion failure | [ ] |
| 1228 | `ZSTD_ldm_generateSequences` (src/compress/zstd_ldm.c:557) | `assert(chunkStart < iend);` | assertion failure | [ ] |
| 1229 | `ZSTD_ldm_generateSequences` (src/compress/zstd_ldm.c:559) | `if (ZSTD_window_needOverflowCorrection(ldmState->window, 0, maxDist, ldmState->loadedDictEnd, chunkStart, chunkEnd)) {` | branch-specific rejection/error | [ ] |
| 1230 | `ZSTD_ldm_generateSequences` (src/compress/zstd_ldm.c:592) | `if (prevSize < sequences->size) {` | branch-specific rejection/error | [ ] |
| 1231 | `ZSTD_ldm_generateSequences` (src/compress/zstd_ldm.c:596) | `assert(newLeftoverSize == chunkSize);` | assertion failure | [ ] |
| 1232 | `ZSTD_ldm_skipSequences` (src/compress/zstd_ldm.c:608) | `if (srcSize <= seq->litLength) {` | branch-specific rejection/error | [ ] |
| 1233 | `ZSTD_ldm_skipSequences` (src/compress/zstd_ldm.c:615) | `if (srcSize < seq->matchLength) {` | branch-specific rejection/error | [ ] |
| 1234 | `ZSTD_ldm_skipSequences` (src/compress/zstd_ldm.c:618) | `if (seq->matchLength < minMatch) {` | branch-specific rejection/error | [ ] |
| 1235 | `ZSTD_ldm_skipSequences` (src/compress/zstd_ldm.c:620) | `if (rawSeqStore->pos + 1 < rawSeqStore->size) {` | branch-specific rejection/error | [ ] |
| 1236 | `maybeSplitSequence` (src/compress/zstd_ldm.c:644) | `assert(sequence.offset > 0);` | assertion failure | [ ] |
| 1237 | `maybeSplitSequence` (src/compress/zstd_ldm.c:646) | `if (remaining >= sequence.litLength + sequence.matchLength) {` | branch-specific rejection/error | [ ] |
| 1238 | `maybeSplitSequence` (src/compress/zstd_ldm.c:651) | `if (remaining <= sequence.litLength) {` | branch-specific rejection/error | [ ] |
| 1239 | `maybeSplitSequence` (src/compress/zstd_ldm.c:653) | `} else if (remaining < sequence.litLength + sequence.matchLength) {` | branch-specific rejection/error | [ ] |
| 1240 | `maybeSplitSequence` (src/compress/zstd_ldm.c:655) | `if (sequence.matchLength < minMatch) {` | branch-specific rejection/error | [ ] |
| 1241 | `ZSTD_ldm_skipRawSeqStoreBytes` (src/compress/zstd_ldm.c:668) | `if (currPos >= currSeq.litLength + currSeq.matchLength) {` | branch-specific rejection/error | [ ] |
| 1242 | `ZSTD_ldm_skipRawSeqStoreBytes` (src/compress/zstd_ldm.c:676) | `if (currPos == 0 \|\| rawSeqStore->pos == rawSeqStore->size) {` | branch-specific rejection/error | [ ] |
| 1243 | `ZSTD_ldm_blockCompress` (src/compress/zstd_ldm.c:698) | `if (cParams->strategy >= ZSTD_btopt) {` | branch-specific rejection/error | [ ] |
| 1244 | `ZSTD_ldm_blockCompress` (src/compress/zstd_ldm.c:706) | `assert(rawSeqStore->pos <= rawSeqStore->size);` | assertion failure | [ ] |
| 1245 | `ZSTD_ldm_blockCompress` (src/compress/zstd_ldm.c:707) | `assert(rawSeqStore->size <= rawSeqStore->capacity);` | assertion failure | [ ] |
| 1246 | `ZSTD_ldm_blockCompress` (src/compress/zstd_ldm.c:714) | `if (sequence.offset == 0)` | branch-specific rejection/error | [ ] |
| 1247 | `ZSTD_ldm_blockCompress` (src/compress/zstd_ldm.c:717) | `assert(ip + sequence.litLength + sequence.matchLength <= iend);` | assertion failure | [ ] |
| 1248 | `ZSTD_fracWeight` (src/compress/zstd_opt.c:63) | `assert(hb + BITCOST_ACCURACY < 31);` | assertion failure | [ ] |
| 1249 | `ZSTD_downscaleStats` (src/compress/zstd_opt.c:110) | `assert(shift < 30);` | assertion failure | [ ] |
| 1250 | `ZSTD_scaleStats` (src/compress/zstd_opt.c:128) | `assert(logTarget < 30);` | assertion failure | [ ] |
| 1251 | `ZSTD_scaleStats` (src/compress/zstd_opt.c:129) | `if (factor <= 1) return prevsum;` | branch-specific rejection/error | [ ] |
| 1252 | `ZSTD_rescaleFreqs` (src/compress/zstd_opt.c:149) | `if (optPtr->litLengthSum == 0) {  /* no literals stats collected -> first block assumed -> init */` | branch-specific rejection/error | [ ] |
| 1253 | `ZSTD_rescaleFreqs` (src/compress/zstd_opt.c:152) | `if (srcSize <= ZSTD_PREDEF_THRESHOLD) {` | branch-specific rejection/error | [ ] |
| 1254 | `ZSTD_rescaleFreqs` (src/compress/zstd_opt.c:157) | `assert(optPtr->symbolCosts != NULL);` | assertion failure | [ ] |
| 1255 | `ZSTD_rescaleFreqs` (src/compress/zstd_opt.c:158) | `if (optPtr->symbolCosts->huf.repeatMode == HUF_repeat_valid) {` | branch-specific rejection/error | [ ] |
| 1256 | `ZSTD_rescaleFreqs` (src/compress/zstd_opt.c:166) | `assert(optPtr->litFreq != NULL);` | assertion failure | [ ] |
| 1257 | `ZSTD_rescaleFreqs` (src/compress/zstd_opt.c:171) | `assert(bitCost <= scaleLog);` | assertion failure | [ ] |
| 1258 | `ZSTD_rescaleFreqs` (src/compress/zstd_opt.c:183) | `assert(bitCost < scaleLog);` | assertion failure | [ ] |
| 1259 | `ZSTD_rescaleFreqs` (src/compress/zstd_opt.c:195) | `assert(bitCost < scaleLog);` | assertion failure | [ ] |
| 1260 | `ZSTD_rescaleFreqs` (src/compress/zstd_opt.c:207) | `assert(bitCost < scaleLog);` | assertion failure | [ ] |
| 1261 | `ZSTD_rescaleFreqs` (src/compress/zstd_opt.c:214) | `assert(optPtr->litFreq != NULL);` | assertion failure | [ ] |
| 1262 | `ZSTD_rawLiteralsCost` (src/compress/zstd_opt.c:271) | `if (litLength == 0) return 0;` | branch-specific rejection/error | [ ] |
| 1263 | `ZSTD_rawLiteralsCost` (src/compress/zstd_opt.c:276) | `if (optPtr->priceType == zop_predef)` | branch-specific rejection/error | [ ] |
| 1264 | `ZSTD_rawLiteralsCost` (src/compress/zstd_opt.c:283) | `assert(optPtr->litSumBasePrice >= BITCOST_MULTIPLIER);` | assertion failure | [ ] |
| 1265 | `ZSTD_rawLiteralsCost` (src/compress/zstd_opt.c:286) | `if (UNLIKELY(litPrice > litPriceMax)) litPrice = litPriceMax;` | branch-specific rejection/error | [ ] |
| 1266 | `ZSTD_litLengthPrice` (src/compress/zstd_opt.c:297) | `assert(litLength <= ZSTD_BLOCKSIZE_MAX);` | assertion failure | [ ] |
| 1267 | `ZSTD_litLengthPrice` (src/compress/zstd_opt.c:298) | `if (optPtr->priceType == zop_predef)` | branch-specific rejection/error | [ ] |
| 1268 | `ZSTD_getMatchPrice` (src/compress/zstd_opt.c:332) | `assert(matchLength >= MINMATCH);` | assertion failure | [ ] |
| 1269 | `ZSTD_getMatchPrice` (src/compress/zstd_opt.c:334) | `if (optPtr->priceType == zop_predef)  /* fixed scheme, does not use statistics */` | branch-specific rejection/error | [ ] |
| 1270 | `ZSTD_getMatchPrice` (src/compress/zstd_opt.c:340) | `if ((optLevel<2) /*static*/ && offCode >= 20)` | branch-specific rejection/error | [ ] |
| 1271 | `ZSTD_updateStats` (src/compress/zstd_opt.c:376) | `assert(offCode <= MaxOff);` | assertion failure | [ ] |
| 1272 | `ZSTD_insertAndFindFirstIndexHash3` (src/compress/zstd_opt.c:421) | `assert(hashLog3 > 0);` | assertion failure | [ ] |
| 1273 | `ZSTD_insertBt1` (src/compress/zstd_opt.c:484) | `assert(curr <= target);` | assertion failure | [ ] |
| 1274 | `ZSTD_insertBt1` (src/compress/zstd_opt.c:485) | `assert(ip <= iend-8);   /* required for h calculation */` | assertion failure | [ ] |
| 1275 | `ZSTD_insertBt1` (src/compress/zstd_opt.c:488) | `assert(windowLow > 0);` | assertion failure | [ ] |
| 1276 | `ZSTD_insertBt1` (src/compress/zstd_opt.c:492) | `assert(matchIndex < curr);` | assertion failure | [ ] |
| 1277 | `ZSTD_insertBt1` (src/compress/zstd_opt.c:499) | `if (matchIndex <= btLow) { smallerPtr=&dummy32; break; }   /* beyond tree size, stop the search */` | branch-specific rejection/error | [ ] |
| 1278 | `ZSTD_insertBt1` (src/compress/zstd_opt.c:507) | `if (matchIndex <= btLow) { largerPtr=&dummy32; break; }   /* beyond tree size, stop the search */` | branch-specific rejection/error | [ ] |
| 1279 | `ZSTD_insertBt1` (src/compress/zstd_opt.c:515) | `if (!extDict \|\| (matchIndex+matchLength >= dictLimit)) {` | branch-specific rejection/error | [ ] |
| 1280 | `ZSTD_insertBt1` (src/compress/zstd_opt.c:516) | `assert(matchIndex+matchLength >= dictLimit);   /* might be wrong if actually extDict */` | assertion failure | [ ] |
| 1281 | `ZSTD_insertBt1` (src/compress/zstd_opt.c:522) | `if (matchIndex+matchLength >= dictLimit)` | branch-specific rejection/error | [ ] |
| 1282 | `ZSTD_insertBt1` (src/compress/zstd_opt.c:526) | `if (matchLength > bestLength) {` | branch-specific rejection/error | [ ] |
| 1283 | `ZSTD_insertBt1` (src/compress/zstd_opt.c:528) | `if (matchLength > matchEndIdx - matchIndex)` | branch-specific rejection/error | [ ] |
| 1284 | `ZSTD_insertBt1` (src/compress/zstd_opt.c:536) | `if (match[matchLength] < ip[matchLength]) {  /* necessarily within buffer */` | branch-specific rejection/error | [ ] |
| 1285 | `ZSTD_insertBt1` (src/compress/zstd_opt.c:540) | `if (matchIndex <= btLow) { smallerPtr=&dummy32; break; }   /* beyond tree size, stop searching */` | branch-specific rejection/error | [ ] |
| 1286 | `ZSTD_insertBt1` (src/compress/zstd_opt.c:547) | `if (matchIndex <= btLow) { largerPtr=&dummy32; break; }   /* beyond tree size, stop searching */` | branch-specific rejection/error | [ ] |
| 1287 | `ZSTD_insertBt1` (src/compress/zstd_opt.c:554) | `if (bestLength > 384) positions = MIN(192, (U32)(bestLength - 384));   /* speed optimization */` | branch-specific rejection/error | [ ] |
| 1288 | `ZSTD_insertBt1` (src/compress/zstd_opt.c:555) | `assert(matchEndIdx > curr + 8);` | assertion failure | [ ] |
| 1289 | `ZSTD_updateTree_internal` (src/compress/zstd_opt.c:575) | `assert(idx < (U32)(idx + forward));` | assertion failure | [ ] |
| 1290 | `ZSTD_updateTree_internal` (src/compress/zstd_opt.c:578) | `assert((size_t)(ip - base) <= (size_t)(U32)(-1));` | assertion failure | [ ] |
| 1291 | `ZSTD_updateTree_internal` (src/compress/zstd_opt.c:579) | `assert((size_t)(iend - base) <= (size_t)(U32)(-1));` | assertion failure | [ ] |
| 1292 | `ZSTD_insertBtAndGetAllMatches` (src/compress/zstd_opt.c:645) | `assert(ll0 <= 1);   /* necessarily 1 or 0 */` | assertion failure | [ ] |
| 1293 | `ZSTD_insertBtAndGetAllMatches` (src/compress/zstd_opt.c:652) | `assert(curr >= dictLimit);` | assertion failure | [ ] |
| 1294 | `ZSTD_insertBtAndGetAllMatches` (src/compress/zstd_opt.c:653) | `if (repOffset-1 /* intentional overflow, discards 0 and -1 */ < curr-dictLimit) {  /* equivalent to 'curr > repIndex >= dictLimit' */` | branch-specific rejection/error | [ ] |
| 1295 | `ZSTD_insertBtAndGetAllMatches` (src/compress/zstd_opt.c:657) | `if ((repIndex >= windowLow) & (ZSTD_readMINMATCH(ip, minMatch) == ZSTD_readMINMATCH(ip - repOffset, minMatch))) {` | branch-specific rejection/error | [ ] |
| 1296 | `ZSTD_insertBtAndGetAllMatches` (src/compress/zstd_opt.c:664) | `assert(curr >= windowLow);` | assertion failure | [ ] |
| 1297 | `ZSTD_insertBtAndGetAllMatches` (src/compress/zstd_opt.c:678) | `if (repLen > bestLength) {` | branch-specific rejection/error | [ ] |
| 1298 | `ZSTD_insertBtAndGetAllMatches` (src/compress/zstd_opt.c:685) | `if ( (repLen > sufficient_len)` | branch-specific rejection/error | [ ] |
| 1299 | `ZSTD_insertBtAndGetAllMatches` (src/compress/zstd_opt.c:691) | `if ((mls == 3) /*static*/ && (bestLength < mls)) {` | branch-specific rejection/error | [ ] |
| 1300 | `ZSTD_insertBtAndGetAllMatches` (src/compress/zstd_opt.c:693) | `if ((matchIndex3 >= matchLow)` | branch-specific rejection/error | [ ] |
| 1301 | `ZSTD_insertBtAndGetAllMatches` (src/compress/zstd_opt.c:696) | `if ((dictMode == ZSTD_noDict) /*static*/ \|\| (dictMode == ZSTD_dictMatchState) /*static*/ \|\| (matchIndex3 >= dictLimit)) {` | branch-specific rejection/error | [ ] |
| 1302 | `ZSTD_insertBtAndGetAllMatches` (src/compress/zstd_opt.c:705) | `if (mlen >= mls /* == 3 > bestLength */) {` | branch-specific rejection/error | [ ] |
| 1303 | `ZSTD_insertBtAndGetAllMatches` (src/compress/zstd_opt.c:709) | `assert(curr > matchIndex3);` | assertion failure | [ ] |
| 1304 | `ZSTD_insertBtAndGetAllMatches` (src/compress/zstd_opt.c:710) | `assert(mnum==0);  /* no prior solution */` | assertion failure | [ ] |
| 1305 | `ZSTD_insertBtAndGetAllMatches` (src/compress/zstd_opt.c:714) | `if ( (mlen > sufficient_len) \|` | branch-specific rejection/error | [ ] |
| 1306 | `ZSTD_insertBtAndGetAllMatches` (src/compress/zstd_opt.c:728) | `assert(curr > matchIndex);` | assertion failure | [ ] |
| 1307 | `ZSTD_insertBtAndGetAllMatches` (src/compress/zstd_opt.c:730) | `if ((dictMode == ZSTD_noDict) \|\| (dictMode == ZSTD_dictMatchState) \|\| (matchIndex+matchLength >= dictLimit)) {` | branch-specific rejection/error | [ ] |
| 1308 | `ZSTD_insertBtAndGetAllMatches` (src/compress/zstd_opt.c:731) | `assert(matchIndex+matchLength >= dictLimit);  /* ensure the condition is correct when !extDict */` | assertion failure | [ ] |
| 1309 | `ZSTD_insertBtAndGetAllMatches` (src/compress/zstd_opt.c:733) | `if (matchIndex >= dictLimit) assert(memcmp(match, ip, matchLength) == 0);  /* ensure early section of match is equal as expected */` | assertion failure | [ ] |
| 1310 | `ZSTD_insertBtAndGetAllMatches` (src/compress/zstd_opt.c:737) | `assert(memcmp(match, ip, matchLength) == 0);  /* ensure early section of match is equal as expected */` | assertion failure | [ ] |
| 1311 | `ZSTD_insertBtAndGetAllMatches` (src/compress/zstd_opt.c:739) | `if (matchIndex+matchLength >= dictLimit)` | branch-specific rejection/error | [ ] |
| 1312 | `ZSTD_insertBtAndGetAllMatches` (src/compress/zstd_opt.c:743) | `if (matchLength > bestLength) {` | branch-specific rejection/error | [ ] |
| 1313 | `ZSTD_insertBtAndGetAllMatches` (src/compress/zstd_opt.c:746) | `assert(matchEndIdx > matchIndex);` | assertion failure | [ ] |
| 1314 | `ZSTD_insertBtAndGetAllMatches` (src/compress/zstd_opt.c:747) | `if (matchLength > matchEndIdx - matchIndex)` | branch-specific rejection/error | [ ] |
| 1315 | `ZSTD_insertBtAndGetAllMatches` (src/compress/zstd_opt.c:753) | `if ( (matchLength > ZSTD_OPT_NUM)` | branch-specific rejection/error | [ ] |
| 1316 | `ZSTD_insertBtAndGetAllMatches` (src/compress/zstd_opt.c:759) | `if (match[matchLength] < ip[matchLength]) {` | branch-specific rejection/error | [ ] |
| 1317 | `ZSTD_insertBtAndGetAllMatches` (src/compress/zstd_opt.c:763) | `if (matchIndex <= btLow) { smallerPtr=&dummy32; break; }   /* beyond tree size, stop the search */` | branch-specific rejection/error | [ ] |
| 1318 | `ZSTD_insertBtAndGetAllMatches` (src/compress/zstd_opt.c:769) | `if (matchIndex <= btLow) { largerPtr=&dummy32; break; }   /* beyond tree size, stop the search */` | branch-specific rejection/error | [ ] |
| 1319 | `ZSTD_insertBtAndGetAllMatches` (src/compress/zstd_opt.c:776) | `assert(nbCompares <= (1U << ZSTD_SEARCHLOG_MAX)); /* Check we haven't underflowed. */` | assertion failure | [ ] |
| 1320 | `ZSTD_insertBtAndGetAllMatches` (src/compress/zstd_opt.c:787) | `if (dictMatchIndex+matchLength >= dmsHighLimit)` | branch-specific rejection/error | [ ] |
| 1321 | `ZSTD_insertBtAndGetAllMatches` (src/compress/zstd_opt.c:790) | `if (matchLength > bestLength) {` | branch-specific rejection/error | [ ] |
| 1322 | `ZSTD_insertBtAndGetAllMatches` (src/compress/zstd_opt.c:794) | `if (matchLength > matchEndIdx - matchIndex)` | branch-specific rejection/error | [ ] |
| 1323 | `ZSTD_insertBtAndGetAllMatches` (src/compress/zstd_opt.c:800) | `if ( (matchLength > ZSTD_OPT_NUM)` | branch-specific rejection/error | [ ] |
| 1324 | `ZSTD_insertBtAndGetAllMatches` (src/compress/zstd_opt.c:805) | `if (dictMatchIndex <= dmsBtLow) { break; }   /* beyond tree size, stop the search */` | branch-specific rejection/error | [ ] |
| 1325 | `ZSTD_insertBtAndGetAllMatches` (src/compress/zstd_opt.c:806) | `if (match[matchLength] < ip[matchLength]) {` | branch-specific rejection/error | [ ] |
| 1326 | `ZSTD_insertBtAndGetAllMatches` (src/compress/zstd_opt.c:815) | `assert(matchEndIdx > curr+8);` | assertion failure | [ ] |
| 1327 | `ZSTD_btGetAllMatches_internal` (src/compress/zstd_opt.c:844) | `assert(BOUNDED(3, ms->cParams.minMatch, 6) == mls);` | assertion failure | [ ] |
| 1328 | `ZSTD_btGetAllMatches_internal` (src/compress/zstd_opt.c:846) | `if (ip < ms->window.base + ms->nextToUpdate)` | branch-specific rejection/error | [ ] |
| 1329 | `GEN_ZSTD_BT_GET_ALL_MATCHES` (src/compress/zstd_opt.c:897) | `assert((U32)dictMode < 3);` | assertion failure | [ ] |
| 1330 | `GEN_ZSTD_BT_GET_ALL_MATCHES` (src/compress/zstd_opt.c:898) | `assert(mls - 3 < 4);` | assertion failure | [ ] |
| 1331 | `ZSTD_optLdm_skipRawSeqStoreBytes` (src/compress/zstd_opt.c:923) | `if (currPos >= currSeq.litLength + currSeq.matchLength) {` | branch-specific rejection/error | [ ] |
| 1332 | `ZSTD_optLdm_skipRawSeqStoreBytes` (src/compress/zstd_opt.c:931) | `if (currPos == 0 \|\| rawSeqStore->pos == rawSeqStore->size) {` | branch-specific rejection/error | [ ] |
| 1333 | `ZSTD_opt_getNextMatchAndUpdateSeqStore` (src/compress/zstd_opt.c:950) | `if (optLdm->seqStore.size == 0 \|\| optLdm->seqStore.pos >= optLdm->seqStore.size) {` | branch-specific rejection/error | [ ] |
| 1334 | `ZSTD_opt_getNextMatchAndUpdateSeqStore` (src/compress/zstd_opt.c:958) | `assert(optLdm->seqStore.posInSequence <= currSeq.litLength + currSeq.matchLength);` | assertion failure | [ ] |
| 1335 | `ZSTD_opt_getNextMatchAndUpdateSeqStore` (src/compress/zstd_opt.c:968) | `if (literalsBytesRemaining >= blockBytesRemaining) {` | branch-specific rejection/error | [ ] |
| 1336 | `ZSTD_opt_getNextMatchAndUpdateSeqStore` (src/compress/zstd_opt.c:981) | `if (optLdm->endPosInBlock > currBlockEndPos) {` | branch-specific rejection/error | [ ] |
| 1337 | `ZSTD_optLdm_maybeAddMatch` (src/compress/zstd_opt.c:1005) | `if (currPosInBlock < optLdm->startPosInBlock` | branch-specific rejection/error | [ ] |
| 1338 | `ZSTD_optLdm_maybeAddMatch` (src/compress/zstd_opt.c:1011) | `if (*nbMatches == 0 \|\| ((candidateMatchLength > matches[*nbMatches-1].len) && *nbMatches < ZSTD_OPT_NUM)) {` | branch-specific rejection/error | [ ] |
| 1339 | `ZSTD_optLdm_processMatchCandidate` (src/compress/zstd_opt.c:1030) | `if (optLdm->seqStore.size == 0 \|\| optLdm->seqStore.pos >= optLdm->seqStore.size) {` | branch-specific rejection/error | [ ] |
| 1340 | `ZSTD_optLdm_processMatchCandidate` (src/compress/zstd_opt.c:1034) | `if (currPosInBlock >= optLdm->endPosInBlock) {` | branch-specific rejection/error | [ ] |
| 1341 | `ZSTD_optLdm_processMatchCandidate` (src/compress/zstd_opt.c:1035) | `if (currPosInBlock > optLdm->endPosInBlock) {` | branch-specific rejection/error | [ ] |
| 1342 | `ZSTD_compressBlock_opt_generic` (src/compress/zstd_opt.c:1114) | `assert(optLevel <= 2);` | assertion failure | [ ] |
| 1343 | `ZSTD_compressBlock_opt_generic` (src/compress/zstd_opt.c:1160) | `if (maxML > sufficient_len) {` | branch-specific rejection/error | [ ] |
| 1344 | `ZSTD_compressBlock_opt_generic` (src/compress/zstd_opt.c:1172) | `assert(opt[0].price >= 0);` | assertion failure | [ ] |
| 1345 | `ZSTD_compressBlock_opt_generic` (src/compress/zstd_opt.c:1202) | `assert(cur <= ZSTD_OPT_NUM);` | assertion failure | [ ] |
| 1346 | `ZSTD_compressBlock_opt_generic` (src/compress/zstd_opt.c:1210) | `assert(price < 1000000000); /* overflow check */` | assertion failure | [ ] |
| 1347 | `ZSTD_compressBlock_opt_generic` (src/compress/zstd_opt.c:1211) | `if (price <= opt[cur].price) {` | branch-specific rejection/error | [ ] |
| 1348 | `ZSTD_compressBlock_opt_generic` (src/compress/zstd_opt.c:1219) | `if ( (optLevel >= 1) /* additional check only for higher modes */` | branch-specific rejection/error | [ ] |
| 1349 | `ZSTD_compressBlock_opt_generic` (src/compress/zstd_opt.c:1229) | `if ( (with1literal < withMoreLiterals)` | branch-specific rejection/error | [ ] |
| 1350 | `ZSTD_compressBlock_opt_generic` (src/compress/zstd_opt.c:1234) | `assert(cur >= prevMatch.mlen);` | assertion failure | [ ] |
| 1351 | `ZSTD_compressBlock_opt_generic` (src/compress/zstd_opt.c:1242) | `if (last_pos < cur+1) last_pos = cur+1;` | branch-specific rejection/error | [ ] |
| 1352 | `ZSTD_compressBlock_opt_generic` (src/compress/zstd_opt.c:1255) | `assert(cur >= opt[cur].mlen);` | assertion failure | [ ] |
| 1353 | `ZSTD_compressBlock_opt_generic` (src/compress/zstd_opt.c:1256) | `if (opt[cur].litlen == 0) {` | branch-specific rejection/error | [ ] |
| 1354 | `ZSTD_compressBlock_opt_generic` (src/compress/zstd_opt.c:1264) | `if (inr > ilimit) continue;` | branch-specific rejection/error | [ ] |
| 1355 | `ZSTD_compressBlock_opt_generic` (src/compress/zstd_opt.c:1268) | `if ( (optLevel==0) /*static_test*/` | branch-specific rejection/error | [ ] |
| 1356 | `ZSTD_compressBlock_opt_generic` (src/compress/zstd_opt.c:1274) | `assert(opt[cur].price >= 0);` | assertion failure | [ ] |
| 1357 | `ZSTD_compressBlock_opt_generic` (src/compress/zstd_opt.c:1294) | `if ( (longestML > sufficient_len)` | branch-specific rejection/error | [ ] |
| 1358 | `ZSTD_compressBlock_opt_generic` (src/compress/zstd_opt.c:1318) | `if ((pos > last_pos) \|\| (price < opt[pos].price)) {` | branch-specific rejection/error | [ ] |
| 1359 | `ZSTD_compressBlock_opt_generic` (src/compress/zstd_opt.c:1334) | `if (optLevel==0) break;  /* early update abort; gets ~+10% speed for about -0.01 ratio loss */` | branch-specific rejection/error | [ ] |
| 1360 | `ZSTD_compressBlock_opt_generic` (src/compress/zstd_opt.c:1341) | `assert(cur >= lastStretch.mlen);` | assertion failure | [ ] |
| 1361 | `ZSTD_compressBlock_opt_generic` (src/compress/zstd_opt.c:1345) | `assert(opt[0].mlen == 0);` | assertion failure | [ ] |
| 1362 | `ZSTD_compressBlock_opt_generic` (src/compress/zstd_opt.c:1346) | `assert(last_pos >= lastStretch.mlen);` | assertion failure | [ ] |
| 1363 | `ZSTD_compressBlock_opt_generic` (src/compress/zstd_opt.c:1347) | `assert(cur == last_pos - lastStretch.mlen);` | assertion failure | [ ] |
| 1364 | `ZSTD_compressBlock_opt_generic` (src/compress/zstd_opt.c:1349) | `if (lastStretch.mlen==0) {` | branch-specific rejection/error | [ ] |
| 1365 | `ZSTD_compressBlock_opt_generic` (src/compress/zstd_opt.c:1351) | `assert(lastStretch.litlen == (ip - anchor) + last_pos);` | assertion failure | [ ] |
| 1366 | `ZSTD_compressBlock_opt_generic` (src/compress/zstd_opt.c:1355) | `assert(lastStretch.off > 0);` | assertion failure | [ ] |
| 1367 | `ZSTD_compressBlock_opt_generic` (src/compress/zstd_opt.c:1358) | `if (lastStretch.litlen == 0) {` | branch-specific rejection/error | [ ] |
| 1368 | `ZSTD_compressBlock_opt_generic` (src/compress/zstd_opt.c:1364) | `assert(cur >= lastStretch.litlen);` | assertion failure | [ ] |
| 1369 | `ZSTD_compressBlock_opt_generic` (src/compress/zstd_opt.c:1382) | `assert(storeEnd < ZSTD_OPT_SIZE);` | assertion failure | [ ] |
| 1370 | `ZSTD_compressBlock_opt_generic` (src/compress/zstd_opt.c:1385) | `if (lastStretch.litlen > 0) {` | branch-specific rejection/error | [ ] |
| 1371 | `ZSTD_compressBlock_opt_generic` (src/compress/zstd_opt.c:1400) | `if (nextStretch.mlen == 0) {` | branch-specific rejection/error | [ ] |
| 1372 | `ZSTD_compressBlock_opt_generic` (src/compress/zstd_opt.c:1406) | `assert(nextStretch.litlen + nextStretch.mlen <= stretchPos);` | assertion failure | [ ] |
| 1373 | `ZSTD_compressBlock_opt_generic` (src/compress/zstd_opt.c:1421) | `if (mlen==0) {  /* only literals => must be last "sequence", actually starting a new stream of sequences */` | branch-specific rejection/error | [ ] |
| 1374 | `ZSTD_compressBlock_opt_generic` (src/compress/zstd_opt.c:1422) | `assert(storePos == storeEnd);   /* must be last sequence */` | assertion failure | [ ] |
| 1375 | `ZSTD_compressBlock_opt_generic` (src/compress/zstd_opt.c:1427) | `assert(anchor + llen <= iend);` | assertion failure | [ ] |
| 1376 | `ZSTD_initStats_ultra` (src/compress/zstd_opt.c:1493) | `assert(ms->opt.litLengthSum == 0);    /* first block */` | assertion failure | [ ] |
| 1377 | `ZSTD_initStats_ultra` (src/compress/zstd_opt.c:1494) | `assert(seqStore->sequences == seqStore->sequencesStart);   /* no ldm */` | assertion failure | [ ] |
| 1378 | `ZSTD_initStats_ultra` (src/compress/zstd_opt.c:1495) | `assert(ms->window.dictLimit == ms->window.lowLimit);   /* no dictionary */` | assertion failure | [ ] |
| 1379 | `ZSTD_initStats_ultra` (src/compress/zstd_opt.c:1496) | `assert(ms->window.dictLimit - ms->nextToUpdate <= 1);  /* no prefix (note: intentional overflow, defined as 2-complement) */` | assertion failure | [ ] |
| 1380 | `ZSTD_compressBlock_btultra2` (src/compress/zstd_opt.c:1532) | `assert(srcSize <= ZSTD_BLOCKSIZE_MAX);` | assertion failure | [ ] |
| 1381 | `ZSTD_compressBlock_btultra2` (src/compress/zstd_opt.c:1533) | `if ( (ms->opt.litLengthSum==0)   /* first block */` | branch-specific rejection/error | [ ] |
| 1382 | `hash2` (src/compress/zstd_preSplit.c:35) | `assert(hashLog >= 8);` | assertion failure | [ ] |
| 1383 | `hash2` (src/compress/zstd_preSplit.c:37) | `assert(hashLog <= HASHLOG_MAX);` | assertion failure | [ ] |
| 1384 | `addEvents_generic` (src/compress/zstd_preSplit.c:62) | `assert(srcSize >= HASHLENGTH);` | assertion failure | [ ] |
| 1385 | `fpDistance` (src/compress/zstd_preSplit.c:99) | `assert(hashLog <= HASHLOG_MAX);` | assertion failure | [ ] |
| 1386 | `compareFingerprints` (src/compress/zstd_preSplit.c:115) | `assert(ref->nbEvents > 0);` | assertion failure | [ ] |
| 1387 | `compareFingerprints` (src/compress/zstd_preSplit.c:116) | `assert(newfp->nbEvents > 0);` | assertion failure | [ ] |
| 1388 | `removeEvents` (src/compress/zstd_preSplit.c:147) | `assert(acc->events[n] >= slice->events[n]);` | assertion failure | [ ] |
| 1389 | `ZSTD_splitBlock_byChunks` (src/compress/zstd_preSplit.c:162) | `const RecordEvents_f record_f = (assert(0<=level && level<=3), records_fs[level]);` | assertion failure | [ ] |
| 1390 | `ZSTD_splitBlock_byChunks` (src/compress/zstd_preSplit.c:167) | `assert(blockSize == (128 << 10));` | assertion failure | [ ] |
| 1391 | `ZSTD_splitBlock_byChunks` (src/compress/zstd_preSplit.c:168) | `assert(workspace != NULL);` | assertion failure | [ ] |
| 1392 | `ZSTD_splitBlock_byChunks` (src/compress/zstd_preSplit.c:169) | `assert((size_t)workspace % ZSTD_ALIGNOF(FPStats) == 0);` | assertion failure | [ ] |
| 1393 | `ZSTD_splitBlock_byChunks` (src/compress/zstd_preSplit.c:171) | `assert(wkspSize >= sizeof(FPStats)); (void)wkspSize;` | assertion failure | [ ] |
| 1394 | `ZSTD_splitBlock_byChunks` (src/compress/zstd_preSplit.c:177) | `if (compareFingerprints(&fpstats->pastEvents, &fpstats->newEvents, penalty, hashParams[level])) {` | branch-specific rejection/error | [ ] |
| 1395 | `ZSTD_splitBlock_byChunks` (src/compress/zstd_preSplit.c:181) | `if (penalty > 0) penalty--;` | branch-specific rejection/error | [ ] |
| 1396 | `ZSTD_splitBlock_byChunks` (src/compress/zstd_preSplit.c:184) | `assert(pos == blockSize);` | assertion failure | [ ] |
| 1397 | `ZSTD_splitBlock_fromBorders` (src/compress/zstd_preSplit.c:204) | `assert(blockSize == (128 << 10));` | assertion failure | [ ] |
| 1398 | `ZSTD_splitBlock_fromBorders` (src/compress/zstd_preSplit.c:205) | `assert(workspace != NULL);` | assertion failure | [ ] |
| 1399 | `ZSTD_splitBlock_fromBorders` (src/compress/zstd_preSplit.c:206) | `assert((size_t)workspace % ZSTD_ALIGNOF(FPStats) == 0);` | assertion failure | [ ] |
| 1400 | `ZSTD_splitBlock_fromBorders` (src/compress/zstd_preSplit.c:208) | `assert(wkspSize >= sizeof(FPStats)); (void)wkspSize;` | assertion failure | [ ] |
| 1401 | `ZSTD_splitBlock_fromBorders` (src/compress/zstd_preSplit.c:214) | `if (!compareFingerprints(&fpstats->pastEvents, &fpstats->newEvents, 0, 8))` | branch-specific rejection/error | [ ] |
| 1402 | `ZSTD_splitBlock_fromBorders` (src/compress/zstd_preSplit.c:222) | `if (abs64((S64)distFromBegin - (S64)distFromEnd) < minDistance)` | branch-specific rejection/error | [ ] |
| 1403 | `ZSTD_splitBlock` (src/compress/zstd_preSplit.c:233) | `assert(0<=level && level<=4);` | assertion failure | [ ] |
| 1404 | `ZSTD_splitBlock` (src/compress/zstd_preSplit.c:234) | `if (level == 0)` | branch-specific rejection/error | [ ] |
| 1405 | `GetCurrentClockTimeMicroseconds` (src/compress/zstdmt_compress.c:54) | `if (_ticksPerSecond <= 0) _ticksPerSecond = sysconf(_SC_CLK_TCK);` | branch-specific rejection/error | [ ] |
| 1406 | `GetCurrentClockTimeMicroseconds` (src/compress/zstdmt_compress.c:63) | `if (DEBUGLEVEL >= MUTEX_WAIT_TIME_DLEVEL) {                                     \` | branch-specific rejection/error | [ ] |
| 1407 | `GetCurrentClockTimeMicroseconds` (src/compress/zstdmt_compress.c:68) | `if (elapsedTime > 1000) {                                               \` | branch-specific rejection/error | [ ] |
| 1408 | `ZSTDMT_freeBufferPool` (src/compress/zstdmt_compress.c:110) | `if (bufPool->buffers) {` | branch-specific rejection/error | [ ] |
| 1409 | `ZSTDMT_createBufferPool` (src/compress/zstdmt_compress.c:126) | `if (bufPool==NULL) return NULL;` | `NULL` | [ ] |
| 1410 | `ZSTDMT_createBufferPool` (src/compress/zstdmt_compress.c:127) | `if (ZSTD_pthread_mutex_init(&bufPool->poolMutex, NULL)) {` | `NULL` | [ ] |
| 1411 | `ZSTDMT_createBufferPool` (src/compress/zstdmt_compress.c:129) | `return NULL;` | `NULL` | [ ] |
| 1412 | `ZSTDMT_createBufferPool` (src/compress/zstdmt_compress.c:132) | `if (bufPool->buffers==NULL) {` | `NULL` | [ ] |
| 1413 | `ZSTDMT_createBufferPool` (src/compress/zstdmt_compress.c:134) | `return NULL;` | `NULL` | [ ] |
| 1414 | `ZSTDMT_expandBufferPool` (src/compress/zstdmt_compress.c:173) | `if (srcBufPool==NULL) return NULL;` | `NULL` | [ ] |
| 1415 | `ZSTDMT_expandBufferPool` (src/compress/zstdmt_compress.c:174) | `if (srcBufPool->totalBuffers >= maxNbBuffers) /* good enough */` | branch-specific rejection/error | [ ] |
| 1416 | `ZSTDMT_expandBufferPool` (src/compress/zstdmt_compress.c:182) | `if (newBufPool==NULL) return newBufPool;` | branch-specific rejection/error | [ ] |
| 1417 | `ZSTDMT_getBuffer` (src/compress/zstdmt_compress.c:197) | `if (bufPool->nbBuffers) {   /* try to use an existing buffer */` | branch-specific rejection/error | [ ] |
| 1418 | `ZSTDMT_getBuffer` (src/compress/zstdmt_compress.c:201) | `if ((availBufferSize >= bSize) & ((availBufferSize>>3) <= bSize)) {` | branch-specific rejection/error | [ ] |
| 1419 | `ZSTDMT_getBuffer` (src/compress/zstdmt_compress.c:219) | `if (start==NULL) {` | branch-specific rejection/error | [ ] |
| 1420 | `ZSTDMT_resizeBuffer` (src/compress/zstdmt_compress.c:237) | `if (buffer.capacity < bSize) {` | branch-specific rejection/error | [ ] |
| 1421 | `ZSTDMT_resizeBuffer` (src/compress/zstdmt_compress.c:242) | `if (start != NULL) {` | branch-specific rejection/error | [ ] |
| 1422 | `ZSTDMT_resizeBuffer` (src/compress/zstdmt_compress.c:243) | `assert(newBuffer.capacity >= buffer.capacity);` | assertion failure | [ ] |
| 1423 | `ZSTDMT_releaseBuffer` (src/compress/zstdmt_compress.c:258) | `if (buf.start == NULL) return;   /* compatible with release on NULL */` | branch-specific rejection/error | [ ] |
| 1424 | `ZSTDMT_releaseBuffer` (src/compress/zstdmt_compress.c:260) | `if (bufPool->nbBuffers < bufPool->totalBuffers) {` | branch-specific rejection/error | [ ] |
| 1425 | `ZSTDMT_getSeq` (src/compress/zstdmt_compress.c:311) | `if (seqPool->bufferSize == 0) {` | branch-specific rejection/error | [ ] |
| 1426 | `ZSTDMT_createSeqPool` (src/compress/zstdmt_compress.c:337) | `if (seqPool == NULL) return NULL;` | `NULL` | [ ] |
| 1427 | `ZSTDMT_freeCCtxPool` (src/compress/zstdmt_compress.c:369) | `if (pool->cctxs) {` | branch-specific rejection/error | [ ] |
| 1428 | `ZSTDMT_createCCtxPool` (src/compress/zstdmt_compress.c:385) | `assert(nbWorkers > 0);` | `NULL` | [ ] |
| 1429 | `ZSTDMT_createCCtxPool` (src/compress/zstdmt_compress.c:386) | `if (!cctxPool) return NULL;` | `NULL` | [ ] |
| 1430 | `ZSTDMT_createCCtxPool` (src/compress/zstdmt_compress.c:387) | `if (ZSTD_pthread_mutex_init(&cctxPool->poolMutex, NULL)) {` | `NULL` | [ ] |
| 1431 | `ZSTDMT_createCCtxPool` (src/compress/zstdmt_compress.c:389) | `return NULL;` | `NULL` | [ ] |
| 1432 | `ZSTDMT_createCCtxPool` (src/compress/zstdmt_compress.c:393) | `if (!cctxPool->cctxs) {` | `NULL` | [ ] |
| 1433 | `ZSTDMT_createCCtxPool` (src/compress/zstdmt_compress.c:395) | `return NULL;` | `NULL` | [ ] |
| 1434 | `ZSTDMT_createCCtxPool` (src/compress/zstdmt_compress.c:399) | `if (!cctxPool->cctxs[0]) { ZSTDMT_freeCCtxPool(cctxPool); return NULL; }` | `NULL` | [ ] |
| 1435 | `ZSTDMT_expandCCtxPool` (src/compress/zstdmt_compress.c:408) | `if (srcPool==NULL) return NULL;` | `NULL` | [ ] |
| 1436 | `ZSTDMT_expandCCtxPool` (src/compress/zstdmt_compress.c:409) | `if (nbWorkers <= srcPool->totalCCtx) return srcPool;   /* good enough */` | branch-specific rejection/error | [ ] |
| 1437 | `ZSTDMT_sizeof_CCtxPool` (src/compress/zstdmt_compress.c:430) | `assert(nbWorkers > 0);` | assertion failure | [ ] |
| 1438 | `ZSTDMT_getCCtx` (src/compress/zstdmt_compress.c:439) | `if (cctxPool->availCCtx) {` | branch-specific rejection/error | [ ] |
| 1439 | `ZSTDMT_releaseCCtx` (src/compress/zstdmt_compress.c:452) | `if (cctx==NULL) return;   /* compatibility with release on NULL */` | branch-specific rejection/error | [ ] |
| 1440 | `ZSTDMT_releaseCCtx` (src/compress/zstdmt_compress.c:454) | `if (pool->availCCtx < pool->totalCCtx)` | branch-specific rejection/error | [ ] |
| 1441 | `ZSTDMT_serialState_reset` (src/compress/zstdmt_compress.c:499) | `assert(params.ldmParams.hashLog >= params.ldmParams.bucketSizeLog);` | assertion failure | [ ] |
| 1442 | `ZSTDMT_serialState_reset` (src/compress/zstdmt_compress.c:500) | `assert(params.ldmParams.hashRateLog < 32);` | assertion failure | [ ] |
| 1443 | `ZSTDMT_serialState_reset` (src/compress/zstdmt_compress.c:522) | `if (serialState->ldmState.hashTable == NULL \|\| serialState->params.ldmParams.hashLog < hashLog) {` | branch-specific rejection/error | [ ] |
| 1444 | `ZSTDMT_serialState_reset` (src/compress/zstdmt_compress.c:526) | `if (serialState->ldmState.bucketOffsets == NULL \|\| prevBucketLog < bucketLog) {` | branch-specific rejection/error | [ ] |
| 1445 | `ZSTDMT_serialState_reset` (src/compress/zstdmt_compress.c:530) | `if (!serialState->ldmState.hashTable \|\| !serialState->ldmState.bucketOffsets)` | branch-specific rejection/error | [ ] |
| 1446 | `ZSTDMT_serialState_reset` (src/compress/zstdmt_compress.c:538) | `if (dictSize > 0) {` | branch-specific rejection/error | [ ] |
| 1447 | `ZSTDMT_serialState_genSequences` (src/compress/zstdmt_compress.c:592) | `if (serialState->nextJobID == jobID) {` | branch-specific rejection/error | [ ] |
| 1448 | `ZSTDMT_serialState_genSequences` (src/compress/zstdmt_compress.c:594) | `if (serialState->params.ldmParams.enableLdm == ZSTD_ps_enable) {` | branch-specific rejection/error | [ ] |
| 1449 | `ZSTDMT_serialState_genSequences` (src/compress/zstdmt_compress.c:597) | `assert(seqStore->seq != NULL && seqStore->pos == 0 &&` | assertion failure | [ ] |
| 1450 | `ZSTDMT_serialState_genSequences` (src/compress/zstdmt_compress.c:599) | `assert(src.size <= serialState->params.jobSize);` | assertion failure | [ ] |
| 1451 | `ZSTDMT_serialState_genSequences` (src/compress/zstdmt_compress.c:605) | `assert(!ZSTD_isError(error)); (void)error;` | assertion failure | [ ] |
| 1452 | `ZSTDMT_serialState_genSequences` (src/compress/zstdmt_compress.c:614) | `if (serialState->params.fParams.checksumFlag && src.size > 0)` | branch-specific rejection/error | [ ] |
| 1453 | `ZSTDMT_serialState_applySequences` (src/compress/zstdmt_compress.c:624) | `ZSTDMT_serialState_applySequences(const SerialState* serialState, /* just for an assert() check */` | assertion failure | [ ] |
| 1454 | `ZSTDMT_serialState_applySequences` (src/compress/zstdmt_compress.c:628) | `if (seqStore->size > 0) {` | branch-specific rejection/error | [ ] |
| 1455 | `ZSTDMT_serialState_applySequences` (src/compress/zstdmt_compress.c:630) | `assert(serialState->params.ldmParams.enableLdm == ZSTD_ps_enable); (void)serialState;` | assertion failure | [ ] |
| 1456 | `ZSTDMT_serialState_applySequences` (src/compress/zstdmt_compress.c:631) | `assert(jobCCtx);` | assertion failure | [ ] |
| 1457 | `ZSTDMT_serialState_ensureFinished` (src/compress/zstdmt_compress.c:640) | `if (serialState->nextJobID <= jobID) {` | branch-specific rejection/error | [ ] |
| 1458 | `ZSTDMT_serialState_ensureFinished` (src/compress/zstdmt_compress.c:641) | `assert(ZSTD_isError(cSize)); (void)cSize;` | assertion failure | [ ] |
| 1459 | `ZSTDMT_compressionJob` (src/compress/zstdmt_compress.c:704) | `if (cctx==NULL) JOB_ERROR(ERROR(memory_allocation));` | `ERROR(memory_allocation)` | [ ] |
| 1460 | `ZSTDMT_compressionJob` (src/compress/zstdmt_compress.c:705) | `if (dstBuff.start == NULL) {   /* streaming job : doesn't provide a dstBuffer */` | `ERROR(memory_allocation)` | [ ] |
| 1461 | `ZSTDMT_compressionJob` (src/compress/zstdmt_compress.c:707) | `if (dstBuff.start==NULL) JOB_ERROR(ERROR(memory_allocation));` | `ERROR(memory_allocation)` | [ ] |
| 1462 | `ZSTDMT_compressionJob` (src/compress/zstdmt_compress.c:710) | `if (jobParams.ldmParams.enableLdm == ZSTD_ps_enable && rawSeqStore.seq == NULL)` | `ERROR(memory_allocation)` | [ ] |
| 1463 | `ZSTDMT_compressionJob` (src/compress/zstdmt_compress.c:711) | `JOB_ERROR(ERROR(memory_allocation));` | `ERROR(memory_allocation)` | [ ] |
| 1464 | `ZSTDMT_compressionJob` (src/compress/zstdmt_compress.c:716) | `if (job->jobID != 0) jobParams.fParams.checksumFlag = 0;` | branch-specific rejection/error | [ ] |
| 1465 | `ZSTDMT_compressionJob` (src/compress/zstdmt_compress.c:728) | `if (job->cdict) {` | branch-specific rejection/error | [ ] |
| 1466 | `ZSTDMT_compressionJob` (src/compress/zstdmt_compress.c:730) | `assert(job->firstJob);  /* only allowed for first job */` | assertion failure | [ ] |
| 1467 | `ZSTDMT_compressionJob` (src/compress/zstdmt_compress.c:737) | `if (!job->firstJob) {` | branch-specific rejection/error | [ ] |
| 1468 | `ZSTDMT_compressionJob` (src/compress/zstdmt_compress.c:753) | `if (!job->firstJob) {  /* flush and overwrite frame header when it's not first job */` | branch-specific rejection/error | [ ] |
| 1469 | `ZSTDMT_compressionJob` (src/compress/zstdmt_compress.c:768) | `if (sizeof(size_t) > sizeof(int)) assert(job->src.size < ((size_t)INT_MAX) * chunkSize);   /* check overflow */` | assertion failure | [ ] |
| 1470 | `ZSTDMT_compressionJob` (src/compress/zstdmt_compress.c:770) | `assert(job->cSize == 0);` | assertion failure | [ ] |
| 1471 | `ZSTDMT_compressionJob` (src/compress/zstdmt_compress.c:775) | `op += cSize; assert(op < oend);` | assertion failure | [ ] |
| 1472 | `ZSTDMT_compressionJob` (src/compress/zstdmt_compress.c:786) | `assert(chunkSize > 0);` | assertion failure | [ ] |
| 1473 | `ZSTDMT_compressionJob` (src/compress/zstdmt_compress.c:787) | `assert((chunkSize & (chunkSize - 1)) == 0);  /* chunkSize must be power of 2 for mask==(chunkSize-1) to work */` | assertion failure | [ ] |
| 1474 | `ZSTDMT_compressionJob` (src/compress/zstdmt_compress.c:788) | `if ((nbChunks > 0) \| job->lastJob /*must output a "last block" flag*/ ) {` | branch-specific rejection/error | [ ] |
| 1475 | `ZSTDMT_compressionJob` (src/compress/zstdmt_compress.c:797) | `if (!job->firstJob) {` | branch-specific rejection/error | [ ] |
| 1476 | `ZSTDMT_compressionJob` (src/compress/zstdmt_compress.c:801) | `assert(!ZSTD_window_hasExtDict(cctx->blockState.matchState.window));` | assertion failure | [ ] |
| 1477 | `ZSTDMT_compressionJob` (src/compress/zstdmt_compress.c:807) | `if (job->prefix.size > 0)` | branch-specific rejection/error | [ ] |
| 1478 | `ZSTDMT_compressionJob` (src/compress/zstdmt_compress.c:815) | `if (ZSTD_isError(job->cSize)) assert(lastCBlockSize == 0);` | assertion failure | [ ] |
| 1479 | `ZSTDMT_freeJobsTable` (src/compress/zstdmt_compress.c:897) | `if (jobTable == NULL) return;` | branch-specific rejection/error | [ ] |
| 1480 | `ZSTDMT_createJobsTable` (src/compress/zstdmt_compress.c:916) | `if (jobTable==NULL) return NULL;` | `NULL` | [ ] |
| 1481 | `ZSTDMT_createJobsTable` (src/compress/zstdmt_compress.c:922) | `if (initError != 0) {` | `NULL` | [ ] |
| 1482 | `ZSTDMT_createJobsTable` (src/compress/zstdmt_compress.c:924) | `return NULL;` | `NULL` | [ ] |
| 1483 | `ZSTDMT_expandJobsTable` (src/compress/zstdmt_compress.c:931) | `if (nbJobs > mtctx->jobIDMask+1) {  /* need more job capacity */` | `ERROR(memory_allocation)` | [ ] |
| 1484 | `ZSTDMT_expandJobsTable` (src/compress/zstdmt_compress.c:935) | `if (mtctx->jobs==NULL) return ERROR(memory_allocation);` | `ERROR(memory_allocation)` | [ ] |
| 1485 | `ZSTDMT_expandJobsTable` (src/compress/zstdmt_compress.c:936) | `assert((nbJobs != 0) && ((nbJobs & (nbJobs - 1)) == 0));  /* ensure nbJobs is a power of 2 */` | assertion failure | [ ] |
| 1486 | `ZSTDMT_createCCtx_advanced_internal` (src/compress/zstdmt_compress.c:957) | `if (nbWorkers < 1) return NULL;` | `NULL` | [ ] |
| 1487 | `ZSTDMT_createCCtx_advanced_internal` (src/compress/zstdmt_compress.c:959) | `if ((cMem.customAlloc!=NULL) ^ (cMem.customFree!=NULL))` | `NULL` | [ ] |
| 1488 | `ZSTDMT_createCCtx_advanced_internal` (src/compress/zstdmt_compress.c:961) | `return NULL;` | `NULL` | [ ] |
| 1489 | `ZSTDMT_createCCtx_advanced_internal` (src/compress/zstdmt_compress.c:964) | `if (!mtctx) return NULL;` | `NULL` | [ ] |
| 1490 | `ZSTDMT_createCCtx_advanced_internal` (src/compress/zstdmt_compress.c:968) | `if (pool != NULL) {` | branch-specific rejection/error | [ ] |
| 1491 | `ZSTDMT_createCCtx_advanced_internal` (src/compress/zstdmt_compress.c:977) | `assert(nbJobs > 0); assert((nbJobs & (nbJobs - 1)) == 0);  /* ensure nbJobs is a power of 2 */` | assertion failure | [ ] |
| 1492 | `ZSTDMT_createCCtx_advanced_internal` (src/compress/zstdmt_compress.c:984) | `if (!mtctx->factory \| !mtctx->jobs \| !mtctx->bufPool \| !mtctx->cctxPool \| !mtctx->seqPool \| initError) {` | `NULL` | [ ] |
| 1493 | `ZSTDMT_createCCtx_advanced_internal` (src/compress/zstdmt_compress.c:986) | `return NULL;` | `NULL` | [ ] |
| 1494 | `ZSTDMT_createCCtx_advanced` (src/compress/zstdmt_compress.c:1000) | `return NULL;` | `NULL` | [ ] |
| 1495 | `ZSTDMT_freeCCtx` (src/compress/zstdmt_compress.c:1046) | `if (mtctx==NULL) return 0;   /* compatible with free on NULL */` | branch-specific rejection/error | [ ] |
| 1496 | `ZSTDMT_freeCCtx` (src/compress/zstdmt_compress.c:1047) | `if (!mtctx->providedFactory)` | branch-specific rejection/error | [ ] |
| 1497 | `ZSTDMT_freeCCtx` (src/compress/zstdmt_compress.c:1056) | `if (mtctx->roundBuff.buffer)` | branch-specific rejection/error | [ ] |
| 1498 | `ZSTDMT_sizeof_CCtx` (src/compress/zstdmt_compress.c:1064) | `if (mtctx == NULL) return 0;   /* supports sizeof NULL */` | branch-specific rejection/error | [ ] |
| 1499 | `ZSTDMT_resize` (src/compress/zstdmt_compress.c:1080) | `if (POOL_resize(mtctx->factory, nbWorkers)) return ERROR(memory_allocation);` | `ERROR(memory_allocation)` | [ ] |
| 1500 | `ZSTDMT_resize` (src/compress/zstdmt_compress.c:1083) | `if (mtctx->bufPool == NULL) return ERROR(memory_allocation);` | `ERROR(memory_allocation)` | [ ] |
| 1501 | `ZSTDMT_resize` (src/compress/zstdmt_compress.c:1085) | `if (mtctx->cctxPool == NULL) return ERROR(memory_allocation);` | `ERROR(memory_allocation)` | [ ] |
| 1502 | `ZSTDMT_resize` (src/compress/zstdmt_compress.c:1087) | `if (mtctx->seqPool == NULL) return ERROR(memory_allocation);` | `ERROR(memory_allocation)` | [ ] |
| 1503 | `ZSTDMT_getFrameProgression` (src/compress/zstdmt_compress.c:1123) | `unsigned lastJobNb = mtctx->nextJobID + mtctx->jobReady; assert(mtctx->jobReady <= 1);` | assertion failure | [ ] |
| 1504 | `ZSTDMT_getFrameProgression` (src/compress/zstdmt_compress.c:1133) | `assert(flushed <= produced);` | assertion failure | [ ] |
| 1505 | `ZSTDMT_toFlushNow` (src/compress/zstdmt_compress.c:1151) | `assert(jobID <= mtctx->nextJobID);` | assertion failure | [ ] |
| 1506 | `ZSTDMT_toFlushNow` (src/compress/zstdmt_compress.c:1152) | `if (jobID == mtctx->nextJobID) return 0;   /* no active job => nothing to flush */` | branch-specific rejection/error | [ ] |
| 1507 | `ZSTDMT_toFlushNow` (src/compress/zstdmt_compress.c:1161) | `assert(flushed <= produced);` | assertion failure | [ ] |
| 1508 | `ZSTDMT_toFlushNow` (src/compress/zstdmt_compress.c:1162) | `assert(jobPtr->consumed <= jobPtr->src.size);` | assertion failure | [ ] |
| 1509 | `ZSTDMT_toFlushNow` (src/compress/zstdmt_compress.c:1169) | `if (toFlush==0) {` | branch-specific rejection/error | [ ] |
| 1510 | `ZSTDMT_toFlushNow` (src/compress/zstdmt_compress.c:1170) | `assert(jobPtr->consumed < jobPtr->src.size);` | assertion failure | [ ] |
| 1511 | `ZSTDMT_computeTargetJobLog` (src/compress/zstdmt_compress.c:1187) | `if (params->ldmParams.enableLdm == ZSTD_ps_enable) {` | branch-specific rejection/error | [ ] |
| 1512 | `ZSTDMT_overlapLog` (src/compress/zstdmt_compress.c:1221) | `assert(0 <= ovlog && ovlog <= 9);` | assertion failure | [ ] |
| 1513 | `ZSTDMT_overlapLog` (src/compress/zstdmt_compress.c:1222) | `if (ovlog == 0) return ZSTDMT_overlapLog_default(strat);` | branch-specific rejection/error | [ ] |
| 1514 | `ZSTDMT_computeOverlapSize` (src/compress/zstdmt_compress.c:1230) | `assert(0 <= overlapRLog && overlapRLog <= 8);` | assertion failure | [ ] |
| 1515 | `ZSTDMT_computeOverlapSize` (src/compress/zstdmt_compress.c:1231) | `if (params->ldmParams.enableLdm == ZSTD_ps_enable) {` | branch-specific rejection/error | [ ] |
| 1516 | `ZSTDMT_computeOverlapSize` (src/compress/zstdmt_compress.c:1239) | `assert(0 <= ovLog && ovLog <= ZSTD_WINDOWLOG_MAX);` | assertion failure | [ ] |
| 1517 | `ZSTDMT_initCStream_internal` (src/compress/zstdmt_compress.c:1259) | `assert(!ZSTD_isError(ZSTD_checkCParams(params.cParams)));` | assertion failure | [ ] |
| 1518 | `ZSTDMT_initCStream_internal` (src/compress/zstdmt_compress.c:1260) | `assert(!((dict) && (cdict)));  /* either dict or cdict, not both */` | assertion failure | [ ] |
| 1519 | `ZSTDMT_initCStream_internal` (src/compress/zstdmt_compress.c:1263) | `if (params.nbWorkers != mtctx->params.nbWorkers)` | branch-specific rejection/error | [ ] |
| 1520 | `ZSTDMT_initCStream_internal` (src/compress/zstdmt_compress.c:1266) | `if (params.jobSize != 0 && params.jobSize < ZSTDMT_JOBSIZE_MIN) params.jobSize = ZSTDMT_JOBSIZE_MIN;` | branch-specific rejection/error | [ ] |
| 1521 | `ZSTDMT_initCStream_internal` (src/compress/zstdmt_compress.c:1267) | `if (params.jobSize > (size_t)ZSTDMT_JOBSIZE_MAX) params.jobSize = (size_t)ZSTDMT_JOBSIZE_MAX;` | branch-specific rejection/error | [ ] |
| 1522 | `ZSTDMT_initCStream_internal` (src/compress/zstdmt_compress.c:1269) | `if (mtctx->allJobsCompleted == 0) {   /* previous compression not correctly finished */` | branch-specific rejection/error | [ ] |
| 1523 | `ZSTDMT_initCStream_internal` (src/compress/zstdmt_compress.c:1283) | `if (mtctx->cdictLocal == NULL) return ERROR(memory_allocation);` | `ERROR(memory_allocation)` | [ ] |
| 1524 | `ZSTDMT_initCStream_internal` (src/compress/zstdmt_compress.c:1292) | `if (mtctx->targetSectionSize == 0) {` | branch-specific rejection/error | [ ] |
| 1525 | `ZSTDMT_initCStream_internal` (src/compress/zstdmt_compress.c:1295) | `assert(mtctx->targetSectionSize <= (size_t)ZSTDMT_JOBSIZE_MAX);` | assertion failure | [ ] |
| 1526 | `ZSTDMT_initCStream_internal` (src/compress/zstdmt_compress.c:1300) | `U32 const rsyncBits = (assert(jobSizeKB >= 1), ZSTD_highbit32(jobSizeKB) + 10);` | assertion failure | [ ] |
| 1527 | `ZSTDMT_initCStream_internal` (src/compress/zstdmt_compress.c:1303) | `assert(rsyncBits >= RSYNC_MIN_BLOCK_LOG + 2);` | assertion failure | [ ] |
| 1528 | `ZSTDMT_initCStream_internal` (src/compress/zstdmt_compress.c:1309) | `if (mtctx->targetSectionSize < mtctx->targetPrefixSize) mtctx->targetSectionSize = mtctx->targetPrefixSize;  /* job size must be >= overlap size */` | branch-specific rejection/error | [ ] |
| 1529 | `ZSTDMT_initCStream_internal` (src/compress/zstdmt_compress.c:1328) | `if (mtctx->roundBuff.capacity < capacity) {` | branch-specific rejection/error | [ ] |
| 1530 | `ZSTDMT_initCStream_internal` (src/compress/zstdmt_compress.c:1329) | `if (mtctx->roundBuff.buffer)` | branch-specific rejection/error | [ ] |
| 1531 | `ZSTDMT_initCStream_internal` (src/compress/zstdmt_compress.c:1332) | `if (mtctx->roundBuff.buffer == NULL) {` | `ERROR(memory_allocation)` | [ ] |
| 1532 | `ZSTDMT_initCStream_internal` (src/compress/zstdmt_compress.c:1334) | `return ERROR(memory_allocation);` | `ERROR(memory_allocation)` | [ ] |
| 1533 | `ZSTDMT_initCStream_internal` (src/compress/zstdmt_compress.c:1365) | `if (mtctx->cdictLocal == NULL) return ERROR(memory_allocation);` | `ERROR(memory_allocation)` | [ ] |
| 1534 | `ZSTDMT_initCStream_internal` (src/compress/zstdmt_compress.c:1371) | `if (ZSTDMT_serialState_reset(&mtctx->serial, mtctx->seqPool, params, mtctx->targetSectionSize,` | `ERROR(memory_allocation)` | [ ] |
| 1535 | `ZSTDMT_initCStream_internal` (src/compress/zstdmt_compress.c:1373) | `return ERROR(memory_allocation);` | `ERROR(memory_allocation)` | [ ] |
| 1536 | `ZSTDMT_writeLastEmptyBlock` (src/compress/zstdmt_compress.c:1387) | `assert(job->lastJob == 1);` | assertion failure | [ ] |
| 1537 | `ZSTDMT_writeLastEmptyBlock` (src/compress/zstdmt_compress.c:1388) | `assert(job->src.size == 0);   /* last job is empty -> will be simplified into a last empty block */` | assertion failure | [ ] |
| 1538 | `ZSTDMT_writeLastEmptyBlock` (src/compress/zstdmt_compress.c:1389) | `assert(job->firstJob == 0);   /* cannot be first job, as it also needs to create frame header */` | `ERROR(memory_allocation)` | [ ] |
| 1539 | `ZSTDMT_writeLastEmptyBlock` (src/compress/zstdmt_compress.c:1390) | `assert(job->dstBuff.start == NULL);   /* invoked from streaming variant only (otherwise, dstBuff might be user's output) */` | `ERROR(memory_allocation)` | [ ] |
| 1540 | `ZSTDMT_writeLastEmptyBlock` (src/compress/zstdmt_compress.c:1392) | `if (job->dstBuff.start == NULL) {` | `ERROR(memory_allocation)` | [ ] |
| 1541 | `ZSTDMT_writeLastEmptyBlock` (src/compress/zstdmt_compress.c:1393) | `job->cSize = ERROR(memory_allocation);` | `ERROR(memory_allocation)` | [ ] |
| 1542 | `ZSTDMT_writeLastEmptyBlock` (src/compress/zstdmt_compress.c:1396) | `assert(job->dstBuff.capacity >= ZSTD_blockHeaderSize);   /* no buffer should ever be that small */` | assertion failure | [ ] |
| 1543 | `ZSTDMT_writeLastEmptyBlock` (src/compress/zstdmt_compress.c:1399) | `assert(!ZSTD_isError(job->cSize));` | assertion failure | [ ] |
| 1544 | `ZSTDMT_writeLastEmptyBlock` (src/compress/zstdmt_compress.c:1400) | `assert(job->consumed == 0);` | assertion failure | [ ] |
| 1545 | `ZSTDMT_createCompressionJob` (src/compress/zstdmt_compress.c:1408) | `if (mtctx->nextJobID > mtctx->doneJobID + mtctx->jobIDMask) {` | branch-specific rejection/error | [ ] |
| 1546 | `ZSTDMT_createCompressionJob` (src/compress/zstdmt_compress.c:1410) | `assert((mtctx->nextJobID & mtctx->jobIDMask) == (mtctx->doneJobID & mtctx->jobIDMask));` | assertion failure | [ ] |
| 1547 | `ZSTDMT_createCompressionJob` (src/compress/zstdmt_compress.c:1414) | `if (!mtctx->jobReady) {` | branch-specific rejection/error | [ ] |
| 1548 | `ZSTDMT_createCompressionJob` (src/compress/zstdmt_compress.c:1420) | `assert(mtctx->inBuff.filled >= srcSize);` | assertion failure | [ ] |
| 1549 | `ZSTDMT_createCompressionJob` (src/compress/zstdmt_compress.c:1450) | `if (mtctx->nextJobID == 0) {` | branch-specific rejection/error | [ ] |
| 1550 | `ZSTDMT_createCompressionJob` (src/compress/zstdmt_compress.c:1455) | `if ( (srcSize == 0)` | branch-specific rejection/error | [ ] |
| 1551 | `ZSTDMT_createCompressionJob` (src/compress/zstdmt_compress.c:1458) | `assert(endOp == ZSTD_e_end);  /* only possible case : need to end the frame with an empty last block */` | assertion failure | [ ] |
| 1552 | `ZSTDMT_createCompressionJob` (src/compress/zstdmt_compress.c:1471) | `if (POOL_tryAdd(mtctx->factory, ZSTDMT_compressionJob, &mtctx->jobs[jobID])) {` | branch-specific rejection/error | [ ] |
| 1553 | `ZSTDMT_flushProduced` (src/compress/zstdmt_compress.c:1493) | `assert(output->size >= output->pos);` | assertion failure | [ ] |
| 1554 | `ZSTDMT_flushProduced` (src/compress/zstdmt_compress.c:1498) | `assert(mtctx->jobs[wJobID].dstFlushed <= mtctx->jobs[wJobID].cSize);` | assertion failure | [ ] |
| 1555 | `ZSTDMT_flushProduced` (src/compress/zstdmt_compress.c:1500) | `if (mtctx->jobs[wJobID].consumed == mtctx->jobs[wJobID].src.size) {` | branch-specific rejection/error | [ ] |
| 1556 | `ZSTDMT_flushProduced` (src/compress/zstdmt_compress.c:1523) | `assert(srcConsumed <= srcSize);` | assertion failure | [ ] |
| 1557 | `ZSTDMT_flushProduced` (src/compress/zstdmt_compress.c:1524) | `if ( (srcConsumed == srcSize)   /* job completed -> worker no longer active */` | branch-specific rejection/error | [ ] |
| 1558 | `ZSTDMT_flushProduced` (src/compress/zstdmt_compress.c:1534) | `if (cSize > 0) {   /* compression is ongoing or completed */` | branch-specific rejection/error | [ ] |
| 1559 | `ZSTDMT_flushProduced` (src/compress/zstdmt_compress.c:1538) | `assert(mtctx->doneJobID < mtctx->nextJobID);` | assertion failure | [ ] |
| 1560 | `ZSTDMT_flushProduced` (src/compress/zstdmt_compress.c:1539) | `assert(cSize >= mtctx->jobs[wJobID].dstFlushed);` | assertion failure | [ ] |
| 1561 | `ZSTDMT_flushProduced` (src/compress/zstdmt_compress.c:1540) | `assert(mtctx->jobs[wJobID].dstBuff.start != NULL);` | assertion failure | [ ] |
| 1562 | `ZSTDMT_flushProduced` (src/compress/zstdmt_compress.c:1541) | `if (toFlush > 0) {` | branch-specific rejection/error | [ ] |
| 1563 | `ZSTDMT_flushProduced` (src/compress/zstdmt_compress.c:1563) | `if (cSize > mtctx->jobs[wJobID].dstFlushed) return (cSize - mtctx->jobs[wJobID].dstFlushed);` | branch-specific rejection/error | [ ] |
| 1564 | `ZSTDMT_flushProduced` (src/compress/zstdmt_compress.c:1564) | `if (srcSize > srcConsumed) return 1;   /* current job not completely compressed */` | branch-specific rejection/error | [ ] |
| 1565 | `ZSTDMT_flushProduced` (src/compress/zstdmt_compress.c:1566) | `if (mtctx->doneJobID < mtctx->nextJobID) return 1;   /* some more jobs ongoing */` | branch-specific rejection/error | [ ] |
| 1566 | `ZSTDMT_flushProduced` (src/compress/zstdmt_compress.c:1567) | `if (mtctx->jobReady) return 1;      /* one job is ready to push, just not yet in the list */` | branch-specific rejection/error | [ ] |
| 1567 | `ZSTDMT_flushProduced` (src/compress/zstdmt_compress.c:1568) | `if (mtctx->inBuff.filled > 0) return 1;   /* input is not empty, and still needs to be converted into a job */` | branch-specific rejection/error | [ ] |
| 1568 | `ZSTDMT_flushProduced` (src/compress/zstdmt_compress.c:1570) | `if (end == ZSTD_e_end) return !mtctx->frameEnded;  /* for ZSTD_e_end, question becomes : is frame completed ? instead of : are internal buffers fully flushed ? */` | branch-specific rejection/error | [ ] |
| 1569 | `ZSTDMT_getInputDataInUse` (src/compress/zstdmt_compress.c:1588) | `if (lastJobID < nbJobs1stRoundMin) return kNullRange;` | branch-specific rejection/error | [ ] |
| 1570 | `ZSTDMT_getInputDataInUse` (src/compress/zstdmt_compress.c:1598) | `if (consumed < mtctx->jobs[wJobID].src.size) {` | branch-specific rejection/error | [ ] |
| 1571 | `ZSTDMT_getInputDataInUse` (src/compress/zstdmt_compress.c:1600) | `if (range.size == 0) {` | branch-specific rejection/error | [ ] |
| 1572 | `ZSTDMT_getInputDataInUse` (src/compress/zstdmt_compress.c:1605) | `assert(range.start <= mtctx->jobs[wJobID].src.start);` | assertion failure | [ ] |
| 1573 | `ZSTDMT_isOverlapped` (src/compress/zstdmt_compress.c:1620) | `if (rangeStart == NULL \|\| bufferStart == NULL)` | branch-specific rejection/error | [ ] |
| 1574 | `ZSTDMT_waitForLdmComplete` (src/compress/zstdmt_compress.c:1659) | `if (mtctx->params.ldmParams.enableLdm == ZSTD_ps_enable) {` | branch-specific rejection/error | [ ] |
| 1575 | `ZSTDMT_tryGetInputRange` (src/compress/zstdmt_compress.c:1688) | `assert(mtctx->inBuff.buffer.start == NULL);` | assertion failure | [ ] |
| 1576 | `ZSTDMT_tryGetInputRange` (src/compress/zstdmt_compress.c:1689) | `assert(mtctx->roundBuff.capacity >= spaceNeeded);` | assertion failure | [ ] |
| 1577 | `ZSTDMT_tryGetInputRange` (src/compress/zstdmt_compress.c:1691) | `if (spaceLeft < spaceNeeded) {` | branch-specific rejection/error | [ ] |
| 1578 | `ZSTDMT_tryGetInputRange` (src/compress/zstdmt_compress.c:1716) | `assert(!ZSTDMT_isOverlapped(buffer, mtctx->inBuff.prefix));` | assertion failure | [ ] |
| 1579 | `ZSTDMT_tryGetInputRange` (src/compress/zstdmt_compress.c:1730) | `assert(mtctx->roundBuff.pos + buffer.capacity <= mtctx->roundBuff.capacity);` | assertion failure | [ ] |
| 1580 | `findSynchronizationPoint` (src/compress/zstdmt_compress.c:1759) | `if (!mtctx->params.rsyncable)` | branch-specific rejection/error | [ ] |
| 1581 | `findSynchronizationPoint` (src/compress/zstdmt_compress.c:1762) | `if (mtctx->inBuff.filled + input.size - input.pos < RSYNC_MIN_BLOCK_SIZE)` | branch-specific rejection/error | [ ] |
| 1582 | `findSynchronizationPoint` (src/compress/zstdmt_compress.c:1767) | `if (mtctx->inBuff.filled + syncPoint.toLoad < RSYNC_LENGTH)` | branch-specific rejection/error | [ ] |
| 1583 | `findSynchronizationPoint` (src/compress/zstdmt_compress.c:1777) | `if (mtctx->inBuff.filled < RSYNC_MIN_BLOCK_SIZE) {` | branch-specific rejection/error | [ ] |
| 1584 | `findSynchronizationPoint` (src/compress/zstdmt_compress.c:1783) | `if (pos >= RSYNC_LENGTH) {` | branch-specific rejection/error | [ ] |
| 1585 | `findSynchronizationPoint` (src/compress/zstdmt_compress.c:1787) | `assert(mtctx->inBuff.filled >= RSYNC_LENGTH);` | assertion failure | [ ] |
| 1586 | `findSynchronizationPoint` (src/compress/zstdmt_compress.c:1797) | `assert(mtctx->inBuff.filled >= RSYNC_MIN_BLOCK_SIZE);` | assertion failure | [ ] |
| 1587 | `findSynchronizationPoint` (src/compress/zstdmt_compress.c:1798) | `assert(RSYNC_MIN_BLOCK_SIZE >= RSYNC_LENGTH);` | assertion failure | [ ] |
| 1588 | `findSynchronizationPoint` (src/compress/zstdmt_compress.c:1821) | `assert(pos < RSYNC_LENGTH \|\| ZSTD_rollingHash_compute(istart + pos - RSYNC_LENGTH, RSYNC_LENGTH) == hash);` | assertion failure | [ ] |
| 1589 | `findSynchronizationPoint` (src/compress/zstdmt_compress.c:1830) | `assert(mtctx->inBuff.filled + pos >= RSYNC_MIN_BLOCK_SIZE);` | assertion failure | [ ] |
| 1590 | `findSynchronizationPoint` (src/compress/zstdmt_compress.c:1838) | `assert(pos < RSYNC_LENGTH \|\| ZSTD_rollingHash_compute(istart + pos - RSYNC_LENGTH, RSYNC_LENGTH) == hash);` | assertion failure | [ ] |
| 1591 | `ZSTDMT_nextInputSizeHint` (src/compress/zstdmt_compress.c:1845) | `if (hintInSize==0) hintInSize = mtctx->targetSectionSize;` | branch-specific rejection/error | [ ] |
| 1592 | `ZSTDMT_compressStream_generic` (src/compress/zstdmt_compress.c:1861) | `assert(output->pos <= output->size);` | assertion failure | [ ] |
| 1593 | `ZSTDMT_compressStream_generic` (src/compress/zstdmt_compress.c:1862) | `assert(input->pos  <= input->size);` | `ERROR(stage_wrong)` | [ ] |
| 1594 | `ZSTDMT_compressStream_generic` (src/compress/zstdmt_compress.c:1864) | `if ((mtctx->frameEnded) && (endOp==ZSTD_e_continue)) {` | `ERROR(stage_wrong)` | [ ] |
| 1595 | `ZSTDMT_compressStream_generic` (src/compress/zstdmt_compress.c:1866) | `return ERROR(stage_wrong);` | `ERROR(stage_wrong)` | [ ] |
| 1596 | `ZSTDMT_compressStream_generic` (src/compress/zstdmt_compress.c:1870) | `if ( (!mtctx->jobReady)` | branch-specific rejection/error | [ ] |
| 1597 | `ZSTDMT_compressStream_generic` (src/compress/zstdmt_compress.c:1872) | `if (mtctx->inBuff.buffer.start == NULL) {` | branch-specific rejection/error | [ ] |
| 1598 | `ZSTDMT_compressStream_generic` (src/compress/zstdmt_compress.c:1873) | `assert(mtctx->inBuff.filled == 0); /* Can't fill an empty buffer */` | assertion failure | [ ] |
| 1599 | `ZSTDMT_compressStream_generic` (src/compress/zstdmt_compress.c:1879) | `assert(mtctx->doneJobID != mtctx->nextJobID);` | assertion failure | [ ] |
| 1600 | `ZSTDMT_compressStream_generic` (src/compress/zstdmt_compress.c:1883) | `if (mtctx->inBuff.buffer.start != NULL) {` | branch-specific rejection/error | [ ] |
| 1601 | `ZSTDMT_compressStream_generic` (src/compress/zstdmt_compress.c:1888) | `assert(mtctx->inBuff.buffer.capacity >= mtctx->targetSectionSize);` | assertion failure | [ ] |
| 1602 | `ZSTDMT_compressStream_generic` (src/compress/zstdmt_compress.c:1897) | `if ((input->pos < input->size) && (endOp == ZSTD_e_end)) {` | branch-specific rejection/error | [ ] |
| 1603 | `ZSTDMT_compressStream_generic` (src/compress/zstdmt_compress.c:1904) | `assert(mtctx->inBuff.filled == 0 \|\| mtctx->inBuff.filled == mtctx->targetSectionSize \|\| mtctx->params.rsyncable);` | assertion failure | [ ] |
| 1604 | `ZSTDMT_compressStream_generic` (src/compress/zstdmt_compress.c:1908) | `if ( (mtctx->jobReady)` | branch-specific rejection/error | [ ] |
| 1605 | `ZSTDMT_compressStream_generic` (src/compress/zstdmt_compress.c:1913) | `assert(mtctx->inBuff.filled <= mtctx->targetSectionSize);` | assertion failure | [ ] |
| 1606 | `ZSTDMT_compressStream_generic` (src/compress/zstdmt_compress.c:1919) | `if (input->pos < input->size) return MAX(remainingToFlush, 1);  /* input not consumed : do not end flush yet */` | branch-specific rejection/error | [ ] |
| 1607 | `HUF_initFastDStream` (src/decompress/huf_decompress.c:154) | `assert(bitsConsumed <= 8);` | assertion failure | [ ] |
| 1608 | `HUF_initFastDStream` (src/decompress/huf_decompress.c:155) | `assert(sizeof(size_t) == 8);` | assertion failure | [ ] |
| 1609 | `HUF_DecompressFastArgs_init` (src/decompress/huf_decompress.c:207) | `if (dstSize == 0)` | branch-specific rejection/error | [ ] |
| 1610 | `HUF_DecompressFastArgs_init` (src/decompress/huf_decompress.c:209) | `assert(dst != NULL);` | `ERROR(corruption_detected)` | [ ] |
| 1611 | `HUF_DecompressFastArgs_init` (src/decompress/huf_decompress.c:212) | `if (srcSize < 10)` | `ERROR(corruption_detected)` | [ ] |
| 1612 | `HUF_DecompressFastArgs_init` (src/decompress/huf_decompress.c:213) | `return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 1613 | `HUF_DecompressFastArgs_init` (src/decompress/huf_decompress.c:236) | `if (length1 < 8 \|\| length2 < 8 \|\| length3 < 8 \|\| length4 < 8)` | `ERROR(corruption_detected)` | [ ] |
| 1614 | `HUF_DecompressFastArgs_init` (src/decompress/huf_decompress.c:238) | `if (length4 > srcSize) return ERROR(corruption_detected);   /* overflow */` | `ERROR(corruption_detected)` | [ ] |
| 1615 | `HUF_DecompressFastArgs_init` (src/decompress/huf_decompress.c:253) | `if (args->op[3] >= oend)` | branch-specific rejection/error | [ ] |
| 1616 | `HUF_initRemainingDStream` (src/decompress/huf_decompress.c:284) | `if (args->op[stream] > segmentEnd)` | `ERROR(corruption_detected)` | [ ] |
| 1617 | `HUF_initRemainingDStream` (src/decompress/huf_decompress.c:285) | `return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 1618 | `HUF_initRemainingDStream` (src/decompress/huf_decompress.c:291) | `if (args->ip[stream] < args->iend[stream] - 8)` | `ERROR(corruption_detected)` | [ ] |
| 1619 | `HUF_initRemainingDStream` (src/decompress/huf_decompress.c:292) | `return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 1620 | `HUF_initRemainingDStream` (src/decompress/huf_decompress.c:295) | `assert(sizeof(size_t) == 8);` | assertion failure | [ ] |
| 1621 | `HUF_DEltX1_set4` (src/decompress/huf_decompress.c:342) | `assert(D4 < (1U << 16));` | assertion failure | [ ] |
| 1622 | `HUF_rescaleStats` (src/decompress/huf_decompress.c:354) | `if (tableLog > targetTableLog)` | branch-specific rejection/error | [ ] |
| 1623 | `HUF_rescaleStats` (src/decompress/huf_decompress.c:356) | `if (tableLog < targetTableLog) {` | branch-specific rejection/error | [ ] |
| 1624 | `HUF_readDTableX1_wksp` (src/decompress/huf_decompress.c:395) | `if (sizeof(*wksp) > wkspSize) return ERROR(tableLog_tooLarge);` | `ERROR(tableLog_tooLarge)` | [ ] |
| 1625 | `HUF_readDTableX1_wksp` (src/decompress/huf_decompress.c:409) | `if (tableLog > (U32)(dtd.maxTableLog+1)) return ERROR(tableLog_tooLarge);   /* DTable too small, Huffman tree cannot fit in */` | `ERROR(tableLog_tooLarge)` | [ ] |
| 1626 | `HUF_readDTableX1_wksp` (src/decompress/huf_decompress.c:509) | `assert(u == length);` | assertion failure | [ ] |
| 1627 | `HUF_decodeSymbolX1` (src/decompress/huf_decompress.c:535) | `if (MEM_64bits() \|\| (HUF_TABLELOG_MAX<=12)) \` | branch-specific rejection/error | [ ] |
| 1628 | `HUF_decodeStreamX1` (src/decompress/huf_decompress.c:551) | `if ((pEnd - p) > 3) {` | branch-specific rejection/error | [ ] |
| 1629 | `HUF_decompress1X1_usingDTable_internal_body` (src/decompress/huf_decompress.c:592) | `if (!BIT_endOfDStream(&bitD)) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 1630 | `HUF_decompress4X1_usingDTable_internal_body` (src/decompress/huf_decompress.c:608) | `if (cSrcSize < 10) return ERROR(corruption_detected);  /* strict minimum : jump table + 1 byte per stream */` | `ERROR(corruption_detected)` | [ ] |
| 1631 | `HUF_decompress4X1_usingDTable_internal_body` (src/decompress/huf_decompress.c:609) | `if (dstSize < 6) return ERROR(corruption_detected);         /* stream 4-split doesn't work */` | `ERROR(corruption_detected)` | [ ] |
| 1632 | `HUF_decompress4X1_usingDTable_internal_body` (src/decompress/huf_decompress.c:643) | `if (length4 > cSrcSize) return ERROR(corruption_detected);   /* overflow */` | `ERROR(corruption_detected)` | [ ] |
| 1633 | `HUF_decompress4X1_usingDTable_internal_body` (src/decompress/huf_decompress.c:644) | `if (opStart4 > oend) return ERROR(corruption_detected);      /* overflow */` | `ERROR(corruption_detected)` | [ ] |
| 1634 | `HUF_decompress4X1_usingDTable_internal_body` (src/decompress/huf_decompress.c:645) | `assert(dstSize >= 6); /* validated above */` | assertion failure | [ ] |
| 1635 | `HUF_decompress4X1_usingDTable_internal_body` (src/decompress/huf_decompress.c:652) | `if ((size_t)(oend - op4) >= sizeof(size_t)) {` | branch-specific rejection/error | [ ] |
| 1636 | `HUF_decompress4X1_usingDTable_internal_body` (src/decompress/huf_decompress.c:680) | `if (op1 > opStart2) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 1637 | `HUF_decompress4X1_usingDTable_internal_body` (src/decompress/huf_decompress.c:681) | `if (op2 > opStart3) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 1638 | `HUF_decompress4X1_usingDTable_internal_body` (src/decompress/huf_decompress.c:682) | `if (op3 > opStart4) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 1639 | `HUF_decompress4X1_usingDTable_internal_body` (src/decompress/huf_decompress.c:693) | `if (!endCheck) return ERROR(corruption_detected); }` | `ERROR(corruption_detected)` | [ ] |
| 1640 | `HUF_decompress4X1_usingDTable_internal_fast_c_loop` (src/decompress/huf_decompress.c:735) | `assert(MEM_isLittleEndian());` | assertion failure | [ ] |
| 1641 | `HUF_decompress4X1_usingDTable_internal_fast_c_loop` (src/decompress/huf_decompress.c:736) | `assert(!MEM_32bits());` | assertion failure | [ ] |
| 1642 | `HUF_decompress4X1_usingDTable_internal_fast_c_loop` (src/decompress/huf_decompress.c:745) | `assert(op[stream] <= (stream == 3 ? oend : op[stream + 1]));` | assertion failure | [ ] |
| 1643 | `HUF_decompress4X1_usingDTable_internal_fast_c_loop` (src/decompress/huf_decompress.c:746) | `assert(ip[stream] >= ilowest);` | assertion failure | [ ] |
| 1644 | `HUF_decompress4X1_usingDTable_internal_fast_c_loop` (src/decompress/huf_decompress.c:776) | `if (ip[stream] < ip[stream - 1])` | branch-specific rejection/error | [ ] |
| 1645 | `HUF_decompress4X1_usingDTable_internal_fast_c_loop` (src/decompress/huf_decompress.c:783) | `assert(ip[stream] >= ip[stream - 1]);` | assertion failure | [ ] |
| 1646 | `HUF_decompress4X1_usingDTable_internal_fast` (src/decompress/huf_decompress.c:852) | `if (ret == 0)` | branch-specific rejection/error | [ ] |
| 1647 | `HUF_decompress4X1_usingDTable_internal_fast` (src/decompress/huf_decompress.c:856) | `assert(args.ip[0] >= args.ilowest);` | assertion failure | [ ] |
| 1648 | `HUF_decompress4X1_usingDTable_internal_fast` (src/decompress/huf_decompress.c:862) | `assert(args.ip[0] >= ilowest);` | assertion failure | [ ] |
| 1649 | `HUF_decompress4X1_usingDTable_internal_fast` (src/decompress/huf_decompress.c:863) | `assert(args.ip[0] >= ilowest);` | assertion failure | [ ] |
| 1650 | `HUF_decompress4X1_usingDTable_internal_fast` (src/decompress/huf_decompress.c:864) | `assert(args.ip[1] >= ilowest);` | assertion failure | [ ] |
| 1651 | `HUF_decompress4X1_usingDTable_internal_fast` (src/decompress/huf_decompress.c:865) | `assert(args.ip[2] >= ilowest);` | assertion failure | [ ] |
| 1652 | `HUF_decompress4X1_usingDTable_internal_fast` (src/decompress/huf_decompress.c:866) | `assert(args.ip[3] >= ilowest);` | assertion failure | [ ] |
| 1653 | `HUF_decompress4X1_usingDTable_internal_fast` (src/decompress/huf_decompress.c:867) | `assert(args.op[3] <= oend);` | assertion failure | [ ] |
| 1654 | `HUF_decompress4X1_usingDTable_internal_fast` (src/decompress/huf_decompress.c:869) | `assert(ilowest == args.ilowest);` | assertion failure | [ ] |
| 1655 | `HUF_decompress4X1_usingDTable_internal_fast` (src/decompress/huf_decompress.c:870) | `assert(ilowest + 6 == args.iend[0]);` | assertion failure | [ ] |
| 1656 | `HUF_decompress4X1_usingDTable_internal_fast` (src/decompress/huf_decompress.c:879) | `if (segmentSize <= (size_t)(oend - segmentEnd))` | branch-specific rejection/error | [ ] |
| 1657 | `HUF_decompress4X1_usingDTable_internal_fast` (src/decompress/huf_decompress.c:886) | `if (args.op[i] != segmentEnd) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 1658 | `HUF_decompress4X1_usingDTable_internal_fast` (src/decompress/huf_decompress.c:891) | `assert(dstSize != 0);` | assertion failure | [ ] |
| 1659 | `HUF_DGEN` (src/decompress/huf_decompress.c:924) | `if (ret != 0)` | branch-specific rejection/error | [ ] |
| 1660 | `HUF_decompress4X1_DCtx_wksp` (src/decompress/huf_decompress.c:938) | `if (hSize >= cSrcSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 1661 | `HUF_fillDTableX2ForWeight` (src/decompress/huf_decompress.c:1018) | `assert(level >= 1 && level <= 2);` | assertion failure | [ ] |
| 1662 | `HUF_fillDTableX2Level2` (src/decompress/huf_decompress.c:1078) | `if (minWeight>1) {` | branch-specific rejection/error | [ ] |
| 1663 | `HUF_fillDTableX2Level2` (src/decompress/huf_decompress.c:1082) | `assert(length > 1);` | assertion failure | [ ] |
| 1664 | `HUF_fillDTableX2Level2` (src/decompress/huf_decompress.c:1083) | `assert((U32)skipSize < length);` | assertion failure | [ ] |
| 1665 | `HUF_fillDTableX2Level2` (src/decompress/huf_decompress.c:1086) | `assert(skipSize == 1);` | assertion failure | [ ] |
| 1666 | `HUF_fillDTableX2Level2` (src/decompress/huf_decompress.c:1090) | `assert(skipSize <= 4);` | assertion failure | [ ] |
| 1667 | `HUF_fillDTableX2` (src/decompress/huf_decompress.c:1141) | `if (targetLog-nbBits >= minBits) {` | branch-specific rejection/error | [ ] |
| 1668 | `HUF_fillDTableX2` (src/decompress/huf_decompress.c:1147) | `if (minWeight < 1) minWeight = 1;` | branch-specific rejection/error | [ ] |
| 1669 | `HUF_readDTableX2_wksp` (src/decompress/huf_decompress.c:1193) | `if (sizeof(*wksp) > wkspSize) return ERROR(GENERIC);` | `ERROR(GENERIC)` | [ ] |
| 1670 | `HUF_readDTableX2_wksp` (src/decompress/huf_decompress.c:1200) | `if (maxTableLog > HUF_TABLELOG_MAX) return ERROR(tableLog_tooLarge);` | `ERROR(tableLog_tooLarge)` | [ ] |
| 1671 | `HUF_readDTableX2_wksp` (src/decompress/huf_decompress.c:1207) | `if (tableLog > maxTableLog) return ERROR(tableLog_tooLarge);   /* DTable can't fit code depth */` | `ERROR(tableLog_tooLarge)` | [ ] |
| 1672 | `HUF_readDTableX2_wksp` (src/decompress/huf_decompress.c:1208) | `if (tableLog <= HUF_DECODER_FAST_TABLELOG && maxTableLog > HUF_DECODER_FAST_TABLELOG) maxTableLog = HUF_DECODER_FAST_TABLELOG;` | branch-specific rejection/error | [ ] |
| 1673 | `HUF_decodeLastSymbolX2` (src/decompress/huf_decompress.c:1282) | `if (DStream->bitsConsumed < (sizeof(DStream->bitContainer)*8)) {` | branch-specific rejection/error | [ ] |
| 1674 | `HUF_decodeLastSymbolX2` (src/decompress/huf_decompress.c:1284) | `if (DStream->bitsConsumed > (sizeof(DStream->bitContainer)*8))` | branch-specific rejection/error | [ ] |
| 1675 | `HUF_decodeLastSymbolX2` (src/decompress/huf_decompress.c:1297) | `if (MEM_64bits() \|\| (HUF_TABLELOG_MAX<=12))                \` | branch-specific rejection/error | [ ] |
| 1676 | `HUF_decodeStreamX2` (src/decompress/huf_decompress.c:1314) | `if ((size_t)(pEnd - p) >= sizeof(bitDPtr->bitContainer)) {` | branch-specific rejection/error | [ ] |
| 1677 | `HUF_decodeStreamX2` (src/decompress/huf_decompress.c:1315) | `if (dtLog <= 11 && MEM_64bits()) {` | branch-specific rejection/error | [ ] |
| 1678 | `HUF_decodeStreamX2` (src/decompress/huf_decompress.c:1338) | `if ((size_t)(pEnd - p) >= 2) {` | branch-specific rejection/error | [ ] |
| 1679 | `HUF_decodeStreamX2` (src/decompress/huf_decompress.c:1346) | `if (p < pEnd)` | branch-specific rejection/error | [ ] |
| 1680 | `HUF_decompress1X2_usingDTable_internal_body` (src/decompress/huf_decompress.c:1373) | `if (!BIT_endOfDStream(&bitD)) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 1681 | `HUF_decompress4X2_usingDTable_internal_body` (src/decompress/huf_decompress.c:1389) | `if (cSrcSize < 10) return ERROR(corruption_detected);   /* strict minimum : jump table + 1 byte per stream */` | `ERROR(corruption_detected)` | [ ] |
| 1682 | `HUF_decompress4X2_usingDTable_internal_body` (src/decompress/huf_decompress.c:1390) | `if (dstSize < 6) return ERROR(corruption_detected);         /* stream 4-split doesn't work */` | `ERROR(corruption_detected)` | [ ] |
| 1683 | `HUF_decompress4X2_usingDTable_internal_body` (src/decompress/huf_decompress.c:1424) | `if (length4 > cSrcSize) return ERROR(corruption_detected);  /* overflow */` | `ERROR(corruption_detected)` | [ ] |
| 1684 | `HUF_decompress4X2_usingDTable_internal_body` (src/decompress/huf_decompress.c:1425) | `if (opStart4 > oend) return ERROR(corruption_detected);     /* overflow */` | `ERROR(corruption_detected)` | [ ] |
| 1685 | `HUF_decompress4X2_usingDTable_internal_body` (src/decompress/huf_decompress.c:1426) | `assert(dstSize >= 6 /* validated above */);` | assertion failure | [ ] |
| 1686 | `HUF_decompress4X2_usingDTable_internal_body` (src/decompress/huf_decompress.c:1433) | `if ((size_t)(oend - op4) >= sizeof(size_t)) {` | branch-specific rejection/error | [ ] |
| 1687 | `HUF_decompress4X2_usingDTable_internal_body` (src/decompress/huf_decompress.c:1483) | `if (op1 > opStart2) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 1688 | `HUF_decompress4X2_usingDTable_internal_body` (src/decompress/huf_decompress.c:1484) | `if (op2 > opStart3) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 1689 | `HUF_decompress4X2_usingDTable_internal_body` (src/decompress/huf_decompress.c:1485) | `if (op3 > opStart4) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 1690 | `HUF_decompress4X2_usingDTable_internal_body` (src/decompress/huf_decompress.c:1496) | `if (!endCheck) return ERROR(corruption_detected); }` | `ERROR(corruption_detected)` | [ ] |
| 1691 | `HUF_decompress4X2_usingDTable_internal_fast_c_loop` (src/decompress/huf_decompress.c:1543) | `assert(MEM_isLittleEndian());` | assertion failure | [ ] |
| 1692 | `HUF_decompress4X2_usingDTable_internal_fast_c_loop` (src/decompress/huf_decompress.c:1544) | `assert(!MEM_32bits());` | assertion failure | [ ] |
| 1693 | `HUF_decompress4X2_usingDTable_internal_fast_c_loop` (src/decompress/huf_decompress.c:1553) | `assert(op[stream] <= oend[stream]);` | assertion failure | [ ] |
| 1694 | `HUF_decompress4X2_usingDTable_internal_fast_c_loop` (src/decompress/huf_decompress.c:1554) | `assert(ip[stream] >= ilowest);` | assertion failure | [ ] |
| 1695 | `HUF_decompress4X2_usingDTable_internal_fast_c_loop` (src/decompress/huf_decompress.c:1594) | `if (ip[stream] < ip[stream - 1])` | branch-specific rejection/error | [ ] |
| 1696 | `HUF_decompress4X2_usingDTable_internal_fast_c_loop` (src/decompress/huf_decompress.c:1601) | `assert(ip[stream] >= ip[stream - 1]);` | assertion failure | [ ] |
| 1697 | `HUF_decompress4X2_usingDTable_internal_fast` (src/decompress/huf_decompress.c:1679) | `if (ret == 0)` | branch-specific rejection/error | [ ] |
| 1698 | `HUF_decompress4X2_usingDTable_internal_fast` (src/decompress/huf_decompress.c:1683) | `assert(args.ip[0] >= args.ilowest);` | assertion failure | [ ] |
| 1699 | `HUF_decompress4X2_usingDTable_internal_fast` (src/decompress/huf_decompress.c:1687) | `assert(args.ip[0] >= ilowest);` | assertion failure | [ ] |
| 1700 | `HUF_decompress4X2_usingDTable_internal_fast` (src/decompress/huf_decompress.c:1688) | `assert(args.ip[1] >= ilowest);` | assertion failure | [ ] |
| 1701 | `HUF_decompress4X2_usingDTable_internal_fast` (src/decompress/huf_decompress.c:1689) | `assert(args.ip[2] >= ilowest);` | assertion failure | [ ] |
| 1702 | `HUF_decompress4X2_usingDTable_internal_fast` (src/decompress/huf_decompress.c:1690) | `assert(args.ip[3] >= ilowest);` | assertion failure | [ ] |
| 1703 | `HUF_decompress4X2_usingDTable_internal_fast` (src/decompress/huf_decompress.c:1691) | `assert(args.op[3] <= oend);` | assertion failure | [ ] |
| 1704 | `HUF_decompress4X2_usingDTable_internal_fast` (src/decompress/huf_decompress.c:1693) | `assert(ilowest == args.ilowest);` | assertion failure | [ ] |
| 1705 | `HUF_decompress4X2_usingDTable_internal_fast` (src/decompress/huf_decompress.c:1694) | `assert(ilowest + 6 == args.iend[0]);` | assertion failure | [ ] |
| 1706 | `HUF_decompress4X2_usingDTable_internal_fast` (src/decompress/huf_decompress.c:1704) | `if (segmentSize <= (size_t)(oend - segmentEnd))` | branch-specific rejection/error | [ ] |
| 1707 | `HUF_decompress4X2_usingDTable_internal_fast` (src/decompress/huf_decompress.c:1711) | `return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 1708 | `HUF_decompress4X2_usingDTable_internal` (src/decompress/huf_decompress.c:1746) | `if (ret != 0)` | branch-specific rejection/error | [ ] |
| 1709 | `HUF_DGEN` (src/decompress/huf_decompress.c:1763) | `if (hSize >= cSrcSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 1710 | `HUF_decompress4X2_DCtx_wksp` (src/decompress/huf_decompress.c:1778) | `if (hSize >= cSrcSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 1711 | `HUF_selectDecoder` (src/decompress/huf_decompress.c:1823) | `assert(dstSize > 0);` | assertion failure | [ ] |
| 1712 | `HUF_selectDecoder` (src/decompress/huf_decompress.c:1824) | `assert(dstSize <= 128*1024);` | assertion failure | [ ] |
| 1713 | `HUF_decompress1X_DCtx_wksp` (src/decompress/huf_decompress.c:1850) | `if (dstSize == 0) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [ ] |
| 1714 | `HUF_decompress1X_DCtx_wksp` (src/decompress/huf_decompress.c:1851) | `if (cSrcSize > dstSize) return ERROR(corruption_detected);   /* invalid */` | `ERROR(corruption_detected)` | [ ] |
| 1715 | `HUF_decompress1X_DCtx_wksp` (src/decompress/huf_decompress.c:1858) | `assert(algoNb == 0);` | assertion failure | [ ] |
| 1716 | `HUF_decompress1X_DCtx_wksp` (src/decompress/huf_decompress.c:1863) | `assert(algoNb == 1);` | assertion failure | [ ] |
| 1717 | `HUF_decompress1X_usingDTable` (src/decompress/huf_decompress.c:1881) | `assert(dtd.tableType == 0);` | assertion failure | [ ] |
| 1718 | `HUF_decompress1X_usingDTable` (src/decompress/huf_decompress.c:1885) | `assert(dtd.tableType == 1);` | assertion failure | [ ] |
| 1719 | `HUF_decompress1X1_DCtx_wksp` (src/decompress/huf_decompress.c:1900) | `if (hSize >= cSrcSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 1720 | `HUF_decompress4X_usingDTable` (src/decompress/huf_decompress.c:1912) | `assert(dtd.tableType == 0);` | assertion failure | [ ] |
| 1721 | `HUF_decompress4X_usingDTable` (src/decompress/huf_decompress.c:1916) | `assert(dtd.tableType == 1);` | assertion failure | [ ] |
| 1722 | `HUF_decompress4X_hufOnly_wksp` (src/decompress/huf_decompress.c:1927) | `if (dstSize == 0) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [ ] |
| 1723 | `HUF_decompress4X_hufOnly_wksp` (src/decompress/huf_decompress.c:1928) | `if (cSrcSize == 0) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 1724 | `HUF_decompress4X_hufOnly_wksp` (src/decompress/huf_decompress.c:1933) | `assert(algoNb == 0);` | assertion failure | [ ] |
| 1725 | `HUF_decompress4X_hufOnly_wksp` (src/decompress/huf_decompress.c:1937) | `assert(algoNb == 1);` | assertion failure | [ ] |
| 1726 | `ZSTD_DDict_dictContent` (src/decompress/zstd_ddict.c:48) | `assert(ddict != NULL);` | assertion failure | [ ] |
| 1727 | `ZSTD_DDict_dictSize` (src/decompress/zstd_ddict.c:54) | `assert(ddict != NULL);` | assertion failure | [ ] |
| 1728 | `ZSTD_copyDDictParameters` (src/decompress/zstd_ddict.c:61) | `assert(dctx != NULL);` | assertion failure | [ ] |
| 1729 | `ZSTD_copyDDictParameters` (src/decompress/zstd_ddict.c:62) | `assert(ddict != NULL);` | assertion failure | [ ] |
| 1730 | `ZSTD_copyDDictParameters` (src/decompress/zstd_ddict.c:72) | `if (ddict->entropyPresent) {` | branch-specific rejection/error | [ ] |
| 1731 | `ZSTD_loadEntropy_intoDDict` (src/decompress/zstd_ddict.c:97) | `if (ddict->dictSize < 8) {` | `ERROR(dictionary_corrupted)` | [ ] |
| 1732 | `ZSTD_loadEntropy_intoDDict` (src/decompress/zstd_ddict.c:99) | `return ERROR(dictionary_corrupted);   /* only accept specified dictionaries */` | `ERROR(dictionary_corrupted)` | [ ] |
| 1733 | `ZSTD_loadEntropy_intoDDict` (src/decompress/zstd_ddict.c:105) | `return ERROR(dictionary_corrupted);   /* only accept specified dictionaries */` | `ERROR(dictionary_corrupted)` | [ ] |
| 1734 | `ZSTD_loadEntropy_intoDDict` (src/decompress/zstd_ddict.c:112) | `RETURN_ERROR_IF(ZSTD_isError(ZSTD_loadDEntropy(` | `ERROR(ddict)` | [ ] |
| 1735 | `ZSTD_initDDict_internal` (src/decompress/zstd_ddict.c:133) | `if (!internalBuffer) return ERROR(memory_allocation);` | `ERROR(memory_allocation)` | [ ] |
| 1736 | `ZSTD_createDDict_advanced` (src/decompress/zstd_ddict.c:150) | `if ((!customMem.customAlloc) ^ (!customMem.customFree)) return NULL;` | `NULL` | [ ] |
| 1737 | `ZSTD_createDDict_advanced` (src/decompress/zstd_ddict.c:153) | `if (ddict == NULL) return NULL;` | `NULL` | [ ] |
| 1738 | `ZSTD_createDDict_advanced` (src/decompress/zstd_ddict.c:160) | `return NULL;` | `NULL` | [ ] |
| 1739 | `ZSTD_initStaticDDict` (src/decompress/zstd_ddict.c:196) | `assert(sBuffer != NULL);` | `NULL` | [ ] |
| 1740 | `ZSTD_initStaticDDict` (src/decompress/zstd_ddict.c:197) | `assert(dict != NULL);` | `NULL` | [ ] |
| 1741 | `ZSTD_initStaticDDict` (src/decompress/zstd_ddict.c:198) | `if ((size_t)sBuffer & 7) return NULL;   /* 8-aligned */` | `NULL` | [ ] |
| 1742 | `ZSTD_initStaticDDict` (src/decompress/zstd_ddict.c:199) | `if (sBufferSize < neededSpace) return NULL;` | `NULL` | [ ] |
| 1743 | `ZSTD_initStaticDDict` (src/decompress/zstd_ddict.c:207) | `return NULL;` | `NULL` | [ ] |
| 1744 | `ZSTD_freeDDict` (src/decompress/zstd_ddict.c:214) | `if (ddict==NULL) return 0;   /* support free on NULL */` | branch-specific rejection/error | [ ] |
| 1745 | `ZSTD_sizeof_DDict` (src/decompress/zstd_ddict.c:232) | `if (ddict==NULL) return 0;   /* support sizeof on NULL */` | branch-specific rejection/error | [ ] |
| 1746 | `ZSTD_getDictID_fromDDict` (src/decompress/zstd_ddict.c:242) | `if (ddict==NULL) return 0;` | branch-specific rejection/error | [ ] |
| 1747 | `ZSTD_DDictHashSet_emplaceDDict` (src/decompress/zstd_decompress.c:109) | `RETURN_ERROR_IF(hashSet->ddictPtrCount == hashSet->ddictPtrTableSize, GENERIC, "Hash set is full!");` | `ERROR(GENERIC)` | [ ] |
| 1748 | `ZSTD_DDictHashSet_emplaceDDict` (src/decompress/zstd_decompress.c:113) | `if (ZSTD_getDictID_fromDDict(hashSet->ddictPtrTable[idx]) == dictID) {` | branch-specific rejection/error | [ ] |
| 1749 | `ZSTD_DDictHashSet_expand` (src/decompress/zstd_decompress.c:139) | `RETURN_ERROR_IF(!newTable, memory_allocation, "Expanded hashset allocation failed!");` | `ERROR(memory_allocation)` | [ ] |
| 1750 | `ZSTD_DDictHashSet_expand` (src/decompress/zstd_decompress.c:144) | `if (oldTable[i] != NULL) {` | branch-specific rejection/error | [ ] |
| 1751 | `ZSTD_DDictHashSet_getDDict` (src/decompress/zstd_decompress.c:162) | `if (currDictID == dictID \|\| currDictID == 0) {` | branch-specific rejection/error | [ ] |
| 1752 | `ZSTD_createDDictHashSet` (src/decompress/zstd_decompress.c:182) | `return NULL;` | `NULL` | [ ] |
| 1753 | `ZSTD_createDDictHashSet` (src/decompress/zstd_decompress.c:184) | `if (!ret->ddictPtrTable) {` | `NULL` | [ ] |
| 1754 | `ZSTD_createDDictHashSet` (src/decompress/zstd_decompress.c:186) | `return NULL;` | `NULL` | [ ] |
| 1755 | `ZSTD_freeDDictHashSet` (src/decompress/zstd_decompress.c:198) | `if (hashSet && hashSet->ddictPtrTable) {` | branch-specific rejection/error | [ ] |
| 1756 | `ZSTD_DDictHashSet_addDDict` (src/decompress/zstd_decompress.c:211) | `if (hashSet->ddictPtrCount * DDICT_HASHSET_MAX_LOAD_FACTOR_COUNT_MULT / hashSet->ddictPtrTableSize * DDICT_HASHSET_MAX_LOAD_FACTOR_SIZE_MULT != 0) {` | branch-specific rejection/error | [ ] |
| 1757 | `ZSTD_sizeof_DCtx` (src/decompress/zstd_decompress.c:223) | `if (dctx==NULL) return 0;   /* support sizeof NULL */` | branch-specific rejection/error | [ ] |
| 1758 | `ZSTD_startingInputLength` (src/decompress/zstd_decompress.c:236) | `assert( (format == ZSTD_f_zstd1) \|\| (format == ZSTD_f_zstd1_magicless) );` | assertion failure | [ ] |
| 1759 | `ZSTD_DCtx_resetParameters` (src/decompress/zstd_decompress.c:242) | `assert(dctx->streamStage == zdss_init);` | assertion failure | [ ] |
| 1760 | `ZSTD_initStaticDCtx` (src/decompress/zstd_decompress.c:285) | `if ((size_t)workspace & 7) return NULL;  /* 8-aligned */` | `NULL` | [ ] |
| 1761 | `ZSTD_initStaticDCtx` (src/decompress/zstd_decompress.c:286) | `if (workspaceSize < sizeof(ZSTD_DCtx)) return NULL;  /* minimum size */` | `NULL` | [ ] |
| 1762 | `ZSTD_createDCtx_internal` (src/decompress/zstd_decompress.c:295) | `if ((!customMem.customAlloc) ^ (!customMem.customFree)) return NULL;` | `NULL` | [ ] |
| 1763 | `ZSTD_createDCtx_internal` (src/decompress/zstd_decompress.c:298) | `if (!dctx) return NULL;` | `NULL` | [ ] |
| 1764 | `ZSTD_freeDCtx` (src/decompress/zstd_decompress.c:326) | `if (dctx==NULL) return 0;   /* support free on NULL */` | `ERROR(memory_allocation)` | [ ] |
| 1765 | `ZSTD_freeDCtx` (src/decompress/zstd_decompress.c:327) | `RETURN_ERROR_IF(dctx->staticSize, memory_allocation, "not compatible with static DCtx");` | `ERROR(memory_allocation)` | [ ] |
| 1766 | `ZSTD_freeDCtx` (src/decompress/zstd_decompress.c:333) | `if (dctx->legacyContext)` | branch-specific rejection/error | [ ] |
| 1767 | `ZSTD_freeDCtx` (src/decompress/zstd_decompress.c:336) | `if (dctx->ddictSet) {` | branch-specific rejection/error | [ ] |
| 1768 | `ZSTD_DCtx_selectFrameDDict` (src/decompress/zstd_decompress.c:361) | `assert(dctx->refMultipleDDicts && dctx->ddictSet);` | assertion failure | [ ] |
| 1769 | `ZSTD_DCtx_selectFrameDDict` (src/decompress/zstd_decompress.c:363) | `if (dctx->ddict) {` | branch-specific rejection/error | [ ] |
| 1770 | `ZSTD_isFrame` (src/decompress/zstd_decompress.c:387) | `if (size < ZSTD_FRAMEIDSIZE) return 0;` | branch-specific rejection/error | [ ] |
| 1771 | `ZSTD_isSkippableFrame` (src/decompress/zstd_decompress.c:404) | `if (size < ZSTD_FRAMEIDSIZE) return 0;` | branch-specific rejection/error | [ ] |
| 1772 | `ZSTD_frameHeaderSize_internal` (src/decompress/zstd_decompress.c:419) | `RETURN_ERROR_IF(srcSize < minInputSize, srcSize_wrong, "");` | `ERROR(srcSize_wrong)` | [ ] |
| 1773 | `ZSTD_getFrameHeader_advanced` (src/decompress/zstd_decompress.c:454) | `if (srcSize > 0) {` | `ERROR(GENERIC)` | [ ] |
| 1774 | `ZSTD_getFrameHeader_advanced` (src/decompress/zstd_decompress.c:456) | `RETURN_ERROR_IF(src==NULL, GENERIC, "invalid parameter : src==NULL, but srcSize>0");` | `ERROR(GENERIC)` | [ ] |
| 1775 | `ZSTD_getFrameHeader_advanced` (src/decompress/zstd_decompress.c:458) | `if (srcSize < minInputSize) {` | branch-specific rejection/error | [ ] |
| 1776 | `ZSTD_getFrameHeader_advanced` (src/decompress/zstd_decompress.c:459) | `if (srcSize > 0 && format != ZSTD_f_zstd1_magicless) {` | branch-specific rejection/error | [ ] |
| 1777 | `ZSTD_getFrameHeader_advanced` (src/decompress/zstd_decompress.c:466) | `assert(src != NULL);` | assertion failure | [ ] |
| 1778 | `ZSTD_getFrameHeader_advanced` (src/decompress/zstd_decompress.c:473) | `RETURN_ERROR(prefix_unknown,` | `ERROR(n)` | [ ] |
| 1779 | `ZSTD_getFrameHeader_advanced` (src/decompress/zstd_decompress.c:484) | `if (srcSize < ZSTD_SKIPPABLEHEADERSIZE)` | branch-specific rejection/error | [ ] |
| 1780 | `ZSTD_getFrameHeader_advanced` (src/decompress/zstd_decompress.c:493) | `RETURN_ERROR(prefix_unknown, "");` | `ERROR(n)` | [ ] |
| 1781 | `ZSTD_getFrameHeader_advanced` (src/decompress/zstd_decompress.c:498) | `if (srcSize < fhsize) return fhsize;` | branch-specific rejection/error | [ ] |
| 1782 | `ZSTD_getFrameHeader_advanced` (src/decompress/zstd_decompress.c:511) | `RETURN_ERROR_IF((fhdByte & 0x08) != 0, frameParameter_unsupported,` | `ERROR(8)` | [ ] |
| 1783 | `ZSTD_getFrameHeader_advanced` (src/decompress/zstd_decompress.c:517) | `RETURN_ERROR_IF(windowLog > ZSTD_WINDOWLOG_MAX, frameParameter_windowTooLarge, "");` | `ERROR(frameParameter_windowTooLarge)` | [ ] |
| 1784 | `ZSTD_getFrameHeader_advanced` (src/decompress/zstd_decompress.c:524) | `assert(0);  /* impossible */` | assertion failure | [ ] |
| 1785 | `ZSTD_getFrameHeader_advanced` (src/decompress/zstd_decompress.c:534) | `assert(0);  /* impossible */` | assertion failure | [ ] |
| 1786 | `ZSTD_getFrameContentSize` (src/decompress/zstd_decompress.c:578) | `if (ZSTD_getFrameHeader(&zfh, src, srcSize) != 0)` | branch-specific rejection/error | [ ] |
| 1787 | `readSkippableFrameSize` (src/decompress/zstd_decompress.c:592) | `RETURN_ERROR_IF(srcSize < ZSTD_SKIPPABLEHEADERSIZE, srcSize_wrong, "");` | `ERROR(srcSize_wrong)` | [ ] |
| 1788 | `readSkippableFrameSize` (src/decompress/zstd_decompress.c:595) | `RETURN_ERROR_IF((U32)(sizeU32 + ZSTD_SKIPPABLEHEADERSIZE) < sizeU32,` | `ERROR(2)` | [ ] |
| 1789 | `readSkippableFrameSize` (src/decompress/zstd_decompress.c:598) | `RETURN_ERROR_IF(skippableSize > srcSize, srcSize_wrong, "");` | `ERROR(srcSize_wrong)` | [ ] |
| 1790 | `ZSTD_readSkippableFrame` (src/decompress/zstd_decompress.c:618) | `RETURN_ERROR_IF(srcSize < ZSTD_SKIPPABLEHEADERSIZE, srcSize_wrong, "");` | `ERROR(srcSize_wrong)` | [ ] |
| 1791 | `ZSTD_readSkippableFrame` (src/decompress/zstd_decompress.c:625) | `RETURN_ERROR_IF(!ZSTD_isSkippableFrame(src, srcSize), frameParameter_unsupported, "");` | `ERROR(srcSize)` | [ ] |
| 1792 | `ZSTD_readSkippableFrame` (src/decompress/zstd_decompress.c:626) | `RETURN_ERROR_IF(skippableFrameSize < ZSTD_SKIPPABLEHEADERSIZE \|\| skippableFrameSize > srcSize, srcSize_wrong, "");` | `ERROR(srcSize_wrong)` | [ ] |
| 1793 | `ZSTD_readSkippableFrame` (src/decompress/zstd_decompress.c:627) | `RETURN_ERROR_IF(skippableContentSize > dstCapacity, dstSize_tooSmall, "");` | `ERROR(dstSize_tooSmall)` | [ ] |
| 1794 | `ZSTD_readSkippableFrame` (src/decompress/zstd_decompress.c:630) | `if (skippableContentSize > 0  && dst != NULL)` | branch-specific rejection/error | [ ] |
| 1795 | `ZSTD_readSkippableFrame` (src/decompress/zstd_decompress.c:632) | `if (magicVariant != NULL)` | branch-specific rejection/error | [ ] |
| 1796 | `ZSTD_findDecompressedSize` (src/decompress/zstd_decompress.c:653) | `assert(skippableSize <= srcSize);` | assertion failure | [ ] |
| 1797 | `ZSTD_findDecompressedSize` (src/decompress/zstd_decompress.c:661) | `if (fcs >= ZSTD_CONTENTSIZE_ERROR) return fcs;` | branch-specific rejection/error | [ ] |
| 1798 | `ZSTD_findDecompressedSize` (src/decompress/zstd_decompress.c:663) | `if (totalDstSize + fcs < totalDstSize)` | branch-specific rejection/error | [ ] |
| 1799 | `ZSTD_findDecompressedSize` (src/decompress/zstd_decompress.c:670) | `assert(frameSrcSize <= srcSize);` | assertion failure | [ ] |
| 1800 | `ZSTD_decodeFrameHeader` (src/decompress/zstd_decompress.c:706) | `RETURN_ERROR_IF(result>0, srcSize_wrong, "headerSize too small");` | `ERROR(srcSize_wrong)` | [ ] |
| 1801 | `ZSTD_decodeFrameHeader` (src/decompress/zstd_decompress.c:709) | `if (dctx->refMultipleDDicts == ZSTD_rmd_refMultipleDDicts && dctx->ddictSet) {` | branch-specific rejection/error | [ ] |
| 1802 | `ZSTD_decodeFrameHeader` (src/decompress/zstd_decompress.c:717) | `RETURN_ERROR_IF(dctx->fParams.dictID && (dctx->dictID != dctx->fParams.dictID),` | `ERROR(D)` | [ ] |
| 1803 | `ZSTD_decodeFrameHeader` (src/decompress/zstd_decompress.c:721) | `if (dctx->validateChecksum) XXH64_reset(&dctx->xxhState, 0);` | branch-specific rejection/error | [ ] |
| 1804 | `ZSTD_findFrameSizeInfo` (src/decompress/zstd_decompress.c:744) | `if (format == ZSTD_f_zstd1 && (srcSize >= ZSTD_SKIPPABLEHEADERSIZE)` | branch-specific rejection/error | [ ] |
| 1805 | `ZSTD_findFrameSizeInfo` (src/decompress/zstd_decompress.c:747) | `assert(ZSTD_isError(frameSizeInfo.compressedSize) \|\|` | assertion failure | [ ] |
| 1806 | `ZSTD_findFrameSizeInfo` (src/decompress/zstd_decompress.c:761) | `if (ret > 0)` | `ERROR(srcSize_wrong)` | [ ] |
| 1807 | `ZSTD_findFrameSizeInfo` (src/decompress/zstd_decompress.c:762) | `return ZSTD_errorFrameSizeInfo(ERROR(srcSize_wrong));` | `ERROR(srcSize_wrong)` | [ ] |
| 1808 | `ZSTD_findFrameSizeInfo` (src/decompress/zstd_decompress.c:775) | `if (ZSTD_blockHeaderSize + cBlockSize > remainingSize)` | `ERROR(srcSize_wrong)` | [ ] |
| 1809 | `ZSTD_findFrameSizeInfo` (src/decompress/zstd_decompress.c:776) | `return ZSTD_errorFrameSizeInfo(ERROR(srcSize_wrong));` | `ERROR(srcSize_wrong)` | [ ] |
| 1810 | `ZSTD_findFrameSizeInfo` (src/decompress/zstd_decompress.c:787) | `if (remainingSize < 4)` | `ERROR(srcSize_wrong)` | [ ] |
| 1811 | `ZSTD_findFrameSizeInfo` (src/decompress/zstd_decompress.c:788) | `return ZSTD_errorFrameSizeInfo(ERROR(srcSize_wrong));` | `ERROR(srcSize_wrong)` | [ ] |
| 1812 | `ZSTD_decompressBound` (src/decompress/zstd_decompress.c:830) | `assert(srcSize >= compressedSize);` | assertion failure | [ ] |
| 1813 | `ZSTD_decompressionMargin` (src/decompress/zstd_decompress.c:852) | `return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 1814 | `ZSTD_decompressionMargin` (src/decompress/zstd_decompress.c:865) | `assert(zfh.frameType == ZSTD_skippableFrame);` | assertion failure | [ ] |
| 1815 | `ZSTD_decompressionMargin` (src/decompress/zstd_decompress.c:870) | `assert(srcSize >= compressedSize);` | assertion failure | [ ] |
| 1816 | `ZSTD_copyRawBlock` (src/decompress/zstd_decompress.c:900) | `RETURN_ERROR_IF(srcSize > dstCapacity, dstSize_tooSmall, "");` | `ERROR(dstSize_tooSmall)` | [ ] |
| 1817 | `ZSTD_copyRawBlock` (src/decompress/zstd_decompress.c:901) | `if (dst == NULL) {` | `ERROR(l)` | [ ] |
| 1818 | `ZSTD_copyRawBlock` (src/decompress/zstd_decompress.c:902) | `if (srcSize == 0) return 0;` | `ERROR(l)` | [ ] |
| 1819 | `ZSTD_copyRawBlock` (src/decompress/zstd_decompress.c:903) | `RETURN_ERROR(dstBuffer_null, "");` | `ERROR(l)` | [ ] |
| 1820 | `ZSTD_setRleBlock` (src/decompress/zstd_decompress.c:913) | `RETURN_ERROR_IF(regenSize > dstCapacity, dstSize_tooSmall, "");` | `ERROR(dstSize_tooSmall)` | [ ] |
| 1821 | `ZSTD_setRleBlock` (src/decompress/zstd_decompress.c:914) | `if (dst == NULL) {` | `ERROR(l)` | [ ] |
| 1822 | `ZSTD_setRleBlock` (src/decompress/zstd_decompress.c:915) | `if (regenSize == 0) return 0;` | `ERROR(l)` | [ ] |
| 1823 | `ZSTD_setRleBlock` (src/decompress/zstd_decompress.c:916) | `RETURN_ERROR(dstBuffer_null, "");` | `ERROR(l)` | [ ] |
| 1824 | `ZSTD_DCtx_trace_end` (src/decompress/zstd_decompress.c:925) | `if (dctx->traceCtx && ZSTD_trace_decompress_end != NULL) {` | branch-specific rejection/error | [ ] |
| 1825 | `ZSTD_DCtx_trace_end` (src/decompress/zstd_decompress.c:930) | `if (dctx->ddict) {` | branch-specific rejection/error | [ ] |
| 1826 | `ZSTD_decompressFrame` (src/decompress/zstd_decompress.c:967) | `RETURN_ERROR_IF(` | `ERROR(t)` | [ ] |
| 1827 | `ZSTD_decompressFrame` (src/decompress/zstd_decompress.c:975) | `RETURN_ERROR_IF(remainingSrcSize < frameHeaderSize+ZSTD_blockHeaderSize,` | `ERROR(srcSize_wrong)` | [ ] |
| 1828 | `ZSTD_decompressFrame` (src/decompress/zstd_decompress.c:982) | `if (dctx->maxBlockSizeParam != 0)` | branch-specific rejection/error | [ ] |
| 1829 | `ZSTD_decompressFrame` (src/decompress/zstd_decompress.c:995) | `RETURN_ERROR_IF(cBlockSize > remainingSrcSize, srcSize_wrong, "");` | `ERROR(srcSize_wrong)` | [ ] |
| 1830 | `ZSTD_decompressFrame` (src/decompress/zstd_decompress.c:997) | `if (ip >= op && ip < oBlockEnd) {` | branch-specific rejection/error | [ ] |
| 1831 | `ZSTD_decompressFrame` (src/decompress/zstd_decompress.c:1017) | `assert(dctx->isFrameDecompression == 1);` | assertion failure | [ ] |
| 1832 | `ZSTD_decompressFrame` (src/decompress/zstd_decompress.c:1029) | `RETURN_ERROR(corruption_detected, "invalid block type");` | `ERROR(d)` | [ ] |
| 1833 | `ZSTD_decompressFrame` (src/decompress/zstd_decompress.c:1033) | `if (dctx->validateChecksum) {` | branch-specific rejection/error | [ ] |
| 1834 | `ZSTD_decompressFrame` (src/decompress/zstd_decompress.c:1036) | `if (decodedSize) /* support dst = NULL,0 */ {` | branch-specific rejection/error | [ ] |
| 1835 | `ZSTD_decompressFrame` (src/decompress/zstd_decompress.c:1039) | `assert(ip != NULL);` | assertion failure | [ ] |
| 1836 | `ZSTD_decompressFrame` (src/decompress/zstd_decompress.c:1045) | `if (dctx->fParams.frameContentSize != ZSTD_CONTENTSIZE_UNKNOWN) {` | `ERROR(4)` | [ ] |
| 1837 | `ZSTD_decompressFrame` (src/decompress/zstd_decompress.c:1046) | `RETURN_ERROR_IF((U64)(op-ostart) != dctx->fParams.frameContentSize,` | `ERROR(4)` | [ ] |
| 1838 | `ZSTD_decompressFrame` (src/decompress/zstd_decompress.c:1049) | `if (dctx->fParams.checksumFlag) { /* Frame content checksum verification */` | `ERROR(checksum_wrong)` | [ ] |
| 1839 | `ZSTD_decompressFrame` (src/decompress/zstd_decompress.c:1050) | `RETURN_ERROR_IF(remainingSrcSize<4, checksum_wrong, "");` | `ERROR(checksum_wrong)` | [ ] |
| 1840 | `ZSTD_decompressFrame` (src/decompress/zstd_decompress.c:1051) | `if (!dctx->forceIgnoreChecksum) {` | `ERROR(checksum_wrong)` | [ ] |
| 1841 | `ZSTD_decompressFrame` (src/decompress/zstd_decompress.c:1055) | `RETURN_ERROR_IF(checkRead != checkCalc, checksum_wrong, "");` | `ERROR(checksum_wrong)` | [ ] |
| 1842 | `ZSTD_decompressMultiFrame` (src/decompress/zstd_decompress.c:1080) | `assert(dict==NULL \|\| ddict==NULL);  /* either dict or ddict set, not both */` | assertion failure | [ ] |
| 1843 | `ZSTD_decompressMultiFrame` (src/decompress/zstd_decompress.c:1090) | `if (dctx->format == ZSTD_f_zstd1 && ZSTD_isLegacy(src, srcSize)) {` | `ERROR(memory_allocation)` | [ ] |
| 1844 | `ZSTD_decompressMultiFrame` (src/decompress/zstd_decompress.c:1094) | `RETURN_ERROR_IF(dctx->staticSize, memory_allocation,` | `ERROR(memory_allocation)` | [ ] |
| 1845 | `ZSTD_decompressMultiFrame` (src/decompress/zstd_decompress.c:1102) | `RETURN_ERROR_IF(expectedSize == ZSTD_CONTENTSIZE_ERROR, corruption_detected, "Corrupted frame header!");` | `ERROR(corruption_detected)` | [ ] |
| 1846 | `ZSTD_decompressMultiFrame` (src/decompress/zstd_decompress.c:1104) | `RETURN_ERROR_IF(expectedSize != decodedSize, corruption_detected,` | `ERROR(corruption_detected)` | [ ] |
| 1847 | `ZSTD_decompressMultiFrame` (src/decompress/zstd_decompress.c:1109) | `assert(decodedSize <= dstCapacity);` | assertion failure | [ ] |
| 1848 | `ZSTD_decompressMultiFrame` (src/decompress/zstd_decompress.c:1120) | `if (dctx->format == ZSTD_f_zstd1 && srcSize >= 4) {` | branch-specific rejection/error | [ ] |
| 1849 | `ZSTD_decompressMultiFrame` (src/decompress/zstd_decompress.c:1127) | `assert(skippableSize <= srcSize);` | assertion failure | [ ] |
| 1850 | `ZSTD_decompressMultiFrame` (src/decompress/zstd_decompress.c:1146) | `RETURN_ERROR_IF(` | `ERROR(s)` | [ ] |
| 1851 | `ZSTD_decompressMultiFrame` (src/decompress/zstd_decompress.c:1158) | `assert(res <= dstCapacity);` | assertion failure | [ ] |
| 1852 | `ZSTD_decompressMultiFrame` (src/decompress/zstd_decompress.c:1159) | `if (res != 0)` | branch-specific rejection/error | [ ] |
| 1853 | `ZSTD_decompressMultiFrame` (src/decompress/zstd_decompress.c:1166) | `RETURN_ERROR_IF(srcSize, srcSize_wrong, "input not entirely consumed");` | `ERROR(srcSize_wrong)` | [ ] |
| 1854 | `ZSTD_getDDict` (src/decompress/zstd_decompress.c:1184) | `assert(0 /* Impossible */);` | `NULL` | [ ] |
| 1855 | `ZSTD_getDDict` (src/decompress/zstd_decompress.c:1188) | `return NULL;` | `NULL` | [ ] |
| 1856 | `ZSTD_decompress` (src/decompress/zstd_decompress.c:1208) | `RETURN_ERROR_IF(dctx==NULL, memory_allocation, "NULL pointer!");` | `ERROR(memory_allocation)` | [ ] |
| 1857 | `ZSTD_nextSrcSizeToDecompressWithInputSize` (src/decompress/zstd_decompress.c:1237) | `if (!(dctx->stage == ZSTDds_decompressBlock \|\| dctx->stage == ZSTDds_decompressLastBlock))` | branch-specific rejection/error | [ ] |
| 1858 | `ZSTD_nextSrcSizeToDecompressWithInputSize` (src/decompress/zstd_decompress.c:1239) | `if (dctx->bType != bt_raw)` | branch-specific rejection/error | [ ] |
| 1859 | `ZSTD_nextInputType` (src/decompress/zstd_decompress.c:1248) | `assert(0);` | assertion failure | [ ] |
| 1860 | `ZSTD_decompressContinue` (src/decompress/zstd_decompress.c:1279) | `RETURN_ERROR_IF(srcSize != ZSTD_nextSrcSizeToDecompressWithInputSize(dctx, srcSize), srcSize_wrong, "not allowed");` | `ERROR(srcSize)` | [ ] |
| 1861 | `ZSTD_decompressContinue` (src/decompress/zstd_decompress.c:1287) | `assert(src != NULL);` | assertion failure | [ ] |
| 1862 | `ZSTD_decompressContinue` (src/decompress/zstd_decompress.c:1288) | `if (dctx->format == ZSTD_f_zstd1) {  /* allows header */` | branch-specific rejection/error | [ ] |
| 1863 | `ZSTD_decompressContinue` (src/decompress/zstd_decompress.c:1289) | `assert(srcSize >= ZSTD_FRAMEIDSIZE);  /* to read skippable magic number */` | assertion failure | [ ] |
| 1864 | `ZSTD_decompressContinue` (src/decompress/zstd_decompress.c:1297) | `if (ZSTD_isError(dctx->headerSize)) return dctx->headerSize;` | branch-specific rejection/error | [ ] |
| 1865 | `ZSTD_decompressContinue` (src/decompress/zstd_decompress.c:1304) | `assert(src != NULL);` | assertion failure | [ ] |
| 1866 | `ZSTD_decompressContinue` (src/decompress/zstd_decompress.c:1315) | `RETURN_ERROR_IF(cBlockSize > dctx->fParams.blockSizeMax, corruption_detected, "Block Size Exceeds Maximum");` | `ERROR(corruption_detected)` | [ ] |
| 1867 | `ZSTD_decompressContinue` (src/decompress/zstd_decompress.c:1325) | `if (dctx->fParams.checksumFlag) {` | branch-specific rejection/error | [ ] |
| 1868 | `ZSTD_decompressContinue` (src/decompress/zstd_decompress.c:1347) | `assert(dctx->isFrameDecompression == 1);` | assertion failure | [ ] |
| 1869 | `ZSTD_decompressContinue` (src/decompress/zstd_decompress.c:1352) | `assert(srcSize <= dctx->expected);` | assertion failure | [ ] |
| 1870 | `ZSTD_decompressContinue` (src/decompress/zstd_decompress.c:1355) | `assert(rSize == srcSize);` | assertion failure | [ ] |
| 1871 | `ZSTD_decompressContinue` (src/decompress/zstd_decompress.c:1364) | `RETURN_ERROR(corruption_detected, "invalid block type");` | `ERROR(d)` | [ ] |
| 1872 | `ZSTD_decompressContinue` (src/decompress/zstd_decompress.c:1367) | `RETURN_ERROR_IF(rSize > dctx->fParams.blockSizeMax, corruption_detected, "Decompressed Block Size Exceeds Maximum");` | `ERROR(corruption_detected)` | [ ] |
| 1873 | `ZSTD_decompressContinue` (src/decompress/zstd_decompress.c:1370) | `if (dctx->validateChecksum) XXH64_update(&dctx->xxhState, dst, rSize);` | branch-specific rejection/error | [ ] |
| 1874 | `ZSTD_decompressContinue` (src/decompress/zstd_decompress.c:1374) | `if (dctx->expected > 0) {` | branch-specific rejection/error | [ ] |
| 1875 | `ZSTD_decompressContinue` (src/decompress/zstd_decompress.c:1378) | `if (dctx->stage == ZSTDds_decompressLastBlock) {   /* end of frame */` | `ERROR(e)` | [ ] |
| 1876 | `ZSTD_decompressContinue` (src/decompress/zstd_decompress.c:1380) | `RETURN_ERROR_IF(` | `ERROR(corruption_detected)` | [ ] |
| 1877 | `ZSTD_decompressContinue` (src/decompress/zstd_decompress.c:1384) | `if (dctx->fParams.checksumFlag) {  /* another round for frame checksum */` | branch-specific rejection/error | [ ] |
| 1878 | `ZSTD_decompressContinue` (src/decompress/zstd_decompress.c:1400) | `assert(srcSize == 4);  /* guaranteed by dctx->expected */` | assertion failure | [ ] |
| 1879 | `ZSTD_decompressContinue` (src/decompress/zstd_decompress.c:1402) | `if (dctx->validateChecksum) {` | `ERROR(checksum_wrong)` | [ ] |
| 1880 | `ZSTD_decompressContinue` (src/decompress/zstd_decompress.c:1406) | `RETURN_ERROR_IF(check32 != h32, checksum_wrong, "");` | `ERROR(checksum_wrong)` | [ ] |
| 1881 | `ZSTD_decompressContinue` (src/decompress/zstd_decompress.c:1415) | `assert(src != NULL);` | assertion failure | [ ] |
| 1882 | `ZSTD_decompressContinue` (src/decompress/zstd_decompress.c:1416) | `assert(srcSize <= ZSTD_SKIPPABLEHEADERSIZE);` | assertion failure | [ ] |
| 1883 | `ZSTD_decompressContinue` (src/decompress/zstd_decompress.c:1417) | `assert(dctx->format != ZSTD_f_zstd1_magicless);` | assertion failure | [ ] |
| 1884 | `ZSTD_decompressContinue` (src/decompress/zstd_decompress.c:1429) | `assert(0);   /* impossible */` | `ERROR(C)` | [ ] |
| 1885 | `ZSTD_decompressContinue` (src/decompress/zstd_decompress.c:1430) | `RETURN_ERROR(GENERIC, "impossible to reach");   /* some compilers require default to do something */` | `ERROR(C)` | [ ] |
| 1886 | `ZSTD_loadDEntropy` (src/decompress/zstd_decompress.c:1458) | `RETURN_ERROR_IF(dictSize <= 8, dictionary_corrupted, "dict is too small");` | `ERROR(dictionary_corrupted)` | [ ] |
| 1887 | `ZSTD_loadDEntropy` (src/decompress/zstd_decompress.c:1459) | `assert(MEM_readLE32(dict) == ZSTD_MAGIC_DICTIONARY);   /* dict must be valid */` | assertion failure | [ ] |
| 1888 | `ZSTD_loadDEntropy` (src/decompress/zstd_decompress.c:1477) | `RETURN_ERROR_IF(HUF_isError(hSize), dictionary_corrupted, "");` | `ERROR(e)` | [ ] |
| 1889 | `ZSTD_loadDEntropy` (src/decompress/zstd_decompress.c:1484) | `RETURN_ERROR_IF(FSE_isError(offcodeHeaderSize), dictionary_corrupted, "");` | `ERROR(e)` | [ ] |
| 1890 | `ZSTD_loadDEntropy` (src/decompress/zstd_decompress.c:1485) | `RETURN_ERROR_IF(offcodeMaxValue > MaxOff, dictionary_corrupted, "");` | `ERROR(dictionary_corrupted)` | [ ] |
| 1891 | `ZSTD_loadDEntropy` (src/decompress/zstd_decompress.c:1486) | `RETURN_ERROR_IF(offcodeLog > OffFSELog, dictionary_corrupted, "");` | `ERROR(dictionary_corrupted)` | [ ] |
| 1892 | `ZSTD_loadDEntropy` (src/decompress/zstd_decompress.c:1499) | `RETURN_ERROR_IF(FSE_isError(matchlengthHeaderSize), dictionary_corrupted, "");` | `ERROR(e)` | [ ] |
| 1893 | `ZSTD_loadDEntropy` (src/decompress/zstd_decompress.c:1500) | `RETURN_ERROR_IF(matchlengthMaxValue > MaxML, dictionary_corrupted, "");` | `ERROR(dictionary_corrupted)` | [ ] |
| 1894 | `ZSTD_loadDEntropy` (src/decompress/zstd_decompress.c:1501) | `RETURN_ERROR_IF(matchlengthLog > MLFSELog, dictionary_corrupted, "");` | `ERROR(dictionary_corrupted)` | [ ] |
| 1895 | `ZSTD_loadDEntropy` (src/decompress/zstd_decompress.c:1514) | `RETURN_ERROR_IF(FSE_isError(litlengthHeaderSize), dictionary_corrupted, "");` | `ERROR(e)` | [ ] |
| 1896 | `ZSTD_loadDEntropy` (src/decompress/zstd_decompress.c:1515) | `RETURN_ERROR_IF(litlengthMaxValue > MaxLL, dictionary_corrupted, "");` | `ERROR(dictionary_corrupted)` | [ ] |
| 1897 | `ZSTD_loadDEntropy` (src/decompress/zstd_decompress.c:1516) | `RETURN_ERROR_IF(litlengthLog > LLFSELog, dictionary_corrupted, "");` | `ERROR(dictionary_corrupted)` | [ ] |
| 1898 | `ZSTD_loadDEntropy` (src/decompress/zstd_decompress.c:1526) | `RETURN_ERROR_IF(dictPtr+12 > dictEnd, dictionary_corrupted, "");` | `ERROR(dictionary_corrupted)` | [ ] |
| 1899 | `ZSTD_loadDEntropy` (src/decompress/zstd_decompress.c:1531) | `RETURN_ERROR_IF(rep==0 \|\| rep > dictContentSize,` | `ERROR(dictionary_corrupted)` | [ ] |
| 1900 | `ZSTD_decompress_insertDictionary` (src/decompress/zstd_decompress.c:1541) | `if (dictSize < 8) return ZSTD_refDictContent(dctx, dict, dictSize);` | branch-specific rejection/error | [ ] |
| 1901 | `ZSTD_decompress_insertDictionary` (src/decompress/zstd_decompress.c:1550) | `RETURN_ERROR_IF(ZSTD_isError(eSize), dictionary_corrupted, "");` | `ERROR(e)` | [ ] |
| 1902 | `ZSTD_decompressBegin` (src/decompress/zstd_decompress.c:1562) | `assert(dctx != NULL);` | assertion failure | [ ] |
| 1903 | `ZSTD_decompressBegin_usingDict` (src/decompress/zstd_decompress.c:1592) | `RETURN_ERROR_IF(` | `ERROR(dict)` | [ ] |
| 1904 | `ZSTD_decompressBegin_usingDDict` (src/decompress/zstd_decompress.c:1604) | `assert(dctx != NULL);` | assertion failure | [ ] |
| 1905 | `ZSTD_decompressBegin_usingDDict` (src/decompress/zstd_decompress.c:1614) | `if (ddict) {   /* NULL ddict is equivalent to no dictionary */` | branch-specific rejection/error | [ ] |
| 1906 | `ZSTD_getDictID_fromDict` (src/decompress/zstd_decompress.c:1626) | `if (dictSize < 8) return 0;` | branch-specific rejection/error | [ ] |
| 1907 | `ZSTD_DCtx_loadDictionary_advanced` (src/decompress/zstd_decompress.c:1704) | `RETURN_ERROR_IF(dctx->streamStage != zdss_init, stage_wrong, "");` | `ERROR(stage_wrong)` | [ ] |
| 1908 | `ZSTD_DCtx_loadDictionary_advanced` (src/decompress/zstd_decompress.c:1706) | `if (dict && dictSize != 0) {` | `ERROR(memory_allocation)` | [ ] |
| 1909 | `ZSTD_DCtx_loadDictionary_advanced` (src/decompress/zstd_decompress.c:1708) | `RETURN_ERROR_IF(dctx->ddictLocal == NULL, memory_allocation, "NULL pointer!");` | `ERROR(memory_allocation)` | [ ] |
| 1910 | `ZSTD_DCtx_refDDict` (src/decompress/zstd_decompress.c:1782) | `RETURN_ERROR_IF(dctx->streamStage != zdss_init, stage_wrong, "");` | `ERROR(stage_wrong)` | [ ] |
| 1911 | `ZSTD_DCtx_refDDict` (src/decompress/zstd_decompress.c:1787) | `if (dctx->refMultipleDDicts == ZSTD_rmd_refMultipleDDicts) {` | `ERROR(n)` | [ ] |
| 1912 | `ZSTD_DCtx_refDDict` (src/decompress/zstd_decompress.c:1788) | `if (dctx->ddictSet == NULL) {` | `ERROR(n)` | [ ] |
| 1913 | `ZSTD_DCtx_refDDict` (src/decompress/zstd_decompress.c:1790) | `if (!dctx->ddictSet) {` | `ERROR(n)` | [ ] |
| 1914 | `ZSTD_DCtx_refDDict` (src/decompress/zstd_decompress.c:1791) | `RETURN_ERROR(memory_allocation, "Failed to allocate memory for hash set!");` | `ERROR(n)` | [ ] |
| 1915 | `ZSTD_DCtx_refDDict` (src/decompress/zstd_decompress.c:1794) | `assert(!dctx->staticSize);  /* Impossible: ddictSet cannot have been allocated if static dctx */` | assertion failure | [ ] |
| 1916 | `ZSTD_DCtx_setMaxWindowSize` (src/decompress/zstd_decompress.c:1809) | `RETURN_ERROR_IF(dctx->streamStage != zdss_init, stage_wrong, "");` | `ERROR(stage_wrong)` | [ ] |
| 1917 | `ZSTD_DCtx_setMaxWindowSize` (src/decompress/zstd_decompress.c:1810) | `RETURN_ERROR_IF(maxWindowSize < min, parameter_outOfBound, "");` | `ERROR(parameter_outOfBound)` | [ ] |
| 1918 | `ZSTD_DCtx_setMaxWindowSize` (src/decompress/zstd_decompress.c:1811) | `RETURN_ERROR_IF(maxWindowSize > max, parameter_outOfBound, "");` | `ERROR(parameter_outOfBound)` | [ ] |
| 1919 | `ZSTD_dParam_getBounds` (src/decompress/zstd_decompress.c:1857) | `bounds.error = ERROR(parameter_unsupported);` | `ERROR(parameter_unsupported)` | [ ] |
| 1920 | `ZSTD_dParam_withinBounds` (src/decompress/zstd_decompress.c:1868) | `if (value < bounds.lowerBound) return 0;` | branch-specific rejection/error | [ ] |
| 1921 | `ZSTD_dParam_withinBounds` (src/decompress/zstd_decompress.c:1869) | `if (value > bounds.upperBound) return 0;` | branch-specific rejection/error | [ ] |
| 1922 | `ZSTD_dParam_withinBounds` (src/decompress/zstd_decompress.c:1874) | `RETURN_ERROR_IF(!ZSTD_dParam_withinBounds(p, v), parameter_outOfBound, ""); \` | `ERROR(v)` | [ ] |
| 1923 | `ZSTD_DCtx_getParameter` (src/decompress/zstd_decompress.c:1903) | `RETURN_ERROR(parameter_unsupported, "");` | `ERROR(d)` | [ ] |
| 1924 | `ZSTD_DCtx_setParameter` (src/decompress/zstd_decompress.c:1908) | `RETURN_ERROR_IF(dctx->streamStage != zdss_init, stage_wrong, "");` | `ERROR(stage_wrong)` | [ ] |
| 1925 | `ZSTD_DCtx_setParameter` (src/decompress/zstd_decompress.c:1911) | `if (value == 0) value = ZSTD_WINDOWLOG_LIMIT_DEFAULT;` | branch-specific rejection/error | [ ] |
| 1926 | `ZSTD_DCtx_setParameter` (src/decompress/zstd_decompress.c:1929) | `if (dctx->staticSize != 0) {` | `ERROR(d)` | [ ] |
| 1927 | `ZSTD_DCtx_setParameter` (src/decompress/zstd_decompress.c:1930) | `RETURN_ERROR(parameter_unsupported, "Static dctx does not support multiple DDicts!");` | `ERROR(d)` | [ ] |
| 1928 | `ZSTD_DCtx_setParameter` (src/decompress/zstd_decompress.c:1939) | `if (value != 0) CHECK_DBOUNDS(ZSTD_d_maxBlockSize, value);` | branch-specific rejection/error | [ ] |
| 1929 | `ZSTD_DCtx_setParameter` (src/decompress/zstd_decompress.c:1944) | `RETURN_ERROR(parameter_unsupported, "");` | `ERROR(d)` | [ ] |
| 1930 | `ZSTD_DCtx_reset` (src/decompress/zstd_decompress.c:1957) | `RETURN_ERROR_IF(dctx->streamStage != zdss_init, stage_wrong, "");` | `ERROR(stage_wrong)` | [ ] |
| 1931 | `ZSTD_decodingBufferSize_internal` (src/decompress/zstd_decompress.c:1983) | `RETURN_ERROR_IF((unsigned long long)minRBSize != neededSize,` | `ERROR(g)` | [ ] |
| 1932 | `ZSTD_estimateDStreamSize_fromFrame` (src/decompress/zstd_decompress.c:2007) | `RETURN_ERROR_IF(err>0, srcSize_wrong, "");` | `ERROR(srcSize_wrong)` | [ ] |
| 1933 | `ZSTD_estimateDStreamSize_fromFrame` (src/decompress/zstd_decompress.c:2008) | `RETURN_ERROR_IF(zfh.windowSize > windowSizeMax,` | `ERROR(frameParameter_windowTooLarge)` | [ ] |
| 1934 | `ZSTD_checkOutBuffer` (src/decompress/zstd_decompress.c:2039) | `if (zds->outBufferMode != ZSTD_bm_stable)` | branch-specific rejection/error | [ ] |
| 1935 | `ZSTD_checkOutBuffer` (src/decompress/zstd_decompress.c:2044) | `if (zds->streamStage == zdss_init)` | branch-specific rejection/error | [ ] |
| 1936 | `ZSTD_checkOutBuffer` (src/decompress/zstd_decompress.c:2047) | `if (expect.dst == output->dst && expect.pos == output->pos && expect.size == output->size)` | `ERROR(g)` | [ ] |
| 1937 | `ZSTD_checkOutBuffer` (src/decompress/zstd_decompress.c:2049) | `RETURN_ERROR(dstBuffer_wrong, "ZSTD_d_stableOutBuffer enabled but output differs!");` | `ERROR(g)` | [ ] |
| 1938 | `ZSTD_decompressContinueStream` (src/decompress/zstd_decompress.c:2061) | `if (zds->outBufferMode == ZSTD_bm_buffered) {` | branch-specific rejection/error | [ ] |
| 1939 | `ZSTD_decompressContinueStream` (src/decompress/zstd_decompress.c:2080) | `assert(*op <= oend);` | assertion failure | [ ] |
| 1940 | `ZSTD_decompressContinueStream` (src/decompress/zstd_decompress.c:2081) | `assert(zds->outBufferMode == ZSTD_bm_stable);` | assertion failure | [ ] |
| 1941 | `ZSTD_decompressStream` (src/decompress/zstd_decompress.c:2099) | `assert(zds != NULL);` | `ERROR(srcSize_wrong)` | [ ] |
| 1942 | `ZSTD_decompressStream` (src/decompress/zstd_decompress.c:2100) | `RETURN_ERROR_IF(` | `ERROR(srcSize_wrong)` | [ ] |
| 1943 | `ZSTD_decompressStream` (src/decompress/zstd_decompress.c:2105) | `RETURN_ERROR_IF(` | `ERROR(dstSize_tooSmall)` | [ ] |
| 1944 | `ZSTD_decompressStream` (src/decompress/zstd_decompress.c:2130) | `if (zds->legacyVersion) {` | `ERROR(memory_allocation)` | [ ] |
| 1945 | `ZSTD_decompressStream` (src/decompress/zstd_decompress.c:2131) | `RETURN_ERROR_IF(zds->staticSize, memory_allocation,` | `ERROR(memory_allocation)` | [ ] |
| 1946 | `ZSTD_decompressStream` (src/decompress/zstd_decompress.c:2134) | `if (hint==0) zds->streamStage = zdss_init;` | branch-specific rejection/error | [ ] |
| 1947 | `ZSTD_decompressStream` (src/decompress/zstd_decompress.c:2139) | `if (zds->refMultipleDDicts && zds->ddictSet) {` | branch-specific rejection/error | [ ] |
| 1948 | `ZSTD_decompressStream` (src/decompress/zstd_decompress.c:2150) | `RETURN_ERROR_IF(zds->staticSize, memory_allocation,` | `ERROR(memory_allocation)` | [ ] |
| 1949 | `ZSTD_decompressStream` (src/decompress/zstd_decompress.c:2157) | `if (hint==0) zds->streamStage = zdss_init;   /* or stay in stage zdss_loadHeader */` | branch-specific rejection/error | [ ] |
| 1950 | `ZSTD_decompressStream` (src/decompress/zstd_decompress.c:2163) | `if (hSize != 0) {   /* need more input */` | branch-specific rejection/error | [ ] |
| 1951 | `ZSTD_decompressStream` (src/decompress/zstd_decompress.c:2166) | `assert(iend >= ip);` | assertion failure | [ ] |
| 1952 | `ZSTD_decompressStream` (src/decompress/zstd_decompress.c:2167) | `if (toLoad > remainingInput) {   /* not enough input to load full header */` | branch-specific rejection/error | [ ] |
| 1953 | `ZSTD_decompressStream` (src/decompress/zstd_decompress.c:2168) | `if (remainingInput > 0) {` | branch-specific rejection/error | [ ] |
| 1954 | `ZSTD_decompressStream` (src/decompress/zstd_decompress.c:2180) | `assert(ip != NULL);` | assertion failure | [ ] |
| 1955 | `ZSTD_decompressStream` (src/decompress/zstd_decompress.c:2186) | `if (zds->fParams.frameContentSize != ZSTD_CONTENTSIZE_UNKNOWN` | branch-specific rejection/error | [ ] |
| 1956 | `ZSTD_decompressStream` (src/decompress/zstd_decompress.c:2190) | `if (cSize <= (size_t)(iend-istart)) {` | branch-specific rejection/error | [ ] |
| 1957 | `ZSTD_decompressStream` (src/decompress/zstd_decompress.c:2195) | `assert(istart != NULL);` | assertion failure | [ ] |
| 1958 | `ZSTD_decompressStream` (src/decompress/zstd_decompress.c:2205) | `if (zds->outBufferMode == ZSTD_bm_stable` | `ERROR(l)` | [ ] |
| 1959 | `ZSTD_decompressStream` (src/decompress/zstd_decompress.c:2209) | `RETURN_ERROR(dstSize_tooSmall, "ZSTD_obm_stable passed but ZSTD_outBuffer is too small");` | `ERROR(l)` | [ ] |
| 1960 | `ZSTD_decompressStream` (src/decompress/zstd_decompress.c:2216) | `if (zds->format == ZSTD_f_zstd1` | branch-specific rejection/error | [ ] |
| 1961 | `ZSTD_decompressStream` (src/decompress/zstd_decompress.c:2231) | `RETURN_ERROR_IF(zds->fParams.windowSize > zds->maxWindowSize,` | `ERROR(frameParameter_windowTooLarge)` | [ ] |
| 1962 | `ZSTD_decompressStream` (src/decompress/zstd_decompress.c:2233) | `if (zds->maxBlockSizeParam != 0)` | branch-specific rejection/error | [ ] |
| 1963 | `ZSTD_decompressStream` (src/decompress/zstd_decompress.c:2253) | `if (zds->staticSize) {  /* static DCtx */` | `ERROR(x)` | [ ] |
| 1964 | `ZSTD_decompressStream` (src/decompress/zstd_decompress.c:2255) | `assert(zds->staticSize >= sizeof(ZSTD_DCtx));  /* controlled at init */` | `ERROR(x)` | [ ] |
| 1965 | `ZSTD_decompressStream` (src/decompress/zstd_decompress.c:2256) | `RETURN_ERROR_IF(` | `ERROR(x)` | [ ] |
| 1966 | `ZSTD_decompressStream` (src/decompress/zstd_decompress.c:2264) | `RETURN_ERROR_IF(zds->inBuff == NULL, memory_allocation, "");` | `ERROR(memory_allocation)` | [ ] |
| 1967 | `ZSTD_decompressStream` (src/decompress/zstd_decompress.c:2277) | `if (neededInSize==0) {  /* end of frame */` | branch-specific rejection/error | [ ] |
| 1968 | `ZSTD_decompressStream` (src/decompress/zstd_decompress.c:2282) | `if ((size_t)(iend-ip) >= neededInSize) {  /* decode directly from src */` | branch-specific rejection/error | [ ] |
| 1969 | `ZSTD_decompressStream` (src/decompress/zstd_decompress.c:2284) | `assert(ip != NULL);` | assertion failure | [ ] |
| 1970 | `ZSTD_decompressStream` (src/decompress/zstd_decompress.c:2299) | `assert(neededInSize == ZSTD_nextSrcSizeToDecompressWithInputSize(zds, (size_t)(iend - ip)));` | `ERROR(s)` | [ ] |
| 1971 | `ZSTD_decompressStream` (src/decompress/zstd_decompress.c:2303) | `RETURN_ERROR_IF(toLoad > zds->inBuffSize - zds->inPos,` | `ERROR(corruption_detected)` | [ ] |
| 1972 | `ZSTD_decompressStream` (src/decompress/zstd_decompress.c:2308) | `if (loadedSize != 0) {` | branch-specific rejection/error | [ ] |
| 1973 | `ZSTD_decompressStream` (src/decompress/zstd_decompress.c:2313) | `if (loadedSize < toLoad) { someMoreWork = 0; break; }   /* not enough input, wait for more */` | branch-specific rejection/error | [ ] |
| 1974 | `ZSTD_decompressStream` (src/decompress/zstd_decompress.c:2331) | `if ( (zds->outBuffSize < zds->fParams.frameContentSize)` | branch-specific rejection/error | [ ] |
| 1975 | `ZSTD_decompressStream` (src/decompress/zstd_decompress.c:2345) | `assert(0);    /* impossible */` | `ERROR(C)` | [ ] |
| 1976 | `ZSTD_decompressStream` (src/decompress/zstd_decompress.c:2346) | `RETURN_ERROR(GENERIC, "impossible to reach");   /* some compilers require default to do something */` | `ERROR(C)` | [ ] |
| 1977 | `ZSTD_decompressStream` (src/decompress/zstd_decompress.c:2358) | `if (zds->noForwardProgress >= ZSTD_NO_FORWARD_PROGRESS_MAX) {` | `ERROR(noForwardProgress_destFull)` | [ ] |
| 1978 | `ZSTD_decompressStream` (src/decompress/zstd_decompress.c:2359) | `RETURN_ERROR_IF(op==oend, noForwardProgress_destFull, "");` | `ERROR(noForwardProgress_destFull)` | [ ] |
| 1979 | `ZSTD_decompressStream` (src/decompress/zstd_decompress.c:2360) | `RETURN_ERROR_IF(ip==iend, noForwardProgress_inputEmpty, "");` | `ERROR(noForwardProgress_inputEmpty)` | [ ] |
| 1980 | `ZSTD_decompressStream` (src/decompress/zstd_decompress.c:2361) | `assert(0);` | assertion failure | [ ] |
| 1981 | `ZSTD_decompressStream` (src/decompress/zstd_decompress.c:2368) | `if (zds->outEnd == zds->outStart) {  /* output fully flushed */` | branch-specific rejection/error | [ ] |
| 1982 | `ZSTD_decompressStream` (src/decompress/zstd_decompress.c:2369) | `if (zds->hostageByte) {` | branch-specific rejection/error | [ ] |
| 1983 | `ZSTD_decompressStream` (src/decompress/zstd_decompress.c:2370) | `if (input->pos >= input->size) {` | branch-specific rejection/error | [ ] |
| 1984 | `ZSTD_decompressStream` (src/decompress/zstd_decompress.c:2379) | `if (!zds->hostageByte) { /* output not fully flushed; keep last byte as hostage; will be released when all output is flushed */` | branch-specific rejection/error | [ ] |
| 1985 | `ZSTD_decompressStream` (src/decompress/zstd_decompress.c:2386) | `assert(zds->inPos <= nextSrcSizeHint);` | assertion failure | [ ] |
| 1986 | `ZSTD_blockSizeMax` (src/decompress/zstd_decompress_block.c:57) | `assert(blockSizeMax <= ZSTD_BLOCKSIZE_MAX);` | assertion failure | [ ] |
| 1987 | `ZSTD_getcBlockSize` (src/decompress/zstd_decompress_block.c:66) | `RETURN_ERROR_IF(srcSize < ZSTD_blockHeaderSize, srcSize_wrong, "");` | `ERROR(srcSize_wrong)` | [ ] |
| 1988 | `ZSTD_getcBlockSize` (src/decompress/zstd_decompress_block.c:73) | `if (bpPtr->blockType == bt_rle) return 1;` | `ERROR(corruption_detected)` | [ ] |
| 1989 | `ZSTD_getcBlockSize` (src/decompress/zstd_decompress_block.c:74) | `RETURN_ERROR_IF(bpPtr->blockType == bt_reserved, corruption_detected, "");` | `ERROR(corruption_detected)` | [ ] |
| 1990 | `ZSTD_allocateLiteralsBuffer` (src/decompress/zstd_decompress_block.c:84) | `assert(litSize <= blockSizeMax);` | assertion failure | [ ] |
| 1991 | `ZSTD_allocateLiteralsBuffer` (src/decompress/zstd_decompress_block.c:85) | `assert(dctx->isFrameDecompression \|\| streaming == not_streaming);` | assertion failure | [ ] |
| 1992 | `ZSTD_allocateLiteralsBuffer` (src/decompress/zstd_decompress_block.c:86) | `assert(expectedWriteSize <= blockSizeMax);` | assertion failure | [ ] |
| 1993 | `ZSTD_allocateLiteralsBuffer` (src/decompress/zstd_decompress_block.c:87) | `if (streaming == not_streaming && dstCapacity > blockSizeMax + WILDCOPY_OVERLENGTH + litSize + WILDCOPY_OVERLENGTH) {` | branch-specific rejection/error | [ ] |
| 1994 | `ZSTD_allocateLiteralsBuffer` (src/decompress/zstd_decompress_block.c:96) | `} else if (litSize <= ZSTD_LITBUFFEREXTRASIZE) {` | branch-specific rejection/error | [ ] |
| 1995 | `ZSTD_allocateLiteralsBuffer` (src/decompress/zstd_decompress_block.c:104) | `assert(blockSizeMax > ZSTD_LITBUFFEREXTRASIZE);` | assertion failure | [ ] |
| 1996 | `ZSTD_allocateLiteralsBuffer` (src/decompress/zstd_decompress_block.c:122) | `assert(dctx->litBufferEnd <= (BYTE*)dst + expectedWriteSize);` | assertion failure | [ ] |
| 1997 | `ZSTD_decodeLiteralsBlock` (src/decompress/zstd_decompress_block.c:139) | `RETURN_ERROR_IF(srcSize < MIN_CBLOCK_SIZE, corruption_detected, "");` | `ERROR(corruption_detected)` | [ ] |
| 1998 | `ZSTD_decodeLiteralsBlock` (src/decompress/zstd_decompress_block.c:149) | `RETURN_ERROR_IF(dctx->litEntropy==0, dictionary_corrupted, "");` | `ERROR(dictionary_corrupted)` | [ ] |
| 1999 | `ZSTD_decodeLiteralsBlock` (src/decompress/zstd_decompress_block.c:153) | `RETURN_ERROR_IF(srcSize < 5, corruption_detected, "srcSize >= MIN_CBLOCK_SIZE == 2; here we need up to 5 for case 3");` | `ERROR(corruption_detected)` | [ ] |
| 2000 | `ZSTD_decodeLiteralsBlock` (src/decompress/zstd_decompress_block.c:185) | `RETURN_ERROR_IF(litSize > 0 && dst == NULL, dstSize_tooSmall, "NULL not handled");` | `ERROR(dstSize_tooSmall)` | [ ] |
| 2001 | `ZSTD_decodeLiteralsBlock` (src/decompress/zstd_decompress_block.c:186) | `RETURN_ERROR_IF(litSize > blockSizeMax, corruption_detected, "");` | `ERROR(corruption_detected)` | [ ] |
| 2002 | `ZSTD_decodeLiteralsBlock` (src/decompress/zstd_decompress_block.c:188) | `RETURN_ERROR_IF(litSize < MIN_LITERALS_FOR_4_STREAMS, literals_headerWrong,` | `ERROR(literals_headerWrong)` | [ ] |
| 2003 | `ZSTD_decodeLiteralsBlock` (src/decompress/zstd_decompress_block.c:191) | `RETURN_ERROR_IF(litCSize + lhSize > srcSize, corruption_detected, "");` | `ERROR(corruption_detected)` | [ ] |
| 2004 | `ZSTD_decodeLiteralsBlock` (src/decompress/zstd_decompress_block.c:192) | `RETURN_ERROR_IF(expectedWriteSize < litSize , dstSize_tooSmall, "");` | `ERROR(dstSize_tooSmall)` | [ ] |
| 2005 | `ZSTD_decodeLiteralsBlock` (src/decompress/zstd_decompress_block.c:196) | `if (dctx->ddictIsCold && (litSize > 768 /* heuristic */)) {` | branch-specific rejection/error | [ ] |
| 2006 | `ZSTD_decodeLiteralsBlock` (src/decompress/zstd_decompress_block.c:206) | `assert(litSize >= MIN_LITERALS_FOR_4_STREAMS);` | assertion failure | [ ] |
| 2007 | `ZSTD_decodeLiteralsBlock` (src/decompress/zstd_decompress_block.c:231) | `if (dctx->litBufferLocation == ZSTD_split)` | branch-specific rejection/error | [ ] |
| 2008 | `ZSTD_decodeLiteralsBlock` (src/decompress/zstd_decompress_block.c:233) | `assert(litSize > ZSTD_LITBUFFEREXTRASIZE);` | assertion failure | [ ] |
| 2009 | `ZSTD_decodeLiteralsBlock` (src/decompress/zstd_decompress_block.c:238) | `assert(dctx->litBufferEnd <= (BYTE*)dst + blockSizeMax);` | `ERROR(s)` | [ ] |
| 2010 | `ZSTD_decodeLiteralsBlock` (src/decompress/zstd_decompress_block.c:241) | `RETURN_ERROR_IF(HUF_isError(hufSuccess), corruption_detected, "");` | `ERROR(s)` | [ ] |
| 2011 | `ZSTD_decodeLiteralsBlock` (src/decompress/zstd_decompress_block.c:246) | `if (litEncType==set_compressed) dctx->HUFptr = dctx->entropy.hufTable;` | branch-specific rejection/error | [ ] |
| 2012 | `ZSTD_decodeLiteralsBlock` (src/decompress/zstd_decompress_block.c:266) | `RETURN_ERROR_IF(srcSize<3, corruption_detected, "srcSize >= MIN_CBLOCK_SIZE == 2; here we need lhSize = 3");` | `ERROR(corruption_detected)` | [ ] |
| 2013 | `ZSTD_decodeLiteralsBlock` (src/decompress/zstd_decompress_block.c:271) | `RETURN_ERROR_IF(litSize > 0 && dst == NULL, dstSize_tooSmall, "NULL not handled");` | `ERROR(dstSize_tooSmall)` | [ ] |
| 2014 | `ZSTD_decodeLiteralsBlock` (src/decompress/zstd_decompress_block.c:272) | `RETURN_ERROR_IF(litSize > blockSizeMax, corruption_detected, "");` | `ERROR(corruption_detected)` | [ ] |
| 2015 | `ZSTD_decodeLiteralsBlock` (src/decompress/zstd_decompress_block.c:273) | `RETURN_ERROR_IF(expectedWriteSize < litSize, dstSize_tooSmall, "");` | `ERROR(dstSize_tooSmall)` | [ ] |
| 2016 | `ZSTD_decodeLiteralsBlock` (src/decompress/zstd_decompress_block.c:275) | `if (lhSize+litSize+WILDCOPY_OVERLENGTH > srcSize) {  /* risk reading beyond src buffer with wildcopy */` | `ERROR(corruption_detected)` | [ ] |
| 2017 | `ZSTD_decodeLiteralsBlock` (src/decompress/zstd_decompress_block.c:276) | `RETURN_ERROR_IF(litSize+lhSize > srcSize, corruption_detected, "");` | `ERROR(corruption_detected)` | [ ] |
| 2018 | `ZSTD_decodeLiteralsBlock` (src/decompress/zstd_decompress_block.c:277) | `if (dctx->litBufferLocation == ZSTD_split)` | branch-specific rejection/error | [ ] |
| 2019 | `ZSTD_decodeLiteralsBlock` (src/decompress/zstd_decompress_block.c:310) | `RETURN_ERROR_IF(srcSize<3, corruption_detected, "srcSize >= MIN_CBLOCK_SIZE == 2; here we need lhSize+1 = 3");` | `ERROR(corruption_detected)` | [ ] |
| 2020 | `ZSTD_decodeLiteralsBlock` (src/decompress/zstd_decompress_block.c:315) | `RETURN_ERROR_IF(srcSize<4, corruption_detected, "srcSize >= MIN_CBLOCK_SIZE == 2; here we need lhSize+1 = 4");` | `ERROR(corruption_detected)` | [ ] |
| 2021 | `ZSTD_decodeLiteralsBlock` (src/decompress/zstd_decompress_block.c:319) | `RETURN_ERROR_IF(litSize > 0 && dst == NULL, dstSize_tooSmall, "NULL not handled");` | `ERROR(dstSize_tooSmall)` | [ ] |
| 2022 | `ZSTD_decodeLiteralsBlock` (src/decompress/zstd_decompress_block.c:320) | `RETURN_ERROR_IF(litSize > blockSizeMax, corruption_detected, "");` | `ERROR(corruption_detected)` | [ ] |
| 2023 | `ZSTD_decodeLiteralsBlock` (src/decompress/zstd_decompress_block.c:321) | `RETURN_ERROR_IF(expectedWriteSize < litSize, dstSize_tooSmall, "");` | `ERROR(dstSize_tooSmall)` | [ ] |
| 2024 | `ZSTD_decodeLiteralsBlock` (src/decompress/zstd_decompress_block.c:323) | `if (dctx->litBufferLocation == ZSTD_split)` | branch-specific rejection/error | [ ] |
| 2025 | `ZSTD_decodeLiteralsBlock` (src/decompress/zstd_decompress_block.c:337) | `RETURN_ERROR(corruption_detected, "impossible");` | `ERROR(d)` | [ ] |
| 2026 | `ZSTD_buildSeqTable_rle` (src/decompress/zstd_decompress_block.c:474) | `assert(nbAddBits < 255);` | assertion failure | [ ] |
| 2027 | `ZSTD_buildFSETable_body` (src/decompress/zstd_decompress_block.c:500) | `assert(maxSymbolValue <= MaxSeq);` | assertion failure | [ ] |
| 2028 | `ZSTD_buildFSETable_body` (src/decompress/zstd_decompress_block.c:501) | `assert(tableLog <= MaxFSELog);` | assertion failure | [ ] |
| 2029 | `ZSTD_buildFSETable_body` (src/decompress/zstd_decompress_block.c:502) | `assert(wkspSize >= ZSTD_BUILD_FSE_TABLE_WKSP_SIZE);` | assertion failure | [ ] |
| 2030 | `ZSTD_buildFSETable_body` (src/decompress/zstd_decompress_block.c:515) | `if (normalizedCounter[s] >= largeLimit) DTableH.fastMode=0;` | branch-specific rejection/error | [ ] |
| 2031 | `ZSTD_buildFSETable_body` (src/decompress/zstd_decompress_block.c:516) | `assert(normalizedCounter[s]>=0);` | assertion failure | [ ] |
| 2032 | `ZSTD_buildFSETable_body` (src/decompress/zstd_decompress_block.c:523) | `assert(tableSize <= 512);` | assertion failure | [ ] |
| 2033 | `ZSTD_buildFSETable_body` (src/decompress/zstd_decompress_block.c:550) | `assert(n>=0);` | assertion failure | [ ] |
| 2034 | `ZSTD_buildFSETable_body` (src/decompress/zstd_decompress_block.c:564) | `assert(tableSize % unroll == 0); /* FSE_MIN_TABLELOG is 5 */` | assertion failure | [ ] |
| 2035 | `ZSTD_buildFSETable_body` (src/decompress/zstd_decompress_block.c:573) | `assert(position == 0);` | assertion failure | [ ] |
| 2036 | `ZSTD_buildFSETable_body` (src/decompress/zstd_decompress_block.c:587) | `assert(position == 0); /* position must reach all cells once, otherwise normalizedCounter is incorrect */` | assertion failure | [ ] |
| 2037 | `ZSTD_buildFSETable_body` (src/decompress/zstd_decompress_block.c:598) | `assert(nbAdditionalBits[symbol] < 255);` | assertion failure | [ ] |
| 2038 | `ZSTD_buildSeqTable` (src/decompress/zstd_decompress_block.c:658) | `RETURN_ERROR_IF(!srcSize, srcSize_wrong, "");` | `ERROR(srcSize_wrong)` | [ ] |
| 2039 | `ZSTD_buildSeqTable` (src/decompress/zstd_decompress_block.c:659) | `RETURN_ERROR_IF((*(const BYTE*)src) > max, corruption_detected, "");` | `ERROR(E)` | [ ] |
| 2040 | `ZSTD_buildSeqTable` (src/decompress/zstd_decompress_block.c:671) | `RETURN_ERROR_IF(!flagRepeatTable, corruption_detected, "");` | `ERROR(corruption_detected)` | [ ] |
| 2041 | `ZSTD_buildSeqTable` (src/decompress/zstd_decompress_block.c:673) | `if (ddictIsCold && (nbSeq > 24 /* heuristic */)) {` | branch-specific rejection/error | [ ] |
| 2042 | `ZSTD_buildSeqTable` (src/decompress/zstd_decompress_block.c:683) | `RETURN_ERROR_IF(FSE_isError(headerSize), corruption_detected, "");` | `ERROR(e)` | [ ] |
| 2043 | `ZSTD_buildSeqTable` (src/decompress/zstd_decompress_block.c:684) | `RETURN_ERROR_IF(tableLog > maxLog, corruption_detected, "");` | `ERROR(corruption_detected)` | [ ] |
| 2044 | `ZSTD_buildSeqTable` (src/decompress/zstd_decompress_block.c:690) | `assert(0);` | `ERROR(C)` | [ ] |
| 2045 | `ZSTD_buildSeqTable` (src/decompress/zstd_decompress_block.c:691) | `RETURN_ERROR(GENERIC, "impossible");` | `ERROR(C)` | [ ] |
| 2046 | `ZSTD_decodeSeqHeaders` (src/decompress/zstd_decompress_block.c:705) | `RETURN_ERROR_IF(srcSize < MIN_SEQUENCES_SIZE, srcSize_wrong, "");` | `ERROR(srcSize_wrong)` | [ ] |
| 2047 | `ZSTD_decodeSeqHeaders` (src/decompress/zstd_decompress_block.c:709) | `if (nbSeq > 0x7F) {` | `ERROR(srcSize_wrong)` | [ ] |
| 2048 | `ZSTD_decodeSeqHeaders` (src/decompress/zstd_decompress_block.c:710) | `if (nbSeq == 0xFF) {` | `ERROR(srcSize_wrong)` | [ ] |
| 2049 | `ZSTD_decodeSeqHeaders` (src/decompress/zstd_decompress_block.c:711) | `RETURN_ERROR_IF(ip+2 > iend, srcSize_wrong, "");` | `ERROR(srcSize_wrong)` | [ ] |
| 2050 | `ZSTD_decodeSeqHeaders` (src/decompress/zstd_decompress_block.c:715) | `RETURN_ERROR_IF(ip >= iend, srcSize_wrong, "");` | `ERROR(srcSize_wrong)` | [ ] |
| 2051 | `ZSTD_decodeSeqHeaders` (src/decompress/zstd_decompress_block.c:721) | `if (nbSeq == 0) {` | `ERROR(corruption_detected)` | [ ] |
| 2052 | `ZSTD_decodeSeqHeaders` (src/decompress/zstd_decompress_block.c:723) | `RETURN_ERROR_IF(ip != iend, corruption_detected,` | `ERROR(corruption_detected)` | [ ] |
| 2053 | `ZSTD_decodeSeqHeaders` (src/decompress/zstd_decompress_block.c:729) | `RETURN_ERROR_IF(ip+1 > iend, srcSize_wrong, ""); /* minimum possible size: 1 byte for symbol encoding types */` | `ERROR(srcSize_wrong)` | [ ] |
| 2054 | `ZSTD_decodeSeqHeaders` (src/decompress/zstd_decompress_block.c:730) | `RETURN_ERROR_IF(*ip & 3, corruption_detected, ""); /* The last field, Reserved, must be all-zeroes. */` | `ERROR(corruption_detected)` | [ ] |
| 2055 | `ZSTD_decodeSeqHeaders` (src/decompress/zstd_decompress_block.c:745) | `RETURN_ERROR_IF(ZSTD_isError(llhSize), corruption_detected, "ZSTD_buildSeqTable failed");` | `ERROR(e)` | [ ] |
| 2056 | `ZSTD_decodeSeqHeaders` (src/decompress/zstd_decompress_block.c:757) | `RETURN_ERROR_IF(ZSTD_isError(ofhSize), corruption_detected, "ZSTD_buildSeqTable failed");` | `ERROR(e)` | [ ] |
| 2057 | `ZSTD_decodeSeqHeaders` (src/decompress/zstd_decompress_block.c:769) | `RETURN_ERROR_IF(ZSTD_isError(mlhSize), corruption_detected, "ZSTD_buildSeqTable failed");` | `ERROR(e)` | [ ] |
| 2058 | `ZSTD_overlapCopy8` (src/decompress/zstd_decompress_block.c:805) | `assert(*ip <= *op);` | assertion failure | [ ] |
| 2059 | `ZSTD_overlapCopy8` (src/decompress/zstd_decompress_block.c:806) | `if (offset < 8) {` | branch-specific rejection/error | [ ] |
| 2060 | `ZSTD_overlapCopy8` (src/decompress/zstd_decompress_block.c:823) | `assert(*op - *ip >= 8);` | assertion failure | [ ] |
| 2061 | `ZSTD_safecopy` (src/decompress/zstd_decompress_block.c:841) | `assert((ovtype == ZSTD_no_overlap && (diff <= -8 \|\| diff >= 8 \|\| op >= oend_w)) \|\|` | assertion failure | [ ] |
| 2062 | `ZSTD_safecopy` (src/decompress/zstd_decompress_block.c:844) | `if (length < 8) {` | branch-specific rejection/error | [ ] |
| 2063 | `ZSTD_safecopy` (src/decompress/zstd_decompress_block.c:851) | `assert(length >= 8);` | assertion failure | [ ] |
| 2064 | `ZSTD_safecopy` (src/decompress/zstd_decompress_block.c:854) | `assert(op - ip >= 8);` | assertion failure | [ ] |
| 2065 | `ZSTD_safecopy` (src/decompress/zstd_decompress_block.c:855) | `assert(op <= oend);` | assertion failure | [ ] |
| 2066 | `ZSTD_safecopy` (src/decompress/zstd_decompress_block.c:858) | `if (oend <= oend_w) {` | branch-specific rejection/error | [ ] |
| 2067 | `ZSTD_safecopy` (src/decompress/zstd_decompress_block.c:863) | `if (op <= oend_w) {` | branch-specific rejection/error | [ ] |
| 2068 | `ZSTD_safecopy` (src/decompress/zstd_decompress_block.c:865) | `assert(oend > oend_w);` | assertion failure | [ ] |
| 2069 | `ZSTD_safecopyDstBeforeSrc` (src/decompress/zstd_decompress_block.c:881) | `if (length < 8 \|\| diff > -8) {` | branch-specific rejection/error | [ ] |
| 2070 | `ZSTD_safecopyDstBeforeSrc` (src/decompress/zstd_decompress_block.c:887) | `if (op <= oend - WILDCOPY_OVERLENGTH && diff < -WILDCOPY_VECLEN) {` | branch-specific rejection/error | [ ] |
| 2071 | `ZSTD_execSequenceEnd` (src/decompress/zstd_decompress_block.c:919) | `RETURN_ERROR_IF(sequenceLength > (size_t)(oend - op), dstSize_tooSmall, "last match must fit within dstBuffer");` | `ERROR(t)` | [ ] |
| 2072 | `ZSTD_execSequenceEnd` (src/decompress/zstd_decompress_block.c:920) | `RETURN_ERROR_IF(sequence.litLength > (size_t)(litLimit - *litPtr), corruption_detected, "try to read beyond literal buffer");` | `ERROR(t)` | [ ] |
| 2073 | `ZSTD_execSequenceEnd` (src/decompress/zstd_decompress_block.c:921) | `assert(op < op + sequenceLength);` | assertion failure | [ ] |
| 2074 | `ZSTD_execSequenceEnd` (src/decompress/zstd_decompress_block.c:922) | `assert(oLitEnd < op + sequenceLength);` | assertion failure | [ ] |
| 2075 | `ZSTD_execSequenceEnd` (src/decompress/zstd_decompress_block.c:930) | `if (sequence.offset > (size_t)(oLitEnd - prefixStart)) {` | `ERROR(t)` | [ ] |
| 2076 | `ZSTD_execSequenceEnd` (src/decompress/zstd_decompress_block.c:932) | `RETURN_ERROR_IF(sequence.offset > (size_t)(oLitEnd - virtualStart), corruption_detected, "");` | `ERROR(t)` | [ ] |
| 2077 | `ZSTD_execSequenceEnd` (src/decompress/zstd_decompress_block.c:934) | `if (match + sequence.matchLength <= dictEnd) {` | branch-specific rejection/error | [ ] |
| 2078 | `ZSTD_execSequenceEndSplitLitBuffer` (src/decompress/zstd_decompress_block.c:967) | `RETURN_ERROR_IF(sequenceLength > (size_t)(oend - op), dstSize_tooSmall, "last match must fit within dstBuffer");` | `ERROR(t)` | [ ] |
| 2079 | `ZSTD_execSequenceEndSplitLitBuffer` (src/decompress/zstd_decompress_block.c:968) | `RETURN_ERROR_IF(sequence.litLength > (size_t)(litLimit - *litPtr), corruption_detected, "try to read beyond literal buffer");` | `ERROR(t)` | [ ] |
| 2080 | `ZSTD_execSequenceEndSplitLitBuffer` (src/decompress/zstd_decompress_block.c:969) | `assert(op < op + sequenceLength);` | `ERROR(dstSize_tooSmall)` | [ ] |
| 2081 | `ZSTD_execSequenceEndSplitLitBuffer` (src/decompress/zstd_decompress_block.c:970) | `assert(oLitEnd < op + sequenceLength);` | `ERROR(dstSize_tooSmall)` | [ ] |
| 2082 | `ZSTD_execSequenceEndSplitLitBuffer` (src/decompress/zstd_decompress_block.c:973) | `RETURN_ERROR_IF(op > *litPtr && op < *litPtr + sequence.litLength, dstSize_tooSmall, "output should not catch up to and overwrite literal buffer");` | `ERROR(dstSize_tooSmall)` | [ ] |
| 2083 | `ZSTD_execSequenceEndSplitLitBuffer` (src/decompress/zstd_decompress_block.c:979) | `if (sequence.offset > (size_t)(oLitEnd - prefixStart)) {` | `ERROR(t)` | [ ] |
| 2084 | `ZSTD_execSequenceEndSplitLitBuffer` (src/decompress/zstd_decompress_block.c:981) | `RETURN_ERROR_IF(sequence.offset > (size_t)(oLitEnd - virtualStart), corruption_detected, "");` | `ERROR(t)` | [ ] |
| 2085 | `ZSTD_execSequenceEndSplitLitBuffer` (src/decompress/zstd_decompress_block.c:983) | `if (match + sequence.matchLength <= dictEnd) {` | branch-specific rejection/error | [ ] |
| 2086 | `ZSTD_execSequence` (src/decompress/zstd_decompress_block.c:1013) | `assert(op != NULL /* Precondition */);` | assertion failure | [ ] |
| 2087 | `ZSTD_execSequence` (src/decompress/zstd_decompress_block.c:1014) | `assert(oend_w < oend /* No underflow */);` | assertion failure | [ ] |
| 2088 | `ZSTD_execSequence` (src/decompress/zstd_decompress_block.c:1032) | `assert(op <= oLitEnd /* No overflow */);` | assertion failure | [ ] |
| 2089 | `ZSTD_execSequence` (src/decompress/zstd_decompress_block.c:1033) | `assert(oLitEnd < oMatchEnd /* Non-zero match & no overflow */);` | assertion failure | [ ] |
| 2090 | `ZSTD_execSequence` (src/decompress/zstd_decompress_block.c:1034) | `assert(oMatchEnd <= oend /* No underflow */);` | assertion failure | [ ] |
| 2091 | `ZSTD_execSequence` (src/decompress/zstd_decompress_block.c:1035) | `assert(iLitEnd <= litLimit /* Literal length is in bounds */);` | assertion failure | [ ] |
| 2092 | `ZSTD_execSequence` (src/decompress/zstd_decompress_block.c:1036) | `assert(oLitEnd <= oend_w /* Can wildcopy literals */);` | assertion failure | [ ] |
| 2093 | `ZSTD_execSequence` (src/decompress/zstd_decompress_block.c:1037) | `assert(oMatchEnd <= oend_w /* Can wildcopy matches */);` | assertion failure | [ ] |
| 2094 | `ZSTD_execSequence` (src/decompress/zstd_decompress_block.c:1043) | `assert(WILDCOPY_OVERLENGTH >= 16);` | assertion failure | [ ] |
| 2095 | `ZSTD_execSequence` (src/decompress/zstd_decompress_block.c:1045) | `if (UNLIKELY(sequence.litLength > 16)) {` | branch-specific rejection/error | [ ] |
| 2096 | `ZSTD_execSequence` (src/decompress/zstd_decompress_block.c:1052) | `if (sequence.offset > (size_t)(oLitEnd - prefixStart)) {` | `ERROR(t)` | [ ] |
| 2097 | `ZSTD_execSequence` (src/decompress/zstd_decompress_block.c:1054) | `RETURN_ERROR_IF(UNLIKELY(sequence.offset > (size_t)(oLitEnd - virtualStart)), corruption_detected, "");` | `ERROR(t)` | [ ] |
| 2098 | `ZSTD_execSequence` (src/decompress/zstd_decompress_block.c:1056) | `if (match + sequence.matchLength <= dictEnd) {` | branch-specific rejection/error | [ ] |
| 2099 | `ZSTD_execSequence` (src/decompress/zstd_decompress_block.c:1069) | `assert(op <= oMatchEnd);` | assertion failure | [ ] |
| 2100 | `ZSTD_execSequence` (src/decompress/zstd_decompress_block.c:1070) | `assert(oMatchEnd <= oend_w);` | assertion failure | [ ] |
| 2101 | `ZSTD_execSequence` (src/decompress/zstd_decompress_block.c:1071) | `assert(match >= prefixStart);` | assertion failure | [ ] |
| 2102 | `ZSTD_execSequence` (src/decompress/zstd_decompress_block.c:1072) | `assert(sequence.matchLength >= 1);` | assertion failure | [ ] |
| 2103 | `ZSTD_execSequence` (src/decompress/zstd_decompress_block.c:1077) | `if (LIKELY(sequence.offset >= WILDCOPY_VECLEN)) {` | branch-specific rejection/error | [ ] |
| 2104 | `ZSTD_execSequence` (src/decompress/zstd_decompress_block.c:1085) | `assert(sequence.offset < WILDCOPY_VECLEN);` | assertion failure | [ ] |
| 2105 | `ZSTD_execSequence` (src/decompress/zstd_decompress_block.c:1091) | `if (sequence.matchLength > 8) {` | branch-specific rejection/error | [ ] |
| 2106 | `ZSTD_execSequence` (src/decompress/zstd_decompress_block.c:1092) | `assert(op < oMatchEnd);` | assertion failure | [ ] |
| 2107 | `ZSTD_execSequenceSplitLitBuffer` (src/decompress/zstd_decompress_block.c:1111) | `assert(op != NULL /* Precondition */);` | assertion failure | [ ] |
| 2108 | `ZSTD_execSequenceSplitLitBuffer` (src/decompress/zstd_decompress_block.c:1112) | `assert(oend_w < oend /* No underflow */);` | assertion failure | [ ] |
| 2109 | `ZSTD_execSequenceSplitLitBuffer` (src/decompress/zstd_decompress_block.c:1125) | `assert(op <= oLitEnd /* No overflow */);` | assertion failure | [ ] |
| 2110 | `ZSTD_execSequenceSplitLitBuffer` (src/decompress/zstd_decompress_block.c:1126) | `assert(oLitEnd < oMatchEnd /* Non-zero match & no overflow */);` | assertion failure | [ ] |
| 2111 | `ZSTD_execSequenceSplitLitBuffer` (src/decompress/zstd_decompress_block.c:1127) | `assert(oMatchEnd <= oend /* No underflow */);` | assertion failure | [ ] |
| 2112 | `ZSTD_execSequenceSplitLitBuffer` (src/decompress/zstd_decompress_block.c:1128) | `assert(iLitEnd <= litLimit /* Literal length is in bounds */);` | assertion failure | [ ] |
| 2113 | `ZSTD_execSequenceSplitLitBuffer` (src/decompress/zstd_decompress_block.c:1129) | `assert(oLitEnd <= oend_w /* Can wildcopy literals */);` | assertion failure | [ ] |
| 2114 | `ZSTD_execSequenceSplitLitBuffer` (src/decompress/zstd_decompress_block.c:1130) | `assert(oMatchEnd <= oend_w /* Can wildcopy matches */);` | assertion failure | [ ] |
| 2115 | `ZSTD_execSequenceSplitLitBuffer` (src/decompress/zstd_decompress_block.c:1136) | `assert(WILDCOPY_OVERLENGTH >= 16);` | assertion failure | [ ] |
| 2116 | `ZSTD_execSequenceSplitLitBuffer` (src/decompress/zstd_decompress_block.c:1138) | `if (UNLIKELY(sequence.litLength > 16)) {` | branch-specific rejection/error | [ ] |
| 2117 | `ZSTD_execSequenceSplitLitBuffer` (src/decompress/zstd_decompress_block.c:1145) | `if (sequence.offset > (size_t)(oLitEnd - prefixStart)) {` | `ERROR(t)` | [ ] |
| 2118 | `ZSTD_execSequenceSplitLitBuffer` (src/decompress/zstd_decompress_block.c:1147) | `RETURN_ERROR_IF(UNLIKELY(sequence.offset > (size_t)(oLitEnd - virtualStart)), corruption_detected, "");` | `ERROR(t)` | [ ] |
| 2119 | `ZSTD_execSequenceSplitLitBuffer` (src/decompress/zstd_decompress_block.c:1149) | `if (match + sequence.matchLength <= dictEnd) {` | branch-specific rejection/error | [ ] |
| 2120 | `ZSTD_execSequenceSplitLitBuffer` (src/decompress/zstd_decompress_block.c:1161) | `assert(op <= oMatchEnd);` | assertion failure | [ ] |
| 2121 | `ZSTD_execSequenceSplitLitBuffer` (src/decompress/zstd_decompress_block.c:1162) | `assert(oMatchEnd <= oend_w);` | assertion failure | [ ] |
| 2122 | `ZSTD_execSequenceSplitLitBuffer` (src/decompress/zstd_decompress_block.c:1163) | `assert(match >= prefixStart);` | assertion failure | [ ] |
| 2123 | `ZSTD_execSequenceSplitLitBuffer` (src/decompress/zstd_decompress_block.c:1164) | `assert(sequence.matchLength >= 1);` | assertion failure | [ ] |
| 2124 | `ZSTD_execSequenceSplitLitBuffer` (src/decompress/zstd_decompress_block.c:1169) | `if (LIKELY(sequence.offset >= WILDCOPY_VECLEN)) {` | branch-specific rejection/error | [ ] |
| 2125 | `ZSTD_execSequenceSplitLitBuffer` (src/decompress/zstd_decompress_block.c:1177) | `assert(sequence.offset < WILDCOPY_VECLEN);` | assertion failure | [ ] |
| 2126 | `ZSTD_execSequenceSplitLitBuffer` (src/decompress/zstd_decompress_block.c:1183) | `if (sequence.matchLength > 8) {` | branch-specific rejection/error | [ ] |
| 2127 | `ZSTD_execSequenceSplitLitBuffer` (src/decompress/zstd_decompress_block.c:1184) | `assert(op < oMatchEnd);` | assertion failure | [ ] |
| 2128 | `ZSTD_decodeSequence` (src/decompress/zstd_decompress_block.c:1268) | `assert(llBits <= MaxLLBits);` | assertion failure | [ ] |
| 2129 | `ZSTD_decodeSequence` (src/decompress/zstd_decompress_block.c:1269) | `assert(mlBits <= MaxMLBits);` | assertion failure | [ ] |
| 2130 | `ZSTD_decodeSequence` (src/decompress/zstd_decompress_block.c:1270) | `assert(ofBits <= MaxOff);` | assertion failure | [ ] |
| 2131 | `ZSTD_decodeSequence` (src/decompress/zstd_decompress_block.c:1279) | `if (ofBits > 1) {` | branch-specific rejection/error | [ ] |
| 2132 | `ZSTD_decodeSequence` (src/decompress/zstd_decompress_block.c:1284) | `if (MEM_32bits() && longOffsets && (ofBits >= STREAM_ACCUMULATOR_MIN_32)) {` | branch-specific rejection/error | [ ] |
| 2133 | `ZSTD_decodeSequence` (src/decompress/zstd_decompress_block.c:1294) | `if (MEM_32bits()) BIT_reloadDStream(&seqState->DStream);` | branch-specific rejection/error | [ ] |
| 2134 | `ZSTD_decodeSequence` (src/decompress/zstd_decompress_block.c:1301) | `if (LIKELY((ofBits == 0))) {` | branch-specific rejection/error | [ ] |
| 2135 | `ZSTD_decodeSequence` (src/decompress/zstd_decompress_block.c:1309) | `if (offset != 1) seqState->prevOffset[2] = seqState->prevOffset[1];` | branch-specific rejection/error | [ ] |
| 2136 | `ZSTD_decodeSequence` (src/decompress/zstd_decompress_block.c:1316) | `if (mlBits > 0)` | branch-specific rejection/error | [ ] |
| 2137 | `ZSTD_decodeSequence` (src/decompress/zstd_decompress_block.c:1319) | `if (MEM_32bits() && (mlBits+llBits >= STREAM_ACCUMULATOR_MIN_32-LONG_OFFSETS_MAX_EXTRA_BITS_32))` | branch-specific rejection/error | [ ] |
| 2138 | `ZSTD_decodeSequence` (src/decompress/zstd_decompress_block.c:1321) | `if (MEM_64bits() && UNLIKELY(totalBits >= STREAM_ACCUMULATOR_MIN_64-(LLFSELog+MLFSELog+OffFSELog)))` | branch-specific rejection/error | [ ] |
| 2139 | `ZSTD_decodeSequence` (src/decompress/zstd_decompress_block.c:1326) | `if (llBits > 0)` | branch-specific rejection/error | [ ] |
| 2140 | `ZSTD_decodeSequence` (src/decompress/zstd_decompress_block.c:1339) | `if (MEM_32bits()) BIT_reloadDStream(&seqState->DStream);    /* <= 18 bits */` | branch-specific rejection/error | [ ] |
| 2141 | `ZSTD_dictionaryIsActive` (src/decompress/zstd_decompress_block.c:1354) | `if (dctx->dictContentEndForFuzzing == NULL) return 0;` | branch-specific rejection/error | [ ] |
| 2142 | `ZSTD_dictionaryIsActive` (src/decompress/zstd_decompress_block.c:1356) | `if (prefixStart == dctx->dictContentBeginForFuzzing) return 1;` | branch-specific rejection/error | [ ] |
| 2143 | `ZSTD_dictionaryIsActive` (src/decompress/zstd_decompress_block.c:1358) | `if (dctx->dictEnd != dctx->dictContentEndForFuzzing) return 0;` | branch-specific rejection/error | [ ] |
| 2144 | `ZSTD_dictionaryIsActive` (src/decompress/zstd_decompress_block.c:1360) | `if ((size_t)(oLitEnd - prefixStart) >= windowSize) return 0;` | branch-specific rejection/error | [ ] |
| 2145 | `ZSTD_assertValidSequence` (src/decompress/zstd_decompress_block.c:1373) | `if (dctx->isFrameDecompression) {` | branch-specific rejection/error | [ ] |
| 2146 | `ZSTD_assertValidSequence` (src/decompress/zstd_decompress_block.c:1379) | `assert(op <= oend);` | assertion failure | [ ] |
| 2147 | `ZSTD_assertValidSequence` (src/decompress/zstd_decompress_block.c:1380) | `assert((size_t)(oend - op) >= sequenceSize);` | assertion failure | [ ] |
| 2148 | `ZSTD_assertValidSequence` (src/decompress/zstd_decompress_block.c:1381) | `assert(sequenceSize <= ZSTD_blockSizeMax(dctx));` | assertion failure | [ ] |
| 2149 | `ZSTD_assertValidSequence` (src/decompress/zstd_decompress_block.c:1385) | `assert(seq.offset <= (size_t)(oLitEnd - virtualStart));` | assertion failure | [ ] |
| 2150 | `ZSTD_assertValidSequence` (src/decompress/zstd_decompress_block.c:1386) | `assert(seq.offset <= windowSize + dictSize);` | assertion failure | [ ] |
| 2151 | `ZSTD_assertValidSequence` (src/decompress/zstd_decompress_block.c:1389) | `assert(seq.offset <= windowSize);` | assertion failure | [ ] |
| 2152 | `ZSTD_decompressSequences_bodySplitLitBuffer` (src/decompress/zstd_decompress_block.c:1425) | `RETURN_ERROR_IF(` | `ERROR(ip)` | [ ] |
| 2153 | `ZSTD_decompressSequences_bodySplitLitBuffer` (src/decompress/zstd_decompress_block.c:1431) | `assert(dst != NULL);` | assertion failure | [ ] |
| 2154 | `ZSTD_decompressSequences_bodySplitLitBuffer` (src/decompress/zstd_decompress_block.c:1503) | `if (litPtr + sequence.litLength > dctx->litBufferEnd) break;` | branch-specific rejection/error | [ ] |
| 2155 | `ZSTD_decompressSequences_bodySplitLitBuffer` (src/decompress/zstd_decompress_block.c:1506) | `assert(!ZSTD_isError(oneSeqSize));` | assertion failure | [ ] |
| 2156 | `ZSTD_decompressSequences_bodySplitLitBuffer` (src/decompress/zstd_decompress_block.c:1517) | `if (nbSeq > 0) {` | `ERROR(t)` | [ ] |
| 2157 | `ZSTD_decompressSequences_bodySplitLitBuffer` (src/decompress/zstd_decompress_block.c:1521) | `RETURN_ERROR_IF(leftoverLit > (size_t)(oend - op), dstSize_tooSmall, "remaining lit must fit within dstBuffer");` | `ERROR(t)` | [ ] |
| 2158 | `ZSTD_decompressSequences_bodySplitLitBuffer` (src/decompress/zstd_decompress_block.c:1531) | `assert(!ZSTD_isError(oneSeqSize));` | assertion failure | [ ] |
| 2159 | `ZSTD_decompressSequences_bodySplitLitBuffer` (src/decompress/zstd_decompress_block.c:1543) | `if (nbSeq > 0) {` | branch-specific rejection/error | [ ] |
| 2160 | `ZSTD_decompressSequences_bodySplitLitBuffer` (src/decompress/zstd_decompress_block.c:1567) | `assert(!ZSTD_isError(oneSeqSize));` | assertion failure | [ ] |
| 2161 | `ZSTD_decompressSequences_bodySplitLitBuffer` (src/decompress/zstd_decompress_block.c:1579) | `RETURN_ERROR_IF(nbSeq, corruption_detected, "");` | `ERROR(corruption_detected)` | [ ] |
| 2162 | `ZSTD_decompressSequences_bodySplitLitBuffer` (src/decompress/zstd_decompress_block.c:1581) | `RETURN_ERROR_IF(!BIT_endOfDStream(&seqState.DStream), corruption_detected, "");` | `ERROR(m)` | [ ] |
| 2163 | `ZSTD_decompressSequences_bodySplitLitBuffer` (src/decompress/zstd_decompress_block.c:1587) | `if (dctx->litBufferLocation == ZSTD_split) {` | `ERROR(t)` | [ ] |
| 2164 | `ZSTD_decompressSequences_bodySplitLitBuffer` (src/decompress/zstd_decompress_block.c:1591) | `RETURN_ERROR_IF(lastLLSize > (size_t)(oend - op), dstSize_tooSmall, "");` | `ERROR(t)` | [ ] |
| 2165 | `ZSTD_decompressSequences_bodySplitLitBuffer` (src/decompress/zstd_decompress_block.c:1592) | `if (op != NULL) {` | branch-specific rejection/error | [ ] |
| 2166 | `ZSTD_decompressSequences_bodySplitLitBuffer` (src/decompress/zstd_decompress_block.c:1603) | `RETURN_ERROR_IF(lastLLSize > (size_t)(oend-op), dstSize_tooSmall, "");` | `ERROR(t)` | [ ] |
| 2167 | `ZSTD_decompressSequences_bodySplitLitBuffer` (src/decompress/zstd_decompress_block.c:1604) | `if (op != NULL) {` | branch-specific rejection/error | [ ] |
| 2168 | `ZSTD_decompressSequences_body` (src/decompress/zstd_decompress_block.c:1637) | `RETURN_ERROR_IF(` | `ERROR(ip)` | [ ] |
| 2169 | `ZSTD_decompressSequences_body` (src/decompress/zstd_decompress_block.c:1643) | `assert(dst != NULL);` | assertion failure | [ ] |
| 2170 | `ZSTD_decompressSequences_body` (src/decompress/zstd_decompress_block.c:1663) | `assert(!ZSTD_isError(oneSeqSize));` | assertion failure | [ ] |
| 2171 | `ZSTD_decompressSequences_body` (src/decompress/zstd_decompress_block.c:1673) | `assert(nbSeq == 0);` | `ERROR(m)` | [ ] |
| 2172 | `ZSTD_decompressSequences_body` (src/decompress/zstd_decompress_block.c:1674) | `RETURN_ERROR_IF(!BIT_endOfDStream(&seqState.DStream), corruption_detected, "");` | `ERROR(m)` | [ ] |
| 2173 | `ZSTD_decompressSequences_body` (src/decompress/zstd_decompress_block.c:1682) | `RETURN_ERROR_IF(lastLLSize > (size_t)(oend-op), dstSize_tooSmall, "");` | `ERROR(t)` | [ ] |
| 2174 | `ZSTD_decompressSequences_body` (src/decompress/zstd_decompress_block.c:1683) | `if (op != NULL) {` | branch-specific rejection/error | [ ] |
| 2175 | `ZSTD_decompressSequencesLong_body` (src/decompress/zstd_decompress_block.c:1763) | `assert(dst != NULL);` | `ERROR(ip)` | [ ] |
| 2176 | `ZSTD_decompressSequencesLong_body` (src/decompress/zstd_decompress_block.c:1764) | `assert(iend >= ip);` | `ERROR(ip)` | [ ] |
| 2177 | `ZSTD_decompressSequencesLong_body` (src/decompress/zstd_decompress_block.c:1765) | `RETURN_ERROR_IF(` | `ERROR(ip)` | [ ] |
| 2178 | `ZSTD_decompressSequencesLong_body` (src/decompress/zstd_decompress_block.c:1783) | `if (dctx->litBufferLocation == ZSTD_split && litPtr + sequences[(seqNb - ADVANCED_SEQS) & STORED_SEQS_MASK].litLength > dctx->litBufferEnd) {` | branch-specific rejection/error | [ ] |
| 2179 | `ZSTD_decompressSequencesLong_body` (src/decompress/zstd_decompress_block.c:1788) | `RETURN_ERROR_IF(leftoverLit > (size_t)(oend - op), dstSize_tooSmall, "remaining lit must fit within dstBuffer");` | `ERROR(t)` | [ ] |
| 2180 | `ZSTD_decompressSequencesLong_body` (src/decompress/zstd_decompress_block.c:1798) | `assert(!ZSTD_isError(oneSeqSize));` | assertion failure | [ ] |
| 2181 | `ZSTD_decompressSequencesLong_body` (src/decompress/zstd_decompress_block.c:1814) | `assert(!ZSTD_isError(oneSeqSize));` | assertion failure | [ ] |
| 2182 | `ZSTD_decompressSequencesLong_body` (src/decompress/zstd_decompress_block.c:1824) | `RETURN_ERROR_IF(!BIT_endOfDStream(&seqState.DStream), corruption_detected, "");` | `ERROR(m)` | [ ] |
| 2183 | `ZSTD_decompressSequencesLong_body` (src/decompress/zstd_decompress_block.c:1830) | `if (dctx->litBufferLocation == ZSTD_split && litPtr + sequence->litLength > dctx->litBufferEnd) {` | `ERROR(t)` | [ ] |
| 2184 | `ZSTD_decompressSequencesLong_body` (src/decompress/zstd_decompress_block.c:1833) | `RETURN_ERROR_IF(leftoverLit > (size_t)(oend - op), dstSize_tooSmall, "remaining lit must fit within dstBuffer");` | `ERROR(t)` | [ ] |
| 2185 | `ZSTD_decompressSequencesLong_body` (src/decompress/zstd_decompress_block.c:1843) | `assert(!ZSTD_isError(oneSeqSize));` | assertion failure | [ ] |
| 2186 | `ZSTD_decompressSequencesLong_body` (src/decompress/zstd_decompress_block.c:1856) | `assert(!ZSTD_isError(oneSeqSize));` | assertion failure | [ ] |
| 2187 | `ZSTD_decompressSequencesLong_body` (src/decompress/zstd_decompress_block.c:1869) | `if (dctx->litBufferLocation == ZSTD_split) { /* first deplete literal buffer in dst, then copy litExtraBuffer */` | `ERROR(t)` | [ ] |
| 2188 | `ZSTD_decompressSequencesLong_body` (src/decompress/zstd_decompress_block.c:1871) | `RETURN_ERROR_IF(lastLLSize > (size_t)(oend - op), dstSize_tooSmall, "");` | `ERROR(t)` | [ ] |
| 2189 | `ZSTD_decompressSequencesLong_body` (src/decompress/zstd_decompress_block.c:1872) | `if (op != NULL) {` | branch-specific rejection/error | [ ] |
| 2190 | `ZSTD_decompressSequencesLong_body` (src/decompress/zstd_decompress_block.c:1880) | `RETURN_ERROR_IF(lastLLSize > (size_t)(oend-op), dstSize_tooSmall, "");` | `ERROR(t)` | [ ] |
| 2191 | `ZSTD_decompressSequencesLong_body` (src/decompress/zstd_decompress_block.c:1881) | `if (op != NULL) {` | branch-specific rejection/error | [ ] |
| 2192 | `ZSTD_getOffsetInfo` (src/decompress/zstd_decompress_block.c:2019) | `if (nbSeq != 0) {` | branch-specific rejection/error | [ ] |
| 2193 | `ZSTD_getOffsetInfo` (src/decompress/zstd_decompress_block.c:2027) | `assert(max <= (1 << OffFSELog));  /* max not too large */` | assertion failure | [ ] |
| 2194 | `ZSTD_getOffsetInfo` (src/decompress/zstd_decompress_block.c:2030) | `if (table[u].nbAdditionalBits > 22) info.longOffsetShare += 1;` | branch-specific rejection/error | [ ] |
| 2195 | `ZSTD_getOffsetInfo` (src/decompress/zstd_decompress_block.c:2033) | `assert(tableLog <= OffFSELog);` | assertion failure | [ ] |
| 2196 | `ZSTD_maxShortOffset` (src/decompress/zstd_decompress_block.c:2060) | `assert(ZSTD_highbit32((U32)maxOffbase) == STREAM_ACCUMULATOR_MIN);` | assertion failure | [ ] |
| 2197 | `ZSTD_decompressBlock_internal` (src/decompress/zstd_decompress_block.c:2081) | `RETURN_ERROR_IF(srcSize > ZSTD_blockSizeMax(dctx), srcSize_wrong, "");` | `ERROR(x)` | [ ] |
| 2198 | `ZSTD_decompressBlock_internal` (src/decompress/zstd_decompress_block.c:2129) | `RETURN_ERROR_IF((dst == NULL \|\| dstCapacity == 0) && nbSeq > 0, dstSize_tooSmall, "NULL not handled");` | `ERROR(0)` | [ ] |
| 2199 | `ZSTD_decompressBlock_internal` (src/decompress/zstd_decompress_block.c:2130) | `RETURN_ERROR_IF(MEM_64bits() && sizeof(size_t) == sizeof(void*) && (size_t)(-1) - (size_t)dst < (size_t)(1 << 20), dstSize_tooSmall,` | `ERROR(s)` | [ ] |
| 2200 | `ZSTD_decompressBlock_internal` (src/decompress/zstd_decompress_block.c:2137) | `if (isLongOffset \|\| (!usePrefetchDecoder && (totalHistorySize > (1u << 24)) && (nbSeq > 8))) {` | branch-specific rejection/error | [ ] |
| 2201 | `ZSTD_decompressBlock_internal` (src/decompress/zstd_decompress_block.c:2139) | `if (isLongOffset && info.maxNbAdditionalBits <= STREAM_ACCUMULATOR_MIN) {` | branch-specific rejection/error | [ ] |
| 2202 | `ZSTD_decompressBlock_internal` (src/decompress/zstd_decompress_block.c:2168) | `if (dctx->litBufferLocation == ZSTD_split)` | branch-specific rejection/error | [ ] |
| 2203 | `ZSTD_checkContinuity` (src/decompress/zstd_decompress_block.c:2180) | `if (dst != dctx->previousDstEnd && dstSize > 0) {   /* not contiguous */` | branch-specific rejection/error | [ ] |
| 2204 | `ZBUFF_compressInit_advanced` (src/deprecated/zbuff_compress.c:76) | `if (pledgedSrcSize==0) pledgedSrcSize = ZSTD_CONTENTSIZE_UNKNOWN;  /* preserve "0 == unknown" behavior */` | branch-specific rejection/error | [ ] |
| 2205 | `(file scope)` (src/dictBuilder/cover.c:77) | `if (displayLevel >= l) {                                                     \` | branch-specific rejection/error | [ ] |
| 2206 | `(file scope)` (src/dictBuilder/cover.c:89) | `if (displayLevel >= l) {                                                     \` | branch-specific rejection/error | [ ] |
| 2207 | `(file scope)` (src/dictBuilder/cover.c:90) | `if ((clock() - g_time > g_refreshRate) \|\| (displayLevel >= 4)) {           \` | branch-specific rejection/error | [ ] |
| 2208 | `COVER_map_init` (src/dictBuilder/cover.c:138) | `if (!map->data) {` | branch-specific rejection/error | [ ] |
| 2209 | `COVER_map_index` (src/dictBuilder/cover.c:163) | `if (pos->value == MAP_EMPTY_VALUE) {` | branch-specific rejection/error | [ ] |
| 2210 | `COVER_map_index` (src/dictBuilder/cover.c:166) | `if (pos->key == key) {` | branch-specific rejection/error | [ ] |
| 2211 | `COVER_map_at` (src/dictBuilder/cover.c:179) | `if (pos->value == MAP_EMPTY_VALUE) {` | branch-specific rejection/error | [ ] |
| 2212 | `COVER_map_remove` (src/dictBuilder/cover.c:193) | `if (del->value == MAP_EMPTY_VALUE) {` | branch-specific rejection/error | [ ] |
| 2213 | `COVER_map_remove` (src/dictBuilder/cover.c:199) | `if (pos->value == MAP_EMPTY_VALUE) {` | branch-specific rejection/error | [ ] |
| 2214 | `COVER_map_remove` (src/dictBuilder/cover.c:204) | `if (((i - COVER_map_hash(map, pos->key)) & map->sizeMask) >= shift) {` | branch-specific rejection/error | [ ] |
| 2215 | `COVER_map_destroy` (src/dictBuilder/cover.c:219) | `if (map->data) {` | branch-specific rejection/error | [ ] |
| 2216 | `COVER_cmp8` (src/dictBuilder/cover.c:282) | `if (lhs < rhs) {` | `-1` | [ ] |
| 2217 | `COVER_cmp8` (src/dictBuilder/cover.c:283) | `return -1;` | `-1` | [ ] |
| 2218 | `COVER_strict_cmp` (src/dictBuilder/cover.c:299) | `if (result == 0) {` | branch-specific rejection/error | [ ] |
| 2219 | `COVER_strict_cmp8` (src/dictBuilder/cover.c:315) | `if (result == 0) {` | branch-specific rejection/error | [ ] |
| 2220 | `COVER_lower_bound` (src/dictBuilder/cover.c:358) | `assert(last >= first);` | assertion failure | [ ] |
| 2221 | `COVER_lower_bound` (src/dictBuilder/cover.c:363) | `if (*ptr < value) {` | branch-specific rejection/error | [ ] |
| 2222 | `COVER_group` (src/dictBuilder/cover.c:431) | `if (*grpPtr < curSampleEnd) {` | branch-specific rejection/error | [ ] |
| 2223 | `COVER_selectSegment` (src/dictBuilder/cover.c:492) | `if (*newDmerOcc == 0) {` | branch-specific rejection/error | [ ] |
| 2224 | `COVER_selectSegment` (src/dictBuilder/cover.c:509) | `if (*delDmerOcc == 0) {` | branch-specific rejection/error | [ ] |
| 2225 | `COVER_selectSegment` (src/dictBuilder/cover.c:516) | `if (activeSegment.score > bestSegment.score) {` | branch-specific rejection/error | [ ] |
| 2226 | `COVER_selectSegment` (src/dictBuilder/cover.c:527) | `if (freq != 0) {` | branch-specific rejection/error | [ ] |
| 2227 | `COVER_checkParameters` (src/dictBuilder/cover.c:552) | `if (parameters.d == 0 \|\| parameters.k == 0) {` | branch-specific rejection/error | [ ] |
| 2228 | `COVER_checkParameters` (src/dictBuilder/cover.c:556) | `if (parameters.k > maxDictSize) {` | branch-specific rejection/error | [ ] |
| 2229 | `COVER_checkParameters` (src/dictBuilder/cover.c:560) | `if (parameters.d > parameters.k) {` | branch-specific rejection/error | [ ] |
| 2230 | `COVER_checkParameters` (src/dictBuilder/cover.c:564) | `if (parameters.splitPoint <= 0 \|\| parameters.splitPoint > 1){` | branch-specific rejection/error | [ ] |
| 2231 | `COVER_ctx_destroy` (src/dictBuilder/cover.c:577) | `if (ctx->suffix) {` | branch-specific rejection/error | [ ] |
| 2232 | `COVER_ctx_destroy` (src/dictBuilder/cover.c:581) | `if (ctx->freqs) {` | branch-specific rejection/error | [ ] |
| 2233 | `COVER_ctx_destroy` (src/dictBuilder/cover.c:585) | `if (ctx->dmerAt) {` | branch-specific rejection/error | [ ] |
| 2234 | `COVER_ctx_destroy` (src/dictBuilder/cover.c:589) | `if (ctx->offsets) {` | branch-specific rejection/error | [ ] |
| 2235 | `COVER_ctx_init` (src/dictBuilder/cover.c:614) | `if (totalSamplesSize < MAX(d, sizeof(U64)) \|\|` | `ERROR(srcSize_wrong)` | [ ] |
| 2236 | `COVER_ctx_init` (src/dictBuilder/cover.c:618) | `return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 2237 | `COVER_ctx_init` (src/dictBuilder/cover.c:621) | `if (nbTrainSamples < 5) {` | `ERROR(srcSize_wrong)` | [ ] |
| 2238 | `COVER_ctx_init` (src/dictBuilder/cover.c:623) | `return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 2239 | `COVER_ctx_init` (src/dictBuilder/cover.c:626) | `if (nbTestSamples < 1) {` | `ERROR(srcSize_wrong)` | [ ] |
| 2240 | `COVER_ctx_init` (src/dictBuilder/cover.c:628) | `return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 2241 | `COVER_ctx_init` (src/dictBuilder/cover.c:648) | `if (!ctx->suffix \|\| !ctx->dmerAt \|\| !ctx->offsets) {` | `ERROR(memory_allocation)` | [ ] |
| 2242 | `COVER_ctx_init` (src/dictBuilder/cover.c:651) | `return ERROR(memory_allocation);` | `ERROR(memory_allocation)` | [ ] |
| 2243 | `COVER_warnOnSmallCorpus` (src/dictBuilder/cover.c:694) | `if (ratio >= 10) {` | branch-specific rejection/error | [ ] |
| 2244 | `COVER_computeEpochs` (src/dictBuilder/cover.c:714) | `if (epochs.size >= minEpochSize) {` | branch-specific rejection/error | [ ] |
| 2245 | `COVER_computeEpochs` (src/dictBuilder/cover.c:715) | `assert(epochs.size * epochs.num <= nbDmers);` | assertion failure | [ ] |
| 2246 | `COVER_computeEpochs` (src/dictBuilder/cover.c:720) | `assert(epochs.size * epochs.num <= nbDmers);` | assertion failure | [ ] |
| 2247 | `COVER_buildDictionary` (src/dictBuilder/cover.c:754) | `if (segment.score == 0) {` | branch-specific rejection/error | [ ] |
| 2248 | `COVER_buildDictionary` (src/dictBuilder/cover.c:755) | `if (++zeroScoreRun >= maxZeroScoreRun) {` | branch-specific rejection/error | [ ] |
| 2249 | `COVER_buildDictionary` (src/dictBuilder/cover.c:763) | `if (segmentSize < parameters.d) {` | branch-specific rejection/error | [ ] |
| 2250 | `ZDICT_trainFromBuffer_cover` (src/dictBuilder/cover.c:793) | `return ERROR(parameter_outOfBound);` | `ERROR(parameter_outOfBound)` | [ ] |
| 2251 | `ZDICT_trainFromBuffer_cover` (src/dictBuilder/cover.c:795) | `if (nbSamples == 0) {` | `ERROR(srcSize_wrong)` | [ ] |
| 2252 | `ZDICT_trainFromBuffer_cover` (src/dictBuilder/cover.c:797) | `return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 2253 | `ZDICT_trainFromBuffer_cover` (src/dictBuilder/cover.c:799) | `if (dictBufferCapacity < ZDICT_DICTSIZE_MIN) {` | `ERROR(dstSize_tooSmall)` | [ ] |
| 2254 | `ZDICT_trainFromBuffer_cover` (src/dictBuilder/cover.c:802) | `return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [ ] |
| 2255 | `ZDICT_trainFromBuffer_cover` (src/dictBuilder/cover.c:816) | `return ERROR(memory_allocation);` | `ERROR(memory_allocation)` | [ ] |
| 2256 | `COVER_checkTotalCompressedSize` (src/dictBuilder/cover.c:844) | `size_t totalCompressedSize = ERROR(GENERIC);` | `ERROR(GENERIC)` | [ ] |
| 2257 | `COVER_best_init` (src/dictBuilder/cover.c:896) | `if (best==NULL) return; /* compatible with init on NULL */` | branch-specific rejection/error | [ ] |
| 2258 | `COVER_best_destroy` (src/dictBuilder/cover.c:928) | `if (best->dict) {` | branch-specific rejection/error | [ ] |
| 2259 | `COVER_best_finish` (src/dictBuilder/cover.c:969) | `if (compressedSize < best->compressedSize) {` | branch-specific rejection/error | [ ] |
| 2260 | `COVER_best_finish` (src/dictBuilder/cover.c:971) | `if (!best->dict \|\| best->dictSize < dictSize) {` | branch-specific rejection/error | [ ] |
| 2261 | `COVER_best_finish` (src/dictBuilder/cover.c:972) | `if (best->dict) {` | branch-specific rejection/error | [ ] |
| 2262 | `COVER_best_finish` (src/dictBuilder/cover.c:976) | `if (!best->dict) {` | `ERROR(GENERIC)` | [ ] |
| 2263 | `COVER_best_finish` (src/dictBuilder/cover.c:977) | `best->compressedSize = ERROR(GENERIC);` | `ERROR(GENERIC)` | [ ] |
| 2264 | `COVER_best_finish` (src/dictBuilder/cover.c:992) | `if (liveJobs == 0) {` | branch-specific rejection/error | [ ] |
| 2265 | `COVER_selectDict` (src/dictBuilder/cover.c:1061) | `if (params.shrinkDict == 0) {` | branch-specific rejection/error | [ ] |
| 2266 | `COVER_selectDict` (src/dictBuilder/cover.c:1095) | `if ((double)totalCompressedSize <= (double)largestCompressed * regressionTolerance) {` | branch-specific rejection/error | [ ] |
| 2267 | `COVER_tryParameters` (src/dictBuilder/cover.c:1129) | `size_t totalCompressedSize = ERROR(GENERIC);` | `ERROR(GENERIC)` | [ ] |
| 2268 | `COVER_tryParameters` (src/dictBuilder/cover.c:1133) | `COVER_dictSelection_t selection = COVER_dictSelectionError(ERROR(GENERIC));` | `ERROR(GENERIC)` | [ ] |
| 2269 | `ZDICT_optimizeTrainFromBuffer_cover` (src/dictBuilder/cover.c:1195) | `if (splitPoint <= 0 \|\| splitPoint > 1) {` | `ERROR(parameter_outOfBound)` | [ ] |
| 2270 | `ZDICT_optimizeTrainFromBuffer_cover` (src/dictBuilder/cover.c:1197) | `return ERROR(parameter_outOfBound);` | `ERROR(parameter_outOfBound)` | [ ] |
| 2271 | `ZDICT_optimizeTrainFromBuffer_cover` (src/dictBuilder/cover.c:1199) | `if (kMinK < kMaxD \|\| kMaxK < kMinK) {` | `ERROR(parameter_outOfBound)` | [ ] |
| 2272 | `ZDICT_optimizeTrainFromBuffer_cover` (src/dictBuilder/cover.c:1201) | `return ERROR(parameter_outOfBound);` | `ERROR(parameter_outOfBound)` | [ ] |
| 2273 | `ZDICT_optimizeTrainFromBuffer_cover` (src/dictBuilder/cover.c:1203) | `if (nbSamples == 0) {` | `ERROR(srcSize_wrong)` | [ ] |
| 2274 | `ZDICT_optimizeTrainFromBuffer_cover` (src/dictBuilder/cover.c:1205) | `return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 2275 | `ZDICT_optimizeTrainFromBuffer_cover` (src/dictBuilder/cover.c:1207) | `if (dictBufferCapacity < ZDICT_DICTSIZE_MIN) {` | `ERROR(dstSize_tooSmall)` | [ ] |
| 2276 | `ZDICT_optimizeTrainFromBuffer_cover` (src/dictBuilder/cover.c:1210) | `return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [ ] |
| 2277 | `ZDICT_optimizeTrainFromBuffer_cover` (src/dictBuilder/cover.c:1212) | `if (nbThreads > 1) {` | `ERROR(memory_allocation)` | [ ] |
| 2278 | `ZDICT_optimizeTrainFromBuffer_cover` (src/dictBuilder/cover.c:1215) | `return ERROR(memory_allocation);` | `ERROR(memory_allocation)` | [ ] |
| 2279 | `ZDICT_optimizeTrainFromBuffer_cover` (src/dictBuilder/cover.c:1253) | `return ERROR(memory_allocation);` | `ERROR(memory_allocation)` | [ ] |
| 2280 | `ZDICT_optimizeTrainFromBuffer_cover` (src/dictBuilder/cover.c:1266) | `if (!COVER_checkParameters(data->parameters, dictBufferCapacity)) {` | branch-specific rejection/error | [ ] |
| 2281 | `(file scope)` (src/dictBuilder/divsufsort.c:104) | `assert(ssize < STACK_SIZE);\` | assertion failure | [ ] |
| 2282 | `(file scope)` (src/dictBuilder/divsufsort.c:110) | `assert(ssize < STACK_SIZE);\` | assertion failure | [ ] |
| 2283 | `(file scope)` (src/dictBuilder/divsufsort.c:116) | `assert(0 <= ssize);\` | assertion failure | [ ] |
| 2284 | `(file scope)` (src/dictBuilder/divsufsort.c:117) | `if(ssize == 0) { return; }\` | branch-specific rejection/error | [ ] |
| 2285 | `(file scope)` (src/dictBuilder/divsufsort.c:123) | `assert(0 <= ssize);\` | assertion failure | [ ] |
| 2286 | `(file scope)` (src/dictBuilder/divsufsort.c:124) | `if(ssize == 0) { return; }\` | branch-specific rejection/error | [ ] |
| 2287 | `ss_isqrt` (src/dictBuilder/divsufsort.c:201) | `if(x >= (SS_BLOCKSIZE * SS_BLOCKSIZE)) { return SS_BLOCKSIZE; }` | branch-specific rejection/error | [ ] |
| 2288 | `ss_isqrt` (src/dictBuilder/divsufsort.c:210) | `if(e >= 16) {` | branch-specific rejection/error | [ ] |
| 2289 | `ss_isqrt` (src/dictBuilder/divsufsort.c:212) | `if(e >= 24) { y = (y + 1 + x / y) >> 1; }` | branch-specific rejection/error | [ ] |
| 2290 | `ss_isqrt` (src/dictBuilder/divsufsort.c:214) | `} else if(e >= 8) {` | branch-specific rejection/error | [ ] |
| 2291 | `ss_insertionsort` (src/dictBuilder/divsufsort.c:266) | `if(last <= j) { break; }` | branch-specific rejection/error | [ ] |
| 2292 | `ss_insertionsort` (src/dictBuilder/divsufsort.c:268) | `if(r == 0) { *j = ~*j; }` | branch-specific rejection/error | [ ] |
| 2293 | `ss_fixdown` (src/dictBuilder/divsufsort.c:290) | `if(d < (e = Td[PA[SA[j]]])) { k = j; d = e; }` | branch-specific rejection/error | [ ] |
| 2294 | `ss_fixdown` (src/dictBuilder/divsufsort.c:291) | `if(d <= c) { break; }` | branch-specific rejection/error | [ ] |
| 2295 | `ss_heapsort` (src/dictBuilder/divsufsort.c:304) | `if((size % 2) == 0) {` | branch-specific rejection/error | [ ] |
| 2296 | `ss_heapsort` (src/dictBuilder/divsufsort.c:306) | `if(Td[PA[SA[m / 2]]] < Td[PA[SA[m]]]) { SWAP(SA[m], SA[m / 2]); }` | branch-specific rejection/error | [ ] |
| 2297 | `ss_heapsort` (src/dictBuilder/divsufsort.c:310) | `if((size % 2) == 0) { SWAP(SA[0], SA[m]); ss_fixdown(Td, PA, SA, 0, m); }` | branch-specific rejection/error | [ ] |
| 2298 | `ss_median3` (src/dictBuilder/divsufsort.c:327) | `if(Td[PA[*v1]] > Td[PA[*v2]]) { SWAP(v1, v2); }` | branch-specific rejection/error | [ ] |
| 2299 | `ss_median3` (src/dictBuilder/divsufsort.c:328) | `if(Td[PA[*v2]] > Td[PA[*v3]]) {` | branch-specific rejection/error | [ ] |
| 2300 | `ss_median3` (src/dictBuilder/divsufsort.c:329) | `if(Td[PA[*v1]] > Td[PA[*v3]]) { return v1; }` | branch-specific rejection/error | [ ] |
| 2301 | `ss_median5` (src/dictBuilder/divsufsort.c:341) | `if(Td[PA[*v2]] > Td[PA[*v3]]) { SWAP(v2, v3); }` | branch-specific rejection/error | [ ] |
| 2302 | `ss_median5` (src/dictBuilder/divsufsort.c:342) | `if(Td[PA[*v4]] > Td[PA[*v5]]) { SWAP(v4, v5); }` | branch-specific rejection/error | [ ] |
| 2303 | `ss_median5` (src/dictBuilder/divsufsort.c:343) | `if(Td[PA[*v2]] > Td[PA[*v4]]) { SWAP(v2, v4); SWAP(v3, v5); }` | branch-specific rejection/error | [ ] |
| 2304 | `ss_median5` (src/dictBuilder/divsufsort.c:344) | `if(Td[PA[*v1]] > Td[PA[*v3]]) { SWAP(v1, v3); }` | branch-specific rejection/error | [ ] |
| 2305 | `ss_median5` (src/dictBuilder/divsufsort.c:345) | `if(Td[PA[*v1]] > Td[PA[*v4]]) { SWAP(v1, v4); SWAP(v3, v5); }` | branch-specific rejection/error | [ ] |
| 2306 | `ss_median5` (src/dictBuilder/divsufsort.c:346) | `if(Td[PA[*v3]] > Td[PA[*v4]]) { return v4; }` | branch-specific rejection/error | [ ] |
| 2307 | `ss_pivot` (src/dictBuilder/divsufsort.c:360) | `if(t <= 512) {` | branch-specific rejection/error | [ ] |
| 2308 | `ss_pivot` (src/dictBuilder/divsufsort.c:361) | `if(t <= 32) {` | branch-specific rejection/error | [ ] |
| 2309 | `ss_partition` (src/dictBuilder/divsufsort.c:388) | `if(b <= a) { break; }` | branch-specific rejection/error | [ ] |
| 2310 | `ss_partition` (src/dictBuilder/divsufsort.c:393) | `if(first < a) { *first = ~*first; }` | branch-specific rejection/error | [ ] |
| 2311 | `ss_mintrosort` (src/dictBuilder/divsufsort.c:414) | `if((last - first) <= SS_INSERTIONSORT_THRESHOLD) {` | branch-specific rejection/error | [ ] |
| 2312 | `ss_mintrosort` (src/dictBuilder/divsufsort.c:416) | `if(1 < (last - first)) { ss_insertionsort(T, PA, first, last, depth); }` | branch-specific rejection/error | [ ] |
| 2313 | `ss_mintrosort` (src/dictBuilder/divsufsort.c:423) | `if(limit-- == 0) { ss_heapsort(Td, PA, first, last - first); }` | branch-specific rejection/error | [ ] |
| 2314 | `ss_mintrosort` (src/dictBuilder/divsufsort.c:424) | `if(limit < 0) {` | branch-specific rejection/error | [ ] |
| 2315 | `ss_mintrosort` (src/dictBuilder/divsufsort.c:427) | `if(1 < (a - first)) { break; }` | branch-specific rejection/error | [ ] |
| 2316 | `ss_mintrosort` (src/dictBuilder/divsufsort.c:432) | `if(Td[PA[*first] - 1] < v) {` | branch-specific rejection/error | [ ] |
| 2317 | `ss_mintrosort` (src/dictBuilder/divsufsort.c:435) | `if((a - first) <= (last - a)) {` | branch-specific rejection/error | [ ] |
| 2318 | `ss_mintrosort` (src/dictBuilder/divsufsort.c:436) | `if(1 < (a - first)) {` | branch-specific rejection/error | [ ] |
| 2319 | `ss_mintrosort` (src/dictBuilder/divsufsort.c:443) | `if(1 < (last - a)) {` | branch-specific rejection/error | [ ] |
| 2320 | `ss_mintrosort` (src/dictBuilder/divsufsort.c:460) | `if(((a = b) < last) && (x < v)) {` | branch-specific rejection/error | [ ] |
| 2321 | `ss_mintrosort` (src/dictBuilder/divsufsort.c:466) | `if((b < (d = c)) && (x > v)) {` | branch-specific rejection/error | [ ] |
| 2322 | `ss_mintrosort` (src/dictBuilder/divsufsort.c:481) | `if(a <= d) {` | branch-specific rejection/error | [ ] |
| 2323 | `ss_mintrosort` (src/dictBuilder/divsufsort.c:484) | `if((s = a - first) > (t = b - a)) { s = t; }` | branch-specific rejection/error | [ ] |
| 2324 | `ss_mintrosort` (src/dictBuilder/divsufsort.c:486) | `if((s = d - c) > (t = last - d - 1)) { s = t; }` | branch-specific rejection/error | [ ] |
| 2325 | `ss_mintrosort` (src/dictBuilder/divsufsort.c:492) | `if((a - first) <= (last - c)) {` | branch-specific rejection/error | [ ] |
| 2326 | `ss_mintrosort` (src/dictBuilder/divsufsort.c:493) | `if((last - c) <= (c - b)) {` | branch-specific rejection/error | [ ] |
| 2327 | `ss_mintrosort` (src/dictBuilder/divsufsort.c:497) | `} else if((a - first) <= (c - b)) {` | branch-specific rejection/error | [ ] |
| 2328 | `ss_mintrosort` (src/dictBuilder/divsufsort.c:507) | `if((a - first) <= (c - b)) {` | branch-specific rejection/error | [ ] |
| 2329 | `ss_mintrosort` (src/dictBuilder/divsufsort.c:511) | `} else if((last - c) <= (c - b)) {` | branch-specific rejection/error | [ ] |
| 2330 | `ss_mintrosort` (src/dictBuilder/divsufsort.c:523) | `if(Td[PA[*first] - 1] < v) {` | branch-specific rejection/error | [ ] |
| 2331 | `ss_rotate` (src/dictBuilder/divsufsort.c:557) | `if(l < r) {` | branch-specific rejection/error | [ ] |
| 2332 | `ss_rotate` (src/dictBuilder/divsufsort.c:562) | `if(b < first) {` | branch-specific rejection/error | [ ] |
| 2333 | `ss_rotate` (src/dictBuilder/divsufsort.c:565) | `if((r -= l + 1) <= l) { break; }` | branch-specific rejection/error | [ ] |
| 2334 | `ss_rotate` (src/dictBuilder/divsufsort.c:575) | `if(last <= b) {` | branch-specific rejection/error | [ ] |
| 2335 | `ss_rotate` (src/dictBuilder/divsufsort.c:578) | `if((l -= r + 1) <= r) { break; }` | branch-specific rejection/error | [ ] |
| 2336 | `ss_inplacemerge` (src/dictBuilder/divsufsort.c:602) | `if(*(last - 1) < 0) { x = 1; p = PA + ~*(last - 1); }` | branch-specific rejection/error | [ ] |
| 2337 | `ss_inplacemerge` (src/dictBuilder/divsufsort.c:609) | `if(q < 0) {` | branch-specific rejection/error | [ ] |
| 2338 | `ss_inplacemerge` (src/dictBuilder/divsufsort.c:616) | `if(a < middle) {` | branch-specific rejection/error | [ ] |
| 2339 | `ss_inplacemerge` (src/dictBuilder/divsufsort.c:617) | `if(r == 0) { *a = ~*a; }` | branch-specific rejection/error | [ ] |
| 2340 | `ss_inplacemerge` (src/dictBuilder/divsufsort.c:624) | `if(x != 0) { while(*--last < 0) { } }` | branch-specific rejection/error | [ ] |
| 2341 | `ss_mergeforward` (src/dictBuilder/divsufsort.c:647) | `if(r < 0) {` | branch-specific rejection/error | [ ] |
| 2342 | `ss_mergeforward` (src/dictBuilder/divsufsort.c:650) | `if(bufend <= b) { *bufend = t; return; }` | branch-specific rejection/error | [ ] |
| 2343 | `ss_mergeforward` (src/dictBuilder/divsufsort.c:653) | `} else if(r > 0) {` | branch-specific rejection/error | [ ] |
| 2344 | `ss_mergeforward` (src/dictBuilder/divsufsort.c:656) | `if(last <= c) {` | branch-specific rejection/error | [ ] |
| 2345 | `ss_mergeforward` (src/dictBuilder/divsufsort.c:666) | `if(bufend <= b) { *bufend = t; return; }` | branch-specific rejection/error | [ ] |
| 2346 | `ss_mergeforward` (src/dictBuilder/divsufsort.c:672) | `if(last <= c) {` | branch-specific rejection/error | [ ] |
| 2347 | `ss_mergebackward` (src/dictBuilder/divsufsort.c:698) | `if(*bufend < 0)       { p1 = PA + ~*bufend; x \|= 1; }` | branch-specific rejection/error | [ ] |
| 2348 | `ss_mergebackward` (src/dictBuilder/divsufsort.c:700) | `if(*(middle - 1) < 0) { p2 = PA + ~*(middle - 1); x \|= 2; }` | branch-specific rejection/error | [ ] |
| 2349 | `ss_mergebackward` (src/dictBuilder/divsufsort.c:704) | `if(0 < r) {` | branch-specific rejection/error | [ ] |
| 2350 | `ss_mergebackward` (src/dictBuilder/divsufsort.c:707) | `if(b <= buf) { *buf = t; break; }` | branch-specific rejection/error | [ ] |
| 2351 | `ss_mergebackward` (src/dictBuilder/divsufsort.c:709) | `if(*b < 0) { p1 = PA + ~*b; x \|= 1; }` | branch-specific rejection/error | [ ] |
| 2352 | `ss_mergebackward` (src/dictBuilder/divsufsort.c:711) | `} else if(r < 0) {` | branch-specific rejection/error | [ ] |
| 2353 | `ss_mergebackward` (src/dictBuilder/divsufsort.c:714) | `if(c < first) {` | branch-specific rejection/error | [ ] |
| 2354 | `ss_mergebackward` (src/dictBuilder/divsufsort.c:719) | `if(*c < 0) { p2 = PA + ~*c; x \|= 2; }` | branch-specific rejection/error | [ ] |
| 2355 | `ss_mergebackward` (src/dictBuilder/divsufsort.c:724) | `if(b <= buf) { *buf = t; break; }` | branch-specific rejection/error | [ ] |
| 2356 | `ss_mergebackward` (src/dictBuilder/divsufsort.c:728) | `if(c < first) {` | branch-specific rejection/error | [ ] |
| 2357 | `ss_mergebackward` (src/dictBuilder/divsufsort.c:733) | `if(*b < 0) { p1 = PA + ~*b; x \|= 1; }` | branch-specific rejection/error | [ ] |
| 2358 | `ss_mergebackward` (src/dictBuilder/divsufsort.c:735) | `if(*c < 0) { p2 = PA + ~*c; x \|= 2; }` | branch-specific rejection/error | [ ] |
| 2359 | `ss_swapmerge` (src/dictBuilder/divsufsort.c:755) | `if(((c) & 4) && ((ss_compare(T, PA + GETIDX(*((b) - 1)), PA + *(b), depth) == 0))) {\` | branch-specific rejection/error | [ ] |
| 2360 | `ss_swapmerge` (src/dictBuilder/divsufsort.c:766) | `if((last - middle) <= bufsize) {` | branch-specific rejection/error | [ ] |
| 2361 | `ss_swapmerge` (src/dictBuilder/divsufsort.c:767) | `if((first < middle) && (middle < last)) {` | branch-specific rejection/error | [ ] |
| 2362 | `ss_swapmerge` (src/dictBuilder/divsufsort.c:775) | `if((middle - first) <= bufsize) {` | branch-specific rejection/error | [ ] |
| 2363 | `ss_swapmerge` (src/dictBuilder/divsufsort.c:776) | `if(first < middle) {` | branch-specific rejection/error | [ ] |
| 2364 | `ss_swapmerge` (src/dictBuilder/divsufsort.c:794) | `if(0 < m) {` | branch-specific rejection/error | [ ] |
| 2365 | `ss_swapmerge` (src/dictBuilder/divsufsort.c:798) | `if(rm < last) {` | branch-specific rejection/error | [ ] |
| 2366 | `ss_swapmerge` (src/dictBuilder/divsufsort.c:799) | `if(*rm < 0) {` | branch-specific rejection/error | [ ] |
| 2367 | `ss_swapmerge` (src/dictBuilder/divsufsort.c:801) | `if(first < lm) { for(; *--l < 0;) { } next \|= 4; }` | branch-specific rejection/error | [ ] |
| 2368 | `ss_swapmerge` (src/dictBuilder/divsufsort.c:803) | `} else if(first < lm) {` | branch-specific rejection/error | [ ] |
| 2369 | `ss_swapmerge` (src/dictBuilder/divsufsort.c:809) | `if((l - first) <= (last - r)) {` | branch-specific rejection/error | [ ] |
| 2370 | `ss_swapmerge` (src/dictBuilder/divsufsort.c:818) | `if(ss_compare(T, PA + GETIDX(*(middle - 1)), PA + *middle, depth) == 0) {` | branch-specific rejection/error | [ ] |
| 2371 | `sssort` (src/dictBuilder/divsufsort.c:847) | `if(lastsuffix != 0) { ++first; }` | branch-specific rejection/error | [ ] |
| 2372 | `sssort` (src/dictBuilder/divsufsort.c:852) | `if((bufsize < SS_BLOCKSIZE) &&` | branch-specific rejection/error | [ ] |
| 2373 | `sssort` (src/dictBuilder/divsufsort.c:855) | `if(SS_BLOCKSIZE < limit) { limit = SS_BLOCKSIZE; }` | branch-specific rejection/error | [ ] |
| 2374 | `sssort` (src/dictBuilder/divsufsort.c:868) | `if(curbufsize <= bufsize) { curbufsize = bufsize, curbuf = buf; }` | branch-specific rejection/error | [ ] |
| 2375 | `sssort` (src/dictBuilder/divsufsort.c:884) | `if(limit != 0) {` | branch-specific rejection/error | [ ] |
| 2376 | `sssort` (src/dictBuilder/divsufsort.c:894) | `if(lastsuffix != 0) {` | branch-specific rejection/error | [ ] |
| 2377 | `tr_insertionsort` (src/dictBuilder/divsufsort.c:934) | `if(b < first) { break; }` | branch-specific rejection/error | [ ] |
| 2378 | `tr_insertionsort` (src/dictBuilder/divsufsort.c:936) | `if(r == 0) { *b = ~*b; }` | branch-specific rejection/error | [ ] |
| 2379 | `tr_fixdown` (src/dictBuilder/divsufsort.c:953) | `if(d < (e = ISAd[SA[j]])) { k = j; d = e; }` | branch-specific rejection/error | [ ] |
| 2380 | `tr_fixdown` (src/dictBuilder/divsufsort.c:954) | `if(d <= c) { break; }` | branch-specific rejection/error | [ ] |
| 2381 | `tr_heapsort` (src/dictBuilder/divsufsort.c:967) | `if((size % 2) == 0) {` | branch-specific rejection/error | [ ] |
| 2382 | `tr_heapsort` (src/dictBuilder/divsufsort.c:969) | `if(ISAd[SA[m / 2]] < ISAd[SA[m]]) { SWAP(SA[m], SA[m / 2]); }` | branch-specific rejection/error | [ ] |
| 2383 | `tr_heapsort` (src/dictBuilder/divsufsort.c:973) | `if((size % 2) == 0) { SWAP(SA[0], SA[m]); tr_fixdown(ISAd, SA, 0, m); }` | branch-specific rejection/error | [ ] |
| 2384 | `tr_median3` (src/dictBuilder/divsufsort.c:989) | `if(ISAd[*v1] > ISAd[*v2]) { SWAP(v1, v2); }` | branch-specific rejection/error | [ ] |
| 2385 | `tr_median3` (src/dictBuilder/divsufsort.c:990) | `if(ISAd[*v2] > ISAd[*v3]) {` | branch-specific rejection/error | [ ] |
| 2386 | `tr_median3` (src/dictBuilder/divsufsort.c:991) | `if(ISAd[*v1] > ISAd[*v3]) { return v1; }` | branch-specific rejection/error | [ ] |
| 2387 | `tr_median5` (src/dictBuilder/divsufsort.c:1003) | `if(ISAd[*v2] > ISAd[*v3]) { SWAP(v2, v3); }` | branch-specific rejection/error | [ ] |
| 2388 | `tr_median5` (src/dictBuilder/divsufsort.c:1004) | `if(ISAd[*v4] > ISAd[*v5]) { SWAP(v4, v5); }` | branch-specific rejection/error | [ ] |
| 2389 | `tr_median5` (src/dictBuilder/divsufsort.c:1005) | `if(ISAd[*v2] > ISAd[*v4]) { SWAP(v2, v4); SWAP(v3, v5); }` | branch-specific rejection/error | [ ] |
| 2390 | `tr_median5` (src/dictBuilder/divsufsort.c:1006) | `if(ISAd[*v1] > ISAd[*v3]) { SWAP(v1, v3); }` | branch-specific rejection/error | [ ] |
| 2391 | `tr_median5` (src/dictBuilder/divsufsort.c:1007) | `if(ISAd[*v1] > ISAd[*v4]) { SWAP(v1, v4); SWAP(v3, v5); }` | branch-specific rejection/error | [ ] |
| 2392 | `tr_median5` (src/dictBuilder/divsufsort.c:1008) | `if(ISAd[*v3] > ISAd[*v4]) { return v4; }` | branch-specific rejection/error | [ ] |
| 2393 | `tr_pivot` (src/dictBuilder/divsufsort.c:1022) | `if(t <= 512) {` | branch-specific rejection/error | [ ] |
| 2394 | `tr_pivot` (src/dictBuilder/divsufsort.c:1023) | `if(t <= 32) {` | branch-specific rejection/error | [ ] |
| 2395 | `trbudget_check` (src/dictBuilder/divsufsort.c:1058) | `if(size <= budget->remain) { budget->remain -= size; return 1; }` | branch-specific rejection/error | [ ] |
| 2396 | `trbudget_check` (src/dictBuilder/divsufsort.c:1059) | `if(budget->chance == 0) { budget->count += size; return 0; }` | branch-specific rejection/error | [ ] |
| 2397 | `tr_partition` (src/dictBuilder/divsufsort.c:1078) | `if(((a = b) < last) && (x < v)) {` | branch-specific rejection/error | [ ] |
| 2398 | `tr_partition` (src/dictBuilder/divsufsort.c:1084) | `if((b < (d = c)) && (x > v)) {` | branch-specific rejection/error | [ ] |
| 2399 | `tr_partition` (src/dictBuilder/divsufsort.c:1099) | `if(a <= d) {` | branch-specific rejection/error | [ ] |
| 2400 | `tr_partition` (src/dictBuilder/divsufsort.c:1101) | `if((s = a - first) > (t = b - a)) { s = t; }` | branch-specific rejection/error | [ ] |
| 2401 | `tr_partition` (src/dictBuilder/divsufsort.c:1103) | `if((s = d - c) > (t = last - d - 1)) { s = t; }` | branch-specific rejection/error | [ ] |
| 2402 | `tr_copy` (src/dictBuilder/divsufsort.c:1122) | `if((0 <= (s = *c - depth)) && (ISA[s] == v)) {` | branch-specific rejection/error | [ ] |
| 2403 | `tr_copy` (src/dictBuilder/divsufsort.c:1128) | `if((0 <= (s = *c - depth)) && (ISA[s] == v)) {` | branch-specific rejection/error | [ ] |
| 2404 | `tr_partialcopy` (src/dictBuilder/divsufsort.c:1147) | `if((0 <= (s = *c - depth)) && (ISA[s] == v)) {` | branch-specific rejection/error | [ ] |
| 2405 | `tr_partialcopy` (src/dictBuilder/divsufsort.c:1164) | `if((0 <= (s = *c - depth)) && (ISA[s] == v)) {` | branch-specific rejection/error | [ ] |
| 2406 | `tr_introsort` (src/dictBuilder/divsufsort.c:1189) | `if(limit < 0) {` | branch-specific rejection/error | [ ] |
| 2407 | `tr_introsort` (src/dictBuilder/divsufsort.c:1195) | `if(a < last) {` | branch-specific rejection/error | [ ] |
| 2408 | `tr_introsort` (src/dictBuilder/divsufsort.c:1198) | `if(b < last) {` | branch-specific rejection/error | [ ] |
| 2409 | `tr_introsort` (src/dictBuilder/divsufsort.c:1203) | `if(1 < (b - a)) {` | branch-specific rejection/error | [ ] |
| 2410 | `tr_introsort` (src/dictBuilder/divsufsort.c:1208) | `if((a - first) <= (last - b)) {` | branch-specific rejection/error | [ ] |
| 2411 | `tr_introsort` (src/dictBuilder/divsufsort.c:1209) | `if(1 < (a - first)) {` | branch-specific rejection/error | [ ] |
| 2412 | `tr_introsort` (src/dictBuilder/divsufsort.c:1212) | `} else if(1 < (last - b)) {` | branch-specific rejection/error | [ ] |
| 2413 | `tr_introsort` (src/dictBuilder/divsufsort.c:1218) | `if(1 < (last - b)) {` | branch-specific rejection/error | [ ] |
| 2414 | `tr_introsort` (src/dictBuilder/divsufsort.c:1221) | `} else if(1 < (a - first)) {` | branch-specific rejection/error | [ ] |
| 2415 | `tr_introsort` (src/dictBuilder/divsufsort.c:1230) | `if(stack[ssize].d == 0) {` | branch-specific rejection/error | [ ] |
| 2416 | `tr_introsort` (src/dictBuilder/divsufsort.c:1233) | `if(0 <= trlink) { stack[trlink].d = -1; }` | branch-specific rejection/error | [ ] |
| 2417 | `tr_introsort` (src/dictBuilder/divsufsort.c:1239) | `if(0 <= *first) {` | branch-specific rejection/error | [ ] |
| 2418 | `tr_introsort` (src/dictBuilder/divsufsort.c:1244) | `if(first < last) {` | branch-specific rejection/error | [ ] |
| 2419 | `tr_introsort` (src/dictBuilder/divsufsort.c:1247) | `if(++a < last) { for(b = first, v = a - SA - 1; b < a; ++b) { ISA[*b] = v; } }` | branch-specific rejection/error | [ ] |
| 2420 | `tr_introsort` (src/dictBuilder/divsufsort.c:1251) | `if((a - first) <= (last - a)) {` | branch-specific rejection/error | [ ] |
| 2421 | `tr_introsort` (src/dictBuilder/divsufsort.c:1255) | `if(1 < (last - a)) {` | branch-specific rejection/error | [ ] |
| 2422 | `tr_introsort` (src/dictBuilder/divsufsort.c:1263) | `if(0 <= trlink) { stack[trlink].d = -1; }` | branch-specific rejection/error | [ ] |
| 2423 | `tr_introsort` (src/dictBuilder/divsufsort.c:1264) | `if(1 < (last - a)) {` | branch-specific rejection/error | [ ] |
| 2424 | `tr_introsort` (src/dictBuilder/divsufsort.c:1277) | `if((last - first) <= TR_INSERTIONSORT_THRESHOLD) {` | branch-specific rejection/error | [ ] |
| 2425 | `tr_introsort` (src/dictBuilder/divsufsort.c:1283) | `if(limit-- == 0) {` | branch-specific rejection/error | [ ] |
| 2426 | `tr_introsort` (src/dictBuilder/divsufsort.c:1304) | `if(b < last) { for(c = a, v = b - SA - 1; c < b; ++c) { ISA[*c] = v; } }` | branch-specific rejection/error | [ ] |
| 2427 | `tr_introsort` (src/dictBuilder/divsufsort.c:1307) | `if((1 < (b - a)) && (trbudget_check(budget, b - a))) {` | branch-specific rejection/error | [ ] |
| 2428 | `tr_introsort` (src/dictBuilder/divsufsort.c:1308) | `if((a - first) <= (last - b)) {` | branch-specific rejection/error | [ ] |
| 2429 | `tr_introsort` (src/dictBuilder/divsufsort.c:1309) | `if((last - b) <= (b - a)) {` | branch-specific rejection/error | [ ] |
| 2430 | `tr_introsort` (src/dictBuilder/divsufsort.c:1310) | `if(1 < (a - first)) {` | branch-specific rejection/error | [ ] |
| 2431 | `tr_introsort` (src/dictBuilder/divsufsort.c:1314) | `} else if(1 < (last - b)) {` | branch-specific rejection/error | [ ] |
| 2432 | `tr_introsort` (src/dictBuilder/divsufsort.c:1320) | `} else if((a - first) <= (b - a)) {` | branch-specific rejection/error | [ ] |
| 2433 | `tr_introsort` (src/dictBuilder/divsufsort.c:1321) | `if(1 < (a - first)) {` | branch-specific rejection/error | [ ] |
| 2434 | `tr_introsort` (src/dictBuilder/divsufsort.c:1335) | `if((a - first) <= (b - a)) {` | branch-specific rejection/error | [ ] |
| 2435 | `tr_introsort` (src/dictBuilder/divsufsort.c:1336) | `if(1 < (last - b)) {` | branch-specific rejection/error | [ ] |
| 2436 | `tr_introsort` (src/dictBuilder/divsufsort.c:1340) | `} else if(1 < (a - first)) {` | branch-specific rejection/error | [ ] |
| 2437 | `tr_introsort` (src/dictBuilder/divsufsort.c:1346) | `} else if((last - b) <= (b - a)) {` | branch-specific rejection/error | [ ] |
| 2438 | `tr_introsort` (src/dictBuilder/divsufsort.c:1347) | `if(1 < (last - b)) {` | branch-specific rejection/error | [ ] |
| 2439 | `tr_introsort` (src/dictBuilder/divsufsort.c:1362) | `if((1 < (b - a)) && (0 <= trlink)) { stack[trlink].d = -1; }` | branch-specific rejection/error | [ ] |
| 2440 | `tr_introsort` (src/dictBuilder/divsufsort.c:1363) | `if((a - first) <= (last - b)) {` | branch-specific rejection/error | [ ] |
| 2441 | `tr_introsort` (src/dictBuilder/divsufsort.c:1364) | `if(1 < (a - first)) {` | branch-specific rejection/error | [ ] |
| 2442 | `tr_introsort` (src/dictBuilder/divsufsort.c:1367) | `} else if(1 < (last - b)) {` | branch-specific rejection/error | [ ] |
| 2443 | `tr_introsort` (src/dictBuilder/divsufsort.c:1373) | `if(1 < (last - b)) {` | branch-specific rejection/error | [ ] |
| 2444 | `tr_introsort` (src/dictBuilder/divsufsort.c:1376) | `} else if(1 < (a - first)) {` | branch-specific rejection/error | [ ] |
| 2445 | `tr_introsort` (src/dictBuilder/divsufsort.c:1387) | `if(0 <= trlink) { stack[trlink].d = -1; }` | branch-specific rejection/error | [ ] |
| 2446 | `trsort` (src/dictBuilder/divsufsort.c:1415) | `if((t = *first) < 0) { first -= t; skip += t; }` | branch-specific rejection/error | [ ] |
| 2447 | `trsort` (src/dictBuilder/divsufsort.c:1417) | `if(skip != 0) { *(first + skip) = skip; skip = 0; }` | branch-specific rejection/error | [ ] |
| 2448 | `trsort` (src/dictBuilder/divsufsort.c:1419) | `if(1 < (last - first)) {` | branch-specific rejection/error | [ ] |
| 2449 | `trsort` (src/dictBuilder/divsufsort.c:1422) | `if(budget.count != 0) { unsorted += budget.count; }` | branch-specific rejection/error | [ ] |
| 2450 | `trsort` (src/dictBuilder/divsufsort.c:1430) | `if(skip != 0) { *(first + skip) = skip; }` | branch-specific rejection/error | [ ] |
| 2451 | `trsort` (src/dictBuilder/divsufsort.c:1431) | `if(unsorted == 0) { break; }` | branch-specific rejection/error | [ ] |
| 2452 | `sort_typeBstar` (src/dictBuilder/divsufsort.c:1466) | `if(0 <= i) {` | branch-specific rejection/error | [ ] |
| 2453 | `sort_typeBstar` (src/dictBuilder/divsufsort.c:1495) | `if(0 < m) {` | branch-specific rejection/error | [ ] |
| 2454 | `sort_typeBstar` (src/dictBuilder/divsufsort.c:1519) | `if(0 < (l = j)) {` | branch-specific rejection/error | [ ] |
| 2455 | `sort_typeBstar` (src/dictBuilder/divsufsort.c:1523) | `if(--d1 <= d0) {` | branch-specific rejection/error | [ ] |
| 2456 | `sort_typeBstar` (src/dictBuilder/divsufsort.c:1525) | `if(--d0 < 0) { break; }` | branch-specific rejection/error | [ ] |
| 2457 | `sort_typeBstar` (src/dictBuilder/divsufsort.c:1531) | `if(l == 0) { break; }` | branch-specific rejection/error | [ ] |
| 2458 | `sort_typeBstar` (src/dictBuilder/divsufsort.c:1543) | `if(1 < (j - i)) {` | branch-specific rejection/error | [ ] |
| 2459 | `sort_typeBstar` (src/dictBuilder/divsufsort.c:1555) | `if(1 < (j - i)) {` | branch-specific rejection/error | [ ] |
| 2460 | `sort_typeBstar` (src/dictBuilder/divsufsort.c:1565) | `if(0 <= SA[i]) {` | branch-specific rejection/error | [ ] |
| 2461 | `sort_typeBstar` (src/dictBuilder/divsufsort.c:1569) | `if(i <= 0) { break; }` | branch-specific rejection/error | [ ] |
| 2462 | `sort_typeBstar` (src/dictBuilder/divsufsort.c:1582) | `if(0 <= i) {` | branch-specific rejection/error | [ ] |
| 2463 | `construct_SA` (src/dictBuilder/divsufsort.c:1620) | `if(0 < m) {` | branch-specific rejection/error | [ ] |
| 2464 | `construct_SA` (src/dictBuilder/divsufsort.c:1629) | `if(0 < (s = *j)) {` | branch-specific rejection/error | [ ] |
| 2465 | `construct_SA` (src/dictBuilder/divsufsort.c:1630) | `assert(T[s] == c1);` | assertion failure | [ ] |
| 2466 | `construct_SA` (src/dictBuilder/divsufsort.c:1631) | `assert(((s + 1) < n) && (T[s] <= T[s + 1]));` | assertion failure | [ ] |
| 2467 | `construct_SA` (src/dictBuilder/divsufsort.c:1632) | `assert(T[s - 1] <= T[s]);` | assertion failure | [ ] |
| 2468 | `construct_SA` (src/dictBuilder/divsufsort.c:1635) | `if((0 < s) && (T[s - 1] > c0)) { s = ~s; }` | branch-specific rejection/error | [ ] |
| 2469 | `construct_SA` (src/dictBuilder/divsufsort.c:1637) | `if(0 <= c2) { BUCKET_B(c2, c1) = k - SA; }` | branch-specific rejection/error | [ ] |
| 2470 | `construct_SA` (src/dictBuilder/divsufsort.c:1640) | `assert(k < j); assert(k != NULL);` | assertion failure | [ ] |
| 2471 | `construct_SA` (src/dictBuilder/divsufsort.c:1643) | `assert(((s == 0) && (T[s] == c1)) \|\| (s < 0));` | assertion failure | [ ] |
| 2472 | `construct_SA` (src/dictBuilder/divsufsort.c:1656) | `if(0 < (s = *i)) {` | branch-specific rejection/error | [ ] |
| 2473 | `construct_SA` (src/dictBuilder/divsufsort.c:1657) | `assert(T[s - 1] >= T[s]);` | assertion failure | [ ] |
| 2474 | `construct_SA` (src/dictBuilder/divsufsort.c:1659) | `if((s == 0) \|\| (T[s - 1] < c0)) { s = ~s; }` | branch-specific rejection/error | [ ] |
| 2475 | `construct_SA` (src/dictBuilder/divsufsort.c:1664) | `assert(i < k);` | assertion failure | [ ] |
| 2476 | `construct_SA` (src/dictBuilder/divsufsort.c:1667) | `assert(s < 0);` | assertion failure | [ ] |
| 2477 | `construct_BWT` (src/dictBuilder/divsufsort.c:1684) | `if(0 < m) {` | branch-specific rejection/error | [ ] |
| 2478 | `construct_BWT` (src/dictBuilder/divsufsort.c:1693) | `if(0 < (s = *j)) {` | branch-specific rejection/error | [ ] |
| 2479 | `construct_BWT` (src/dictBuilder/divsufsort.c:1694) | `assert(T[s] == c1);` | assertion failure | [ ] |
| 2480 | `construct_BWT` (src/dictBuilder/divsufsort.c:1695) | `assert(((s + 1) < n) && (T[s] <= T[s + 1]));` | assertion failure | [ ] |
| 2481 | `construct_BWT` (src/dictBuilder/divsufsort.c:1696) | `assert(T[s - 1] <= T[s]);` | assertion failure | [ ] |
| 2482 | `construct_BWT` (src/dictBuilder/divsufsort.c:1699) | `if((0 < s) && (T[s - 1] > c0)) { s = ~s; }` | branch-specific rejection/error | [ ] |
| 2483 | `construct_BWT` (src/dictBuilder/divsufsort.c:1701) | `if(0 <= c2) { BUCKET_B(c2, c1) = k - SA; }` | branch-specific rejection/error | [ ] |
| 2484 | `construct_BWT` (src/dictBuilder/divsufsort.c:1704) | `assert(k < j); assert(k != NULL);` | assertion failure | [ ] |
| 2485 | `construct_BWT` (src/dictBuilder/divsufsort.c:1706) | `} else if(s != 0) {` | branch-specific rejection/error | [ ] |
| 2486 | `construct_BWT` (src/dictBuilder/divsufsort.c:1710) | `assert(T[s] == c1);` | assertion failure | [ ] |
| 2487 | `construct_BWT` (src/dictBuilder/divsufsort.c:1723) | `if(0 < (s = *i)) {` | branch-specific rejection/error | [ ] |
| 2488 | `construct_BWT` (src/dictBuilder/divsufsort.c:1724) | `assert(T[s - 1] >= T[s]);` | assertion failure | [ ] |
| 2489 | `construct_BWT` (src/dictBuilder/divsufsort.c:1727) | `if((0 < s) && (T[s - 1] < c0)) { s = ~((int)T[s - 1]); }` | branch-specific rejection/error | [ ] |
| 2490 | `construct_BWT` (src/dictBuilder/divsufsort.c:1732) | `assert(i < k);` | assertion failure | [ ] |
| 2491 | `construct_BWT` (src/dictBuilder/divsufsort.c:1734) | `} else if(s != 0) {` | branch-specific rejection/error | [ ] |
| 2492 | `construct_BWT_indexes` (src/dictBuilder/divsufsort.c:1765) | `if(0 < m) {` | branch-specific rejection/error | [ ] |
| 2493 | `construct_BWT_indexes` (src/dictBuilder/divsufsort.c:1774) | `if(0 < (s = *j)) {` | branch-specific rejection/error | [ ] |
| 2494 | `construct_BWT_indexes` (src/dictBuilder/divsufsort.c:1775) | `assert(T[s] == c1);` | assertion failure | [ ] |
| 2495 | `construct_BWT_indexes` (src/dictBuilder/divsufsort.c:1776) | `assert(((s + 1) < n) && (T[s] <= T[s + 1]));` | assertion failure | [ ] |
| 2496 | `construct_BWT_indexes` (src/dictBuilder/divsufsort.c:1777) | `assert(T[s - 1] <= T[s]);` | assertion failure | [ ] |
| 2497 | `construct_BWT_indexes` (src/dictBuilder/divsufsort.c:1779) | `if ((s & mod) == 0) indexes[s / (mod + 1) - 1] = j - SA;` | branch-specific rejection/error | [ ] |
| 2498 | `construct_BWT_indexes` (src/dictBuilder/divsufsort.c:1783) | `if((0 < s) && (T[s - 1] > c0)) { s = ~s; }` | branch-specific rejection/error | [ ] |
| 2499 | `construct_BWT_indexes` (src/dictBuilder/divsufsort.c:1785) | `if(0 <= c2) { BUCKET_B(c2, c1) = k - SA; }` | branch-specific rejection/error | [ ] |
| 2500 | `construct_BWT_indexes` (src/dictBuilder/divsufsort.c:1788) | `assert(k < j); assert(k != NULL);` | assertion failure | [ ] |
| 2501 | `construct_BWT_indexes` (src/dictBuilder/divsufsort.c:1790) | `} else if(s != 0) {` | branch-specific rejection/error | [ ] |
| 2502 | `construct_BWT_indexes` (src/dictBuilder/divsufsort.c:1794) | `assert(T[s] == c1);` | assertion failure | [ ] |
| 2503 | `construct_BWT_indexes` (src/dictBuilder/divsufsort.c:1804) | `if (T[n - 2] < c2) {` | branch-specific rejection/error | [ ] |
| 2504 | `construct_BWT_indexes` (src/dictBuilder/divsufsort.c:1805) | `if (((n - 1) & mod) == 0) indexes[(n - 1) / (mod + 1) - 1] = k - SA;` | branch-specific rejection/error | [ ] |
| 2505 | `construct_BWT_indexes` (src/dictBuilder/divsufsort.c:1814) | `if(0 < (s = *i)) {` | branch-specific rejection/error | [ ] |
| 2506 | `construct_BWT_indexes` (src/dictBuilder/divsufsort.c:1815) | `assert(T[s - 1] >= T[s]);` | assertion failure | [ ] |
| 2507 | `construct_BWT_indexes` (src/dictBuilder/divsufsort.c:1817) | `if ((s & mod) == 0) indexes[s / (mod + 1) - 1] = i - SA;` | branch-specific rejection/error | [ ] |
| 2508 | `construct_BWT_indexes` (src/dictBuilder/divsufsort.c:1825) | `assert(i < k);` | assertion failure | [ ] |
| 2509 | `construct_BWT_indexes` (src/dictBuilder/divsufsort.c:1826) | `if((0 < s) && (T[s - 1] < c0)) {` | branch-specific rejection/error | [ ] |
| 2510 | `construct_BWT_indexes` (src/dictBuilder/divsufsort.c:1827) | `if ((s & mod) == 0) indexes[s / (mod + 1) - 1] = k - SA;` | branch-specific rejection/error | [ ] |
| 2511 | `construct_BWT_indexes` (src/dictBuilder/divsufsort.c:1831) | `} else if(s != 0) {` | branch-specific rejection/error | [ ] |
| 2512 | `divsufsort` (src/dictBuilder/divsufsort.c:1853) | `if((T == NULL) \|\| (SA == NULL) \|\| (n < 0)) { return -1; }` | `-1` | [ ] |
| 2513 | `divsufsort` (src/dictBuilder/divsufsort.c:1854) | `else if(n == 0) { return 0; }` | branch-specific rejection/error | [ ] |
| 2514 | `divsufsort` (src/dictBuilder/divsufsort.c:1856) | `else if(n == 2) { m = (T[0] < T[1]); SA[m ^ 1] = 0, SA[m] = 1; return 0; }` | branch-specific rejection/error | [ ] |
| 2515 | `divsufsort` (src/dictBuilder/divsufsort.c:1862) | `if((bucket_A != NULL) && (bucket_B != NULL)) {` | branch-specific rejection/error | [ ] |
| 2516 | `divbwt` (src/dictBuilder/divsufsort.c:1882) | `if((T == NULL) \|\| (U == NULL) \|\| (n < 0)) { return -1; }` | `-1` | [ ] |
| 2517 | `divbwt` (src/dictBuilder/divsufsort.c:1883) | `else if(n <= 1) { if(n == 1) { U[0] = T[0]; } return n; }` | branch-specific rejection/error | [ ] |
| 2518 | `divbwt` (src/dictBuilder/divsufsort.c:1885) | `if((B = A) == NULL) { B = (int *)malloc((size_t)(n + 1) * sizeof(int)); }` | branch-specific rejection/error | [ ] |
| 2519 | `divbwt` (src/dictBuilder/divsufsort.c:1890) | `if((B != NULL) && (bucket_A != NULL) && (bucket_B != NULL)) {` | branch-specific rejection/error | [ ] |
| 2520 | `divbwt` (src/dictBuilder/divsufsort.c:1893) | `if (num_indexes == NULL \|\| indexes == NULL) {` | branch-specific rejection/error | [ ] |
| 2521 | `divbwt` (src/dictBuilder/divsufsort.c:1910) | `if(A == NULL) { free(B); }` | branch-specific rejection/error | [ ] |
| 2522 | `(file scope)` (src/dictBuilder/fastcover.c:64) | `if (displayLevel >= l) {                                                     \` | branch-specific rejection/error | [ ] |
| 2523 | `(file scope)` (src/dictBuilder/fastcover.c:76) | `if (displayLevel >= l) {                                                     \` | branch-specific rejection/error | [ ] |
| 2524 | `(file scope)` (src/dictBuilder/fastcover.c:77) | `if ((clock() - g_time > g_refreshRate) \|\| (displayLevel >= 4)) {             \` | branch-specific rejection/error | [ ] |
| 2525 | `FASTCOVER_selectSegment` (src/dictBuilder/fastcover.c:184) | `if (segmentFreqs[idx] == 0) {` | branch-specific rejection/error | [ ] |
| 2526 | `FASTCOVER_selectSegment` (src/dictBuilder/fastcover.c:196) | `if (segmentFreqs[delIndex] == 0) {` | branch-specific rejection/error | [ ] |
| 2527 | `FASTCOVER_selectSegment` (src/dictBuilder/fastcover.c:204) | `if (activeSegment.score > bestSegment.score) {` | branch-specific rejection/error | [ ] |
| 2528 | `FASTCOVER_checkParameters` (src/dictBuilder/fastcover.c:233) | `if (parameters.d == 0 \|\| parameters.k == 0) {` | branch-specific rejection/error | [ ] |
| 2529 | `FASTCOVER_checkParameters` (src/dictBuilder/fastcover.c:241) | `if (parameters.k > maxDictSize) {` | branch-specific rejection/error | [ ] |
| 2530 | `FASTCOVER_checkParameters` (src/dictBuilder/fastcover.c:245) | `if (parameters.d > parameters.k) {` | branch-specific rejection/error | [ ] |
| 2531 | `FASTCOVER_checkParameters` (src/dictBuilder/fastcover.c:249) | `if (f > FASTCOVER_MAX_F \|\| f == 0) {` | branch-specific rejection/error | [ ] |
| 2532 | `FASTCOVER_checkParameters` (src/dictBuilder/fastcover.c:253) | `if (parameters.splitPoint <= 0 \|\| parameters.splitPoint > 1) {` | branch-specific rejection/error | [ ] |
| 2533 | `FASTCOVER_checkParameters` (src/dictBuilder/fastcover.c:257) | `if (accel > 10 \|\| accel == 0) {` | branch-specific rejection/error | [ ] |
| 2534 | `FASTCOVER_computeFrequency` (src/dictBuilder/fastcover.c:291) | `assert(ctx->nbTrainSamples >= 5);` | assertion failure | [ ] |
| 2535 | `FASTCOVER_computeFrequency` (src/dictBuilder/fastcover.c:292) | `assert(ctx->nbTrainSamples <= ctx->nbSamples);` | assertion failure | [ ] |
| 2536 | `FASTCOVER_ctx_init` (src/dictBuilder/fastcover.c:328) | `if (totalSamplesSize < MAX(d, sizeof(U64)) \|\|` | `ERROR(srcSize_wrong)` | [ ] |
| 2537 | `FASTCOVER_ctx_init` (src/dictBuilder/fastcover.c:332) | `return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 2538 | `FASTCOVER_ctx_init` (src/dictBuilder/fastcover.c:336) | `if (nbTrainSamples < 5) {` | `ERROR(srcSize_wrong)` | [ ] |
| 2539 | `FASTCOVER_ctx_init` (src/dictBuilder/fastcover.c:338) | `return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 2540 | `FASTCOVER_ctx_init` (src/dictBuilder/fastcover.c:342) | `if (nbTestSamples < 1) {` | `ERROR(srcSize_wrong)` | [ ] |
| 2541 | `FASTCOVER_ctx_init` (src/dictBuilder/fastcover.c:344) | `return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 2542 | `FASTCOVER_ctx_init` (src/dictBuilder/fastcover.c:366) | `if (ctx->offsets == NULL) {` | `ERROR(memory_allocation)` | [ ] |
| 2543 | `FASTCOVER_ctx_init` (src/dictBuilder/fastcover.c:369) | `return ERROR(memory_allocation);` | `ERROR(memory_allocation)` | [ ] |
| 2544 | `FASTCOVER_ctx_init` (src/dictBuilder/fastcover.c:375) | `assert(nbSamples >= 5);` | assertion failure | [ ] |
| 2545 | `FASTCOVER_ctx_init` (src/dictBuilder/fastcover.c:383) | `if (ctx->freqs == NULL) {` | `ERROR(memory_allocation)` | [ ] |
| 2546 | `FASTCOVER_ctx_init` (src/dictBuilder/fastcover.c:386) | `return ERROR(memory_allocation);` | `ERROR(memory_allocation)` | [ ] |
| 2547 | `FASTCOVER_buildDictionary` (src/dictBuilder/fastcover.c:430) | `if (segment.score == 0) {` | branch-specific rejection/error | [ ] |
| 2548 | `FASTCOVER_buildDictionary` (src/dictBuilder/fastcover.c:431) | `if (++zeroScoreRun >= maxZeroScoreRun) {` | branch-specific rejection/error | [ ] |
| 2549 | `FASTCOVER_buildDictionary` (src/dictBuilder/fastcover.c:440) | `if (segmentSize < parameters.d) {` | branch-specific rejection/error | [ ] |
| 2550 | `FASTCOVER_tryParameters` (src/dictBuilder/fastcover.c:480) | `size_t totalCompressedSize = ERROR(GENERIC);` | `ERROR(GENERIC)` | [ ] |
| 2551 | `FASTCOVER_tryParameters` (src/dictBuilder/fastcover.c:485) | `COVER_dictSelection_t selection = COVER_dictSelectionError(ERROR(GENERIC));` | `ERROR(GENERIC)` | [ ] |
| 2552 | `ZDICT_trainFromBuffer_fastCover` (src/dictBuilder/fastcover.c:571) | `return ERROR(parameter_outOfBound);` | `ERROR(parameter_outOfBound)` | [ ] |
| 2553 | `ZDICT_trainFromBuffer_fastCover` (src/dictBuilder/fastcover.c:573) | `if (nbSamples == 0) {` | `ERROR(srcSize_wrong)` | [ ] |
| 2554 | `ZDICT_trainFromBuffer_fastCover` (src/dictBuilder/fastcover.c:575) | `return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 2555 | `ZDICT_trainFromBuffer_fastCover` (src/dictBuilder/fastcover.c:577) | `if (dictBufferCapacity < ZDICT_DICTSIZE_MIN) {` | `ERROR(dstSize_tooSmall)` | [ ] |
| 2556 | `ZDICT_trainFromBuffer_fastCover` (src/dictBuilder/fastcover.c:580) | `return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [ ] |
| 2557 | `ZDICT_optimizeTrainFromBuffer_fastCover` (src/dictBuilder/fastcover.c:650) | `if (splitPoint <= 0 \|\| splitPoint > 1) {` | `ERROR(parameter_outOfBound)` | [ ] |
| 2558 | `ZDICT_optimizeTrainFromBuffer_fastCover` (src/dictBuilder/fastcover.c:652) | `return ERROR(parameter_outOfBound);` | `ERROR(parameter_outOfBound)` | [ ] |
| 2559 | `ZDICT_optimizeTrainFromBuffer_fastCover` (src/dictBuilder/fastcover.c:654) | `if (accel == 0 \|\| accel > FASTCOVER_MAX_ACCEL) {` | `ERROR(parameter_outOfBound)` | [ ] |
| 2560 | `ZDICT_optimizeTrainFromBuffer_fastCover` (src/dictBuilder/fastcover.c:656) | `return ERROR(parameter_outOfBound);` | `ERROR(parameter_outOfBound)` | [ ] |
| 2561 | `ZDICT_optimizeTrainFromBuffer_fastCover` (src/dictBuilder/fastcover.c:658) | `if (kMinK < kMaxD \|\| kMaxK < kMinK) {` | `ERROR(parameter_outOfBound)` | [ ] |
| 2562 | `ZDICT_optimizeTrainFromBuffer_fastCover` (src/dictBuilder/fastcover.c:660) | `return ERROR(parameter_outOfBound);` | `ERROR(parameter_outOfBound)` | [ ] |
| 2563 | `ZDICT_optimizeTrainFromBuffer_fastCover` (src/dictBuilder/fastcover.c:662) | `if (nbSamples == 0) {` | `ERROR(srcSize_wrong)` | [ ] |
| 2564 | `ZDICT_optimizeTrainFromBuffer_fastCover` (src/dictBuilder/fastcover.c:664) | `return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 2565 | `ZDICT_optimizeTrainFromBuffer_fastCover` (src/dictBuilder/fastcover.c:666) | `if (dictBufferCapacity < ZDICT_DICTSIZE_MIN) {` | `ERROR(dstSize_tooSmall)` | [ ] |
| 2566 | `ZDICT_optimizeTrainFromBuffer_fastCover` (src/dictBuilder/fastcover.c:669) | `return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [ ] |
| 2567 | `ZDICT_optimizeTrainFromBuffer_fastCover` (src/dictBuilder/fastcover.c:671) | `if (nbThreads > 1) {` | `ERROR(memory_allocation)` | [ ] |
| 2568 | `ZDICT_optimizeTrainFromBuffer_fastCover` (src/dictBuilder/fastcover.c:674) | `return ERROR(memory_allocation);` | `ERROR(memory_allocation)` | [ ] |
| 2569 | `ZDICT_optimizeTrainFromBuffer_fastCover` (src/dictBuilder/fastcover.c:715) | `return ERROR(memory_allocation);` | `ERROR(memory_allocation)` | [ ] |
| 2570 | `ZDICT_optimizeTrainFromBuffer_fastCover` (src/dictBuilder/fastcover.c:728) | `if (!FASTCOVER_checkParameters(data->parameters, dictBufferCapacity,` | branch-specific rejection/error | [ ] |
| 2571 | `ZDICT_printHex` (src/dictBuilder/zdict.c:89) | `if (c<32 \|\| c>126) c = '.';   /* non-printable char */` | branch-specific rejection/error | [ ] |
| 2572 | `ZDICT_getDictID` (src/dictBuilder/zdict.c:104) | `if (dictSize < 8) return 0;` | branch-specific rejection/error | [ ] |
| 2573 | `ZDICT_getDictHeaderSize` (src/dictBuilder/zdict.c:112) | `if (dictSize <= 8 \|\| MEM_readLE32(dictBuffer) != ZSTD_MAGIC_DICTIONARY) return ERROR(dictionary_corrupted);` | `ERROR(dictionary_corrupted)` | [ ] |
| 2574 | `ZDICT_getDictHeaderSize` (src/dictBuilder/zdict.c:117) | `headerSize = ERROR(memory_allocation);` | `ERROR(memory_allocation)` | [ ] |
| 2575 | `ZDICT_analyzePos` (src/dictBuilder/zdict.c:213) | `if (length >=MINMATCHLENGTH) start--;` | branch-specific rejection/error | [ ] |
| 2576 | `ZDICT_analyzePos` (src/dictBuilder/zdict.c:218) | `if (end-start < minRatio) {` | branch-specific rejection/error | [ ] |
| 2577 | `ZDICT_analyzePos` (src/dictBuilder/zdict.c:243) | `if (currentCount > selectedCount) {` | branch-specific rejection/error | [ ] |
| 2578 | `ZDICT_analyzePos` (src/dictBuilder/zdict.c:253) | `if (currentCount > selectedCount) {  /* for last */` | branch-specific rejection/error | [ ] |
| 2579 | `ZDICT_analyzePos` (src/dictBuilder/zdict.c:258) | `if (selectedCount < minRatio)` | branch-specific rejection/error | [ ] |
| 2580 | `ZDICT_analyzePos` (src/dictBuilder/zdict.c:275) | `if (length >= LLIMIT) length = LLIMIT-1;` | branch-specific rejection/error | [ ] |
| 2581 | `ZDICT_analyzePos` (src/dictBuilder/zdict.c:284) | `if (length >= LLIMIT) length = LLIMIT - 1;` | branch-specific rejection/error | [ ] |
| 2582 | `ZDICT_analyzePos` (src/dictBuilder/zdict.c:286) | `if (length >= MINMATCHLENGTH) start--;` | branch-specific rejection/error | [ ] |
| 2583 | `ZDICT_analyzePos` (src/dictBuilder/zdict.c:296) | `for (i=LLIMIT-1; i>=MINMATCHLENGTH; i--) if (cumulLength[i]>=minRatio) break;` | branch-specific rejection/error | [ ] |
| 2584 | `ZDICT_analyzePos` (src/dictBuilder/zdict.c:305) | `if (maxLength < MINMATCHLENGTH) return solution;   /* skip : no long-enough solution */` | branch-specific rejection/error | [ ] |
| 2585 | `ZDICT_analyzePos` (src/dictBuilder/zdict.c:328) | `if (length > solution.length) length = solution.length;` | branch-specific rejection/error | [ ] |
| 2586 | `ZDICT_tryMerge` (src/dictBuilder/zdict.c:365) | `if ((table[u].pos > elt.pos) && (table[u].pos <= eltEnd)) {  /* overlap, existing > new */` | branch-specific rejection/error | [ ] |
| 2587 | `ZDICT_tryMerge` (src/dictBuilder/zdict.c:384) | `if ((table[u].pos + table[u].length >= elt.pos) && (table[u].pos < elt.pos)) {  /* overlap, existing < new */` | branch-specific rejection/error | [ ] |
| 2588 | `ZDICT_tryMerge` (src/dictBuilder/zdict.c:388) | `if (addedLength > 0) {   /* otherwise, elt fully included into existing */` | branch-specific rejection/error | [ ] |
| 2589 | `ZDICT_insertDictItem` (src/dictBuilder/zdict.c:444) | `if (nextElt >= maxSize) nextElt = maxSize-1;` | branch-specific rejection/error | [ ] |
| 2590 | `ZDICT_trainBuffer_legacy` (src/dictBuilder/zdict.c:482) | `if (notificationLevel>=l) {                            \` | branch-specific rejection/error | [ ] |
| 2591 | `ZDICT_trainBuffer_legacy` (src/dictBuilder/zdict.c:483) | `if (ZDICT_clockSpan(displayClock) > refreshRate) { \` | branch-specific rejection/error | [ ] |
| 2592 | `ZDICT_trainBuffer_legacy` (src/dictBuilder/zdict.c:487) | `if (notificationLevel>=4) fflush(stderr);          \` | branch-specific rejection/error | [ ] |
| 2593 | `ZDICT_trainBuffer_legacy` (src/dictBuilder/zdict.c:494) | `result = ERROR(memory_allocation);` | `ERROR(memory_allocation)` | [ ] |
| 2594 | `ZDICT_trainBuffer_legacy` (src/dictBuilder/zdict.c:497) | `if (minRatio < MINRATIO) minRatio = MINRATIO;` | branch-specific rejection/error | [ ] |
| 2595 | `ZDICT_trainBuffer_legacy` (src/dictBuilder/zdict.c:501) | `if (bufferSize > ZDICT_MAX_SAMPLES_SIZE) DISPLAYLEVEL(3, "sample set too large : reduced to %u MB ...\n", (unsigned)(ZDICT_MAX_SAMPLES_SIZE>>20));` | branch-specific rejection/error | [ ] |
| 2596 | `ZDICT_trainBuffer_legacy` (src/dictBuilder/zdict.c:507) | `if (divSuftSortResult != 0) { result = ERROR(GENERIC); goto _cleanup; }` | `ERROR(GENERIC)` | [ ] |
| 2597 | `ZDICT_trainBuffer_legacy` (src/dictBuilder/zdict.c:529) | `if (solution.length==0) { cursor++; continue; }` | branch-specific rejection/error | [ ] |
| 2598 | `ZDICT_countEStats` (src/dictBuilder/zdict.c:574) | `if (srcSize > blockSizeMax) srcSize = blockSizeMax;   /* protection vs large samples */` | branch-specific rejection/error | [ ] |
| 2599 | `ZDICT_countEStats` (src/dictBuilder/zdict.c:582) | `if (cSize) {  /* if == 0; block is not compressible */` | branch-specific rejection/error | [ ] |
| 2600 | `ZDICT_countEStats` (src/dictBuilder/zdict.c:610) | `if (nbSeq >= 2) { /* rep offsets */` | branch-specific rejection/error | [ ] |
| 2601 | `ZDICT_countEStats` (src/dictBuilder/zdict.c:614) | `if (offset1 >= MAXREPOFFSET) offset1 = 0;` | branch-specific rejection/error | [ ] |
| 2602 | `ZDICT_countEStats` (src/dictBuilder/zdict.c:615) | `if (offset2 >= MAXREPOFFSET) offset2 = 0;` | branch-specific rejection/error | [ ] |
| 2603 | `ZDICT_insertSortCount` (src/dictBuilder/zdict.c:638) | `if (table[u-1].count >= table[u].count) break;` | branch-specific rejection/error | [ ] |
| 2604 | `ZDICT_analyzeEntropy` (src/dictBuilder/zdict.c:688) | `if (offcodeMax>OFFCODE_MAX) { eSize = ERROR(dictionaryCreation_failed); goto _cleanup; }   /* too large dictionary */` | `ERROR(dictionaryCreation_failed)` | [ ] |
| 2605 | `ZDICT_analyzeEntropy` (src/dictBuilder/zdict.c:696) | `if (compressionLevel==0) compressionLevel = ZSTD_CLEVEL_DEFAULT;` | branch-specific rejection/error | [ ] |
| 2606 | `ZDICT_analyzeEntropy` (src/dictBuilder/zdict.c:703) | `eSize = ERROR(memory_allocation);` | `ERROR(memory_allocation)` | [ ] |
| 2607 | `ZDICT_analyzeEntropy` (src/dictBuilder/zdict.c:717) | `if (notificationLevel >= 4) {` | branch-specific rejection/error | [ ] |
| 2608 | `ZDICT_analyzeEntropy` (src/dictBuilder/zdict.c:735) | `assert(maxNbBits==9);` | assertion failure | [ ] |
| 2609 | `ZDICT_analyzeEntropy` (src/dictBuilder/zdict.c:819) | `if (maxDstSize<12) {` | `ERROR(dstSize_tooSmall)` | [ ] |
| 2610 | `ZDICT_analyzeEntropy` (src/dictBuilder/zdict.c:820) | `eSize = ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [ ] |
| 2611 | `ZDICT_finalizeDictionary` (src/dictBuilder/zdict.c:874) | `if (dictBufferCapacity < dictContentSize) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [ ] |
| 2612 | `ZDICT_finalizeDictionary` (src/dictBuilder/zdict.c:875) | `if (dictBufferCapacity < ZDICT_DICTSIZE_MIN) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [ ] |
| 2613 | `ZDICT_finalizeDictionary` (src/dictBuilder/zdict.c:899) | `if (hSize + dictContentSize > dictBufferCapacity) {` | branch-specific rejection/error | [ ] |
| 2614 | `ZDICT_finalizeDictionary` (src/dictBuilder/zdict.c:904) | `if (dictContentSize < minContentSize) {` | `ERROR(dstSize_tooSmall)` | [ ] |
| 2615 | `ZDICT_finalizeDictionary` (src/dictBuilder/zdict.c:905) | `RETURN_ERROR_IF(hSize + minContentSize > dictBufferCapacity, dstSize_tooSmall,` | `ERROR(dstSize_tooSmall)` | [ ] |
| 2616 | `ZDICT_finalizeDictionary` (src/dictBuilder/zdict.c:923) | `assert(dictSize <= dictBufferCapacity);` | assertion failure | [ ] |
| 2617 | `ZDICT_finalizeDictionary` (src/dictBuilder/zdict.c:924) | `assert(outDictContent + dictContentSize == (BYTE*)dictBuffer + dictSize);` | assertion failure | [ ] |
| 2618 | `ZDICT_addEntropyTablesFromBuffer_advanced` (src/dictBuilder/zdict.c:969) | `if (hSize + dictContentSize < dictBufferCapacity)` | branch-specific rejection/error | [ ] |
| 2619 | `ZDICT_trainFromBuffer_unsafe_legacy` (src/dictBuilder/zdict.c:993) | `if (!dictList) return ERROR(memory_allocation);` | `ERROR(memory_allocation)` | [ ] |
| 2620 | `ZDICT_trainFromBuffer_unsafe_legacy` (src/dictBuilder/zdict.c:994) | `if (maxDictSize < ZDICT_DICTSIZE_MIN) { free(dictList); return ERROR(dstSize_tooSmall); }   /* requested dictionary size is too small */` | `ERROR(dstSize_tooSmall)` | [ ] |
| 2621 | `ZDICT_trainFromBuffer_unsafe_legacy` (src/dictBuilder/zdict.c:995) | `if (samplesBuffSize < ZDICT_MIN_SAMPLES_SIZE) { free(dictList); return ERROR(dictionaryCreation_failed); }   /* not enough source to create dictionary */` | `ERROR(dictionaryCreation_failed)` | [ ] |
| 2622 | `ZDICT_trainFromBuffer_unsafe_legacy` (src/dictBuilder/zdict.c:1007) | `if (params.zParams.notificationLevel>= 3) {` | branch-specific rejection/error | [ ] |
| 2623 | `ZDICT_trainFromBuffer_unsafe_legacy` (src/dictBuilder/zdict.c:1017) | `if ((pos > samplesBuffSize) \|\| ((pos + length) > samplesBuffSize)) {` | `ERROR(GENERIC)` | [ ] |
| 2624 | `ZDICT_trainFromBuffer_unsafe_legacy` (src/dictBuilder/zdict.c:1019) | `return ERROR(GENERIC);   /* should never happen */` | `ERROR(GENERIC)` | [ ] |
| 2625 | `ZDICT_trainFromBuffer_unsafe_legacy` (src/dictBuilder/zdict.c:1030) | `if (dictContentSize < ZDICT_CONTENTSIZE_MIN) { free(dictList); return ERROR(dictionaryCreation_failed); }   /* dictionary content too small */` | `ERROR(dictionaryCreation_failed)` | [ ] |
| 2626 | `ZDICT_trainFromBuffer_unsafe_legacy` (src/dictBuilder/zdict.c:1031) | `if (dictContentSize < targetDictSize/4) {` | branch-specific rejection/error | [ ] |
| 2627 | `ZDICT_trainFromBuffer_unsafe_legacy` (src/dictBuilder/zdict.c:1033) | `if (samplesBuffSize < 10 * targetDictSize)` | branch-specific rejection/error | [ ] |
| 2628 | `ZDICT_trainFromBuffer_unsafe_legacy` (src/dictBuilder/zdict.c:1035) | `if (minRep > MINRATIO) {` | branch-specific rejection/error | [ ] |
| 2629 | `ZDICT_trainFromBuffer_unsafe_legacy` (src/dictBuilder/zdict.c:1041) | `if ((dictContentSize > targetDictSize*3) && (nbSamples > 2*MINRATIO) && (selectivity>1)) {` | branch-specific rejection/error | [ ] |
| 2630 | `ZDICT_trainFromBuffer_unsafe_legacy` (src/dictBuilder/zdict.c:1054) | `if (currentSize > targetDictSize) { currentSize -= dictList[n].length; break; }` | branch-specific rejection/error | [ ] |
| 2631 | `ZDICT_trainFromBuffer_unsafe_legacy` (src/dictBuilder/zdict.c:1066) | `if (ptr<(BYTE*)dictBuffer) { free(dictList); return ERROR(GENERIC); }   /* should not happen */` | `ERROR(GENERIC)` | [ ] |
| 2632 | `ZDICT_trainFromBuffer_legacy` (src/dictBuilder/zdict.c:1091) | `if (sBuffSize < ZDICT_MIN_SAMPLES_SIZE) return 0;   /* not enough content => no dictionary */` | `ERROR(memory_allocation)` | [ ] |
| 2633 | `ZDICT_trainFromBuffer_legacy` (src/dictBuilder/zdict.c:1094) | `if (!newBuff) return ERROR(memory_allocation);` | `ERROR(memory_allocation)` | [ ] |
| 2634 | `FSE_buildDTable` (src/legacy/zstd_v01.c:374) | `if (maxSymbolValue > FSE_MAX_SYMBOL_VALUE) return (size_t)-FSE_ERROR_maxSymbolValue_tooLarge;` | branch-specific rejection/error | [ ] |
| 2635 | `FSE_buildDTable` (src/legacy/zstd_v01.c:375) | `if (tableLog > FSE_MAX_TABLELOG) return (size_t)-FSE_ERROR_tableLog_tooLarge;` | branch-specific rejection/error | [ ] |
| 2636 | `FSE_buildDTable` (src/legacy/zstd_v01.c:388) | `if (normalizedCounter[s] >= largeLimit) noLarge=0;` | branch-specific rejection/error | [ ] |
| 2637 | `FSE_buildDTable` (src/legacy/zstd_v01.c:405) | `if (position!=0) return (size_t)-FSE_ERROR_GENERIC;   /* position must reach all cells once, otherwise normalizedCounter is incorrect */` | branch-specific rejection/error | [ ] |
| 2638 | `FSE_readNCount` (src/legacy/zstd_v01.c:454) | `if (hbSize < 4) return (size_t)-FSE_ERROR_srcSize_wrong;` | branch-specific rejection/error | [ ] |
| 2639 | `FSE_readNCount` (src/legacy/zstd_v01.c:457) | `if (nbBits > FSE_TABLELOG_ABSOLUTE_MAX) return (size_t)-FSE_ERROR_tableLog_tooLarge;` | branch-specific rejection/error | [ ] |
| 2640 | `FSE_readNCount` (src/legacy/zstd_v01.c:473) | `if (ip < iend-5)` | branch-specific rejection/error | [ ] |
| 2641 | `FSE_readNCount` (src/legacy/zstd_v01.c:492) | `if (n0 > *maxSVPtr) return (size_t)-FSE_ERROR_maxSymbolValue_tooSmall;` | branch-specific rejection/error | [ ] |
| 2642 | `FSE_readNCount` (src/legacy/zstd_v01.c:494) | `if ((ip <= iend-7) \|\| (ip + (bitCount>>3) <= iend-4))` | branch-specific rejection/error | [ ] |
| 2643 | `FSE_readNCount` (src/legacy/zstd_v01.c:507) | `if ((bitStream & (threshold-1)) < (U32)max)` | branch-specific rejection/error | [ ] |
| 2644 | `FSE_readNCount` (src/legacy/zstd_v01.c:515) | `if (count >= threshold) count -= max;` | branch-specific rejection/error | [ ] |
| 2645 | `FSE_readNCount` (src/legacy/zstd_v01.c:530) | `if ((ip <= iend-7) \|\| (ip + (bitCount>>3) <= iend-4))` | branch-specific rejection/error | [ ] |
| 2646 | `FSE_readNCount` (src/legacy/zstd_v01.c:548) | `if ((size_t)(ip-istart) > hbSize) return (size_t)-FSE_ERROR_srcSize_wrong;` | branch-specific rejection/error | [ ] |
| 2647 | `FSE_buildDTable_raw` (src/legacy/zstd_v01.c:584) | `if (nbBits < 1) return (size_t)-FSE_ERROR_GENERIC;             /* min size */` | branch-specific rejection/error | [ ] |
| 2648 | `FSE_initDStream` (src/legacy/zstd_v01.c:608) | `if (srcSize < 1) return (size_t)-FSE_ERROR_srcSize_wrong;` | branch-specific rejection/error | [ ] |
| 2649 | `FSE_initDStream` (src/legacy/zstd_v01.c:610) | `if (srcSize >=  sizeof(size_t))` | branch-specific rejection/error | [ ] |
| 2650 | `FSE_initDStream` (src/legacy/zstd_v01.c:617) | `if (contain32 == 0) return (size_t)-FSE_ERROR_GENERIC;   /* stop bit not present */` | branch-specific rejection/error | [ ] |
| 2651 | `FSE_initDStream` (src/legacy/zstd_v01.c:643) | `if (contain32 == 0) return (size_t)-FSE_ERROR_GENERIC;   /* stop bit not present */` | branch-specific rejection/error | [ ] |
| 2652 | `FSE_reloadDStream` (src/legacy/zstd_v01.c:700) | `if (bitD->bitsConsumed > (sizeof(bitD->bitContainer)*8))  /* should never happen */` | branch-specific rejection/error | [ ] |
| 2653 | `FSE_reloadDStream` (src/legacy/zstd_v01.c:703) | `if (bitD->ptr >= bitD->start + sizeof(bitD->bitContainer))` | branch-specific rejection/error | [ ] |
| 2654 | `FSE_reloadDStream` (src/legacy/zstd_v01.c:710) | `if (bitD->ptr == bitD->start)` | branch-specific rejection/error | [ ] |
| 2655 | `FSE_reloadDStream` (src/legacy/zstd_v01.c:712) | `if (bitD->bitsConsumed < sizeof(bitD->bitContainer)*8) return FSE_DStream_endOfBuffer;` | branch-specific rejection/error | [ ] |
| 2656 | `FSE_reloadDStream` (src/legacy/zstd_v01.c:718) | `if (bitD->ptr - nbBytes < bitD->start)` | branch-specific rejection/error | [ ] |
| 2657 | `FSE_decompress_usingDTable_generic` (src/legacy/zstd_v01.c:805) | `if (FSE_MAX_TABLELOG*2+7 > sizeof(bitD.bitContainer)*8)    /* This test must be static */` | branch-specific rejection/error | [ ] |
| 2658 | `FSE_decompress_usingDTable_generic` (src/legacy/zstd_v01.c:810) | `if (FSE_MAX_TABLELOG*4+7 > sizeof(bitD.bitContainer)*8)    /* This test must be static */` | branch-specific rejection/error | [ ] |
| 2659 | `FSE_decompress_usingDTable_generic` (src/legacy/zstd_v01.c:811) | `{ if (FSE_reloadDStream(&bitD) > FSE_DStream_unfinished) { op+=2; break; } }` | branch-specific rejection/error | [ ] |
| 2660 | `FSE_decompress_usingDTable_generic` (src/legacy/zstd_v01.c:815) | `if (FSE_MAX_TABLELOG*2+7 > sizeof(bitD.bitContainer)*8)    /* This test must be static */` | branch-specific rejection/error | [ ] |
| 2661 | `FSE_decompress_usingDTable_generic` (src/legacy/zstd_v01.c:825) | `if ( (FSE_reloadDStream(&bitD)>FSE_DStream_completed) \|\| (op==omax) \|\| (FSE_endOfDStream(&bitD) && (fast \|\| FSE_endOfDState(&state1))) )` | branch-specific rejection/error | [ ] |
| 2662 | `FSE_decompress_usingDTable_generic` (src/legacy/zstd_v01.c:830) | `if ( (FSE_reloadDStream(&bitD)>FSE_DStream_completed) \|\| (op==omax) \|\| (FSE_endOfDStream(&bitD) && (fast \|\| FSE_endOfDState(&state2))) )` | branch-specific rejection/error | [ ] |
| 2663 | `FSE_decompress` (src/legacy/zstd_v01.c:869) | `if (cSrcSize<2) return (size_t)-FSE_ERROR_srcSize_wrong;   /* too small input size */` | branch-specific rejection/error | [ ] |
| 2664 | `FSE_decompress` (src/legacy/zstd_v01.c:874) | `if (errorCode >= cSrcSize) return (size_t)-FSE_ERROR_srcSize_wrong;   /* too small input size */` | branch-specific rejection/error | [ ] |
| 2665 | `HUF_readDTable` (src/legacy/zstd_v01.c:938) | `if (iSize >= 128)  /* special header */` | branch-specific rejection/error | [ ] |
| 2666 | `HUF_readDTable` (src/legacy/zstd_v01.c:940) | `if (iSize >= (242))   /* RLE */` | branch-specific rejection/error | [ ] |
| 2667 | `HUF_readDTable` (src/legacy/zstd_v01.c:951) | `if (iSize+1 > srcSize) return (size_t)-FSE_ERROR_srcSize_wrong;` | branch-specific rejection/error | [ ] |
| 2668 | `HUF_readDTable` (src/legacy/zstd_v01.c:962) | `if (iSize+1 > srcSize) return (size_t)-FSE_ERROR_srcSize_wrong;` | branch-specific rejection/error | [ ] |
| 2669 | `HUF_readDTable` (src/legacy/zstd_v01.c:972) | `if (huffWeight[n] >= HUF_ABSOLUTEMAX_TABLELOG) return (size_t)-FSE_ERROR_corruptionDetected;` | branch-specific rejection/error | [ ] |
| 2670 | `HUF_readDTable` (src/legacy/zstd_v01.c:976) | `if (weightTotal == 0) return (size_t)-FSE_ERROR_corruptionDetected;` | branch-specific rejection/error | [ ] |
| 2671 | `HUF_readDTable` (src/legacy/zstd_v01.c:980) | `if (maxBits > DTable[0]) return (size_t)-FSE_ERROR_tableLog_tooLarge;   /* DTable is too small */` | branch-specific rejection/error | [ ] |
| 2672 | `HUF_readDTable` (src/legacy/zstd_v01.c:993) | `if ((rankVal[1] < 2) \|\| (rankVal[1] & 1)) return (size_t)-FSE_ERROR_corruptionDetected;   /* by construction : at least 2 elts of rank 1, must be even */` | branch-specific rejection/error | [ ] |
| 2673 | `HUF_decompress_usingDTable` (src/legacy/zstd_v01.c:1034) | `if (cSrcSize < 6) return (size_t)-FSE_ERROR_srcSize_wrong;` | branch-specific rejection/error | [ ] |
| 2674 | `HUF_decompress_usingDTable` (src/legacy/zstd_v01.c:1060) | `if (length1+length2+length3+6 >= cSrcSize) return (size_t)-FSE_ERROR_srcSize_wrong;` | branch-specific rejection/error | [ ] |
| 2675 | `HUF_decompress_usingDTable` (src/legacy/zstd_v01.c:1082) | `if (FSE_32bits() && (HUF_MAX_TABLELOG>12)) FSE_reloadDStream(&Dstream)` | branch-specific rejection/error | [ ] |
| 2676 | `HUF_decompress` (src/legacy/zstd_v01.c:1141) | `if (errorCode >= cSrcSize) return (size_t)-FSE_ERROR_srcSize_wrong;` | branch-specific rejection/error | [ ] |
| 2677 | `ZSTDv01_getcBlockSize` (src/legacy/zstd_v01.c:1431) | `if (srcSize < 3) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 2678 | `ZSTDv01_getcBlockSize` (src/legacy/zstd_v01.c:1439) | `if (bpPtr->blockType == bt_end) return 0;` | branch-specific rejection/error | [ ] |
| 2679 | `ZSTDv01_getcBlockSize` (src/legacy/zstd_v01.c:1440) | `if (bpPtr->blockType == bt_rle) return 1;` | branch-specific rejection/error | [ ] |
| 2680 | `ZSTD_copyUncompressedBlock` (src/legacy/zstd_v01.c:1447) | `if (srcSize > maxDstSize) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [ ] |
| 2681 | `ZSTD_copyUncompressedBlock` (src/legacy/zstd_v01.c:1448) | `if (srcSize > 0) {` | branch-specific rejection/error | [ ] |
| 2682 | `ZSTD_decompressLiterals` (src/legacy/zstd_v01.c:1466) | `if (srcSize <= 3) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 2683 | `ZSTD_decompressLiterals` (src/legacy/zstd_v01.c:1473) | `if (litSize > maxDstSize) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [ ] |
| 2684 | `ZSTD_decompressLiterals` (src/legacy/zstd_v01.c:1475) | `if (FSE_isError(errorCode)) return ERROR(GENERIC);` | `ERROR(GENERIC)` | [ ] |
| 2685 | `ZSTDv01_decodeLiteralsBlock` (src/legacy/zstd_v01.c:1493) | `if (litcSize > srcSize - ZSTD_blockHeaderSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 2686 | `ZSTDv01_decodeLiteralsBlock` (src/legacy/zstd_v01.c:1506) | `if (rleSize>maxDstSize) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [ ] |
| 2687 | `ZSTDv01_decodeLiteralsBlock` (src/legacy/zstd_v01.c:1507) | `if (!srcSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 2688 | `ZSTDv01_decodeLiteralsBlock` (src/legacy/zstd_v01.c:1508) | `if (rleSize > 0) {` | branch-specific rejection/error | [ ] |
| 2689 | `ZSTDv01_decodeLiteralsBlock` (src/legacy/zstd_v01.c:1527) | `return ERROR(GENERIC);` | `ERROR(GENERIC)` | [ ] |
| 2690 | `ZSTDv01_decodeSeqHeaders` (src/legacy/zstd_v01.c:1546) | `if (srcSize < 5) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 2691 | `ZSTDv01_decodeSeqHeaders` (src/legacy/zstd_v01.c:1570) | `if (ip > iend-3) return ERROR(srcSize_wrong); /* min : all 3 are "raw", hence no header, but at least xxLog bits per type */` | `ERROR(srcSize_wrong)` | [ ] |
| 2692 | `ZSTDv01_decodeSeqHeaders` (src/legacy/zstd_v01.c:1589) | `if (FSE_isError(headerSize)) return ERROR(GENERIC);` | `ERROR(GENERIC)` | [ ] |
| 2693 | `ZSTDv01_decodeSeqHeaders` (src/legacy/zstd_v01.c:1590) | `if (LLlog > LLFSELog) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 2694 | `ZSTDv01_decodeSeqHeaders` (src/legacy/zstd_v01.c:1599) | `if (ip > iend-2) return ERROR(srcSize_wrong); /* min : "raw", hence no header, but at least xxLog bits */` | `ERROR(srcSize_wrong)` | [ ] |
| 2695 | `ZSTDv01_decodeSeqHeaders` (src/legacy/zstd_v01.c:1607) | `if (FSE_isError(headerSize)) return ERROR(GENERIC);` | `ERROR(GENERIC)` | [ ] |
| 2696 | `ZSTDv01_decodeSeqHeaders` (src/legacy/zstd_v01.c:1608) | `if (Offlog > OffFSELog) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 2697 | `ZSTDv01_decodeSeqHeaders` (src/legacy/zstd_v01.c:1617) | `if (ip > iend-2) return ERROR(srcSize_wrong); /* min : "raw", hence no header, but at least xxLog bits */` | `ERROR(srcSize_wrong)` | [ ] |
| 2698 | `ZSTDv01_decodeSeqHeaders` (src/legacy/zstd_v01.c:1625) | `if (FSE_isError(headerSize)) return ERROR(GENERIC);` | `ERROR(GENERIC)` | [ ] |
| 2699 | `ZSTDv01_decodeSeqHeaders` (src/legacy/zstd_v01.c:1626) | `if (MLlog > MLFSELog) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 2700 | `ZSTD_decodeSequence` (src/legacy/zstd_v01.c:1668) | `if (add < 255) litLength += add;` | branch-specific rejection/error | [ ] |
| 2701 | `ZSTD_decodeSequence` (src/legacy/zstd_v01.c:1671) | `if (dumps<=(de-3))` | branch-specific rejection/error | [ ] |
| 2702 | `ZSTD_decodeSequence` (src/legacy/zstd_v01.c:1683) | `if (ZSTD_32bits()) FSE_reloadDStream(&(seqState->DStream));` | branch-specific rejection/error | [ ] |
| 2703 | `ZSTD_decodeSequence` (src/legacy/zstd_v01.c:1685) | `if (offsetCode==0) nbBits = 0;   /* cmove */` | branch-specific rejection/error | [ ] |
| 2704 | `ZSTD_decodeSequence` (src/legacy/zstd_v01.c:1687) | `if (ZSTD_32bits()) FSE_reloadDStream(&(seqState->DStream));` | branch-specific rejection/error | [ ] |
| 2705 | `ZSTD_decodeSequence` (src/legacy/zstd_v01.c:1688) | `if (offsetCode==0) offset = prevOffset;` | branch-specific rejection/error | [ ] |
| 2706 | `ZSTD_decodeSequence` (src/legacy/zstd_v01.c:1696) | `if (add < 255) matchLength += add;` | branch-specific rejection/error | [ ] |
| 2707 | `ZSTD_decodeSequence` (src/legacy/zstd_v01.c:1699) | `if (dumps<=(de-3))` | branch-specific rejection/error | [ ] |
| 2708 | `ZSTD_execSequence` (src/legacy/zstd_v01.c:1732) | `if (seqLength > (size_t)(oend - op)) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [ ] |
| 2709 | `ZSTD_execSequence` (src/legacy/zstd_v01.c:1733) | `if (sequence.litLength > (size_t)(litLimit - *litPtr)) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 2710 | `ZSTD_execSequence` (src/legacy/zstd_v01.c:1735) | `if (sequence.offset > (U32)(oLitEnd - base)) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 2711 | `ZSTD_execSequence` (src/legacy/zstd_v01.c:1737) | `if (endMatch > oend) return ERROR(dstSize_tooSmall);   /* overwrite beyond dst buffer */` | `ERROR(dstSize_tooSmall)` | [ ] |
| 2712 | `ZSTD_execSequence` (src/legacy/zstd_v01.c:1738) | `if (litEnd > litLimit) return ERROR(corruption_detected);   /* overRead beyond lit buffer */` | `ERROR(corruption_detected)` | [ ] |
| 2713 | `ZSTD_execSequence` (src/legacy/zstd_v01.c:1739) | `if (sequence.matchLength > (size_t)(*litPtr-op)) return ERROR(dstSize_tooSmall);  /* overwrite literal segment */` | `ERROR(dstSize_tooSmall)` | [ ] |
| 2714 | `ZSTD_execSequence` (src/legacy/zstd_v01.c:1748) | `if (oend-op < 8) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [ ] |
| 2715 | `ZSTD_execSequence` (src/legacy/zstd_v01.c:1758) | `if (match < base) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 2716 | `ZSTD_execSequence` (src/legacy/zstd_v01.c:1759) | `if (sequence.offset > (size_t)base) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 2717 | `ZSTD_execSequence` (src/legacy/zstd_v01.c:1764) | `if ((endMatch + qutt) > oend) qutt = oend-endMatch;` | branch-specific rejection/error | [ ] |
| 2718 | `ZSTD_execSequence` (src/legacy/zstd_v01.c:1768) | `if (sequence.offset < 8)` | branch-specific rejection/error | [ ] |
| 2719 | `ZSTD_execSequence` (src/legacy/zstd_v01.c:1781) | `if (endMatch > oend-(16-MINMATCH))` | branch-specific rejection/error | [ ] |
| 2720 | `ZSTD_execSequence` (src/legacy/zstd_v01.c:1783) | `if (op < oend-8)` | branch-specific rejection/error | [ ] |
| 2721 | `ZSTD_decompressSequences` (src/legacy/zstd_v01.c:1853) | `if (FSE_isError(errorCode)) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 2722 | `ZSTD_decompressSequences` (src/legacy/zstd_v01.c:1869) | `if ( !FSE_endOfDStream(&(seqState.DStream)) ) return ERROR(corruption_detected);   /* requested too much : data is corrupted */` | `ERROR(corruption_detected)` | [ ] |
| 2723 | `ZSTD_decompressSequences` (src/legacy/zstd_v01.c:1870) | `if (nbSeq<0) return ERROR(corruption_detected);   /* requested too many sequences : data is corrupted */` | `ERROR(corruption_detected)` | [ ] |
| 2724 | `ZSTD_decompressSequences` (src/legacy/zstd_v01.c:1875) | `if (op+lastLLSize > oend) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [ ] |
| 2725 | `ZSTD_decompressSequences` (src/legacy/zstd_v01.c:1876) | `if (lastLLSize > 0) {` | branch-specific rejection/error | [ ] |
| 2726 | `ZSTDv01_decompressDCtx` (src/legacy/zstd_v01.c:1921) | `if (srcSize < ZSTD_frameHeaderSize+ZSTD_blockHeaderSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 2727 | `ZSTDv01_decompressDCtx` (src/legacy/zstd_v01.c:1923) | `if (magicNumber != ZSTD_magicNumber) return ERROR(prefix_unknown);` | `ERROR(prefix_unknown)` | [ ] |
| 2728 | `ZSTDv01_decompressDCtx` (src/legacy/zstd_v01.c:1934) | `if (blockSize > remainingSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 2729 | `ZSTDv01_decompressDCtx` (src/legacy/zstd_v01.c:1945) | `return ERROR(GENERIC);   /* not yet supported */` | `ERROR(GENERIC)` | [ ] |
| 2730 | `ZSTDv01_decompressDCtx` (src/legacy/zstd_v01.c:1949) | `if (remainingSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 2731 | `ZSTDv01_decompressDCtx` (src/legacy/zstd_v01.c:1952) | `return ERROR(GENERIC);` | `ERROR(GENERIC)` | [ ] |
| 2732 | `ZSTDv01_decompressDCtx` (src/legacy/zstd_v01.c:1954) | `if (blockSize == 0) break;   /* bt_end */` | branch-specific rejection/error | [ ] |
| 2733 | `ZSTDv01_findFrameSizeInfoLegacy` (src/legacy/zstd_v01.c:1989) | `if (srcSize < ZSTD_frameHeaderSize+ZSTD_blockHeaderSize) {` | `ERROR(srcSize_wrong)` | [ ] |
| 2734 | `ZSTDv01_findFrameSizeInfoLegacy` (src/legacy/zstd_v01.c:1990) | `ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, ERROR(srcSize_wrong));` | `ERROR(srcSize_wrong)` | [ ] |
| 2735 | `ZSTDv01_findFrameSizeInfoLegacy` (src/legacy/zstd_v01.c:1995) | `ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, ERROR(prefix_unknown));` | `ERROR(prefix_unknown)` | [ ] |
| 2736 | `ZSTDv01_findFrameSizeInfoLegacy` (src/legacy/zstd_v01.c:2011) | `if (blockSize > remainingSize) {` | `ERROR(srcSize_wrong)` | [ ] |
| 2737 | `ZSTDv01_findFrameSizeInfoLegacy` (src/legacy/zstd_v01.c:2012) | `ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, ERROR(srcSize_wrong));` | `ERROR(srcSize_wrong)` | [ ] |
| 2738 | `ZSTDv01_findFrameSizeInfoLegacy` (src/legacy/zstd_v01.c:2016) | `if (blockSize == 0) break;   /* bt_end */` | branch-specific rejection/error | [ ] |
| 2739 | `ZSTDv01_createDCtx` (src/legacy/zstd_v01.c:2043) | `if (dctx==NULL) return NULL;` | `NULL` | [ ] |
| 2740 | `ZSTDv01_decompressContinue` (src/legacy/zstd_v01.c:2064) | `if (srcSize != ctx->expected) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 2741 | `ZSTDv01_decompressContinue` (src/legacy/zstd_v01.c:2065) | `if (dst != ctx->previousDstEnd)  /* not contiguous */` | branch-specific rejection/error | [ ] |
| 2742 | `ZSTDv01_decompressContinue` (src/legacy/zstd_v01.c:2069) | `if (ctx->phase == 0)` | `ERROR(prefix_unknown)` | [ ] |
| 2743 | `ZSTDv01_decompressContinue` (src/legacy/zstd_v01.c:2073) | `if (magicNumber != ZSTD_magicNumber) return ERROR(prefix_unknown);` | `ERROR(prefix_unknown)` | [ ] |
| 2744 | `ZSTDv01_decompressContinue` (src/legacy/zstd_v01.c:2080) | `if (ctx->phase == 1)` | branch-specific rejection/error | [ ] |
| 2745 | `ZSTDv01_decompressContinue` (src/legacy/zstd_v01.c:2112) | `return ERROR(GENERIC);   /* not yet handled */` | `ERROR(GENERIC)` | [ ] |
| 2746 | `ZSTDv01_decompressContinue` (src/legacy/zstd_v01.c:2118) | `return ERROR(GENERIC);` | `ERROR(GENERIC)` | [ ] |
| 2747 | `BIT_initDStream` (src/legacy/zstd_v02.c:325) | `if (srcSize < 1) { memset(bitD, 0, sizeof(*bitD)); return ERROR(srcSize_wrong); }` | `ERROR(srcSize_wrong)` | [ ] |
| 2748 | `BIT_initDStream` (src/legacy/zstd_v02.c:327) | `if (srcSize >=  sizeof(size_t))   /* normal case */` | branch-specific rejection/error | [ ] |
| 2749 | `BIT_initDStream` (src/legacy/zstd_v02.c:334) | `if (contain32 == 0) return ERROR(GENERIC);   /* endMark not present */` | `ERROR(GENERIC)` | [ ] |
| 2750 | `BIT_initDStream` (src/legacy/zstd_v02.c:360) | `if (contain32 == 0) return ERROR(GENERIC);   /* endMark not present */` | `ERROR(GENERIC)` | [ ] |
| 2751 | `BIT_reloadDStream` (src/legacy/zstd_v02.c:405) | `if (bitD->bitsConsumed > (sizeof(bitD->bitContainer)*8))  /* should never happen */` | branch-specific rejection/error | [ ] |
| 2752 | `BIT_reloadDStream` (src/legacy/zstd_v02.c:408) | `if (bitD->ptr >= bitD->start + sizeof(bitD->bitContainer))` | branch-specific rejection/error | [ ] |
| 2753 | `BIT_reloadDStream` (src/legacy/zstd_v02.c:415) | `if (bitD->ptr == bitD->start)` | branch-specific rejection/error | [ ] |
| 2754 | `BIT_reloadDStream` (src/legacy/zstd_v02.c:417) | `if (bitD->bitsConsumed < sizeof(bitD->bitContainer)*8) return BIT_DStream_endOfBuffer;` | branch-specific rejection/error | [ ] |
| 2755 | `BIT_reloadDStream` (src/legacy/zstd_v02.c:423) | `if (bitD->ptr - nbBytes < bitD->start)` | branch-specific rejection/error | [ ] |
| 2756 | `ERR_isError` (src/legacy/zstd_v02.c:524) | `ERR_STATIC unsigned ERR_isError(size_t code) { return (code > ERROR(maxCode)); }` | `ERROR(maxCode)` | [ ] |
| 2757 | `FSE_buildDTable` (src/legacy/zstd_v02.c:1051) | `if (maxSymbolValue > FSE_MAX_SYMBOL_VALUE) return ERROR(maxSymbolValue_tooLarge);` | `ERROR(maxSymbolValue_tooLarge)` | [ ] |
| 2758 | `FSE_buildDTable` (src/legacy/zstd_v02.c:1052) | `if (tableLog > FSE_MAX_TABLELOG) return ERROR(tableLog_tooLarge);` | `ERROR(tableLog_tooLarge)` | [ ] |
| 2759 | `FSE_buildDTable` (src/legacy/zstd_v02.c:1065) | `if (normalizedCounter[s] >= largeLimit) noLarge=0;` | branch-specific rejection/error | [ ] |
| 2760 | `FSE_buildDTable` (src/legacy/zstd_v02.c:1082) | `if (position!=0) return ERROR(GENERIC);   /* position must reach all cells once, otherwise normalizedCounter is incorrect */` | `ERROR(GENERIC)` | [ ] |
| 2761 | `FSE_readNCount` (src/legacy/zstd_v02.c:1131) | `if (hbSize < 4) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 2762 | `FSE_readNCount` (src/legacy/zstd_v02.c:1134) | `if (nbBits > FSE_TABLELOG_ABSOLUTE_MAX) return ERROR(tableLog_tooLarge);` | `ERROR(tableLog_tooLarge)` | [ ] |
| 2763 | `FSE_readNCount` (src/legacy/zstd_v02.c:1150) | `if (ip < iend-5)` | branch-specific rejection/error | [ ] |
| 2764 | `FSE_readNCount` (src/legacy/zstd_v02.c:1169) | `if (n0 > *maxSVPtr) return ERROR(maxSymbolValue_tooSmall);` | `ERROR(maxSymbolValue_tooSmall)` | [ ] |
| 2765 | `FSE_readNCount` (src/legacy/zstd_v02.c:1171) | `if ((ip <= iend-7) \|\| (ip + (bitCount>>3) <= iend-4))` | branch-specific rejection/error | [ ] |
| 2766 | `FSE_readNCount` (src/legacy/zstd_v02.c:1184) | `if ((bitStream & (threshold-1)) < (U32)max)` | branch-specific rejection/error | [ ] |
| 2767 | `FSE_readNCount` (src/legacy/zstd_v02.c:1192) | `if (count >= threshold) count -= max;` | branch-specific rejection/error | [ ] |
| 2768 | `FSE_readNCount` (src/legacy/zstd_v02.c:1207) | `if ((ip <= iend-7) \|\| (ip + (bitCount>>3) <= iend-4))` | branch-specific rejection/error | [ ] |
| 2769 | `FSE_readNCount` (src/legacy/zstd_v02.c:1221) | `if (remaining != 1) return ERROR(GENERIC);` | `ERROR(GENERIC)` | [ ] |
| 2770 | `FSE_readNCount` (src/legacy/zstd_v02.c:1225) | `if ((size_t)(ip-istart) > hbSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 2771 | `FSE_buildDTable_raw` (src/legacy/zstd_v02.c:1261) | `if (nbBits < 1) return ERROR(GENERIC);         /* min size */` | `ERROR(GENERIC)` | [ ] |
| 2772 | `FSE_decompress_usingDTable_generic` (src/legacy/zstd_v02.c:1305) | `if (FSE_MAX_TABLELOG*2+7 > sizeof(bitD.bitContainer)*8)    /* This test must be static */` | branch-specific rejection/error | [ ] |
| 2773 | `FSE_decompress_usingDTable_generic` (src/legacy/zstd_v02.c:1310) | `if (FSE_MAX_TABLELOG*4+7 > sizeof(bitD.bitContainer)*8)    /* This test must be static */` | branch-specific rejection/error | [ ] |
| 2774 | `FSE_decompress_usingDTable_generic` (src/legacy/zstd_v02.c:1311) | `{ if (BIT_reloadDStream(&bitD) > BIT_DStream_unfinished) { op+=2; break; } }` | branch-specific rejection/error | [ ] |
| 2775 | `FSE_decompress_usingDTable_generic` (src/legacy/zstd_v02.c:1315) | `if (FSE_MAX_TABLELOG*2+7 > sizeof(bitD.bitContainer)*8)    /* This test must be static */` | branch-specific rejection/error | [ ] |
| 2776 | `FSE_decompress_usingDTable_generic` (src/legacy/zstd_v02.c:1325) | `if ( (BIT_reloadDStream(&bitD)>BIT_DStream_completed) \|\| (op==omax) \|\| (BIT_endOfDStream(&bitD) && (fast \|\| FSE_endOfDState(&state1))) )` | branch-specific rejection/error | [ ] |
| 2777 | `FSE_decompress_usingDTable_generic` (src/legacy/zstd_v02.c:1330) | `if ( (BIT_reloadDStream(&bitD)>BIT_DStream_completed) \|\| (op==omax) \|\| (BIT_endOfDStream(&bitD) && (fast \|\| FSE_endOfDState(&state2))) )` | branch-specific rejection/error | [ ] |
| 2778 | `FSE_decompress_usingDTable_generic` (src/legacy/zstd_v02.c:1340) | `if (op==omax) return ERROR(dstSize_tooSmall);   /* dst buffer is full, but cSrc unfinished */` | `ERROR(dstSize_tooSmall)` | [ ] |
| 2779 | `FSE_decompress_usingDTable_generic` (src/legacy/zstd_v02.c:1342) | `return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 2780 | `FSE_decompress` (src/legacy/zstd_v02.c:1369) | `if (cSrcSize<2) return ERROR(srcSize_wrong);   /* too small input size */` | `ERROR(srcSize_wrong)` | [ ] |
| 2781 | `FSE_decompress` (src/legacy/zstd_v02.c:1374) | `if (errorCode >= cSrcSize) return ERROR(srcSize_wrong);   /* too small input size */` | `ERROR(srcSize_wrong)` | [ ] |
| 2782 | `HUF_readStats` (src/legacy/zstd_v02.c:1492) | `if (!srcSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 2783 | `HUF_readStats` (src/legacy/zstd_v02.c:1496) | `if (iSize >= 128)  /* special header */` | branch-specific rejection/error | [ ] |
| 2784 | `HUF_readStats` (src/legacy/zstd_v02.c:1498) | `if (iSize >= (242))   /* RLE */` | branch-specific rejection/error | [ ] |
| 2785 | `HUF_readStats` (src/legacy/zstd_v02.c:1509) | `if (iSize+1 > srcSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 2786 | `HUF_readStats` (src/legacy/zstd_v02.c:1510) | `if (oSize >= hwSize) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 2787 | `HUF_readStats` (src/legacy/zstd_v02.c:1521) | `if (iSize+1 > srcSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 2788 | `HUF_readStats` (src/legacy/zstd_v02.c:1531) | `if (huffWeight[n] >= HUF_ABSOLUTEMAX_TABLELOG) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 2789 | `HUF_readStats` (src/legacy/zstd_v02.c:1535) | `if (weightTotal == 0) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 2790 | `HUF_readStats` (src/legacy/zstd_v02.c:1539) | `if (tableLog > HUF_ABSOLUTEMAX_TABLELOG) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 2791 | `HUF_readStats` (src/legacy/zstd_v02.c:1545) | `if (verif != rest) return ERROR(corruption_detected);    /* last value must be a clean power of 2 */` | `ERROR(corruption_detected)` | [ ] |
| 2792 | `HUF_readStats` (src/legacy/zstd_v02.c:1551) | `if ((rankStats[1] < 2) \|\| (rankStats[1] & 1)) return ERROR(corruption_detected);   /* by construction : at least 2 elts of rank 1, must be even */` | `ERROR(corruption_detected)` | [ ] |
| 2793 | `HUF_readDTableX2` (src/legacy/zstd_v02.c:1584) | `if (tableLog > DTable[0]) return ERROR(tableLog_tooLarge);   /* DTable is too small */` | `ERROR(tableLog_tooLarge)` | [ ] |
| 2794 | `HUF_decodeSymbolX2` (src/legacy/zstd_v02.c:1624) | `if (MEM_64bits() \|\| (HUF_MAX_TABLELOG<=12)) \` | branch-specific rejection/error | [ ] |
| 2795 | `HUF_decompress4X2_usingDTable` (src/legacy/zstd_v02.c:1661) | `if (cSrcSize < 10) return ERROR(corruption_detected);   /* strict minimum : jump table + 1 byte per stream */` | `ERROR(corruption_detected)` | [ ] |
| 2796 | `HUF_decompress4X2_usingDTable` (src/legacy/zstd_v02.c:1697) | `if (length4 > cSrcSize) return ERROR(corruption_detected);   /* overflow */` | `ERROR(corruption_detected)` | [ ] |
| 2797 | `HUF_decompress4X2_usingDTable` (src/legacy/zstd_v02.c:1732) | `if (op1 > opStart2) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 2798 | `HUF_decompress4X2_usingDTable` (src/legacy/zstd_v02.c:1733) | `if (op2 > opStart3) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 2799 | `HUF_decompress4X2_usingDTable` (src/legacy/zstd_v02.c:1734) | `if (op3 > opStart4) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 2800 | `HUF_decompress4X2_usingDTable` (src/legacy/zstd_v02.c:1745) | `if (!endSignal) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 2801 | `HUF_decompress4X2` (src/legacy/zstd_v02.c:1761) | `if (errorCode >= cSrcSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 2802 | `HUF_fillDTableX4Level2` (src/legacy/zstd_v02.c:1786) | `if (minWeight>1)` | branch-specific rejection/error | [ ] |
| 2803 | `HUF_fillDTableX4` (src/legacy/zstd_v02.c:1839) | `if (targetLog-nbBits >= minBits)   /* enough room for a second symbol */` | branch-specific rejection/error | [ ] |
| 2804 | `HUF_fillDTableX4` (src/legacy/zstd_v02.c:1843) | `if (minWeight < 1) minWeight = 1;` | branch-specific rejection/error | [ ] |
| 2805 | `HUF_readDTableX4` (src/legacy/zstd_v02.c:1882) | `if (memLog > HUF_ABSOLUTEMAX_TABLELOG) return ERROR(tableLog_tooLarge);` | `ERROR(tableLog_tooLarge)` | [ ] |
| 2806 | `HUF_readDTableX4` (src/legacy/zstd_v02.c:1889) | `if (tableLog > memLog) return ERROR(tableLog_tooLarge);   /* DTable can't fit code depth */` | `ERROR(tableLog_tooLarge)` | [ ] |
| 2807 | `HUF_readDTableX4` (src/legacy/zstd_v02.c:1893) | `{if (!maxW) return ERROR(GENERIC); }  /* necessarily finds a solution before maxW==0 */` | `ERROR(GENERIC)` | [ ] |
| 2808 | `HUF_decodeLastSymbolX4` (src/legacy/zstd_v02.c:1968) | `if (DStream->bitsConsumed < (sizeof(DStream->bitContainer)*8))` | branch-specific rejection/error | [ ] |
| 2809 | `HUF_decodeLastSymbolX4` (src/legacy/zstd_v02.c:1971) | `if (DStream->bitsConsumed > (sizeof(DStream->bitContainer)*8))` | branch-specific rejection/error | [ ] |
| 2810 | `HUF_decodeLastSymbolX4` (src/legacy/zstd_v02.c:1983) | `if (MEM_64bits() \|\| (HUF_MAX_TABLELOG<=12)) \` | branch-specific rejection/error | [ ] |
| 2811 | `HUF_decodeStreamX4` (src/legacy/zstd_v02.c:2010) | `if (p < pEnd)` | branch-specific rejection/error | [ ] |
| 2812 | `HUF_decompress4X4_usingDTable` (src/legacy/zstd_v02.c:2023) | `if (cSrcSize < 10) return ERROR(corruption_detected);   /* strict minimum : jump table + 1 byte per stream */` | `ERROR(corruption_detected)` | [ ] |
| 2813 | `HUF_decompress4X4_usingDTable` (src/legacy/zstd_v02.c:2059) | `if (length4 > cSrcSize) return ERROR(corruption_detected);   /* overflow */` | `ERROR(corruption_detected)` | [ ] |
| 2814 | `HUF_decompress4X4_usingDTable` (src/legacy/zstd_v02.c:2094) | `if (op1 > opStart2) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 2815 | `HUF_decompress4X4_usingDTable` (src/legacy/zstd_v02.c:2095) | `if (op2 > opStart3) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 2816 | `HUF_decompress4X4_usingDTable` (src/legacy/zstd_v02.c:2096) | `if (op3 > opStart4) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 2817 | `HUF_decompress4X4_usingDTable` (src/legacy/zstd_v02.c:2107) | `if (!endSignal) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 2818 | `HUF_decompress4X4` (src/legacy/zstd_v02.c:2122) | `if (hSize >= cSrcSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 2819 | `HUF_fillDTableX6LevelN` (src/legacy/zstd_v02.c:2152) | `if (minWeight>1)` | branch-specific rejection/error | [ ] |
| 2820 | `HUF_fillDTableX6LevelN` (src/legacy/zstd_v02.c:2177) | `if ((level<3) && (sizeLog-totalBits >= minBits))   /* enough room for another symbol */` | branch-specific rejection/error | [ ] |
| 2821 | `HUF_fillDTableX6LevelN` (src/legacy/zstd_v02.c:2180) | `if (nextMinWeight < 1) nextMinWeight = 1;` | branch-specific rejection/error | [ ] |
| 2822 | `HUF_readDTableX6` (src/legacy/zstd_v02.c:2215) | `if (memLog > HUF_ABSOLUTEMAX_TABLELOG) return ERROR(tableLog_tooLarge);` | `ERROR(tableLog_tooLarge)` | [ ] |
| 2823 | `HUF_readDTableX6` (src/legacy/zstd_v02.c:2222) | `if (tableLog > memLog) return ERROR(tableLog_tooLarge);   /* DTable is too small */` | `ERROR(tableLog_tooLarge)` | [ ] |
| 2824 | `HUF_readDTableX6` (src/legacy/zstd_v02.c:2226) | `{ if (!maxW) return ERROR(GENERIC); }  /* necessarily finds a solution before maxW==0 */` | `ERROR(GENERIC)` | [ ] |
| 2825 | `HUF_decodeLastSymbolsX6` (src/legacy/zstd_v02.c:2313) | `if (length <= maxL)` | branch-specific rejection/error | [ ] |
| 2826 | `HUF_decodeLastSymbolsX6` (src/legacy/zstd_v02.c:2320) | `if (DStream->bitsConsumed < (sizeof(DStream->bitContainer)*8))` | branch-specific rejection/error | [ ] |
| 2827 | `HUF_decodeLastSymbolsX6` (src/legacy/zstd_v02.c:2323) | `if (DStream->bitsConsumed > (sizeof(DStream->bitContainer)*8))` | branch-specific rejection/error | [ ] |
| 2828 | `HUF_decodeLastSymbolsX6` (src/legacy/zstd_v02.c:2334) | `if (MEM_64bits() \|\| (HUF_MAX_TABLELOG<=12)) \` | branch-specific rejection/error | [ ] |
| 2829 | `HUF_decompress4X6_usingDTable` (src/legacy/zstd_v02.c:2378) | `if (cSrcSize < 10) return ERROR(corruption_detected);   /* strict minimum : jump table + 1 byte per stream */` | `ERROR(corruption_detected)` | [ ] |
| 2830 | `HUF_decompress4X6_usingDTable` (src/legacy/zstd_v02.c:2416) | `if (length4 > cSrcSize) return ERROR(corruption_detected);   /* overflow */` | `ERROR(corruption_detected)` | [ ] |
| 2831 | `HUF_decompress4X6_usingDTable` (src/legacy/zstd_v02.c:2451) | `if (op1 > opStart2) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 2832 | `HUF_decompress4X6_usingDTable` (src/legacy/zstd_v02.c:2452) | `if (op2 > opStart3) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 2833 | `HUF_decompress4X6_usingDTable` (src/legacy/zstd_v02.c:2453) | `if (op3 > opStart4) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 2834 | `HUF_decompress4X6_usingDTable` (src/legacy/zstd_v02.c:2464) | `if (!endSignal) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 2835 | `HUF_decompress4X6` (src/legacy/zstd_v02.c:2479) | `if (hSize >= cSrcSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 2836 | `HUF_decompress` (src/legacy/zstd_v02.c:2526) | `if (dstSize == 0) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [ ] |
| 2837 | `HUF_decompress` (src/legacy/zstd_v02.c:2527) | `if (cSrcSize > dstSize) return ERROR(corruption_detected);   /* invalid */` | `ERROR(corruption_detected)` | [ ] |
| 2838 | `HUF_decompress` (src/legacy/zstd_v02.c:2538) | `if (Dtime[1] < Dtime[0]) algoNb = 1;` | branch-specific rejection/error | [ ] |
| 2839 | `HUF_decompress` (src/legacy/zstd_v02.c:2539) | `if (Dtime[2] < Dtime[algoNb]) algoNb = 2;` | branch-specific rejection/error | [ ] |
| 2840 | `ZSTD_getcBlockSize` (src/legacy/zstd_v02.c:2762) | `if (srcSize < 3) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 2841 | `ZSTD_getcBlockSize` (src/legacy/zstd_v02.c:2770) | `if (bpPtr->blockType == bt_end) return 0;` | branch-specific rejection/error | [ ] |
| 2842 | `ZSTD_getcBlockSize` (src/legacy/zstd_v02.c:2771) | `if (bpPtr->blockType == bt_rle) return 1;` | branch-specific rejection/error | [ ] |
| 2843 | `ZSTD_copyUncompressedBlock` (src/legacy/zstd_v02.c:2777) | `if (srcSize > maxDstSize) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [ ] |
| 2844 | `ZSTD_copyUncompressedBlock` (src/legacy/zstd_v02.c:2778) | `if (srcSize > 0) {` | branch-specific rejection/error | [ ] |
| 2845 | `ZSTD_decompressLiterals` (src/legacy/zstd_v02.c:2795) | `if (litSize > *maxDstSizePtr) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 2846 | `ZSTD_decompressLiterals` (src/legacy/zstd_v02.c:2796) | `if (litCSize + 5 > srcSize) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 2847 | `ZSTD_decompressLiterals` (src/legacy/zstd_v02.c:2798) | `if (HUF_isError(HUF_decompress(dst, litSize, ip+5, litCSize))) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 2848 | `ZSTD_decodeLiteralsBlock` (src/legacy/zstd_v02.c:2814) | `if (srcSize < MIN_CBLOCK_SIZE) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 2849 | `ZSTD_decodeLiteralsBlock` (src/legacy/zstd_v02.c:2831) | `if (litSize > srcSize-11)   /* risk of reading too far with wildcopy */` | `ERROR(corruption_detected)` | [ ] |
| 2850 | `ZSTD_decodeLiteralsBlock` (src/legacy/zstd_v02.c:2833) | `if (litSize > BLOCKSIZE) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 2851 | `ZSTD_decodeLiteralsBlock` (src/legacy/zstd_v02.c:2834) | `if (litSize > srcSize-3) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 2852 | `ZSTD_decodeLiteralsBlock` (src/legacy/zstd_v02.c:2849) | `if (litSize > BLOCKSIZE) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 2853 | `ZSTD_decodeSeqHeaders` (src/legacy/zstd_v02.c:2871) | `if (srcSize < 5) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 2854 | `ZSTD_decodeSeqHeaders` (src/legacy/zstd_v02.c:2895) | `if (ip > iend-3) return ERROR(srcSize_wrong); /* min : all 3 are "raw", hence no header, but at least xxLog bits per type */` | `ERROR(srcSize_wrong)` | [ ] |
| 2855 | `ZSTD_decodeSeqHeaders` (src/legacy/zstd_v02.c:2914) | `if (FSE_isError(headerSize)) return ERROR(GENERIC);` | `ERROR(GENERIC)` | [ ] |
| 2856 | `ZSTD_decodeSeqHeaders` (src/legacy/zstd_v02.c:2915) | `if (LLlog > LLFSELog) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 2857 | `ZSTD_decodeSeqHeaders` (src/legacy/zstd_v02.c:2924) | `if (ip > iend-2) return ERROR(srcSize_wrong);   /* min : "raw", hence no header, but at least xxLog bits */` | `ERROR(srcSize_wrong)` | [ ] |
| 2858 | `ZSTD_decodeSeqHeaders` (src/legacy/zstd_v02.c:2933) | `if (FSE_isError(headerSize)) return ERROR(GENERIC);` | `ERROR(GENERIC)` | [ ] |
| 2859 | `ZSTD_decodeSeqHeaders` (src/legacy/zstd_v02.c:2934) | `if (Offlog > OffFSELog) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 2860 | `ZSTD_decodeSeqHeaders` (src/legacy/zstd_v02.c:2943) | `if (ip > iend-2) return ERROR(srcSize_wrong); /* min : "raw", hence no header, but at least xxLog bits */` | `ERROR(srcSize_wrong)` | [ ] |
| 2861 | `ZSTD_decodeSeqHeaders` (src/legacy/zstd_v02.c:2951) | `if (FSE_isError(headerSize)) return ERROR(GENERIC);` | `ERROR(GENERIC)` | [ ] |
| 2862 | `ZSTD_decodeSeqHeaders` (src/legacy/zstd_v02.c:2952) | `if (MLlog > MLFSELog) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 2863 | `ZSTD_decodeSequence` (src/legacy/zstd_v02.c:2994) | `if (add < 255) litLength += add;` | branch-specific rejection/error | [ ] |
| 2864 | `ZSTD_decodeSequence` (src/legacy/zstd_v02.c:2995) | `else if (dumps + 3 <= de)` | branch-specific rejection/error | [ ] |
| 2865 | `ZSTD_decodeSequence` (src/legacy/zstd_v02.c:3000) | `if (dumps >= de) dumps = de-1;   /* late correction, to avoid read overflow (data is now corrupted anyway) */` | branch-specific rejection/error | [ ] |
| 2866 | `ZSTD_decodeSequence` (src/legacy/zstd_v02.c:3011) | `if (MEM_32bits()) BIT_reloadDStream(&(seqState->DStream));` | branch-specific rejection/error | [ ] |
| 2867 | `ZSTD_decodeSequence` (src/legacy/zstd_v02.c:3013) | `if (offsetCode==0) nbBits = 0;   /* cmove */` | branch-specific rejection/error | [ ] |
| 2868 | `ZSTD_decodeSequence` (src/legacy/zstd_v02.c:3015) | `if (MEM_32bits()) BIT_reloadDStream(&(seqState->DStream));` | branch-specific rejection/error | [ ] |
| 2869 | `ZSTD_decodeSequence` (src/legacy/zstd_v02.c:3016) | `if (offsetCode==0) offset = prevOffset;   /* cmove */` | branch-specific rejection/error | [ ] |
| 2870 | `ZSTD_decodeSequence` (src/legacy/zstd_v02.c:3024) | `if (add < 255) matchLength += add;` | branch-specific rejection/error | [ ] |
| 2871 | `ZSTD_decodeSequence` (src/legacy/zstd_v02.c:3025) | `else if (dumps + 3 <= de)` | branch-specific rejection/error | [ ] |
| 2872 | `ZSTD_decodeSequence` (src/legacy/zstd_v02.c:3030) | `if (dumps >= de) dumps = de-1;   /* late correction, to avoid read overflow (data is now corrupted anyway) */` | branch-specific rejection/error | [ ] |
| 2873 | `ZSTD_execSequence` (src/legacy/zstd_v02.c:3058) | `if (seqLength > (size_t)(oend - op)) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [ ] |
| 2874 | `ZSTD_execSequence` (src/legacy/zstd_v02.c:3059) | `if (sequence.litLength > (size_t)(litLimit - *litPtr)) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 2875 | `ZSTD_execSequence` (src/legacy/zstd_v02.c:3061) | `if (oLitEnd > oend_8) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [ ] |
| 2876 | `ZSTD_execSequence` (src/legacy/zstd_v02.c:3062) | `if (sequence.offset > (U32)(oLitEnd - base)) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 2877 | `ZSTD_execSequence` (src/legacy/zstd_v02.c:3064) | `if (oMatchEnd > oend) return ERROR(dstSize_tooSmall);   /* overwrite beyond dst buffer */` | `ERROR(dstSize_tooSmall)` | [ ] |
| 2878 | `ZSTD_execSequence` (src/legacy/zstd_v02.c:3065) | `if (litEnd > litLimit) return ERROR(corruption_detected);   /* overRead beyond lit buffer */` | `ERROR(corruption_detected)` | [ ] |
| 2879 | `ZSTD_execSequence` (src/legacy/zstd_v02.c:3077) | `if (sequence.offset > (size_t)op) return ERROR(corruption_detected);   /* address space overflow test (this test seems kept by clang optimizer) */` | `ERROR(corruption_detected)` | [ ] |
| 2880 | `ZSTD_execSequence` (src/legacy/zstd_v02.c:3079) | `if (match < base) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 2881 | `ZSTD_execSequence` (src/legacy/zstd_v02.c:3082) | `if (sequence.offset < 8)` | branch-specific rejection/error | [ ] |
| 2882 | `ZSTD_execSequence` (src/legacy/zstd_v02.c:3099) | `if (oMatchEnd > oend-(16-MINMATCH))` | branch-specific rejection/error | [ ] |
| 2883 | `ZSTD_execSequence` (src/legacy/zstd_v02.c:3101) | `if (op < oend_8)` | branch-specific rejection/error | [ ] |
| 2884 | `ZSTD_decompressSequences` (src/legacy/zstd_v02.c:3156) | `if (ERR_isError(errorCode)) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 2885 | `ZSTD_decompressSequences` (src/legacy/zstd_v02.c:3172) | `if ( !BIT_endOfDStream(&(seqState.DStream)) ) return ERROR(corruption_detected);   /* requested too much : data is corrupted */` | `ERROR(corruption_detected)` | [ ] |
| 2886 | `ZSTD_decompressSequences` (src/legacy/zstd_v02.c:3173) | `if (nbSeq<0) return ERROR(corruption_detected);   /* requested too many sequences : data is corrupted */` | `ERROR(corruption_detected)` | [ ] |
| 2887 | `ZSTD_decompressSequences` (src/legacy/zstd_v02.c:3178) | `if (litPtr > litEnd) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 2888 | `ZSTD_decompressSequences` (src/legacy/zstd_v02.c:3179) | `if (op+lastLLSize > oend) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [ ] |
| 2889 | `ZSTD_decompressSequences` (src/legacy/zstd_v02.c:3180) | `if (lastLLSize > 0) {` | branch-specific rejection/error | [ ] |
| 2890 | `ZSTD_decompressDCtx` (src/legacy/zstd_v02.c:3221) | `if (srcSize < ZSTD_frameHeaderSize+ZSTD_blockHeaderSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 2891 | `ZSTD_decompressDCtx` (src/legacy/zstd_v02.c:3223) | `if (magicNumber != ZSTD_magicNumber) return ERROR(prefix_unknown);` | `ERROR(prefix_unknown)` | [ ] |
| 2892 | `ZSTD_decompressDCtx` (src/legacy/zstd_v02.c:3235) | `if (cBlockSize > remainingSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 2893 | `ZSTD_decompressDCtx` (src/legacy/zstd_v02.c:3246) | `return ERROR(GENERIC);   /* not yet supported */` | `ERROR(GENERIC)` | [ ] |
| 2894 | `ZSTD_decompressDCtx` (src/legacy/zstd_v02.c:3250) | `if (remainingSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 2895 | `ZSTD_decompressDCtx` (src/legacy/zstd_v02.c:3253) | `return ERROR(GENERIC);   /* impossible */` | `ERROR(GENERIC)` | [ ] |
| 2896 | `ZSTD_decompressDCtx` (src/legacy/zstd_v02.c:3255) | `if (cBlockSize == 0) break;   /* bt_end */` | branch-specific rejection/error | [ ] |
| 2897 | `ZSTDv02_findFrameSizeInfoLegacy` (src/legacy/zstd_v02.c:3290) | `if (srcSize < ZSTD_frameHeaderSize+ZSTD_blockHeaderSize) {` | `ERROR(srcSize_wrong)` | [ ] |
| 2898 | `ZSTDv02_findFrameSizeInfoLegacy` (src/legacy/zstd_v02.c:3291) | `ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, ERROR(srcSize_wrong));` | `ERROR(srcSize_wrong)` | [ ] |
| 2899 | `ZSTDv02_findFrameSizeInfoLegacy` (src/legacy/zstd_v02.c:3296) | `ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, ERROR(prefix_unknown));` | `ERROR(prefix_unknown)` | [ ] |
| 2900 | `ZSTDv02_findFrameSizeInfoLegacy` (src/legacy/zstd_v02.c:3312) | `if (cBlockSize > remainingSize) {` | `ERROR(srcSize_wrong)` | [ ] |
| 2901 | `ZSTDv02_findFrameSizeInfoLegacy` (src/legacy/zstd_v02.c:3313) | `ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, ERROR(srcSize_wrong));` | `ERROR(srcSize_wrong)` | [ ] |
| 2902 | `ZSTDv02_findFrameSizeInfoLegacy` (src/legacy/zstd_v02.c:3317) | `if (cBlockSize == 0) break;   /* bt_end */` | branch-specific rejection/error | [ ] |
| 2903 | `ZSTD_createDCtx` (src/legacy/zstd_v02.c:3344) | `if (dctx==NULL) return NULL;` | `NULL` | [ ] |
| 2904 | `ZSTD_decompressContinue` (src/legacy/zstd_v02.c:3363) | `if (srcSize != ctx->expected) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 2905 | `ZSTD_decompressContinue` (src/legacy/zstd_v02.c:3364) | `if (dst != ctx->previousDstEnd)  /* not contiguous */` | branch-specific rejection/error | [ ] |
| 2906 | `ZSTD_decompressContinue` (src/legacy/zstd_v02.c:3368) | `if (ctx->phase == 0)` | `ERROR(prefix_unknown)` | [ ] |
| 2907 | `ZSTD_decompressContinue` (src/legacy/zstd_v02.c:3372) | `if (magicNumber != ZSTD_magicNumber) return ERROR(prefix_unknown);` | `ERROR(prefix_unknown)` | [ ] |
| 2908 | `ZSTD_decompressContinue` (src/legacy/zstd_v02.c:3379) | `if (ctx->phase == 1)` | branch-specific rejection/error | [ ] |
| 2909 | `ZSTD_decompressContinue` (src/legacy/zstd_v02.c:3411) | `return ERROR(GENERIC);   /* not yet handled */` | `ERROR(GENERIC)` | [ ] |
| 2910 | `ZSTD_decompressContinue` (src/legacy/zstd_v02.c:3417) | `return ERROR(GENERIC);` | `ERROR(GENERIC)` | [ ] |
| 2911 | `BIT_initDStream` (src/legacy/zstd_v03.c:327) | `if (srcSize < 1) { memset(bitD, 0, sizeof(*bitD)); return ERROR(srcSize_wrong); }` | `ERROR(srcSize_wrong)` | [ ] |
| 2912 | `BIT_initDStream` (src/legacy/zstd_v03.c:329) | `if (srcSize >=  sizeof(size_t))   /* normal case */` | branch-specific rejection/error | [ ] |
| 2913 | `BIT_initDStream` (src/legacy/zstd_v03.c:336) | `if (contain32 == 0) return ERROR(GENERIC);   /* endMark not present */` | `ERROR(GENERIC)` | [ ] |
| 2914 | `BIT_initDStream` (src/legacy/zstd_v03.c:362) | `if (contain32 == 0) return ERROR(GENERIC);   /* endMark not present */` | `ERROR(GENERIC)` | [ ] |
| 2915 | `BIT_reloadDStream` (src/legacy/zstd_v03.c:406) | `if (bitD->bitsConsumed > (sizeof(bitD->bitContainer)*8))  /* should never happen */` | branch-specific rejection/error | [ ] |
| 2916 | `BIT_reloadDStream` (src/legacy/zstd_v03.c:409) | `if (bitD->ptr >= bitD->start + sizeof(bitD->bitContainer))` | branch-specific rejection/error | [ ] |
| 2917 | `BIT_reloadDStream` (src/legacy/zstd_v03.c:416) | `if (bitD->ptr == bitD->start)` | branch-specific rejection/error | [ ] |
| 2918 | `BIT_reloadDStream` (src/legacy/zstd_v03.c:418) | `if (bitD->bitsConsumed < sizeof(bitD->bitContainer)*8) return BIT_DStream_endOfBuffer;` | branch-specific rejection/error | [ ] |
| 2919 | `BIT_reloadDStream` (src/legacy/zstd_v03.c:424) | `if (bitD->ptr - nbBytes < bitD->start)` | branch-specific rejection/error | [ ] |
| 2920 | `ERR_isError` (src/legacy/zstd_v03.c:525) | `ERR_STATIC unsigned ERR_isError(size_t code) { return (code > ERROR(maxCode)); }` | `ERROR(maxCode)` | [ ] |
| 2921 | `FSE_buildDTable` (src/legacy/zstd_v03.c:1051) | `if (maxSymbolValue > FSE_MAX_SYMBOL_VALUE) return ERROR(maxSymbolValue_tooLarge);` | `ERROR(maxSymbolValue_tooLarge)` | [ ] |
| 2922 | `FSE_buildDTable` (src/legacy/zstd_v03.c:1052) | `if (tableLog > FSE_MAX_TABLELOG) return ERROR(tableLog_tooLarge);` | `ERROR(tableLog_tooLarge)` | [ ] |
| 2923 | `FSE_buildDTable` (src/legacy/zstd_v03.c:1065) | `if (normalizedCounter[s] >= largeLimit) noLarge=0;` | branch-specific rejection/error | [ ] |
| 2924 | `FSE_buildDTable` (src/legacy/zstd_v03.c:1082) | `if (position!=0) return ERROR(GENERIC);   /* position must reach all cells once, otherwise normalizedCounter is incorrect */` | `ERROR(GENERIC)` | [ ] |
| 2925 | `FSE_readNCount` (src/legacy/zstd_v03.c:1131) | `if (hbSize < 4) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 2926 | `FSE_readNCount` (src/legacy/zstd_v03.c:1134) | `if (nbBits > FSE_TABLELOG_ABSOLUTE_MAX) return ERROR(tableLog_tooLarge);` | `ERROR(tableLog_tooLarge)` | [ ] |
| 2927 | `FSE_readNCount` (src/legacy/zstd_v03.c:1150) | `if (ip < iend-5)` | branch-specific rejection/error | [ ] |
| 2928 | `FSE_readNCount` (src/legacy/zstd_v03.c:1169) | `if (n0 > *maxSVPtr) return ERROR(maxSymbolValue_tooSmall);` | `ERROR(maxSymbolValue_tooSmall)` | [ ] |
| 2929 | `FSE_readNCount` (src/legacy/zstd_v03.c:1171) | `if ((ip <= iend-7) \|\| (ip + (bitCount>>3) <= iend-4))` | branch-specific rejection/error | [ ] |
| 2930 | `FSE_readNCount` (src/legacy/zstd_v03.c:1184) | `if ((bitStream & (threshold-1)) < (U32)max)` | branch-specific rejection/error | [ ] |
| 2931 | `FSE_readNCount` (src/legacy/zstd_v03.c:1192) | `if (count >= threshold) count -= max;` | branch-specific rejection/error | [ ] |
| 2932 | `FSE_readNCount` (src/legacy/zstd_v03.c:1207) | `if ((ip <= iend-7) \|\| (ip + (bitCount>>3) <= iend-4))` | branch-specific rejection/error | [ ] |
| 2933 | `FSE_readNCount` (src/legacy/zstd_v03.c:1221) | `if (remaining != 1) return ERROR(GENERIC);` | `ERROR(GENERIC)` | [ ] |
| 2934 | `FSE_readNCount` (src/legacy/zstd_v03.c:1225) | `if ((size_t)(ip-istart) > hbSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 2935 | `FSE_buildDTable_raw` (src/legacy/zstd_v03.c:1261) | `if (nbBits < 1) return ERROR(GENERIC);         /* min size */` | `ERROR(GENERIC)` | [ ] |
| 2936 | `FSE_decompress_usingDTable_generic` (src/legacy/zstd_v03.c:1305) | `if (FSE_MAX_TABLELOG*2+7 > sizeof(bitD.bitContainer)*8)    /* This test must be static */` | branch-specific rejection/error | [ ] |
| 2937 | `FSE_decompress_usingDTable_generic` (src/legacy/zstd_v03.c:1310) | `if (FSE_MAX_TABLELOG*4+7 > sizeof(bitD.bitContainer)*8)    /* This test must be static */` | branch-specific rejection/error | [ ] |
| 2938 | `FSE_decompress_usingDTable_generic` (src/legacy/zstd_v03.c:1311) | `{ if (BIT_reloadDStream(&bitD) > BIT_DStream_unfinished) { op+=2; break; } }` | branch-specific rejection/error | [ ] |
| 2939 | `FSE_decompress_usingDTable_generic` (src/legacy/zstd_v03.c:1315) | `if (FSE_MAX_TABLELOG*2+7 > sizeof(bitD.bitContainer)*8)    /* This test must be static */` | branch-specific rejection/error | [ ] |
| 2940 | `FSE_decompress_usingDTable_generic` (src/legacy/zstd_v03.c:1325) | `if ( (BIT_reloadDStream(&bitD)>BIT_DStream_completed) \|\| (op==omax) \|\| (BIT_endOfDStream(&bitD) && (fast \|\| FSE_endOfDState(&state1))) )` | branch-specific rejection/error | [ ] |
| 2941 | `FSE_decompress_usingDTable_generic` (src/legacy/zstd_v03.c:1330) | `if ( (BIT_reloadDStream(&bitD)>BIT_DStream_completed) \|\| (op==omax) \|\| (BIT_endOfDStream(&bitD) && (fast \|\| FSE_endOfDState(&state2))) )` | branch-specific rejection/error | [ ] |
| 2942 | `FSE_decompress_usingDTable_generic` (src/legacy/zstd_v03.c:1340) | `if (op==omax) return ERROR(dstSize_tooSmall);   /* dst buffer is full, but cSrc unfinished */` | `ERROR(dstSize_tooSmall)` | [ ] |
| 2943 | `FSE_decompress_usingDTable_generic` (src/legacy/zstd_v03.c:1342) | `return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 2944 | `FSE_decompress` (src/legacy/zstd_v03.c:1369) | `if (cSrcSize<2) return ERROR(srcSize_wrong);   /* too small input size */` | `ERROR(srcSize_wrong)` | [ ] |
| 2945 | `FSE_decompress` (src/legacy/zstd_v03.c:1374) | `if (errorCode >= cSrcSize) return ERROR(srcSize_wrong);   /* too small input size */` | `ERROR(srcSize_wrong)` | [ ] |
| 2946 | `HUF_readStats` (src/legacy/zstd_v03.c:1488) | `if (!srcSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 2947 | `HUF_readStats` (src/legacy/zstd_v03.c:1492) | `if (iSize >= 128)  /* special header */` | branch-specific rejection/error | [ ] |
| 2948 | `HUF_readStats` (src/legacy/zstd_v03.c:1494) | `if (iSize >= (242))   /* RLE */` | branch-specific rejection/error | [ ] |
| 2949 | `HUF_readStats` (src/legacy/zstd_v03.c:1505) | `if (iSize+1 > srcSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 2950 | `HUF_readStats` (src/legacy/zstd_v03.c:1506) | `if (oSize >= hwSize) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 2951 | `HUF_readStats` (src/legacy/zstd_v03.c:1517) | `if (iSize+1 > srcSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 2952 | `HUF_readStats` (src/legacy/zstd_v03.c:1527) | `if (huffWeight[n] >= HUF_ABSOLUTEMAX_TABLELOG) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 2953 | `HUF_readStats` (src/legacy/zstd_v03.c:1531) | `if (weightTotal == 0) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 2954 | `HUF_readStats` (src/legacy/zstd_v03.c:1535) | `if (tableLog > HUF_ABSOLUTEMAX_TABLELOG) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 2955 | `HUF_readStats` (src/legacy/zstd_v03.c:1541) | `if (verif != rest) return ERROR(corruption_detected);    /* last value must be a clean power of 2 */` | `ERROR(corruption_detected)` | [ ] |
| 2956 | `HUF_readStats` (src/legacy/zstd_v03.c:1547) | `if ((rankStats[1] < 2) \|\| (rankStats[1] & 1)) return ERROR(corruption_detected);   /* by construction : at least 2 elts of rank 1, must be even */` | `ERROR(corruption_detected)` | [ ] |
| 2957 | `HUF_readDTableX2` (src/legacy/zstd_v03.c:1580) | `if (tableLog > DTable[0]) return ERROR(tableLog_tooLarge);   /* DTable is too small */` | `ERROR(tableLog_tooLarge)` | [ ] |
| 2958 | `HUF_decodeSymbolX2` (src/legacy/zstd_v03.c:1620) | `if (MEM_64bits() \|\| (HUF_MAX_TABLELOG<=12)) \` | branch-specific rejection/error | [ ] |
| 2959 | `HUF_decompress4X2_usingDTable` (src/legacy/zstd_v03.c:1657) | `if (cSrcSize < 10) return ERROR(corruption_detected);   /* strict minimum : jump table + 1 byte per stream */` | `ERROR(corruption_detected)` | [ ] |
| 2960 | `HUF_decompress4X2_usingDTable` (src/legacy/zstd_v03.c:1693) | `if (length4 > cSrcSize) return ERROR(corruption_detected);   /* overflow */` | `ERROR(corruption_detected)` | [ ] |
| 2961 | `HUF_decompress4X2_usingDTable` (src/legacy/zstd_v03.c:1728) | `if (op1 > opStart2) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 2962 | `HUF_decompress4X2_usingDTable` (src/legacy/zstd_v03.c:1729) | `if (op2 > opStart3) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 2963 | `HUF_decompress4X2_usingDTable` (src/legacy/zstd_v03.c:1730) | `if (op3 > opStart4) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 2964 | `HUF_decompress4X2_usingDTable` (src/legacy/zstd_v03.c:1741) | `if (!endSignal) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 2965 | `HUF_decompress4X2` (src/legacy/zstd_v03.c:1757) | `if (errorCode >= cSrcSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 2966 | `HUF_fillDTableX4Level2` (src/legacy/zstd_v03.c:1782) | `if (minWeight>1)` | branch-specific rejection/error | [ ] |
| 2967 | `HUF_fillDTableX4` (src/legacy/zstd_v03.c:1835) | `if (targetLog-nbBits >= minBits)   /* enough room for a second symbol */` | branch-specific rejection/error | [ ] |
| 2968 | `HUF_fillDTableX4` (src/legacy/zstd_v03.c:1839) | `if (minWeight < 1) minWeight = 1;` | branch-specific rejection/error | [ ] |
| 2969 | `HUF_readDTableX4` (src/legacy/zstd_v03.c:1878) | `if (memLog > HUF_ABSOLUTEMAX_TABLELOG) return ERROR(tableLog_tooLarge);` | `ERROR(tableLog_tooLarge)` | [ ] |
| 2970 | `HUF_readDTableX4` (src/legacy/zstd_v03.c:1885) | `if (tableLog > memLog) return ERROR(tableLog_tooLarge);   /* DTable can't fit code depth */` | `ERROR(tableLog_tooLarge)` | [ ] |
| 2971 | `HUF_readDTableX4` (src/legacy/zstd_v03.c:1889) | `{ if (!maxW) return ERROR(GENERIC); }  /* necessarily finds a solution before maxW==0 */` | `ERROR(GENERIC)` | [ ] |
| 2972 | `HUF_decodeLastSymbolX4` (src/legacy/zstd_v03.c:1964) | `if (DStream->bitsConsumed < (sizeof(DStream->bitContainer)*8))` | branch-specific rejection/error | [ ] |
| 2973 | `HUF_decodeLastSymbolX4` (src/legacy/zstd_v03.c:1967) | `if (DStream->bitsConsumed > (sizeof(DStream->bitContainer)*8))` | branch-specific rejection/error | [ ] |
| 2974 | `HUF_decodeLastSymbolX4` (src/legacy/zstd_v03.c:1979) | `if (MEM_64bits() \|\| (HUF_MAX_TABLELOG<=12)) \` | branch-specific rejection/error | [ ] |
| 2975 | `HUF_decodeStreamX4` (src/legacy/zstd_v03.c:2006) | `if (p < pEnd)` | branch-specific rejection/error | [ ] |
| 2976 | `HUF_decompress4X4_usingDTable` (src/legacy/zstd_v03.c:2019) | `if (cSrcSize < 10) return ERROR(corruption_detected);   /* strict minimum : jump table + 1 byte per stream */` | `ERROR(corruption_detected)` | [ ] |
| 2977 | `HUF_decompress4X4_usingDTable` (src/legacy/zstd_v03.c:2055) | `if (length4 > cSrcSize) return ERROR(corruption_detected);   /* overflow */` | `ERROR(corruption_detected)` | [ ] |
| 2978 | `HUF_decompress4X4_usingDTable` (src/legacy/zstd_v03.c:2090) | `if (op1 > opStart2) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 2979 | `HUF_decompress4X4_usingDTable` (src/legacy/zstd_v03.c:2091) | `if (op2 > opStart3) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 2980 | `HUF_decompress4X4_usingDTable` (src/legacy/zstd_v03.c:2092) | `if (op3 > opStart4) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 2981 | `HUF_decompress4X4_usingDTable` (src/legacy/zstd_v03.c:2103) | `if (!endSignal) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 2982 | `HUF_decompress4X4` (src/legacy/zstd_v03.c:2118) | `if (hSize >= cSrcSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 2983 | `HUF_decompress` (src/legacy/zstd_v03.c:2165) | `if (dstSize == 0) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [ ] |
| 2984 | `HUF_decompress` (src/legacy/zstd_v03.c:2166) | `if (cSrcSize > dstSize) return ERROR(corruption_detected);   /* invalid */` | `ERROR(corruption_detected)` | [ ] |
| 2985 | `HUF_decompress` (src/legacy/zstd_v03.c:2177) | `if (Dtime[1] < Dtime[0]) algoNb = 1;` | branch-specific rejection/error | [ ] |
| 2986 | `ZSTD_getcBlockSize` (src/legacy/zstd_v03.c:2402) | `if (srcSize < 3) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 2987 | `ZSTD_getcBlockSize` (src/legacy/zstd_v03.c:2410) | `if (bpPtr->blockType == bt_end) return 0;` | branch-specific rejection/error | [ ] |
| 2988 | `ZSTD_getcBlockSize` (src/legacy/zstd_v03.c:2411) | `if (bpPtr->blockType == bt_rle) return 1;` | branch-specific rejection/error | [ ] |
| 2989 | `ZSTD_copyUncompressedBlock` (src/legacy/zstd_v03.c:2417) | `if (srcSize > maxDstSize) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [ ] |
| 2990 | `ZSTD_copyUncompressedBlock` (src/legacy/zstd_v03.c:2418) | `if (srcSize > 0) {` | branch-specific rejection/error | [ ] |
| 2991 | `ZSTD_decompressLiterals` (src/legacy/zstd_v03.c:2435) | `if (litSize > *maxDstSizePtr) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 2992 | `ZSTD_decompressLiterals` (src/legacy/zstd_v03.c:2436) | `if (litCSize + 5 > srcSize) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 2993 | `ZSTD_decompressLiterals` (src/legacy/zstd_v03.c:2438) | `if (HUF_isError(HUF_decompress(dst, litSize, ip+5, litCSize))) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 2994 | `ZSTD_decodeLiteralsBlock` (src/legacy/zstd_v03.c:2454) | `if (srcSize < MIN_CBLOCK_SIZE) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 2995 | `ZSTD_decodeLiteralsBlock` (src/legacy/zstd_v03.c:2471) | `if (litSize > srcSize-11)   /* risk of reading too far with wildcopy */` | `ERROR(corruption_detected)` | [ ] |
| 2996 | `ZSTD_decodeLiteralsBlock` (src/legacy/zstd_v03.c:2473) | `if (litSize > BLOCKSIZE) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 2997 | `ZSTD_decodeLiteralsBlock` (src/legacy/zstd_v03.c:2474) | `if (litSize > srcSize-3) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 2998 | `ZSTD_decodeLiteralsBlock` (src/legacy/zstd_v03.c:2489) | `if (litSize > BLOCKSIZE) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 2999 | `ZSTD_decodeSeqHeaders` (src/legacy/zstd_v03.c:2511) | `if (srcSize < 5) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 3000 | `ZSTD_decodeSeqHeaders` (src/legacy/zstd_v03.c:2535) | `if (ip > iend-3) return ERROR(srcSize_wrong); /* min : all 3 are "raw", hence no header, but at least xxLog bits per type */` | `ERROR(srcSize_wrong)` | [ ] |
| 3001 | `ZSTD_decodeSeqHeaders` (src/legacy/zstd_v03.c:2554) | `if (FSE_isError(headerSize)) return ERROR(GENERIC);` | `ERROR(GENERIC)` | [ ] |
| 3002 | `ZSTD_decodeSeqHeaders` (src/legacy/zstd_v03.c:2555) | `if (LLlog > LLFSELog) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3003 | `ZSTD_decodeSeqHeaders` (src/legacy/zstd_v03.c:2564) | `if (ip > iend-2) return ERROR(srcSize_wrong);   /* min : "raw", hence no header, but at least xxLog bits */` | `ERROR(srcSize_wrong)` | [ ] |
| 3004 | `ZSTD_decodeSeqHeaders` (src/legacy/zstd_v03.c:2573) | `if (FSE_isError(headerSize)) return ERROR(GENERIC);` | `ERROR(GENERIC)` | [ ] |
| 3005 | `ZSTD_decodeSeqHeaders` (src/legacy/zstd_v03.c:2574) | `if (Offlog > OffFSELog) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3006 | `ZSTD_decodeSeqHeaders` (src/legacy/zstd_v03.c:2583) | `if (ip > iend-2) return ERROR(srcSize_wrong); /* min : "raw", hence no header, but at least xxLog bits */` | `ERROR(srcSize_wrong)` | [ ] |
| 3007 | `ZSTD_decodeSeqHeaders` (src/legacy/zstd_v03.c:2591) | `if (FSE_isError(headerSize)) return ERROR(GENERIC);` | `ERROR(GENERIC)` | [ ] |
| 3008 | `ZSTD_decodeSeqHeaders` (src/legacy/zstd_v03.c:2592) | `if (MLlog > MLFSELog) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3009 | `ZSTD_decodeSequence` (src/legacy/zstd_v03.c:2634) | `if (add < 255) litLength += add;` | branch-specific rejection/error | [ ] |
| 3010 | `ZSTD_decodeSequence` (src/legacy/zstd_v03.c:2635) | `else if (dumps + 3 <= de)` | branch-specific rejection/error | [ ] |
| 3011 | `ZSTD_decodeSequence` (src/legacy/zstd_v03.c:2640) | `if (dumps >= de) dumps = de-1;   /* late correction, to avoid read overflow (data is now corrupted anyway) */` | branch-specific rejection/error | [ ] |
| 3012 | `ZSTD_decodeSequence` (src/legacy/zstd_v03.c:2651) | `if (MEM_32bits()) BIT_reloadDStream(&(seqState->DStream));` | branch-specific rejection/error | [ ] |
| 3013 | `ZSTD_decodeSequence` (src/legacy/zstd_v03.c:2653) | `if (offsetCode==0) nbBits = 0;   /* cmove */` | branch-specific rejection/error | [ ] |
| 3014 | `ZSTD_decodeSequence` (src/legacy/zstd_v03.c:2655) | `if (MEM_32bits()) BIT_reloadDStream(&(seqState->DStream));` | branch-specific rejection/error | [ ] |
| 3015 | `ZSTD_decodeSequence` (src/legacy/zstd_v03.c:2656) | `if (offsetCode==0) offset = prevOffset;   /* cmove */` | branch-specific rejection/error | [ ] |
| 3016 | `ZSTD_decodeSequence` (src/legacy/zstd_v03.c:2664) | `if (add < 255) matchLength += add;` | branch-specific rejection/error | [ ] |
| 3017 | `ZSTD_decodeSequence` (src/legacy/zstd_v03.c:2665) | `else if (dumps + 3 <= de)` | branch-specific rejection/error | [ ] |
| 3018 | `ZSTD_decodeSequence` (src/legacy/zstd_v03.c:2670) | `if (dumps >= de) dumps = de-1;   /* late correction, to avoid read overflow (data is now corrupted anyway) */` | branch-specific rejection/error | [ ] |
| 3019 | `ZSTD_execSequence` (src/legacy/zstd_v03.c:2698) | `if (seqLength > (size_t)(oend - op)) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [ ] |
| 3020 | `ZSTD_execSequence` (src/legacy/zstd_v03.c:2699) | `if (sequence.litLength > (size_t)(litLimit - *litPtr)) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3021 | `ZSTD_execSequence` (src/legacy/zstd_v03.c:2701) | `if (oLitEnd > oend_8) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [ ] |
| 3022 | `ZSTD_execSequence` (src/legacy/zstd_v03.c:2702) | `if (sequence.offset > (U32)(oLitEnd - base)) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3023 | `ZSTD_execSequence` (src/legacy/zstd_v03.c:2704) | `if (oMatchEnd > oend) return ERROR(dstSize_tooSmall);   /* overwrite beyond dst buffer */` | `ERROR(dstSize_tooSmall)` | [ ] |
| 3024 | `ZSTD_execSequence` (src/legacy/zstd_v03.c:2705) | `if (litEnd > litLimit) return ERROR(corruption_detected);   /* overRead beyond lit buffer */` | `ERROR(corruption_detected)` | [ ] |
| 3025 | `ZSTD_execSequence` (src/legacy/zstd_v03.c:2716) | `if (sequence.offset > (size_t)op) return ERROR(corruption_detected);   /* address space overflow test (this test seems kept by clang optimizer) */` | `ERROR(corruption_detected)` | [ ] |
| 3026 | `ZSTD_execSequence` (src/legacy/zstd_v03.c:2718) | `if (match < base) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3027 | `ZSTD_execSequence` (src/legacy/zstd_v03.c:2721) | `if (sequence.offset < 8)` | branch-specific rejection/error | [ ] |
| 3028 | `ZSTD_execSequence` (src/legacy/zstd_v03.c:2738) | `if (oMatchEnd > oend-(16-MINMATCH))` | branch-specific rejection/error | [ ] |
| 3029 | `ZSTD_execSequence` (src/legacy/zstd_v03.c:2740) | `if (op < oend_8)` | branch-specific rejection/error | [ ] |
| 3030 | `ZSTD_decompressSequences` (src/legacy/zstd_v03.c:2795) | `if (ERR_isError(errorCode)) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3031 | `ZSTD_decompressSequences` (src/legacy/zstd_v03.c:2811) | `if ( !BIT_endOfDStream(&(seqState.DStream)) ) return ERROR(corruption_detected);   /* requested too much : data is corrupted */` | `ERROR(corruption_detected)` | [ ] |
| 3032 | `ZSTD_decompressSequences` (src/legacy/zstd_v03.c:2812) | `if (nbSeq<0) return ERROR(corruption_detected);   /* requested too many sequences : data is corrupted */` | `ERROR(corruption_detected)` | [ ] |
| 3033 | `ZSTD_decompressSequences` (src/legacy/zstd_v03.c:2817) | `if (litPtr > litEnd) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3034 | `ZSTD_decompressSequences` (src/legacy/zstd_v03.c:2818) | `if (op+lastLLSize > oend) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [ ] |
| 3035 | `ZSTD_decompressSequences` (src/legacy/zstd_v03.c:2819) | `if (lastLLSize > 0) {` | branch-specific rejection/error | [ ] |
| 3036 | `ZSTD_decompressDCtx` (src/legacy/zstd_v03.c:2860) | `if (srcSize < ZSTD_frameHeaderSize+ZSTD_blockHeaderSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 3037 | `ZSTD_decompressDCtx` (src/legacy/zstd_v03.c:2862) | `if (magicNumber != ZSTD_magicNumber) return ERROR(prefix_unknown);` | `ERROR(prefix_unknown)` | [ ] |
| 3038 | `ZSTD_decompressDCtx` (src/legacy/zstd_v03.c:2874) | `if (cBlockSize > remainingSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 3039 | `ZSTD_decompressDCtx` (src/legacy/zstd_v03.c:2885) | `return ERROR(GENERIC);   /* not yet supported */` | `ERROR(GENERIC)` | [ ] |
| 3040 | `ZSTD_decompressDCtx` (src/legacy/zstd_v03.c:2889) | `if (remainingSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 3041 | `ZSTD_decompressDCtx` (src/legacy/zstd_v03.c:2892) | `return ERROR(GENERIC);   /* impossible */` | `ERROR(GENERIC)` | [ ] |
| 3042 | `ZSTD_decompressDCtx` (src/legacy/zstd_v03.c:2894) | `if (cBlockSize == 0) break;   /* bt_end */` | branch-specific rejection/error | [ ] |
| 3043 | `ZSTDv03_findFrameSizeInfoLegacy` (src/legacy/zstd_v03.c:2929) | `if (srcSize < ZSTD_frameHeaderSize+ZSTD_blockHeaderSize) {` | `ERROR(srcSize_wrong)` | [ ] |
| 3044 | `ZSTDv03_findFrameSizeInfoLegacy` (src/legacy/zstd_v03.c:2930) | `ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, ERROR(srcSize_wrong));` | `ERROR(srcSize_wrong)` | [ ] |
| 3045 | `ZSTDv03_findFrameSizeInfoLegacy` (src/legacy/zstd_v03.c:2935) | `ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, ERROR(prefix_unknown));` | `ERROR(prefix_unknown)` | [ ] |
| 3046 | `ZSTDv03_findFrameSizeInfoLegacy` (src/legacy/zstd_v03.c:2951) | `if (cBlockSize > remainingSize) {` | `ERROR(srcSize_wrong)` | [ ] |
| 3047 | `ZSTDv03_findFrameSizeInfoLegacy` (src/legacy/zstd_v03.c:2952) | `ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, ERROR(srcSize_wrong));` | `ERROR(srcSize_wrong)` | [ ] |
| 3048 | `ZSTDv03_findFrameSizeInfoLegacy` (src/legacy/zstd_v03.c:2956) | `if (cBlockSize == 0) break;   /* bt_end */` | branch-specific rejection/error | [ ] |
| 3049 | `ZSTD_createDCtx` (src/legacy/zstd_v03.c:2984) | `if (dctx==NULL) return NULL;` | `NULL` | [ ] |
| 3050 | `ZSTD_decompressContinue` (src/legacy/zstd_v03.c:3003) | `if (srcSize != ctx->expected) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 3051 | `ZSTD_decompressContinue` (src/legacy/zstd_v03.c:3004) | `if (dst != ctx->previousDstEnd)  /* not contiguous */` | branch-specific rejection/error | [ ] |
| 3052 | `ZSTD_decompressContinue` (src/legacy/zstd_v03.c:3008) | `if (ctx->phase == 0)` | `ERROR(prefix_unknown)` | [ ] |
| 3053 | `ZSTD_decompressContinue` (src/legacy/zstd_v03.c:3012) | `if (magicNumber != ZSTD_magicNumber) return ERROR(prefix_unknown);` | `ERROR(prefix_unknown)` | [ ] |
| 3054 | `ZSTD_decompressContinue` (src/legacy/zstd_v03.c:3019) | `if (ctx->phase == 1)` | branch-specific rejection/error | [ ] |
| 3055 | `ZSTD_decompressContinue` (src/legacy/zstd_v03.c:3051) | `return ERROR(GENERIC);   /* not yet handled */` | `ERROR(GENERIC)` | [ ] |
| 3056 | `ZSTD_decompressContinue` (src/legacy/zstd_v03.c:3057) | `return ERROR(GENERIC);` | `ERROR(GENERIC)` | [ ] |
| 3057 | `BIT_initDStream` (src/legacy/zstd_v04.c:603) | `if (srcSize < 1) { memset(bitD, 0, sizeof(*bitD)); return ERROR(srcSize_wrong); }` | `ERROR(srcSize_wrong)` | [ ] |
| 3058 | `BIT_initDStream` (src/legacy/zstd_v04.c:605) | `if (srcSize >=  sizeof(size_t))   /* normal case */` | branch-specific rejection/error | [ ] |
| 3059 | `BIT_initDStream` (src/legacy/zstd_v04.c:612) | `if (contain32 == 0) return ERROR(GENERIC);   /* endMark not present */` | `ERROR(GENERIC)` | [ ] |
| 3060 | `BIT_initDStream` (src/legacy/zstd_v04.c:632) | `if (contain32 == 0) return ERROR(GENERIC);   /* endMark not present */` | `ERROR(GENERIC)` | [ ] |
| 3061 | `BIT_reloadDStream` (src/legacy/zstd_v04.c:677) | `if (bitD->bitsConsumed > (sizeof(bitD->bitContainer)*8))  /* should never happen */` | branch-specific rejection/error | [ ] |
| 3062 | `BIT_reloadDStream` (src/legacy/zstd_v04.c:680) | `if (bitD->ptr >= bitD->start + sizeof(bitD->bitContainer))` | branch-specific rejection/error | [ ] |
| 3063 | `BIT_reloadDStream` (src/legacy/zstd_v04.c:687) | `if (bitD->ptr == bitD->start)` | branch-specific rejection/error | [ ] |
| 3064 | `BIT_reloadDStream` (src/legacy/zstd_v04.c:689) | `if (bitD->bitsConsumed < sizeof(bitD->bitContainer)*8) return BIT_DStream_endOfBuffer;` | branch-specific rejection/error | [ ] |
| 3065 | `BIT_reloadDStream` (src/legacy/zstd_v04.c:695) | `if (bitD->ptr - nbBytes < bitD->start)` | branch-specific rejection/error | [ ] |
| 3066 | `FSE_buildDTable` (src/legacy/zstd_v04.c:1033) | `if (maxSymbolValue > FSE_MAX_SYMBOL_VALUE) return ERROR(maxSymbolValue_tooLarge);` | `ERROR(maxSymbolValue_tooLarge)` | [ ] |
| 3067 | `FSE_buildDTable` (src/legacy/zstd_v04.c:1034) | `if (tableLog > FSE_MAX_TABLELOG) return ERROR(tableLog_tooLarge);` | `ERROR(tableLog_tooLarge)` | [ ] |
| 3068 | `FSE_buildDTable` (src/legacy/zstd_v04.c:1048) | `if (normalizedCounter[s] >= largeLimit) noLarge=0;` | branch-specific rejection/error | [ ] |
| 3069 | `FSE_buildDTable` (src/legacy/zstd_v04.c:1065) | `if (position!=0) return ERROR(GENERIC);   /* position must reach all cells once, otherwise normalizedCounter is incorrect */` | `ERROR(GENERIC)` | [ ] |
| 3070 | `FSE_readNCount` (src/legacy/zstd_v04.c:1114) | `if (hbSize < 4) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 3071 | `FSE_readNCount` (src/legacy/zstd_v04.c:1117) | `if (nbBits > FSE_TABLELOG_ABSOLUTE_MAX) return ERROR(tableLog_tooLarge);` | `ERROR(tableLog_tooLarge)` | [ ] |
| 3072 | `FSE_readNCount` (src/legacy/zstd_v04.c:1133) | `if (ip < iend-5)` | branch-specific rejection/error | [ ] |
| 3073 | `FSE_readNCount` (src/legacy/zstd_v04.c:1152) | `if (n0 > *maxSVPtr) return ERROR(maxSymbolValue_tooSmall);` | `ERROR(maxSymbolValue_tooSmall)` | [ ] |
| 3074 | `FSE_readNCount` (src/legacy/zstd_v04.c:1154) | `if ((ip <= iend-7) \|\| (ip + (bitCount>>3) <= iend-4))` | branch-specific rejection/error | [ ] |
| 3075 | `FSE_readNCount` (src/legacy/zstd_v04.c:1167) | `if ((bitStream & (threshold-1)) < (U32)max)` | branch-specific rejection/error | [ ] |
| 3076 | `FSE_readNCount` (src/legacy/zstd_v04.c:1175) | `if (count >= threshold) count -= max;` | branch-specific rejection/error | [ ] |
| 3077 | `FSE_readNCount` (src/legacy/zstd_v04.c:1190) | `if ((ip <= iend-7) \|\| (ip + (bitCount>>3) <= iend-4))` | branch-specific rejection/error | [ ] |
| 3078 | `FSE_readNCount` (src/legacy/zstd_v04.c:1204) | `if (remaining != 1) return ERROR(GENERIC);` | `ERROR(GENERIC)` | [ ] |
| 3079 | `FSE_readNCount` (src/legacy/zstd_v04.c:1208) | `if ((size_t)(ip-istart) > hbSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 3080 | `FSE_buildDTable_raw` (src/legacy/zstd_v04.c:1246) | `if (nbBits < 1) return ERROR(GENERIC);         /* min size */` | `ERROR(GENERIC)` | [ ] |
| 3081 | `FSE_decompress_usingDTable_generic` (src/legacy/zstd_v04.c:1290) | `if (FSE_MAX_TABLELOG*2+7 > sizeof(bitD.bitContainer)*8)    /* This test must be static */` | branch-specific rejection/error | [ ] |
| 3082 | `FSE_decompress_usingDTable_generic` (src/legacy/zstd_v04.c:1295) | `if (FSE_MAX_TABLELOG*4+7 > sizeof(bitD.bitContainer)*8)    /* This test must be static */` | branch-specific rejection/error | [ ] |
| 3083 | `FSE_decompress_usingDTable_generic` (src/legacy/zstd_v04.c:1296) | `{ if (BIT_reloadDStream(&bitD) > BIT_DStream_unfinished) { op+=2; break; } }` | branch-specific rejection/error | [ ] |
| 3084 | `FSE_decompress_usingDTable_generic` (src/legacy/zstd_v04.c:1300) | `if (FSE_MAX_TABLELOG*2+7 > sizeof(bitD.bitContainer)*8)    /* This test must be static */` | branch-specific rejection/error | [ ] |
| 3085 | `FSE_decompress_usingDTable_generic` (src/legacy/zstd_v04.c:1310) | `if ( (BIT_reloadDStream(&bitD)>BIT_DStream_completed) \|\| (op==omax) \|\| (BIT_endOfDStream(&bitD) && (fast \|\| FSE_endOfDState(&state1))) )` | branch-specific rejection/error | [ ] |
| 3086 | `FSE_decompress_usingDTable_generic` (src/legacy/zstd_v04.c:1315) | `if ( (BIT_reloadDStream(&bitD)>BIT_DStream_completed) \|\| (op==omax) \|\| (BIT_endOfDStream(&bitD) && (fast \|\| FSE_endOfDState(&state2))) )` | branch-specific rejection/error | [ ] |
| 3087 | `FSE_decompress_usingDTable_generic` (src/legacy/zstd_v04.c:1325) | `if (op==omax) return ERROR(dstSize_tooSmall);   /* dst buffer is full, but cSrc unfinished */` | `ERROR(dstSize_tooSmall)` | [ ] |
| 3088 | `FSE_decompress_usingDTable_generic` (src/legacy/zstd_v04.c:1327) | `return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3089 | `FSE_decompress` (src/legacy/zstd_v04.c:1357) | `if (cSrcSize<2) return ERROR(srcSize_wrong);   /* too small input size */` | `ERROR(srcSize_wrong)` | [ ] |
| 3090 | `FSE_decompress` (src/legacy/zstd_v04.c:1362) | `if (errorCode >= cSrcSize) return ERROR(srcSize_wrong);   /* too small input size */` | `ERROR(srcSize_wrong)` | [ ] |
| 3091 | `HUF_readStats` (src/legacy/zstd_v04.c:1647) | `if (!srcSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 3092 | `HUF_readStats` (src/legacy/zstd_v04.c:1651) | `if (iSize >= 128)  /* special header */` | branch-specific rejection/error | [ ] |
| 3093 | `HUF_readStats` (src/legacy/zstd_v04.c:1653) | `if (iSize >= (242))   /* RLE */` | branch-specific rejection/error | [ ] |
| 3094 | `HUF_readStats` (src/legacy/zstd_v04.c:1664) | `if (iSize+1 > srcSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 3095 | `HUF_readStats` (src/legacy/zstd_v04.c:1665) | `if (oSize >= hwSize) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3096 | `HUF_readStats` (src/legacy/zstd_v04.c:1676) | `if (iSize+1 > srcSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 3097 | `HUF_readStats` (src/legacy/zstd_v04.c:1686) | `if (huffWeight[n] >= HUF_ABSOLUTEMAX_TABLELOG) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3098 | `HUF_readStats` (src/legacy/zstd_v04.c:1690) | `if (weightTotal == 0) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3099 | `HUF_readStats` (src/legacy/zstd_v04.c:1694) | `if (tableLog > HUF_ABSOLUTEMAX_TABLELOG) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3100 | `HUF_readStats` (src/legacy/zstd_v04.c:1700) | `if (verif != rest) return ERROR(corruption_detected);    /* last value must be a clean power of 2 */` | `ERROR(corruption_detected)` | [ ] |
| 3101 | `HUF_readStats` (src/legacy/zstd_v04.c:1706) | `if ((rankStats[1] < 2) \|\| (rankStats[1] & 1)) return ERROR(corruption_detected);   /* by construction : at least 2 elts of rank 1, must be even */` | `ERROR(corruption_detected)` | [ ] |
| 3102 | `HUF_readDTableX2` (src/legacy/zstd_v04.c:1738) | `if (tableLog > DTable[0]) return ERROR(tableLog_tooLarge);   /* DTable is too small */` | `ERROR(tableLog_tooLarge)` | [ ] |
| 3103 | `HUF_decodeSymbolX2` (src/legacy/zstd_v04.c:1778) | `if (MEM_64bits() \|\| (HUF_MAX_TABLELOG<=12)) \` | branch-specific rejection/error | [ ] |
| 3104 | `HUF_decompress4X2_usingDTable` (src/legacy/zstd_v04.c:1815) | `if (cSrcSize < 10) return ERROR(corruption_detected);   /* strict minimum : jump table + 1 byte per stream */` | `ERROR(corruption_detected)` | [ ] |
| 3105 | `HUF_decompress4X2_usingDTable` (src/legacy/zstd_v04.c:1850) | `if (length4 > cSrcSize) return ERROR(corruption_detected);   /* overflow */` | `ERROR(corruption_detected)` | [ ] |
| 3106 | `HUF_decompress4X2_usingDTable` (src/legacy/zstd_v04.c:1885) | `if (op1 > opStart2) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3107 | `HUF_decompress4X2_usingDTable` (src/legacy/zstd_v04.c:1886) | `if (op2 > opStart3) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3108 | `HUF_decompress4X2_usingDTable` (src/legacy/zstd_v04.c:1887) | `if (op3 > opStart4) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3109 | `HUF_decompress4X2_usingDTable` (src/legacy/zstd_v04.c:1898) | `if (!endSignal) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3110 | `HUF_decompress4X2` (src/legacy/zstd_v04.c:1914) | `if (errorCode >= cSrcSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 3111 | `HUF_fillDTableX4Level2` (src/legacy/zstd_v04.c:1939) | `if (minWeight>1)` | branch-specific rejection/error | [ ] |
| 3112 | `HUF_fillDTableX4` (src/legacy/zstd_v04.c:1992) | `if (targetLog-nbBits >= minBits)   /* enough room for a second symbol */` | branch-specific rejection/error | [ ] |
| 3113 | `HUF_fillDTableX4` (src/legacy/zstd_v04.c:1996) | `if (minWeight < 1) minWeight = 1;` | branch-specific rejection/error | [ ] |
| 3114 | `HUF_readDTableX4` (src/legacy/zstd_v04.c:2034) | `if (memLog > HUF_ABSOLUTEMAX_TABLELOG) return ERROR(tableLog_tooLarge);` | `ERROR(tableLog_tooLarge)` | [ ] |
| 3115 | `HUF_readDTableX4` (src/legacy/zstd_v04.c:2041) | `if (tableLog > memLog) return ERROR(tableLog_tooLarge);   /* DTable can't fit code depth */` | `ERROR(tableLog_tooLarge)` | [ ] |
| 3116 | `HUF_readDTableX4` (src/legacy/zstd_v04.c:2045) | `{ if (!maxW) return ERROR(GENERIC); }  /* necessarily finds a solution before maxW==0 */` | `ERROR(GENERIC)` | [ ] |
| 3117 | `HUF_decodeLastSymbolX4` (src/legacy/zstd_v04.c:2120) | `if (DStream->bitsConsumed < (sizeof(DStream->bitContainer)*8))` | branch-specific rejection/error | [ ] |
| 3118 | `HUF_decodeLastSymbolX4` (src/legacy/zstd_v04.c:2123) | `if (DStream->bitsConsumed > (sizeof(DStream->bitContainer)*8))` | branch-specific rejection/error | [ ] |
| 3119 | `HUF_decodeLastSymbolX4` (src/legacy/zstd_v04.c:2135) | `if (MEM_64bits() \|\| (HUF_MAX_TABLELOG<=12)) \` | branch-specific rejection/error | [ ] |
| 3120 | `HUF_decodeStreamX4` (src/legacy/zstd_v04.c:2162) | `if (p < pEnd)` | branch-specific rejection/error | [ ] |
| 3121 | `HUF_decompress4X4_usingDTable` (src/legacy/zstd_v04.c:2173) | `if (cSrcSize < 10) return ERROR(corruption_detected);   /* strict minimum : jump table + 1 byte per stream */` | `ERROR(corruption_detected)` | [ ] |
| 3122 | `HUF_decompress4X4_usingDTable` (src/legacy/zstd_v04.c:2208) | `if (length4 > cSrcSize) return ERROR(corruption_detected);   /* overflow */` | `ERROR(corruption_detected)` | [ ] |
| 3123 | `HUF_decompress4X4_usingDTable` (src/legacy/zstd_v04.c:2243) | `if (op1 > opStart2) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3124 | `HUF_decompress4X4_usingDTable` (src/legacy/zstd_v04.c:2244) | `if (op2 > opStart3) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3125 | `HUF_decompress4X4_usingDTable` (src/legacy/zstd_v04.c:2245) | `if (op3 > opStart4) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3126 | `HUF_decompress4X4_usingDTable` (src/legacy/zstd_v04.c:2256) | `if (!endSignal) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3127 | `HUF_decompress4X4` (src/legacy/zstd_v04.c:2271) | `if (hSize >= cSrcSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 3128 | `HUF_decompress` (src/legacy/zstd_v04.c:2318) | `if (dstSize == 0) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [ ] |
| 3129 | `HUF_decompress` (src/legacy/zstd_v04.c:2319) | `if (cSrcSize > dstSize) return ERROR(corruption_detected);   /* invalid */` | `ERROR(corruption_detected)` | [ ] |
| 3130 | `HUF_decompress` (src/legacy/zstd_v04.c:2330) | `if (Dtime[1] < Dtime[0]) algoNb = 1;` | branch-specific rejection/error | [ ] |
| 3131 | `ZSTD_createDCtx` (src/legacy/zstd_v04.c:2472) | `if (dctx==NULL) return NULL;` | `NULL` | [ ] |
| 3132 | `ZSTD_decodeFrameHeader_Part1` (src/legacy/zstd_v04.c:2494) | `if (srcSize != ZSTD_frameHeaderSize_min) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 3133 | `ZSTD_decodeFrameHeader_Part1` (src/legacy/zstd_v04.c:2496) | `if (magicNumber != ZSTD_MAGICNUMBER) return ERROR(prefix_unknown);` | `ERROR(prefix_unknown)` | [ ] |
| 3134 | `ZSTD_getFrameParams` (src/legacy/zstd_v04.c:2505) | `if (srcSize < ZSTD_frameHeaderSize_min) return ZSTD_frameHeaderSize_max;` | `ERROR(prefix_unknown)` | [ ] |
| 3135 | `ZSTD_getFrameParams` (src/legacy/zstd_v04.c:2507) | `if (magicNumber != ZSTD_MAGICNUMBER) return ERROR(prefix_unknown);` | `ERROR(prefix_unknown)` | [ ] |
| 3136 | `ZSTD_getFrameParams` (src/legacy/zstd_v04.c:2510) | `if ((((const BYTE*)src)[4] >> 4) != 0) return ERROR(frameParameter_unsupported);   /* reserved bits */` | `ERROR(frameParameter_unsupported)` | [ ] |
| 3137 | `ZSTD_decodeFrameHeader_Part2` (src/legacy/zstd_v04.c:2521) | `if (srcSize != zc->headerSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 3138 | `ZSTD_decodeFrameHeader_Part2` (src/legacy/zstd_v04.c:2523) | `if ((MEM_32bits()) && (zc->params.windowLog > 25)) return ERROR(frameParameter_unsupported);` | `ERROR(frameParameter_unsupported)` | [ ] |
| 3139 | `ZSTD_getcBlockSize` (src/legacy/zstd_v04.c:2534) | `if (srcSize < 3) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 3140 | `ZSTD_getcBlockSize` (src/legacy/zstd_v04.c:2542) | `if (bpPtr->blockType == bt_end) return 0;` | branch-specific rejection/error | [ ] |
| 3141 | `ZSTD_getcBlockSize` (src/legacy/zstd_v04.c:2543) | `if (bpPtr->blockType == bt_rle) return 1;` | branch-specific rejection/error | [ ] |
| 3142 | `ZSTD_copyRawBlock` (src/legacy/zstd_v04.c:2549) | `if (srcSize > maxDstSize) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [ ] |
| 3143 | `ZSTD_copyRawBlock` (src/legacy/zstd_v04.c:2550) | `if (srcSize > 0) {` | branch-specific rejection/error | [ ] |
| 3144 | `ZSTD_decompressLiterals` (src/legacy/zstd_v04.c:2567) | `if (litSize > *maxDstSizePtr) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3145 | `ZSTD_decompressLiterals` (src/legacy/zstd_v04.c:2568) | `if (litCSize + 5 > srcSize) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3146 | `ZSTD_decompressLiterals` (src/legacy/zstd_v04.c:2570) | `if (HUF_isError(HUF_decompress(dst, litSize, ip+5, litCSize))) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3147 | `ZSTD_decodeLiteralsBlock` (src/legacy/zstd_v04.c:2585) | `if (srcSize < MIN_CBLOCK_SIZE) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3148 | `ZSTD_decodeLiteralsBlock` (src/legacy/zstd_v04.c:2602) | `if (litSize > srcSize-11)   /* risk of reading too far with wildcopy */` | `ERROR(corruption_detected)` | [ ] |
| 3149 | `ZSTD_decodeLiteralsBlock` (src/legacy/zstd_v04.c:2604) | `if (litSize > BLOCKSIZE) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3150 | `ZSTD_decodeLiteralsBlock` (src/legacy/zstd_v04.c:2605) | `if (litSize > srcSize-3) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3151 | `ZSTD_decodeLiteralsBlock` (src/legacy/zstd_v04.c:2619) | `if (litSize > BLOCKSIZE) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3152 | `ZSTD_decodeLiteralsBlock` (src/legacy/zstd_v04.c:2626) | `return ERROR(corruption_detected);   /* forbidden nominal case */` | `ERROR(corruption_detected)` | [ ] |
| 3153 | `ZSTD_decodeSeqHeaders` (src/legacy/zstd_v04.c:2643) | `if (srcSize < 5) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 3154 | `ZSTD_decodeSeqHeaders` (src/legacy/zstd_v04.c:2667) | `if (ip > iend-3) return ERROR(srcSize_wrong); /* min : all 3 are "raw", hence no header, but at least xxLog bits per type */` | `ERROR(srcSize_wrong)` | [ ] |
| 3155 | `ZSTD_decodeSeqHeaders` (src/legacy/zstd_v04.c:2686) | `if (FSE_isError(headerSize)) return ERROR(GENERIC);` | `ERROR(GENERIC)` | [ ] |
| 3156 | `ZSTD_decodeSeqHeaders` (src/legacy/zstd_v04.c:2687) | `if (LLlog > LLFSELog) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3157 | `ZSTD_decodeSeqHeaders` (src/legacy/zstd_v04.c:2696) | `if (ip > iend-2) return ERROR(srcSize_wrong);   /* min : "raw", hence no header, but at least xxLog bits */` | `ERROR(srcSize_wrong)` | [ ] |
| 3158 | `ZSTD_decodeSeqHeaders` (src/legacy/zstd_v04.c:2705) | `if (FSE_isError(headerSize)) return ERROR(GENERIC);` | `ERROR(GENERIC)` | [ ] |
| 3159 | `ZSTD_decodeSeqHeaders` (src/legacy/zstd_v04.c:2706) | `if (Offlog > OffFSELog) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3160 | `ZSTD_decodeSeqHeaders` (src/legacy/zstd_v04.c:2715) | `if (ip > iend-2) return ERROR(srcSize_wrong); /* min : "raw", hence no header, but at least xxLog bits */` | `ERROR(srcSize_wrong)` | [ ] |
| 3161 | `ZSTD_decodeSeqHeaders` (src/legacy/zstd_v04.c:2723) | `if (FSE_isError(headerSize)) return ERROR(GENERIC);` | `ERROR(GENERIC)` | [ ] |
| 3162 | `ZSTD_decodeSeqHeaders` (src/legacy/zstd_v04.c:2724) | `if (MLlog > MLFSELog) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3163 | `ZSTD_decodeSequence` (src/legacy/zstd_v04.c:2764) | `if (add < 255) litLength += add;` | branch-specific rejection/error | [ ] |
| 3164 | `ZSTD_decodeSequence` (src/legacy/zstd_v04.c:2765) | `else if (dumps + 3 <= de) {` | branch-specific rejection/error | [ ] |
| 3165 | `ZSTD_decodeSequence` (src/legacy/zstd_v04.c:2769) | `if (dumps >= de) { dumps = de-1; }  /* late correction, to avoid read overflow (data is now corrupted anyway) */` | branch-specific rejection/error | [ ] |
| 3166 | `ZSTD_decodeSequence` (src/legacy/zstd_v04.c:2779) | `if (MEM_32bits()) BIT_reloadDStream(&(seqState->DStream));` | branch-specific rejection/error | [ ] |
| 3167 | `ZSTD_decodeSequence` (src/legacy/zstd_v04.c:2781) | `if (offsetCode==0) nbBits = 0;   /* cmove */` | branch-specific rejection/error | [ ] |
| 3168 | `ZSTD_decodeSequence` (src/legacy/zstd_v04.c:2783) | `if (MEM_32bits()) BIT_reloadDStream(&(seqState->DStream));` | branch-specific rejection/error | [ ] |
| 3169 | `ZSTD_decodeSequence` (src/legacy/zstd_v04.c:2784) | `if (offsetCode==0) offset = prevOffset;   /* cmove */` | branch-specific rejection/error | [ ] |
| 3170 | `ZSTD_decodeSequence` (src/legacy/zstd_v04.c:2785) | `if (offsetCode \| !litLength) seqState->prevOffset = seq->offset;   /* cmove */` | branch-specific rejection/error | [ ] |
| 3171 | `ZSTD_decodeSequence` (src/legacy/zstd_v04.c:2792) | `if (add < 255) matchLength += add;` | branch-specific rejection/error | [ ] |
| 3172 | `ZSTD_decodeSequence` (src/legacy/zstd_v04.c:2793) | `else if (dumps + 3 <= de){` | branch-specific rejection/error | [ ] |
| 3173 | `ZSTD_decodeSequence` (src/legacy/zstd_v04.c:2797) | `if (dumps >= de) { dumps = de-1; }  /* late correction, to avoid read overflow (data is now corrupted anyway) */` | branch-specific rejection/error | [ ] |
| 3174 | `ZSTD_execSequence` (src/legacy/zstd_v04.c:2826) | `if (seqLength > (size_t)(oend - op)) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [ ] |
| 3175 | `ZSTD_execSequence` (src/legacy/zstd_v04.c:2827) | `if (sequence.litLength > (size_t)(litLimit - *litPtr)) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3176 | `ZSTD_execSequence` (src/legacy/zstd_v04.c:2829) | `if (oLitEnd > oend_8) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [ ] |
| 3177 | `ZSTD_execSequence` (src/legacy/zstd_v04.c:2831) | `if (oMatchEnd > oend) return ERROR(dstSize_tooSmall);   /* overwrite beyond dst buffer */` | `ERROR(dstSize_tooSmall)` | [ ] |
| 3178 | `ZSTD_execSequence` (src/legacy/zstd_v04.c:2832) | `if (litEnd > litLimit) return ERROR(corruption_detected);   /* overRead beyond lit buffer */` | `ERROR(corruption_detected)` | [ ] |
| 3179 | `ZSTD_execSequence` (src/legacy/zstd_v04.c:2840) | `if (sequence.offset > (size_t)(oLitEnd - base))` | `ERROR(corruption_detected)` | [ ] |
| 3180 | `ZSTD_execSequence` (src/legacy/zstd_v04.c:2843) | `if (sequence.offset > (size_t)(oLitEnd - vBase))` | `ERROR(corruption_detected)` | [ ] |
| 3181 | `ZSTD_execSequence` (src/legacy/zstd_v04.c:2844) | `return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3182 | `ZSTD_execSequence` (src/legacy/zstd_v04.c:2846) | `if (match + sequence.matchLength <= dictEnd)` | branch-specific rejection/error | [ ] |
| 3183 | `ZSTD_execSequence` (src/legacy/zstd_v04.c:2858) | `if (op > oend_8 \|\| sequence.matchLength < MINMATCH) {` | branch-specific rejection/error | [ ] |
| 3184 | `ZSTD_execSequence` (src/legacy/zstd_v04.c:2867) | `if (sequence.offset < 8) {` | branch-specific rejection/error | [ ] |
| 3185 | `ZSTD_execSequence` (src/legacy/zstd_v04.c:2882) | `if (oMatchEnd > oend-(16-MINMATCH))` | branch-specific rejection/error | [ ] |
| 3186 | `ZSTD_execSequence` (src/legacy/zstd_v04.c:2884) | `if (op < oend_8)` | branch-specific rejection/error | [ ] |
| 3187 | `ZSTD_decompressSequences` (src/legacy/zstd_v04.c:2940) | `if (ERR_isError(errorCode)) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3188 | `ZSTD_decompressSequences` (src/legacy/zstd_v04.c:2956) | `if ( !BIT_endOfDStream(&(seqState.DStream)) ) return ERROR(corruption_detected);   /* DStream should be entirely and exactly consumed; otherwise data is corrupted */` | `ERROR(corruption_detected)` | [ ] |
| 3189 | `ZSTD_decompressSequences` (src/legacy/zstd_v04.c:2961) | `if (litPtr > litEnd) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3190 | `ZSTD_decompressSequences` (src/legacy/zstd_v04.c:2962) | `if (op+lastLLSize > oend) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [ ] |
| 3191 | `ZSTD_decompressSequences` (src/legacy/zstd_v04.c:2963) | `if (lastLLSize > 0) {` | branch-specific rejection/error | [ ] |
| 3192 | `ZSTD_checkContinuity` (src/legacy/zstd_v04.c:2976) | `if (dst != dctx->previousDstEnd)   /* not contiguous */` | branch-specific rejection/error | [ ] |
| 3193 | `ZSTD_decompressBlock_internal` (src/legacy/zstd_v04.c:2994) | `if (srcSize > BLOCKSIZE) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3194 | `ZSTD_decompress_usingDict` (src/legacy/zstd_v04.c:3036) | `if (srcSize < ZSTD_frameHeaderSize_min+ZSTD_blockHeaderSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 3195 | `ZSTD_decompress_usingDict` (src/legacy/zstd_v04.c:3039) | `if (srcSize < frameHeaderSize+ZSTD_blockHeaderSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 3196 | `ZSTD_decompress_usingDict` (src/legacy/zstd_v04.c:3054) | `if (cBlockSize > remainingSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 3197 | `ZSTD_decompress_usingDict` (src/legacy/zstd_v04.c:3065) | `return ERROR(GENERIC);   /* not yet supported */` | `ERROR(GENERIC)` | [ ] |
| 3198 | `ZSTD_decompress_usingDict` (src/legacy/zstd_v04.c:3069) | `if (remainingSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 3199 | `ZSTD_decompress_usingDict` (src/legacy/zstd_v04.c:3072) | `return ERROR(GENERIC);   /* impossible */` | `ERROR(GENERIC)` | [ ] |
| 3200 | `ZSTD_decompress_usingDict` (src/legacy/zstd_v04.c:3074) | `if (cBlockSize == 0) break;   /* bt_end */` | branch-specific rejection/error | [ ] |
| 3201 | `ZSTDv04_findFrameSizeInfoLegacy` (src/legacy/zstd_v04.c:3101) | `if (srcSize < ZSTD_frameHeaderSize_min) {` | `ERROR(srcSize_wrong)` | [ ] |
| 3202 | `ZSTDv04_findFrameSizeInfoLegacy` (src/legacy/zstd_v04.c:3102) | `ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, ERROR(srcSize_wrong));` | `ERROR(srcSize_wrong)` | [ ] |
| 3203 | `ZSTDv04_findFrameSizeInfoLegacy` (src/legacy/zstd_v04.c:3106) | `ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, ERROR(prefix_unknown));` | `ERROR(prefix_unknown)` | [ ] |
| 3204 | `ZSTDv04_findFrameSizeInfoLegacy` (src/legacy/zstd_v04.c:3122) | `if (cBlockSize > remainingSize) {` | `ERROR(srcSize_wrong)` | [ ] |
| 3205 | `ZSTDv04_findFrameSizeInfoLegacy` (src/legacy/zstd_v04.c:3123) | `ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, ERROR(srcSize_wrong));` | `ERROR(srcSize_wrong)` | [ ] |
| 3206 | `ZSTDv04_findFrameSizeInfoLegacy` (src/legacy/zstd_v04.c:3127) | `if (cBlockSize == 0) break;   /* bt_end */` | branch-specific rejection/error | [ ] |
| 3207 | `ZSTD_decompressContinue` (src/legacy/zstd_v04.c:3149) | `if (srcSize != ctx->expected) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 3208 | `ZSTD_decompressContinue` (src/legacy/zstd_v04.c:3157) | `if (srcSize != ZSTD_frameHeaderSize_min) return ERROR(srcSize_wrong);   /* impossible */` | `ERROR(srcSize_wrong)` | [ ] |
| 3209 | `ZSTD_decompressContinue` (src/legacy/zstd_v04.c:3159) | `if (ZSTD_isError(ctx->headerSize)) return ctx->headerSize;` | `ERROR(GENERIC)` | [ ] |
| 3210 | `ZSTD_decompressContinue` (src/legacy/zstd_v04.c:3161) | `if (ctx->headerSize > ZSTD_frameHeaderSize_min) return ERROR(GENERIC);   /* impossible */` | `ERROR(GENERIC)` | [ ] |
| 3211 | `ZSTD_decompressContinue` (src/legacy/zstd_v04.c:3203) | `return ERROR(GENERIC);   /* not yet handled */` | `ERROR(GENERIC)` | [ ] |
| 3212 | `ZSTD_decompressContinue` (src/legacy/zstd_v04.c:3209) | `return ERROR(GENERIC);` | `ERROR(GENERIC)` | [ ] |
| 3213 | `ZSTD_decompressContinue` (src/legacy/zstd_v04.c:3218) | `return ERROR(GENERIC);   /* impossible */` | `ERROR(GENERIC)` | [ ] |
| 3214 | `ZBUFF_createDCtx` (src/legacy/zstd_v04.c:3327) | `if (zbc==NULL) return NULL;` | `NULL` | [ ] |
| 3215 | `ZBUFF_freeDCtx` (src/legacy/zstd_v04.c:3336) | `if (zbc==NULL) return 0;   /* support free on null */` | branch-specific rejection/error | [ ] |
| 3216 | `ZBUFF_limitCopy` (src/legacy/zstd_v04.c:3365) | `if (length > 0) {` | branch-specific rejection/error | [ ] |
| 3217 | `ZBUFF_decompressContinue` (src/legacy/zstd_v04.c:3390) | `DEBUGLOG(5, "ZBUFF_decompressContinue: stage==ZBUFFds_init => ERROR(init_missing)");` | `ERROR(init_missing)` | [ ] |
| 3218 | `ZBUFF_decompressContinue` (src/legacy/zstd_v04.c:3391) | `return ERROR(init_missing);` | `ERROR(init_missing)` | [ ] |
| 3219 | `ZBUFF_decompressContinue` (src/legacy/zstd_v04.c:3429) | `if (zbc->inBuffSize < neededInSize) {` | `ERROR(memory_allocation)` | [ ] |
| 3220 | `ZBUFF_decompressContinue` (src/legacy/zstd_v04.c:3433) | `if (zbc->inBuff == NULL) return ERROR(memory_allocation);` | `ERROR(memory_allocation)` | [ ] |
| 3221 | `ZBUFF_decompressContinue` (src/legacy/zstd_v04.c:3435) | `if (zbc->outBuffSize < neededOutSize) {` | `ERROR(memory_allocation)` | [ ] |
| 3222 | `ZBUFF_decompressContinue` (src/legacy/zstd_v04.c:3439) | `if (zbc->outBuff == NULL) return ERROR(memory_allocation);` | `ERROR(memory_allocation)` | [ ] |
| 3223 | `ZBUFF_decompressContinue` (src/legacy/zstd_v04.c:3441) | `if (zbc->dictSize)` | branch-specific rejection/error | [ ] |
| 3224 | `ZBUFF_decompressContinue` (src/legacy/zstd_v04.c:3443) | `if (zbc->hPos) {` | branch-specific rejection/error | [ ] |
| 3225 | `ZBUFF_decompressContinue` (src/legacy/zstd_v04.c:3456) | `if (neededInSize==0)   /* end of frame */` | branch-specific rejection/error | [ ] |
| 3226 | `ZBUFF_decompressContinue` (src/legacy/zstd_v04.c:3462) | `if ((size_t)(iend-ip) >= neededInSize)` | branch-specific rejection/error | [ ] |
| 3227 | `ZBUFF_decompressContinue` (src/legacy/zstd_v04.c:3484) | `if (toLoad > zbc->inBuffSize - zbc->inPos) return ERROR(corruption_detected);   /* should never happen */` | `ERROR(corruption_detected)` | [ ] |
| 3228 | `ZBUFF_decompressContinue` (src/legacy/zstd_v04.c:3488) | `if (loadedSize < toLoad) { notDone = 0; break; }   /* not enough input, wait for more */` | branch-specific rejection/error | [ ] |
| 3229 | `ZBUFF_decompressContinue` (src/legacy/zstd_v04.c:3495) | `if (!decodedSize) { zbc->stage = ZBUFFds_read; break; }   /* this was just a header */` | branch-specific rejection/error | [ ] |
| 3230 | `ZBUFF_decompressContinue` (src/legacy/zstd_v04.c:3511) | `if (zbc->outStart + BLOCKSIZE > zbc->outBuffSize)` | branch-specific rejection/error | [ ] |
| 3231 | `ZBUFF_decompressContinue` (src/legacy/zstd_v04.c:3519) | `default: return ERROR(GENERIC);   /* impossible */` | `ERROR(GENERIC)` | [ ] |
| 3232 | `ZBUFF_decompressContinue` (src/legacy/zstd_v04.c:3528) | `if (nextSrcSizeHint > 3) nextSrcSizeHint+= 3;   /* get the next block header while at it */` | branch-specific rejection/error | [ ] |
| 3233 | `ZSTDv04_decompress` (src/legacy/zstd_v04.c:3560) | `if (dctx==NULL) return ERROR(memory_allocation);` | `ERROR(memory_allocation)` | [ ] |
| 3234 | `BITv05_initDStream` (src/legacy/zstd_v05.c:736) | `if (srcSize < 1) { memset(bitD, 0, sizeof(*bitD)); return ERROR(srcSize_wrong); }` | `ERROR(srcSize_wrong)` | [ ] |
| 3235 | `BITv05_initDStream` (src/legacy/zstd_v05.c:738) | `if (srcSize >=  sizeof(size_t)) {  /* normal case */` | branch-specific rejection/error | [ ] |
| 3236 | `BITv05_initDStream` (src/legacy/zstd_v05.c:744) | `if (contain32 == 0) return ERROR(GENERIC);   /* endMark not present */` | `ERROR(GENERIC)` | [ ] |
| 3237 | `BITv05_initDStream` (src/legacy/zstd_v05.c:762) | `if (contain32 == 0) return ERROR(GENERIC);   /* endMark not present */` | `ERROR(GENERIC)` | [ ] |
| 3238 | `BITv05_reloadDStream` (src/legacy/zstd_v05.c:807) | `if (bitD->bitsConsumed > (sizeof(bitD->bitContainer)*8))  /* should never happen */` | branch-specific rejection/error | [ ] |
| 3239 | `BITv05_reloadDStream` (src/legacy/zstd_v05.c:810) | `if (bitD->ptr >= bitD->start + sizeof(bitD->bitContainer)) {` | branch-specific rejection/error | [ ] |
| 3240 | `BITv05_reloadDStream` (src/legacy/zstd_v05.c:816) | `if (bitD->ptr == bitD->start) {` | branch-specific rejection/error | [ ] |
| 3241 | `BITv05_reloadDStream` (src/legacy/zstd_v05.c:817) | `if (bitD->bitsConsumed < sizeof(bitD->bitContainer)*8) return BITv05_DStream_endOfBuffer;` | branch-specific rejection/error | [ ] |
| 3242 | `BITv05_reloadDStream` (src/legacy/zstd_v05.c:823) | `if (bitD->ptr - nbBytes < bitD->start) {` | branch-specific rejection/error | [ ] |
| 3243 | `FSEv05_createDTable` (src/legacy/zstd_v05.c:1148) | `if (tableLog > FSEv05_TABLELOG_ABSOLUTE_MAX) tableLog = FSEv05_TABLELOG_ABSOLUTE_MAX;` | branch-specific rejection/error | [ ] |
| 3244 | `FSEv05_buildDTable` (src/legacy/zstd_v05.c:1173) | `if (maxSymbolValue > FSEv05_MAX_SYMBOL_VALUE) return ERROR(maxSymbolValue_tooLarge);` | `ERROR(maxSymbolValue_tooLarge)` | [ ] |
| 3245 | `FSEv05_buildDTable` (src/legacy/zstd_v05.c:1174) | `if (tableLog > FSEv05_MAX_TABLELOG) return ERROR(tableLog_tooLarge);` | `ERROR(tableLog_tooLarge)` | [ ] |
| 3246 | `FSEv05_buildDTable` (src/legacy/zstd_v05.c:1184) | `if (normalizedCounter[s] >= largeLimit) noLarge=0;` | branch-specific rejection/error | [ ] |
| 3247 | `FSEv05_buildDTable` (src/legacy/zstd_v05.c:1197) | `if (position!=0) return ERROR(GENERIC);   /* position must reach all cells once, otherwise normalizedCounter is incorrect */` | `ERROR(GENERIC)` | [ ] |
| 3248 | `FSEv05_readNCount` (src/legacy/zstd_v05.c:1244) | `if (hbSize < 4) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 3249 | `FSEv05_readNCount` (src/legacy/zstd_v05.c:1247) | `if (nbBits > FSEv05_TABLELOG_ABSOLUTE_MAX) return ERROR(tableLog_tooLarge);` | `ERROR(tableLog_tooLarge)` | [ ] |
| 3250 | `FSEv05_readNCount` (src/legacy/zstd_v05.c:1260) | `if (ip < iend-5) {` | branch-specific rejection/error | [ ] |
| 3251 | `FSEv05_readNCount` (src/legacy/zstd_v05.c:1274) | `if (n0 > *maxSVPtr) return ERROR(maxSymbolValue_tooSmall);` | `ERROR(maxSymbolValue_tooSmall)` | [ ] |
| 3252 | `FSEv05_readNCount` (src/legacy/zstd_v05.c:1276) | `if ((ip <= iend-7) \|\| (ip + (bitCount>>3) <= iend-4)) {` | branch-specific rejection/error | [ ] |
| 3253 | `FSEv05_readNCount` (src/legacy/zstd_v05.c:1288) | `if ((bitStream & (threshold-1)) < (U32)max) {` | branch-specific rejection/error | [ ] |
| 3254 | `FSEv05_readNCount` (src/legacy/zstd_v05.c:1293) | `if (count >= threshold) count -= max;` | branch-specific rejection/error | [ ] |
| 3255 | `FSEv05_readNCount` (src/legacy/zstd_v05.c:1306) | `if ((ip <= iend-7) \|\| (ip + (bitCount>>3) <= iend-4)) {` | branch-specific rejection/error | [ ] |
| 3256 | `FSEv05_readNCount` (src/legacy/zstd_v05.c:1315) | `if (remaining != 1) return ERROR(GENERIC);` | `ERROR(GENERIC)` | [ ] |
| 3257 | `FSEv05_readNCount` (src/legacy/zstd_v05.c:1319) | `if ((size_t)(ip-istart) > hbSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 3258 | `FSEv05_buildDTable_raw` (src/legacy/zstd_v05.c:1358) | `if (nbBits < 1) return ERROR(GENERIC);         /* min size */` | `ERROR(GENERIC)` | [ ] |
| 3259 | `FSEv05_decompress_usingDTable_generic` (src/legacy/zstd_v05.c:1400) | `if (FSEv05_MAX_TABLELOG*2+7 > sizeof(bitD.bitContainer)*8)    /* This test must be static */` | branch-specific rejection/error | [ ] |
| 3260 | `FSEv05_decompress_usingDTable_generic` (src/legacy/zstd_v05.c:1405) | `if (FSEv05_MAX_TABLELOG*4+7 > sizeof(bitD.bitContainer)*8)    /* This test must be static */` | branch-specific rejection/error | [ ] |
| 3261 | `FSEv05_decompress_usingDTable_generic` (src/legacy/zstd_v05.c:1406) | `{ if (BITv05_reloadDStream(&bitD) > BITv05_DStream_unfinished) { op+=2; break; } }` | branch-specific rejection/error | [ ] |
| 3262 | `FSEv05_decompress_usingDTable_generic` (src/legacy/zstd_v05.c:1410) | `if (FSEv05_MAX_TABLELOG*2+7 > sizeof(bitD.bitContainer)*8)    /* This test must be static */` | branch-specific rejection/error | [ ] |
| 3263 | `FSEv05_decompress_usingDTable_generic` (src/legacy/zstd_v05.c:1419) | `if ( (BITv05_reloadDStream(&bitD)>BITv05_DStream_completed) \|\| (op==omax) \|\| (BITv05_endOfDStream(&bitD) && (fast \|\| FSEv05_endOfDState(&state1))) )` | branch-specific rejection/error | [ ] |
| 3264 | `FSEv05_decompress_usingDTable_generic` (src/legacy/zstd_v05.c:1424) | `if ( (BITv05_reloadDStream(&bitD)>BITv05_DStream_completed) \|\| (op==omax) \|\| (BITv05_endOfDStream(&bitD) && (fast \|\| FSEv05_endOfDState(&state2))) )` | branch-specific rejection/error | [ ] |
| 3265 | `FSEv05_decompress_usingDTable_generic` (src/legacy/zstd_v05.c:1434) | `if (op==omax) return ERROR(dstSize_tooSmall);   /* dst buffer is full, but cSrc unfinished */` | `ERROR(dstSize_tooSmall)` | [ ] |
| 3266 | `FSEv05_decompress_usingDTable_generic` (src/legacy/zstd_v05.c:1436) | `return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3267 | `FSEv05_decompress` (src/legacy/zstd_v05.c:1464) | `if (cSrcSize<2) return ERROR(srcSize_wrong);   /* too small input size */` | `ERROR(srcSize_wrong)` | [ ] |
| 3268 | `FSEv05_decompress` (src/legacy/zstd_v05.c:1469) | `if (errorCode >= cSrcSize) return ERROR(srcSize_wrong);   /* too small input size */` | `ERROR(srcSize_wrong)` | [ ] |
| 3269 | `HUFv05_readStats` (src/legacy/zstd_v05.c:1753) | `if (!srcSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 3270 | `HUFv05_readStats` (src/legacy/zstd_v05.c:1757) | `if (iSize >= 128)  { /* special header */` | branch-specific rejection/error | [ ] |
| 3271 | `HUFv05_readStats` (src/legacy/zstd_v05.c:1758) | `if (iSize >= (242)) {  /* RLE */` | branch-specific rejection/error | [ ] |
| 3272 | `HUFv05_readStats` (src/legacy/zstd_v05.c:1767) | `if (iSize+1 > srcSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 3273 | `HUFv05_readStats` (src/legacy/zstd_v05.c:1768) | `if (oSize >= hwSize) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3274 | `HUFv05_readStats` (src/legacy/zstd_v05.c:1775) | `if (iSize+1 > srcSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 3275 | `HUFv05_readStats` (src/legacy/zstd_v05.c:1784) | `if (huffWeight[n] >= HUFv05_ABSOLUTEMAX_TABLELOG) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3276 | `HUFv05_readStats` (src/legacy/zstd_v05.c:1788) | `if (weightTotal == 0) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3277 | `HUFv05_readStats` (src/legacy/zstd_v05.c:1792) | `if (tableLog > HUFv05_ABSOLUTEMAX_TABLELOG) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3278 | `HUFv05_readStats` (src/legacy/zstd_v05.c:1798) | `if (verif != rest) return ERROR(corruption_detected);    /* last value must be a clean power of 2 */` | `ERROR(corruption_detected)` | [ ] |
| 3279 | `HUFv05_readStats` (src/legacy/zstd_v05.c:1804) | `if ((rankStats[1] < 2) \|\| (rankStats[1] & 1)) return ERROR(corruption_detected);   /* by construction : at least 2 elts of rank 1, must be even */` | `ERROR(corruption_detected)` | [ ] |
| 3280 | `HUFv05_readDTableX2` (src/legacy/zstd_v05.c:1836) | `if (tableLog > DTable[0]) return ERROR(tableLog_tooLarge);   /* DTable is too small */` | `ERROR(tableLog_tooLarge)` | [ ] |
| 3281 | `HUFv05_decodeSymbolX2` (src/legacy/zstd_v05.c:1874) | `if (MEM_64bits() \|\| (HUFv05_MAX_TABLELOG<=12)) \` | branch-specific rejection/error | [ ] |
| 3282 | `HUFv05_decompress1X2_usingDTable` (src/legacy/zstd_v05.c:1916) | `if (dstSize <= cSrcSize) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [ ] |
| 3283 | `HUFv05_decompress1X2_usingDTable` (src/legacy/zstd_v05.c:1923) | `if (!BITv05_endOfDStream(&bitD)) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3284 | `HUFv05_decompress1X2` (src/legacy/zstd_v05.c:1936) | `if (errorCode >= cSrcSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 3285 | `HUFv05_decompress4X2_usingDTable` (src/legacy/zstd_v05.c:1950) | `if (cSrcSize < 10) return ERROR(corruption_detected);   /* strict minimum : jump table + 1 byte per stream */` | `ERROR(corruption_detected)` | [ ] |
| 3286 | `HUFv05_decompress4X2_usingDTable` (src/legacy/zstd_v05.c:1984) | `if (length4 > cSrcSize) return ERROR(corruption_detected);   /* overflow */` | `ERROR(corruption_detected)` | [ ] |
| 3287 | `HUFv05_decompress4X2_usingDTable` (src/legacy/zstd_v05.c:2017) | `if (op1 > opStart2) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3288 | `HUFv05_decompress4X2_usingDTable` (src/legacy/zstd_v05.c:2018) | `if (op2 > opStart3) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3289 | `HUFv05_decompress4X2_usingDTable` (src/legacy/zstd_v05.c:2019) | `if (op3 > opStart4) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3290 | `HUFv05_decompress4X2_usingDTable` (src/legacy/zstd_v05.c:2030) | `if (!endSignal) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3291 | `HUFv05_decompress4X2` (src/legacy/zstd_v05.c:2046) | `if (errorCode >= cSrcSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 3292 | `HUFv05_fillDTableX4Level2` (src/legacy/zstd_v05.c:2071) | `if (minWeight>1) {` | branch-specific rejection/error | [ ] |
| 3293 | `HUFv05_fillDTableX4` (src/legacy/zstd_v05.c:2121) | `if (targetLog-nbBits >= minBits) {   /* enough room for a second symbol */` | branch-specific rejection/error | [ ] |
| 3294 | `HUFv05_fillDTableX4` (src/legacy/zstd_v05.c:2124) | `if (minWeight < 1) minWeight = 1;` | branch-specific rejection/error | [ ] |
| 3295 | `HUFv05_readDTableX4` (src/legacy/zstd_v05.c:2160) | `if (memLog > HUFv05_ABSOLUTEMAX_TABLELOG) return ERROR(tableLog_tooLarge);` | `ERROR(tableLog_tooLarge)` | [ ] |
| 3296 | `HUFv05_readDTableX4` (src/legacy/zstd_v05.c:2167) | `if (tableLog > memLog) return ERROR(tableLog_tooLarge);   /* DTable can't fit code depth */` | `ERROR(tableLog_tooLarge)` | [ ] |
| 3297 | `HUFv05_decodeLastSymbolX4` (src/legacy/zstd_v05.c:2237) | `if (DStream->bitsConsumed < (sizeof(DStream->bitContainer)*8)) {` | branch-specific rejection/error | [ ] |
| 3298 | `HUFv05_decodeLastSymbolX4` (src/legacy/zstd_v05.c:2239) | `if (DStream->bitsConsumed > (sizeof(DStream->bitContainer)*8))` | branch-specific rejection/error | [ ] |
| 3299 | `HUFv05_decodeLastSymbolX4` (src/legacy/zstd_v05.c:2250) | `if (MEM_64bits() \|\| (HUFv05_MAX_TABLELOG<=12)) \` | branch-specific rejection/error | [ ] |
| 3300 | `HUFv05_decodeStreamX4` (src/legacy/zstd_v05.c:2276) | `if (p < pEnd)` | branch-specific rejection/error | [ ] |
| 3301 | `HUFv05_decompress1X4_usingDTable` (src/legacy/zstd_v05.c:2306) | `if (!BITv05_endOfDStream(&bitD)) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3302 | `HUFv05_decompress1X4` (src/legacy/zstd_v05.c:2319) | `if (hSize >= cSrcSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 3303 | `HUFv05_decompress4X4_usingDTable` (src/legacy/zstd_v05.c:2331) | `if (cSrcSize < 10) return ERROR(corruption_detected);   /* strict minimum : jump table + 1 byte per stream */` | `ERROR(corruption_detected)` | [ ] |
| 3304 | `HUFv05_decompress4X4_usingDTable` (src/legacy/zstd_v05.c:2366) | `if (length4 > cSrcSize) return ERROR(corruption_detected);   /* overflow */` | `ERROR(corruption_detected)` | [ ] |
| 3305 | `HUFv05_decompress4X4_usingDTable` (src/legacy/zstd_v05.c:2400) | `if (op1 > opStart2) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3306 | `HUFv05_decompress4X4_usingDTable` (src/legacy/zstd_v05.c:2401) | `if (op2 > opStart3) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3307 | `HUFv05_decompress4X4_usingDTable` (src/legacy/zstd_v05.c:2402) | `if (op3 > opStart4) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3308 | `HUFv05_decompress4X4_usingDTable` (src/legacy/zstd_v05.c:2413) | `if (!endSignal) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3309 | `HUFv05_decompress4X4` (src/legacy/zstd_v05.c:2428) | `if (hSize >= cSrcSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 3310 | `HUFv05_decompress` (src/legacy/zstd_v05.c:2475) | `if (dstSize == 0) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [ ] |
| 3311 | `HUFv05_decompress` (src/legacy/zstd_v05.c:2476) | `if (cSrcSize >= dstSize) return ERROR(corruption_detected);   /* invalid, or not compressed, but not compressed already dealt with */` | `ERROR(corruption_detected)` | [ ] |
| 3312 | `HUFv05_decompress` (src/legacy/zstd_v05.c:2486) | `if (Dtime[1] < Dtime[0]) algoNb = 1;` | branch-specific rejection/error | [ ] |
| 3313 | `ZSTDv05_createDCtx` (src/legacy/zstd_v05.c:2632) | `if (dctx==NULL) return NULL;` | `NULL` | [ ] |
| 3314 | `ZSTDv05_decodeFrameHeader_Part1` (src/legacy/zstd_v05.c:2743) | `return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 3315 | `ZSTDv05_decodeFrameHeader_Part1` (src/legacy/zstd_v05.c:2745) | `if (magicNumber != ZSTDv05_MAGICNUMBER) return ERROR(prefix_unknown);` | `ERROR(prefix_unknown)` | [ ] |
| 3316 | `ZSTDv05_getFrameParams` (src/legacy/zstd_v05.c:2754) | `if (srcSize < ZSTDv05_frameHeaderSize_min) return ZSTDv05_frameHeaderSize_max;` | `ERROR(prefix_unknown)` | [ ] |
| 3317 | `ZSTDv05_getFrameParams` (src/legacy/zstd_v05.c:2756) | `if (magicNumber != ZSTDv05_MAGICNUMBER) return ERROR(prefix_unknown);` | `ERROR(prefix_unknown)` | [ ] |
| 3318 | `ZSTDv05_getFrameParams` (src/legacy/zstd_v05.c:2759) | `if ((((const BYTE*)src)[4] >> 4) != 0) return ERROR(frameParameter_unsupported);   /* reserved bits */` | `ERROR(frameParameter_unsupported)` | [ ] |
| 3319 | `ZSTDv05_decodeFrameHeader_Part2` (src/legacy/zstd_v05.c:2770) | `if (srcSize != zc->headerSize)` | `ERROR(srcSize_wrong)` | [ ] |
| 3320 | `ZSTDv05_decodeFrameHeader_Part2` (src/legacy/zstd_v05.c:2771) | `return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 3321 | `ZSTDv05_decodeFrameHeader_Part2` (src/legacy/zstd_v05.c:2773) | `if ((MEM_32bits()) && (zc->params.windowLog > 25)) return ERROR(frameParameter_unsupported);` | `ERROR(frameParameter_unsupported)` | [ ] |
| 3322 | `ZSTDv05_getcBlockSize` (src/legacy/zstd_v05.c:2784) | `if (srcSize < 3)` | `ERROR(srcSize_wrong)` | [ ] |
| 3323 | `ZSTDv05_getcBlockSize` (src/legacy/zstd_v05.c:2785) | `return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 3324 | `ZSTDv05_getcBlockSize` (src/legacy/zstd_v05.c:2793) | `if (bpPtr->blockType == bt_end) return 0;` | branch-specific rejection/error | [ ] |
| 3325 | `ZSTDv05_getcBlockSize` (src/legacy/zstd_v05.c:2794) | `if (bpPtr->blockType == bt_rle) return 1;` | branch-specific rejection/error | [ ] |
| 3326 | `ZSTDv05_copyRawBlock` (src/legacy/zstd_v05.c:2801) | `if (dst==NULL) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [ ] |
| 3327 | `ZSTDv05_copyRawBlock` (src/legacy/zstd_v05.c:2802) | `if (srcSize > maxDstSize) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [ ] |
| 3328 | `ZSTDv05_decodeLiteralsBlock` (src/legacy/zstd_v05.c:2816) | `if (srcSize < MIN_CBLOCK_SIZE) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3329 | `ZSTDv05_decodeLiteralsBlock` (src/legacy/zstd_v05.c:2824) | `if (srcSize < 5) return ERROR(corruption_detected);   /* srcSize >= MIN_CBLOCK_SIZE == 3; here we need up to 5 for case 3 */` | `ERROR(corruption_detected)` | [ ] |
| 3330 | `ZSTDv05_decodeLiteralsBlock` (src/legacy/zstd_v05.c:2847) | `if (litSize > BLOCKSIZE) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3331 | `ZSTDv05_decodeLiteralsBlock` (src/legacy/zstd_v05.c:2848) | `if (litCSize + lhSize > srcSize) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3332 | `ZSTDv05_decodeLiteralsBlock` (src/legacy/zstd_v05.c:2853) | `return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3333 | `ZSTDv05_decodeLiteralsBlock` (src/legacy/zstd_v05.c:2866) | `return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3334 | `ZSTDv05_decodeLiteralsBlock` (src/legacy/zstd_v05.c:2867) | `if (!dctx->flagStaticTables)` | `ERROR(dictionary_corrupted)` | [ ] |
| 3335 | `ZSTDv05_decodeLiteralsBlock` (src/legacy/zstd_v05.c:2868) | `return ERROR(dictionary_corrupted);` | `ERROR(dictionary_corrupted)` | [ ] |
| 3336 | `ZSTDv05_decodeLiteralsBlock` (src/legacy/zstd_v05.c:2874) | `if (litCSize + lhSize > srcSize) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3337 | `ZSTDv05_decodeLiteralsBlock` (src/legacy/zstd_v05.c:2877) | `if (HUFv05_isError(errorCode)) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3338 | `ZSTDv05_decodeLiteralsBlock` (src/legacy/zstd_v05.c:2902) | `if (lhSize+litSize+WILDCOPY_OVERLENGTH > srcSize) {  /* risk reading beyond src buffer with wildcopy */` | `ERROR(corruption_detected)` | [ ] |
| 3339 | `ZSTDv05_decodeLiteralsBlock` (src/legacy/zstd_v05.c:2903) | `if (litSize+lhSize > srcSize) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3340 | `ZSTDv05_decodeLiteralsBlock` (src/legacy/zstd_v05.c:2930) | `if (srcSize<4) return ERROR(corruption_detected);   /* srcSize >= MIN_CBLOCK_SIZE == 3; here we need lhSize+1 = 4 */` | `ERROR(corruption_detected)` | [ ] |
| 3341 | `ZSTDv05_decodeLiteralsBlock` (src/legacy/zstd_v05.c:2933) | `if (litSize > BLOCKSIZE) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3342 | `ZSTDv05_decodeLiteralsBlock` (src/legacy/zstd_v05.c:2940) | `return ERROR(corruption_detected);   /* impossible */` | `ERROR(corruption_detected)` | [ ] |
| 3343 | `ZSTDv05_decodeSeqHeaders` (src/legacy/zstd_v05.c:2957) | `if (srcSize < MIN_SEQUENCES_SIZE)` | `ERROR(srcSize_wrong)` | [ ] |
| 3344 | `ZSTDv05_decodeSeqHeaders` (src/legacy/zstd_v05.c:2958) | `return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 3345 | `ZSTDv05_decodeSeqHeaders` (src/legacy/zstd_v05.c:2962) | `if (*nbSeq==0) return 1;` | `ERROR(srcSize_wrong)` | [ ] |
| 3346 | `ZSTDv05_decodeSeqHeaders` (src/legacy/zstd_v05.c:2963) | `if (*nbSeq >= 128) {` | `ERROR(srcSize_wrong)` | [ ] |
| 3347 | `ZSTDv05_decodeSeqHeaders` (src/legacy/zstd_v05.c:2964) | `if (ip >= iend) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 3348 | `ZSTDv05_decodeSeqHeaders` (src/legacy/zstd_v05.c:2968) | `if (ip >= iend) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 3349 | `ZSTDv05_decodeSeqHeaders` (src/legacy/zstd_v05.c:2973) | `if (ip+3 > iend) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 3350 | `ZSTDv05_decodeSeqHeaders` (src/legacy/zstd_v05.c:2978) | `if (ip+2 > iend) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 3351 | `ZSTDv05_decodeSeqHeaders` (src/legacy/zstd_v05.c:2988) | `if (ip > iend-3) return ERROR(srcSize_wrong); /* min : all 3 are "raw", hence no header, but at least xxLog bits per type */` | `ERROR(srcSize_wrong)` | [ ] |
| 3352 | `ZSTDv05_decodeSeqHeaders` (src/legacy/zstd_v05.c:3007) | `if (!flagStaticTable) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3353 | `ZSTDv05_decodeSeqHeaders` (src/legacy/zstd_v05.c:3013) | `if (FSEv05_isError(headerSize)) return ERROR(GENERIC);` | `ERROR(GENERIC)` | [ ] |
| 3354 | `ZSTDv05_decodeSeqHeaders` (src/legacy/zstd_v05.c:3014) | `if (LLlog > LLFSEv05Log) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3355 | `ZSTDv05_decodeSeqHeaders` (src/legacy/zstd_v05.c:3023) | `if (ip > iend-2) return ERROR(srcSize_wrong);   /* min : "raw", hence no header, but at least xxLog bits */` | `ERROR(srcSize_wrong)` | [ ] |
| 3356 | `ZSTDv05_decodeSeqHeaders` (src/legacy/zstd_v05.c:3031) | `if (!flagStaticTable) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3357 | `ZSTDv05_decodeSeqHeaders` (src/legacy/zstd_v05.c:3037) | `if (FSEv05_isError(headerSize)) return ERROR(GENERIC);` | `ERROR(GENERIC)` | [ ] |
| 3358 | `ZSTDv05_decodeSeqHeaders` (src/legacy/zstd_v05.c:3038) | `if (Offlog > OffFSEv05Log) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3359 | `ZSTDv05_decodeSeqHeaders` (src/legacy/zstd_v05.c:3047) | `if (ip > iend-2) return ERROR(srcSize_wrong); /* min : "raw", hence no header, but at least xxLog bits */` | `ERROR(srcSize_wrong)` | [ ] |
| 3360 | `ZSTDv05_decodeSeqHeaders` (src/legacy/zstd_v05.c:3055) | `if (!flagStaticTable) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3361 | `ZSTDv05_decodeSeqHeaders` (src/legacy/zstd_v05.c:3061) | `if (FSEv05_isError(headerSize)) return ERROR(GENERIC);` | `ERROR(GENERIC)` | [ ] |
| 3362 | `ZSTDv05_decodeSeqHeaders` (src/legacy/zstd_v05.c:3062) | `if (MLlog > MLFSEv05Log) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3363 | `ZSTDv05_decodeSequence` (src/legacy/zstd_v05.c:3103) | `if (add < 255) litLength += add;` | branch-specific rejection/error | [ ] |
| 3364 | `ZSTDv05_decodeSequence` (src/legacy/zstd_v05.c:3104) | `else if (dumps + 2 <= de) {` | branch-specific rejection/error | [ ] |
| 3365 | `ZSTDv05_decodeSequence` (src/legacy/zstd_v05.c:3107) | `if ((litLength & 1) && dumps < de) {` | branch-specific rejection/error | [ ] |
| 3366 | `ZSTDv05_decodeSequence` (src/legacy/zstd_v05.c:3113) | `if (dumps >= de) { dumps = de-1; }  /* late correction, to avoid read overflow (data is now corrupted anyway) */` | branch-specific rejection/error | [ ] |
| 3367 | `ZSTDv05_decodeSequence` (src/legacy/zstd_v05.c:3124) | `if (offsetCode==0) nbBits = 0;   /* cmove */` | branch-specific rejection/error | [ ] |
| 3368 | `ZSTDv05_decodeSequence` (src/legacy/zstd_v05.c:3126) | `if (MEM_32bits()) BITv05_reloadDStream(&(seqState->DStream));` | branch-specific rejection/error | [ ] |
| 3369 | `ZSTDv05_decodeSequence` (src/legacy/zstd_v05.c:3127) | `if (offsetCode==0) offset = prevOffset;   /* repcode, cmove */` | branch-specific rejection/error | [ ] |
| 3370 | `ZSTDv05_decodeSequence` (src/legacy/zstd_v05.c:3128) | `if (offsetCode \| !litLength) seqState->prevOffset = seq->offset;   /* cmove */` | branch-specific rejection/error | [ ] |
| 3371 | `ZSTDv05_decodeSequence` (src/legacy/zstd_v05.c:3134) | `if (MEM_32bits()) BITv05_reloadDStream(&(seqState->DStream));` | branch-specific rejection/error | [ ] |
| 3372 | `ZSTDv05_decodeSequence` (src/legacy/zstd_v05.c:3140) | `if (add < 255) matchLength += add;` | branch-specific rejection/error | [ ] |
| 3373 | `ZSTDv05_decodeSequence` (src/legacy/zstd_v05.c:3141) | `else if (dumps + 2 <= de) {` | branch-specific rejection/error | [ ] |
| 3374 | `ZSTDv05_decodeSequence` (src/legacy/zstd_v05.c:3144) | `if ((matchLength & 1) && dumps < de) {` | branch-specific rejection/error | [ ] |
| 3375 | `ZSTDv05_decodeSequence` (src/legacy/zstd_v05.c:3150) | `if (dumps >= de) { dumps = de-1; }  /* late correction, to avoid read overflow (data is now corrupted anyway) */` | branch-specific rejection/error | [ ] |
| 3376 | `ZSTDv05_execSequence` (src/legacy/zstd_v05.c:3188) | `if (seqLength > (size_t)(oend - op)) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [ ] |
| 3377 | `ZSTDv05_execSequence` (src/legacy/zstd_v05.c:3189) | `if (sequence.litLength > (size_t)(litLimit - *litPtr)) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3378 | `ZSTDv05_execSequence` (src/legacy/zstd_v05.c:3191) | `if (oLitEnd > oend_8) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [ ] |
| 3379 | `ZSTDv05_execSequence` (src/legacy/zstd_v05.c:3193) | `if (oMatchEnd > oend) return ERROR(dstSize_tooSmall);   /* overwrite beyond dst buffer */` | `ERROR(dstSize_tooSmall)` | [ ] |
| 3380 | `ZSTDv05_execSequence` (src/legacy/zstd_v05.c:3194) | `if (litEnd > litLimit) return ERROR(corruption_detected);   /* overRead beyond lit buffer */` | `ERROR(corruption_detected)` | [ ] |
| 3381 | `ZSTDv05_execSequence` (src/legacy/zstd_v05.c:3202) | `if (sequence.offset > (size_t)(oLitEnd - base)) {` | `ERROR(corruption_detected)` | [ ] |
| 3382 | `ZSTDv05_execSequence` (src/legacy/zstd_v05.c:3204) | `if (sequence.offset > (size_t)(oLitEnd - vBase))` | `ERROR(corruption_detected)` | [ ] |
| 3383 | `ZSTDv05_execSequence` (src/legacy/zstd_v05.c:3205) | `return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3384 | `ZSTDv05_execSequence` (src/legacy/zstd_v05.c:3207) | `if (match + sequence.matchLength <= dictEnd) {` | branch-specific rejection/error | [ ] |
| 3385 | `ZSTDv05_execSequence` (src/legacy/zstd_v05.c:3218) | `if (op > oend_8 \|\| sequence.matchLength < MINMATCH) {` | branch-specific rejection/error | [ ] |
| 3386 | `ZSTDv05_execSequence` (src/legacy/zstd_v05.c:3226) | `if (sequence.offset < 8) {` | branch-specific rejection/error | [ ] |
| 3387 | `ZSTDv05_execSequence` (src/legacy/zstd_v05.c:3241) | `if (oMatchEnd > oend-(16-MINMATCH)) {` | branch-specific rejection/error | [ ] |
| 3388 | `ZSTDv05_execSequence` (src/legacy/zstd_v05.c:3242) | `if (op < oend_8) {` | branch-specific rejection/error | [ ] |
| 3389 | `ZSTDv05_decompressSequences` (src/legacy/zstd_v05.c:3296) | `if (ERR_isError(errorCode)) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3390 | `ZSTDv05_decompressSequences` (src/legacy/zstd_v05.c:3311) | `if (nbSeq) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3391 | `ZSTDv05_decompressSequences` (src/legacy/zstd_v05.c:3317) | `if (litPtr > litEnd) return ERROR(corruption_detected);   /* too many literals already used */` | `ERROR(corruption_detected)` | [ ] |
| 3392 | `ZSTDv05_decompressSequences` (src/legacy/zstd_v05.c:3318) | `if (op+lastLLSize > oend) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [ ] |
| 3393 | `ZSTDv05_decompressSequences` (src/legacy/zstd_v05.c:3319) | `if (lastLLSize > 0) {` | branch-specific rejection/error | [ ] |
| 3394 | `ZSTDv05_checkContinuity` (src/legacy/zstd_v05.c:3331) | `if (dst != dctx->previousDstEnd) {   /* not contiguous */` | branch-specific rejection/error | [ ] |
| 3395 | `ZSTDv05_decompressBlock_internal` (src/legacy/zstd_v05.c:3347) | `if (srcSize >= BLOCKSIZE) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 3396 | `ZSTDv05_decompress_continueDCtx` (src/legacy/zstd_v05.c:3385) | `if (srcSize < ZSTDv05_frameHeaderSize_min+ZSTDv05_blockHeaderSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 3397 | `ZSTDv05_decompress_continueDCtx` (src/legacy/zstd_v05.c:3388) | `if (srcSize < frameHeaderSize+ZSTDv05_blockHeaderSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 3398 | `ZSTDv05_decompress_continueDCtx` (src/legacy/zstd_v05.c:3403) | `if (cBlockSize > remainingSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 3399 | `ZSTDv05_decompress_continueDCtx` (src/legacy/zstd_v05.c:3414) | `return ERROR(GENERIC);   /* not yet supported */` | `ERROR(GENERIC)` | [ ] |
| 3400 | `ZSTDv05_decompress_continueDCtx` (src/legacy/zstd_v05.c:3418) | `if (remainingSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 3401 | `ZSTDv05_decompress_continueDCtx` (src/legacy/zstd_v05.c:3421) | `return ERROR(GENERIC);   /* impossible */` | `ERROR(GENERIC)` | [ ] |
| 3402 | `ZSTDv05_decompress_continueDCtx` (src/legacy/zstd_v05.c:3423) | `if (cBlockSize == 0) break;   /* bt_end */` | branch-specific rejection/error | [ ] |
| 3403 | `ZSTDv05_decompress` (src/legacy/zstd_v05.c:3466) | `if (dctx==NULL) return ERROR(memory_allocation);` | `ERROR(memory_allocation)` | [ ] |
| 3404 | `ZSTDv05_findFrameSizeInfoLegacy` (src/legacy/zstd_v05.c:3492) | `if (srcSize < ZSTDv05_frameHeaderSize_min) {` | `ERROR(srcSize_wrong)` | [ ] |
| 3405 | `ZSTDv05_findFrameSizeInfoLegacy` (src/legacy/zstd_v05.c:3493) | `ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, ERROR(srcSize_wrong));` | `ERROR(srcSize_wrong)` | [ ] |
| 3406 | `ZSTDv05_findFrameSizeInfoLegacy` (src/legacy/zstd_v05.c:3497) | `ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, ERROR(prefix_unknown));` | `ERROR(prefix_unknown)` | [ ] |
| 3407 | `ZSTDv05_findFrameSizeInfoLegacy` (src/legacy/zstd_v05.c:3513) | `if (cBlockSize > remainingSize) {` | `ERROR(srcSize_wrong)` | [ ] |
| 3408 | `ZSTDv05_findFrameSizeInfoLegacy` (src/legacy/zstd_v05.c:3514) | `ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, ERROR(srcSize_wrong));` | `ERROR(srcSize_wrong)` | [ ] |
| 3409 | `ZSTDv05_findFrameSizeInfoLegacy` (src/legacy/zstd_v05.c:3518) | `if (cBlockSize == 0) break;   /* bt_end */` | branch-specific rejection/error | [ ] |
| 3410 | `ZSTDv05_decompressContinue` (src/legacy/zstd_v05.c:3540) | `if (srcSize != dctx->expected) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 3411 | `ZSTDv05_decompressContinue` (src/legacy/zstd_v05.c:3548) | `if (srcSize != ZSTDv05_frameHeaderSize_min) return ERROR(srcSize_wrong);   /* impossible */` | `ERROR(srcSize_wrong)` | [ ] |
| 3412 | `ZSTDv05_decompressContinue` (src/legacy/zstd_v05.c:3550) | `if (ZSTDv05_isError(dctx->headerSize)) return dctx->headerSize;` | `ERROR(GENERIC)` | [ ] |
| 3413 | `ZSTDv05_decompressContinue` (src/legacy/zstd_v05.c:3552) | `if (dctx->headerSize > ZSTDv05_frameHeaderSize_min) return ERROR(GENERIC); /* should never happen */` | `ERROR(GENERIC)` | [ ] |
| 3414 | `ZSTDv05_decompressContinue` (src/legacy/zstd_v05.c:3593) | `return ERROR(GENERIC);   /* not yet handled */` | `ERROR(GENERIC)` | [ ] |
| 3415 | `ZSTDv05_decompressContinue` (src/legacy/zstd_v05.c:3599) | `return ERROR(GENERIC);   /* impossible */` | `ERROR(GENERIC)` | [ ] |
| 3416 | `ZSTDv05_decompressContinue` (src/legacy/zstd_v05.c:3608) | `return ERROR(GENERIC);   /* impossible */` | `ERROR(GENERIC)` | [ ] |
| 3417 | `ZSTDv05_loadEntropy` (src/legacy/zstd_v05.c:3632) | `if (HUFv05_isError(hSize)) return ERROR(dictionary_corrupted);` | `ERROR(dictionary_corrupted)` | [ ] |
| 3418 | `ZSTDv05_loadEntropy` (src/legacy/zstd_v05.c:3637) | `if (FSEv05_isError(offcodeHeaderSize)) return ERROR(dictionary_corrupted);` | `ERROR(dictionary_corrupted)` | [ ] |
| 3419 | `ZSTDv05_loadEntropy` (src/legacy/zstd_v05.c:3638) | `if (offcodeLog > OffFSEv05Log) return ERROR(dictionary_corrupted);` | `ERROR(dictionary_corrupted)` | [ ] |
| 3420 | `ZSTDv05_loadEntropy` (src/legacy/zstd_v05.c:3640) | `if (FSEv05_isError(errorCode)) return ERROR(dictionary_corrupted);` | `ERROR(dictionary_corrupted)` | [ ] |
| 3421 | `ZSTDv05_loadEntropy` (src/legacy/zstd_v05.c:3645) | `if (FSEv05_isError(matchlengthHeaderSize)) return ERROR(dictionary_corrupted);` | `ERROR(dictionary_corrupted)` | [ ] |
| 3422 | `ZSTDv05_loadEntropy` (src/legacy/zstd_v05.c:3646) | `if (matchlengthLog > MLFSEv05Log) return ERROR(dictionary_corrupted);` | `ERROR(dictionary_corrupted)` | [ ] |
| 3423 | `ZSTDv05_loadEntropy` (src/legacy/zstd_v05.c:3648) | `if (FSEv05_isError(errorCode)) return ERROR(dictionary_corrupted);` | `ERROR(dictionary_corrupted)` | [ ] |
| 3424 | `ZSTDv05_loadEntropy` (src/legacy/zstd_v05.c:3653) | `if (litlengthLog > LLFSEv05Log) return ERROR(dictionary_corrupted);` | `ERROR(dictionary_corrupted)` | [ ] |
| 3425 | `ZSTDv05_loadEntropy` (src/legacy/zstd_v05.c:3654) | `if (FSEv05_isError(litlengthHeaderSize)) return ERROR(dictionary_corrupted);` | `ERROR(dictionary_corrupted)` | [ ] |
| 3426 | `ZSTDv05_loadEntropy` (src/legacy/zstd_v05.c:3656) | `if (FSEv05_isError(errorCode)) return ERROR(dictionary_corrupted);` | `ERROR(dictionary_corrupted)` | [ ] |
| 3427 | `ZSTDv05_decompress_insertDictionary` (src/legacy/zstd_v05.c:3675) | `if (ZSTDv05_isError(eSize)) return ERROR(dictionary_corrupted);` | `ERROR(dictionary_corrupted)` | [ ] |
| 3428 | `ZSTDv05_decompressBegin_usingDict` (src/legacy/zstd_v05.c:3694) | `if (ZSTDv05_isError(errorCode)) return ERROR(dictionary_corrupted);` | `ERROR(dictionary_corrupted)` | [ ] |
| 3429 | `ZBUFFv05_limitCopy` (src/legacy/zstd_v05.c:3751) | `if (length > 0) {` | branch-specific rejection/error | [ ] |
| 3430 | `ZBUFFv05_createDCtx` (src/legacy/zstd_v05.c:3807) | `if (zbc==NULL) return NULL;` | `NULL` | [ ] |
| 3431 | `ZBUFFv05_freeDCtx` (src/legacy/zstd_v05.c:3816) | `if (zbc==NULL) return 0;   /* support free on null */` | branch-specific rejection/error | [ ] |
| 3432 | `ZBUFFv05_decompressContinue` (src/legacy/zstd_v05.c:3856) | `return ERROR(init_missing);` | `ERROR(init_missing)` | [ ] |
| 3433 | `ZBUFFv05_decompressContinue` (src/legacy/zstd_v05.c:3898) | `if (zbc->inBuffSize < neededInSize) {` | `ERROR(memory_allocation)` | [ ] |
| 3434 | `ZBUFFv05_decompressContinue` (src/legacy/zstd_v05.c:3902) | `if (zbc->inBuff == NULL) return ERROR(memory_allocation);` | `ERROR(memory_allocation)` | [ ] |
| 3435 | `ZBUFFv05_decompressContinue` (src/legacy/zstd_v05.c:3904) | `if (zbc->outBuffSize < neededOutSize) {` | `ERROR(memory_allocation)` | [ ] |
| 3436 | `ZBUFFv05_decompressContinue` (src/legacy/zstd_v05.c:3908) | `if (zbc->outBuff == NULL) return ERROR(memory_allocation);` | `ERROR(memory_allocation)` | [ ] |
| 3437 | `ZBUFFv05_decompressContinue` (src/legacy/zstd_v05.c:3910) | `if (zbc->hPos) {` | branch-specific rejection/error | [ ] |
| 3438 | `ZBUFFv05_decompressContinue` (src/legacy/zstd_v05.c:3923) | `if (neededInSize==0) {  /* end of frame */` | branch-specific rejection/error | [ ] |
| 3439 | `ZBUFFv05_decompressContinue` (src/legacy/zstd_v05.c:3928) | `if ((size_t)(iend-ip) >= neededInSize) {` | branch-specific rejection/error | [ ] |
| 3440 | `ZBUFFv05_decompressContinue` (src/legacy/zstd_v05.c:3949) | `if (toLoad > zbc->inBuffSize - zbc->inPos) return ERROR(corruption_detected);   /* should never happen */` | `ERROR(corruption_detected)` | [ ] |
| 3441 | `ZBUFFv05_decompressContinue` (src/legacy/zstd_v05.c:3953) | `if (loadedSize < toLoad) { notDone = 0; break; }   /* not enough input, wait for more */` | branch-specific rejection/error | [ ] |
| 3442 | `ZBUFFv05_decompressContinue` (src/legacy/zstd_v05.c:3960) | `if (!decodedSize) { zbc->stage = ZBUFFv05ds_read; break; }   /* this was just a header */` | branch-specific rejection/error | [ ] |
| 3443 | `ZBUFFv05_decompressContinue` (src/legacy/zstd_v05.c:3975) | `if (zbc->outStart + BLOCKSIZE > zbc->outBuffSize)` | branch-specific rejection/error | [ ] |
| 3444 | `ZBUFFv05_decompressContinue` (src/legacy/zstd_v05.c:3983) | `default: return ERROR(GENERIC);   /* impossible */` | `ERROR(GENERIC)` | [ ] |
| 3445 | `ZBUFFv05_decompressContinue` (src/legacy/zstd_v05.c:3990) | `if (nextSrcSizeHint > ZBUFFv05_blockHeaderSize) nextSrcSizeHint+= ZBUFFv05_blockHeaderSize;   /* get next block header too */` | branch-specific rejection/error | [ ] |
| 3446 | `BITv06_initDStream` (src/legacy/zstd_v06.c:835) | `if (srcSize < 1) { memset(bitD, 0, sizeof(*bitD)); return ERROR(srcSize_wrong); }` | `ERROR(srcSize_wrong)` | [ ] |
| 3447 | `BITv06_initDStream` (src/legacy/zstd_v06.c:837) | `if (srcSize >=  sizeof(bitD->bitContainer)) {  /* normal case */` | branch-specific rejection/error | [ ] |
| 3448 | `BITv06_initDStream` (src/legacy/zstd_v06.c:842) | `if (lastByte == 0) return ERROR(GENERIC);   /* endMark not present */` | `ERROR(GENERIC)` | [ ] |
| 3449 | `BITv06_initDStream` (src/legacy/zstd_v06.c:859) | `if (lastByte == 0) return ERROR(GENERIC);   /* endMark not present */` | `ERROR(GENERIC)` | [ ] |
| 3450 | `BITv06_reloadDStream` (src/legacy/zstd_v06.c:905) | `if (bitD->bitsConsumed > (sizeof(bitD->bitContainer)*8))  /* should never happen */` | branch-specific rejection/error | [ ] |
| 3451 | `BITv06_reloadDStream` (src/legacy/zstd_v06.c:908) | `if (bitD->ptr >= bitD->start + sizeof(bitD->bitContainer)) {` | branch-specific rejection/error | [ ] |
| 3452 | `BITv06_reloadDStream` (src/legacy/zstd_v06.c:914) | `if (bitD->ptr == bitD->start) {` | branch-specific rejection/error | [ ] |
| 3453 | `BITv06_reloadDStream` (src/legacy/zstd_v06.c:915) | `if (bitD->bitsConsumed < sizeof(bitD->bitContainer)*8) return BITv06_DStream_endOfBuffer;` | branch-specific rejection/error | [ ] |
| 3454 | `BITv06_reloadDStream` (src/legacy/zstd_v06.c:920) | `if (bitD->ptr - nbBytes < bitD->start) {` | branch-specific rejection/error | [ ] |
| 3455 | `FSEv06_readNCount` (src/legacy/zstd_v06.c:1221) | `if (hbSize < 4) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 3456 | `FSEv06_readNCount` (src/legacy/zstd_v06.c:1224) | `if (nbBits > FSEv06_TABLELOG_ABSOLUTE_MAX) return ERROR(tableLog_tooLarge);` | `ERROR(tableLog_tooLarge)` | [ ] |
| 3457 | `FSEv06_readNCount` (src/legacy/zstd_v06.c:1237) | `if (ip < iend-5) {` | branch-specific rejection/error | [ ] |
| 3458 | `FSEv06_readNCount` (src/legacy/zstd_v06.c:1251) | `if (n0 > *maxSVPtr) return ERROR(maxSymbolValue_tooSmall);` | `ERROR(maxSymbolValue_tooSmall)` | [ ] |
| 3459 | `FSEv06_readNCount` (src/legacy/zstd_v06.c:1253) | `if ((ip <= iend-7) \|\| (ip + (bitCount>>3) <= iend-4)) {` | branch-specific rejection/error | [ ] |
| 3460 | `FSEv06_readNCount` (src/legacy/zstd_v06.c:1264) | `if ((bitStream & (threshold-1)) < (U32)max) {` | branch-specific rejection/error | [ ] |
| 3461 | `FSEv06_readNCount` (src/legacy/zstd_v06.c:1269) | `if (count >= threshold) count -= max;` | branch-specific rejection/error | [ ] |
| 3462 | `FSEv06_readNCount` (src/legacy/zstd_v06.c:1282) | `if ((ip <= iend-7) \|\| (ip + (bitCount>>3) <= iend-4)) {` | branch-specific rejection/error | [ ] |
| 3463 | `FSEv06_readNCount` (src/legacy/zstd_v06.c:1291) | `if (remaining != 1) return ERROR(GENERIC);` | `ERROR(GENERIC)` | [ ] |
| 3464 | `FSEv06_readNCount` (src/legacy/zstd_v06.c:1295) | `if ((size_t)(ip-istart) > hbSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 3465 | `FSEv06_createDTable` (src/legacy/zstd_v06.c:1393) | `if (tableLog > FSEv06_TABLELOG_ABSOLUTE_MAX) tableLog = FSEv06_TABLELOG_ABSOLUTE_MAX;` | branch-specific rejection/error | [ ] |
| 3466 | `FSEv06_buildDTable` (src/legacy/zstd_v06.c:1413) | `if (maxSymbolValue > FSEv06_MAX_SYMBOL_VALUE) return ERROR(maxSymbolValue_tooLarge);` | `ERROR(maxSymbolValue_tooLarge)` | [ ] |
| 3467 | `FSEv06_buildDTable` (src/legacy/zstd_v06.c:1414) | `if (tableLog > FSEv06_MAX_TABLELOG) return ERROR(tableLog_tooLarge);` | `ERROR(tableLog_tooLarge)` | [ ] |
| 3468 | `FSEv06_buildDTable` (src/legacy/zstd_v06.c:1427) | `if (normalizedCounter[s] >= largeLimit) DTableH.fastMode=0;` | branch-specific rejection/error | [ ] |
| 3469 | `FSEv06_buildDTable` (src/legacy/zstd_v06.c:1445) | `if (position!=0) return ERROR(GENERIC);   /* position must reach all cells once, otherwise normalizedCounter is incorrect */` | `ERROR(GENERIC)` | [ ] |
| 3470 | `FSEv06_buildDTable_raw` (src/legacy/zstd_v06.c:1497) | `if (nbBits < 1) return ERROR(GENERIC);         /* min size */` | `ERROR(GENERIC)` | [ ] |
| 3471 | `FSEv06_decompress_usingDTable_generic` (src/legacy/zstd_v06.c:1538) | `if (FSEv06_MAX_TABLELOG*2+7 > sizeof(bitD.bitContainer)*8)    /* This test must be static */` | branch-specific rejection/error | [ ] |
| 3472 | `FSEv06_decompress_usingDTable_generic` (src/legacy/zstd_v06.c:1543) | `if (FSEv06_MAX_TABLELOG*4+7 > sizeof(bitD.bitContainer)*8)    /* This test must be static */` | branch-specific rejection/error | [ ] |
| 3473 | `FSEv06_decompress_usingDTable_generic` (src/legacy/zstd_v06.c:1544) | `{ if (BITv06_reloadDStream(&bitD) > BITv06_DStream_unfinished) { op+=2; break; } }` | branch-specific rejection/error | [ ] |
| 3474 | `FSEv06_decompress_usingDTable_generic` (src/legacy/zstd_v06.c:1548) | `if (FSEv06_MAX_TABLELOG*2+7 > sizeof(bitD.bitContainer)*8)    /* This test must be static */` | branch-specific rejection/error | [ ] |
| 3475 | `FSEv06_decompress_usingDTable_generic` (src/legacy/zstd_v06.c:1557) | `if (op>(omax-2)) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [ ] |
| 3476 | `FSEv06_decompress_usingDTable_generic` (src/legacy/zstd_v06.c:1566) | `if (op>(omax-2)) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [ ] |
| 3477 | `FSEv06_decompress` (src/legacy/zstd_v06.c:1602) | `if (cSrcSize<2) return ERROR(srcSize_wrong);   /* too small input size */` | `ERROR(srcSize_wrong)` | [ ] |
| 3478 | `FSEv06_decompress` (src/legacy/zstd_v06.c:1607) | `if (NCountLength >= cSrcSize) return ERROR(srcSize_wrong);   /* too small input size */` | `ERROR(srcSize_wrong)` | [ ] |
| 3479 | `HUFv06_readStats` (src/legacy/zstd_v06.c:1807) | `if (!srcSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 3480 | `HUFv06_readStats` (src/legacy/zstd_v06.c:1811) | `if (iSize >= 128)  { /* special header */` | branch-specific rejection/error | [ ] |
| 3481 | `HUFv06_readStats` (src/legacy/zstd_v06.c:1812) | `if (iSize >= (242)) {  /* RLE */` | branch-specific rejection/error | [ ] |
| 3482 | `HUFv06_readStats` (src/legacy/zstd_v06.c:1821) | `if (iSize+1 > srcSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 3483 | `HUFv06_readStats` (src/legacy/zstd_v06.c:1822) | `if (oSize >= hwSize) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3484 | `HUFv06_readStats` (src/legacy/zstd_v06.c:1830) | `if (iSize+1 > srcSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 3485 | `HUFv06_readStats` (src/legacy/zstd_v06.c:1839) | `if (huffWeight[n] >= HUFv06_ABSOLUTEMAX_TABLELOG) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3486 | `HUFv06_readStats` (src/legacy/zstd_v06.c:1843) | `if (weightTotal == 0) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3487 | `HUFv06_readStats` (src/legacy/zstd_v06.c:1847) | `if (tableLog > HUFv06_ABSOLUTEMAX_TABLELOG) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3488 | `HUFv06_readStats` (src/legacy/zstd_v06.c:1854) | `if (verif != rest) return ERROR(corruption_detected);    /* last value must be a clean power of 2 */` | `ERROR(corruption_detected)` | [ ] |
| 3489 | `HUFv06_readStats` (src/legacy/zstd_v06.c:1860) | `if ((rankStats[1] < 2) \|\| (rankStats[1] & 1)) return ERROR(corruption_detected);   /* by construction : at least 2 elts of rank 1, must be even */` | `ERROR(corruption_detected)` | [ ] |
| 3490 | `HUFv06_readDTableX2` (src/legacy/zstd_v06.c:1967) | `if (tableLog > DTable[0]) return ERROR(tableLog_tooLarge);   /* DTable is too small */` | `ERROR(tableLog_tooLarge)` | [ ] |
| 3491 | `HUFv06_decodeSymbolX2` (src/legacy/zstd_v06.c:2006) | `if (MEM_64bits() \|\| (HUFv06_MAX_TABLELOG<=12)) \` | branch-specific rejection/error | [ ] |
| 3492 | `HUFv06_decompress1X2_usingDTable` (src/legacy/zstd_v06.c:2054) | `if (!BITv06_endOfDStream(&bitD)) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3493 | `HUFv06_decompress1X2` (src/legacy/zstd_v06.c:2066) | `if (errorCode >= cSrcSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 3494 | `HUFv06_decompress4X2_usingDTable` (src/legacy/zstd_v06.c:2080) | `if (cSrcSize < 10) return ERROR(corruption_detected);  /* strict minimum : jump table + 1 byte per stream */` | `ERROR(corruption_detected)` | [ ] |
| 3495 | `HUFv06_decompress4X2_usingDTable` (src/legacy/zstd_v06.c:2114) | `if (length4 > cSrcSize) return ERROR(corruption_detected);   /* overflow */` | `ERROR(corruption_detected)` | [ ] |
| 3496 | `HUFv06_decompress4X2_usingDTable` (src/legacy/zstd_v06.c:2147) | `if (op1 > opStart2) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3497 | `HUFv06_decompress4X2_usingDTable` (src/legacy/zstd_v06.c:2148) | `if (op2 > opStart3) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3498 | `HUFv06_decompress4X2_usingDTable` (src/legacy/zstd_v06.c:2149) | `if (op3 > opStart4) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3499 | `HUFv06_decompress4X2_usingDTable` (src/legacy/zstd_v06.c:2160) | `if (!endSignal) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3500 | `HUFv06_decompress4X2` (src/legacy/zstd_v06.c:2175) | `if (errorCode >= cSrcSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 3501 | `HUFv06_fillDTableX4Level2` (src/legacy/zstd_v06.c:2199) | `if (minWeight>1) {` | branch-specific rejection/error | [ ] |
| 3502 | `HUFv06_fillDTableX4` (src/legacy/zstd_v06.c:2249) | `if (targetLog-nbBits >= minBits) {   /* enough room for a second symbol */` | branch-specific rejection/error | [ ] |
| 3503 | `HUFv06_fillDTableX4` (src/legacy/zstd_v06.c:2252) | `if (minWeight < 1) minWeight = 1;` | branch-specific rejection/error | [ ] |
| 3504 | `HUFv06_readDTableX4` (src/legacy/zstd_v06.c:2286) | `if (memLog > HUFv06_ABSOLUTEMAX_TABLELOG) return ERROR(tableLog_tooLarge);` | `ERROR(tableLog_tooLarge)` | [ ] |
| 3505 | `HUFv06_readDTableX4` (src/legacy/zstd_v06.c:2293) | `if (tableLog > memLog) return ERROR(tableLog_tooLarge);   /* DTable can't fit code depth */` | `ERROR(tableLog_tooLarge)` | [ ] |
| 3506 | `HUFv06_decodeLastSymbolX4` (src/legacy/zstd_v06.c:2362) | `if (DStream->bitsConsumed < (sizeof(DStream->bitContainer)*8)) {` | branch-specific rejection/error | [ ] |
| 3507 | `HUFv06_decodeLastSymbolX4` (src/legacy/zstd_v06.c:2364) | `if (DStream->bitsConsumed > (sizeof(DStream->bitContainer)*8))` | branch-specific rejection/error | [ ] |
| 3508 | `HUFv06_decodeLastSymbolX4` (src/legacy/zstd_v06.c:2375) | `if (MEM_64bits() \|\| (HUFv06_MAX_TABLELOG<=12)) \` | branch-specific rejection/error | [ ] |
| 3509 | `HUFv06_decodeStreamX4` (src/legacy/zstd_v06.c:2401) | `if (p < pEnd)` | branch-specific rejection/error | [ ] |
| 3510 | `HUFv06_decompress1X4_usingDTable` (src/legacy/zstd_v06.c:2430) | `if (!BITv06_endOfDStream(&bitD)) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3511 | `HUFv06_decompress1X4` (src/legacy/zstd_v06.c:2443) | `if (hSize >= cSrcSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 3512 | `HUFv06_decompress4X4_usingDTable` (src/legacy/zstd_v06.c:2455) | `if (cSrcSize < 10) return ERROR(corruption_detected);   /* strict minimum : jump table + 1 byte per stream */` | `ERROR(corruption_detected)` | [ ] |
| 3513 | `HUFv06_decompress4X4_usingDTable` (src/legacy/zstd_v06.c:2489) | `if (length4 > cSrcSize) return ERROR(corruption_detected);   /* overflow */` | `ERROR(corruption_detected)` | [ ] |
| 3514 | `HUFv06_decompress4X4_usingDTable` (src/legacy/zstd_v06.c:2523) | `if (op1 > opStart2) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3515 | `HUFv06_decompress4X4_usingDTable` (src/legacy/zstd_v06.c:2524) | `if (op2 > opStart3) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3516 | `HUFv06_decompress4X4_usingDTable` (src/legacy/zstd_v06.c:2525) | `if (op3 > opStart4) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3517 | `HUFv06_decompress4X4_usingDTable` (src/legacy/zstd_v06.c:2536) | `if (!endSignal) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3518 | `HUFv06_decompress4X4` (src/legacy/zstd_v06.c:2551) | `if (hSize >= cSrcSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 3519 | `HUFv06_decompress` (src/legacy/zstd_v06.c:2595) | `if (dstSize == 0) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [ ] |
| 3520 | `HUFv06_decompress` (src/legacy/zstd_v06.c:2596) | `if (cSrcSize > dstSize) return ERROR(corruption_detected);   /* invalid */` | `ERROR(corruption_detected)` | [ ] |
| 3521 | `HUFv06_decompress` (src/legacy/zstd_v06.c:2610) | `if (Dtime[1] < Dtime[0]) algoNb = 1;` | branch-specific rejection/error | [ ] |
| 3522 | `ZSTDv06_createDCtx` (src/legacy/zstd_v06.c:2789) | `if (dctx==NULL) return NULL;` | `NULL` | [ ] |
| 3523 | `ZSTDv06_frameHeaderSize` (src/legacy/zstd_v06.c:2913) | `if (srcSize < ZSTDv06_frameHeaderSize_min) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 3524 | `ZSTDv06_getFrameParams` (src/legacy/zstd_v06.c:2928) | `if (srcSize < ZSTDv06_frameHeaderSize_min) return ZSTDv06_frameHeaderSize_min;` | `ERROR(prefix_unknown)` | [ ] |
| 3525 | `ZSTDv06_getFrameParams` (src/legacy/zstd_v06.c:2929) | `if (MEM_readLE32(src) != ZSTDv06_MAGICNUMBER) return ERROR(prefix_unknown);` | `ERROR(prefix_unknown)` | [ ] |
| 3526 | `ZSTDv06_getFrameParams` (src/legacy/zstd_v06.c:2933) | `if (srcSize < fhsize) return fhsize; }` | branch-specific rejection/error | [ ] |
| 3527 | `ZSTDv06_getFrameParams` (src/legacy/zstd_v06.c:2938) | `if ((frameDesc & 0x20) != 0) return ERROR(frameParameter_unsupported);   /* reserved 1 bit */` | `ERROR(frameParameter_unsupported)` | [ ] |
| 3528 | `ZSTDv06_decodeFrameHeader` (src/legacy/zstd_v06.c:2957) | `if ((MEM_32bits()) && (zc->fParams.windowLog > 25)) return ERROR(frameParameter_unsupported);` | `ERROR(frameParameter_unsupported)` | [ ] |
| 3529 | `ZSTDv06_getcBlockSize` (src/legacy/zstd_v06.c:2975) | `if (srcSize < ZSTDv06_blockHeaderSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 3530 | `ZSTDv06_getcBlockSize` (src/legacy/zstd_v06.c:2981) | `if (bpPtr->blockType == bt_end) return 0;` | branch-specific rejection/error | [ ] |
| 3531 | `ZSTDv06_getcBlockSize` (src/legacy/zstd_v06.c:2982) | `if (bpPtr->blockType == bt_rle) return 1;` | branch-specific rejection/error | [ ] |
| 3532 | `ZSTDv06_copyRawBlock` (src/legacy/zstd_v06.c:2989) | `if (dst==NULL) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [ ] |
| 3533 | `ZSTDv06_copyRawBlock` (src/legacy/zstd_v06.c:2990) | `if (srcSize > dstCapacity) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [ ] |
| 3534 | `ZSTDv06_decodeLiteralsBlock` (src/legacy/zstd_v06.c:3004) | `if (srcSize < MIN_CBLOCK_SIZE) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3535 | `ZSTDv06_decodeLiteralsBlock` (src/legacy/zstd_v06.c:3011) | `if (srcSize < 5) return ERROR(corruption_detected);   /* srcSize >= MIN_CBLOCK_SIZE == 3; here we need up to 5 for lhSize, + cSize (+nbSeq) */` | `ERROR(corruption_detected)` | [ ] |
| 3536 | `ZSTDv06_decodeLiteralsBlock` (src/legacy/zstd_v06.c:3034) | `if (litSize > ZSTDv06_BLOCKSIZE_MAX) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3537 | `ZSTDv06_decodeLiteralsBlock` (src/legacy/zstd_v06.c:3035) | `if (litCSize + lhSize > srcSize) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3538 | `ZSTDv06_decodeLiteralsBlock` (src/legacy/zstd_v06.c:3040) | `return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3539 | `ZSTDv06_decodeLiteralsBlock` (src/legacy/zstd_v06.c:3051) | `return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3540 | `ZSTDv06_decodeLiteralsBlock` (src/legacy/zstd_v06.c:3052) | `if (!dctx->flagRepeatTable)` | `ERROR(dictionary_corrupted)` | [ ] |
| 3541 | `ZSTDv06_decodeLiteralsBlock` (src/legacy/zstd_v06.c:3053) | `return ERROR(dictionary_corrupted);` | `ERROR(dictionary_corrupted)` | [ ] |
| 3542 | `ZSTDv06_decodeLiteralsBlock` (src/legacy/zstd_v06.c:3059) | `if (litCSize + lhSize > srcSize) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3543 | `ZSTDv06_decodeLiteralsBlock` (src/legacy/zstd_v06.c:3062) | `if (HUFv06_isError(errorCode)) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3544 | `ZSTDv06_decodeLiteralsBlock` (src/legacy/zstd_v06.c:3086) | `if (lhSize+litSize+WILDCOPY_OVERLENGTH > srcSize) {  /* risk reading beyond src buffer with wildcopy */` | `ERROR(corruption_detected)` | [ ] |
| 3545 | `ZSTDv06_decodeLiteralsBlock` (src/legacy/zstd_v06.c:3087) | `if (litSize+lhSize > srcSize) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3546 | `ZSTDv06_decodeLiteralsBlock` (src/legacy/zstd_v06.c:3113) | `if (srcSize<4) return ERROR(corruption_detected);   /* srcSize >= MIN_CBLOCK_SIZE == 3; here we need lhSize+1 = 4 */` | `ERROR(corruption_detected)` | [ ] |
| 3547 | `ZSTDv06_decodeLiteralsBlock` (src/legacy/zstd_v06.c:3116) | `if (litSize > ZSTDv06_BLOCKSIZE_MAX) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3548 | `ZSTDv06_decodeLiteralsBlock` (src/legacy/zstd_v06.c:3123) | `return ERROR(corruption_detected);   /* impossible */` | `ERROR(corruption_detected)` | [ ] |
| 3549 | `ZSTDv06_buildSeqTable` (src/legacy/zstd_v06.c:3139) | `if (!srcSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 3550 | `ZSTDv06_buildSeqTable` (src/legacy/zstd_v06.c:3140) | `if ( (*(const BYTE*)src) > max) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3551 | `ZSTDv06_buildSeqTable` (src/legacy/zstd_v06.c:3147) | `if (!flagRepeatTable) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3552 | `ZSTDv06_buildSeqTable` (src/legacy/zstd_v06.c:3154) | `if (FSEv06_isError(headerSize)) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3553 | `ZSTDv06_buildSeqTable` (src/legacy/zstd_v06.c:3155) | `if (tableLog > maxLog) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3554 | `ZSTDv06_decodeSeqHeaders` (src/legacy/zstd_v06.c:3171) | `if (srcSize < MIN_SEQUENCES_SIZE) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 3555 | `ZSTDv06_decodeSeqHeaders` (src/legacy/zstd_v06.c:3176) | `if (nbSeq > 0x7F) {` | `ERROR(srcSize_wrong)` | [ ] |
| 3556 | `ZSTDv06_decodeSeqHeaders` (src/legacy/zstd_v06.c:3177) | `if (nbSeq == 0xFF) {` | `ERROR(srcSize_wrong)` | [ ] |
| 3557 | `ZSTDv06_decodeSeqHeaders` (src/legacy/zstd_v06.c:3178) | `if (ip+2 > iend) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 3558 | `ZSTDv06_decodeSeqHeaders` (src/legacy/zstd_v06.c:3181) | `if (ip >= iend) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 3559 | `ZSTDv06_decodeSeqHeaders` (src/legacy/zstd_v06.c:3189) | `if (ip + 4 > iend) return ERROR(srcSize_wrong); /* min : header byte + all 3 are "raw", hence no header, but at least xxLog bits per type */` | `ERROR(srcSize_wrong)` | [ ] |
| 3560 | `ZSTDv06_decodeSeqHeaders` (src/legacy/zstd_v06.c:3197) | `if (ZSTDv06_isError(bhSize)) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3561 | `ZSTDv06_decodeSeqHeaders` (src/legacy/zstd_v06.c:3201) | `if (ZSTDv06_isError(bhSize)) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3562 | `ZSTDv06_decodeSeqHeaders` (src/legacy/zstd_v06.c:3205) | `if (ZSTDv06_isError(bhSize)) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3563 | `ZSTDv06_decodeSequence` (src/legacy/zstd_v06.c:3264) | `if (MEM_32bits()) BITv06_reloadDStream(&(seqState->DStream));` | branch-specific rejection/error | [ ] |
| 3564 | `ZSTDv06_decodeSequence` (src/legacy/zstd_v06.c:3267) | `if (offset < ZSTDv06_REP_NUM) {` | branch-specific rejection/error | [ ] |
| 3565 | `ZSTDv06_decodeSequence` (src/legacy/zstd_v06.c:3268) | `if (llCode == 0 && offset <= 1) offset = 1-offset;` | branch-specific rejection/error | [ ] |
| 3566 | `ZSTDv06_decodeSequence` (src/legacy/zstd_v06.c:3270) | `if (offset != 0) {` | branch-specific rejection/error | [ ] |
| 3567 | `ZSTDv06_decodeSequence` (src/legacy/zstd_v06.c:3291) | `if (MEM_32bits() && (mlBits+llBits>24)) BITv06_reloadDStream(&(seqState->DStream));` | branch-specific rejection/error | [ ] |
| 3568 | `ZSTDv06_decodeSequence` (src/legacy/zstd_v06.c:3300) | `if (MEM_32bits()) BITv06_reloadDStream(&(seqState->DStream));     /* <= 18 bits */` | branch-specific rejection/error | [ ] |
| 3569 | `ZSTDv06_execSequence` (src/legacy/zstd_v06.c:3320) | `if (seqLength > (size_t)(oend - op)) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [ ] |
| 3570 | `ZSTDv06_execSequence` (src/legacy/zstd_v06.c:3321) | `if (sequence.litLength > (size_t)(litLimit - *litPtr)) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3571 | `ZSTDv06_execSequence` (src/legacy/zstd_v06.c:3323) | `if (oLitEnd > oend_8) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [ ] |
| 3572 | `ZSTDv06_execSequence` (src/legacy/zstd_v06.c:3325) | `if (oMatchEnd > oend) return ERROR(dstSize_tooSmall);   /* overwrite beyond dst buffer */` | `ERROR(dstSize_tooSmall)` | [ ] |
| 3573 | `ZSTDv06_execSequence` (src/legacy/zstd_v06.c:3326) | `if (iLitEnd > litLimit) return ERROR(corruption_detected);   /* overRead beyond lit buffer */` | `ERROR(corruption_detected)` | [ ] |
| 3574 | `ZSTDv06_execSequence` (src/legacy/zstd_v06.c:3334) | `if (sequence.offset > (size_t)(oLitEnd - base)) {` | `ERROR(corruption_detected)` | [ ] |
| 3575 | `ZSTDv06_execSequence` (src/legacy/zstd_v06.c:3336) | `if (sequence.offset > (size_t)(oLitEnd - vBase)) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3576 | `ZSTDv06_execSequence` (src/legacy/zstd_v06.c:3338) | `if (match + sequence.matchLength <= dictEnd) {` | branch-specific rejection/error | [ ] |
| 3577 | `ZSTDv06_execSequence` (src/legacy/zstd_v06.c:3348) | `if (op > oend_8 \|\| sequence.matchLength < MINMATCH) {` | branch-specific rejection/error | [ ] |
| 3578 | `ZSTDv06_execSequence` (src/legacy/zstd_v06.c:3356) | `if (sequence.offset < 8) {` | branch-specific rejection/error | [ ] |
| 3579 | `ZSTDv06_execSequence` (src/legacy/zstd_v06.c:3373) | `if (oMatchEnd > oend-(16-MINMATCH)) {` | branch-specific rejection/error | [ ] |
| 3580 | `ZSTDv06_execSequence` (src/legacy/zstd_v06.c:3374) | `if (op < oend_8) {` | branch-specific rejection/error | [ ] |
| 3581 | `ZSTDv06_decompressSequences` (src/legacy/zstd_v06.c:3423) | `if (ERR_isError(errorCode)) return ERROR(corruption_detected); }` | `ERROR(corruption_detected)` | [ ] |
| 3582 | `ZSTDv06_decompressSequences` (src/legacy/zstd_v06.c:3434) | `if (start==NULL) start = op;` | branch-specific rejection/error | [ ] |
| 3583 | `ZSTDv06_decompressSequences` (src/legacy/zstd_v06.c:3436) | `if ((pos >= 5810037) && (pos < 5810400))` | branch-specific rejection/error | [ ] |
| 3584 | `ZSTDv06_decompressSequences` (src/legacy/zstd_v06.c:3447) | `if (nbSeq) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3585 | `ZSTDv06_decompressSequences` (src/legacy/zstd_v06.c:3452) | `if (litPtr > litEnd) return ERROR(corruption_detected);   /* too many literals already used */` | `ERROR(corruption_detected)` | [ ] |
| 3586 | `ZSTDv06_decompressSequences` (src/legacy/zstd_v06.c:3453) | `if (op+lastLLSize > oend) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [ ] |
| 3587 | `ZSTDv06_decompressSequences` (src/legacy/zstd_v06.c:3454) | `if (lastLLSize > 0) {` | branch-specific rejection/error | [ ] |
| 3588 | `ZSTDv06_checkContinuity` (src/legacy/zstd_v06.c:3466) | `if (dst != dctx->previousDstEnd) {   /* not contiguous */` | branch-specific rejection/error | [ ] |
| 3589 | `ZSTDv06_decompressBlock_internal` (src/legacy/zstd_v06.c:3481) | `if (srcSize >= ZSTDv06_BLOCKSIZE_MAX) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 3590 | `ZSTDv06_decompressFrame` (src/legacy/zstd_v06.c:3517) | `if (srcSize < ZSTDv06_frameHeaderSize_min+ZSTDv06_blockHeaderSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 3591 | `ZSTDv06_decompressFrame` (src/legacy/zstd_v06.c:3522) | `if (srcSize < frameHeaderSize+ZSTDv06_blockHeaderSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 3592 | `ZSTDv06_decompressFrame` (src/legacy/zstd_v06.c:3523) | `if (ZSTDv06_decodeFrameHeader(dctx, src, frameHeaderSize)) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3593 | `ZSTDv06_decompressFrame` (src/legacy/zstd_v06.c:3535) | `if (cBlockSize > remainingSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 3594 | `ZSTDv06_decompressFrame` (src/legacy/zstd_v06.c:3546) | `return ERROR(GENERIC);   /* not yet supported */` | `ERROR(GENERIC)` | [ ] |
| 3595 | `ZSTDv06_decompressFrame` (src/legacy/zstd_v06.c:3550) | `if (remainingSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 3596 | `ZSTDv06_decompressFrame` (src/legacy/zstd_v06.c:3553) | `return ERROR(GENERIC);   /* impossible */` | `ERROR(GENERIC)` | [ ] |
| 3597 | `ZSTDv06_decompressFrame` (src/legacy/zstd_v06.c:3555) | `if (cBlockSize == 0) break;   /* bt_end */` | branch-specific rejection/error | [ ] |
| 3598 | `ZSTDv06_decompress` (src/legacy/zstd_v06.c:3599) | `if (dctx==NULL) return ERROR(memory_allocation);` | `ERROR(memory_allocation)` | [ ] |
| 3599 | `ZSTDv06_findFrameSizeInfoLegacy` (src/legacy/zstd_v06.c:3631) | `ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, ERROR(prefix_unknown));` | `ERROR(prefix_unknown)` | [ ] |
| 3600 | `ZSTDv06_findFrameSizeInfoLegacy` (src/legacy/zstd_v06.c:3634) | `if (srcSize < frameHeaderSize+ZSTDv06_blockHeaderSize) {` | `ERROR(srcSize_wrong)` | [ ] |
| 3601 | `ZSTDv06_findFrameSizeInfoLegacy` (src/legacy/zstd_v06.c:3635) | `ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, ERROR(srcSize_wrong));` | `ERROR(srcSize_wrong)` | [ ] |
| 3602 | `ZSTDv06_findFrameSizeInfoLegacy` (src/legacy/zstd_v06.c:3651) | `if (cBlockSize > remainingSize) {` | `ERROR(srcSize_wrong)` | [ ] |
| 3603 | `ZSTDv06_findFrameSizeInfoLegacy` (src/legacy/zstd_v06.c:3652) | `ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, ERROR(srcSize_wrong));` | `ERROR(srcSize_wrong)` | [ ] |
| 3604 | `ZSTDv06_findFrameSizeInfoLegacy` (src/legacy/zstd_v06.c:3656) | `if (cBlockSize == 0) break;   /* bt_end */` | branch-specific rejection/error | [ ] |
| 3605 | `ZSTDv06_decompressContinue` (src/legacy/zstd_v06.c:3678) | `if (srcSize != dctx->expected) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 3606 | `ZSTDv06_decompressContinue` (src/legacy/zstd_v06.c:3685) | `if (srcSize != ZSTDv06_frameHeaderSize_min) return ERROR(srcSize_wrong);   /* impossible */` | `ERROR(srcSize_wrong)` | [ ] |
| 3607 | `ZSTDv06_decompressContinue` (src/legacy/zstd_v06.c:3687) | `if (ZSTDv06_isError(dctx->headerSize)) return dctx->headerSize;` | branch-specific rejection/error | [ ] |
| 3608 | `ZSTDv06_decompressContinue` (src/legacy/zstd_v06.c:3689) | `if (dctx->headerSize > ZSTDv06_frameHeaderSize_min) {` | branch-specific rejection/error | [ ] |
| 3609 | `ZSTDv06_decompressContinue` (src/legacy/zstd_v06.c:3730) | `return ERROR(GENERIC);   /* not yet handled */` | `ERROR(GENERIC)` | [ ] |
| 3610 | `ZSTDv06_decompressContinue` (src/legacy/zstd_v06.c:3736) | `return ERROR(GENERIC);   /* impossible */` | `ERROR(GENERIC)` | [ ] |
| 3611 | `ZSTDv06_decompressContinue` (src/legacy/zstd_v06.c:3745) | `return ERROR(GENERIC);   /* impossible */` | `ERROR(GENERIC)` | [ ] |
| 3612 | `ZSTDv06_loadEntropy` (src/legacy/zstd_v06.c:3763) | `if (HUFv06_isError(hSize)) return ERROR(dictionary_corrupted);` | `ERROR(dictionary_corrupted)` | [ ] |
| 3613 | `ZSTDv06_loadEntropy` (src/legacy/zstd_v06.c:3770) | `if (FSEv06_isError(offcodeHeaderSize)) return ERROR(dictionary_corrupted);` | `ERROR(dictionary_corrupted)` | [ ] |
| 3614 | `ZSTDv06_loadEntropy` (src/legacy/zstd_v06.c:3771) | `if (offcodeLog > OffFSELog) return ERROR(dictionary_corrupted);` | `ERROR(dictionary_corrupted)` | [ ] |
| 3615 | `ZSTDv06_loadEntropy` (src/legacy/zstd_v06.c:3773) | `if (FSEv06_isError(errorCode)) return ERROR(dictionary_corrupted); }` | `ERROR(dictionary_corrupted)` | [ ] |
| 3616 | `ZSTDv06_loadEntropy` (src/legacy/zstd_v06.c:3781) | `if (FSEv06_isError(matchlengthHeaderSize)) return ERROR(dictionary_corrupted);` | `ERROR(dictionary_corrupted)` | [ ] |
| 3617 | `ZSTDv06_loadEntropy` (src/legacy/zstd_v06.c:3782) | `if (matchlengthLog > MLFSELog) return ERROR(dictionary_corrupted);` | `ERROR(dictionary_corrupted)` | [ ] |
| 3618 | `ZSTDv06_loadEntropy` (src/legacy/zstd_v06.c:3784) | `if (FSEv06_isError(errorCode)) return ERROR(dictionary_corrupted); }` | `ERROR(dictionary_corrupted)` | [ ] |
| 3619 | `ZSTDv06_loadEntropy` (src/legacy/zstd_v06.c:3792) | `if (FSEv06_isError(litlengthHeaderSize)) return ERROR(dictionary_corrupted);` | `ERROR(dictionary_corrupted)` | [ ] |
| 3620 | `ZSTDv06_loadEntropy` (src/legacy/zstd_v06.c:3793) | `if (litlengthLog > LLFSELog) return ERROR(dictionary_corrupted);` | `ERROR(dictionary_corrupted)` | [ ] |
| 3621 | `ZSTDv06_loadEntropy` (src/legacy/zstd_v06.c:3795) | `if (FSEv06_isError(errorCode)) return ERROR(dictionary_corrupted); }` | `ERROR(dictionary_corrupted)` | [ ] |
| 3622 | `ZSTDv06_decompress_insertDictionary` (src/legacy/zstd_v06.c:3815) | `if (ZSTDv06_isError(eSize)) return ERROR(dictionary_corrupted);` | `ERROR(dictionary_corrupted)` | [ ] |
| 3623 | `ZSTDv06_decompressBegin_usingDict` (src/legacy/zstd_v06.c:3833) | `if (ZSTDv06_isError(errorCode)) return ERROR(dictionary_corrupted);` | `ERROR(dictionary_corrupted)` | [ ] |
| 3624 | `ZBUFFv06_createDCtx` (src/legacy/zstd_v06.c:3919) | `if (zbd==NULL) return NULL;` | `NULL` | [ ] |
| 3625 | `ZBUFFv06_createDCtx` (src/legacy/zstd_v06.c:3922) | `if (zbd->zd==NULL) {` | `NULL` | [ ] |
| 3626 | `ZBUFFv06_createDCtx` (src/legacy/zstd_v06.c:3924) | `return NULL;` | `NULL` | [ ] |
| 3627 | `ZBUFFv06_freeDCtx` (src/legacy/zstd_v06.c:3932) | `if (zbd==NULL) return 0;   /* support free on null */` | branch-specific rejection/error | [ ] |
| 3628 | `ZBUFFv06_limitCopy` (src/legacy/zstd_v06.c:3960) | `if (length > 0) {` | branch-specific rejection/error | [ ] |
| 3629 | `ZBUFFv06_decompressContinue` (src/legacy/zstd_v06.c:3985) | `return ERROR(init_missing);` | `ERROR(init_missing)` | [ ] |
| 3630 | `ZBUFFv06_decompressContinue` (src/legacy/zstd_v06.c:3989) | `if (hSize != 0) {` | branch-specific rejection/error | [ ] |
| 3631 | `ZBUFFv06_decompressContinue` (src/legacy/zstd_v06.c:3992) | `if (toLoad > (size_t)(iend-ip)) {   /* not enough input to load full header */` | branch-specific rejection/error | [ ] |
| 3632 | `ZBUFFv06_decompressContinue` (src/legacy/zstd_v06.c:3993) | `if (ip != NULL)` | branch-specific rejection/error | [ ] |
| 3633 | `ZBUFFv06_decompressContinue` (src/legacy/zstd_v06.c:4007) | `if (h1Size < zbd->lhSize) {   /* long header */` | branch-specific rejection/error | [ ] |
| 3634 | `ZBUFFv06_decompressContinue` (src/legacy/zstd_v06.c:4016) | `if (zbd->inBuffSize < blockSize) {` | `ERROR(memory_allocation)` | [ ] |
| 3635 | `ZBUFFv06_decompressContinue` (src/legacy/zstd_v06.c:4020) | `if (zbd->inBuff == NULL) return ERROR(memory_allocation);` | `ERROR(memory_allocation)` | [ ] |
| 3636 | `ZBUFFv06_decompressContinue` (src/legacy/zstd_v06.c:4023) | `if (zbd->outBuffSize < neededOutSize) {` | `ERROR(memory_allocation)` | [ ] |
| 3637 | `ZBUFFv06_decompressContinue` (src/legacy/zstd_v06.c:4027) | `if (zbd->outBuff == NULL) return ERROR(memory_allocation);` | `ERROR(memory_allocation)` | [ ] |
| 3638 | `ZBUFFv06_decompressContinue` (src/legacy/zstd_v06.c:4033) | `if (neededInSize==0) {  /* end of frame */` | branch-specific rejection/error | [ ] |
| 3639 | `ZBUFFv06_decompressContinue` (src/legacy/zstd_v06.c:4038) | `if ((size_t)(iend-ip) >= neededInSize) {  /* decode directly from src */` | branch-specific rejection/error | [ ] |
| 3640 | `ZBUFFv06_decompressContinue` (src/legacy/zstd_v06.c:4057) | `if (toLoad > zbd->inBuffSize - zbd->inPos) return ERROR(corruption_detected);   /* should never happen */` | `ERROR(corruption_detected)` | [ ] |
| 3641 | `ZBUFFv06_decompressContinue` (src/legacy/zstd_v06.c:4061) | `if (loadedSize < toLoad) { notDone = 0; break; }   /* not enough input, wait for more */` | branch-specific rejection/error | [ ] |
| 3642 | `ZBUFFv06_decompressContinue` (src/legacy/zstd_v06.c:4069) | `if (!decodedSize) { zbd->stage = ZBUFFds_read; break; }   /* this was just a header */` | branch-specific rejection/error | [ ] |
| 3643 | `ZBUFFv06_decompressContinue` (src/legacy/zstd_v06.c:4083) | `if (zbd->outStart + zbd->blockSize > zbd->outBuffSize)` | branch-specific rejection/error | [ ] |
| 3644 | `ZBUFFv06_decompressContinue` (src/legacy/zstd_v06.c:4091) | `default: return ERROR(GENERIC);   /* impossible */` | `ERROR(GENERIC)` | [ ] |
| 3645 | `ZBUFFv06_decompressContinue` (src/legacy/zstd_v06.c:4098) | `if (nextSrcSizeHint > ZSTDv06_blockHeaderSize) nextSrcSizeHint+= ZSTDv06_blockHeaderSize;   /* get following block header too */` | branch-specific rejection/error | [ ] |
| 3646 | `BITv07_initDStream` (src/legacy/zstd_v07.c:504) | `if (srcSize < 1) { memset(bitD, 0, sizeof(*bitD)); return ERROR(srcSize_wrong); }` | `ERROR(srcSize_wrong)` | [ ] |
| 3647 | `BITv07_initDStream` (src/legacy/zstd_v07.c:506) | `if (srcSize >=  sizeof(bitD->bitContainer)) {  /* normal case */` | branch-specific rejection/error | [ ] |
| 3648 | `BITv07_initDStream` (src/legacy/zstd_v07.c:512) | `if (lastByte == 0) return ERROR(GENERIC); /* endMark not present */ }` | `ERROR(GENERIC)` | [ ] |
| 3649 | `BITv07_initDStream` (src/legacy/zstd_v07.c:529) | `if (lastByte == 0) return ERROR(GENERIC); /* endMark not present */ }` | `ERROR(GENERIC)` | [ ] |
| 3650 | `BITv07_reloadDStream` (src/legacy/zstd_v07.c:574) | `if (bitD->bitsConsumed > (sizeof(bitD->bitContainer)*8))  /* should not happen => corruption detected */` | branch-specific rejection/error | [ ] |
| 3651 | `BITv07_reloadDStream` (src/legacy/zstd_v07.c:577) | `if (bitD->ptr >= bitD->start + sizeof(bitD->bitContainer)) {` | branch-specific rejection/error | [ ] |
| 3652 | `BITv07_reloadDStream` (src/legacy/zstd_v07.c:583) | `if (bitD->ptr == bitD->start) {` | branch-specific rejection/error | [ ] |
| 3653 | `BITv07_reloadDStream` (src/legacy/zstd_v07.c:584) | `if (bitD->bitsConsumed < sizeof(bitD->bitContainer)*8) return BITv07_DStream_endOfBuffer;` | branch-specific rejection/error | [ ] |
| 3654 | `BITv07_reloadDStream` (src/legacy/zstd_v07.c:589) | `if (bitD->ptr - nbBytes < bitD->start) {` | branch-specific rejection/error | [ ] |
| 3655 | `FSEv07_readNCount` (src/legacy/zstd_v07.c:1166) | `if (hbSize < 4) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 3656 | `FSEv07_readNCount` (src/legacy/zstd_v07.c:1169) | `if (nbBits > FSEv07_TABLELOG_ABSOLUTE_MAX) return ERROR(tableLog_tooLarge);` | `ERROR(tableLog_tooLarge)` | [ ] |
| 3657 | `FSEv07_readNCount` (src/legacy/zstd_v07.c:1182) | `if (ip < iend-5) {` | branch-specific rejection/error | [ ] |
| 3658 | `FSEv07_readNCount` (src/legacy/zstd_v07.c:1196) | `if (n0 > *maxSVPtr) return ERROR(maxSymbolValue_tooSmall);` | `ERROR(maxSymbolValue_tooSmall)` | [ ] |
| 3659 | `FSEv07_readNCount` (src/legacy/zstd_v07.c:1198) | `if ((ip <= iend-7) \|\| (ip + (bitCount>>3) <= iend-4)) {` | branch-specific rejection/error | [ ] |
| 3660 | `FSEv07_readNCount` (src/legacy/zstd_v07.c:1209) | `if ((bitStream & (threshold-1)) < (U32)max) {` | branch-specific rejection/error | [ ] |
| 3661 | `FSEv07_readNCount` (src/legacy/zstd_v07.c:1214) | `if (count >= threshold) count -= max;` | branch-specific rejection/error | [ ] |
| 3662 | `FSEv07_readNCount` (src/legacy/zstd_v07.c:1227) | `if ((ip <= iend-7) \|\| (ip + (bitCount>>3) <= iend-4)) {` | branch-specific rejection/error | [ ] |
| 3663 | `FSEv07_readNCount` (src/legacy/zstd_v07.c:1236) | `if (remaining != 1) return ERROR(GENERIC);` | `ERROR(GENERIC)` | [ ] |
| 3664 | `FSEv07_readNCount` (src/legacy/zstd_v07.c:1240) | `if ((size_t)(ip-istart) > hbSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 3665 | `HUFv07_readStats` (src/legacy/zstd_v07.c:1260) | `if (!srcSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 3666 | `HUFv07_readStats` (src/legacy/zstd_v07.c:1264) | `if (iSize >= 128)  { /* special header */` | branch-specific rejection/error | [ ] |
| 3667 | `HUFv07_readStats` (src/legacy/zstd_v07.c:1265) | `if (iSize >= (242)) {  /* RLE */` | branch-specific rejection/error | [ ] |
| 3668 | `HUFv07_readStats` (src/legacy/zstd_v07.c:1274) | `if (iSize+1 > srcSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 3669 | `HUFv07_readStats` (src/legacy/zstd_v07.c:1275) | `if (oSize >= hwSize) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3670 | `HUFv07_readStats` (src/legacy/zstd_v07.c:1283) | `if (iSize+1 > srcSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 3671 | `HUFv07_readStats` (src/legacy/zstd_v07.c:1292) | `if (huffWeight[n] >= HUFv07_TABLELOG_ABSOLUTEMAX) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3672 | `HUFv07_readStats` (src/legacy/zstd_v07.c:1296) | `if (weightTotal == 0) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3673 | `HUFv07_readStats` (src/legacy/zstd_v07.c:1300) | `if (tableLog > HUFv07_TABLELOG_ABSOLUTEMAX) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3674 | `HUFv07_readStats` (src/legacy/zstd_v07.c:1307) | `if (verif != rest) return ERROR(corruption_detected);    /* last value must be a clean power of 2 */` | `ERROR(corruption_detected)` | [ ] |
| 3675 | `HUFv07_readStats` (src/legacy/zstd_v07.c:1313) | `if ((rankStats[1] < 2) \|\| (rankStats[1] & 1)) return ERROR(corruption_detected);   /* by construction : at least 2 elts of rank 1, must be even */` | `ERROR(corruption_detected)` | [ ] |
| 3676 | `FSEv07_createDTable` (src/legacy/zstd_v07.c:1414) | `if (tableLog > FSEv07_TABLELOG_ABSOLUTE_MAX) tableLog = FSEv07_TABLELOG_ABSOLUTE_MAX;` | branch-specific rejection/error | [ ] |
| 3677 | `FSEv07_buildDTable` (src/legacy/zstd_v07.c:1434) | `if (maxSymbolValue > FSEv07_MAX_SYMBOL_VALUE) return ERROR(maxSymbolValue_tooLarge);` | `ERROR(maxSymbolValue_tooLarge)` | [ ] |
| 3678 | `FSEv07_buildDTable` (src/legacy/zstd_v07.c:1435) | `if (tableLog > FSEv07_MAX_TABLELOG) return ERROR(tableLog_tooLarge);` | `ERROR(tableLog_tooLarge)` | [ ] |
| 3679 | `FSEv07_buildDTable` (src/legacy/zstd_v07.c:1448) | `if (normalizedCounter[s] >= largeLimit) DTableH.fastMode=0;` | branch-specific rejection/error | [ ] |
| 3680 | `FSEv07_buildDTable` (src/legacy/zstd_v07.c:1466) | `if (position!=0) return ERROR(GENERIC);   /* position must reach all cells once, otherwise normalizedCounter is incorrect */` | `ERROR(GENERIC)` | [ ] |
| 3681 | `FSEv07_buildDTable_raw` (src/legacy/zstd_v07.c:1518) | `if (nbBits < 1) return ERROR(GENERIC);         /* min size */` | `ERROR(GENERIC)` | [ ] |
| 3682 | `FSEv07_decompress_usingDTable_generic` (src/legacy/zstd_v07.c:1559) | `if (FSEv07_MAX_TABLELOG*2+7 > sizeof(bitD.bitContainer)*8)    /* This test must be static */` | branch-specific rejection/error | [ ] |
| 3683 | `FSEv07_decompress_usingDTable_generic` (src/legacy/zstd_v07.c:1564) | `if (FSEv07_MAX_TABLELOG*4+7 > sizeof(bitD.bitContainer)*8)    /* This test must be static */` | branch-specific rejection/error | [ ] |
| 3684 | `FSEv07_decompress_usingDTable_generic` (src/legacy/zstd_v07.c:1565) | `{ if (BITv07_reloadDStream(&bitD) > BITv07_DStream_unfinished) { op+=2; break; } }` | branch-specific rejection/error | [ ] |
| 3685 | `FSEv07_decompress_usingDTable_generic` (src/legacy/zstd_v07.c:1569) | `if (FSEv07_MAX_TABLELOG*2+7 > sizeof(bitD.bitContainer)*8)    /* This test must be static */` | branch-specific rejection/error | [ ] |
| 3686 | `FSEv07_decompress_usingDTable_generic` (src/legacy/zstd_v07.c:1578) | `if (op>(omax-2)) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [ ] |
| 3687 | `FSEv07_decompress_usingDTable_generic` (src/legacy/zstd_v07.c:1587) | `if (op>(omax-2)) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [ ] |
| 3688 | `FSEv07_decompress` (src/legacy/zstd_v07.c:1623) | `if (cSrcSize<2) return ERROR(srcSize_wrong);   /* too small input size */` | `ERROR(srcSize_wrong)` | [ ] |
| 3689 | `FSEv07_decompress` (src/legacy/zstd_v07.c:1628) | `if (NCountLength >= cSrcSize) return ERROR(srcSize_wrong);   /* too small input size */` | `ERROR(srcSize_wrong)` | [ ] |
| 3690 | `HUFv07_readDTableX2` (src/legacy/zstd_v07.c:1739) | `if (tableLog > (U32)(dtd.maxTableLog+1)) return ERROR(tableLog_tooLarge);   /* DTable too small, huffman tree cannot fit in */` | `ERROR(tableLog_tooLarge)` | [ ] |
| 3691 | `HUFv07_decodeSymbolX2` (src/legacy/zstd_v07.c:1782) | `if (MEM_64bits() \|\| (HUFv07_TABLELOG_MAX<=12)) \` | branch-specific rejection/error | [ ] |
| 3692 | `HUFv07_decompress1X2_usingDTable_internal` (src/legacy/zstd_v07.c:1831) | `if (!BITv07_endOfDStream(&bitD)) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3693 | `HUFv07_decompress1X2_usingDTable` (src/legacy/zstd_v07.c:1842) | `if (dtd.tableType != 0) return ERROR(GENERIC);` | `ERROR(GENERIC)` | [ ] |
| 3694 | `HUFv07_decompress1X2_DCtx` (src/legacy/zstd_v07.c:1852) | `if (hSize >= cSrcSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 3695 | `HUFv07_decompress4X2_usingDTable_internal` (src/legacy/zstd_v07.c:1871) | `if (cSrcSize < 10) return ERROR(corruption_detected);  /* strict minimum : jump table + 1 byte per stream */` | `ERROR(corruption_detected)` | [ ] |
| 3696 | `HUFv07_decompress4X2_usingDTable_internal` (src/legacy/zstd_v07.c:1904) | `if (length4 > cSrcSize) return ERROR(corruption_detected);   /* overflow */` | `ERROR(corruption_detected)` | [ ] |
| 3697 | `HUFv07_decompress4X2_usingDTable_internal` (src/legacy/zstd_v07.c:1937) | `if (op1 > opStart2) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3698 | `HUFv07_decompress4X2_usingDTable_internal` (src/legacy/zstd_v07.c:1938) | `if (op2 > opStart3) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3699 | `HUFv07_decompress4X2_usingDTable_internal` (src/legacy/zstd_v07.c:1939) | `if (op3 > opStart4) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3700 | `HUFv07_decompress4X2_usingDTable_internal` (src/legacy/zstd_v07.c:1950) | `if (!endSignal) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3701 | `HUFv07_decompress4X2_usingDTable` (src/legacy/zstd_v07.c:1964) | `if (dtd.tableType != 0) return ERROR(GENERIC);` | `ERROR(GENERIC)` | [ ] |
| 3702 | `HUFv07_decompress4X2_DCtx` (src/legacy/zstd_v07.c:1975) | `if (hSize >= cSrcSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 3703 | `HUFv07_fillDTableX4Level2` (src/legacy/zstd_v07.c:2007) | `if (minWeight>1) {` | branch-specific rejection/error | [ ] |
| 3704 | `HUFv07_fillDTableX4` (src/legacy/zstd_v07.c:2057) | `if (targetLog-nbBits >= minBits) {   /* enough room for a second symbol */` | branch-specific rejection/error | [ ] |
| 3705 | `HUFv07_fillDTableX4` (src/legacy/zstd_v07.c:2060) | `if (minWeight < 1) minWeight = 1;` | branch-specific rejection/error | [ ] |
| 3706 | `HUFv07_readDTableX4` (src/legacy/zstd_v07.c:2095) | `if (maxTableLog > HUFv07_TABLELOG_ABSOLUTEMAX) return ERROR(tableLog_tooLarge);` | `ERROR(tableLog_tooLarge)` | [ ] |
| 3707 | `HUFv07_readDTableX4` (src/legacy/zstd_v07.c:2102) | `if (tableLog > maxTableLog) return ERROR(tableLog_tooLarge);   /* DTable can't fit code depth */` | `ERROR(tableLog_tooLarge)` | [ ] |
| 3708 | `HUFv07_decodeLastSymbolX4` (src/legacy/zstd_v07.c:2174) | `if (DStream->bitsConsumed < (sizeof(DStream->bitContainer)*8)) {` | branch-specific rejection/error | [ ] |
| 3709 | `HUFv07_decodeLastSymbolX4` (src/legacy/zstd_v07.c:2176) | `if (DStream->bitsConsumed > (sizeof(DStream->bitContainer)*8))` | branch-specific rejection/error | [ ] |
| 3710 | `HUFv07_decodeLastSymbolX4` (src/legacy/zstd_v07.c:2187) | `if (MEM_64bits() \|\| (HUFv07_TABLELOG_MAX<=12)) \` | branch-specific rejection/error | [ ] |
| 3711 | `HUFv07_decodeStreamX4` (src/legacy/zstd_v07.c:2213) | `if (p < pEnd)` | branch-specific rejection/error | [ ] |
| 3712 | `HUFv07_decompress1X4_usingDTable_internal` (src/legacy/zstd_v07.c:2242) | `if (!BITv07_endOfDStream(&bitD)) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3713 | `HUFv07_decompress1X4_usingDTable` (src/legacy/zstd_v07.c:2254) | `if (dtd.tableType != 1) return ERROR(GENERIC);` | `ERROR(GENERIC)` | [ ] |
| 3714 | `HUFv07_decompress1X4_DCtx` (src/legacy/zstd_v07.c:2264) | `if (hSize >= cSrcSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 3715 | `HUFv07_decompress4X4_usingDTable_internal` (src/legacy/zstd_v07.c:2281) | `if (cSrcSize < 10) return ERROR(corruption_detected);   /* strict minimum : jump table + 1 byte per stream */` | `ERROR(corruption_detected)` | [ ] |
| 3716 | `HUFv07_decompress4X4_usingDTable_internal` (src/legacy/zstd_v07.c:2314) | `if (length4 > cSrcSize) return ERROR(corruption_detected);   /* overflow */` | `ERROR(corruption_detected)` | [ ] |
| 3717 | `HUFv07_decompress4X4_usingDTable_internal` (src/legacy/zstd_v07.c:2348) | `if (op1 > opStart2) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3718 | `HUFv07_decompress4X4_usingDTable_internal` (src/legacy/zstd_v07.c:2349) | `if (op2 > opStart3) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3719 | `HUFv07_decompress4X4_usingDTable_internal` (src/legacy/zstd_v07.c:2350) | `if (op3 > opStart4) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3720 | `HUFv07_decompress4X4_usingDTable_internal` (src/legacy/zstd_v07.c:2361) | `if (!endCheck) return ERROR(corruption_detected); }` | `ERROR(corruption_detected)` | [ ] |
| 3721 | `HUFv07_decompress4X4_usingDTable` (src/legacy/zstd_v07.c:2375) | `if (dtd.tableType != 1) return ERROR(GENERIC);` | `ERROR(GENERIC)` | [ ] |
| 3722 | `HUFv07_decompress4X4_DCtx` (src/legacy/zstd_v07.c:2386) | `if (hSize >= cSrcSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 3723 | `HUFv07_decompress` (src/legacy/zstd_v07.c:2469) | `if (dstSize == 0) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [ ] |
| 3724 | `HUFv07_decompress` (src/legacy/zstd_v07.c:2470) | `if (cSrcSize > dstSize) return ERROR(corruption_detected);   /* invalid */` | `ERROR(corruption_detected)` | [ ] |
| 3725 | `HUFv07_decompress4X_DCtx` (src/legacy/zstd_v07.c:2485) | `if (dstSize == 0) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [ ] |
| 3726 | `HUFv07_decompress4X_DCtx` (src/legacy/zstd_v07.c:2486) | `if (cSrcSize > dstSize) return ERROR(corruption_detected);   /* invalid */` | `ERROR(corruption_detected)` | [ ] |
| 3727 | `HUFv07_decompress4X_hufOnly` (src/legacy/zstd_v07.c:2499) | `if (dstSize == 0) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [ ] |
| 3728 | `HUFv07_decompress4X_hufOnly` (src/legacy/zstd_v07.c:2500) | `if ((cSrcSize >= dstSize) \|\| (cSrcSize <= 1)) return ERROR(corruption_detected);   /* invalid */` | `ERROR(corruption_detected)` | [ ] |
| 3729 | `HUFv07_decompress1X_DCtx` (src/legacy/zstd_v07.c:2511) | `if (dstSize == 0) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [ ] |
| 3730 | `HUFv07_decompress1X_DCtx` (src/legacy/zstd_v07.c:2512) | `if (cSrcSize > dstSize) return ERROR(corruption_detected);   /* invalid */` | `ERROR(corruption_detected)` | [ ] |
| 3731 | `ZSTDv07_createDCtx_advanced` (src/legacy/zstd_v07.c:2930) | `return NULL;` | `NULL` | [ ] |
| 3732 | `ZSTDv07_createDCtx_advanced` (src/legacy/zstd_v07.c:2933) | `if (!dctx) return NULL;` | `NULL` | [ ] |
| 3733 | `ZSTDv07_freeDCtx` (src/legacy/zstd_v07.c:2946) | `if (dctx==NULL) return 0;   /* support free on NULL */` | branch-specific rejection/error | [ ] |
| 3734 | `ZSTDv07_frameHeaderSize` (src/legacy/zstd_v07.c:3079) | `if (srcSize < ZSTDv07_frameHeaderSize_min) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 3735 | `ZSTDv07_getFrameParams` (src/legacy/zstd_v07.c:3099) | `if (srcSize < ZSTDv07_frameHeaderSize_min) return ZSTDv07_frameHeaderSize_min;` | branch-specific rejection/error | [ ] |
| 3736 | `ZSTDv07_getFrameParams` (src/legacy/zstd_v07.c:3103) | `if (srcSize < ZSTDv07_skippableHeaderSize) return ZSTDv07_skippableHeaderSize; /* magic number + skippable frame length */` | branch-specific rejection/error | [ ] |
| 3737 | `ZSTDv07_getFrameParams` (src/legacy/zstd_v07.c:3108) | `return ERROR(prefix_unknown);` | `ERROR(prefix_unknown)` | [ ] |
| 3738 | `ZSTDv07_getFrameParams` (src/legacy/zstd_v07.c:3113) | `if (srcSize < fhsize) return fhsize; }` | branch-specific rejection/error | [ ] |
| 3739 | `ZSTDv07_getFrameParams` (src/legacy/zstd_v07.c:3125) | `if ((fhdByte & 0x08) != 0)   /* reserved bits, which must be zero */` | `ERROR(frameParameter_unsupported)` | [ ] |
| 3740 | `ZSTDv07_getFrameParams` (src/legacy/zstd_v07.c:3126) | `return ERROR(frameParameter_unsupported);` | `ERROR(frameParameter_unsupported)` | [ ] |
| 3741 | `ZSTDv07_getFrameParams` (src/legacy/zstd_v07.c:3130) | `if (windowLog > ZSTDv07_WINDOWLOG_MAX)` | `ERROR(frameParameter_unsupported)` | [ ] |
| 3742 | `ZSTDv07_getFrameParams` (src/legacy/zstd_v07.c:3131) | `return ERROR(frameParameter_unsupported);` | `ERROR(frameParameter_unsupported)` | [ ] |
| 3743 | `ZSTDv07_getFrameParams` (src/legacy/zstd_v07.c:3153) | `if (windowSize > windowSizeMax)` | `ERROR(frameParameter_unsupported)` | [ ] |
| 3744 | `ZSTDv07_getFrameParams` (src/legacy/zstd_v07.c:3154) | `return ERROR(frameParameter_unsupported);` | `ERROR(frameParameter_unsupported)` | [ ] |
| 3745 | `ZSTDv07_getDecompressedSize` (src/legacy/zstd_v07.c:3175) | `if (frResult!=0) return 0;` | branch-specific rejection/error | [ ] |
| 3746 | `ZSTDv07_decodeFrameHeader` (src/legacy/zstd_v07.c:3186) | `if (dctx->fParams.dictID && (dctx->dictID != dctx->fParams.dictID)) return ERROR(dictionary_wrong);` | `ERROR(dictionary_wrong)` | [ ] |
| 3747 | `ZSTDv07_decodeFrameHeader` (src/legacy/zstd_v07.c:3187) | `if (dctx->fParams.checksumFlag) XXH64_reset(&dctx->xxhState, 0);` | branch-specific rejection/error | [ ] |
| 3748 | `ZSTDv07_getcBlockSize` (src/legacy/zstd_v07.c:3205) | `if (srcSize < ZSTDv07_blockHeaderSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 3749 | `ZSTDv07_getcBlockSize` (src/legacy/zstd_v07.c:3211) | `if (bpPtr->blockType == bt_end) return 0;` | branch-specific rejection/error | [ ] |
| 3750 | `ZSTDv07_getcBlockSize` (src/legacy/zstd_v07.c:3212) | `if (bpPtr->blockType == bt_rle) return 1;` | branch-specific rejection/error | [ ] |
| 3751 | `ZSTDv07_copyRawBlock` (src/legacy/zstd_v07.c:3219) | `if (srcSize > dstCapacity) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [ ] |
| 3752 | `ZSTDv07_copyRawBlock` (src/legacy/zstd_v07.c:3220) | `if (srcSize > 0) {` | branch-specific rejection/error | [ ] |
| 3753 | `ZSTDv07_decodeLiteralsBlock` (src/legacy/zstd_v07.c:3234) | `if (srcSize < MIN_CBLOCK_SIZE) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3754 | `ZSTDv07_decodeLiteralsBlock` (src/legacy/zstd_v07.c:3241) | `if (srcSize < 5) return ERROR(corruption_detected);   /* srcSize >= MIN_CBLOCK_SIZE == 3; here we need up to 5 for lhSize, + cSize (+nbSeq) */` | `ERROR(corruption_detected)` | [ ] |
| 3755 | `ZSTDv07_decodeLiteralsBlock` (src/legacy/zstd_v07.c:3264) | `if (litSize > ZSTDv07_BLOCKSIZE_ABSOLUTEMAX) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3756 | `ZSTDv07_decodeLiteralsBlock` (src/legacy/zstd_v07.c:3265) | `if (litCSize + lhSize > srcSize) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3757 | `ZSTDv07_decodeLiteralsBlock` (src/legacy/zstd_v07.c:3270) | `return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3758 | `ZSTDv07_decodeLiteralsBlock` (src/legacy/zstd_v07.c:3282) | `return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3759 | `ZSTDv07_decodeLiteralsBlock` (src/legacy/zstd_v07.c:3283) | `if (dctx->litEntropy==0)` | `ERROR(dictionary_corrupted)` | [ ] |
| 3760 | `ZSTDv07_decodeLiteralsBlock` (src/legacy/zstd_v07.c:3284) | `return ERROR(dictionary_corrupted);` | `ERROR(dictionary_corrupted)` | [ ] |
| 3761 | `ZSTDv07_decodeLiteralsBlock` (src/legacy/zstd_v07.c:3290) | `if (litCSize + lhSize > srcSize) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3762 | `ZSTDv07_decodeLiteralsBlock` (src/legacy/zstd_v07.c:3293) | `if (HUFv07_isError(errorCode)) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3763 | `ZSTDv07_decodeLiteralsBlock` (src/legacy/zstd_v07.c:3317) | `if (lhSize+litSize+WILDCOPY_OVERLENGTH > srcSize) {  /* risk reading beyond src buffer with wildcopy */` | `ERROR(corruption_detected)` | [ ] |
| 3764 | `ZSTDv07_decodeLiteralsBlock` (src/legacy/zstd_v07.c:3318) | `if (litSize+lhSize > srcSize) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3765 | `ZSTDv07_decodeLiteralsBlock` (src/legacy/zstd_v07.c:3344) | `if (srcSize<4) return ERROR(corruption_detected);   /* srcSize >= MIN_CBLOCK_SIZE == 3; here we need lhSize+1 = 4 */` | `ERROR(corruption_detected)` | [ ] |
| 3766 | `ZSTDv07_decodeLiteralsBlock` (src/legacy/zstd_v07.c:3347) | `if (litSize > ZSTDv07_BLOCKSIZE_ABSOLUTEMAX) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3767 | `ZSTDv07_decodeLiteralsBlock` (src/legacy/zstd_v07.c:3354) | `return ERROR(corruption_detected);   /* impossible */` | `ERROR(corruption_detected)` | [ ] |
| 3768 | `ZSTDv07_buildSeqTable` (src/legacy/zstd_v07.c:3370) | `if (!srcSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 3769 | `ZSTDv07_buildSeqTable` (src/legacy/zstd_v07.c:3371) | `if ( (*(const BYTE*)src) > max) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3770 | `ZSTDv07_buildSeqTable` (src/legacy/zstd_v07.c:3378) | `if (!flagRepeatTable) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3771 | `ZSTDv07_buildSeqTable` (src/legacy/zstd_v07.c:3385) | `if (FSEv07_isError(headerSize)) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3772 | `ZSTDv07_buildSeqTable` (src/legacy/zstd_v07.c:3386) | `if (tableLog > maxLog) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3773 | `ZSTDv07_decodeSeqHeaders` (src/legacy/zstd_v07.c:3402) | `if (srcSize < MIN_SEQUENCES_SIZE) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 3774 | `ZSTDv07_decodeSeqHeaders` (src/legacy/zstd_v07.c:3407) | `if (nbSeq > 0x7F) {` | `ERROR(srcSize_wrong)` | [ ] |
| 3775 | `ZSTDv07_decodeSeqHeaders` (src/legacy/zstd_v07.c:3408) | `if (nbSeq == 0xFF) {` | `ERROR(srcSize_wrong)` | [ ] |
| 3776 | `ZSTDv07_decodeSeqHeaders` (src/legacy/zstd_v07.c:3409) | `if (ip+2 > iend) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 3777 | `ZSTDv07_decodeSeqHeaders` (src/legacy/zstd_v07.c:3412) | `if (ip >= iend) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 3778 | `ZSTDv07_decodeSeqHeaders` (src/legacy/zstd_v07.c:3420) | `if (ip + 4 > iend) return ERROR(srcSize_wrong); /* min : header byte + all 3 are "raw", hence no header, but at least xxLog bits per type */` | `ERROR(srcSize_wrong)` | [ ] |
| 3779 | `ZSTDv07_decodeSeqHeaders` (src/legacy/zstd_v07.c:3428) | `if (ZSTDv07_isError(llhSize)) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3780 | `ZSTDv07_decodeSeqHeaders` (src/legacy/zstd_v07.c:3432) | `if (ZSTDv07_isError(ofhSize)) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3781 | `ZSTDv07_decodeSeqHeaders` (src/legacy/zstd_v07.c:3436) | `if (ZSTDv07_isError(mlhSize)) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3782 | `ZSTDv07_decodeSequence` (src/legacy/zstd_v07.c:3495) | `if (MEM_32bits()) BITv07_reloadDStream(&(seqState->DStream));` | branch-specific rejection/error | [ ] |
| 3783 | `ZSTDv07_decodeSequence` (src/legacy/zstd_v07.c:3498) | `if (ofCode <= 1) {` | branch-specific rejection/error | [ ] |
| 3784 | `ZSTDv07_decodeSequence` (src/legacy/zstd_v07.c:3499) | `if ((llCode == 0) & (offset <= 1)) offset = 1-offset;` | branch-specific rejection/error | [ ] |
| 3785 | `ZSTDv07_decodeSequence` (src/legacy/zstd_v07.c:3502) | `if (offset != 1) seqState->prevOffset[2] = seqState->prevOffset[1];` | branch-specific rejection/error | [ ] |
| 3786 | `ZSTDv07_decodeSequence` (src/legacy/zstd_v07.c:3517) | `if (MEM_32bits() && (mlBits+llBits>24)) BITv07_reloadDStream(&(seqState->DStream));` | branch-specific rejection/error | [ ] |
| 3787 | `ZSTDv07_decodeSequence` (src/legacy/zstd_v07.c:3526) | `if (MEM_32bits()) BITv07_reloadDStream(&(seqState->DStream));     /* <= 18 bits */` | branch-specific rejection/error | [ ] |
| 3788 | `ZSTDv07_execSequence` (src/legacy/zstd_v07.c:3547) | `assert(oend >= op);` | `ERROR(dstSize_tooSmall)` | [ ] |
| 3789 | `ZSTDv07_execSequence` (src/legacy/zstd_v07.c:3548) | `if (sequence.litLength + WILDCOPY_OVERLENGTH > (size_t)(oend - op)) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [ ] |
| 3790 | `ZSTDv07_execSequence` (src/legacy/zstd_v07.c:3549) | `if (sequenceLength > (size_t)(oend - op)) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [ ] |
| 3791 | `ZSTDv07_execSequence` (src/legacy/zstd_v07.c:3550) | `assert(litLimit >= *litPtr);` | `ERROR(corruption_detected)` | [ ] |
| 3792 | `ZSTDv07_execSequence` (src/legacy/zstd_v07.c:3551) | `if (sequence.litLength > (size_t)(litLimit - *litPtr)) return ERROR(corruption_detected);;` | `ERROR(corruption_detected)` | [ ] |
| 3793 | `ZSTDv07_execSequence` (src/legacy/zstd_v07.c:3559) | `if (sequence.offset > (size_t)(oLitEnd - base)) {` | `ERROR(corruption_detected)` | [ ] |
| 3794 | `ZSTDv07_execSequence` (src/legacy/zstd_v07.c:3561) | `if (sequence.offset > (size_t)(oLitEnd - vBase)) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3795 | `ZSTDv07_execSequence` (src/legacy/zstd_v07.c:3563) | `if (match + sequence.matchLength <= dictEnd) {` | branch-specific rejection/error | [ ] |
| 3796 | `ZSTDv07_execSequence` (src/legacy/zstd_v07.c:3573) | `if (op > oend_w \|\| sequence.matchLength < MINMATCH) {` | branch-specific rejection/error | [ ] |
| 3797 | `ZSTDv07_execSequence` (src/legacy/zstd_v07.c:3581) | `if (sequence.offset < 8) {` | branch-specific rejection/error | [ ] |
| 3798 | `ZSTDv07_execSequence` (src/legacy/zstd_v07.c:3598) | `if (oMatchEnd > oend-(16-MINMATCH)) {` | branch-specific rejection/error | [ ] |
| 3799 | `ZSTDv07_execSequence` (src/legacy/zstd_v07.c:3599) | `if (op < oend_w) {` | branch-specific rejection/error | [ ] |
| 3800 | `ZSTDv07_decompressSequences` (src/legacy/zstd_v07.c:3644) | `if (ERR_isError(errorCode)) return ERROR(corruption_detected); }` | `ERROR(corruption_detected)` | [ ] |
| 3801 | `ZSTDv07_decompressSequences` (src/legacy/zstd_v07.c:3658) | `if (nbSeq) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3802 | `ZSTDv07_decompressSequences` (src/legacy/zstd_v07.c:3666) | `if (lastLLSize > (size_t)(oend-op)) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [ ] |
| 3803 | `ZSTDv07_decompressSequences` (src/legacy/zstd_v07.c:3667) | `if (lastLLSize > 0) {` | branch-specific rejection/error | [ ] |
| 3804 | `ZSTDv07_checkContinuity` (src/legacy/zstd_v07.c:3679) | `if (dst != dctx->previousDstEnd) {   /* not contiguous */` | branch-specific rejection/error | [ ] |
| 3805 | `ZSTDv07_decompressBlock_internal` (src/legacy/zstd_v07.c:3694) | `if (srcSize >= ZSTDv07_BLOCKSIZE_ABSOLUTEMAX) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 3806 | `ZSTDv07_generateNxBytes` (src/legacy/zstd_v07.c:3730) | `if (length > dstCapacity) return ERROR(dstSize_tooSmall);` | `ERROR(dstSize_tooSmall)` | [ ] |
| 3807 | `ZSTDv07_generateNxBytes` (src/legacy/zstd_v07.c:3731) | `if (length > 0) {` | branch-specific rejection/error | [ ] |
| 3808 | `ZSTDv07_decompressFrame` (src/legacy/zstd_v07.c:3752) | `if (srcSize < ZSTDv07_frameHeaderSize_min+ZSTDv07_blockHeaderSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 3809 | `ZSTDv07_decompressFrame` (src/legacy/zstd_v07.c:3757) | `if (srcSize < frameHeaderSize+ZSTDv07_blockHeaderSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 3810 | `ZSTDv07_decompressFrame` (src/legacy/zstd_v07.c:3758) | `if (ZSTDv07_decodeFrameHeader(dctx, src, frameHeaderSize)) return ERROR(corruption_detected);` | `ERROR(corruption_detected)` | [ ] |
| 3811 | `ZSTDv07_decompressFrame` (src/legacy/zstd_v07.c:3771) | `if (cBlockSize > remainingSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 3812 | `ZSTDv07_decompressFrame` (src/legacy/zstd_v07.c:3786) | `if (remainingSize) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 3813 | `ZSTDv07_decompressFrame` (src/legacy/zstd_v07.c:3790) | `return ERROR(GENERIC);   /* impossible */` | `ERROR(GENERIC)` | [ ] |
| 3814 | `ZSTDv07_decompressFrame` (src/legacy/zstd_v07.c:3795) | `if (dctx->fParams.checksumFlag) XXH64_update(&dctx->xxhState, op, decodedSize);` | branch-specific rejection/error | [ ] |
| 3815 | `ZSTDv07_decompress` (src/legacy/zstd_v07.c:3842) | `if (dctx==NULL) return ERROR(memory_allocation);` | `ERROR(memory_allocation)` | [ ] |
| 3816 | `ZSTDv07_findFrameSizeInfoLegacy` (src/legacy/zstd_v07.c:3867) | `if (srcSize < ZSTDv07_frameHeaderSize_min+ZSTDv07_blockHeaderSize) {` | `ERROR(srcSize_wrong)` | [ ] |
| 3817 | `ZSTDv07_findFrameSizeInfoLegacy` (src/legacy/zstd_v07.c:3868) | `ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, ERROR(srcSize_wrong));` | `ERROR(srcSize_wrong)` | [ ] |
| 3818 | `ZSTDv07_findFrameSizeInfoLegacy` (src/legacy/zstd_v07.c:3879) | `ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, ERROR(prefix_unknown));` | `ERROR(prefix_unknown)` | [ ] |
| 3819 | `ZSTDv07_findFrameSizeInfoLegacy` (src/legacy/zstd_v07.c:3882) | `if (srcSize < frameHeaderSize+ZSTDv07_blockHeaderSize) {` | `ERROR(srcSize_wrong)` | [ ] |
| 3820 | `ZSTDv07_findFrameSizeInfoLegacy` (src/legacy/zstd_v07.c:3883) | `ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, ERROR(srcSize_wrong));` | `ERROR(srcSize_wrong)` | [ ] |
| 3821 | `ZSTDv07_findFrameSizeInfoLegacy` (src/legacy/zstd_v07.c:3903) | `if (cBlockSize > remainingSize) {` | `ERROR(srcSize_wrong)` | [ ] |
| 3822 | `ZSTDv07_findFrameSizeInfoLegacy` (src/legacy/zstd_v07.c:3904) | `ZSTD_errorFrameSizeInfoLegacy(cSize, dBound, ERROR(srcSize_wrong));` | `ERROR(srcSize_wrong)` | [ ] |
| 3823 | `ZSTDv07_decompressContinue` (src/legacy/zstd_v07.c:3936) | `if (srcSize != dctx->expected) return ERROR(srcSize_wrong);` | `ERROR(srcSize_wrong)` | [ ] |
| 3824 | `ZSTDv07_decompressContinue` (src/legacy/zstd_v07.c:3942) | `if (srcSize != ZSTDv07_frameHeaderSize_min) return ERROR(srcSize_wrong);   /* impossible */` | `ERROR(srcSize_wrong)` | [ ] |
| 3825 | `ZSTDv07_decompressContinue` (src/legacy/zstd_v07.c:3950) | `if (ZSTDv07_isError(dctx->headerSize)) return dctx->headerSize;` | branch-specific rejection/error | [ ] |
| 3826 | `ZSTDv07_decompressContinue` (src/legacy/zstd_v07.c:3952) | `if (dctx->headerSize > ZSTDv07_frameHeaderSize_min) {` | branch-specific rejection/error | [ ] |
| 3827 | `ZSTDv07_decompressContinue` (src/legacy/zstd_v07.c:3973) | `if (dctx->fParams.checksumFlag) {` | branch-specific rejection/error | [ ] |
| 3828 | `ZSTDv07_decompressContinue` (src/legacy/zstd_v07.c:3978) | `if (check32 != h32) return ERROR(checksum_wrong);` | `ERROR(checksum_wrong)` | [ ] |
| 3829 | `ZSTDv07_decompressContinue` (src/legacy/zstd_v07.c:4000) | `return ERROR(GENERIC);   /* not yet handled */` | `ERROR(GENERIC)` | [ ] |
| 3830 | `ZSTDv07_decompressContinue` (src/legacy/zstd_v07.c:4006) | `return ERROR(GENERIC);   /* impossible */` | `ERROR(GENERIC)` | [ ] |
| 3831 | `ZSTDv07_decompressContinue` (src/legacy/zstd_v07.c:4012) | `if (dctx->fParams.checksumFlag) XXH64_update(&dctx->xxhState, dst, rSize);` | branch-specific rejection/error | [ ] |
| 3832 | `ZSTDv07_decompressContinue` (src/legacy/zstd_v07.c:4027) | `return ERROR(GENERIC);   /* impossible */` | `ERROR(GENERIC)` | [ ] |
| 3833 | `ZSTDv07_loadEntropy` (src/legacy/zstd_v07.c:4047) | `if (HUFv07_isError(hSize)) return ERROR(dictionary_corrupted);` | `ERROR(dictionary_corrupted)` | [ ] |
| 3834 | `ZSTDv07_loadEntropy` (src/legacy/zstd_v07.c:4054) | `if (FSEv07_isError(offcodeHeaderSize)) return ERROR(dictionary_corrupted);` | `ERROR(dictionary_corrupted)` | [ ] |
| 3835 | `ZSTDv07_loadEntropy` (src/legacy/zstd_v07.c:4055) | `if (offcodeLog > OffFSELog) return ERROR(dictionary_corrupted);` | `ERROR(dictionary_corrupted)` | [ ] |
| 3836 | `ZSTDv07_loadEntropy` (src/legacy/zstd_v07.c:4057) | `if (FSEv07_isError(errorCode)) return ERROR(dictionary_corrupted); }` | `ERROR(dictionary_corrupted)` | [ ] |
| 3837 | `ZSTDv07_loadEntropy` (src/legacy/zstd_v07.c:4064) | `if (FSEv07_isError(matchlengthHeaderSize)) return ERROR(dictionary_corrupted);` | `ERROR(dictionary_corrupted)` | [ ] |
| 3838 | `ZSTDv07_loadEntropy` (src/legacy/zstd_v07.c:4065) | `if (matchlengthLog > MLFSELog) return ERROR(dictionary_corrupted);` | `ERROR(dictionary_corrupted)` | [ ] |
| 3839 | `ZSTDv07_loadEntropy` (src/legacy/zstd_v07.c:4067) | `if (FSEv07_isError(errorCode)) return ERROR(dictionary_corrupted); }` | `ERROR(dictionary_corrupted)` | [ ] |
| 3840 | `ZSTDv07_loadEntropy` (src/legacy/zstd_v07.c:4074) | `if (FSEv07_isError(litlengthHeaderSize)) return ERROR(dictionary_corrupted);` | `ERROR(dictionary_corrupted)` | [ ] |
| 3841 | `ZSTDv07_loadEntropy` (src/legacy/zstd_v07.c:4075) | `if (litlengthLog > LLFSELog) return ERROR(dictionary_corrupted);` | `ERROR(dictionary_corrupted)` | [ ] |
| 3842 | `ZSTDv07_loadEntropy` (src/legacy/zstd_v07.c:4077) | `if (FSEv07_isError(errorCode)) return ERROR(dictionary_corrupted); }` | `ERROR(dictionary_corrupted)` | [ ] |
| 3843 | `ZSTDv07_loadEntropy` (src/legacy/zstd_v07.c:4081) | `if (dictPtr+12 > dictEnd) return ERROR(dictionary_corrupted);` | `ERROR(dictionary_corrupted)` | [ ] |
| 3844 | `ZSTDv07_loadEntropy` (src/legacy/zstd_v07.c:4082) | `dctx->rep[0] = MEM_readLE32(dictPtr+0); if (dctx->rep[0] == 0 \|\| dctx->rep[0] >= dictSize) return ERROR(dictionary_corrupted);` | `ERROR(dictionary_corrupted)` | [ ] |
| 3845 | `ZSTDv07_loadEntropy` (src/legacy/zstd_v07.c:4083) | `dctx->rep[1] = MEM_readLE32(dictPtr+4); if (dctx->rep[1] == 0 \|\| dctx->rep[1] >= dictSize) return ERROR(dictionary_corrupted);` | `ERROR(dictionary_corrupted)` | [ ] |
| 3846 | `ZSTDv07_loadEntropy` (src/legacy/zstd_v07.c:4084) | `dctx->rep[2] = MEM_readLE32(dictPtr+8); if (dctx->rep[2] == 0 \|\| dctx->rep[2] >= dictSize) return ERROR(dictionary_corrupted);` | `ERROR(dictionary_corrupted)` | [ ] |
| 3847 | `ZSTDv07_decompress_insertDictionary` (src/legacy/zstd_v07.c:4093) | `if (dictSize < 8) return ZSTDv07_refDictContent(dctx, dict, dictSize);` | branch-specific rejection/error | [ ] |
| 3848 | `ZSTDv07_decompress_insertDictionary` (src/legacy/zstd_v07.c:4104) | `if (ZSTDv07_isError(eSize)) return ERROR(dictionary_corrupted);` | `ERROR(dictionary_corrupted)` | [ ] |
| 3849 | `ZSTDv07_decompressBegin_usingDict` (src/legacy/zstd_v07.c:4121) | `if (ZSTDv07_isError(errorCode)) return ERROR(dictionary_corrupted);` | `ERROR(dictionary_corrupted)` | [ ] |
| 3850 | `ZSTDv07_createDDict_advanced` (src/legacy/zstd_v07.c:4140) | `return NULL;` | `NULL` | [ ] |
| 3851 | `ZSTDv07_createDDict_advanced` (src/legacy/zstd_v07.c:4150) | `return NULL;` | `NULL` | [ ] |
| 3852 | `ZSTDv07_createDDict_advanced` (src/legacy/zstd_v07.c:4159) | `return NULL;` | `NULL` | [ ] |
| 3853 | `ZBUFFv07_createDCtx_advanced` (src/legacy/zstd_v07.c:4293) | `return NULL;` | `NULL` | [ ] |
| 3854 | `ZBUFFv07_createDCtx_advanced` (src/legacy/zstd_v07.c:4296) | `if (zbd==NULL) return NULL;` | `NULL` | [ ] |
| 3855 | `ZBUFFv07_createDCtx_advanced` (src/legacy/zstd_v07.c:4300) | `if (zbd->zd == NULL) { ZBUFFv07_freeDCtx(zbd); return NULL; }` | `NULL` | [ ] |
| 3856 | `ZBUFFv07_freeDCtx` (src/legacy/zstd_v07.c:4307) | `if (zbd==NULL) return 0;   /* support free on null */` | branch-specific rejection/error | [ ] |
| 3857 | `ZBUFFv07_freeDCtx` (src/legacy/zstd_v07.c:4309) | `if (zbd->inBuff) zbd->customMem.customFree(zbd->customMem.opaque, zbd->inBuff);` | branch-specific rejection/error | [ ] |
| 3858 | `ZBUFFv07_freeDCtx` (src/legacy/zstd_v07.c:4310) | `if (zbd->outBuff) zbd->customMem.customFree(zbd->customMem.opaque, zbd->outBuff);` | branch-specific rejection/error | [ ] |
| 3859 | `ZBUFFv07_limitCopy` (src/legacy/zstd_v07.c:4335) | `if (length > 0) {` | branch-specific rejection/error | [ ] |
| 3860 | `ZBUFFv07_decompressContinue` (src/legacy/zstd_v07.c:4360) | `return ERROR(init_missing);` | `ERROR(init_missing)` | [ ] |
| 3861 | `ZBUFFv07_decompressContinue` (src/legacy/zstd_v07.c:4365) | `if (hSize != 0) {` | branch-specific rejection/error | [ ] |
| 3862 | `ZBUFFv07_decompressContinue` (src/legacy/zstd_v07.c:4367) | `if (toLoad > (size_t)(iend-ip)) {   /* not enough input to load full header */` | branch-specific rejection/error | [ ] |
| 3863 | `ZBUFFv07_decompressContinue` (src/legacy/zstd_v07.c:4368) | `if (ip != NULL)` | branch-specific rejection/error | [ ] |
| 3864 | `ZBUFFv07_decompressContinue` (src/legacy/zstd_v07.c:4382) | `if (h1Size < zbd->lhSize) {   /* long header */` | branch-specific rejection/error | [ ] |
| 3865 | `ZBUFFv07_decompressContinue` (src/legacy/zstd_v07.c:4393) | `if (zbd->inBuffSize < blockSize) {` | `ERROR(memory_allocation)` | [ ] |
| 3866 | `ZBUFFv07_decompressContinue` (src/legacy/zstd_v07.c:4397) | `if (zbd->inBuff == NULL) return ERROR(memory_allocation);` | `ERROR(memory_allocation)` | [ ] |
| 3867 | `ZBUFFv07_decompressContinue` (src/legacy/zstd_v07.c:4400) | `if (zbd->outBuffSize < neededOutSize) {` | `ERROR(memory_allocation)` | [ ] |
| 3868 | `ZBUFFv07_decompressContinue` (src/legacy/zstd_v07.c:4404) | `if (zbd->outBuff == NULL) return ERROR(memory_allocation);` | `ERROR(memory_allocation)` | [ ] |
| 3869 | `ZBUFFv07_decompressContinue` (src/legacy/zstd_v07.c:4411) | `if (neededInSize==0) {  /* end of frame */` | branch-specific rejection/error | [ ] |
| 3870 | `ZBUFFv07_decompressContinue` (src/legacy/zstd_v07.c:4416) | `if ((size_t)(iend-ip) >= neededInSize) {  /* decode directly from src */` | branch-specific rejection/error | [ ] |
| 3871 | `ZBUFFv07_decompressContinue` (src/legacy/zstd_v07.c:4436) | `if (toLoad > zbd->inBuffSize - zbd->inPos) return ERROR(corruption_detected);   /* should never happen */` | `ERROR(corruption_detected)` | [ ] |
| 3872 | `ZBUFFv07_decompressContinue` (src/legacy/zstd_v07.c:4440) | `if (loadedSize < toLoad) { notDone = 0; break; }   /* not enough input, wait for more */` | branch-specific rejection/error | [ ] |
| 3873 | `ZBUFFv07_decompressContinue` (src/legacy/zstd_v07.c:4449) | `if (!decodedSize && !isSkipFrame) { zbd->stage = ZBUFFds_read; break; }   /* this was just a header */` | branch-specific rejection/error | [ ] |
| 3874 | `ZBUFFv07_decompressContinue` (src/legacy/zstd_v07.c:4464) | `if (zbd->outStart + zbd->blockSize > zbd->outBuffSize)` | branch-specific rejection/error | [ ] |
| 3875 | `ZBUFFv07_decompressContinue` (src/legacy/zstd_v07.c:4472) | `default: return ERROR(GENERIC);   /* impossible */` | `ERROR(GENERIC)` | [ ] |
