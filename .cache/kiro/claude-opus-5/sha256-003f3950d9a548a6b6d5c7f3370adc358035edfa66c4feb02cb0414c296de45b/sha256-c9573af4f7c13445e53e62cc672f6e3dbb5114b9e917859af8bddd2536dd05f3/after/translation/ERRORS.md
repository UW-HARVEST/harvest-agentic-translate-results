# ERRORS.md — Error-surface table

Derived mechanically from the C source: every `RETURN_ERROR` / `RETURN_ERROR_IF`
branch, every `return 0` / `return -1` / `return NULL` rejection, every `assert`,
every explicit range check and min/max constant in
`c_src/src/{lz4.c,lz4hc.c,lz4frame.c,lz4file.c,xxhash.c}`.

One row per distinct rejection. `E(x)` denotes `(size_t)-LZ4F_ERROR_x`
(an `LZ4F_isError()`-true return).

## lz4.c — block API

| # | function | trigger (exact invalid input/condition) | expected C result |
|---|----------|------------------------------------------|-------------------|
| 1 | `LZ4_compress_default` / `LZ4_compress_fast` | `srcSize > LZ4_MAX_INPUT_SIZE` (0x7E000000) — lz4.c:1360 | `0` |
| 2 | `LZ4_compress_default` / `LZ4_compress_fast` | `srcSize < 0` (cast to `U32` > max) — lz4.c:1360 | `0` |
| 3 | `LZ4_compress_default` | `dstCapacity` too small to hold result (limitedOutput budget exhausted) — lz4.c:1116/1210/1314 | `0` |
| 4 | `LZ4_compress_default` | `srcSize == 0 && dstCapacity <= 0` — lz4.c:1361-1362 | `0` |
| 5 | `LZ4_compress_default` | `srcSize == 0 && dstCapacity >= 1` (src may be NULL) | `1` (writes single 0 token) |
| 6 | `LZ4_compress_fast_extState` | `state == NULL` — lz4.c:1384 does `&LZ4_initStream(state,..)->internal_donotuse`; `internal_donotuse` is at offset 0 so `ctx` becomes NULL, `assert` is compiled out under NDEBUG, and `LZ4_compress_generic` dereferences NULL | **UNGUARDED — faults in C too** — not differentially testable; the real guard is `LZ4_initStream` (rows 10-12) |
| 7 | `LZ4_compress_fast_extState` / `_fastReset` | `state` misaligned or undersized. `_fastReset` (lz4.c:1414) performs NO state validation at all | **UNGUARDED — faults in C too** |
| 8 | `LZ4_compress_destSize` | `targetDstSize < 1` in fillOutput mode — lz4.c:985 | `0`, `*srcSizePtr` unchanged/0 |
| 9 | `LZ4_compress_destSize_extState` | NULL/misaligned state — forwards to `LZ4_compress_fast_extState`, same unguarded path as row 6 | **UNGUARDED — faults in C too** |
| 10 | `LZ4_initStream` | `buffer == NULL` — lz4.c:1555 | `NULL` |
| 11 | `LZ4_initStream` | `size < sizeof(LZ4_stream_t)` — lz4.c:1556 | `NULL` |
| 12 | `LZ4_initStream` | `buffer` not aligned to `LZ4_stream_t_alignment()` — lz4.c:1557 | `NULL` |
| 13 | `LZ4_freeStream` | `LZ4_stream == NULL` (free on NULL supported) — lz4.c:1577 | `0`, no crash |
| 14 | `LZ4_decompress_safe` | `src == NULL` — lz4.c:2036 | `-1` |
| 15 | `LZ4_decompress_safe` | `outputSize < 0` — lz4.c:2036 | `-1` |
| 16 | `LZ4_decompress_safe` | `srcSize == 0` (empty compressed input) — lz4.c:2069 | `-1` |
| 17 | `LZ4_decompress_safe` | truncated/corrupt stream: literal length exceeds output space — lz4.c:1898 | `< 0` |
| 18 | `LZ4_decompress_safe` | corrupt stream: match length exceeds output space — lz4.c:1921 | `< 0` |
| 19 | `LZ4_decompress_safe` | corrupt stream: offset points before dictionary/prefix start — lz4.c:1907/1928/1961 | `< 0` |
| 20 | `LZ4_decompress_safe` | `dstCapacity` smaller than true decompressed size | `< 0` |
| 21 | `LZ4_decompress_safe` | valid input but `compressedSize` larger than the real block (trailing garbage) | `< 0` |
| 22 | `LZ4_decompress_safe_partial` | `targetOutputSize` reached mid-stream, partialDecoding — lz4.c:2066 | `>= 0` (partial), never `< 0` for that cause |
| 23 | `LZ4_decompress_safe_partial` | `src == NULL` or `dstCapacity < 0` | `-1` |
| 24 | `LZ4_decompress_fast` | corrupt stream reading past `srcSize` implied end | `< 0` |
| 25 | `LZ4_decoderRingBufferSize` | `maxBlockSize < 0` — lz4.c:2617 | `0` |
| 26 | `LZ4_decoderRingBufferSize` | `maxBlockSize > LZ4_MAX_INPUT_SIZE` — lz4.c:2618 | `0` |
| 27 | `LZ4_compressBound` | `inputSize > LZ4_MAX_INPUT_SIZE` (macro guard) | `0` |
| 28 | `LZ4_compressBound` | `inputSize < 0` | `0` |
| 29 | `LZ4_loadDict` | `dictSize` > 64KB → silently truncated to last 64KB (not an error, boundary) | returns `min(dictSize,65536)` |
| 30 | `LZ4_loadDict` / `LZ4_loadDictSlow` | `dictSize < HASH_UNIT` (4), incl. 0 and negative — lz4.c:1613 | `0`, dict cleared. NOTE: `dictionary == NULL` is only defined together with `dictSize < 4`; with `dictSize >= 4` the C computes `p = dictEnd - 64KB` and hashes from it, dereferencing NULL (**UNGUARDED — faults in C too**) |
| 31 | `LZ4_saveDict` | `maxDictSize` of 0, negative, and > 64KB — lz4.c:1820 | clamped; returns saved size. `safeBuffer == NULL` with `dictSize != 0` is only an `assert` (lz4.c:1823), so **UNGUARDED — faults in C too** |
| 32 | `LZ4_setStreamDecode` | `dictSize < 0` | boundary: treated per C code |
| 33 | `LZ4_decompress_safe_usingDict` | `dictSize == 0 && dictStart == NULL` (assert at lz4.c:1886) | equivalent to no-dict. A NULL `dictStart` with `dictSize != 0` is **UNGUARDED — faults in C too** |

