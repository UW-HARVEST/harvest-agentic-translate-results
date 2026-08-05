# ERRORS.md — Error-surface table (C ground truth)

Each row is a distinct way the C public API rejects/errors on input. Errors in
zstd are returned as a `size_t` whose value is `(size_t)-error_code`; testable
with `ZSTD_isError()` (returns 1) and classified with `ZSTD_getErrorCode()`.
Sentinel-returning functions use `ZSTD_CONTENTSIZE_ERROR = (0ULL-2)`.

| # | function | trigger (exact invalid input/condition) | expected C result |
|---|----------|------------------------------------------|-------------------|
| 1 | `ZSTD_compress` | `dstCapacity` too small for output | `ZSTD_isError()==1`, code `dstSize_tooSmall` |
| 2 | `ZSTD_compress` | compression level out of bounds (e.g. 1000) is CLAMPED, not error — but level far below min still valid | non-error (clamped) |
| 3 | `ZSTD_decompress` | `src` is not a valid zstd frame (bad magic) | `ZSTD_isError()==1`, code `prefix_unknown` |
| 4 | `ZSTD_decompress` | `dstCapacity` smaller than decompressed content | `ZSTD_isError()==1`, code `dstSize_tooSmall` |
| 5 | `ZSTD_decompress` | truncated frame (srcSize too small) | `ZSTD_isError()==1`, code `srcSize_wrong` |
| 6 | `ZSTD_decompress` | corrupted payload (flipped bytes in valid frame) | `ZSTD_isError()==1`, code `corruption_detected` (or similar) |
| 7 | `ZSTD_getFrameContentSize` | invalid magic / not a frame | returns `ZSTD_CONTENTSIZE_ERROR` |
| 8 | `ZSTD_getFrameContentSize` | srcSize too small to read header | returns `ZSTD_CONTENTSIZE_ERROR` |
| 9 | `ZSTD_getFrameContentSize` | frame with unknown content size | returns `ZSTD_CONTENTSIZE_UNKNOWN` |
| 10 | `ZSTD_findFrameCompressedSize` | invalid/empty input | `ZSTD_isError()==1` |
| 11 | `ZSTD_compressBound` | `srcSize >= ZSTD_MAX_INPUT_SIZE` | returns 0 |
| 12 | `ZSTD_isError` | pass a normal small value (e.g. 5) | returns 0 |
| 13 | `ZSTD_isError` | pass an error-coded value `(size_t)-10` | returns 1 |
| 14 | `ZSTD_getErrorName` | pass an error code value | returns non-NULL C string |
| 15 | `ZSTD_getErrorCode` | pass error value `(size_t)-x` | returns the ErrorCode enum x |
| 16 | `ZSTD_getErrorString` | out-of-range ErrorCode int (e.g. 9999) | returns non-NULL string ("Unspecified error code") |
| 17 | `ZSTD_CCtx_setParameter` | unknown/invalid `ZSTD_cParameter` value | `ZSTD_isError()==1`, code `parameter_unsupported` |
| 18 | `ZSTD_CCtx_setParameter` | value out of bounds for known param (e.g. windowLog=99) | `ZSTD_isError()==1`, code `parameter_outOfBound` |
| 19 | `ZSTD_cParam_getBounds` | invalid parameter enum value | `bounds.error` is `ZSTD_isError()==1` |
| 20 | `ZSTD_dParam_getBounds` | invalid parameter enum value | `bounds.error` is `ZSTD_isError()==1` |
| 21 | `ZSTD_DCtx_setParameter` | invalid parameter enum value | `ZSTD_isError()==1` |
| 22 | `ZSTD_DCtx_setParameter` | value out of bounds (e.g. windowLogMax=99) | `ZSTD_isError()==1`, code `parameter_outOfBound` |
| 23 | `ZSTD_compress2` | dst too small | `ZSTD_isError()==1`, code `dstSize_tooSmall` |
| 24 | `ZSTD_compressCCtx` | dst too small | `ZSTD_isError()==1` |
| 25 | `ZSTD_decompressDCtx` | bad magic | `ZSTD_isError()==1`, code `prefix_unknown` |
| 26 | `ZSTD_freeCCtx` | NULL pointer | returns 0 (no error) |
| 27 | `ZSTD_freeDCtx` | NULL pointer | returns 0 (no error) |
| 28 | `ZSTD_CCtx_reset` | invalid ResetDirective enum value | `ZSTD_isError()==1`, code `parameter_outOfBound` |
| 29 | `ZSTD_getDictID_fromFrame` | non-dictionary / invalid frame | returns 0 |
| 30 | `ZSTD_decompressBound` | invalid frame | returns `ZSTD_CONTENTSIZE_ERROR` |
| 31 | `ZSTD_findDecompressedSize` | invalid frame | returns `ZSTD_CONTENTSIZE_ERROR` |
| 32 | `ZSTD_compressStream2` | inconsistent state / invalid | error via `ZSTD_isError()` |
| 33 | `ZDICT_trainFromBuffer` | insufficient/empty samples | `ZDICT_isError()==1` |
| 34 | `ZSTD_decompress` | empty src (srcSize 0) | `ZSTD_isError()==1` |
| 35 | `ZSTD_getFrameContentSize` | srcSize 0 | `ZSTD_CONTENTSIZE_ERROR` |
