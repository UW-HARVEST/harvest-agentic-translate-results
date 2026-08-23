# ERRORS.md — Error-surface table (LZ4 v1.10.0 + xxHash 0.6.5)

Derived mechanically from `c_src/src/*.c` by grepping every `RETURN_ERROR*`,
`FORWARD_IF_ERROR`, `return 0/-1/NULL` failure return, `assert`, explicit range
check, null check, and min/max constant.

Build context that scopes this table (from `c_src/CMakeLists.txt`):
`-DXXH_NAMESPACE=LZ4_ -DLZ4_HEAPMODE=0 -DLZ4F_HEAPMODE=0`, no `-DNDEBUG`,
no `-DLZ4_DEBUG`. Therefore `assert()` in `lz4.c`/`lz4hc.c`/`lz4frame.c` expands
to `((void)0)` (LZ4's own `assert` shim) and is NOT a runtime rejection, while
`LZ4HC_HEAPMODE` defaults to 1 so `lz4hc.c` heap-allocation failures ARE compiled.

`LZ4F_errorCode_t` is `size_t`; error N is returned as `(size_t)-N`.
Ordinals: `maxBlockSize_invalid 2, blockMode_invalid 3, parameter_invalid 4,
compressionLevel_invalid 5, headerVersion_wrong 6, blockChecksum_invalid 7,
reservedFlag_set 8, allocation_failed 9, srcSize_tooLarge 10,
dstMaxSize_tooSmall 11, frameHeader_incomplete 12, frameType_unknown 13,
frameSize_wrong 14, srcPtr_wrong 15, decompressionFailed 16,
headerChecksum_invalid 17, contentChecksum_invalid 18,
frameDecoding_alreadyStarted 19, compressionState_uninitialized 20,
parameter_null 21, io_write 22, io_read 23, maxCode 24`.

Test file: `tests/errors.rs`. Every row below has a differential test that calls
BOTH the C `.so` and the Rust `.so` and asserts identical error/sentinel values.

## lz4.c — block API

| # | function | trigger (exact invalid input/condition) | expected C result |
|---|----------|------------------------------------------|-------------------|
| 1 | `LZ4_compressBound` | `isize < 0` | `0` |
| 2 | `LZ4_compressBound` | `isize > LZ4_MAX_INPUT_SIZE` (2113929216) | `0` |
| 3 | `LZ4_compressBound` | `isize == LZ4_MAX_INPUT_SIZE` (boundary, valid) | non-zero bound |
| 4 | `LZ4_compress_default` | `srcSize < 0` (checked before src is read) | `0` |
| 5 | `LZ4_compress_default` | `srcSize > LZ4_MAX_INPUT_SIZE` (checked before src is read) | `0` |
| 6 | `LZ4_compress_default` | `srcSize == 0 && dstCapacity <= 0` | `0` |
| 7 | `LZ4_compress_default` | `srcSize == 0 && dstCapacity >= 1` | `1`, and `dst[0] == 0` |
| 8 | `LZ4_compress_default` | `dstCapacity` one byte below the achievable compressed size (literal-run overflow, lz4.c:1116/1314) | `0` |
| 9 | `LZ4_compress_default` | `dstCapacity == 1` with a large incompressible src | `0` |
| 10 | `LZ4_compress_fast` | `acceleration < 1` (0, -1, INT_MIN) | clamped to 1 ⇒ byte-identical to `acceleration==1` |
| 11 | `LZ4_compress_fast` | `acceleration > LZ4_ACCELERATION_MAX` (65537) | clamped to 65537 ⇒ identical to `acceleration==65537` |
| 12 | `LZ4_compress_destSize` | `targetDstSize < 1` (0 or negative) with `*srcSizePtr > 0` | `0` |
| 13 | `LZ4_compress_destSize` | `*srcSizePtr < 0` | `0` |
| 14 | `LZ4_compress_destSize` | `targetDstSize` smaller than needed for all of src | partial: `*srcSizePtr` reduced, returns `<= targetDstSize` |
| 15 | `LZ4_initStream` | `buffer == NULL` | `NULL` |
| 16 | `LZ4_initStream` | `size < sizeof(LZ4_stream_t)` (16416) | `NULL` |
| 17 | `LZ4_initStream` | `buffer` not 8-byte aligned | `NULL` |
| 18 | `LZ4_initStream` | `size == 16416`, aligned (boundary, valid) | `buffer` |
| 19 | `LZ4_freeStream` | `LZ4_stream == NULL` | `0` |
| 20 | `LZ4_freeStreamDecode` | `LZ4_streamDecode == NULL` | `0` |
| 21 | `LZ4_loadDict` | `dictSize < HASH_UNIT` (8): includes 0, 1..7, negative | `0` |
| 22 | `LZ4_loadDict` | `dictSize > 65536` | `65536` (last 64 KB kept) |
| 23 | `LZ4_loadDict` | `dictSize == 8` / `== 65536` (boundaries, valid) | `8` / `65536` |
| 24 | `LZ4_loadDictSlow` | same three conditions as rows 21-23 | same as rows 21-23 |
| 25 | `LZ4_saveDict` | `dictSize > 65536` | clamped to `min(65536, stream dictSize)` |
| 26 | `LZ4_saveDict` | `dictSize < 0` (`(U32)` cast makes it `> 65536`) | clamped, `>= 0` |
| 27 | `LZ4_saveDict` | `dictSize > stream's current dictSize` | clamped to stream `dictSize` (may be `0`) |
| 28 | `LZ4_saveDict` | `dictSize == 0` | `0` |
| 29 | `LZ4_decompress_safe` | `src == NULL` | `-1` |
| 30 | `LZ4_decompress_safe` | `dstCapacity < 0` | `-1` |
| 31 | `LZ4_decompress_safe` | `dstCapacity == 0` and NOT (`srcSize==1 && src[0]==0`) | `-1` |
| 32 | `LZ4_decompress_safe` | `dstCapacity == 0`, `srcSize == 1`, `src[0] == 0` (canonical empty block) | `0` |
| 33 | `LZ4_decompress_safe` | `srcSize == 0` with `dstCapacity != 0` | `-1` |
| 34 | `LZ4_decompress_safe` | truncated extended literal-length varint (`read_variable_length` rvl_error, lz4.c:1986-2010) | negative `-(consumed)-1` |
| 35 | `LZ4_decompress_safe` | truncated extended match-length varint (lz4.c:2129/2347) | negative `-(consumed)-1` |
| 36 | `LZ4_decompress_safe` | match offset larger than available history (`match + dictSize < lowPrefix`, lz4.c:2161/2356) | negative |
| 37 | `LZ4_decompress_safe` | offset == 0 — NOT rejected: `match == op`, so `match + dictSize < lowPrefix` is false and the C self-copies | no error (bytes self-copied); only errors if it also trips `LASTLITERALS`/overflow. Both cases tested. |
| 38 | `LZ4_decompress_safe` | literals run past end of input (`ip + length != iend`, lz4.c:2312) | negative |
| 39 | `LZ4_decompress_safe` | output overflow: `cpy > oend` on literal copy | negative |
| 40 | `LZ4_decompress_safe` | last 5 bytes of block are not literals (`cpy > oend - LASTLITERALS`, lz4.c:2421) | negative |
| 41 | `LZ4_decompress_safe` | `dstCapacity` one byte less than the true decoded size | negative |
| 42 | `LZ4_decompress_safe` | random/fuzzed byte strings as `src` (all of the above mixed) | identical negative value or identical output |
| 43 | `LZ4_decompress_safe_partial` | `min(targetOutputSize, dstCapacity) == 0` | `0` |
| 44 | `LZ4_decompress_safe_partial` | `targetOutputSize < 0` or `dstCapacity < 0` | `-1` |
| 45 | `LZ4_decompress_safe_partial` | `targetOutputSize > dstCapacity` | governed by `dstCapacity`; `<= dstCapacity` |
| 46 | `LZ4_decompress_safe_partial` | truncated input with `partialDecoding` (silent truncation, lz4.c:2296-2306) | `>= 0` partial byte count (no error) |
| 47 | `LZ4_decompress_fast` | literal length exceeds output room (lz4.c:1898) | `-1` |
| 48 | `LZ4_decompress_fast` | last match too close to end of block (lz4.c:1902-1907) | `-1` |
| 49 | `LZ4_decompress_fast` | match length exceeds output room (lz4.c:1921) | `-1` |
| 50 | `LZ4_decompress_fast` | offset larger than history (lz4.c:1926-1928) | `-1` |
| 51 | `LZ4_decompress_fast` | match ends within LASTLITERALS of block end (lz4.c:1957-1961) | `-1` |
| 52 | `LZ4_decompress_fast` | `originalSize` smaller than the true decoded size | `-1` |
| 53 | `LZ4_uncompress` | same conditions as rows 47-52 (thin wrapper) | `-1` |
| 54 | `LZ4_uncompress_unknownOutputSize` | same conditions as rows 29-42 (thin wrapper over `_safe`) | negative |
| 55 | `LZ4_decoderRingBufferSize` | `maxBlockSize < 0` | `0` |
| 56 | `LZ4_decoderRingBufferSize` | `maxBlockSize > LZ4_MAX_INPUT_SIZE` | `0` |
| 57 | `LZ4_decoderRingBufferSize` | `0 <= maxBlockSize < 16` | `65566` (clamped to 16) |
| 58 | `LZ4_decoderRingBufferSize` | `maxBlockSize == 16` / `== LZ4_MAX_INPUT_SIZE` (boundaries) | valid non-zero size |
| 59 | `LZ4_setStreamDecode` | `dictSize < 0` (cast to `size_t`, no rejection) | `1` |
| 60 | `LZ4_setStreamDecode` | `dictionary == NULL, dictSize == 0` (reset) | `1` |
| 61 | `LZ4_decompress_safe_continue` | first call on a fresh stream with corrupt src | negative, stream state un-advanced ⇒ next call also fails identically |
| 62 | `LZ4_decompress_safe_usingDict` | `dictSize >= 65536` disables `checkOffset` (lz4.c:2047) ⇒ out-of-range offsets NOT rejected | identical (possibly garbage) output, not an error |
| 63 | `LZ4_decompress_safe_usingDict` | `dictSize == 0` (delegates to noDict) | same as row 29-42 behaviour |
| 64 | `LZ4_attach_dictionary` | `dictionaryStream == NULL` (detach) | void; subsequent compression identical to no-dict |
| 65 | `LZ4_attach_dictionary` | dictionary stream with `dictSize == 0` (silently not attached, lz4.c:1679). NOTE: the `currentOffset == 0 -> 64 KB` bump at lz4.c:1673 happens FIRST, so this is NOT byte-identical to no-dict (it flips `noDictIssue` to `dictSmall`) | void; C and Rust must agree; no dictionary is consulted |
| 66 | `LZ4_compress_fast_continue` | `acceleration < 1` / `> 65537` | clamped (1 / 65537) |
| 67 | `LZ4_compress_fast_continue` | `dstCapacity` too small | `0` |
| 68 | `LZ4_compress_forceExtDict` | `srcSize > LZ4_MAX_INPUT_SIZE` / `< 0` | `0` |
| 69 | `LZ4_sizeofState` / `LZ4_sizeofStreamState` | always | `16416` |

## lz4hc.c — high-compression API

| # | function | trigger (exact invalid input/condition) | expected C result |
|---|----------|------------------------------------------|-------------------|
| 70 | `LZ4_initStreamHC` | `buffer == NULL` | `NULL` |
| 71 | `LZ4_initStreamHC` | `size < sizeof(LZ4_streamHC_t)` (262200) | `NULL` |
| 72 | `LZ4_initStreamHC` | `buffer` not 8-byte aligned | `NULL` |
| 73 | `LZ4_initStreamHC` | `size == 262200`, aligned (boundary, valid) | `buffer` |
| 74 | `LZ4_compress_HC` | `srcSize < 0` | `0` |
| 75 | `LZ4_compress_HC` | `srcSize > LZ4_MAX_INPUT_SIZE` | `0` |
| 76 | `LZ4_compress_HC` | `dstCapacity` too small (`_dest_overflow`, lz4hc.c:305/331/713/1314/2065) | `0` |
| 77 | `LZ4_compress_HC` | `dstCapacity == 0` | `0` |
| 78 | `LZ4_compress_HC` | `compressionLevel < 1` (0, -1, INT_MIN) | clamped to `LZ4HC_CLEVEL_DEFAULT` 9 ⇒ identical to level 9 |
| 79 | `LZ4_compress_HC` | `compressionLevel > LZ4HC_CLEVEL_MAX` (13, 100, INT_MAX) | clamped to 12 ⇒ identical to level 12 |
| 80 | `LZ4_compress_HC` | `compressionLevel == 1` (below documented `LZ4HC_CLEVEL_MIN` 2, but ACCEPTED, not clamped) | valid level-1 (`lz4mid`) output |
| 81 | `LZ4_compress_HC` | `srcSize == 0` | `1` (empty block) or `0` if `dstCapacity == 0` |
| 82 | `LZ4_compress_HC_extStateHC` | `state == NULL` (via `LZ4_initStreamHC` NULL) | `0` |
| 83 | `LZ4_compress_HC_extStateHC` | `state` misaligned | `0` |
| 84 | `LZ4_compress_HC_extStateHC_fastReset` | `state` misaligned (`!LZ4_isAligned(state,8)`) | `0` |
| 85 | `LZ4_compress_HC_destSize` | `targetDstSize < 1` | `0` |
| 86 | `LZ4_compress_HC_destSize` | `*srcSizePtr > LZ4_MAX_INPUT_SIZE` / `< 0` | `0` |
| 87 | `LZ4_compress_HC_destSize` | `targetDstSize` smaller than needed (fillOutput salvage) | `<= targetDstSize`, `*srcSizePtr` reduced |
| 88 | `LZ4_compress_HC_continue` | `dstCapacity` too small | `0` |
| 89 | `LZ4_compress_HC_continue_destSize` | `targetDestSize < 1` | `0` |
| 90 | `LZ4_loadDictHC` | `dictSize > 65536` | `65536` (last 64 KB) |
| 91 | `LZ4_loadDictHC` | `dictSize == 0` | `0` |
| 92 | `LZ4_loadDictHC` | `dictSize < LZ4HC_HASHSIZE` (4) at level >= 3 (chain table not seeded) | `dictSize` unchanged |
| 93 | `LZ4_loadDictHC` | `dictSize <= LZ4MID_HASHSIZE` (8) at level 1-2 (hash table not seeded) | `dictSize` unchanged |
| 94 | `LZ4_saveDictHC` | `dictSize < 4` (0..3 and negatives) | `0` |
| 95 | `LZ4_saveDictHC` | `dictSize > 65536` | clamped to `min(65536, prefixSize)` |
| 96 | `LZ4_saveDictHC` | `dictSize > prefixSize` | clamped to `prefixSize` |
| 97 | `LZ4_freeStreamHC` | `LZ4_streamHCPtr == NULL` | `0` |
| 98 | `LZ4_freeHC` | `LZ4HC_Data == NULL` | `0` |
| 99 | `LZ4_resetStreamStateHC` | `state == NULL` or misaligned (INVERTED convention: 1 = error) | `1` |
| 100 | `LZ4_resetStreamStateHC` | valid aligned state | `0` |
| 101 | `LZ4_setCompressionLevel` | `compressionLevel < 1` | stored as 9 ⇒ next compress identical to level 9 |
| 102 | `LZ4_setCompressionLevel` | `compressionLevel > 12` | stored as 12 ⇒ next compress identical to level 12 |
| 103 | `LZ4_favorDecompressionSpeed` | `favor != 0` (1, -1, 12345) | stored as 1 ⇒ identical output for all non-zero values |
| 104 | `LZ4_resetStreamHC_fast` | called after a compression that returned `<= 0` (`dirty` flag set) | full re-init; subsequent output identical to a fresh stream |
| 105 | `LZ4_attach_HC_dictionary` | `dictionary_stream == NULL` (detach) | void; compression identical to no-dict |
| 106 | `LZ4_attach_HC_dictionary` | attached dict with history `position >= 65536` (silently dropped, lz4hc.c:1454) | void; compression identical to no-dict |
| 107 | `LZ4_compressHC` / `LZ4_compressHC2` / `..._limitedOutput` / `..._withStateHC` (deprecated) | `dstCapacity` too small; `cLevel` 0 hard-coded ⇒ 9 | `0` / level-9 output |
| 108 | `LZ4_sizeofStateHC` / `LZ4_sizeofStreamStateHC` | always | `262200` |
| 109 | `LZ4F_compressionLevel_max` (exported from lz4frame.c) | always | `12` (`LZ4HC_CLEVEL_MAX`) |

## xxhash.c — namespaced as `LZ4_XXH*`

| # | function | trigger (exact invalid input/condition) | expected C result |
|---|----------|------------------------------------------|-------------------|
| 110 | `LZ4_XXH32_update` | `input == NULL` with any `len` (`XXH_ACCEPT_NULL_INPUT_POINTER == 0`) | `XXH_ERROR` = `1` |
| 111 | `LZ4_XXH64_update` | `input == NULL` with any `len` | `XXH_ERROR` = `1` |
| 112 | `LZ4_XXH32_update` | `len == 0` with non-NULL input | `XXH_OK` = `0`, digest unchanged |
| 113 | `LZ4_XXH64_update` | `len == 0` with non-NULL input | `XXH_OK` = `0`, digest unchanged |
| 114 | `LZ4_XXH32_freeState` | `statePtr == NULL` | `XXH_OK` = `0` |
| 115 | `LZ4_XXH64_freeState` | `statePtr == NULL` | `XXH_OK` = `0` |
| 116 | `LZ4_XXH32_reset` | any seed incl. `0`, `UINT32_MAX` | `XXH_OK` = `0` |
| 117 | `LZ4_XXH64_reset` | any seed incl. `0`, `UINT64_MAX` | `XXH_OK` = `0` |
| 118 | `LZ4_XXH32` | `input == NULL, len == 0` (safe path, no read) | valid empty-input hash |
| 119 | `LZ4_XXH64` | `input == NULL, len == 0` (safe path, no read) | valid empty-input hash |
| 120 | `LZ4_XXH32_digest` | called on a state never fed any input | hash of the empty string with that seed |
| 121 | `LZ4_XXH64_digest` | called on a state never fed any input | hash of the empty string with that seed |
| 122 | `LZ4_XXH32_hashFromCanonical` / `_canonicalFromHash` | round trip of `0`, `UINT32_MAX`, random | big-endian round trip is the identity |
| 123 | `LZ4_XXH64_hashFromCanonical` / `_canonicalFromHash` | round trip of `0`, `UINT64_MAX`, random | big-endian round trip is the identity |
| 124 | `LZ4_XXH_versionNumber` | always | `605` |

## lz4frame.c — frame API

| # | function | trigger (exact invalid input/condition) | expected C result |
|---|----------|------------------------------------------|-------------------|
| 125 | `LZ4F_getBlockSize` | `blockSizeID` in `{1,2,3}` (gap below `LZ4F_max64KB`) | `(size_t)-2` `maxBlockSize_invalid` |
| 126 | `LZ4F_getBlockSize` | `blockSizeID > 7` (8, 9, 1000, INT_MAX) — out-of-range enum across FFI | `(size_t)-2` |
| 127 | `LZ4F_getBlockSize` | `blockSizeID < 0` (negative enum value across FFI) | `(size_t)-2` |
| 128 | `LZ4F_getBlockSize` | `blockSizeID == 0` (`LZ4F_default`, rewritten to 4) | `65536` |
| 129 | `LZ4F_getBlockSize` | `blockSizeID` in `{4,5,6,7}` (all valid) | `65536`/`262144`/`1048576`/`4194304` |
| 130 | `LZ4F_isError` | `code` in `{0, 1, 23, 24, 25, (size_t)-1, (size_t)-23, (size_t)-24, (size_t)-25}` | `1` iff `code > (size_t)-24` |
| 131 | `LZ4F_getErrorName` | a non-error `code` (e.g. `0`, `5`) | `"Unspecified error code"` |
| 132 | `LZ4F_getErrorName` | every error ordinal `1..23` | the matching `"ERROR_xxx"` string |
| 133 | `LZ4F_getErrorCode` | `!LZ4F_isError(result)` | `LZ4F_OK_NoError` (`0`) |
| 134 | `LZ4F_getErrorCode` | an error value `(size_t)-N` | `N` as a negative-enum code |
| 135 | `LZ4F_getVersion` | always | `100` |
| 136 | `LZ4F_createCompressionContext` | `cctxPtr == NULL` | `(size_t)-21` `parameter_null` |
| 137 | `LZ4F_createCompressionContext` | `version != LZ4F_VERSION` (0, 99, 101, UINT_MAX) — NOT validated | `0` (succeeds) |
| 138 | `LZ4F_freeCompressionContext` | `cctx == NULL` | `0` |
| 139 | `LZ4F_createDecompressionContext` | `dctxPtr == NULL` | `(size_t)-21` |
| 140 | `LZ4F_createDecompressionContext` | `version != LZ4F_VERSION` — NOT validated | `0` (succeeds) |
| 141 | `LZ4F_freeDecompressionContext` | `dctx == NULL` | `0` |
| 142 | `LZ4F_freeDecompressionContext` | mid-frame dctx (returns `dStage`, a stage number, not an error) | small positive stage value |
| 143 | `LZ4F_compressBegin` | `dstCapacity < LZ4F_HEADER_SIZE_MAX` (19) — checked unconditionally | `(size_t)-11` `dstMaxSize_tooSmall` |
| 144 | `LZ4F_compressBegin` | `dstCapacity == 19` (boundary, valid) | header size 7/11/15/19 |
| 145 | `LZ4F_compressBegin` | `prefs.frameInfo.blockSizeID` in `{1,2,3,8,...}` — NOT rejected here; `maxBlockSize` silently becomes `(size_t)-2` | success (header size), NOT an error |
| 146 | `LZ4F_compressBegin_usingDict` | `dictSize > INT_MAX` | `(size_t)-4` `parameter_invalid` |
| 147 | `LZ4F_compressBegin_usingDict` | `dictBuffer == NULL, dictSize == 0` | success (no dict) |
| 148 | `LZ4F_compressBegin_usingCDict` | `cdict == NULL` | success (no dict) |
| 149 | `LZ4F_compressUpdate` | `cctx->cStage != 1` (never `compressBegin`'d) | `(size_t)-20` `compressionState_uninitialized` |
| 150 | `LZ4F_compressUpdate` | called again after a successful `LZ4F_compressEnd` (which resets `cStage` to 0) | `(size_t)-20` |
| 151 | `LZ4F_compressUpdate` | `dstCapacity < LZ4F_compressBound(srcSize, prefs)` | `(size_t)-11` |
| 152 | `LZ4F_compressUpdate` | `dstCapacity == LZ4F_compressBound(...)` (boundary, valid) | `>= 0` |
| 153 | `LZ4F_uncompressedUpdate` | `cctx->cStage != 1` | `(size_t)-20` |
| 154 | `LZ4F_uncompressedUpdate` | `dstCapacity < srcSize` (extra check, lz4frame.c:1009) | `(size_t)-11` |
| 155 | `LZ4F_flush` | `tmpInSize == 0` on an uninitialized cctx (checked BEFORE the cStage check) | `0` — NOT `compressionState_uninitialized` |
| 156 | `LZ4F_flush` | `cStage != 1` AND `tmpInSize != 0` | `(size_t)-20` |
| 157 | `LZ4F_flush` | `dstCapacity < tmpInSize + 4 + 4` | `(size_t)-11` |
| 158 | `LZ4F_compressEnd` | `dstCapacity - flushSize < 4` (no room for endMark) | `(size_t)-11` |
| 159 | `LZ4F_compressEnd` | `contentChecksumEnabled` and `dstCapacity - flushSize < 8` | `(size_t)-11` |
| 160 | `LZ4F_compressEnd` | `prefs.frameInfo.contentSize != 0` and total fed bytes differ | `(size_t)-14` `frameSize_wrong` |
| 161 | `LZ4F_compressEnd` | on a fresh, never-begun cctx (flush returns 0, endMark written) | `4` (succeeds) |
| 162 | `LZ4F_compressBound` | invalid `blockSizeID` in prefs (unchecked `LZ4F_getBlockSize`, lz4frame.c:389) | huge garbage value — must match C exactly |
| 163 | `LZ4F_compressFrameBound` | invalid `blockSizeID` in prefs | huge garbage value — must match C exactly |
| 164 | `LZ4F_compressFrame` | `dstCapacity < LZ4F_compressFrameBound(srcSize, prefs)` | `(size_t)-11` |
| 165 | `LZ4F_compressFrame` | `dstCapacity == LZ4F_compressFrameBound(...)` (boundary, valid) | frame size |
| 166 | `LZ4F_compressFrame_usingCDict` | `dstCapacity` too small | `(size_t)-11` |
| 167 | `LZ4F_headerSize` | `src == NULL` | `(size_t)-15` `srcPtr_wrong` |
| 168 | `LZ4F_headerSize` | `srcSize < LZ4F_MIN_SIZE_TO_KNOW_HEADER_LENGTH` (5): 0..4 | `(size_t)-12` `frameHeader_incomplete` |
| 169 | `LZ4F_headerSize` | magic != `0x184D2204` and not skippable | `(size_t)-13` `frameType_unknown` |
| 170 | `LZ4F_headerSize` | magic in `0x184D2A50..0x184D2A5F` (skippable, masked `0xFFFFFFF0`) | `8` |
| 171 | `LZ4F_headerSize` | valid magic: FLG version/reserved bits NOT validated here | `7` / `11` / `15` / `19` |
| 172 | `LZ4F_getFrameInfo` | `*srcSizePtr < 5` | `*srcSizePtr = 0`, `(size_t)-12` |
| 173 | `LZ4F_getFrameInfo` | bad magic | `*srcSizePtr = 0`, `(size_t)-13` |
| 174 | `LZ4F_getFrameInfo` | `*srcSizePtr >= 5` but `< hSize` (partial header) | `*srcSizePtr = 0`, `(size_t)-12` |
| 175 | `LZ4F_getFrameInfo` | FLG bit 1 set (reserved) | `(size_t)-8` `reservedFlag_set` |
| 176 | `LZ4F_getFrameInfo` | FLG version field `(FLG>>6)&3` in `{0,2,3}` | `(size_t)-6` `headerVersion_wrong` |
| 177 | `LZ4F_getFrameInfo` | BD bit 7 set (reserved) | `(size_t)-8` |
| 178 | `LZ4F_getFrameInfo` | BD blockSizeID `(BD>>4)&7` in `{0,1,2,3}` | `(size_t)-2` |
| 179 | `LZ4F_getFrameInfo` | BD low nibble `(BD & 0xF) != 0` (reserved) | `(size_t)-8` |
| 180 | `LZ4F_getFrameInfo` | header checksum byte wrong (`(XXH32(hdr,n,0)>>8)&0xFF` mismatch) | `(size_t)-17` `headerChecksum_invalid` |
| 181 | `LZ4F_getFrameInfo` | called after feeding a PARTIAL header to `LZ4F_decompress` (`dStage == storeFrameHeader`) | `*srcSizePtr = 0`, `(size_t)-19` `frameDecoding_alreadyStarted` |
| 182 | `LZ4F_getFrameInfo` | called mid-frame after the header was decoded (delegates to `LZ4F_decompress`) | a size hint, or `(size_t)-14` |
| 183 | `LZ4F_decompress` | `srcSize == 0` on a fresh dctx | `7` (`minFHSize` hint), not an error |
| 184 | `LZ4F_decompress` | bad magic (forwarded from `LZ4F_decodeHeader`) | `(size_t)-13` |
| 185 | `LZ4F_decompress` | FLG reserved bit / bad version / bad blockSizeID / bad BD nibble | `(size_t)-8` / `-6` / `-2` / `-8` |
| 186 | `LZ4F_decompress` | wrong header checksum | `(size_t)-17` |
| 187 | `LZ4F_decompress` | block header size `(bh & 0x7FFFFFFF) > dctx->maxBlockSize` | `(size_t)-2` |
| 188 | `LZ4F_decompress` | corrupt compressed block payload (`LZ4_decompress_safe_usingDict < 0`) | `(size_t)-16` `decompressionFailed` |
| 189 | `LZ4F_decompress` | wrong block checksum on a COMPRESSED block (`blockChecksumFlag` set) | `(size_t)-7` `blockChecksum_invalid` |
| 190 | `LZ4F_decompress` | wrong block checksum on an UNCOMPRESSED (stored) block | `(size_t)-7` |
| 191 | `LZ4F_decompress` | wrong content checksum at endMark (`contentChecksumFlag` set) | `(size_t)-18` `contentChecksum_invalid` |
| 192 | `LZ4F_decompress` | declared `contentSize` != bytes actually regenerated (`frameRemainingSize != 0`) | `(size_t)-14` `frameSize_wrong` |
| 193 | `LZ4F_decompress` | truncated frame (input ends before endMark) | non-zero `nextSrcSizeHint`, no error |
| 194 | `LZ4F_decompress` | `*dstSizePtr` smaller than the block output (partial output, buffered remainder) | no error; identical partial output + hint |
| 195 | `LZ4F_decompress` | `*dstSizePtr == 0` with `dstBuffer == NULL` | no error; hint returned |
| 196 | `LZ4F_decompress` | skippable frame with a huge `SFrameSize` (unvalidated) | consumes input, no error |
| 197 | `LZ4F_decompress_usingDict` | wrong dictionary supplied (decode succeeds, wrong bytes) OR corrupt | identical output or `(size_t)-16` |
| 198 | `LZ4F_decompress_usingDict` | `dict == NULL, dictSize == 0` | identical to `LZ4F_decompress` |
| 199 | `LZ4F_resetDecompressionContext` | called mid-frame, then a fresh frame is fed | decodes cleanly, identical output |
| 200 | `LZ4F_createCDict` | `dictSize == 0` | non-NULL cdict (or NULL) — must match C |
| 201 | `LZ4F_freeCDict` | `cdict == NULL` | no-op, no crash |
| 202 | `LZ4F_createCompressionContext_advanced` / `_createDecompressionContext_advanced` | `LZ4F_defaultCMem` (all-NULL callbacks) | success |
| 203 | never-produced enums | `LZ4F_ERROR_GENERIC(1)`, `blockMode_invalid(3)`, `compressionLevel_invalid(5)`, `srcSize_tooLarge(10)` | NO code path returns `(size_t)-1/-3/-5/-10`; out-of-range `blockMode`/`contentChecksumFlag`/`frameType`/`dictID` are silently masked, never rejected |

## lz4file.c — FILE* API

| # | function | trigger (exact invalid input/condition) | expected C result |
|---|----------|------------------------------------------|-------------------|
| 204 | `LZ4F_readOpen` | `fp == NULL` | `(size_t)-21` `parameter_null` |
| 205 | `LZ4F_readOpen` | `lz4fRead == NULL` (out-pointer) | `(size_t)-21` |
| 206 | `LZ4F_readOpen` | file shorter than `LZ4F_HEADER_SIZE_MAX` (19) — even a VALID short `.lz4` | `(size_t)-23` `io_read` |
| 207 | `LZ4F_readOpen` | file at EOF already (`fread` returns 0) | `(size_t)-23` |
| 208 | `LZ4F_readOpen` | >= 19 bytes but bad magic | `(size_t)-13` |
| 209 | `LZ4F_readOpen` | >= 19 bytes, FLG reserved bit set | `(size_t)-8` |
| 210 | `LZ4F_readOpen` | >= 19 bytes, bad header checksum | `(size_t)-17` |
| 211 | `LZ4F_readOpen` | >= 19 bytes, bad FLG version | `(size_t)-6` |
| 212 | `LZ4F_read` | `lz4fRead == NULL` | `(size_t)-21` |
| 213 | `LZ4F_read` | `buf == NULL` | `(size_t)-21` |
| 214 | `LZ4F_read` | `size == 0` with valid handle+buf | `0` |
| 215 | `LZ4F_read` | reading past EOF of a complete frame | `0` (no error) |
| 216 | `LZ4F_read` | corrupt payload mid-file | forwarded `(size_t)-16` / `-7` / `-18` / `-2` / `-14` |
| 217 | `LZ4F_readClose` | `lz4fRead == NULL` | `(size_t)-21` |
| 218 | `LZ4F_readClose` | valid handle, even on a truncated frame | `0` |
| 219 | `LZ4F_writeOpen` | `fp == NULL` | `(size_t)-21` |
| 220 | `LZ4F_writeOpen` | `lz4fWrite == NULL` (out-pointer) | `(size_t)-21` |
| 221 | `LZ4F_writeOpen` | `prefsPtr->frameInfo.blockSizeID` not in `{0,4,5,6,7}` (1,2,3,8,-1) | `(size_t)-2` `maxBlockSize_invalid` |
| 222 | `LZ4F_writeOpen` | `prefsPtr == NULL` (defaults, 64 KB) | `0` |
| 223 | `LZ4F_writeOpen` | read-only `FILE*` (header `fwrite` fails) | `(size_t)-22` `io_write` |
| 224 | `LZ4F_write` | `lz4fWrite == NULL` | `(size_t)-21` |
| 225 | `LZ4F_write` | `buf == NULL` | `(size_t)-21` |
| 226 | `LZ4F_write` | `size == 0` | `0` |
| 227 | `LZ4F_write` | success | returns `size` (the UNCOMPRESSED byte count) |
| 228 | `LZ4F_writeClose` | `lz4fWrite == NULL` | `(size_t)-21` |
| 229 | `LZ4F_writeClose` | after a previous `LZ4F_write` failure (`errCode` latched) — masks the error | `0`, and the file is left truncated without a footer |
| 230 | `LZ4F_writeClose` | normal close — `ret` keeps `LZ4F_compressEnd`'s BYTE COUNT (lz4file.c:326); it is never reset to 0 | the frame-footer byte count (4, or 8 with a content checksum, more with a buffered tail) — NOT `0` |

## Appendix — C paths that are undefined behaviour (NOT differentially testable)

These were found by the same mechanical grep, but the C reaches a NULL
dereference / segfault rather than returning a value, so no differential test
can be written (both libraries would crash). Listed for completeness only:

`LZ4_compress_fast_extState`/`_withState` with a NULL/misaligned/undersized
`state`; `LZ4_resetStream`, `LZ4_resetStream_fast`, `LZ4_slideInputBuffer`,
`LZ4_resetStreamState`, `LZ4_setStreamDecode`, `LZ4_attach_dictionary`
(working stream) with NULL; `LZ4_compress_destSize` with a NULL `srcSizePtr`;
`LZ4_saveDict`/`LZ4_saveDictHC` with a NULL `safeBuffer` and non-zero clamped
size; `LZ4_compress_HC_extStateHC_fastReset` with a NULL or undersized state;
`LZ4_compress_HC_destSize` with a NULL `sourceSizePtr`; `LZ4_resetStreamHC`,
`LZ4_setCompressionLevel`, `LZ4_favorDecompressionSpeed`, `LZ4_loadDictHC`,
`LZ4_attach_HC_dictionary`, `LZ4_slideInputBufferHC`, `LZ4_compressHC2_continue`
with NULL; `LZ4F_getFrameInfo`/`LZ4F_decompress` with a NULL `dctx`,
`frameInfoPtr`, `srcSizePtr`, or `dstSizePtr`; `LZ4F_createCDict(NULL, n>0)`;
`LZ4_XXH32/64` with `input == NULL, len > 0`; `LZ4_XXH32/64_copyState`,
`_reset`, `_digest`, `_canonicalFromHash`, `_hashFromCanonical` with NULL;
`LZ4_XXH32/64_update` with a NULL state and non-NULL input.
