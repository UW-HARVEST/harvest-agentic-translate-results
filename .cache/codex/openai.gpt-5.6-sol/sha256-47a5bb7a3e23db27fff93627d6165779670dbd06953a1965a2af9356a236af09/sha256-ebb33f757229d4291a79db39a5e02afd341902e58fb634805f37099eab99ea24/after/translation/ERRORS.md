# Error surface

Mechanically derived from runtime rejection macros/returns, null and range checks, assertions, and public boundary macros. Source locations are retained to prevent branch collapsing.

| # | function | trigger (exact invalid input/condition) | expected C result | source | [ ] |
|---:|----------|-----------------------------------------|-------------------|--------|:---:|
| 1 | `LZ4_memcpy_using_offset_base` | assertion `srcPtr + offset == dstPtr` is false | assertion failure when assertions are enabled | `lz4.c:498` | [x] |
| 2 | `LZ4_memcpy_using_offset` | assertion `dstEnd >= dstPtr + MINMATCH` is false | assertion failure when assertions are enabled | `lz4.c:539` | [x] |
| 3 | `LZ4_NbCommonBytes` | assertion `val != 0` is false | assertion failure when assertions are enabled | `lz4.c:581` | [x] |
| 4 | `LZ4_clearHash` | assertion `0` is false | assertion failure when assertions are enabled | `lz4.c:813` | [x] |
| 5 | `LZ4_putIndexOnHash` | assertion `0` is false | assertion failure when assertions are enabled | `lz4.c:826` | [x] |
| 6 | `LZ4_putIndexOnHash` | assertion `idx < 65536` is false | assertion failure when assertions are enabled | `lz4.c:828` | [x] |
| 7 | `LZ4_putPositionOnHash` | assertion `tableType == byPtr` is false | assertion failure when assertions are enabled | `lz4.c:837` | [x] |
| 8 | `LZ4_getIndexOnHash` | assertion `h < (1U << (LZ4_MEMORY_USAGE-2))` is false | assertion failure when assertions are enabled | `lz4.c:858` | [x] |
| 9 | `LZ4_getIndexOnHash` | assertion `h < (1U << (LZ4_MEMORY_USAGE-1))` is false | assertion failure when assertions are enabled | `lz4.c:863` | [x] |
| 10 | `LZ4_getIndexOnHash` | assertion `0` is false | assertion failure when assertions are enabled | `lz4.c:866` | [x] |
| 11 | `LZ4_getPositionOnHash` | assertion `tableType == byPtr` is false | assertion failure when assertions are enabled | `lz4.c:871` | [x] |
| 12 | `LZ4_prepareTable` | assertion `inputSize >= 0` is false | assertion failure when assertions are enabled | `lz4.c:892` | [x] |
| 13 | `LZ4_compress_generic_validated` | assertion `ip != NULL` is false | assertion failure when assertions are enabled | `lz4.c:980` | [x] |
| 14 | `LZ4_compress_generic_validated` | assertion `inputSize<LZ4_64Klimit` is false | assertion failure when assertions are enabled | `lz4.c:981` | [x] |
| 15 | `LZ4_compress_generic_validated` | assertion `dictDirective==noDict` is false | assertion failure when assertions are enabled | `lz4.c:982` | [x] |
| 16 | `LZ4_compress_generic_validated` | outputDirective == fillOutput && maxOutputSize < 1 | 0 | `lz4.c:985` | [x] |
| 17 | `LZ4_compress_generic_validated` | assertion `acceleration >= 1` is false | assertion failure when assertions are enabled | `lz4.c:986` | [x] |
| 18 | `LZ4_compress_generic_validated` | assertion `ip < mflimitPlusOne` is false | assertion failure when assertions are enabled | `lz4.c:1031` | [x] |
| 19 | `LZ4_compress_generic_validated` | assertion `matchIndex <= current` is false | assertion failure when assertions are enabled | `lz4.c:1049` | [x] |
| 20 | `LZ4_compress_generic_validated` | assertion `forwardIp - base < (ptrdiff_t)(2 GB - 1)` is false | assertion failure when assertions are enabled | `lz4.c:1050` | [x] |
| 21 | `LZ4_compress_generic_validated` | assertion `ip < mflimitPlusOne` is false | assertion failure when assertions are enabled | `lz4.c:1056` | [x] |
| 22 | `LZ4_compress_generic_validated` | assertion `tableType == byU32` is false | assertion failure when assertions are enabled | `lz4.c:1061` | [x] |
| 23 | `LZ4_compress_generic_validated` | assertion `startIndex - matchIndex >= MINMATCH` is false | assertion failure when assertions are enabled | `lz4.c:1073` | [x] |
| 24 | `LZ4_compress_generic_validated` | assertion `dictBase` is false | assertion failure when assertions are enabled | `lz4.c:1074` | [x] |
| 25 | `LZ4_compress_generic_validated` | assertion `matchIndex < current` is false | assertion failure when assertions are enabled | `lz4.c:1089` | [x] |
| 26 | `LZ4_compress_generic_validated` | assertion `(current - matchIndex) <= LZ4_DISTANCE_MAX` is false | assertion failure when assertions are enabled | `lz4.c:1094` | [x] |
| 27 | `LZ4_compress_generic_validated` | assertion `ip > anchor` is false | assertion failure when assertions are enabled | `lz4.c:1106` | [x] |
| 28 | `LZ4_compress_generic_validated` | if ((outputDirective == limitedOutput) && /* Check output buffer overflow */ | 0 | `lz4.c:1116` | [x] |
| 29 | `LZ4_compress_generic_validated` | assertion `offset <= LZ4_DISTANCE_MAX && offset > 0` is false | assertion failure when assertions are enabled | `lz4.c:1157` | [x] |
| 30 | `LZ4_compress_generic_validated` | assertion `ip-match <= LZ4_DISTANCE_MAX` is false | assertion failure when assertions are enabled | `lz4.c:1161` | [x] |
| 31 | `LZ4_compress_generic_validated` | assertion `dictEnd > match` is false | assertion failure when assertions are enabled | `lz4.c:1171` | [x] |
| 32 | `LZ4_compress_generic_validated` | assertion `newMatchCode < matchCode` is false | assertion failure when assertions are enabled | `lz4.c:1193` | [x] |
| 33 | `LZ4_compress_generic_validated` | assertion `outputDirective == limitedOutput` is false | assertion failure when assertions are enabled | `lz4.c:1209` | [x] |
| 34 | `LZ4_compress_generic_validated` | return 0; /* cannot compress within 'dst' budget. Stored indexes in hash table are nonetheless fine */ | 0 | `lz4.c:1210` | [x] |
| 35 | `LZ4_compress_generic_validated` | assertion `!(outputDirective == fillOutput && op + 1 + LASTLITERALS > olimit)` is false | assertion failure when assertions are enabled | `lz4.c:1228` | [x] |
| 36 | `LZ4_compress_generic_validated` | assertion `matchIndex < current` is false | assertion failure when assertions are enabled | `lz4.c:1258` | [x] |
| 37 | `LZ4_compress_generic_validated` | assertion `tableType == byU32` is false | assertion failure when assertions are enabled | `lz4.c:1262` | [x] |
| 38 | `LZ4_compress_generic_validated` | assertion `dictBase` is false | assertion failure when assertions are enabled | `lz4.c:1273` | [x] |
| 39 | `LZ4_compress_generic_validated` | assertion `matchIndex < current` is false | assertion failure when assertions are enabled | `lz4.c:1284` | [x] |
| 40 | `LZ4_compress_generic_validated` | assertion `olimit >= op` is false | assertion failure when assertions are enabled | `lz4.c:1309` | [x] |
| 41 | `LZ4_compress_generic_validated` | assertion `outputDirective == limitedOutput` is false | assertion failure when assertions are enabled | `lz4.c:1313` | [x] |
| 42 | `LZ4_compress_generic_validated` | return 0; /* cannot compress within 'dst' budget. Stored indexes in hash table are nonetheless fine */ | 0 | `lz4.c:1314` | [x] |
| 43 | `LZ4_compress_generic_validated` | assertion `result > 0` is false | assertion failure when assertions are enabled | `lz4.c:1335` | [x] |
| 44 | `LZ4_compress_generic` | (U32 | 0 | `lz4.c:1360` | [x] |
| 45 | `LZ4_compress_generic` | outputDirective != notLimited && dstCapacity <= 0 | 0 | `lz4.c:1362` | [x] |
| 46 | `LZ4_compress_generic` | assertion `outputDirective == notLimited || dstCapacity >= 1` is false | assertion failure when assertions are enabled | `lz4.c:1364` | [x] |
| 47 | `LZ4_compress_generic` | assertion `dst != NULL` is false | assertion failure when assertions are enabled | `lz4.c:1365` | [x] |
| 48 | `LZ4_compress_generic` | assertion `inputConsumed != NULL` is false | assertion failure when assertions are enabled | `lz4.c:1368` | [x] |
| 49 | `LZ4_compress_generic` | assertion `src != NULL` is false | assertion failure when assertions are enabled | `lz4.c:1373` | [x] |
| 50 | `LZ4_compress_fast_extState` | assertion `ctx != NULL` is false | assertion failure when assertions are enabled | `lz4.c:1385` | [x] |
| 51 | `LZ4_compress_fast_extState_fastReset` | assertion `ctx != NULL` is false | assertion failure when assertions are enabled | `lz4.c:1419` | [x] |
| 52 | `LZ4_compress_fast` | ctxPtr == NULL | 0 | `lz4.c:1458` | [x] |
| 53 | `LZ4_compress_destSize_extState_internal` | assertion `s != NULL` is false | assertion failure when assertions are enabled | `lz4.c:1484` | [x] |
| 54 | `LZ4_compress_destSize` | ctx == NULL | 0 | `lz4.c:1510` | [x] |
| 55 | `LZ4_createStream` | lz4s == NULL | NULL | `lz4.c:1536` | [x] |
| 56 | `LZ4_initStream` | buffer == NULL | NULL | `lz4.c:1555` | [x] |
| 57 | `LZ4_initStream` | size < sizeof(LZ4_stream_t | NULL | `lz4.c:1556` | [x] |
| 58 | `LZ4_initStream` | !LZ4_isAligned(buffer, LZ4_stream_t_alignment( | NULL | `lz4.c:1557` | [x] |
| 59 | `LZ4_freeStream` | !LZ4_stream | 0 | `lz4.c:1577` | [x] |
| 60 | `LZ4_renormDictT` | assertion `nextSize >= 0` is false | assertion failure when assertions are enabled | `lz4.c:1689` | [x] |
| 61 | `LZ4_saveDict` | assertion `dictSize == 0` is false | assertion failure when assertions are enabled | `lz4.c:1823` | [x] |
| 62 | `LZ4_saveDict` | assertion `dict->dictionary` is false | assertion failure when assertions are enabled | `lz4.c:1826` | [x] |
| 63 | `LZ4_decompress_unsafe_generic` | assertion `dictSize == 0` is false | assertion failure when assertions are enabled | `lz4.c:1886` | [x] |
| 64 | `LZ4_decompress_unsafe_generic` | (size_t | -1 | `lz4.c:1898` | [x] |
| 65 | `LZ4_decompress_unsafe_generic` | (size_t | -1 | `lz4.c:1921` | [x] |
| 66 | `read_variable_length` | assertion `ip != NULL` is false | assertion failure when assertions are enabled | `lz4.c:1983` | [x] |
| 67 | `read_variable_length` | assertion `*ip !=  NULL` is false | assertion failure when assertions are enabled | `lz4.c:1984` | [x] |
| 68 | `read_variable_length` | assertion `ilimit != NULL` is false | assertion failure when assertions are enabled | `lz4.c:1985` | [x] |
| 69 | `LZ4_decompress_generic` | (src == NULL | -1 | `lz4.c:2036` | [x] |
| 70 | `LZ4_decompress_generic` | assertion `lowPrefix <= op` is false | assertion failure when assertions are enabled | `lz4.c:2063` | [x] |
| 71 | `LZ4_decompress_generic` | partialDecoding | 0 | `lz4.c:2066` | [x] |
| 72 | `LZ4_decompress_generic` | unlikely(srcSize==0 | -1 | `lz4.c:2069` | [x] |
| 73 | `LZ4_decompress_generic` | assertion `oend - op >= FASTLOOP_SAFE_DISTANCE` is false | assertion failure when assertions are enabled | `lz4.c:2085` | [x] |
| 74 | `LZ4_decompress_generic` | assertion `ip < iend` is false | assertion failure when assertions are enabled | `lz4.c:2086` | [x] |
| 75 | `LZ4_decompress_generic` | assertion `match <= op` is false | assertion failure when assertions are enabled | `lz4.c:2121` | [x] |
| 76 | `LZ4_decompress_generic` | assertion `match >= lowPrefix` is false | assertion failure when assertions are enabled | `lz4.c:2150` | [x] |
| 77 | `LZ4_decompress_generic` | assertion `match <= op` is false | assertion failure when assertions are enabled | `lz4.c:2151` | [x] |
| 78 | `LZ4_decompress_generic` | assertion `op + 18 <= oend` is false | assertion failure when assertions are enabled | `lz4.c:2152` | [x] |
| 79 | `LZ4_decompress_generic` | assertion `dictEnd != NULL` is false | assertion failure when assertions are enabled | `lz4.c:2167` | [x] |
| 80 | `LZ4_decompress_generic` | assertion `(op <= oend) && (oend-op >= 32)` is false | assertion failure when assertions are enabled | `lz4.c:2201` | [x] |
| 81 | `LZ4_decompress_generic` | assertion `ip < iend` is false | assertion failure when assertions are enabled | `lz4.c:2216` | [x] |
| 82 | `LZ4_decompress_generic` | assertion `match <= op` is false | assertion failure when assertions are enabled | `lz4.c:2243` | [x] |
| 83 | `LZ4_decompress_generic` | assertion `op<=oend` is false | assertion failure when assertions are enabled | `lz4.c:2305` | [x] |
| 84 | `LZ4_decompress_generic` | assertion `dictEnd != NULL` is false | assertion failure when assertions are enabled | `lz4.c:2359` | [x] |
| 85 | `LZ4_decompress_generic` | assertion `match >= lowPrefix` is false | assertion failure when assertions are enabled | `lz4.c:2385` | [x] |
| 86 | `LZ4_decompress_generic` | assertion `op<=oend` is false | assertion failure when assertions are enabled | `lz4.c:2391` | [x] |
| 87 | `LZ4_freeStreamDecode` | LZ4_stream == NULL | 0 | `lz4.c:2577` | [x] |
| 88 | `LZ4_setStreamDecode` | assertion `dictionary != NULL` is false | assertion failure when assertions are enabled | `lz4.c:2594` | [x] |
| 89 | `LZ4_decoderRingBufferSize` | maxBlockSize < 0 | 0 | `lz4.c:2617` | [x] |
| 90 | `LZ4_decoderRingBufferSize` | maxBlockSize > LZ4_MAX_INPUT_SIZE | 0 | `lz4.c:2618` | [x] |
| 91 | `LZ4_decompress_safe_continue` | assertion `lz4sd->extDictSize == 0` is false | assertion failure when assertions are enabled | `lz4.c:2638` | [x] |
| 92 | `LZ4_decompress_fast_continue` | assertion `LZ4_streamDecode!=NULL), &LZ4_streamDecode->internal_donotuse` is false | assertion failure when assertions are enabled | `lz4.c:2675` | [x] |
| 93 | `LZ4_decompress_fast_continue` | assertion `originalSize >= 0` is false | assertion failure when assertions are enabled | `lz4.c:2679` | [x] |
| 94 | `LZ4_decompress_fast_continue` | assertion `lz4sd->extDictSize == 0` is false | assertion failure when assertions are enabled | `lz4.c:2683` | [x] |
| 95 | `LZ4_decompress_safe_usingDict` | assertion `dictSize >= 0` is false | assertion failure when assertions are enabled | `lz4.c:2727` | [x] |
| 96 | `LZ4_decompress_safe_usingDict` | assertion `dictSize >= 0` is false | assertion failure when assertions are enabled | `lz4.c:2730` | [x] |
| 97 | `LZ4_decompress_safe_partial_usingDict` | assertion `dictSize >= 0` is false | assertion failure when assertions are enabled | `lz4.c:2742` | [x] |
| 98 | `LZ4_decompress_safe_partial_usingDict` | assertion `dictSize >= 0` is false | assertion failure when assertions are enabled | `lz4.c:2745` | [x] |
| 99 | `LZ4_decompress_fast_usingDict` | assertion `dictSize >= 0` is false | assertion failure when assertions are enabled | `lz4.c:2755` | [x] |
| 100 | `LZ4_compressBound` | inputSize < 0 | 0 | `lz4.h:214` | [x] |
| 101 | `LZ4_compressBound` | inputSize > LZ4_MAX_INPUT_SIZE (0x7E000000) | 0 | `lz4.h:214` | [x] |
| 102 | `LZ4_decompress_safe_partial` | targetOutputSize > dstCapacity | negative decode error | `lz4.h:302` | [x] |
| 103 | `LZ4F_freeAndNullReadFile` | assertion `statePtr != NULL` is false | assertion failure when assertions are enabled | `lz4file.c:68` | [x] |
| 104 | `LZ4F_readOpen` | RETURN_ERROR( | if (fp == NULL \\|\\| lz4fRead == NULL) { RETURN_ERROR(parameter_null); } | `lz4file.c:79` | [x] |
| 105 | `LZ4F_readOpen` | if (fp == NULL \|\| lz4fRead == NULL) { | encoded LZ4F_ERROR_parameter_null | `lz4file.c:80` | [x] |
| 106 | `LZ4F_readOpen` | RETURN_ERROR( | if (*lz4fRead == NULL) { RETURN_ERROR(allocation_failed); } | `lz4file.c:84` | [x] |
| 107 | `LZ4F_readOpen` | if (*lz4fRead == NULL) { | encoded LZ4F_ERROR_allocation_failed | `lz4file.c:85` | [x] |
| 108 | `LZ4F_readOpen` | if (consumedSize != sizeof(buf)) { | encoded LZ4F_ERROR_io_read | `lz4file.c:98` | [x] |
| 109 | `LZ4F_readOpen` | branch at RETURN_ERROR(maxBlockSize_invalid); | encoded LZ4F_ERROR_maxBlockSize_invalid | `lz4file.c:124` | [x] |
| 110 | `LZ4F_readOpen` | if ((*lz4fRead)->srcBuf == NULL) { | encoded LZ4F_ERROR_allocation_failed | `lz4file.c:131` | [x] |
| 111 | `LZ4F_read` | RETURN_ERROR( | if (lz4fRead == NULL \\|\\| buf == NULL) RETURN_ERROR(parameter_null); while (next < size) { | `lz4file.c:145` | [x] |
| 112 | `LZ4F_read` | if (lz4fRead == NULL \|\| buf == NULL) | encoded LZ4F_ERROR_parameter_null | `lz4file.c:146` | [x] |
| 113 | `LZ4F_read` | } else if (ret == 0) { | encoded LZ4F_ERROR_io_read | `lz4file.c:162` | [x] |
| 114 | `LZ4F_readClose` | RETURN_ERROR( | if (lz4fRead == NULL) RETURN_ERROR(parameter_null); LZ4F_freeReadFile(lz4fRead); return LZ4F_OK_NoError; | `lz4file.c:185` | [x] |
| 115 | `LZ4F_readClose` | if (lz4fRead == NULL) | encoded LZ4F_ERROR_parameter_null | `lz4file.c:186` | [x] |
| 116 | `LZ4F_freeAndNullWriteFile` | assertion `statePtr != NULL` is false | assertion failure when assertions are enabled | `lz4file.c:212` | [x] |
| 117 | `LZ4F_writeOpen` | RETURN_ERROR( | if (fp == NULL \\|\\| lz4fWrite == NULL) RETURN_ERROR(parameter_null); *lz4fWrite = (LZ4_writeFile_t*)calloc(1, sizeof(LZ4_writeFile_t)); | `lz4file.c:222` | [x] |
| 118 | `LZ4F_writeOpen` | if (fp == NULL \|\| lz4fWrite == NULL) | encoded LZ4F_ERROR_parameter_null | `lz4file.c:223` | [x] |
| 119 | `LZ4F_writeOpen` | RETURN_ERROR( | if (*lz4fWrite == NULL) { RETURN_ERROR(allocation_failed); } if (prefsPtr != NULL) { | `lz4file.c:226` | [x] |
| 120 | `LZ4F_writeOpen` | if (*lz4fWrite == NULL) { | encoded LZ4F_ERROR_allocation_failed | `lz4file.c:227` | [x] |
| 121 | `LZ4F_writeOpen` | branch at RETURN_ERROR(maxBlockSize_invalid); | encoded LZ4F_ERROR_maxBlockSize_invalid | `lz4file.c:246` | [x] |
| 122 | `LZ4F_writeOpen` | if ((*lz4fWrite)->dstBuf == NULL) { | encoded LZ4F_ERROR_allocation_failed | `lz4file.c:256` | [x] |
| 123 | `LZ4F_writeOpen` | if (ret != fwrite(buf, 1, ret, fp)) { | encoded LZ4F_ERROR_io_write | `lz4file.c:273` | [x] |
| 124 | `LZ4F_write` | RETURN_ERROR( | if (lz4fWrite == NULL \\|\\| buf == NULL) RETURN_ERROR(parameter_null); while (remain) { if (remain > lz4fWrite->maxWriteSize) | `lz4file.c:288` | [x] |
| 125 | `LZ4F_write` | if (lz4fWrite == NULL \|\| buf == NULL) | encoded LZ4F_ERROR_parameter_null | `lz4file.c:289` | [x] |
| 126 | `LZ4F_write` | if (ret != fwrite(lz4fWrite->dstBuf, 1, ret, lz4fWrite->fp)) { | encoded LZ4F_ERROR_io_write | `lz4file.c:307` | [x] |
| 127 | `LZ4F_writeClose` | RETURN_ERROR( | if (lz4fWrite == NULL) { RETURN_ERROR(parameter_null); } | `lz4file.c:321` | [x] |
| 128 | `LZ4F_writeClose` | if (lz4fWrite == NULL) { | encoded LZ4F_ERROR_parameter_null | `lz4file.c:322` | [x] |
| 129 | `LZ4F_returnErrorCode` | if (c) { \ | encoded LZ4F_ERROR_e | `lz4frame.c:323` | [x] |
| 130 | `LZ4F_getBlockSize` | RETURN_ERROR( | if (blockSizeID < LZ4F_max64KB \\|\\| blockSizeID > LZ4F_max4MB) RETURN_ERROR(maxBlockSize_invalid); { int const blockSizeIdx = (int)blockSizeID - (int)LZ4F_max64KB; return blockSizes[blockSizeIdx]; | `lz4frame.c:338` | [x] |
| 131 | `LZ4F_getBlockSize` | if (blockSizeID == 0) blockSizeID = LZ4F_BLOCKSIZEID_DEFAULT; | encoded LZ4F_ERROR_maxBlockSize_invalid | `lz4frame.c:339` | [x] |
| 132 | `LZ4F_compressFrame_usingCDict` | dstCapacity < LZ4F_compressFrameBound(srcSize, &prefs) | encoded LZ4F_ERROR_dstMaxSize_tooSmall | `lz4frame.c:456` | [x] |
| 133 | `LZ4F_compressFrame_usingCDict` | assertion `dstEnd >= dstPtr` is false | assertion failure when assertions are enabled | `lz4frame.c:462` | [x] |
| 134 | `LZ4F_compressFrame_usingCDict` | assertion `dstEnd >= dstPtr` is false | assertion failure when assertions are enabled | `lz4frame.c:467` | [x] |
| 135 | `LZ4F_compressFrame_usingCDict` | assertion `dstEnd >= dstStart` is false | assertion failure when assertions are enabled | `lz4frame.c:472` | [x] |
| 136 | `LZ4F_createCDict_advanced` | !cdict | NULL | `lz4frame.c:544` | [x] |
| 137 | `LZ4F_createCompressionContext_advanced` | cctxPtr==NULL | NULL | `lz4frame.c:600` | [x] |
| 138 | `LZ4F_createCompressionContext` | assertion `LZ4F_compressionContextPtr != NULL` is false | assertion failure when assertions are enabled | `lz4frame.c:620` | [x] |
| 139 | `LZ4F_createCompressionContext` | LZ4F_compressionContextPtr == NULL | encoded LZ4F_ERROR_parameter_null | `lz4frame.c:622` | [x] |
| 140 | `LZ4F_createCompressionContext` | *LZ4F_compressionContextPtr==NULL | encoded LZ4F_ERROR_allocation_failed | `lz4frame.c:625` | [x] |
| 141 | `LZ4F_compressBegin_internal` | dstCapacity < maxFHSize | encoded LZ4F_ERROR_dstMaxSize_tooSmall | `lz4frame.c:700` | [x] |
| 142 | `LZ4F_compressBegin_internal` | cctx->lz4CtxPtr == NULL | encoded LZ4F_ERROR_allocation_failed | `lz4frame.c:722` | [x] |
| 143 | `LZ4F_compressBegin_internal` | cctx->tmpBuff == NULL | encoded LZ4F_ERROR_allocation_failed | `lz4frame.c:750` | [x] |
| 144 | `LZ4F_compressBegin_internal` | assertion `cdict == NULL` is false | assertion failure when assertions are enabled | `lz4frame.c:767` | [x] |
| 145 | `LZ4F_compressBegin_internal` | dictSize > INT_MAX | encoded LZ4F_ERROR_parameter_invalid | `lz4frame.c:768` | [x] |
| 146 | `LZ4F_compressBegin_internal` | assertion `cctx->lz4CtxType == ctxHC` is false | assertion failure when assertions are enabled | `lz4frame.c:774` | [x] |
| 147 | `LZ4F_makeBlock` | assertion `compress != NULL` is false | assertion failure when assertions are enabled | `lz4frame.c:891` | [x] |
| 148 | `LZ4F_compressUpdateImpl` | cctxPtr->cStage != 1 | encoded LZ4F_ERROR_compressionState_uninitialized | `lz4frame.c:1005` | [x] |
| 149 | `LZ4F_compressUpdateImpl` | if (dstCapacity < LZ4F_compressBound_internal(srcSize, &(cctxPtr->prefs), cctxPtr->tmpInSize)) | encoded LZ4F_ERROR_dstMaxSize_tooSmall | `lz4frame.c:1007` | [x] |
| 150 | `LZ4F_compressUpdateImpl` | if (blockCompression == LZ4B_UNCOMPRESSED && dstCapacity < srcSize) | encoded LZ4F_ERROR_dstMaxSize_tooSmall | `lz4frame.c:1010` | [x] |
| 151 | `LZ4F_compressUpdateImpl` | assertion `blockSize > cctxPtr->tmpInSize` is false | assertion failure when assertions are enabled | `lz4frame.c:1024` | [x] |
| 152 | `LZ4F_compressUpdateImpl` | assertion `blockCompression == LZ4B_COMPRESSED` is false | assertion failure when assertions are enabled | `lz4frame.c:1071` | [x] |
| 153 | `LZ4F_compressUpdateImpl` | assertion `0 <= realDictSize && realDictSize <= 64 KB` is false | assertion failure when assertions are enabled | `lz4frame.c:1076` | [x] |
| 154 | `LZ4F_compressUpdateImpl` | assertion `(cctxPtr->tmpIn + blockSize) <= (cctxPtr->tmpBuff + cctxPtr->maxBufferSize)` is false | assertion failure when assertions are enabled | `lz4frame.c:1089` | [x] |
| 155 | `LZ4F_flush` | cctxPtr->tmpInSize == 0 | 0 | `lz4frame.c:1167` | [x] |
| 156 | `LZ4F_flush` | cctxPtr->cStage != 1 | encoded LZ4F_ERROR_compressionState_uninitialized | `lz4frame.c:1168` | [x] |
| 157 | `LZ4F_flush` | dstCapacity < (cctxPtr->tmpInSize + BHSize + BFSize) | encoded LZ4F_ERROR_dstMaxSize_tooSmall | `lz4frame.c:1169` | [x] |
| 158 | `LZ4F_flush` | assertion `((void)"flush overflows dstBuffer!", (size_t)(dstPtr - dstStart) <= dstCapacity)` is false | assertion failure when assertions are enabled | `lz4frame.c:1181` | [x] |
| 159 | `LZ4F_compressEnd` | assertion `flushSize <= dstCapacity` is false | assertion failure when assertions are enabled | `lz4frame.c:1218` | [x] |
| 160 | `LZ4F_compressEnd` | dstCapacity < 4 | encoded LZ4F_ERROR_dstMaxSize_tooSmall | `lz4frame.c:1221` | [x] |
| 161 | `LZ4F_compressEnd` | dstCapacity < 8 | encoded LZ4F_ERROR_dstMaxSize_tooSmall | `lz4frame.c:1227` | [x] |
| 162 | `LZ4F_compressEnd` | if (cctxPtr->prefs.frameInfo.contentSize) { | encoded LZ4F_ERROR_frameSize_wrong | `lz4frame.c:1237` | [x] |
| 163 | `LZ4F_createDecompressionContext_advanced` | dctx == NULL | NULL | `lz4frame.c:1287` | [x] |
| 164 | `LZ4F_createDecompressionContext` | assertion `LZ4F_decompressionContextPtr != NULL` is false | assertion failure when assertions are enabled | `lz4frame.c:1303` | [x] |
| 165 | `LZ4F_createDecompressionContext` | LZ4F_decompressionContextPtr == NULL | encoded LZ4F_ERROR_parameter_null | `lz4frame.c:1304` | [x] |
| 166 | `LZ4F_createDecompressionContext` | RETURN_ERROR( | if (*LZ4F_decompressionContextPtr == NULL) { /* failed allocation */ RETURN_ERROR(allocation_failed); } return LZ4F_OK_NoError; | `lz4frame.c:1307` | [x] |
| 167 | `LZ4F_createDecompressionContext` | if (*LZ4F_decompressionContextPtr == NULL) { /* failed allocation */ | encoded LZ4F_ERROR_allocation_failed | `lz4frame.c:1308` | [x] |
| 168 | `LZ4F_decodeHeader` | srcSize < minFHSize | encoded LZ4F_ERROR_frameHeader_incomplete | `lz4frame.c:1354` | [x] |
| 169 | `LZ4F_decodeHeader` | if (LZ4F_readLE32(srcPtr) != LZ4F_MAGICNUMBER) { | encoded LZ4F_ERROR_frameType_unknown | `lz4frame.c:1374` | [x] |
| 170 | `LZ4F_decodeHeader` | ((FLG>>1 | encoded LZ4F_ERROR_reservedFlag_set | `lz4frame.c:1388` | [x] |
| 171 | `LZ4F_decodeHeader` | version != 1 | encoded LZ4F_ERROR_headerVersion_wrong | `lz4frame.c:1389` | [x] |
| 172 | `LZ4F_decodeHeader` | ((BD>>7 | encoded LZ4F_ERROR_reservedFlag_set | `lz4frame.c:1409` | [x] |
| 173 | `LZ4F_decodeHeader` | blockSizeID < 4 | encoded LZ4F_ERROR_maxBlockSize_invalid | `lz4frame.c:1410` | [x] |
| 174 | `LZ4F_decodeHeader` | ((BD>>0 | encoded LZ4F_ERROR_reservedFlag_set | `lz4frame.c:1411` | [x] |
| 175 | `LZ4F_decodeHeader` | assertion `frameHeaderSize > 5` is false | assertion failure when assertions are enabled | `lz4frame.c:1415` | [x] |
| 176 | `LZ4F_decodeHeader` | HC != srcPtr[frameHeaderSize-1] | encoded LZ4F_ERROR_headerChecksum_invalid | `lz4frame.c:1418` | [x] |
| 177 | `LZ4F_headerSize` | src == NULL | encoded LZ4F_ERROR_srcPtr_wrong | `lz4frame.c:1446` | [x] |
| 178 | `LZ4F_headerSize` | if (srcSize < LZ4F_MIN_SIZE_TO_KNOW_HEADER_LENGTH) | encoded LZ4F_ERROR_frameHeader_incomplete | `lz4frame.c:1450` | [x] |
| 179 | `LZ4F_headerSize` | if (LZ4F_readLE32(src) != LZ4F_MAGICNUMBER) | encoded LZ4F_ERROR_frameType_unknown | `lz4frame.c:1459` | [x] |
| 180 | `LZ4F_getFrameInfo` | if (dctx->dStage == dstage_storeFrameHeader) { | encoded LZ4F_ERROR_frameDecoding_alreadyStarted | `lz4frame.c:1501` | [x] |
| 181 | `LZ4F_getFrameInfo` | if (LZ4F_isError(hSize)) { *srcSizePtr=0; return hSize; } | encoded LZ4F_ERROR_frameHeader_incomplete | `lz4frame.c:1507` | [x] |
| 182 | `LZ4F_updateDict` | assertion `dstPtr != NULL` is false | assertion failure when assertions are enabled | `lz4frame.c:1531` | [x] |
| 183 | `LZ4F_updateDict` | assertion `dctx->dict != NULL` is false | assertion failure when assertions are enabled | `lz4frame.c:1533` | [x] |
| 184 | `LZ4F_updateDict` | assertion `dstPtr >= dstBufferStart` is false | assertion failure when assertions are enabled | `lz4frame.c:1540` | [x] |
| 185 | `LZ4F_updateDict` | assertion `dstSize < 64 KB` is false | assertion failure when assertions are enabled | `lz4frame.c:1547` | [x] |
| 186 | `LZ4F_updateDict` | assertion `dctx->tmpOutBuffer != NULL` is false | assertion failure when assertions are enabled | `lz4frame.c:1550` | [x] |
| 187 | `LZ4F_updateDict` | assertion `dctx->dict + dctx->dictSize == dctx->tmpOut + dctx->tmpOutStart` is false | assertion failure when assertions are enabled | `lz4frame.c:1554` | [x] |
| 188 | `LZ4F_decompress` | assertion `*dstSizePtr == 0` is false | assertion failure when assertions are enabled | `lz4frame.c:1632` | [x] |
| 189 | `LZ4F_decompress` | assertion `dctx != NULL` is false | assertion failure when assertions are enabled | `lz4frame.c:1637` | [x] |
| 190 | `LZ4F_decompress` | dctx->tmpIn == NULL | encoded LZ4F_ERROR_allocation_failed | `lz4frame.c:1686` | [x] |
| 191 | `LZ4F_decompress` | dctx->tmpOutBuffer== NULL | encoded LZ4F_ERROR_allocation_failed | `lz4frame.c:1689` | [x] |
| 192 | `LZ4F_decompress` | if (nextCBlockSize > dctx->maxBlockSize) { | encoded LZ4F_ERROR_maxBlockSize_invalid | `lz4frame.c:1738` | [x] |
| 193 | `LZ4F_decompress` | if (readCRC != calcCRC) { | encoded LZ4F_ERROR_blockChecksum_invalid | `lz4frame.c:1829` | [x] |
| 194 | `LZ4F_decompress` | assertion `dctx->tmpInTarget >= 4` is false | assertion failure when assertions are enabled | `lz4frame.c:1872` | [x] |
| 195 | `LZ4F_decompress` | assertion `selectedIn != NULL` is false | assertion failure when assertions are enabled | `lz4frame.c:1874` | [x] |
| 196 | `LZ4F_decompress` | readBlockCrc != calcBlockCrc | encoded LZ4F_ERROR_blockChecksum_invalid | `lz4frame.c:1878` | [x] |
| 197 | `LZ4F_decompress` | assertion `dstPtr != NULL` is false | assertion failure when assertions are enabled | `lz4frame.c:1895` | [x] |
| 198 | `LZ4F_decompress` | decodedSize < 0 | encoded LZ4F_ERROR_decompressionFailed | `lz4frame.c:1905` | [x] |
| 199 | `LZ4F_decompress` | decodedSize < 0 | encoded LZ4F_ERROR_decompressionFailed | `lz4frame.c:1950` | [x] |
| 200 | `LZ4F_decompress` | dctx->frameRemainingSize | encoded LZ4F_ERROR_frameSize_wrong | `lz4frame.c:1984` | [x] |
| 201 | `LZ4F_decompress` | readCRC != resultCRC | encoded LZ4F_ERROR_contentChecksum_invalid | `lz4frame.c:2021` | [x] |
| 202 | `LZ4F_decompress` | assertion `dctx->tmpOutBuffer != NULL` is false | assertion failure when assertions are enabled | `lz4frame.c:2095` | [x] |
| 203 | `LZ4F_createCompressionContext` | version != LZ4F_VERSION (100) | implementation accepts version; output otherwise follows C exactly | `lz4frame.h:266` | [x] |
| 204 | `LZ4F_createDecompressionContext` | version != LZ4F_VERSION (100) | implementation accepts version; output otherwise follows C exactly | `lz4frame.h:386` | [x] |
| 205 | `LZ4F_headerSize` | srcSize < LZ4F_MIN_SIZE_TO_KNOW_HEADER_LENGTH (5) | encoded LZ4F_ERROR_frameHeader_incomplete | `lz4frame.h:408` | [x] |
| 206 | `LZ4HC_NbCommonBytes32` | assertion `val != 0` is false | assertion failure when assertions are enabled | `lz4hc.c:170` | [x] |
| 207 | `LZ4HC_countBack` | assertion `min <= 0` is false | assertion failure when assertions are enabled | `lz4hc.c:210` | [x] |
| 208 | `LZ4HC_countBack` | assertion `ip >= iMin); assert((size_t)(ip-iMin) < (1U<<31)` is false | assertion failure when assertions are enabled | `lz4hc.c:211` | [x] |
| 209 | `LZ4HC_countBack` | assertion `match >= mMin); assert((size_t)(match - mMin) < (1U<<31)` is false | assertion failure when assertions are enabled | `lz4hc.c:212` | [x] |
| 210 | `LZ4HC_init_internal` | assertion `newStartingOffset >= bufferSize` is false | assertion failure when assertions are enabled | `lz4hc.c:247` | [x] |
| 211 | `LZ4HC_encodeSequence` | assertion `offset <= LZ4_DISTANCE_MAX ` is false | assertion failure when assertions are enabled | `lz4hc.c:324` | [x] |
| 212 | `LZ4HC_encodeSequence` | assertion `offset > 0` is false | assertion failure when assertions are enabled | `lz4hc.c:325` | [x] |
| 213 | `LZ4HC_encodeSequence` | assertion `matchLength >= MINMATCH` is false | assertion failure when assertions are enabled | `lz4hc.c:329` | [x] |
| 214 | `LZ4HC_searchExtDict` | assertion `lDictEndIndex <= 1 GB` is false | assertion failure when assertions are enabled | `lz4hc.c:372` | [x] |
| 215 | `LZ4MID_searchExtDict` | assertion `lDictEndIndex <= 1 GB` is false | assertion failure when assertions are enabled | `lz4hc.c:432` | [x] |
| 216 | `LZ4MID_fillHTable` | assertion `dict == cctx->prefixStart` is false | assertion failure when assertions are enabled | `lz4hc.c:496` | [x] |
| 217 | `select_searchDict_function` | dictCtx == NULL | NULL | `lz4hc.c:516` | [x] |
| 218 | `LZ4MID_compress` | assertion `*srcSizePtr >= 0` is false | assertion failure when assertions are enabled | `lz4hc.c:556` | [x] |
| 219 | `LZ4MID_compress` | assertion `src != NULL` is false | assertion failure when assertions are enabled | `lz4hc.c:557` | [x] |
| 220 | `LZ4MID_compress` | assertion `dst != NULL` is false | assertion failure when assertions are enabled | `lz4hc.c:558` | [x] |
| 221 | `LZ4MID_compress` | *srcSizePtr < 0 | 0 | `lz4hc.c:559` | [x] |
| 222 | `LZ4MID_compress` | maxOutputSize < 0 | 0 | `lz4hc.c:560` | [x] |
| 223 | `LZ4MID_compress` | return 0 | if (*srcSizePtr > LZ4_MAX_INPUT_SIZE) { /* forbidden: no input is allowed to be that large */ return 0; } | `lz4hc.c:561` | [x] |
| 224 | `LZ4MID_compress` | assertion `h8 < LZ4MID_HASHTABLESIZE` is false | assertion failure when assertions are enabled | `lz4hc.c:575` | [x] |
| 225 | `LZ4MID_compress` | assertion `pos8 < ipIndex` is false | assertion failure when assertions are enabled | `lz4hc.c:576` | [x] |
| 226 | `LZ4MID_compress` | assertion `matchPtr < ip` is false | assertion failure when assertions are enabled | `lz4hc.c:582` | [x] |
| 227 | `LZ4MID_compress` | assertion `h4 < LZ4MID_HASHTABLESIZE` is false | assertion failure when assertions are enabled | `lz4hc.c:606` | [x] |
| 228 | `LZ4MID_compress` | assertion `pos4 < ipIndex` is false | assertion failure when assertions are enabled | `lz4hc.c:607` | [x] |
| 229 | `LZ4MID_compress` | assertion `matchPtr < ip` is false | assertion failure when assertions are enabled | `lz4hc.c:614` | [x] |
| 230 | `LZ4MID_compress` | assertion `matchPtr >= prefixPtr` is false | assertion failure when assertions are enabled | `lz4hc.c:615` | [x] |
| 231 | `LZ4MID_compress` | assertion `dMatch.back == 0` is false | assertion failure when assertions are enabled | `lz4hc.c:660` | [x] |
| 232 | `LZ4MID_compress` | limit == limitedOutput | 0 | `lz4hc.c:714` | [x] |
| 233 | `LZ4MID_compress` | assertion `lastRunSize <= (size_t)(oend - op)` is false | assertion failure when assertions are enabled | `lz4hc.c:732` | [x] |
| 234 | `LZ4MID_compress` | assertion `ip >= (const BYTE*)src` is false | assertion failure when assertions are enabled | `lz4hc.c:739` | [x] |
| 235 | `LZ4MID_compress` | assertion `ip <= iend` is false | assertion failure when assertions are enabled | `lz4hc.c:740` | [x] |
| 236 | `LZ4MID_compress` | assertion `(char*)op >= dst` is false | assertion failure when assertions are enabled | `lz4hc.c:742` | [x] |
| 237 | `LZ4MID_compress` | assertion `op <= oend` is false | assertion failure when assertions are enabled | `lz4hc.c:743` | [x] |
| 238 | `LZ4MID_compress` | assertion `(char*)op - dst < INT_MAX` is false | assertion failure when assertions are enabled | `lz4hc.c:744` | [x] |
| 239 | `LZ4MID_compress` | assertion `maxMlSize < INT_MAX` is false | assertion failure when assertions are enabled | `lz4hc.c:760` | [x] |
| 240 | `LZ4HC_Insert` | assertion `ip >= prefixPtr` is false | assertion failure when assertions are enabled | `lz4hc.c:789` | [x] |
| 241 | `LZ4HC_Insert` | assertion `target >= prefixIdx` is false | assertion failure when assertions are enabled | `lz4hc.c:790` | [x] |
| 242 | `LZ4HC_InsertAndGetWiderMatch` | assertion `matchIndex < ipIndex` is false | assertion failure when assertions are enabled | `lz4hc.c:925` | [x] |
| 243 | `LZ4HC_InsertAndGetWiderMatch` | assertion `matchPtr < ip` is false | assertion failure when assertions are enabled | `lz4hc.c:931` | [x] |
| 244 | `LZ4HC_InsertAndGetWiderMatch` | assertion `longest >= 1` is false | assertion failure when assertions are enabled | `lz4hc.c:932` | [x] |
| 245 | `LZ4HC_InsertAndGetWiderMatch` | assertion `matchIndex >= dictIdx` is false | assertion failure when assertions are enabled | `lz4hc.c:946` | [x] |
| 246 | `LZ4HC_InsertAndGetWiderMatch` | assertion `lookBackLength==0` is false | assertion failure when assertions are enabled | `lz4hc.c:965` | [x] |
| 247 | `LZ4HC_InsertAndGetWiderMatch` | assertion `matchCandidateIdx - backLength >= lowestMatchIndex` is false | assertion failure when assertions are enabled | `lz4hc.c:1022` | [x] |
| 248 | `LZ4HC_InsertAndGetWiderMatch` | assertion `newMatchIndex >= prefixIdx - 3 && newMatchIndex < prefixIdx && !extDict` is false | assertion failure when assertions are enabled | `lz4hc.c:1032` | [x] |
| 249 | `LZ4HC_InsertAndGetWiderMatch` | assertion `newMatchIndex >= prefixIdx - 3 && newMatchIndex < prefixIdx && !extDict` is false | assertion failure when assertions are enabled | `lz4hc.c:1038` | [x] |
| 250 | `LZ4HC_InsertAndGetWiderMatch` | assertion `prefixPtr - prefixIdx + matchIndex != ip` is false | assertion failure when assertions are enabled | `lz4hc.c:1045` | [x] |
| 251 | `LZ4HC_InsertAndGetWiderMatch` | assertion `maxML < 2 GB` is false | assertion failure when assertions are enabled | `lz4hc.c:1047` | [x] |
| 252 | `LZ4HC_InsertAndGetWiderMatch` | assertion `sBack == 0` is false | assertion failure when assertions are enabled | `lz4hc.c:1050` | [x] |
| 253 | `LZ4HC_InsertAndGetWiderMatch` | assertion `dictEndOffset <= 1 GB` is false | assertion failure when assertions are enabled | `lz4hc.c:1071` | [x] |
| 254 | `LZ4HC_InsertAndGetWiderMatch` | assertion `longest >= 0` is false | assertion failure when assertions are enabled | `lz4hc.c:1098` | [x] |
| 255 | `LZ4HC_compress_hashChain` | limit == limitedOutput | 0 | `lz4hc.c:1315` | [x] |
| 256 | `LZ4HC_compress_hashChain` | assertion `maxMlSize < INT_MAX); assert(m1.len >= 0` is false | assertion failure when assertions are enabled | `lz4hc.c:1353` | [x] |
| 257 | `LZ4HC_compress_generic_internal` | limit == fillOutput && dstCapacity < 1 | 0 | `lz4hc.c:1388` | [x] |
| 258 | `LZ4HC_compress_generic_internal` | (U32 | 0 | `lz4hc.c:1389` | [x] |
| 259 | `LZ4HC_compress_generic_internal` | assertion `cParam.strat == lz4opt` is false | assertion failure when assertions are enabled | `lz4hc.c:1405` | [x] |
| 260 | `LZ4HC_compress_generic_noDictCtx` | assertion `ctx->dictCtx == NULL` is false | assertion failure when assertions are enabled | `lz4hc.c:1430` | [x] |
| 261 | `LZ4HC_compress_generic_dictCtx` | assertion `ctx->dictCtx != NULL` is false | assertion failure when assertions are enabled | `lz4hc.c:1453` | [x] |
| 262 | `LZ4_compress_HC_extStateHC_fastReset` | !LZ4_isAligned(state, LZ4_streamHC_t_alignment( | 0 | `lz4hc.c:1503` | [x] |
| 263 | `LZ4_compress_HC_extStateHC` | ctx==NULL | 0 | `lz4hc.c:1515` | [x] |
| 264 | `LZ4_compress_HC` | statePtr==NULL | 0 | `lz4hc.c:1524` | [x] |
| 265 | `LZ4_compress_HC_destSize` | ctx==NULL | 0 | `lz4hc.c:1541` | [x] |
| 266 | `LZ4_createStreamHC` | state == NULL | NULL | `lz4hc.c:1558` | [x] |
| 267 | `LZ4_freeStreamHC` | !LZ4_streamHCPtr | 0 | `lz4hc.c:1566` | [x] |
| 268 | `LZ4_initStreamHC` | buffer == NULL | NULL | `lz4hc.c:1578` | [x] |
| 269 | `LZ4_initStreamHC` | size < sizeof(LZ4_streamHC_t | NULL | `lz4hc.c:1579` | [x] |
| 270 | `LZ4_initStreamHC` | !LZ4_isAligned(buffer, LZ4_streamHC_t_alignment( | NULL | `lz4hc.c:1580` | [x] |
| 271 | `LZ4_resetStreamHC_fast` | assertion `s->end >= s->prefixStart` is false | assertion failure when assertions are enabled | `lz4hc.c:1602` | [x] |
| 272 | `LZ4_loadDictHC` | assertion `dictSize >= 0` is false | assertion failure when assertions are enabled | `lz4hc.c:1632` | [x] |
| 273 | `LZ4_loadDictHC` | assertion `LZ4_streamHCPtr != NULL` is false | assertion failure when assertions are enabled | `lz4hc.c:1633` | [x] |
| 274 | `LZ4_compressHC_continue_generic` | assertion `ctxPtr != NULL` is false | assertion failure when assertions are enabled | `lz4hc.c:1689` | [x] |
| 275 | `LZ4_saveDictHC` | assertion `prefixSize >= 0` is false | assertion failure when assertions are enabled | `lz4hc.c:1747` | [x] |
| 276 | `LZ4_saveDictHC` | assertion `dictSize == 0` is false | assertion failure when assertions are enabled | `lz4hc.c:1751` | [x] |
| 277 | `LZ4HC_literalsPrice` | assertion `litlen >= 0` is false | assertion failure when assertions are enabled | `lz4hc.c:1781` | [x] |
| 278 | `LZ4HC_sequencePrice` | assertion `litlen >= 0` is false | assertion failure when assertions are enabled | `lz4hc.c:1791` | [x] |
| 279 | `LZ4HC_sequencePrice` | assertion `mlen >= MINMATCH` is false | assertion failure when assertions are enabled | `lz4hc.c:1792` | [x] |
| 280 | `LZ4HC_FindLongerMatch` | assertion `md.back == 0` is false | assertion failure when assertions are enabled | `lz4hc.c:1814` | [x] |
| 281 | `LZ4HC_compress_optimal` | assertion `matchML < LZ4_OPT_NUM` is false | assertion failure when assertions are enabled | `lz4hc.c:1899` | [x] |
| 282 | `LZ4HC_compress_optimal` | assertion `cur + newMatch.len < LZ4_OPT_NUM` is false | assertion failure when assertions are enabled | `lz4hc.c:1975` | [x] |
| 283 | `LZ4HC_compress_optimal` | assertion `(U32)favorDecSpeed <= 1` is false | assertion failure when assertions are enabled | `lz4hc.c:1992` | [x] |
| 284 | `LZ4HC_compress_optimal` | assertion `pos < LZ4_OPT_NUM` is false | assertion failure when assertions are enabled | `lz4hc.c:1997` | [x] |
| 285 | `LZ4HC_compress_optimal` | assertion `last_match_pos < LZ4_OPT_NUM + TRAILING_LITERALS` is false | assertion failure when assertions are enabled | `lz4hc.c:2017` | [x] |
| 286 | `LZ4HC_compress_optimal` | assertion `cur < LZ4_OPT_NUM` is false | assertion failure when assertions are enabled | `lz4hc.c:2023` | [x] |
| 287 | `LZ4HC_compress_optimal` | assertion `last_match_pos >= 1` is false | assertion failure when assertions are enabled | `lz4hc.c:2024` | [x] |
| 288 | `LZ4HC_compress_optimal` | assertion `next_matchLength > 0` is false | assertion failure when assertions are enabled | `lz4hc.c:2038` | [x] |
| 289 | `LZ4HC_compress_optimal` | assertion `ml >= MINMATCH` is false | assertion failure when assertions are enabled | `lz4hc.c:2049` | [x] |
| 290 | `LZ4HC_compress_optimal` | assertion `(offset >= 1) && (offset <= LZ4_DISTANCE_MAX)` is false | assertion failure when assertions are enabled | `lz4hc.c:2050` | [x] |
| 291 | `LZ4HC_compress_optimal` | assertion `maxMlSize < INT_MAX); assert(ovml >= 0` is false | assertion failure when assertions are enabled | `lz4hc.c:2108` | [x] |
| 292 | `LZ4_resetStreamStateHC` | return 0 | if (hc4 == NULL) return 1; /* init failed */ LZ4HC_init_internal (&hc4->internal_donotuse, (const BYTE*)inputBuffer); return 0; } | `lz4hc.c:2153` | [x] |
| 293 | `LZ4_createHC` | hc4 == NULL | NULL | `lz4hc.c:2162` | [x] |
| 294 | `LZ4_freeHC` | !LZ4HC_Data | 0 | `lz4hc.c:2169` | [x] |
| 295 | `XXH32_finalize` | assertion `0` is false | assertion failure when assertions are enabled | `xxhash.c:346` | [x] |
| 296 | `XXH64_finalize` | assertion `0` is false | assertion failure when assertions are enabled | `xxhash.c:806` | [x] |
