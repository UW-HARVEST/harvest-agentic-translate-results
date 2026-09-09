# ERRORS.md — Error-surface table

Derived mechanically from the C sources (`c_src/src/{lz4,lz4hc,lz4frame,lz4file,xxhash}.c`,
`c_src/include/*.h`) by grepping every `RETURN_ERROR`, `return -1`, `return 0`, `return NULL`,
`_output_error`, `err0r`, `rvl_error`, `XXH_ERROR`, `assert`, explicit range check, null check
and min/max constant.

## Conventions

* `LZ4F_*` functions return errors as `(size_t)-(ptrdiff_t)code`.
  `LZ4F_isError(r)` is `r > (size_t)(-LZ4F_ERROR_maxCode)` = `r > SIZE_MAX-24`,
  so exactly codes **1..23** are detectable errors.
* `_output_error` in `LZ4_decompress_generic` expands to
  `return (int)(-(ip - (const BYTE*)src)) - 1` — a *negative* value whose exact
  magnitude depends on how far into the input the error was detected. Tests must
  compare the exact int returned by C vs Rust, not merely "both < 0".
* `rvl_error` is `(Rvl_t)(-1)`, an internal sentinel, surfaced through `_output_error`.
* `XXH_ERROR` = `1`, `XXH_OK` = `0`.

## `LZ4F_errorCodes` values (declaration order = numeric value)

| code | name | returned value |
|---|---|---|
| 0 | `LZ4F_OK_NoError` | `0` (not an error) |
| 1 | `LZ4F_ERROR_GENERIC` | `(size_t)-1` |
| 2 | `LZ4F_ERROR_maxBlockSize_invalid` | `(size_t)-2` |
| 3 | `LZ4F_ERROR_blockMode_invalid` | `(size_t)-3` |
| 4 | `LZ4F_ERROR_parameter_invalid` | `(size_t)-4` |
| 5 | `LZ4F_ERROR_compressionLevel_invalid` | `(size_t)-5` |
| 6 | `LZ4F_ERROR_headerVersion_wrong` | `(size_t)-6` |
| 7 | `LZ4F_ERROR_blockChecksum_invalid` | `(size_t)-7` |
| 8 | `LZ4F_ERROR_reservedFlag_set` | `(size_t)-8` |
| 9 | `LZ4F_ERROR_allocation_failed` | `(size_t)-9` |
| 10 | `LZ4F_ERROR_srcSize_tooLarge` | `(size_t)-10` |
| 11 | `LZ4F_ERROR_dstMaxSize_tooSmall` | `(size_t)-11` |
| 12 | `LZ4F_ERROR_frameHeader_incomplete` | `(size_t)-12` |
| 13 | `LZ4F_ERROR_frameType_unknown` | `(size_t)-13` |
| 14 | `LZ4F_ERROR_frameSize_wrong` | `(size_t)-14` |
| 15 | `LZ4F_ERROR_srcPtr_wrong` | `(size_t)-15` |
| 16 | `LZ4F_ERROR_decompressionFailed` | `(size_t)-16` |
| 17 | `LZ4F_ERROR_headerChecksum_invalid` | `(size_t)-17` |
| 18 | `LZ4F_ERROR_contentChecksum_invalid` | `(size_t)-18` |
| 19 | `LZ4F_ERROR_frameDecoding_alreadyStarted` | `(size_t)-19` |
| 20 | `LZ4F_ERROR_compressionState_uninitialized` | `(size_t)-20` |
| 21 | `LZ4F_ERROR_parameter_null` | `(size_t)-21` |
| 22 | `LZ4F_ERROR_io_write` | `(size_t)-22` |
| 23 | `LZ4F_ERROR_io_read` | `(size_t)-23` |
| 24 | `LZ4F_ERROR_maxCode` | sentinel only |

Codes 1, 3, 5, 10 (`GENERIC`, `blockMode_invalid`, `compressionLevel_invalid`,
`srcSize_tooLarge`) have **no raise site** in the C sources — they exist only in
the enum + `LZ4F_getErrorName` string table. They are still covered by the
`LZ4F_getErrorName` / `LZ4F_getErrorCode` / `LZ4F_isError` round-trip rows below.

## Rejection table