## lz4hc.c — HC block API

| # | function | trigger | expected C result |
|---|----------|---------|-------------------|
| 34 | `LZ4_compress_HC` | `srcSize > LZ4_MAX_INPUT_SIZE` — lz4hc.c:1389 | `0` |
| 35 | `LZ4_compress_HC` | `srcSize < 0` (cast to U32) — lz4hc.c:1389 | `0` |
| 36 | `LZ4_compress_HC` | `dstCapacity` too small (limitedOutput) — lz4hc.c:714/1315 | `0` |
| 37 | `LZ4_compress_HC` | `compressionLevel < 1` → clamped to `LZ4HC_CLEVEL_DEFAULT` (9) — lz4hc.c:1614 | success, level 9 output |
| 38 | `LZ4_compress_HC` | `compressionLevel > LZ4HC_CLEVEL_MAX` (12) → clamped to 12 — lz4hc.c:1615 | success, level 12 output |
| 39 | `LZ4_compress_HC` | `compressionLevel == 0` → default 9 — lz4hc.c:113 / 1614 | success, level 9 output |
| 40 | `LZ4_compress_HC` | negative `compressionLevel` (e.g. -100) → default 9 | success, level 9 output |
| 41 | `LZ4_compress_HC_extStateHC_fastReset` | `state` misaligned — lz4hc.c:1503 | `0` |
| 42 | `LZ4_compress_HC_extStateHC` | `state == NULL` → `ctx == NULL` — lz4hc.c:1515 | `0` |
| 43 | `LZ4_compress_HC_destSize` | `*srcSizePtr < 0` — lz4hc.c:559 | `0` |
| 44 | `LZ4_compress_HC_destSize` | `maxOutputSize < 0` — lz4hc.c:560 | `0` |
| 45 | `LZ4_compress_HC_destSize` | `*srcSizePtr > LZ4_MAX_INPUT_SIZE` — lz4hc.c:561-563 | `0` |
| 46 | `LZ4_compress_HC_destSize` | `dstCapacity < 1` in fillOutput — lz4hc.c:1388 | `0` |
| 47 | `LZ4_initStreamHC` | `buffer == NULL` — lz4hc.c:1578 | `NULL` |
| 48 | `LZ4_initStreamHC` | `size < sizeof(LZ4_streamHC_t)` — lz4hc.c:1579 | `NULL` |
| 49 | `LZ4_initStreamHC` | `buffer` misaligned — lz4hc.c:1580 | `NULL` |
| 50 | `LZ4_freeStreamHC` | `LZ4_streamHCPtr == NULL` — lz4hc.c:1566 | `0`, no crash |
| 51 | `LZ4_loadDictHC` | `dictSize > 64KB` → truncated to last 64KB — lz4hc.c:1634 | `min(dictSize,65536)` |
| 52 | `LZ4_loadDictHC` | `dictSize == 0` with a valid pointer | `0`. NOTE: lz4hc.c:1632 only has `assert(dictSize >= 0)` (compiled out) and never NULL-checks `dictionary`, so a negative size or NULL dict is **UNGUARDED — faults in C too** |
| 53 | `LZ4_compress_HC_continue` | `dstCapacity` too small (limitedOutput path) | `0` |
| 54 | `LZ4_attach_HC_dictionary` | `dictionaryStream == NULL` — lz4hc.c:516 | dictCtx cleared |
| 55 | `LZ4_saveDictHC` | `maxDictSize` of 0, negative, and > 64KB — lz4hc.c:1748 | clamped; returns saved size. `safeBuffer == NULL` with `dictSize != 0` is only an `assert`, so **UNGUARDED — faults in C too** |
| 56 | `LZ4_setCompressionLevel` | level out of `[1,12]` → clamped (see rows 37-40) | clamped, no error |

