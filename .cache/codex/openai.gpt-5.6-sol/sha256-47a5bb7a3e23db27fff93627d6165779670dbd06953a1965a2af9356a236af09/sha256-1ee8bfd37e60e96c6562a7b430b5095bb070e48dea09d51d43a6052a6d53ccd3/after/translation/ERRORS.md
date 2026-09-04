# Error surface

Rows come from explicit public/propagated-core rejection branches in the C sources plus public-header macro boundaries. Purely internal invariant assertions with no external invalid-input construction are excluded.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---:|----------|---------------------------------------------|-------------------|
| 1 | `LZ4_compress_fast_extState` | lz4.c:1385 — `assertion violated: ctx != NULL` | `assertion failure` [x] |
| 2 | `LZ4_compress_fast_extState_fastReset` | lz4.c:1419 — `assertion violated: ctx != NULL` | `assertion failure` [x] |
| 3 | `LZ4_createStream` | lz4.c:1536 — `lz4s == NULL` | `NULL` [x] |
| 4 | `LZ4_initStream` | lz4.c:1555 — `buffer == NULL` | `NULL` [x] |
| 5 | `LZ4_initStream` | lz4.c:1556 — `size < sizeof(LZ4_stream_t` | `NULL` [x] |
| 6 | `LZ4_initStream` | lz4.c:1557 — `!LZ4_isAligned(buffer, LZ4_stream_t_alignment(` | `NULL` [x] |
| 7 | `LZ4_decompress_unsafe_generic` | lz4.c:1898 — `(size_t` | `-1` [x] |
| 8 | `LZ4_decompress_unsafe_generic` | lz4.c:1921 — `(size_t` | `-1` [x] |
| 9 | `LZ4_decompress_generic` | lz4.c:2036 — `(src == NULL` | `-1` [x] |
| 10 | `LZ4_decompress_generic` | lz4.c:2069 — `unlikely(srcSize==0` | `-1` [x] |
| 11 | `LZ4_setStreamDecode` | lz4.c:2594 — `assertion violated: dictionary != NULL` | `assertion failure` [x] |
| 12 | `LZ4_decompress_fast_continue` | lz4.c:2675 — `assertion violated: LZ4_streamDecode!=NULL), &LZ4_streamDecode->internal_donotuse` | `assertion failure` [x] |
| 13 | `LZ4_decompress_fast_continue` | lz4.c:2679 — `assertion violated: originalSize >= 0` | `assertion failure` [x] |
| 14 | `LZ4_decompress_safe_usingDict` | lz4.c:2727 — `assertion violated: dictSize >= 0` | `assertion failure` [x] |
| 15 | `LZ4_decompress_safe_usingDict` | lz4.c:2730 — `assertion violated: dictSize >= 0` | `assertion failure` [x] |
| 16 | `LZ4_decompress_safe_partial_usingDict` | lz4.c:2742 — `assertion violated: dictSize >= 0` | `assertion failure` [x] |
| 17 | `LZ4_decompress_safe_partial_usingDict` | lz4.c:2745 — `assertion violated: dictSize >= 0` | `assertion failure` [x] |
| 18 | `LZ4_decompress_fast_usingDict` | lz4.c:2755 — `assertion violated: dictSize >= 0` | `assertion failure` [x] |
| 19 | `LZ4HC_searchExtDict` | lz4hc.c:372 — `assertion violated: lDictEndIndex <= 1 GB` | `assertion failure` [x] |
| 20 | `LZ4_createStreamHC` | lz4hc.c:1558 — `state == NULL` | `NULL` [x] |
| 21 | `LZ4_initStreamHC` | lz4hc.c:1578 — `buffer == NULL` | `NULL` [x] |
| 22 | `LZ4_initStreamHC` | lz4hc.c:1579 — `size < sizeof(LZ4_streamHC_t` | `NULL` [x] |
| 23 | `LZ4_initStreamHC` | lz4hc.c:1580 — `!LZ4_isAligned(buffer, LZ4_streamHC_t_alignment(` | `NULL` [x] |
| 24 | `LZ4_resetStreamHC_fast` | lz4hc.c:1602 — `assertion violated: s->end >= s->prefixStart` | `assertion failure` [x] |
| 25 | `LZ4_loadDictHC` | lz4hc.c:1632 — `assertion violated: dictSize >= 0` | `assertion failure` [x] |
| 26 | `LZ4_loadDictHC` | lz4hc.c:1633 — `assertion violated: LZ4_streamHCPtr != NULL` | `assertion failure` [x] |
| 27 | `LZ4_saveDictHC` | lz4hc.c:1747 — `assertion violated: prefixSize >= 0` | `assertion failure` [x] |
| 28 | `LZ4_createHC` | lz4hc.c:2162 — `hc4 == NULL` | `NULL` [x] |
| 29 | `LZ4F_getBlockSize` | lz4frame.c:339 — `if (blockSizeID < LZ4F_max64KB \|\| blockSizeID > LZ4F_max4MB)` | `LZ4F_ERROR_maxBlockSize_invalid` [x] |
| 30 | `LZ4F_compressFrame_usingCDict` | lz4frame.c:456 — `dstCapacity < LZ4F_compressFrameBound(srcSize, &prefs)` | `LZ4F_ERROR_dstMaxSize_tooSmall` [x] |
| 31 | `LZ4F_compressFrame_usingCDict` | lz4frame.c:462 — `assertion violated: dstEnd >= dstPtr` | `assertion failure` [x] |
| 32 | `LZ4F_compressFrame_usingCDict` | lz4frame.c:467 — `assertion violated: dstEnd >= dstPtr` | `assertion failure` [x] |
| 33 | `LZ4F_compressFrame_usingCDict` | lz4frame.c:472 — `assertion violated: dstEnd >= dstStart` | `assertion failure` [x] |
| 34 | `LZ4F_createCDict_advanced` | lz4frame.c:544 — `!cdict` | `NULL` [x] |
| 35 | `LZ4F_createCompressionContext_advanced` | lz4frame.c:600 — `cctxPtr==NULL` | `NULL` [x] |
| 36 | `LZ4F_createCompressionContext` | lz4frame.c:620 — `assertion violated: LZ4F_compressionContextPtr != NULL` | `assertion failure` [x] |
| 37 | `LZ4F_createCompressionContext` | lz4frame.c:622 — `LZ4F_compressionContextPtr == NULL` | `LZ4F_ERROR_parameter_null` [x] |
| 38 | `LZ4F_createCompressionContext` | lz4frame.c:625 — `*LZ4F_compressionContextPtr==NULL` | `LZ4F_ERROR_allocation_failed` [x] |
| 39 | `LZ4F_compressBegin_internal` | lz4frame.c:700 — `dstCapacity < maxFHSize` | `LZ4F_ERROR_dstMaxSize_tooSmall` [x] |
| 40 | `LZ4F_compressBegin_internal` | lz4frame.c:722 — `cctx->lz4CtxPtr == NULL` | `LZ4F_ERROR_allocation_failed` [x] |
| 41 | `LZ4F_compressBegin_internal` | lz4frame.c:750 — `cctx->tmpBuff == NULL` | `LZ4F_ERROR_allocation_failed` [x] |
| 42 | `LZ4F_compressBegin_internal` | lz4frame.c:767 — `assertion violated: cdict == NULL` | `assertion failure` [x] |
| 43 | `LZ4F_compressBegin_internal` | lz4frame.c:768 — `dictSize > INT_MAX` | `LZ4F_ERROR_parameter_invalid` [x] |
| 44 | `LZ4F_compressUpdateImpl` | lz4frame.c:1005 — `cctxPtr->cStage != 1` | `LZ4F_ERROR_compressionState_uninitialized` [x] |
| 45 | `LZ4F_compressUpdateImpl` | lz4frame.c:1007 — `if (dstCapacity < LZ4F_compressBound_internal(srcSize, &(cctxPtr->prefs), cctxPtr->tmpInSize))` | `LZ4F_ERROR_dstMaxSize_tooSmall` [x] |
| 46 | `LZ4F_compressUpdateImpl` | lz4frame.c:1010 — `if (blockCompression == LZ4B_UNCOMPRESSED && dstCapacity < srcSize)` | `LZ4F_ERROR_dstMaxSize_tooSmall` [x] |
| 47 | `LZ4F_flush` | lz4frame.c:1168 — `cctxPtr->cStage != 1` | `LZ4F_ERROR_compressionState_uninitialized` [x] |
| 48 | `LZ4F_flush` | lz4frame.c:1169 — `dstCapacity < (cctxPtr->tmpInSize + BHSize + BFSize)` | `LZ4F_ERROR_dstMaxSize_tooSmall` [x] |
| 49 | `LZ4F_flush` | lz4frame.c:1181 — `assertion violated: ((void)"flush overflows dstBuffer!", (size_t)(dstPtr - dstStart) <= dstCapacity)` | `assertion failure` [x] |
| 50 | `LZ4F_compressEnd` | lz4frame.c:1218 — `assertion violated: flushSize <= dstCapacity` | `assertion failure` [x] |
| 51 | `LZ4F_compressEnd` | lz4frame.c:1221 — `dstCapacity < 4` | `LZ4F_ERROR_dstMaxSize_tooSmall` [x] |
| 52 | `LZ4F_compressEnd` | lz4frame.c:1227 — `dstCapacity < 8` | `LZ4F_ERROR_dstMaxSize_tooSmall` [x] |
| 53 | `LZ4F_compressEnd` | lz4frame.c:1237 — `if (cctxPtr->prefs.frameInfo.contentSize != cctxPtr->totalInSize)` | `LZ4F_ERROR_frameSize_wrong` [x] |
| 54 | `LZ4F_createDecompressionContext_advanced` | lz4frame.c:1287 — `dctx == NULL` | `NULL` [x] |
| 55 | `LZ4F_createDecompressionContext` | lz4frame.c:1303 — `assertion violated: LZ4F_decompressionContextPtr != NULL` | `assertion failure` [x] |
| 56 | `LZ4F_createDecompressionContext` | lz4frame.c:1304 — `LZ4F_decompressionContextPtr == NULL` | `LZ4F_ERROR_parameter_null` [x] |
| 57 | `LZ4F_createDecompressionContext` | lz4frame.c:1308 — `if (*LZ4F_decompressionContextPtr == NULL) { /* failed allocation */` | `LZ4F_ERROR_allocation_failed` [x] |
| 58 | `LZ4F_decodeHeader` | lz4frame.c:1354 — `srcSize < minFHSize` | `LZ4F_ERROR_frameHeader_incomplete` [x] |
| 59 | `LZ4F_decodeHeader` | lz4frame.c:1374 — `if (LZ4F_readLE32(srcPtr) != LZ4F_MAGICNUMBER) {` | `LZ4F_ERROR_frameType_unknown` [x] |
| 60 | `LZ4F_decodeHeader` | lz4frame.c:1388 — `((FLG>>1)&_1BIT) != 0` | `LZ4F_ERROR_reservedFlag_set` [x] |
| 61 | `LZ4F_decodeHeader` | lz4frame.c:1389 — `version != 1` | `LZ4F_ERROR_headerVersion_wrong` [x] |
| 62 | `LZ4F_decodeHeader` | lz4frame.c:1409 — `((BD>>7)&_1BIT) != 0` | `LZ4F_ERROR_reservedFlag_set` [x] |
| 63 | `LZ4F_decodeHeader` | lz4frame.c:1410 — `blockSizeID < 4` | `LZ4F_ERROR_maxBlockSize_invalid` [x] |
| 64 | `LZ4F_decodeHeader` | lz4frame.c:1411 — `((BD>>0)&_4BITS) != 0` | `LZ4F_ERROR_reservedFlag_set` [x] |
| 65 | `LZ4F_decodeHeader` | lz4frame.c:1418 — `HC != srcPtr[frameHeaderSize-1]` | `LZ4F_ERROR_headerChecksum_invalid` [x] |
| 66 | `LZ4F_headerSize` | lz4frame.c:1446 — `src == NULL` | `LZ4F_ERROR_srcPtr_wrong` [x] |
| 67 | `LZ4F_headerSize` | lz4frame.c:1450 — `if (srcSize < LZ4F_MIN_SIZE_TO_KNOW_HEADER_LENGTH)` | `LZ4F_ERROR_frameHeader_incomplete` [x] |
| 68 | `LZ4F_headerSize` | lz4frame.c:1459 — `if (LZ4F_readLE32(src) != LZ4F_MAGICNUMBER)` | `LZ4F_ERROR_frameType_unknown` [x] |
| 69 | `LZ4F_getFrameInfo` | lz4frame.c:1501 — `if (dctx->dStage == dstage_storeFrameHeader) {` | `LZ4F_ERROR_frameDecoding_alreadyStarted` [x] |
| 70 | `LZ4F_getFrameInfo` | lz4frame.c:1507 — `if (*srcSizePtr < hSize) {` | `LZ4F_ERROR_frameHeader_incomplete` [x] |
| 71 | `LZ4F_decompress` | lz4frame.c:1637 — `assertion violated: dctx != NULL` | `assertion failure` [x] |
| 72 | `LZ4F_decompress` | lz4frame.c:1686 — `dctx->tmpIn == NULL` | `LZ4F_ERROR_allocation_failed` [x] |
| 73 | `LZ4F_decompress` | lz4frame.c:1689 — `dctx->tmpOutBuffer== NULL` | `LZ4F_ERROR_allocation_failed` [x] |
| 74 | `LZ4F_decompress` | lz4frame.c:1738 — `if (nextCBlockSize > dctx->maxBlockSize) {` | `LZ4F_ERROR_maxBlockSize_invalid` [x] |
| 75 | `LZ4F_decompress` | lz4frame.c:1829 — `if (readCRC != calcCRC) {` | `LZ4F_ERROR_blockChecksum_invalid` [x] |
| 76 | `LZ4F_decompress` | lz4frame.c:1872 — `assertion violated: dctx->tmpInTarget >= 4` | `assertion failure` [x] |
| 77 | `LZ4F_decompress` | lz4frame.c:1874 — `assertion violated: selectedIn != NULL` | `assertion failure` [x] |
| 78 | `LZ4F_decompress` | lz4frame.c:1878 — `readBlockCrc != calcBlockCrc` | `LZ4F_ERROR_blockChecksum_invalid` [x] |
| 79 | `LZ4F_decompress` | lz4frame.c:1895 — `assertion violated: dstPtr != NULL` | `assertion failure` [x] |
| 80 | `LZ4F_decompress` | lz4frame.c:1905 — `decodedSize < 0` | `LZ4F_ERROR_decompressionFailed` [x] |
| 81 | `LZ4F_decompress` | lz4frame.c:1950 — `decodedSize < 0` | `LZ4F_ERROR_decompressionFailed` [x] |
| 82 | `LZ4F_decompress` | lz4frame.c:1984 — `dctx->frameRemainingSize` | `LZ4F_ERROR_frameSize_wrong` [x] |
| 83 | `LZ4F_decompress` | lz4frame.c:2021 — `readCRC != resultCRC` | `LZ4F_ERROR_contentChecksum_invalid` [x] |
| 84 | `LZ4F_decompress` | lz4frame.c:2095 — `assertion violated: dctx->tmpOutBuffer != NULL` | `assertion failure` [x] |
| 85 | `LZ4F_readOpen` | lz4file.c:80 — `if (fp == NULL \|\| lz4fRead == NULL) {` | `LZ4F_ERROR_parameter_null` [x] |
| 86 | `LZ4F_readOpen` | lz4file.c:85 — `if (*lz4fRead == NULL) {` | `LZ4F_ERROR_allocation_failed` [x] |
| 87 | `LZ4F_readOpen` | lz4file.c:98 — `if (consumedSize != sizeof(buf)) {` | `LZ4F_ERROR_io_read` [x] |
| 88 | `LZ4F_readOpen` | lz4file.c:124 — `default:` | `LZ4F_ERROR_maxBlockSize_invalid` [x] |
| 89 | `LZ4F_readOpen` | lz4file.c:131 — `if ((*lz4fRead)->srcBuf == NULL) {` | `LZ4F_ERROR_allocation_failed` [x] |
| 90 | `LZ4F_read` | lz4file.c:146 — `if (lz4fRead == NULL \|\| buf == NULL)` | `LZ4F_ERROR_parameter_null` [x] |
| 91 | `LZ4F_read` | lz4file.c:162 — `} else if (ret == 0) {` | `LZ4F_ERROR_io_read` [x] |
| 92 | `LZ4F_readClose` | lz4file.c:186 — `if (lz4fRead == NULL)` | `LZ4F_ERROR_parameter_null` [x] |
| 93 | `LZ4F_writeOpen` | lz4file.c:223 — `if (fp == NULL \|\| lz4fWrite == NULL)` | `LZ4F_ERROR_parameter_null` [x] |
| 94 | `LZ4F_writeOpen` | lz4file.c:227 — `if (*lz4fWrite == NULL) {` | `LZ4F_ERROR_allocation_failed` [x] |
| 95 | `LZ4F_writeOpen` | lz4file.c:246 — `default:` | `LZ4F_ERROR_maxBlockSize_invalid` [x] |
| 96 | `LZ4F_writeOpen` | lz4file.c:256 — `if ((*lz4fWrite)->dstBuf == NULL) {` | `LZ4F_ERROR_allocation_failed` [x] |
| 97 | `LZ4F_writeOpen` | lz4file.c:273 — `if (ret != fwrite(buf, 1, ret, fp)) {` | `LZ4F_ERROR_io_write` [x] |
| 98 | `LZ4F_write` | lz4file.c:289 — `if (lz4fWrite == NULL \|\| buf == NULL)` | `LZ4F_ERROR_parameter_null` [x] |
| 99 | `LZ4F_write` | lz4file.c:307 — `if (ret != fwrite(lz4fWrite->dstBuf, 1, ret, lz4fWrite->fp)) {` | `LZ4F_ERROR_io_write` [x] |
| 100 | `LZ4F_writeClose` | lz4file.c:322 — `if (lz4fWrite == NULL) {` | `LZ4F_ERROR_parameter_null` [x] |
| 101 | `LZ4_compressBound` | inputSize < 0 or inputSize > LZ4_MAX_INPUT_SIZE (header macro boundary) | `0` [x] |
| 102 | `LZ4_decoderRingBufferSize` | maxBlockSize < 0 or maxBlockSize > LZ4_MAX_INPUT_SIZE | `0` [x] |
| 103 | `LZ4_initStream` | stateBuffer is NULL | `NULL` [x] |
| 104 | `LZ4_initStream` | size < sizeof(LZ4_stream_t) | `NULL` [x] |
| 105 | `LZ4_initStream` | stateBuffer is not aligned for LZ4_stream_t | `NULL` [x] |
| 106 | `LZ4_initStreamHC` | buffer is NULL | `NULL` [x] |
| 107 | `LZ4_initStreamHC` | size < sizeof(LZ4_streamHC_t) | `NULL` [x] |
| 108 | `LZ4_initStreamHC` | buffer is not aligned for LZ4_streamHC_t | `NULL` [x] |
| 109 | `LZ4F_getBlockSize` | blockSizeID < LZ4F_max64KB or blockSizeID > LZ4F_max4MB, including invalid enum integers | `LZ4F_ERROR_maxBlockSize_invalid` [x] |
| 110 | `LZ4F_headerSize` | src is NULL | `LZ4F_ERROR_srcPtr_wrong` [x] |
| 111 | `LZ4F_headerSize` | srcSize < LZ4F_MIN_SIZE_TO_KNOW_HEADER_LENGTH | `LZ4F_ERROR_frameHeader_incomplete` [x] |
| 112 | `LZ4F_createCompressionContext` | output context pointer is NULL | `LZ4F_ERROR_parameter_null` [x] |
| 113 | `LZ4F_createDecompressionContext` | output context pointer is NULL | `LZ4F_ERROR_parameter_null` [x] |
| 114 | `LZ4F_readOpen` | FILE pointer or output state pointer is NULL | `LZ4F_ERROR_parameter_null` [x] |
| 115 | `LZ4F_read` | state pointer or output buffer is NULL | `LZ4F_ERROR_parameter_null` [x] |
| 116 | `LZ4F_readClose` | state pointer is NULL | `LZ4F_ERROR_parameter_null` [x] |
| 117 | `LZ4F_writeOpen` | FILE pointer or output state pointer is NULL | `LZ4F_ERROR_parameter_null` [x] |
| 118 | `LZ4F_write` | state pointer or input buffer is NULL | `LZ4F_ERROR_parameter_null` [x] |
| 119 | `LZ4F_writeClose` | state pointer is NULL | `LZ4F_ERROR_parameter_null` [x] |
| 120 | `LZ4_XXH32_update` | input is NULL and len is nonzero | `XXH_ERROR` [x] |
| 121 | `LZ4_XXH64_update` | input is NULL and len is nonzero | `XXH_ERROR` [x] |