| # | function | trigger (exact invalid input/condition) | expected C result | test |
|---|----------|------------------------------------------|-------------------|------|
| 1 | `LZ4_compressBound` | `(unsigned)inputSize > LZ4_MAX_INPUT_SIZE` (0x7E000000), incl. negative int (`lz4.c:751`) | `0` | [x] |
| 2 | `LZ4_compress_default` / `_fast` / `_fast_extState` (`LZ4_compress_generic`) | `(U32)srcSize > (U32)LZ4_MAX_INPUT_SIZE` (too large or negative) (`lz4.c:1360`) | `0` | [x] |
| 3 | `LZ4_compress_generic` | `srcSize == 0` && `outputDirective != notLimited` && `dstCapacity <= 0` (`lz4.c:1362`) | `0` | [x] |
| 4 | `LZ4_compress_destSize` / `_extState` | `outputDirective == fillOutput && maxOutputSize < 1` (`lz4.c:985`) | `0` | [x] |
| 5 | `LZ4_compress_default` (limitedOutput) | literal run does not fit: `op+litLength+(2+1+LASTLITERALS)+(litLength/255) > olimit` (`lz4.c:1116`) | `0` | [x] |
| 6 | `LZ4_compress_default` (limitedOutput) | match-length bytes do not fit `olimit` (`lz4.c:1210`) | `0` | [x] |
| 7 | `LZ4_compress_default` (limitedOutput) | last-literals run does not fit `olimit` (`lz4.c:1314`) | `0` | [x] |
| 8 | `LZ4_createStream` | `ALLOC` failure (`lz4.c:1536`) — not triggerable from FFI; covered by non-NULL assertion | `NULL` | [x] |
| 9 | `LZ4_initStream` | `buffer == NULL` (`lz4.c:1555`) | `NULL` | [x] |
| 10 | `LZ4_initStream` | `size < sizeof(LZ4_stream_t)` (`lz4.c:1556`) | `NULL` | [x] |
| 11 | `LZ4_initStream` | `buffer` misaligned vs `LZ4_stream_t_alignment()` (`lz4.c:1557`) | `NULL` | [x] |
| 12 | `LZ4_freeStream` | `LZ4_stream == NULL` (tolerated) (`lz4.c:1577`) | `0` | [x] |
| 13 | `LZ4_loadDict` / `LZ4_loadDictSlow` | `dictSize < (int)HASH_UNIT` (8 on 64-bit) → dict silently rejected (`lz4.c:1614`) | `0` | [x] |
| 14 | `LZ4_loadDict` / `LZ4_loadDictHC` | `dictionary == NULL` or `dictSize <= 0` (`lz4.c:1609`, `lz4hc.c:1630`) | `0` | [x] |
| 15 | `LZ4_decompress_fast` (`LZ4_decompress_unsafe_generic`) | literal length `ll` > remaining output (`lz4.c:1898`) | `-1` | [x] |
| 16 | `LZ4_decompress_fast` | literals end `< MFLIMIT`(12) from block end and `op != oend` (`lz4.c:1907`) | `-1` | [x] |
| 17 | `LZ4_decompress_fast` | match length `ml` > remaining output (`lz4.c:1921`) | `-1` | [x] |
| 18 | `LZ4_decompress_fast` | `offset > (size_t)(op - prefixStart) + dictSize` (`lz4.c:1928`) | `-1` | [x] |
| 19 | `LZ4_decompress_fast` | match ends `< LASTLITERALS`(5) from block end (`lz4.c:1961`) | `-1` | [x] |
| 20 | `read_variable_length` via `LZ4_decompress_safe` | `initial_check && *ip >= ilimit` — no length byte available (`lz4.c:1987`) | `_output_error` (exact negative int) | [x] |
| 21 | `read_variable_length` | `*ip > ilimit` after first length byte (`lz4.c:1993`) | `_output_error` | [x] |
| 22 | `read_variable_length` | `*ip > ilimit` inside the 255-continuation loop (`lz4.c:2005`) | `_output_error` | [x] |
| 23 | `read_variable_length` | 32-bit only: accumulator overflow `length > ((Rvl_t)-1)/2` (`lz4.c:1997`, `2009`) | `_output_error` (n/a on 64-bit) | [x] |
| 24 | `LZ4_decompress_safe` (`LZ4_decompress_generic`) | `src == NULL` (`lz4.c:2036`) | `-1` | [x] |
| 25 | `LZ4_decompress_safe` | `outputSize < 0` (`lz4.c:2036`) | `-1` | [x] |
| 26 | `LZ4_decompress_safe` | `outputSize == 0`, full decode, and NOT (`srcSize==1 && *src==0`) (`lz4.c:2068`) | `-1` (`0` for the valid empty-block form) | [x] |
| 27 | `LZ4_decompress_safe` | `srcSize == 0` with `outputSize > 0` (`lz4.c:2069`) | `-1` | [x] |
| 28 | `LZ4_decompress_safe` (fast loop) | `rvl_error` on long literal length (`lz4.c:2096`) | `_output_error` | [x] |
| 29 | `LZ4_decompress_safe` (fast loop) | output pointer overflow `(uptrval)op+length < (uptrval)op` (`lz4.c:2099`) | `_output_error` | [x] |
| 30 | `LZ4_decompress_safe` (fast loop) | input pointer overflow `(uptrval)ip+length < (uptrval)ip` (`lz4.c:2100`) | `_output_error` | [x] |
| 31 | `LZ4_decompress_safe` (fast loop) | `rvl_error` on long match length (`lz4.c:2131`) | `_output_error` | [x] |
| 32 | `LZ4_decompress_safe` (fast loop) | output pointer overflow on match length (`lz4.c:2136`) | `_output_error` | [x] |
| 33 | `LZ4_decompress_safe` (fast loop) | `checkOffset && (match + dictSize < lowPrefix)` — offset outside buffers (`lz4.c:2163`) | `_output_error` | [x] |
| 34 | `LZ4_decompress_safe_usingDict` (fast loop) | extDict match with `op+length > oend-LASTLITERALS` and not partialDecoding (`lz4.c:2174`) | `_output_error` | [x] |
| 35 | `LZ4_decompress_safe` (safe loop) | `rvl_error` on long literal length (`lz4.c:2266`) | `_output_error` | [x] |
| 36 | `LZ4_decompress_safe` (safe loop) | output pointer overflow on literal length (`lz4.c:2268`) | `_output_error` | [x] |
| 37 | `LZ4_decompress_safe` (safe loop) | input pointer overflow on literal length (`lz4.c:2269`) | `_output_error` | [x] |
| 38 | `LZ4_decompress_safe` (safe loop) | last literal run `(ip+length != iend) \|\| (cpy > oend)` (`lz4.c:2317`) | `_output_error` | [x] |
| 39 | `LZ4_decompress_safe` (safe loop) | `rvl_error` on long match length (`lz4.c:2347`) | `_output_error` | [x] |
| 40 | `LZ4_decompress_safe` (safe loop) | output pointer overflow on match length (`lz4.c:2349`) | `_output_error` | [x] |
| 41 | `LZ4_decompress_safe` (safe loop) | `checkOffset && (match + dictSize < lowPrefix)` (`lz4.c:2356`) | `_output_error` | [x] |
| 42 | `LZ4_decompress_safe_usingDict` (safe loop) | extDict match, `op+length > oend-LASTLITERALS`, not partial (`lz4.c:2362`) | `_output_error` | [x] |
| 43 | `LZ4_decompress_safe` (safe loop) | match copy `cpy > oend-LASTLITERALS` (`lz4.c:2423`) | `_output_error` | [x] |
| 44 | `LZ4_freeStreamDecode` | `LZ4_stream == NULL` (`lz4.c:2577`) | `0` | [x] |
| 45 | `LZ4_decoderRingBufferSize` | `maxBlockSize < 0` (`lz4.c:2617`) | `0` | [x] |
| 46 | `LZ4_decoderRingBufferSize` | `maxBlockSize > LZ4_MAX_INPUT_SIZE` (`lz4.c:2618`) | `0` | [x] |
| 47 | `LZ4_decompress_safe_continue` / `_fast_continue` | inner decode returns `<= 0` → forwarded, stream state NOT advanced (`lz4.c:2681`,`2688`,`2694`,`2696`,`2700`,`2705`) | forwarded value | [x] |
| 48 | `LZ4_setStreamDecode` | `dictionary == NULL`/`dictSize == 0` reset form vs invalid | `1` (always succeeds) | [x] |
| 49 | `LZ4_compress_HC` (`LZ4HC_encodeSequence`, limited) | literals do not fit: `op+(length/255)+length+(2+1+LASTLITERALS) > oend` (`lz4hc.c:308`) | `1` internally → caller `0` | [x] |
| 50 | `LZ4_compress_HC` (`LZ4HC_encodeSequence`, limited) | match encoding does not fit output (`lz4hc.c:333`) | `1` internally → caller `0` | [x] |
| 51 | `LZ4_compress_HC` clevel 1..2 (`LZ4MID_compress`) | `*srcSizePtr < 0` (`lz4hc.c:559`) | `0` | [x] |
| 52 | `LZ4_compress_HC` clevel 1..2 | `maxOutputSize < 0` (`lz4hc.c:560`) | `0` | [x] |
| 53 | `LZ4_compress_HC` clevel 1..2 | `*srcSizePtr > LZ4_MAX_INPUT_SIZE` (`lz4hc.c:563`) | `0` | [x] |
| 54 | `LZ4_compress_HC` clevel 1..2 | limitedOutput and last literal run does not fit (`lz4hc.c:714`) | `0` | [x] |
| 55 | `LZ4_compress_HC_destSize` clevel 1..2 | `_lz4mid_dest_overflow` — fillOutput exhausted (`lz4hc.c:772`) | `0` | [x] |
| 56 | `LZ4_compress_HC` clevel 3..9 (`hashChain`) | limitedOutput and last literal run does not fit (`lz4hc.c:1315`) | `0` | [x] |
| 57 | `LZ4_compress_HC` clevel 3..9 | `_dest_overflow` — compression failed (`lz4hc.c:1361`) | `0` | [x] |
| 58 | `LZ4_compress_HC` clevel 10..12 (`optimal`) | limitedOutput and last literal run does not fit → `retval = 0` (`lz4hc.c:2066`) | `0` | [x] |
| 59 | `LZ4HC_compress_generic_internal` | `limit == fillOutput && dstCapacity < 1` (`lz4hc.c:1388`) | `0` | [x] |
| 60 | `LZ4HC_compress_generic_internal` | `(U32)*srcSizePtr > (U32)LZ4_MAX_INPUT_SIZE` (too large or negative) (`lz4hc.c:1389`) | `0` | [x] |
| 61 | `LZ4_compress_HC_extStateHC_fastReset` | `state` misaligned vs `LZ4_streamHC_t_alignment()` (`lz4hc.c:1503`) | `0` | [x] |
| 62 | `LZ4_compress_HC_extStateHC` | `LZ4_initStreamHC(state,size)` returns NULL (NULL / undersized / misaligned) (`lz4hc.c:1515`) | `0` | [x] |
| 63 | `LZ4_compress_HC_destSize` | `LZ4_initStreamHC` returns NULL (`lz4hc.c:1541`) | `0` | [x] |
| 64 | `LZ4_createStreamHC` / `LZ4_createHC` | `ALLOC_AND_ZERO` failure (`lz4hc.c:1558`, `2162`) — not FFI-triggerable | `NULL` | [x] |
| 65 | `LZ4_freeStreamHC` | `LZ4_streamHCPtr == NULL` (`lz4hc.c:1566`) | `0` | [x] |
| 66 | `LZ4_initStreamHC` | `buffer == NULL` (`lz4hc.c:1578`) | `NULL` | [x] |
| 67 | `LZ4_initStreamHC` | `size < sizeof(LZ4_streamHC_t)` (`lz4hc.c:1579`) | `NULL` | [x] |
| 68 | `LZ4_initStreamHC` | `buffer` misaligned (`lz4hc.c:1580`) | `NULL` | [x] |
| 69 | `LZ4_resetStreamStateHC` | `LZ4_initStreamHC` returns NULL (NULL/misaligned state) (`lz4hc.c:2153`) | `1` (nonzero == error) | [x] |
| 70 | `LZ4_freeHC` | `LZ4HC_Data == NULL` (`lz4hc.c:2169`) | `0` | [x] |
| 71 | `LZ4F_getBlockSize` | `blockSizeID < 4` after 0→default remap, or `> 7` (`lz4frame.c:337-339`) | `(size_t)-2` `maxBlockSize_invalid` | [x] |
| 72 | `LZ4F_getBlockSize` | out-of-range enum value across FFI (e.g. 8, 99, -1, INT_MAX) | `(size_t)-2` | [x] |
| 73 | `LZ4F_compressFrame` / `_usingCDict` | `dstCapacity < LZ4F_compressFrameBound(srcSize,&prefs)` (`lz4frame.c:456`) | `(size_t)-11` `dstMaxSize_tooSmall` | [x] |
| 74 | `LZ4F_createCDict_advanced` | any allocation failure (`lz4frame.c:544`, `555-557`) | `NULL` | [x] |
| 75 | `LZ4F_createCompressionContext_advanced` | `LZ4F_calloc` failure (`lz4frame.c:600`) | `NULL` | [x] |
| 76 | `LZ4F_createCompressionContext` | `LZ4F_compressionContextPtr == NULL` (`lz4frame.c:622`) | `(size_t)-21` `parameter_null` | [x] |
| 77 | `LZ4F_createCompressionContext` | inner `_advanced()` returned NULL (`lz4frame.c:625`) | `(size_t)-9` `allocation_failed` | [x] |
| 78 | `LZ4F_compressBound` (`ctxTypeID_to_size`) | `ctxTypeID` not 1 or 2 (`lz4frame.c:683`) | `0` | [x] |
| 79 | `LZ4F_compressBegin` / `_usingCDict` / `_usingDict` | `dstCapacity < LZ4F_HEADER_SIZE_MAX` (19) (`lz4frame.c:700`) | `(size_t)-11` | [x] |
| 80 | `LZ4F_compressBegin_internal` | `lz4CtxPtr` alloc failure (`lz4frame.c:722`) | `(size_t)-9` | [x] |
| 81 | `LZ4F_compressBegin_internal` | `tmpBuff` alloc failure (`lz4frame.c:750`) | `(size_t)-9` | [x] |
| 82 | `LZ4F_compressBegin_usingDict` | `dictBuffer != NULL && dictSize > INT_MAX` (`lz4frame.c:768`) | `(size_t)-4` `parameter_invalid` | [x] |
| 83 | `LZ4F_compressUpdate` | `cctx->cStage != 1` — no `compressBegin`, or frame already ended (`lz4frame.c:1005`) | `(size_t)-20` `compressionState_uninitialized` | [x] |
| 84 | `LZ4F_compressUpdate` | `dstCapacity < LZ4F_compressBound_internal(srcSize, prefs, tmpInSize)` (`lz4frame.c:1006`) | `(size_t)-11` | [x] |
| 85 | `LZ4F_uncompressedUpdate` | `dstCapacity < srcSize` for `LZ4B_UNCOMPRESSED` (`lz4frame.c:1009`) | `(size_t)-11` | [x] |
| 86 | `LZ4F_flush` | `cctx->cStage != 1` and `tmpInSize != 0` (`lz4frame.c:1168`) | `(size_t)-20` | [x] |
| 87 | `LZ4F_flush` | `dstCapacity < tmpInSize + BHSize(4) + BFSize` (`lz4frame.c:1169`) | `(size_t)-11` | [x] |
| 88 | `LZ4F_compressEnd` | after flush, `dstCapacity < 4` for the endMark (`lz4frame.c:1221`) | `(size_t)-11` | [x] |
| 89 | `LZ4F_compressEnd` | contentChecksum on and remaining `dstCapacity < 8` (`lz4frame.c:1227`) | `(size_t)-11` | [x] |
| 90 | `LZ4F_compressEnd` | declared `frameInfo.contentSize != totalInSize` (`lz4frame.c:1236`) | `(size_t)-14` `frameSize_wrong` | [x] |
| 91 | `LZ4F_createDecompressionContext_advanced` | `LZ4F_calloc` failure (`lz4frame.c:1287`) | `NULL` | [x] |
| 92 | `LZ4F_createDecompressionContext` | `LZ4F_decompressionContextPtr == NULL` (`lz4frame.c:1304`) | `(size_t)-21` | [x] |
| 93 | `LZ4F_createDecompressionContext` | inner `_advanced()` returned NULL (`lz4frame.c:1307`) | `(size_t)-9` | [x] |
| 94 | `LZ4F_getFrameInfo` (`LZ4F_decodeHeader`) | `srcSize < LZ4F_HEADER_SIZE_MIN` (7) (`lz4frame.c:1354`) | `(size_t)-12` `frameHeader_incomplete` | [x] |
| 95 | `LZ4F_decompress` / `LZ4F_getFrameInfo` | magic != `0x184D2204` and not in skippable range `0x184D2A5x` (`lz4frame.c:1372`) | `(size_t)-13` `frameType_unknown` | [x] |
| 96 | `LZ4F_decompress` | FLG bit 1 (reserved) set (`lz4frame.c:1388`) | `(size_t)-8` `reservedFlag_set` | [x] |
| 97 | `LZ4F_decompress` | FLG version field (bits 6-7) != 1 (`lz4frame.c:1389`) | `(size_t)-6` `headerVersion_wrong` | [x] |
| 98 | `LZ4F_decompress` | BD bit 7 (reserved) set (`lz4frame.c:1409`) | `(size_t)-8` | [x] |
| 99 | `LZ4F_decompress` | header `blockSizeID = (BD>>4)&7` `< 4` (`lz4frame.c:1410`) | `(size_t)-2` | [x] |
| 100 | `LZ4F_decompress` | BD low 4 bits (reserved) nonzero (`lz4frame.c:1411`) | `(size_t)-8` | [x] |
| 101 | `LZ4F_decompress` | header checksum byte mismatch (`lz4frame.c:1417`) | `(size_t)-17` `headerChecksum_invalid` | [x] |
| 102 | `LZ4F_headerSize` | `src == NULL` (`lz4frame.c:1446`) | `(size_t)-15` `srcPtr_wrong` | [x] |
| 103 | `LZ4F_headerSize` | `srcSize < LZ4F_MIN_SIZE_TO_KNOW_HEADER_LENGTH` (5) (`lz4frame.c:1449`) | `(size_t)-12` | [x] |
| 104 | `LZ4F_headerSize` | magic neither LZ4F nor skippable (`lz4frame.c:1458`) | `(size_t)-13` | [x] |
| 105 | `LZ4F_getFrameInfo` | `dctx->dStage == dstage_storeFrameHeader` (called mid-header) (`lz4frame.c:1500`) | `(size_t)-19` `frameDecoding_alreadyStarted`, `*srcSizePtr = 0` | [x] |
| 106 | `LZ4F_getFrameInfo` | `*srcSizePtr < hSize` from `LZ4F_headerSize` (`lz4frame.c:1506`) | `(size_t)-12`, `*srcSizePtr = 0` | [x] |
| 107 | `LZ4F_decompress` (`dstage_init`) | `tmpIn` alloc failure (`lz4frame.c:1686`) | `(size_t)-9` | [x] |
| 108 | `LZ4F_decompress` (`dstage_init`) | `tmpOutBuffer` alloc failure (`lz4frame.c:1689`) | `(size_t)-9` | [x] |
| 109 | `LZ4F_decompress` | `nextCBlockSize = blockHeader & 0x7FFFFFFF > dctx->maxBlockSize` (`lz4frame.c:1737`) | `(size_t)-2` | [x] |
| 110 | `LZ4F_decompress` | uncompressed block: stored block CRC != `XXH32` of data (`lz4frame.c:1826`) | `(size_t)-7` `blockChecksum_invalid` | [x] |
| 111 | `LZ4F_decompress` | compressed block: trailing block CRC mismatch (`lz4frame.c:1878`) | `(size_t)-7` | [x] |
| 112 | `LZ4F_decompress` (into dst) | `LZ4_decompress_safe_usingDict < 0` — corrupt block (`lz4frame.c:1905`) | `(size_t)-16` `decompressionFailed` | [x] |
| 113 | `LZ4F_decompress` (into tmpOut) | `LZ4_decompress_safe_usingDict < 0` (`lz4frame.c:1950`) | `(size_t)-16` | [x] |
| 114 | `LZ4F_decompress` (`dstage_getSuffix`) | `dctx->frameRemainingSize != 0` at endMark (`lz4frame.c:1984`) | `(size_t)-14` | [x] |
| 115 | `LZ4F_decompress` (suffix) | content checksum `readCRC != XXH32_digest(&dctx->xxh)` (`lz4frame.c:2021`) | `(size_t)-18` `contentChecksum_invalid` | [x] |
| 116 | `LZ4F_readOpen` | `fp == NULL \|\| lz4fRead == NULL` (`lz4file.c:79`) | `(size_t)-21` | [x] |
| 117 | `LZ4F_readOpen` | `calloc` failure (`lz4file.c:84`) | `(size_t)-9` | [x] |
| 118 | `LZ4F_readOpen` | `fread` of 19 bytes short (empty / truncated file) (`lz4file.c:96`) | `(size_t)-23` `io_read` | [x] |
| 119 | `LZ4F_readOpen` | decoded `info.blockSizeID` outside {0,4,5,6,7} (`lz4file.c:122`) | `(size_t)-2` | [x] |
| 120 | `LZ4F_readOpen` | `malloc(srcBufMaxSize)` failure (`lz4file.c:129`) | `(size_t)-9` | [x] |
| 121 | `LZ4F_readOpen` | inner `LZ4F_createDecompressionContext` / `LZ4F_getFrameInfo` error, e.g. bad magic (`lz4file.c:89`,`103`) | forwarded code (e.g. `(size_t)-13`) | [x] |
| 122 | `LZ4F_read` | `lz4fRead == NULL \|\| buf == NULL` (`lz4file.c:145`) | `(size_t)-21` | [x] |
| 123 | `LZ4F_read` | `fread` returns neither `>0` nor `0` (`lz4file.c:161`) | `(size_t)-23` | [x] |
| 124 | `LZ4F_read` | inner `LZ4F_decompress` error (truncated/corrupt payload) (`lz4file.c:171`) | forwarded code | [x] |
| 125 | `LZ4F_readClose` | `lz4fRead == NULL` (`lz4file.c:185`) | `(size_t)-21` | [x] |
| 126 | `LZ4F_writeOpen` | `fp == NULL \|\| lz4fWrite == NULL` (`lz4file.c:222`) | `(size_t)-21` | [x] |
| 127 | `LZ4F_writeOpen` | `calloc` failure (`lz4file.c:226`) | `(size_t)-9` | [x] |
| 128 | `LZ4F_writeOpen` | `prefsPtr->frameInfo.blockSizeID` outside {0,4,5,6,7} (`lz4file.c:244`) | `(size_t)-2` | [x] |
| 129 | `LZ4F_writeOpen` | `malloc(dstBufMaxSize)` failure (`lz4file.c:254`) | `(size_t)-9` | [x] |
| 130 | `LZ4F_writeOpen` | `fwrite` of header short-writes (read-only `FILE*`) (`lz4file.c:271`) | `(size_t)-22` `io_write` | [x] |
| 131 | `LZ4F_writeOpen` | inner `createCompressionContext`/`compressBegin` error (`lz4file.c:260`,`266`) | forwarded code | [x] |
| 132 | `LZ4F_write` | `lz4fWrite == NULL \|\| buf == NULL` (`lz4file.c:288`) | `(size_t)-21` | [x] |
| 133 | `LZ4F_write` | inner `LZ4F_compressUpdate` error, latched into `errCode` (`lz4file.c:300`) | forwarded code | [x] |
| 134 | `LZ4F_write` | `fwrite` short write, latches `errCode = -io_write` (`lz4file.c:305`) | `(size_t)-22` | [x] |
| 135 | `LZ4F_writeClose` | `lz4fWrite == NULL` (`lz4file.c:321`) | `(size_t)-21` | [x] |
| 136 | `LZ4F_writeClose` | footer `fwrite` short-writes (state still freed) (`lz4file.c:333`) | `(size_t)-22` | [x] |
| 137 | `LZ4F_writeClose` | inner `LZ4F_compressEnd` fails (state still freed) (`lz4file.c:329`) | forwarded code | [x] |
| 138 | `LZ4_XXH32_update` | `input == NULL` with `len != 0` (`xxhash.c:454`) | `XXH_ERROR` = `1` | [x] |
| 139 | `LZ4_XXH64_update` | `input == NULL` with `len != 0` (`xxhash.c:914`) | `XXH_ERROR` = `1` | [x] |
| 140 | `LZ4_XXH32_createState` / `LZ4_XXH64_createState` | `XXH_malloc` failure (`xxhash.c:422`, `884`) | `NULL` | [x] |
| 141 | `LZ4_XXH32_freeState` / `LZ4_XXH64_freeState` | `statePtr == NULL` | `XXH_OK` = `0` | [x] |
| 142 | `LZ4F_isError` / `LZ4F_getErrorCode` / `LZ4F_getErrorName` | every code 0..24 plus non-error values (0, 1, 100, `SIZE_MAX-24`, `SIZE_MAX-23`) | identical classification + identical string | [x] |
| 143 | `LZ4F_getErrorName` | out-of-range enum value (`>= LZ4F_ERROR_maxCode`, e.g. 25, 1000, -1) | identical string (`"Unspecified error code"` path) | [x] |
| 144 | `LZ4F_resetDecompressionContext` | called on a mid-frame / errored dctx, then reused | state reset, subsequent decode identical | [x] |
| 145 | `LZ4_decompress_safe_partial` | `targetOutputSize > dstCapacity` → silently clamped to `dstCapacity` | clamped, not an error | [x] |
| 146 | `LZ4_decompress_safe_partial` | `targetOutputSize < 0` / `dstCapacity < 0` | matches C exactly | [x] |
| 147 | `LZ4_compress_fast` | `acceleration < 1` → clamped to 1; `> LZ4_ACCELERATION_MAX` (65537) → clamped (`lz4.c:1386`) | clamped, not an error | [x] |
| 148 | `LZ4_setCompressionLevel` | `< 1` → `LZ4HC_CLEVEL_DEFAULT` (9); `> LZ4HC_CLEVEL_MAX` (12) → 12 (`lz4hc.c:1614`) | clamped, not an error | [x] |
| 149 | `LZ4_saveDict` | `dictSize` clamped to 64 KB and to `dict->dictSize` (`lz4.c:1820`) | clamped value returned | [x] |
| 150 | `LZ4_saveDictHC` | clamp to 64 KB, `<4 → 0`, clamp to `prefixSize` (`lz4hc.c:1748`) | clamped value returned | [x] |
| 151 | `LZ4_loadDictHC` | `dictSize > 64 KB` → keep last 64 KB (`lz4hc.c:1634`) | truncated | [x] |
| 152 | `LZ4F_createCDict` | `dictSize > 64 KB` → keep last 64 KB (`lz4frame.c:546`) | truncated | [x] |
| 153 | `LZ4_compressHC2` / `LZ4_compressHC2_limitedOutput` (legacy) | `compressionLevel` outside 1..12 (0, -5, 13, 99) → clamped | matches C | [x] |
| 154 | `LZ4_compress_HC_continue` | `dstCapacity` insufficient after a dictionary load | `0` | [x] |
| 155 | `LZ4_compress_fast_continue` | `dstCapacity` insufficient mid-stream | `0` | [x] |
| 156 | all `LZ4F_*` enum params (`blockSizeID`, `blockMode`, `contentChecksum`, `frameType`, `blockChecksum`) | out-of-range int values passed across FFI (C enums accept any int) | identical result (error or silent) | [x] |

