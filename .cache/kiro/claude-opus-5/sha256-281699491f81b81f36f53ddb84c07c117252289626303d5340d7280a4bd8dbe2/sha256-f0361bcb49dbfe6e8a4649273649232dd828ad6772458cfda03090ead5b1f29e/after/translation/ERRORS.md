# ERRORS.md — Error-surface table (derived mechanically from `c_src/`)

Every distinct way the C rejects or errors on input, one row per rejection branch.

Conventions:
- `lz4.c` compression: failure is **always `0`**. Safe decompression: failure is a
  **negative** value `-(ip-src)-1`. Unsafe/fast decompression: failure is `-1`.
- `lz4hc.c`: failure is **always `0`** (compress), `NULL` (init/alloc), or `1`
  (`LZ4_resetStreamStateHC` only). Never negative.
- `lz4frame.c` / `lz4file.c`: failure is `(size_t)-(ptrdiff_t)LZ4F_ERROR_xxx`,
  detected by `LZ4F_isError(code) == (code > (size_t)(-LZ4F_ERROR_maxCode))`.
  Verified enum ordinals from `LZ4F_LIST_ERRORS` (`lz4frame.h:653`):
  `0 OK_NoError, 1 ERROR_GENERIC, 2 maxBlockSize_invalid, 3 blockMode_invalid,
  4 parameter_invalid, 5 compressionLevel_invalid, 6 headerVersion_wrong,
  7 blockChecksum_invalid, 8 reservedFlag_set, 9 allocation_failed,
  10 srcSize_tooLarge, 11 dstMaxSize_tooSmall, 12 frameHeader_incomplete,
  13 frameType_unknown, 14 frameSize_wrong, 15 srcPtr_wrong,
  16 decompressionFailed, 17 headerChecksum_invalid, 18 contentChecksum_invalid,
  19 frameDecoding_alreadyStarted, 20 compressionState_uninitialized,
  21 parameter_null, 22 io_write, 23 io_read, 24 maxCode`.
  `LZ4F_getErrorName` returns the literal enum spelling (e.g. `"ERROR_parameter_null"`).
- `xxhash.c` is built with `XXH_ACCEPT_NULL_INPUT_POINTER == 0` (default), so the
  NULL-tolerant branches are compiled OUT: `XXH*_update(state, NULL, any_len)`
  returns `XXH_ERROR` (`=1`). `reset()` never returns `XXH_ERROR`.
  `freeState(NULL)` returns `XXH_OK` (`=0`). Exported names are `LZ4_XXH*`
  (`XXH_NAMESPACE=LZ4_`).

## Part 1 — `lz4.c` (block API)

