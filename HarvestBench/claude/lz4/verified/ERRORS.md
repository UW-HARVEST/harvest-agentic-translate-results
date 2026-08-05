# ERRORS.md — Error-surface table

Mechanically derived from the C source (`c_src/src/*.c`). Each row is a distinct
rejection/error condition. LZ4 block functions signal errors with return `0`
(compress) or negative (`return -1` / `_output_error`) (decompress). LZ4F frame
functions return an error code via `LZ4F_returnErrorCode(LZ4F_ERROR_*)`, testable
with `LZ4F_isError()` and identified with `LZ4F_getErrorCode()`.

## Block API (lz4.c / lz4hc.c)

| # | function | trigger (invalid input/condition) | expected C result |
|---|----------|-------------------------------------|-------------------|
| 1 | LZ4_compressBound | inputSize > LZ4_MAX_INPUT_SIZE (0x7E000000) or negative | returns 0 |
| 2 | LZ4_compress_default / _fast | srcSize > LZ4_MAX_INPUT_SIZE or negative (lz4.c:1360) | returns 0 |
| 3 | LZ4_compress_default / _fast | dstCapacity too small to hold compressed output (lz4.c:1116,1210,1314) | returns 0 |
| 4 | LZ4_compress_destSize | outputDirective==fillOutput && maxOutputSize < 1 (lz4.c:985) | returns 0 |
| 5 | LZ4_compress_fast_continue | dstCapacity <=0 with limited output & cannot fit | returns 0 |
| 6 | LZ4_decompress_safe | src==NULL || outputSize<0 (lz4.c:2036) | returns -1 |
| 7 | LZ4_decompress_safe | dstCapacity too small (output overflow, lz4.c:1898,1921) | returns negative |
| 8 | LZ4_decompress_safe | malformed stream / offset underflow / read overrun (_output_error) | returns negative |
| 9 | LZ4_decompress_safe | srcSize==0 (non-partial), lz4.c:2069 | returns -1 |
| 10 | LZ4_decompress_safe_partial | targetOutputSize handling; malformed input | returns negative |
| 11 | LZ4_compress_HC (lz4hc) | *srcSizePtr < 0 (lz4hc.c:559) | returns 0 |
| 12 | LZ4_compress_HC (lz4hc) | maxOutputSize < 0 (lz4hc.c:560) | returns 0 |
| 13 | LZ4_compress_HC (lz4hc) | limitedOutput && not enough space in dst (lz4hc.c:714,1315) | returns 0 |
| 14 | LZ4_compress_HC_destSize | limit==fillOutput && dstCapacity < 1 (lz4hc.c:1388) | returns 0 |
| 15 | LZ4_compress_HC_destSize | *srcSizePtr > LZ4_MAX_INPUT_SIZE (lz4hc.c:1389) | returns 0 |
| 16 | LZ4_initStream | stateBuffer NULL or bad size/alignment | returns NULL |
| 17 | LZ4_freeStream / LZ4_freeStreamHC | NULL ptr (support free on NULL) | returns 0, no crash |
| 18 | LZ4_decoderRingBufferSize | invalid maxBlockSize (<=0) | returns 0 |

## Frame API (lz4frame.c) — LZ4F_ERROR_* codes