## Non-rejections deliberately recorded (must NOT become errors in Rust)

* **`assert()` is LIVE in this C build.** `c_src/CMakeLists.txt` sets no
  `CMAKE_BUILD_TYPE` and no `NDEBUG`, so `CMAKE_C_FLAGS` is empty and every
  `assert()` in `lz4.c` / `lz4hc.c` / `lz4frame.c` / `xxhash.c` compiles to a
  real check.  Verified: `nm -D --undefined-only c_src/build/liblz4.so` lists
  `U __assert_fail@GLIBC_2.2.5`.

  Consequence for testing: an input that violates an internal C precondition
  makes the C library `abort()` (SIGABRT), which is **not** a return value and
  therefore **not** part of the error surface this table describes.  Such inputs
  are excluded from the differential tests, and each exclusion is documented at
  the call site.  The known ones are:
  - `LZ4_loadDictHC(stream, dict, dictSize)` with `dictSize < 0`
    (`assert` at `lz4hc.c:1632`).
  - `LZ4_saveDictHC` on a virgin stream followed by a compression call: leaves
    `dictLimit == 0` and trips `assert(matchIndex < ipIndex)`.
  - `LZ4_compress_HC_extStateHC_fastReset(NULL, ...)`: `NULL` passes
    `LZ4_isAligned` and is then dereferenced (`lz4hc.c:1503`) — a segfault, not
    the `0` return that row 61 describes for a *misaligned* state.  Only the
    misaligned case is a real error path.
  - `srcSize == 0` must be passed with a genuinely heap-backed buffer: a
    zero-capacity Rust `Vec`'s dangling `0x1` pointer makes `iend - MFLIMIT`
    wrap in `LZ4HC_compress_optimal` (`lz4hc.c:1846`) and the parser
    dereferences it — identically in both libraries.
  - "Undersized state" for `LZ4_compress_HC_extStateHC` (row 62) is
    unreachable by construction: the size passed to `LZ4_initStreamHC` is
    `sizeof(*ctx)`, not caller-supplied, so a short buffer is a plain overflow
    rather than a detected condition.  Only `NULL` and misaligned are testable.

  The Rust translation must **not** turn any of these asserts into a defined
  error return, because the C does not define one either.