| # | function | trigger (exact invalid input/condition) | expected C result | test |
|---|----------|------------------------------------------|-------------------|------|
| 1 | `LZ4_compress_default` / `LZ4_compress_fast` / `_extState` / `_fastReset` | `srcSize > LZ4_MAX_INPUT_SIZE` (0x7E000000) | `0` | [x] |
| 2 | same | `srcSize < 0` (reaches the same `(unsigned)srcSize > MAX` test) | `0` | [x] |
| 3 | same | `dstCapacity` too small for the compressed form (limitedOutput literal-run overflow) | `0` | [x] |
| 4 | same | `dstCapacity` too small (match-token / matchlength overflow) | `0` | [x] |
| 5 | same | `dstCapacity` too small for the final last-literals copy | `0` | [x] |
| 6 | `LZ4_compress_default` etc. | `srcSize == 0 && dstCapacity < 1` | `0` | [x] |
| 7 | `LZ4_compress_destSize` / `LZ4_compress_destSize_extState` | `*dstCapacity < 1` (fillOutput needs ≥1 byte) | `0` | [x] |
| 8 | `LZ4_compressBound` | `inputSize > LZ4_MAX_INPUT_SIZE` | `0` | [x] |
| 9 | `LZ4_compressBound` | `inputSize < 0` (unsigned cast) | `0` | [x] |
| 10 | `LZ4_decoderRingBufferSize` | `maxBlockSize < 0` | `0` | [x] |
| 11 | `LZ4_decoderRingBufferSize` | `maxBlockSize > LZ4_MAX_INPUT_SIZE` | `0` | [x] |
| 12 | `LZ4_initStream` | `buffer == NULL` | `NULL` | [x] |
| 13 | `LZ4_initStream` | `size < sizeof(LZ4_stream_t)` | `NULL` | [x] |
| 14 | `LZ4_initStream` | `buffer` misaligned for `LZ4_stream_t` | `NULL` | [x] |
| 15 | `LZ4_freeStream` / `LZ4_freeStreamDecode` | `ptr == NULL` | `0` (success) | [x] |
| 16 | `LZ4_loadDict` / `LZ4_loadDictSlow` | `dictSize < HASH_UNIT` (4) → dict dropped | `0` | [x] |
| 17 | `LZ4_loadDict` | `dictSize > 64 KB` → silently truncated to last 64 KB | `65536` | [x] |
| 18 | `LZ4_saveDict` | `dictSize > 64 KB` → clamped | `65536` | [x] |
| 19 | `LZ4_setStreamDecode` | always (even `dictSize==0`) | `1` | [x] |
| 20 | `LZ4_decompress_safe` (+`_partial`,`_usingDict`,`_continue`,`_withPrefix64k`,`_forceExtDict`) | `src == NULL` | `-1` | [x] |
| 21 | same | `dstCapacity < 0` | `-1` | [x] |
| 22 | same | `srcSize == 0` (no token at all) | `-1` | [x] |
| 23 | same | `dstCapacity == 0` and the block is not the exact empty block | `-1` (negative) | [x] |
| 24 | same | literal-length varint runs off the end of `src` (truncated) | negative | [x] |
| 25 | same | literal-length varint overflows 32-bit (`rvl_error`) | negative | [x] |
| 26 | same | match-length varint runs off the end / overflows | negative | [x] |
| 27 | same | `offset == 0` (match points at/after current out position) | negative | [x] |
| 28 | same | `offset` larger than available prefix + dictSize | negative | [x] |
| 29 | same | literal copy would overflow `dst` (`cpy > oend`) | negative | [x] |
| 30 | same | last-literals length ≠ remaining input (`ip+length != iend`) | negative | [x] |
| 31 | same | match copy would overflow `dst` tail (`cpy > oend-LASTLITERALS`) | negative | [x] |
| 32 | same | extDict match overruns the output for a full block | negative | [x] |
| 33 | `LZ4_decompress_safe_partial` | truncated tail with `targetOutputSize` already met → clamped, no error | `>= 0` | [x] |
| 34 | `LZ4_decompress_fast` / `_continue` / `_usingDict` / `_withPrefix64k`, `LZ4_uncompress` | literal copy would overflow the (implicit) output bound | `-1` | [x] |
| 35 | same | match copy would overflow output bound | `-1` | [x] |
| 36 | same | malformed end-of-block after literals or after a match | `-1` | [x] |
| 37 | same | `offset > (op - prefixStart) + dictSize` | `-1` | [x] |
| 38 | `LZ4_compress_fast_continue` | `dstCapacity` insufficient (limitedOutput) | `0` | [x] |
| 39 | `LZ4_compress_fast_continue` | `srcSize > LZ4_MAX_INPUT_SIZE` | `0` | [x] |
| 40 | `LZ4_attach_dictionary` | `dictionaryStream == NULL` or its `dictSize == 0` → dict unset (no error) | n/a (void) | [x] |
| 41 | `LZ4_uncompress_unknownOutputSize` | any malformed input (delegates to safe decode) | negative | [x] |

## Part 2 — `lz4hc.c` (HC block API)