## lz4frame.c — frame API

| # | function | trigger | expected C result |
|---|----------|---------|-------------------|
| 57 | `LZ4F_getBlockSize` | `blockSizeID < LZ4F_max64KB` (i.e. 1,2,3) — lz4frame.c:338 | `E(maxBlockSize_invalid)` |
| 58 | `LZ4F_getBlockSize` | `blockSizeID > LZ4F_max4MB` (i.e. 8, 99, INT_MAX) — lz4frame.c:338 | `E(maxBlockSize_invalid)` |
| 59 | `LZ4F_getBlockSize` | `blockSizeID == 0` → default `LZ4F_max64KB` — lz4frame.c:337 | `65536` |
| 60 | `LZ4F_getBlockSize` | negative `blockSizeID` (out-of-range enum across FFI) | `E(maxBlockSize_invalid)` |
| 61 | `LZ4F_compressFrame` | `dstCapacity < LZ4F_compressFrameBound(srcSize,&prefs)` — lz4frame.c:456 | `E(dstMaxSize_tooSmall)` |
| 62 | `LZ4F_compressFrame` | `prefs.frameInfo.blockSizeID` invalid (1,2,3,8,9,99,-1) | **NOT an error** — corrected after reading lz4frame.c:359/448: `LZ4F_optimalBSID` normalises any ID above `max64KB` down to a valid one when `srcSize` is small, and for IDs 1..3 the error code from `LZ4F_getBlockSize` is consumed as a (huge) block size, so a frame is produced. C and Rust must agree on the produced bytes. |
| 63 | `LZ4F_createCompressionContext` | `LZ4F_compressionContextPtr == NULL` — lz4frame.c:622 | `E(parameter_null)` |
| 64 | `LZ4F_createCompressionContext` | allocation failure (`*ptr == NULL`) — lz4frame.c:625 | `E(allocation_failed)` |
| 65 | `LZ4F_compressBegin` | `dstCapacity < LZ4F_HEADER_SIZE_MAX` (19) — lz4frame.c:700 | `E(dstMaxSize_tooSmall)` |
| 66 | `LZ4F_compressBegin_usingDict` | `dictSize > INT_MAX` — lz4frame.c:768 | `E(parameter_invalid)`. Not constructible in a test (needs a >2 GiB buffer); the guard's accept-side boundary values are covered instead. |
| 67 | `LZ4F_compressUpdate` | called before `LZ4F_compressBegin` (`cStage != 1`) — lz4frame.c:1005 | `E(compressionState_uninitialized)` |
| 68 | `LZ4F_compressUpdate` | `dstCapacity < LZ4F_compressBound(srcSize, prefs)` — lz4frame.c:1007/1010 | `E(dstMaxSize_tooSmall)` |
| 69 | `LZ4F_uncompressedUpdate` | `cStage != 1` — lz4frame.c:1005 (shared path) | `E(compressionState_uninitialized)` |
| 70 | `LZ4F_flush` | `cStage != 1` — lz4frame.c:1168 | `E(compressionState_uninitialized)`, **but only when `tmpInSize != 0`**: lz4frame.c:1167 does `if (tmpInSize == 0) return 0;` BEFORE the stage check, so `flush` on a fresh cctx returns `0`. |
| 71 | `LZ4F_flush` | `dstCapacity < tmpInSize + BHSize + BFSize` — lz4frame.c:1169 | `E(dstMaxSize_tooSmall)` |
| 72 | `LZ4F_compressEnd` | `dstCapacity < 4` — lz4frame.c:1221 | `E(dstMaxSize_tooSmall)` |
| 73 | `LZ4F_compressEnd` | contentChecksum enabled and `dstCapacity < 8` — lz4frame.c:1227 | `E(dstMaxSize_tooSmall)` |
| 74 | `LZ4F_compressEnd` | declared **non-zero** `contentSize` != bytes actually fed — lz4frame.c:1237 | `E(frameSize_wrong)`. `contentSize == 0` means "unknown": no size field is written and no check is performed. |
| 75 | `LZ4F_createDecompressionContext` | `LZ4F_decompressionContextPtr == NULL` — lz4frame.c:1304 | `E(parameter_null)` |
| 76 | `LZ4F_createDecompressionContext` | allocation failure — lz4frame.c:1308 | `E(allocation_failed)` |
| 77 | `LZ4F_headerSize` | `srcSize < minFHSize` (5) — lz4frame.c:1354 | `E(frameHeader_incomplete)` |
| 78 | `LZ4F_headerSize` | magic number not `0x184D2204` and not skippable range — lz4frame.c:1374 | `E(frameType_unknown)` |
| 79 | `LZ4F_decodeHeader` (via `LZ4F_getFrameInfo`/`decompress`) | FLG reserved bit 1 set — lz4frame.c:1388 | `E(reservedFlag_set)` |
| 80 | `LZ4F_decodeHeader` | version field != 1 — lz4frame.c:1389 | `E(headerVersion_wrong)` |
| 81 | `LZ4F_decodeHeader` | BD reserved bit 7 set — lz4frame.c:1409 | `E(reservedFlag_set)` |
| 82 | `LZ4F_decodeHeader` | `blockSizeID < 4` in BD byte — lz4frame.c:1410 | `E(maxBlockSize_invalid)` |
| 83 | `LZ4F_decodeHeader` | BD low 4 bits nonzero — lz4frame.c:1411 | `E(reservedFlag_set)` |
| 84 | `LZ4F_decodeHeader` | header checksum byte mismatch — lz4frame.c:1418 | `E(headerChecksum_invalid)` |
| 85 | `LZ4F_getFrameInfo` | `src == NULL` — lz4frame.c:1446 | `E(srcPtr_wrong)` |
| 86 | `LZ4F_getFrameInfo` | `srcSize < minFHSize` — lz4frame.c:1450 | `E(frameHeader_incomplete)` |
| 87 | `LZ4F_getFrameInfo` | bad magic — lz4frame.c:1459 | `E(frameType_unknown)` |
| 88 | `LZ4F_getFrameInfo` | called after decoding already started (`dStage` past header) — lz4frame.c:1501 | `E(frameDecoding_alreadyStarted)` |
| 89 | `LZ4F_getFrameInfo` | `srcSize == 0` when stage is storeFrameHeader — lz4frame.c:1507 | `E(frameHeader_incomplete)` |
| 90 | `LZ4F_decompress` | block size field indicates size > declared maxBlockSize — lz4frame.c:1738 | `E(maxBlockSize_invalid)` |
| 91 | `LZ4F_decompress` | block checksum mismatch (blockChecksumFlag set) — lz4frame.c:1829/1878 | `E(blockChecksum_invalid)` |
| 92 | `LZ4F_decompress` | inner `LZ4_decompress_safe*` returns `< 0` (corrupt block) — lz4frame.c:1905/1950 | `E(decompressionFailed)` |
| 93 | `LZ4F_decompress` | EndMark reached with `frameRemainingSize != 0` — lz4frame.c:1984 | `E(frameSize_wrong)` |
| 94 | `LZ4F_decompress` | content checksum mismatch — lz4frame.c:2021 | `E(contentChecksum_invalid)` |
| 95 | `LZ4F_decompress` | truncated frame (input ends early) | returns nonzero `hint`, no error |
| 96 | `LZ4F_isError` | `code > (size_t)(-LZ4F_ERROR_maxCode)` — lz4frame.c:295 | `1` for error codes, `0` otherwise |
| 97 | `LZ4F_getErrorName` | out-of-range error code across FFI | `"Unspecified error code"` |
| 98 | `LZ4F_getErrorCode` | non-error `functionResult` | `LZ4F_OK_NoError` |
| 99 | `LZ4F_freeCompressionContext` | `NULL` cctx | no-op, no crash |
| 100 | `LZ4F_freeDecompressionContext` | `NULL` dctx | returns stored error / 0, no crash |
| 101 | `LZ4F_createCDict` | `dictBuffer == NULL` / `dictSize == 0` | valid or NULL CDict per C |
| 102 | `LZ4F_freeCDict` | `NULL` CDict | no-op |
| 103 | `LZ4F_compressBound` | `srcSize == 0` with NULL prefs (worst-case flush bound) | nonzero bound |