* `#error` guards (`LZ4_MEMORY_USAGE` bounds) and `LZ4F_STATIC_ASSERT` /
  `XXH_STATIC_ASSERT` are compile-time only.
* `FUZZING_BUILD_MODE_UNSAFE_FOR_PRODUCTION` is **not** defined in this build, so
  rows 95, 101, 110, 111, 115 are all active.
* **No `compressionLevel <= 0` fast-mode fall-through exists in this `lz4hc.c`.**
  `LZ4HC_getCLevelParams` remaps every level `< 1` (including `INT_MIN`) to
  `LZ4HC_CLEVEL_DEFAULT (9)` -> `hashChain` with 256 searches; it never delegates
  to `LZ4_compress_fast`.  (`lz4frame.c` is the only place where a negative level
  becomes an `acceleration`.)

## Which test covers which rows

All error-path tests load BOTH `.so` files with `libloading` and assert the
**exact** sentinel — the exact `LZ4F_*` code, the exact negative int from
`_output_error`, or the exact `NULL`/`0`/`1` — never merely "both failed".

| rows | test file | notes |
|------|-----------|-------|
| 1-14, 44-48 | `tests/block.rs` | bounds, `initStream`, `freeStream`, `loadDict`, `decoderRingBufferSize` |
| 15-43 | `tests/block.rs` | corrupt/truncated block fuzzing: ~24 000 truncations, ~4 800 bit-flip/byte-smash mutations, ~2 500 pure-junk inputs, each fed to `LZ4_decompress_safe`, `_safe` at exact capacity, `_safe_partial` at 5 targets, `_fast`, and every `usingDict` / `forceExtDict` variant. Compares the exact `_output_error` integer, which encodes how far into the input the error was found. |
| 49-70 | `tests/hc.rs` | HC `_dest_overflow` / limitedOutput branches, `initStreamHC`, misaligned state, `srcSize` range, `loadDictHC` |
| 71-115, 142-144, 156 | `tests/frame_err.rs` | includes an exhaustive FLG x BD header sweep (all 256 FLG values, both with a stale and with a recomputed header checksum), block-header oversize, block/content checksum corruption, truncation at **every** frame length, and 3 000 random-corruption + 2 000 pure-garbage iterations |
| 116-137 | `tests/file.rs` | NULL args, files shorter than 19 bytes, forwarded header errors, truncated/corrupt payloads, read-only `FILE*` (`io_write`), latched-error `writeClose` |
| 138-141 | `tests/xxh.rs` | `XXH*_update(NULL, len)`, `createState`, `freeState(NULL)` |
| 145-155 | `tests/block.rs`, `tests/hc.rs`, `tests/frame_err.rs` | the CLAMPING rows — assert both libraries clamp identically rather than erroring |

**All 156 rows pass.**  Rows 74-77, 80-81, 91, 93, 107-108, 117, 119-120, 122,
127, 129, 140, 142-143 (`allocation_failed` / `NULL` on allocator failure) are
reached through the `*_advanced` entry points with a `LZ4F_CustomMem` whose
allocator fails after a budget of *N* successful calls, swept over
`N = 0..3` — that is the only way to drive those branches from outside the
library.

Rows 8, 64, 75, 91 for the *default* allocator (plain `malloc`/`calloc`
failure) are not forced; they are covered only by asserting the success path
returns non-`NULL` in both libraries, since deliberately exhausting the process
heap is not something a differential test can do reproducibly.  This is recorded
as an explicit limitation rather than checked off silently.

See `CONFIGS.md` -> "Mutation testing" for the evidence that these tests can
actually fail: seven injected single-token mutations, six detected (the seventh
provably semantics-preserving).