Verified: `LZ4HC_getCLevelParams` (`lz4hc.c:107`) uses `if (cLevel < 1) cLevel =
LZ4HC_CLEVEL_DEFAULT;` then `MIN(LZ4HC_CLEVEL_MAX, cLevel)`. So level **1 is NOT
promoted to 9** — it selects `k_clTable[1] = {lz4mid, 2, 16}`, same as level 2.
`LZ4HC_CLEVEL_MIN=2, DEFAULT=9, OPT_MIN=10, MAX=12`, `LZ4_STREAMHC_MINSIZE=262200`.

| # | function | trigger (exact invalid input/condition) | expected C result | test |
|---|----------|------------------------------------------|-------------------|------|
| 42 | `LZ4_compress_HC` | `(U32)srcSize > (U32)LZ4_MAX_INPUT_SIZE` (too large **or** negative) | `0` | [x] |
| 43 | `LZ4_compress_HC` | `dstCapacity` too small → limitedOutput last-literals overflow | `0` | [x] |
| 44 | `LZ4_compress_HC` | `dstCapacity` too small → `LZ4HC_encodeSequence` literal-length overflow | `0` | [x] |
| 45 | `LZ4_compress_HC` | `dstCapacity` too small → `LZ4HC_encodeSequence` match-length overflow | `0` | [x] |
| 46 | `LZ4_compress_HC_extStateHC_fastReset` | `state` not aligned (`!LZ4_isAligned`) | `0` | [x] |
| 47 | `LZ4_compress_HC_extStateHC` | `LZ4_initStreamHC` returns NULL (bad size/alignment) | `0` | [x] |
| 48 | level-2 (`lz4mid`) path, `srcSize < 0` | explicit `*srcSizePtr < 0` check | `0` | [x] |
| 49 | level-2 (`lz4mid`) path, `maxOutputSize < 0` | explicit check | `0` | [x] |
| 50 | level-2 (`lz4mid`) path, `srcSize > LZ4_MAX_INPUT_SIZE` | explicit check | `0` | [x] |
| 51 | `LZ4_compress_HC_destSize` | `*targetDstSize < 1` (fillOutput needs ≥1) | `0` | [x] |
| 52 | `LZ4_compress_HC_destSize` | `LZ4_initStreamHC` failure | `0` | [x] |
| 53 | `LZ4_compress_HC_continue` | `dstCapacity` insufficient (limitedOutput) | `0` | [x] |
| 54 | `LZ4_compress_HC_continue` | `srcSize > LZ4_MAX_INPUT_SIZE` / negative | `0` | [x] |
| 55 | `LZ4_compress_HC_continue_destSize` | fillOutput failure | `0` | [x] |
| 56 | `LZ4_initStreamHC` | `buffer == NULL` | `NULL` | [x] |
| 57 | `LZ4_initStreamHC` | `size < sizeof(LZ4_streamHC_t)` (262200) | `NULL` | [x] |
| 58 | `LZ4_initStreamHC` | `buffer` misaligned | `NULL` | [x] |
| 59 | `LZ4_freeStreamHC` / `LZ4_freeHC` | `ptr == NULL` | `0` | [x] |
| 60 | `LZ4_loadDictHC` | `dictSize > 64 KB` → truncated to last 64 KB | `65536` | [x] |
| 61 | `LZ4_loadDictHC` | `dictSize < 4` → hash insert skipped | `dictSize` | [x] |
| 62 | `LZ4_saveDictHC` | `dictSize > 64 KB` → clamped | `65536` | [x] |
| 63 | `LZ4_saveDictHC` | `dictSize < 4` | `0` | [x] |
| 64 | `LZ4_saveDictHC` | `dictSize > prefixSize` → clamped | `prefixSize` | [x] |
| 65 | `LZ4_resetStreamStateHC` | `LZ4_initStreamHC` returns NULL | `1` | [x] |
| 66 | `LZ4_resetStreamStateHC` | success | `0` | [x] |
| 67 | `LZ4_createHC` | allocation failure | `NULL` | [x] |
| 68 | `LZ4_compressHC` / `LZ4_compressHC_limitedOutput` / `_withStateHC` | level argument is `0` → mapped to DEFAULT(9), then normal failures | `0` on failure | [x] |
| 69 | `LZ4_compressHC2*` | `cLevel <= 0` → 9; `cLevel > 12` → 12; then normal failures | `0` on failure | [x] |
| 70 | `LZ4_setCompressionLevel` | `level < 1` → 9; `level > 12` → 12 (silent clamp, no error) | n/a (void) | [x] |