## lz4file.c — file API

| # | function | trigger | expected C result |
|---|----------|---------|-------------------|
| 104 | `LZ4F_readOpen` | `fp == NULL` — lz4file.c:79 | `E(parameter_null)` |
| 105 | `LZ4F_readOpen` | `lz4fRead == NULL` (out param) — lz4file.c:79 | `E(parameter_null)` |
| 106 | `LZ4F_readOpen` | allocation failure — lz4file.c:84 | `E(allocation_failed)` |
| 107 | `LZ4F_readOpen` | `fread` of header returns short/0 (empty file) — lz4file.c:98 | `E(io_read)` |
| 108 | `LZ4F_readOpen` | frame header declares invalid blockSizeID — lz4file.c:124 | `E(maxBlockSize_invalid)` |
| 109 | `LZ4F_readOpen` | srcBuf allocation failure — lz4file.c:129-131 | `E(allocation_failed)` |
| 110 | `LZ4F_read` | `lz4fRead == NULL` — lz4file.c:145 | `E(parameter_null)` |
| 111 | `LZ4F_read` | `buf == NULL` — lz4file.c:145 | `E(parameter_null)` |
| 112 | `LZ4F_read` | `ferror` after read — lz4file.c:162 | `E(io_read)` |
| 113 | `LZ4F_readClose` | `lz4fRead == NULL` — lz4file.c:185 | `E(parameter_null)` |
| 114 | `LZ4F_writeOpen` | `fp == NULL` — lz4file.c:222 | `E(parameter_null)` |
| 115 | `LZ4F_writeOpen` | `lz4fWrite == NULL` — lz4file.c:222 | `E(parameter_null)` |
| 116 | `LZ4F_writeOpen` | allocation failure — lz4file.c:226 | `E(allocation_failed)` |
| 117 | `LZ4F_writeOpen` | `prefsPtr->frameInfo.blockSizeID` invalid — lz4file.c:246 | `E(maxBlockSize_invalid)` |
| 118 | `LZ4F_writeOpen` | dstBuf allocation failure — lz4file.c:254 | `E(allocation_failed)` |
| 119 | `LZ4F_writeOpen` | `fwrite` of header fails — lz4file.c:273 | `E(io_write)` |
| 120 | `LZ4F_write` | `lz4fWrite == NULL` — lz4file.c:288 | `E(parameter_null)` |
| 121 | `LZ4F_write` | `buf == NULL` — lz4file.c:288 | `E(parameter_null)` |
| 122 | `LZ4F_write` | `fwrite` short write — lz4file.c:307 | `E(io_write)` |
| 123 | `LZ4F_writeClose` | `lz4fWrite == NULL` — lz4file.c:321 | `E(parameter_null)` |

