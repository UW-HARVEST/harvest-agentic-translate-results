# ERRORS.md — Error-surface table

Derived mechanically from `c_src/src/*.c` by grepping every `RETURN_ERROR`,
`RETURN_ERROR_IF`, `return 0` (sentinel-failure), `return -1`, `return NULL`,
`assert`, and every explicit range/null/min/max check on a public entry point.

`err(X)` = `(size_t)-(ptrdiff_t)LZ4F_ERROR_X` (i.e. `LZ4F_getErrorCode(r) == X`,
`LZ4F_isError(r) != 0`).

Checkbox = differential test written and passing (C `.so` result == Rust `.so` result).

## lz4.c

| #  | function | trigger (exact invalid input/condition) | expected C result | ok |
|----|----------|------------------------------------------|-------------------|----|
| 1  | `LZ4_compressBound` | `inputSize < 0` (unsigned cast > LZ4_MAX_INPUT_SIZE) | `0` | [x] |
| 2  | `LZ4_compressBound` | `inputSize > LZ4_MAX_INPUT_SIZE` (0x7E000000) | `0` | [x] |
| 3  | `LZ4_compress_default` / `_fast` | `srcSize < 0` → `(U32)srcSize > LZ4_MAX_INPUT_SIZE` | `0` | [x] |
| 4  | `LZ4_compress_default` / `_fast` | `srcSize > LZ4_MAX_INPUT_SIZE` | `0` | [x] |
| 5  | `LZ4_compress_default` / `_fast` | `dstCapacity` too small for the data (limitedOutput budget exceeded) | `0` | [x] |
| 6  | `LZ4_compress_default` / `_fast` | `srcSize == 0`, `dstCapacity == 0` (`dstCapacity <= 0` in limited mode) | `0` | [x] |
| 7  | `LZ4_compress_default` / `_fast` | `srcSize == 0`, `dstCapacity >= 1` → writes single zero token | `1` | [x] |
| 8  | `LZ4_compress_fast` | `acceleration < 1` → clamped to LZ4_ACCELERATION_DEFAULT (1) | same bytes as accel=1 | [x] |
| 9  | `LZ4_compress_fast` | `acceleration > LZ4_ACCELERATION_MAX` (65537) → clamped to MAX | same bytes as accel=65537 | [x] |
| 10 | `LZ4_compress_fast_extState` | same srcSize/dstCapacity/accel guards as #3–#9 | `0` / clamped | [x] |
| 11 | `LZ4_compress_fast_extState_fastReset` | same guards (#3–#9) | `0` / clamped | [x] |
| 12 | `LZ4_compress_destSize` | `*srcSizePtr` yields nothing storable / `targetDstSize < 1` | `0`, `*srcSizePtr` updated | [x] |
| 13 | `LZ4_compress_destSize` | `targetDstSize >= compressBound(*srcSizePtr)` → falls back to full compress | `>0`, `*srcSizePtr` unchanged | [x] |
| 14 | `LZ4_decompress_safe` | `src == NULL` | `-1` | [x] |
| 15 | `LZ4_decompress_safe` | `dstCapacity < 0` (`outputSize < 0`) | `-1` | [x] |
| 16 | `LZ4_decompress_safe` | `srcSize == 0` (empty input, non-partial) | `-1` | [x] |
| 17 | `LZ4_decompress_safe` | truncated compressed input (input ends mid-sequence) | `< 0` | [x] |
| 18 | `LZ4_decompress_safe` | corrupted token / literal length runs past input end | `< 0` | [x] |
| 19 | `LZ4_decompress_safe` | offset == 0 (invalid match offset) | `< 0` | [x] |
| 20 | `LZ4_decompress_safe` | offset > current output position (match before buffer start) | `< 0` | [x] |
| 21 | `LZ4_decompress_safe` | `dstCapacity` smaller than decoded size → output overflow | `< 0` | [x] |
| 22 | `LZ4_decompress_safe_partial` | `targetOutputSize == 0` and `dstCapacity == 0` | `0` (partialDecoding early return) | [x] |
| 23 | `LZ4_decompress_safe_partial` | `src == NULL` or `dstCapacity < 0` | `-1` | [x] |
| 24 | `LZ4_decompress_safe_partial` | `targetOutputSize > dstCapacity` → clamped to dstCapacity | same as targetOutputSize==dstCapacity | [x] |
| 25 | `LZ4_decompress_safe_partial` | corrupt input | `< 0` | [x] |
| 26 | `LZ4_decompress_fast` (deprecated) | corrupt input, output overflow (`(size_t)(oend-op) < ll`) | `-1` | [x] |
| 27 | `LZ4_decompress_fast` | match offset points before prefix/dict start | `-1` | [x] |
| 28 | `LZ4_initStream` | `buffer == NULL` | `NULL` | [x] |
| 29 | `LZ4_initStream` | `size < sizeof(LZ4_stream_t)` | `NULL` | [x] |
| 30 | `LZ4_initStream` | `buffer` misaligned w.r.t. `LZ4_stream_t_alignment()` | `NULL` | [x] |
| 31 | `LZ4_freeStream` | `streamPtr == NULL` (free on NULL supported) | `0` | [x] |
| 32 | `LZ4_freeStreamDecode` | `LZ4_stream == NULL` | `0` | [x] |
| 33 | `LZ4_loadDict` / `LZ4_loadDictSlow` | `dictSize < 4` (`HASH_UNIT`) → dict ignored | `0` | [x] |
| 34 | `LZ4_loadDict` / `LZ4_loadDictSlow` | `dictSize > 64 KB` → truncated to last 64 KB | `65536` | [x] |
| 35 | `LZ4_saveDict` | `dictSize > 64 KB` → clamped to 64 KB | `<= 65536` | [x] |
| 36 | `LZ4_saveDict` | `dictSize > stream's stored dictSize` → clamped down | stored dictSize | [x] |
| 37 | `LZ4_saveDict` | `dictSize < 0` → `(U32)dictSize > 64KB` → clamped | clamped value | [x] |
| 38 | `LZ4_setStreamDecode` | `dictSize == 0` / `dictionary == NULL` | `1` (always success) | [x] |
| 39 | `LZ4_decoderRingBufferSize` | `maxBlockSize < 0` | `0` | [x] |
| 40 | `LZ4_decoderRingBufferSize` | `maxBlockSize > LZ4_MAX_INPUT_SIZE` | `0` | [x] |
| 41 | `LZ4_decoderRingBufferSize` | `0 <= maxBlockSize < 16` → clamped to 16 | ring size for 16 | [x] |
| 42 | `LZ4_decompress_safe_continue` | corrupt/overlong input with dict state | `< 0` | [x] |
| 43 | `LZ4_decompress_safe_usingDict` | `dictSize` 0 → plain decode; corrupt input | `< 0` | [x] |
| 44 | `LZ4_decompress_safe_partial_usingDict` | corrupt input / short dstCapacity | `< 0` | [x] |
| 45 | `LZ4_compress_fast_continue` | `srcSize` too large / `dstCapacity` too small | `0` | [x] |
| 46 | `LZ4_resetStreamState` (deprecated) | C body is `LZ4_resetStream(state); return 0;` — **no** NULL/alignment guard, so NULL state is UB (faults in C too). Only the defined path is comparable. | `0` always | [x] |
| 47 | `LZ4_create` (deprecated) | always allocates; returns non-NULL | non-NULL | [x] |

## lz4hc.c

| #  | function | trigger | expected C result | ok |
|----|----------|---------|-------------------|----|
| 48 | `LZ4_compress_HC` | `compressionLevel < 1` → LZ4HC_CLEVEL_DEFAULT (9) | same bytes as level 9 | [x] |
| 49 | `LZ4_compress_HC` | `compressionLevel > LZ4HC_CLEVEL_MAX` (12) → clamped to 12 | same bytes as level 12 | [x] |
| 50 | `LZ4_compress_HC` | `srcSize > LZ4_MAX_INPUT_SIZE` or `< 0` | `0` | [x] |
| 51 | `LZ4_compress_HC` | `dstCapacity` too small | `0` | [x] |
| 52 | `LZ4_compress_HC_extStateHC` | `state == NULL` | `0` | [x] |
| 53 | `LZ4_compress_HC_extStateHC` | `state` misaligned (`!LZ4_isAligned`) | `0` | [x] |
| 54 | `LZ4_compress_HC_extStateHC_fastReset` | `state` misaligned (`!LZ4_isAligned`, alignof=8) — NOTE: a NULL state is dereferenced before any check (documented as "presumed correctly initialized"), so NULL is UB in the C too and not a testable rejection | `0` | [x] |
| 55 | `LZ4_compress_HC_destSize` | `*srcSizePtr < 0` | `0` | [x] |
| 56 | `LZ4_compress_HC_destSize` | `targetDstSize < 0` | `0` | [x] |
| 57 | `LZ4_compress_HC_destSize` | `targetDstSize < 1` (fillOutput, nothing storable) | `0` | [x] |
| 58 | `LZ4_compress_HC_destSize` | `stateHC == NULL` | `0` | [x] |
| 59 | `LZ4_initStreamHC` | `buffer == NULL` | `NULL` | [x] |
| 60 | `LZ4_initStreamHC` | `size < sizeof(LZ4_streamHC_t)` | `NULL` | [x] |
| 61 | `LZ4_initStreamHC` | `buffer` misaligned | `NULL` | [x] |
| 62 | `LZ4_freeStreamHC` | `streamHCPtr == NULL` | `0` | [x] |
| 63 | `LZ4_freeHC` (deprecated) | `LZ4HC_Data == NULL` | `0` | [x] |
| 64 | `LZ4_setCompressionLevel` | `compressionLevel < 1` → clamped to LZ4HC_CLEVEL_DEFAULT | effective level 9 | [x] |
| 65 | `LZ4_setCompressionLevel` | `compressionLevel > LZ4HC_CLEVEL_MAX` → clamped to 12 | effective level 12 | [x] |
| 66 | `LZ4_resetStreamHC_fast` | `compressionLevel` out of range → same clamping | clamped | [x] |
| 67 | `LZ4_loadDictHC` | `dictSize > 64 KB` → truncated to last 64 KB | `65536` | [x] |
| 68 | `LZ4_loadDictHC` | `dictSize < 4` → dict effectively unusable | `dictSize` | [x] |
| 69 | `LZ4_saveDictHC` | `dictSize > 64 KB` or > stored size → clamped | clamped | [x] |
| 70 | `LZ4_compress_HC_continue` | `dstCapacity` too small | `0` | [x] |
| 71 | `LZ4_compress_HC_continue_destSize` | `dstCapacity < 1` | `0` | [x] |
| 72 | `LZ4_resetStreamStateHC` (deprecated) | `state == NULL` / misaligned → `LZ4_initStreamHC` returns NULL | `1` | [x] |

## lz4frame.c

| #  | function | trigger | expected C result | ok |
|----|----------|---------|-------------------|----|
| 73 | `LZ4F_getBlockSize` | `blockSizeID` in 1..3 (`< LZ4F_max64KB`, non-zero) | `err(maxBlockSize_invalid)` | [x] |
| 74 | `LZ4F_getBlockSize` | `blockSizeID > LZ4F_max4MB` (e.g. 8, 99, negative) | `err(maxBlockSize_invalid)` | [x] |
| 75 | `LZ4F_getBlockSize` | `blockSizeID == 0` → default (max64KB) | `65536` | [x] |
| 76 | `LZ4F_compressFrame` | `dstCapacity < LZ4F_compressFrameBound(srcSize, prefs)` | `err(dstMaxSize_tooSmall)` | [x] |
| 77 | `LZ4F_compressFrame` | `prefs.frameInfo.blockSizeID` invalid (1..3 or >7) | `err(maxBlockSize_invalid)` | [x] |
| 78 | `LZ4F_compressFrameBound` | invalid `blockSizeID` → `LZ4F_getBlockSize` error propagates into arithmetic | (huge value, matched exactly) | [x] |
| 79 | `LZ4F_createCompressionContext` | `version` mismatch — C ignores version, still succeeds | `0` (OK_NoError) | [x] |
| 80 | `LZ4F_createCompressionContext_advanced` | customMem alloc returns NULL | `NULL` | [x] |
| 81 | `LZ4F_freeCompressionContext` | `cctx == NULL` (free on NULL supported) | `0` | [x] |
| 82 | `LZ4F_compressBegin` | `dstCapacity < LZ4F_HEADER_SIZE_MAX` (19) | `err(dstMaxSize_tooSmall)` | [x] |
| 83 | `LZ4F_compressBegin_internal` | `dictBuffer != NULL` and `dictSize > INT_MAX` | `err(parameter_invalid)` | [x] |
| 84 | `LZ4F_compressUpdate` | called before `LZ4F_compressBegin` (`cStage != 1`) | `err(compressionState_uninitialized)` | [x] |
| 85 | `LZ4F_compressUpdate` | called after `LZ4F_compressEnd` (`cStage == 0`) | `err(compressionState_uninitialized)` | [x] |
| 86 | `LZ4F_compressUpdate` | `dstCapacity < LZ4F_compressBound(srcSize, prefs)` (autoFlush or not) | `err(dstMaxSize_tooSmall)` | [x] |
| 87 | `LZ4F_flush` | `cStage != 1` | `err(compressionState_uninitialized)` | [x] |
| 88 | `LZ4F_flush` | `dstCapacity < tmpInSize + BHSize + BFSize` | `err(dstMaxSize_tooSmall)` | [x] |
| 89 | `LZ4F_flush` | `tmpInSize == 0` (nothing buffered) | `0` (no-op success) | [x] |
| 90 | `LZ4F_compressEnd` | `dstCapacity < 4` (no room for EndMark) | `err(dstMaxSize_tooSmall)` | [x] |
| 91 | `LZ4F_compressEnd` | contentChecksum enabled and `dstCapacity < 8` | `err(dstMaxSize_tooSmall)` | [x] |
| 92 | `LZ4F_compressEnd` | declared `contentSize != totalInSize` actually fed | `err(frameSize_wrong)` | [x] |
| 93 | `LZ4F_createDecompressionContext` | `version` mismatch — ignored, succeeds | `0` | [x] |
| 94 | `LZ4F_freeDecompressionContext` | `dctx == NULL` | `0` | [x] |
| 95 | `LZ4F_headerSize` | `src == NULL` | `err(srcPtr_wrong)` | [x] |
| 96 | `LZ4F_headerSize` | `srcSize < 5` (`LZ4F_MIN_SIZE_TO_KNOW_HEADER_LENGTH`) | `err(frameHeader_incomplete)` | [x] |
| 97 | `LZ4F_headerSize` | magic not LZ4F_MAGICNUMBER and not skippable | `err(frameType_unknown)` | [x] |
| 98 | `LZ4F_headerSize` | magic in skippable range `0x184D2A5?` | `8` | [x] |
| 99 | `LZ4F_decodeHeader` (via getFrameInfo) | `srcSize < minFHSize` (7) | `err(frameHeader_incomplete)` | [x] |
| 100| `LZ4F_decodeHeader` | bad magic | `err(frameType_unknown)` | [x] |
| 101| `LZ4F_decodeHeader` | FLG reserved bit 1 set | `err(reservedFlag_set)` | [x] |
| 102| `LZ4F_decodeHeader` | FLG version bits != 1 | `err(headerVersion_wrong)` | [x] |
| 103| `LZ4F_decodeHeader` | BD reserved bit 7 set | `err(reservedFlag_set)` | [x] |
| 104| `LZ4F_decodeHeader` | BD blockSizeID < 4 | `err(maxBlockSize_invalid)` | [x] |
| 105| `LZ4F_decodeHeader` | BD low 4 bits (reserved) non-zero | `err(reservedFlag_set)` | [x] |
| 106| `LZ4F_decodeHeader` | header checksum byte mismatch | `err(headerChecksum_invalid)` | [x] |
| 107| `LZ4F_getFrameInfo` | called mid-header (`dStage == dstage_storeFrameHeader`) | `err(frameDecoding_alreadyStarted)` | [x] |
| 108| `LZ4F_getFrameInfo` | `*srcSizePtr < headerSize` | `err(frameHeader_incomplete)` | [x] |
| 109| `LZ4F_getFrameInfo` | `srcBuffer == NULL` | `err(srcPtr_wrong)` | [x] |
| 110| `LZ4F_decompress` | `srcSize == 0` and header not yet complete | `err(frameHeader_incomplete)` (when forced) / hint | [x] |
| 111| `LZ4F_decompress` | block size field > `maxBlockSize` from header | `err(maxBlockSize_invalid)` | [x] |
| 112| `LZ4F_decompress` | blockChecksum enabled and stored CRC != computed | `err(blockChecksum_invalid)` | [x] |
| 113| `LZ4F_decompress` | corrupt compressed block → `LZ4_decompress_safe*` returns < 0 | `err(decompressionFailed)` | [x] |
| 114| `LZ4F_decompress` | uncompressed-block flag set but decoded size wrong | `err(decompressionFailed)` | [x] |
| 115| `LZ4F_decompress` | frame declared contentSize but EndMark reached early (`frameRemainingSize != 0`) | `err(frameSize_wrong)` | [x] |
| 116| `LZ4F_decompress` | contentChecksum enabled and stored xxh32 != computed | `err(contentChecksum_invalid)` | [x] |
| 117| `LZ4F_decompress` | bad magic in first bytes | `err(frameType_unknown)` | [x] |
| 118| `LZ4F_decompress` | `srcBuffer == NULL` with `*srcSizePtr == 0` → "0-size input" shortcut. NOTE: unlike `LZ4F_headerSize`, `LZ4F_decompress` has **no** NULL-src guard — it computes `src + *srcSizePtr` and dereferences, so NULL with a non-zero size is UB in the C too and is not a testable rejection. `dstBuffer == NULL` with `*dstSizePtr == 0` is explicitly allowed. | `7` (minFHSize) | [x] |
| 119| `LZ4F_decompress_usingDict` | corrupt frame with dict set | same error as no-dict corrupt | [x] |
| 120| `LZ4F_createCDict` | `dictBuffer == NULL` / alloc fail | `NULL` (when cdict alloc fails) | [x] |
| 121| `LZ4F_freeCDict` | `CDict == NULL` | no-op | [x] |
| 122| `LZ4F_compressBegin_usingCDict` | `dstCapacity < maxFHSize` | `err(dstMaxSize_tooSmall)` | [x] |
| 123| `LZ4F_isError` | code 0 | `0` | [x] |
| 124| `LZ4F_isError` | code `-(LZ4F_ERROR_maxCode-1)` … `-1` | `1` | [x] |
| 125| `LZ4F_getErrorName` | out-of-range / non-error code | `"Unspecified error code"` | [x] |
| 126| `LZ4F_getErrorCode` | non-error result | garbage/`OK_NoError`-relative value, matched exactly | [x] |
| 127| `LZ4F_compressBound` | `srcSize == 0` (flush bound) with each blockSizeID | exact bound value | [x] |

## lz4file.c

| #  | function | trigger | expected C result | ok |
|----|----------|---------|-------------------|----|
| 128| `LZ4F_readOpen` | `fp == NULL` | `err(parameter_null)` | [x] |
| 129| `LZ4F_readOpen` | `lz4fRead == NULL` | `err(parameter_null)` | [x] |
| 130| `LZ4F_readOpen` | file shorter than `LZ4F_HEADER_SIZE_MAX` (19) — the C freads exactly `sizeof(buf) == 19` bytes and rejects any short read, so even a valid but small frame cannot be opened | `err(io_read)` | [x] |
| 131| `LZ4F_readOpen` | bad magic in first 4 bytes → `LZ4F_getFrameInfo` error | `err(frameType_unknown)` | [x] |
| 132| `LZ4F_readOpen` | header blockSizeID invalid → switch default | `err(maxBlockSize_invalid)` | [x] |
| 133| `LZ4F_read` | `lz4fRead == NULL` | `err(parameter_null)` | [x] |
| 134| `LZ4F_read` | `buf == NULL` | `err(parameter_null)` | [x] |
| 135| `LZ4F_read` | truncated frame body → EOF before end (`ret == 0` mid-frame) | `err(io_read)` | [x] |
| 136| `LZ4F_read` | corrupt block → `LZ4F_decompress` error propagated | that error | [x] |
| 137| `LZ4F_read` | `size == 0` | `0` | [x] |
| 138| `LZ4F_readClose` | `lz4fRead == NULL` | `err(parameter_null)` | [x] |
| 139| `LZ4F_writeOpen` | `fp == NULL` | `err(parameter_null)` | [x] |
| 140| `LZ4F_writeOpen` | `lz4fWrite == NULL` | `err(parameter_null)` | [x] |
| 141| `LZ4F_writeOpen` | `prefsPtr->frameInfo.blockSizeID` invalid → switch default | `err(maxBlockSize_invalid)` | [x] |
| 142| `LZ4F_write` | `lz4fWrite == NULL` | `err(parameter_null)` | [x] |
| 143| `LZ4F_write` | `buf == NULL` | `err(parameter_null)` | [x] |
| 144| `LZ4F_write` | `size == 0` | `0` | [x] |
| 145| `LZ4F_writeClose` | `lz4fWrite == NULL` | `err(parameter_null)` | [x] |
| 146| `LZ4F_writeClose` | prior `errCode` set → skips compressEnd, returns stored err | stored error | [x] |

## xxhash.c (namespaced `LZ4_XXH*`)

| #  | function | trigger | expected C result | ok |
|----|----------|---------|-------------------|----|
| 147| `LZ4_XXH32` | `input == NULL` with `length == 0` | seed-only hash (no NULL guard in one-shot) | [x] |
| 148| `LZ4_XXH32_update` | `input == NULL` | `XXH_ERROR` (1) | [x] |
| 149| `LZ4_XXH64_update` | `input == NULL` | `XXH_ERROR` (1) | [x] |
| 150| `LZ4_XXH32_update` | `length == 0`, non-NULL input | `XXH_OK` (0), digest unchanged | [x] |
| 151| `LZ4_XXH64_update` | `length == 0`, non-NULL input | `XXH_OK` (0), digest unchanged | [x] |
| 152| `LZ4_XXH32_reset` / `LZ4_XXH64_reset` | any seed (no null check on statePtr in C) | `XXH_OK` | [x] |
| 153| `LZ4_XXH32_freeState` / `LZ4_XXH64_freeState` | `statePtr == NULL` (free(NULL)) | `XXH_OK` | [x] |
| 154| `LZ4_XXH32_digest` | state with 0 bytes fed | hash of empty input with seed | [x] |
| 155| `LZ4_XXH64_digest` | state with 0 bytes fed | hash of empty input with seed | [x] |
| 156| `LZ4_XXH32_hashFromCanonical` / `LZ4_XXH64_hashFromCanonical` | round-trip of any hash | original hash | [x] |

## Generic FFI boundary boundaries (not tied to one `RETURN_ERROR`)

| #  | area | trigger | expected C result | ok |
|----|------|---------|-------------------|----|
| 157| all `LZ4F_*` taking enums | out-of-range `LZ4F_blockSizeID_t` (e.g. `-1`, `3`, `8`, `255`, `1<<30`) | as rows 73/74/77 | [x] |
| 158| all `LZ4F_*` taking enums | out-of-range `LZ4F_blockMode_t` (2, 255, -1) — C masks with `_1BIT` | header FLG bit = value&1 | [x] |
| 159| all `LZ4F_*` taking enums | out-of-range `LZ4F_contentChecksum_t` (2, 255) — masked `_1BIT` | FLG bit = value&1 | [x] |
| 160| all `LZ4F_*` taking enums | out-of-range `LZ4F_blockChecksum_t` (2, 255) — masked `_1BIT` | FLG bit = value&1 | [x] |
| 161| `LZ4F_preferences_t` | non-zero `reserved[3]` — C ignores them | same output as zeroed | [x] |
| 162| `LZ4F_compressOptions_t` | non-zero `reserved[3]`; `stableSrc` 0/1/other | same output for nonzero stableSrc | [x] |
| 163| `LZ4F_decompressOptions_t` | `stableDst` / `skipChecksums` 0/1/other + reserved nonzero | matched | [x] |
| 164| `LZ4F_getVersion` / `LZ4_versionNumber` / `LZ4_versionString` | — | 100 / 10904 / "1.10.0" | [x] |
| 165| `LZ4_sizeofState*`, `LZ4_sizeofStreamState*` | — | exact byte sizes must match | [x] |

## Where each row is tested

| rows | test |
|------|------|
| 1–2 | `tests/lz4_block.rs::r001_compress_bound` |
| 3–7 | `tests/lz4_block.rs::e003_compress_default_bad_sizes` |
| 8–11 | `tests/lz4_block.rs::r006_compress_fast_accel`, `r007_compress_fast_extState`, `r008_compress_fast_extState_fastReset` |
| 12–13 | `tests/lz4_block.rs::r009_compress_destSize` |
| 14–21 | `tests/lz4_block.rs::e014_decompress_safe_bad_inputs` |
| 22–25 | `tests/lz4_block.rs::r025_decompress_safe_partial` |
| 26–27 | `tests/lz4_block.rs::r026_decompress_fast` |
| 28–30 | `tests/lz4_block.rs::e028_initStream_guards` |
| 31–32 | `tests/lz4_block.rs::e031_free_on_null` |
| 33–34 | `tests/lz4_block.rs::r015_r016_loadDict_extDict` |
| 35–37 | `tests/lz4_block.rs::r019_saveDict` |
| 38 | `tests/lz4_block.rs::r028_setStreamDecode` |
| 39–41 | `tests/lz4_block.rs::r034_decoderRingBufferSize` |
| 42–44 | `tests/lz4_block.rs::r027_r029_decode_continue`, `r030_r032_usingDict_decoders` |
| 45 | `tests/lz4_block.rs::e045_continue_tight_dst` |
| 46–47 | `tests/lz4_block.rs::r039_legacy_create_and_resetStreamState` |
| 48–51 | `tests/lz4hc.rs::r040_compress_HC_levels`, `e050_compress_HC_bad_sizes_and_tight_dst` |
| 52–54 | `tests/lz4hc.rs::e052_extStateHC_bad_state` |
| 55–58 | `tests/lz4hc.rs::r044_HC_destSize` |
| 59–61 | `tests/lz4hc.rs::e059_initStreamHC_guards` |
| 62–63 | `tests/lz4_block.rs::e031_free_on_null` |
| 64–66 | `tests/lz4hc.rs::r047_setCompressionLevel_midstream`, `r045_r046_HC_streaming` |
| 67–69 | `tests/lz4hc.rs::r049_loadDictHC`, `r051_saveDictHC` |
| 70–71 | `tests/lz4hc.rs::e070_HC_continue_tight_dst`, `r052_HC_continue_destSize` |
| 72 | `tests/lz4hc.rs::r056_resetStreamStateHC` |
| 73–75 | `tests/lz4frame_errors.rs::e073_getBlockSize_invalid` |
| 76 | `tests/lz4frame_errors.rs::e076_compressFrame_dst_too_small` |
| 77–78 | `tests/lz4frame_errors.rs::e077_compressFrame_invalid_blockSizeID`, `tests/lz4frame_valid.rs::r058_r059_bounds` |
| 79–81, 93–94 | `tests/lz4frame_errors.rs::e079_context_creation_and_free` |
| 82, 122 | `tests/lz4frame_errors.rs::e082_compressBegin_dst_too_small` |
| 83 | `tests/lz4frame_errors.rs::e083_dictSize_too_large` |
| 84–85 | `tests/lz4frame_errors.rs::e084_update_without_begin` |
| 86–91 | `tests/lz4frame_errors.rs::e086_update_flush_end_capacity` |
| 92 | `tests/lz4frame_errors.rs::e092_compressEnd_frameSize_wrong` |
| 95–98 | `tests/lz4frame_errors.rs::e095_headerSize_rejections` |
| 99–106 | `tests/lz4frame_errors.rs::e099_header_decode_rejections` |
| 107–108 | `tests/lz4frame_errors.rs::e107_getFrameInfo_alreadyStarted` |
| 109 | `tests/lz4frame_errors.rs::e109_getFrameInfo_null_src` |
| 110–111 | `tests/lz4frame_errors.rs::e111_block_size_too_large` |
| 112 | `tests/lz4frame_errors.rs::e112_block_checksum_invalid` |
| 113–114 | `tests/lz4frame_errors.rs::e113_decompression_failed` |
| 115–116 | `tests/lz4frame_errors.rs::e115_e116_frameSize_and_contentChecksum` |
| 117–118 | `tests/lz4frame_errors.rs::e117_e118_decompress_bad_src` |
| 119 | `tests/lz4frame_errors.rs::e119_decompress_usingDict_corrupt` |
| 120–121 | `tests/lz4frame_errors.rs::e120_cdict_edge_cases` |
| 123–126 | `tests/lz4frame_errors.rs::e123_error_helpers` |
| 127 | `tests/lz4frame_errors.rs::e127_compressBound_zero` |
| 128–133, 138–140, 142–143, 145 | `tests/lz4file.rs::e128_null_arguments`, `e133_read_null_buffer` |
| 134–136 | `tests/lz4file.rs::e130_readOpen_short_and_bad_files`, `e134_write_oversized_chunks` |
| 137, 144 | `tests/lz4file.rs::e137_read_write_size_zero` |
| 141 | `tests/lz4file.rs::e141_writeOpen_invalid_blockSizeID` |
| 146 | `tests/lz4file.rs::e146_writeClose_after_error` |
| 147–156 | `tests/xxhash.rs` (`r099_r100_oneshot`, `e148_update_null_and_zero`, `e153_freeState_null`, `r101_r102_streaming`, `r104_canonical`) |
| 157–160 | `tests/lz4frame_errors.rs::e157_out_of_range_enums` |
| 161–163 | `tests/lz4frame_errors.rs::e161_reserved_fields_ignored` |
| 164 | `tests/lz4frame_errors.rs::e164_version_constants` |
| 165 | `tests/lz4_block.rs::r011_sizeof_and_versions` |

## Divergence found and fixed

**`LZ4F_*` enum-typed struct fields were signed in the Rust translation.**

`LZ4F_blockSizeID_t`, `LZ4F_blockMode_t`, `LZ4F_contentChecksum_t`,
`LZ4F_blockChecksum_t` and `LZ4F_frameType_t` are C enums whose enumerators are
all non-negative, so GCC gives them an **unsigned** 32-bit underlying type
(confirmed empirically: `(LZ4F_contentChecksum_t)-1 < 0` is false, and
`(size_t)(LZ4F_contentChecksum_t)-1 == 4294967295`).

`translation/src/lz4frame.rs` declared the corresponding `LZ4F_frameInfo_t`
fields as `c_int` and widened them with `as usize`, which **sign-extends**.
With `contentChecksumFlag = i32::MIN`, `LZ4F_compressFrameBound` therefore
returned `0xFFFFFFFE_00007563` in Rust versus `0x00000002_00007563` in C — a
difference of 2^34 in the reported bound. The same signedness also changed the
`while (requestedBSID > proposedBSID)` loop in `LZ4F_optimalBSID` for negative
`blockSizeID` values.

Fix: the five enum-typed fields, the matching constants, `LZ4F_getBlockSize`,
`LZ4F_optimalBSID`, `LZ4F_initStream_internal`, `LZ4F_selectCompression` and
`LZ4F_makeBlock`'s `crcFlag` now use `c_uint`, matching the C ABI. Caught by
`tests/lz4frame_errors.rs::e157_out_of_range_enums`.