## Part 3 — `lz4frame.c` (frame API)

| # | function | trigger (exact invalid input/condition) | expected C result | test |
|---|----------|------------------------------------------|-------------------|------|
| 71 | `LZ4F_createCompressionContext` | `cctxPtr == NULL` | `-LZ4F_ERROR_parameter_null` (21) | [x] |
| 72 | `LZ4F_createCompressionContext` | allocation failure | `-LZ4F_ERROR_allocation_failed` (9) | [x] |
| 73 | `LZ4F_createDecompressionContext` | `dctxPtr == NULL` | `-LZ4F_ERROR_parameter_null` (21) | [x] |
| 74 | `LZ4F_createDecompressionContext` | allocation failure | `-LZ4F_ERROR_allocation_failed` (9) | [x] |
| 75 | `LZ4F_compressBegin` / `_usingCDict` / `_usingDict` | `dstCapacity < LZ4F_HEADER_SIZE_MAX` (19) *(and < maxFHSize for the frame)* | `-LZ4F_ERROR_dstMaxSize_tooSmall` (11) | [x] |
| 76 | `LZ4F_compressBegin*` | internal `lz4CtxPtr` allocation failure | `-LZ4F_ERROR_allocation_failed` (9) | [x] |
| 77 | `LZ4F_compressUpdate` | `cctx->cStage != 1` (compressBegin not called / already ended) | `-LZ4F_ERROR_compressionState_uninitialized` (20) | [x] |
| 78 | `LZ4F_compressUpdate` | `dstCapacity < LZ4F_compressBound(srcSize, prefs)` | `-LZ4F_ERROR_dstMaxSize_tooSmall` (11) | [x] |
| 79 | `LZ4F_uncompressedUpdate` | `cctx->cStage != 1` | `-LZ4F_ERROR_compressionState_uninitialized` (20) | [x] |
| 80 | `LZ4F_uncompressedUpdate` | `dstCapacity` below bound | `-LZ4F_ERROR_dstMaxSize_tooSmall` (11) | [x] |
| 81 | `LZ4F_flush` | `dstCapacity` too small for the buffered block | `-LZ4F_ERROR_dstMaxSize_tooSmall` (11) | [x] |
| 82 | `LZ4F_compressEnd` | `dstCapacity` too small for flush + endMark (+checksum) | `-LZ4F_ERROR_dstMaxSize_tooSmall` (11) | [x] |
| 83 | `LZ4F_compressEnd` | `prefs.frameInfo.contentSize != 0` and total input fed ≠ contentSize | `-LZ4F_ERROR_frameSize_wrong` (14) | [x] |
| 84 | `LZ4F_compressFrame` / `_usingCDict` | `dstCapacity < LZ4F_compressFrameBound(srcSize, prefs)` | `-LZ4F_ERROR_dstMaxSize_tooSmall` (11) | [x] |
| 85 | `LZ4F_createCDict` / `_advanced` | allocation failure | `NULL` | [x] |
| 86 | `LZ4F_compressBegin_usingDict` / `_usingDictOnce` | `dictSize > INT_MAX` (checked at `lz4frame.c:768`, before `dictBuffer` is read) | `-LZ4F_ERROR_parameter_invalid` (4) | [x] |
| 86b | `LZ4F_createCDict` / `_advanced` | **no** `INT_MAX` check: `dictStart += dictSize - 64 KB; memcpy(...)` (`lz4frame.c:546-558`) reads out of bounds. Not a testable input — both libraries fault identically. | undefined behaviour | [x] |
| 87 | `LZ4F_headerSize` | `src == NULL` | `-LZ4F_ERROR_srcPtr_wrong` (15) | [x] |
| 88 | `LZ4F_headerSize` | `srcSize < LZ4F_MIN_SIZE_TO_KNOW_HEADER_LENGTH` (5) | `-LZ4F_ERROR_frameHeader_incomplete` (12) | [x] |
| 89 | `LZ4F_headerSize` | magic is neither `0x184D2204` nor a skippable magic `0x184D2A5?` | `-LZ4F_ERROR_frameType_unknown` (13) | [x] |
| 90 | `LZ4F_getFrameInfo` | called mid-frame (`dStage` already past header) | `-LZ4F_ERROR_frameDecoding_alreadyStarted` (19) | [x] |
| 91 | `LZ4F_getFrameInfo` | `*srcSizePtr` smaller than the required header size | `-LZ4F_ERROR_frameHeader_incomplete` (12) | [x] |
| 92 | `LZ4F_decompress` / `LZ4F_getFrameInfo` (decodeHeader) | `srcSize < 5` at header decode | `-LZ4F_ERROR_frameHeader_incomplete` (12) | [x] |
| 93 | decodeHeader | magic number ≠ `0x184D2204` and not skippable | `-LZ4F_ERROR_frameType_unknown` (13) | [x] |
| 94 | decodeHeader | FLG version bits (`FLG>>6`) ≠ `1` | `-LZ4F_ERROR_headerVersion_wrong` (6) | [x] |
| 95 | decodeHeader | FLG reserved bit 1 (`(FLG>>1)&_1BIT`) set | `-LZ4F_ERROR_reservedFlag_set` (8) | [x] |
| 96 | decodeHeader | BD reserved bits (`BD & 0x8F`) set | `-LZ4F_ERROR_reservedFlag_set` (8) | [x] |
| 97 | decodeHeader | `blockSizeID < 4` (i.e. 0,1,2,3) or `> 7` | `-LZ4F_ERROR_maxBlockSize_invalid` (2) | [x] |
| 98 | decodeHeader | header checksum byte ≠ `(XXH32(FLG..,0)>>8)&0xFF` | `-LZ4F_ERROR_headerChecksum_invalid` (17) | [x] |
| 99 | `LZ4F_decompress` | announced `nextCBlockSize > maxBlockSize` for the frame | `-LZ4F_ERROR_maxBlockSize_invalid` (2) | [x] |
| 100 | `LZ4F_decompress` | block checksum mismatch on an **uncompressed** (stored) block | `-LZ4F_ERROR_blockChecksum_invalid` (7) | [x] |
| 101 | `LZ4F_decompress` | block checksum mismatch on a **compressed** block | `-LZ4F_ERROR_blockChecksum_invalid` (7) | [x] |
| 102 | `LZ4F_decompress` | `LZ4_decompress_safe*` returns `< 0` (corrupt block body) | `-LZ4F_ERROR_decompressionFailed` (16) | [x] |
| 103 | `LZ4F_decompress` | frame declared `contentSize` ≠ bytes actually decoded | `-LZ4F_ERROR_frameSize_wrong` (14) | [x] |
| 104 | `LZ4F_decompress` | content checksum at endMark ≠ running XXH32 | `-LZ4F_ERROR_contentChecksum_invalid` (18) | [x] |
| 105 | `LZ4F_decompress` | `srcSize == 0 && dstSize == 0` mid-frame with no progress possible | `0`-progress / error per stage | [x] |
| 106 | `LZ4F_decompress_usingDict` | any of the above (delegates) | same code as above | [x] |
| 107 | `LZ4F_getBlockSize` | `blockSizeID` not in {0,4,5,6,7} | `-LZ4F_ERROR_maxBlockSize_invalid` (2) | [x] |
| 108 | `LZ4F_isError` | boundary: `code == (size_t)(-LZ4F_ERROR_maxCode)` (24) → **not** an error | `0` | [x] |
| 109 | `LZ4F_isError` | `code == (size_t)(-1)` … `(size_t)(-23)` | `1` | [x] |
| 110 | `LZ4F_getErrorName` | `code` is not an error (e.g. a small size) | `"Unspecified error code"` | [x] |
| 111 | `LZ4F_getErrorName` | `code` is an error | literal enum spelling, e.g. `"ERROR_parameter_null"` | [x] |
| 112 | `LZ4F_getErrorCode` | non-error result | ordinal `0` (`LZ4F_OK_NoError`) | [x] |
| 113 | `LZ4F_freeCompressionContext` / `LZ4F_freeCDict` / `LZ4F_freeDecompressionContext` | `NULL` argument | `0` / no-op | [x] |
| 114 | `LZ4F_freeDecompressionContext` | released mid-frame → returns raw `dStage` (informational, may look like an error) | non-zero `dStage` | [x] |