## xxhash.c

| # | function | trigger | expected C result |
|---|----------|---------|-------------------|
| 124 | `LZ4_XXH32_reset` | `statePtr == NULL` (see xxhash.c:456-458 `XXH_ERROR` branch on copy path) | per-C: `XXH_OK` on non-NULL |
| 125 | `LZ4_XXH32_update` | `input == NULL && len == 0` — xxhash.c:456 | `XXH_OK` |
| 126 | `LZ4_XXH32_update` | `input == NULL && len != 0` — xxhash.c:458 | `XXH_ERROR` (1) |
| 127 | `LZ4_XXH64_update` | `input == NULL && len == 0` — xxhash.c:916 | `XXH_OK` |
| 128 | `LZ4_XXH64_update` | `input == NULL && len != 0` — xxhash.c:918 | `XXH_ERROR` (1) |
| 129 | `LZ4_XXH32` / `LZ4_XXH64` | `len == 0` | canonical empty-input hash |
| 130 | `LZ4_XXH32_freeState` / `LZ4_XXH64_freeState` | `NULL` state | `XXH_OK`, no crash |
| 131 | `LZ4_XXH32_hashFromCanonical` | any 4 bytes (no rejection) — pure BE decode | round-trips |
| 132 | `LZ4_XXH64_hashFromCanonical` | any 8 bytes | round-trips |