| # | function | trigger | expected C result (LZ4F_ERROR_*) |
|---|----------|---------|----------------------------------|
| 19 | LZ4F_compressFrame | dstCapacity < compressFrameBound (lz4frame.c:456) | ERROR_dstMaxSize_tooSmall |
| 20 | LZ4F_createCompressionContext | cctxPtr == NULL (lz4frame.c:622) | ERROR_parameter_null |
| 21 | LZ4F_compressBegin | dstCapacity < maxFHSize (lz4frame.c:700) | ERROR_dstMaxSize_tooSmall |
| 22 | LZ4F_compressUpdate | cctx->cStage != 1 (uninitialized, lz4frame.c:1005) | ERROR_compressionState_uninitialized |
| 23 | LZ4F_compressUpdate | dstCapacity too small (lz4frame.c:1007,1010) | ERROR_dstMaxSize_tooSmall |
| 24 | LZ4F_flush/compressEnd | cctx->cStage != 1 (lz4frame.c:1168) | ERROR_compressionState_uninitialized |
| 25 | LZ4F_compressEnd | dstCapacity < tmpInSize+BHSize+BFSize (lz4frame.c:1169) | ERROR_dstMaxSize_tooSmall |
| 26 | LZ4F_compressEnd (checksum) | dstCapacity < 4 / < 8 (lz4frame.c:1221,1227) | ERROR_dstMaxSize_tooSmall |
| 27 | LZ4F_compressEnd | frameRemainingSize mismatch (lz4frame.c:1237) | ERROR_frameSize_wrong |
| 28 | LZ4F_createDecompressionContext | dctxPtr == NULL (lz4frame.c:1304) | ERROR_parameter_null |
| 29 | LZ4F_headerSize | srcSize < minFHSize (lz4frame.c:1354) | ERROR_frameHeader_incomplete |
| 30 | LZ4F_headerSize/decodeHeader | unknown magic / frameType (lz4frame.c:1374,1459) | ERROR_frameType_unknown |
| 31 | decodeHeader | reserved FLG bit set (lz4frame.c:1388,1409,1411) | ERROR_reservedFlag_set |
| 32 | decodeHeader | version != 1 (lz4frame.c:1389) | ERROR_headerVersion_wrong |
| 33 | decodeHeader | blockSizeID < 4 (lz4frame.c:1410) | ERROR_maxBlockSize_invalid |
| 34 | decodeHeader | header checksum mismatch (lz4frame.c:1418) | ERROR_headerChecksum_invalid |
| 35 | LZ4F_getFrameInfo | src == NULL (lz4frame.c:1446) | ERROR_srcPtr_wrong |
| 36 | LZ4F_getFrameInfo | srcSize too small for header (lz4frame.c:1450,1507) | ERROR_frameHeader_incomplete |
| 37 | LZ4F_getFrameInfo | decoding already started (lz4frame.c:1501) | ERROR_frameDecoding_alreadyStarted |
| 38 | LZ4F_decompress | blockSizeID/maxBlockSize invalid (lz4frame.c:1738) | ERROR_maxBlockSize_invalid |
| 39 | LZ4F_decompress | block checksum invalid (lz4frame.c:1829,1878) | ERROR_blockChecksum_invalid |
| 40 | LZ4F_decompress | inner block decode failed (lz4frame.c:1905,1950) | ERROR_decompressionFailed |
| 41 | LZ4F_decompress | frame size wrong at end (lz4frame.c:1984) | ERROR_frameSize_wrong |
| 42 | LZ4F_decompress | content checksum mismatch (lz4frame.c:2021) | ERROR_contentChecksum_invalid |
| 43 | LZ4F_createCDict / alloc paths | allocation failed | ERROR_allocation_failed |
| 44 | LZ4F_compressBegin_usingDict | dictSize > INT_MAX (lz4frame.c:768) | ERROR_parameter_invalid |
| 45 | LZ4F_isError | error code (>-LZ4F_ERROR_maxCode) | returns 1 for errors, 0 otherwise |
| 46 | LZ4F_getErrorName / getErrorCode | any error code | returns matching name/enum |

## File API (lz4file.c) — LZ4F_ERROR_* codes

| # | function | trigger | expected C result |
|---|----------|---------|-------------------|
| 47 | LZ4F_readOpen | fp==NULL or lz4fRead==NULL (lz4file.c:80) | ERROR_parameter_null |
| 48 | LZ4F_readOpen | context allocation failed (lz4file.c:85) | ERROR_allocation_failed |
| 49 | LZ4F_readOpen | fread error (lz4file.c:98) | ERROR_io_read |
| 50 | LZ4F_readOpen | invalid maxBlockSize (lz4file.c:124) | ERROR_maxBlockSize_invalid |
| 51 | LZ4F_read | lz4fRead==NULL / buf==NULL (lz4file.c:146) | ERROR_parameter_null |
| 52 | LZ4F_read | fread error (lz4file.c:162) | ERROR_io_read |
| 53 | LZ4F_readClose | lz4fRead==NULL (lz4file.c:186) | ERROR_parameter_null |
| 54 | LZ4F_writeOpen | fp==NULL/lz4fWrite==NULL (lz4file.c:223) | ERROR_parameter_null |
| 55 | LZ4F_writeOpen | allocation failed (lz4file.c:227,256) | ERROR_allocation_failed |
| 56 | LZ4F_writeOpen | fwrite error (lz4file.c:273) | ERROR_io_write |
| 57 | LZ4F_write | lz4fWrite==NULL/buf==NULL (lz4file.c:289) | ERROR_parameter_null |
| 58 | LZ4F_write | fwrite error (lz4file.c:306,307) | ERROR_io_write |
| 59 | LZ4F_writeClose | lz4fWrite==NULL (lz4file.c:322) | ERROR_parameter_null |
| 60 | LZ4F_writeClose | fwrite error (lz4file.c:334) | ERROR_io_write |

## xxHash API (xxhash.c)

| # | function | trigger | expected C result |
|---|----------|---------|-------------------|
| 61 | LZ4_XXH32_update / XXH64_update | input==NULL (xxhash.c:454) | XXH_ERROR (unless len==0 → XXH_OK) |
| 62 | LZ4_XXH*_reset / state ops | p==NULL (xxhash.c:360,819) | handled (no crash) |