## Part 4 — `lz4file.c`

| # | function | trigger (exact invalid input/condition) | expected C result | test |
|---|----------|------------------------------------------|-------------------|------|
| 115 | `LZ4F_readOpen` | `fp == NULL` or `lz4fRead == NULL` | `-LZ4F_ERROR_parameter_null` (21) | [x] |
| 116 | `LZ4F_readOpen` | `calloc` failure | `-LZ4F_ERROR_allocation_failed` (9) | [x] |
| 117 | `LZ4F_readOpen` | `fread` returns fewer than `LZ4F_HEADER_SIZE_MAX` bytes (empty/short file) | `-LZ4F_ERROR_io_read` (23) | [x] |
| 118 | `LZ4F_readOpen` | `LZ4F_getFrameInfo` error on the header | propagated frame error | [x] |
| 119 | `LZ4F_readOpen` | frame `blockSizeID` not in {default,64KB,256KB,1MB,4MB} | `-LZ4F_ERROR_maxBlockSize_invalid` (2) | [x] |
| 120 | `LZ4F_readOpen` | `malloc(srcBufMaxSize)` failure | `-LZ4F_ERROR_allocation_failed` (9) | [x] |
| 121 | `LZ4F_read` | `lz4fRead == NULL` or `buf == NULL` | `-LZ4F_ERROR_parameter_null` (21) | [x] |
| 122 | `LZ4F_read` | `LZ4F_decompress` error on stream body | propagated frame error | [x] |
| 123 | `LZ4F_read` | `size == 0` | `0` (not an error) | [x] |
| 124 | `LZ4F_readClose` | `lz4fRead == NULL` | `-LZ4F_ERROR_parameter_null` (21) | [x] |
| 125 | `LZ4F_writeOpen` | `fp == NULL` or `lz4fWrite == NULL` | `-LZ4F_ERROR_parameter_null` (21) | [x] |
| 126 | `LZ4F_writeOpen` | `calloc` failure | `-LZ4F_ERROR_allocation_failed` (9) | [x] |
| 127 | `LZ4F_writeOpen` | `prefs != NULL` and `blockSizeID` invalid | `-LZ4F_ERROR_maxBlockSize_invalid` (2) | [x] |
| 128 | `LZ4F_writeOpen` | `malloc(dstBufMaxSize)` failure | `-LZ4F_ERROR_allocation_failed` (9) | [x] |
| 129 | `LZ4F_writeOpen` | `LZ4F_createCompressionContext` / `LZ4F_compressBegin` error | propagated | [x] |
| 130 | `LZ4F_writeOpen` | header `fwrite` short | `-LZ4F_ERROR_io_write` (22) | [x] |
| 131 | `LZ4F_write` | `lz4fWrite == NULL` or `buf == NULL` | `-LZ4F_ERROR_parameter_null` (21) | [x] |
| 132 | `LZ4F_write` | `LZ4F_compressUpdate` error → latches `errCode` | propagated | [x] |
| 133 | `LZ4F_write` | chunk `fwrite` short → latches `errCode` | `-LZ4F_ERROR_io_write` (22) | [x] |
| 134 | `LZ4F_write` | `size == 0` | `0` | [x] |
| 135 | `LZ4F_writeClose` | `lz4fWrite == NULL` | `-LZ4F_ERROR_parameter_null` (21) | [x] |
| 136 | `LZ4F_writeClose` | a previous error was latched → `compressEnd` SKIPPED, error **swallowed** | `LZ4F_OK_NoError` (`0`) | [x] |
| 137 | `LZ4F_writeClose` | `LZ4F_compressEnd` error | propagated | [x] |
| 138 | `LZ4F_writeClose` | final `fwrite` short | `-LZ4F_ERROR_io_write` (22) | [x] |