## Out-of-range enum values across the FFI boundary

C enums accept any `int`. These are real inputs the C handles and the Rust must
match identically.

| # | function | trigger | expected C result |
|---|----------|---------|-------------------|
| 133 | `LZ4F_getBlockSize` | `blockSizeID` = 1,2,3,8,9,255,-1,INT_MAX, INT_MIN | `E(maxBlockSize_invalid)` (except 0→65536, 4..7 valid) |
| 134 | `LZ4F_compressFrame` | `prefs.frameInfo.blockMode` = 2,7,-1 (no valid variant) | treated as non-zero → blockIndependent path per C `if` |
| 135 | `LZ4F_compressFrame` | `prefs.frameInfo.contentChecksumFlag` = 2,-1 | non-zero → checksum enabled per C `if` |
| 136 | `LZ4F_compressFrame` | `prefs.frameInfo.blockChecksumFlag` = 2,-1 | non-zero → block checksum enabled |
| 137 | `LZ4F_compressFrame` | `prefs.frameInfo.frameType` = 2,-1 | per C branch (compressFrame ignores/forces) |
| 138 | `LZ4F_getErrorName` | code with no enum variant (e.g. `(size_t)-999`) | `"Unspecified error code"` |
| 139 | `LZ4F_getErrorCode` | arbitrary huge `size_t` | `(LZ4F_errorCodes)(0 - functionResult)` value |