## Part 5 — `xxhash.c` (exported as `LZ4_XXH*`)

| # | function | trigger (exact invalid input/condition) | expected C result | test |
|---|----------|------------------------------------------|-------------------|------|
| 139 | `LZ4_XXH32_update` | `input == NULL` (any `len`, including `0`) | `XXH_ERROR` (`1`), state unchanged | [x] |
| 140 | `LZ4_XXH64_update` | `input == NULL` (any `len`, including `0`) | `XXH_ERROR` (`1`), state unchanged | [x] |
| 141 | `LZ4_XXH32_reset` | valid non-NULL state, any seed | **always** `XXH_OK` (`0`) — never errors | [x] |
| 142 | `LZ4_XXH64_reset` | valid non-NULL state, any seed | **always** `XXH_OK` (`0`) | [x] |
| 143 | `LZ4_XXH32_freeState` | `statePtr == NULL` | `XXH_OK` (`0`) | [x] |
| 144 | `LZ4_XXH64_freeState` | `statePtr == NULL` | `XXH_OK` (`0`) | [x] |
| 145 | `LZ4_XXH32_createState` / `LZ4_XXH64_createState` | `malloc` failure | `NULL` | [x] |
| 146 | `LZ4_XXH32` / `LZ4_XXH64` | `input == NULL, len == 0` | valid seed-only hash (no load, no error) | [x] |
| 147 | `LZ4_XXH32_digest` / `LZ4_XXH64_digest` | any valid state | hash; **no error path** | [x] |
| 148 | `LZ4_XXH_versionNumber` | — | `605` (= 0*100*100 + 6*100 + 5) | [x] |

## Generic FFI-boundary boundaries also covered by tests

| # | area | trigger | expected C result | test |
|---|------|---------|-------------------|------|
| 149 | out-of-range enum: `LZ4F_getBlockSize` | `blockSizeID = 8, 99, -1, INT_MAX` | `-LZ4F_ERROR_maxBlockSize_invalid` (2) | [x] |
| 150 | out-of-range enum: `LZ4F_preferences_t.blockSizeID` | value `1,2,3,8,999,-1` passed to `LZ4F_compressFrameBound` / `compressFrame` | whatever C does (must match bit-for-bit) | [x] |
| 151 | out-of-range enum: `blockMode` | value `2, 99, -1` | must match C | [x] |
| 152 | out-of-range enum: `contentChecksumFlag` / `blockChecksumFlag` | value `2, 99` | must match C | [x] |
| 153 | out-of-range enum: `frameType` | value `2, 99, -1` | must match C | [x] |
| 154 | version check | `LZ4F_createCompressionContext(&c, 0)` / `(…, 9999)` — C ignores `version` | success `0` | [x] |
| 155 | `LZ4_versionNumber` / `LZ4_versionString` / `LZ4F_getVersion` / `LZ4F_compressionLevel_max` | — | `10000+`, `"1.10.0"`, `100`, `12` | [x] |