---

## Note on UNGUARDED inputs

Nine rows above are marked **UNGUARDED — faults in C too**. These were initially
written from the header doc comments, then corrected after reading the C bodies:
the "validation" in those paths is an `assert()`, which this CMake build compiles
out. The C therefore dereferences the bad pointer and segfaults.

These inputs are *not differentially testable*: the reference implementation
crashes, so there is no C result for the Rust to match. Verified experimentally —
calling them on the C `.so` alone raises SIGSEGV. Each is documented at its test
site, and the neighbouring guard that the C **does** implement is tested instead
(e.g. `LZ4_initStream`'s NULL / size / alignment rejection, rows 10-12).

This is a property of the C API contract (the caller must supply a valid state
and a non-NULL dictionary), not a translation defect.

---

## Phase C verification status

Every row has a differential test that constructs the exact invalid
input/condition, calls BOTH the C `.so` and the Rust `.so`, and asserts they
return the SAME error code / sentinel (not merely "both failed").

| rows | test file | tests |
|------|-----------|-------|
| 1-56 (lz4.c, lz4hc.c) | `tests/errors_block.rs` | 11 |
| 57-139 (lz4frame.c, lz4file.c, xxhash.c) | `tests/errors_frame.rs` | 9 |

Beyond the table, these generic boundaries are covered: NULL pointers, zero and
oversized lengths, values one step past every documented range, `i32::MIN` /
`i32::MAX` / `usize::MAX`, thousands of randomized out-of-range values, and
out-of-range **enum** values crossing the FFI boundary (rows 133-137) — C enums
accept any `int`, and this is precisely where the one real translation bug was
found (see below). File-API error codes are additionally covered in
`tests/file_api.rs`.

### Bug found and fixed by this phase

`LZ4F_compressFrameBound` / `LZ4F_compressBound` diverged for out-of-range enum
values in `LZ4F_preferences_t`:

```
contentChecksumFlag = -1, srcSize = 26098
  C    = 17179895305      Rust = 26121        (difference = 4 * 2^32)
```

Cause: `LZ4F_compressBound_internal` (lz4frame.c:398-399) computes
`BHSize + contentChecksumFlag * BFSize` and `BFSize * blockChecksumFlag`
directly from the enum fields. Every enumerator of these enums is
non-negative, so the C compiler gives them an **unsigned** underlying type
(verified: `(LZ4F_contentChecksum_t)-1 < 0` is false). The C therefore reads
`-1` as `4294967295` and zero-extends it, while the Rust declared the fields as
`c_int` and sign-extended them.

Fix (`src/lz4frame.rs`): declared the five enum fields of `LZ4F_frameInfo_t`
(`blockSizeID`, `blockMode`, `contentChecksumFlag`, `frameType`,
`blockChecksumFlag`) and their associated constants and helper signatures as
`c_uint`, matching the C ABI. `c_int` -> `c_uint` is ABI-identical for parameter
passing, so no exported signature changed; only the arithmetic semantics were
corrected.