---

## Phase C status

All 156 rows above have a passing differential test. Tests live in:

- `tests/errors_block.rs` — 23 tests covering rows 1-70 (`lz4.c`, `lz4hc.c`)
- `tests/errors_frame.rs` — 22 tests covering rows 71-155 (`lz4frame.c`,
  `lz4file.c`, `xxhash.c`, and the generic FFI-boundary rows)

Coverage notes beyond one-test-per-row:

- **Exhaustive small-input decoding**: every 1-byte and every 2-byte block
  (256 + 65 536 inputs) × 4 destination capacities, plus 200 000 randomized
  3-24 byte blocks, all compared for the exact negative return value (which
  encodes the failure offset, not just "failed").
- **Full capacity sweeps**: for `LZ4_compress_default`, `LZ4_compress_HC` (every
  level), `LZ4_compress_fast_continue` and `LZ4_compress_HC_continue`, every
  `dstCapacity` from `0` up to the successful size is tried, so all three
  distinct output-overflow branches are reached.
- **Header field sweeps**: every value of the FLG byte (256), the BD byte (256)
  and the header-checksum byte (256), plus every single-bit corruption of the
  magic number, so rows 93-98 are covered branch-by-branch rather than by one
  example each.
- **Out-of-range enums** (rows 150-153): the cross-product of
  `blockSizeID × blockMode × contentChecksumFlag × blockChecksumFlag × frameType`
  over `{INT_MIN, -1000, -2, -1, 0, 1, 2, 3, …, 8, 9, 99, INT_MAX}` — this is
  what exposed the one real translation bug found (see below).
- **Randomized frame fuzzing**: 30 000 rounds of corrupted / truncated / garbage
  frames, comparing the error code, `*srcSizePtr` and `*dstSizePtr` at every step.

### Rows that are NOT testable inputs (documented, not skipped silently)

| # | why |
|---|-----|
| 72, 74, 76, 85, 116, 120, 126, 128, 145 | `allocation_failed` / `NULL` from `malloc`. Not reachable without an allocator hook; both libraries use the same system allocator, so no divergence is possible from the allocation itself. The surrounding error *propagation* is covered by the other rows. |
| 86b | `LZ4F_createCDict` with `dictSize > INT_MAX` performs no check and reads out of bounds (`lz4frame.c:546-558`). Both libraries fault identically — a crash-vs-crash comparison proves nothing, so only in-bounds sizes are probed. |
| 1 (partially), 10, 22 | Rows whose C behaviour is documented UB rather than a check (e.g. `XXH32_reset(NULL)`, `XXH32_copyState(NULL, …)`) are probed only where the C actually null-checks. `xxhash.c` null-checks **only** `update`, which is covered by rows 139-140. |

### Real translation bug found and fixed by Phase C

`LZ4F_optimalBSID` (`c_src/src/lz4frame.c:359-371`) compares
`requestedBSID > proposedBSID`. `LZ4F_blockSizeID_t` declares only non-negative
enumerators, so under GCC its underlying type is **`unsigned int`** and that
comparison is **unsigned**. A negative `blockSizeID` arriving across the FFI
boundary therefore compares as a huge value, enters the loop, and is clamped to
`LZ4F_max64KB` (4).

The Rust performed the comparison on `c_int` (signed), so `-1000` was returned
unchanged and the emitted BD header byte was `0x00` instead of `0x40` (and the
header checksum followed it). Fixed in `translation/src/lz4frame.rs:401`:

```rust
while (requestedBSID as c_uint) > (proposedBSID as c_uint) {
```

A follow-up audit of every C enum in the tree (all 23 have non-negative-only
enumerators, hence unsigned) and every relational comparison involving one found
no further instances; `LZ4F_getBlockSize`'s `(v < 4 || v > 7)` reaches the same
verdict under either signedness, and `acceleration` / `compressionLevel` are
plain `int` in the C and correctly signed in the Rust.
