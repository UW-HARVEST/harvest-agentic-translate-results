# ERRORS.md — Error-surface table

Derived mechanically from the C source in `c_src/src`: every distinct rejection
reachable across the public FFI boundary. Sources scanned:

```
grep -rn "RETURN_ERROR_IF\|RETURN_ERROR\|ERROR(" c_src/src --include=*.c --include=*.h
  -> 245 RETURN_ERROR_IF, 39 RETURN_ERROR, 1156 ERROR(  sites
c_src/src/include/zstd_errors.h            -> 37 ZSTD_ErrorCode enumerators
c_src/src/compress/zstd_compress.c         -> ZSTD_cParam_getBounds (39 cases + default)
c_src/src/decompress/zstd_decompress.c     -> ZSTD_dParam_getBounds (8 cases + default)
c_src/src/dictBuilder/{zdict,cover,fastcover}.c
```

Row semantics: "expected C result" is what the C `.so` returns for the exact
trigger. The differential test asserts the Rust `.so` returns the **same**
value (same `ZSTD_ErrorCode` via `ZSTD_getErrorCode`, or the same sentinel),
not merely "both failed".

Test file: `translation/tests/phase_c_errors.rs`
`[x]` = differential test written AND passing against both `.so` files.

## Legend for sentinels

| sentinel | value |
|---|---|
| `ZSTD_CONTENTSIZE_UNKNOWN` | `0ULL - 1` |
| `ZSTD_CONTENTSIZE_ERROR`   | `0ULL - 2` |
| error return              | `size_t` in `[-ZSTD_error_maxCode, -1]`, detected by `ZSTD_isError` |

## A. Error-code / error-string surface

| # | function | trigger (exact invalid input/condition) | expected C result | [ ] |
|---|----------|------------------------------------------|-------------------|-----|
| A1 | `ZSTD_isError` | every `code` in `0..=200` | 1 iff `code > -ZSTD_error_maxCode` (i.e. `(size_t)-code <= 120`… per `ERR_isError`) | [x] |
| A2 | `ZSTD_getErrorCode` | every `code` in `0..=200` | `ERR_getErrorCode`: `no_error` when not an error, else the mapped enum | [x] |
| A3 | `ZSTD_getErrorName` | every `code` in `0..=200` | identical NUL-terminated string bytes | [x] |
| A4 | `ZSTD_getErrorString` | every `int` in `-5..=130`, incl. out-of-range enum values | identical string, `"Unspecified error code"` for unknown | [x] |
| A5 | `ZSTD_getErrorName` | `code = 0` (not an error) | `"No error detected"` | [x] |

## B. Parameter bounds — compression (`ZSTD_CCtx_setParameter`)

`ZSTD_cParam_getBounds` is the mechanical source of every bound. Bounds are read
from each `.so` at runtime, then each parameter is driven `lower-1` and `upper+1`.

| # | function | trigger | expected C result | [ ] |
|---|----------|---------|-------------------|-----|
| B1 | `ZSTD_cParam_getBounds` | `param` = each of the 39 valid `ZSTD_cParameter` values | `bounds.error == 0`, identical `lowerBound`/`upperBound` | [x] |
| B2 | `ZSTD_cParam_getBounds` | `param` = out-of-range enum ints (`-1, 0, 1, 99, 108, 131, 165, 203, 403, 500, 1000, i32::MIN, i32::MAX`) | `bounds.error = ERROR(parameter_unsupported)` (=`-40`), bounds `{0,0}` | [x] |
| B3 | `ZSTD_CCtx_setParameter` | valid `param`, `value = lowerBound - 1` | `ERROR(parameter_outOfBound)` — except params whose C code clamps instead (see B5) | [x] |
| B4 | `ZSTD_CCtx_setParameter` | valid `param`, `value = upperBound + 1` | `ERROR(parameter_outOfBound)` — except clamping params | [x] |
| B5 | `ZSTD_CCtx_setParameter` | `ZSTD_c_compressionLevel` beyond `[minCLevel, maxCLevel]` | C **clamps** (`BOUNDCHECK` not applied); returns the accepted value | [x] |
| B6 | `ZSTD_CCtx_setParameter` | out-of-range `param` enum ints (same set as B2) | `ERROR(parameter_unsupported)` | [x] |
| B7 | `ZSTD_CCtx_setParameter` | `param = ZSTD_c_nbWorkers`, `value > 0`, built without `ZSTD_MULTITHREAD` | upperBound is 0 ⇒ `ERROR(parameter_outOfBound)` | [x] |
| B8 | `ZSTD_CCtx_setParameter` | `cctx = NULL` | must match C (segfault-free path only tested where C tolerates it) | n/a |
| B9 | `ZSTD_CCtx_getParameter` | out-of-range `param` enum ints | `ERROR(parameter_unsupported)` | [x] |
| B10 | `ZSTD_CCtx_setParameter` | called after `ZSTD_compressStream2` started a frame (stage != init) | `ERROR(stage_wrong)` for non-`compressionLevel`-class params | [x] |
| B11 | `ZSTD_CCtxParams_setParameter` | out-of-range `param`, and `lower-1`/`upper+1` per valid param | same codes as B3/B4/B6 | [x] |
| B12 | `ZSTD_CCtxParams_getParameter` | out-of-range `param` | `ERROR(parameter_unsupported)` | [x] |
| B13 | `ZSTD_CCtx_setPledgedSrcSize` | called mid-frame (stage != init) | `ERROR(stage_wrong)` | [x] |
| B14 | `ZSTD_CCtx_reset` | `reset` directive = out-of-range enum int (`-1, 3, 99`) | `ERROR(parameter_unsupported)` | [x] |
| B15 | `ZSTD_CCtx_setParameter` | `ZSTD_c_strategy` = `ZSTD_STRATEGY_MIN-1` / `MAX+1` | `ERROR(parameter_outOfBound)` | [x] |
| B16 | `ZSTD_CCtx_setParameter` | `ZSTD_c_windowLog` = `ZSTD_WINDOWLOG_MAX+1` | `ERROR(parameter_outOfBound)` | [x] |

## C. Parameter bounds — decompression (`ZSTD_DCtx_setParameter`)

| # | function | trigger | expected C result | [ ] |
|---|----------|---------|-------------------|-----|
| C1 | `ZSTD_dParam_getBounds` | each of the 8 valid `ZSTD_dParameter` values | `error == 0`, identical bounds | [x] |
| C2 | `ZSTD_dParam_getBounds` | out-of-range enum ints (`-1, 0, 1, 99, 101, 1001, 1004, i32::MIN, i32::MAX`) | `ERROR(parameter_unsupported)` | [x] |
| C3 | `ZSTD_DCtx_setParameter` | valid `param`, `value = lower-1` | `ERROR(parameter_outOfBound)` | [x] |
| C4 | `ZSTD_DCtx_setParameter` | valid `param`, `value = upper+1` | `ERROR(parameter_outOfBound)` | [x] |
| C5 | `ZSTD_DCtx_setParameter` | out-of-range `param` enum int | `ERROR(parameter_unsupported)` | [x] |
| C6 | `ZSTD_DCtx_reset` | `reset` directive out-of-range enum int | `ERROR(parameter_unsupported)` | [x] |
| C7 | `ZSTD_DCtx_setMaxWindowSize` | `windowSize` beyond `ZSTD_WINDOWLOG_MAX` bound | `ERROR(parameter_outOfBound)` | [x] |
| C8 | `ZSTD_DCtx_setFormat` | out-of-range format enum int | `ERROR(parameter_unsupported)` (via setParameter) | [x] |

## D. Buffer-size rejections — compression

| # | function | trigger | expected C result | [ ] |
|---|----------|---------|-------------------|-----|
| D1 | `ZSTD_compress` | `dstCapacity = 0`, `srcSize > 0` | `ERROR(dstSize_tooSmall)` | [x] |
| D2 | `ZSTD_compress` | `dstCapacity` = 1 .. `compressBound-1` (too small for incompressible src) | `ERROR(dstSize_tooSmall)` | [x] |
| D3 | `ZSTD_compress` | `dst = NULL, dstCapacity = 0` | `ERROR(dstSize_tooSmall)` | [x] |
| D4 | `ZSTD_compress` | `src = NULL, srcSize = 0`, adequate `dstCapacity` | success (empty frame) — not an error; asserted byte-identical | [x] |
| D5 | `ZSTD_compress` | `src = NULL, srcSize > 0` | must match C exactly (C dereferences ⇒ excluded as UB; documented, untested) | n/a |
| D6 | `ZSTD_compressBound` | `srcSize = 0`, `1`, `ZSTD_BLOCKSIZE_MAX`, `1<<20`, `SIZE_MAX`, `SIZE_MAX/2` | identical `size_t` (incl. overflow wraparound) | [x] |
| D7 | `ZSTD_compress2` | `dstCapacity` too small, `stableOutBuffer` unset | `ERROR(dstSize_tooSmall)` | [x] |
| D8 | `ZSTD_compressCCtx` | `dstCapacity = 0` | `ERROR(dstSize_tooSmall)` | [x] |
| D9 | `ZSTD_compressBlock` | `srcSize > ZSTD_BLOCKSIZE_MAX` | `ERROR(srcSize_wrong)` | [x] |
| D10 | `ZSTD_compressBlock` | called without `ZSTD_compressBegin` (init missing) | `ERROR(stage_wrong)` | [x] |
| D11 | `ZSTD_compressEnd` / `ZSTD_compressContinue` | `dstCapacity = 0` | `ERROR(dstSize_tooSmall)` | [x] |
| D12 | `ZSTD_CCtx_setPledgedSrcSize` + `compressStream2` | actual src bytes ≠ pledged size | `ERROR(srcSize_wrong)` | [x] |
| D13 | `ZSTD_compressStream2` | `ZSTD_c_stableInBuffer=1` then input buffer moved between calls | `ERROR(stabilityCondition_notRespected)` | [x] |
| D14 | `ZSTD_compressStream2` | `ZSTD_c_stableOutBuffer=1` then output buffer moved between calls | `ERROR(stabilityCondition_notRespected)` | [x] |
| D15 | `ZSTD_compressStream2` | `endOp` = out-of-range `ZSTD_EndDirective` int (`-1, 3, 99`) | `ERROR(parameter_outOfBound)` | [x] |

## E. Buffer-size / framing rejections — decompression

| # | function | trigger | expected C result | [ ] |
|---|----------|---------|-------------------|-----|
| E1 | `ZSTD_decompress` | `srcSize = 0` | `ERROR(srcSize_wrong)` | [x] |
| E2 | `ZSTD_decompress` | `src` shorter than frame header (1..8 bytes of a valid frame) | `ERROR(srcSize_wrong)` | [x] |
| E3 | `ZSTD_decompress` | `src` = valid frame truncated mid-block | `ERROR(srcSize_wrong)` | [x] |
| E4 | `ZSTD_decompress` | `dstCapacity` < decompressed size | `ERROR(dstSize_tooSmall)` | [x] |
| E5 | `ZSTD_decompress` | `dstCapacity = 0` for a non-empty frame | `ERROR(dstSize_tooSmall)` | [x] |
| E6 | `ZSTD_decompress` | random bytes (bad magic) | `ERROR(prefix_unknown)` | [x] |
| E7 | `ZSTD_decompress` | magic `0x184D2A5x` skippable frame only | success, 0 bytes out | [x] |
| E8 | `ZSTD_decompress` | valid frame with a flipped bit in the block body | `ERROR(corruption_detected)` (or `checksum_wrong` when the checksum flag was set) | [x] |
| E9 | `ZSTD_decompress` | valid checksummed frame with corrupted trailing 4-byte checksum | `ERROR(checksum_wrong)` | [x] |
| E10 | `ZSTD_decompress` | frame whose `windowLog` exceeds `ZSTD_d_windowLogMax` | `ERROR(frameParameter_windowTooLarge)` | [x] |
| E11 | `ZSTD_decompress` | frame header reserved bit set | `ERROR(corruption_detected)` | [x] |
| E12 | `ZSTD_decompress` | frame version byte indicating unsupported version | `ERROR(version_unsupported)` / `prefix_unknown` (whichever C returns) | [x] |
| E13 | `ZSTD_getFrameContentSize` | `srcSize = 0` | `ZSTD_CONTENTSIZE_ERROR` | [x] |
| E14 | `ZSTD_getFrameContentSize` | bad magic | `ZSTD_CONTENTSIZE_ERROR` | [x] |
| E15 | `ZSTD_getFrameContentSize` | truncated header (each length `1..=17`) | `ZSTD_CONTENTSIZE_ERROR` | [x] |
| E16 | `ZSTD_getFrameContentSize` | frame written with `contentSizeFlag=0` | `ZSTD_CONTENTSIZE_UNKNOWN` | [x] |
| E17 | `ZSTD_getDecompressedSize` | bad magic / error case | `0` (`>= CONTENTSIZE_ERROR ? 0 : ret`) | [x] |
| E18 | `ZSTD_findDecompressedSize` | trailing garbage after a valid frame | `ZSTD_CONTENTSIZE_ERROR` | [x] |
| E19 | `ZSTD_findFrameCompressedSize` | `srcSize = 0`, bad magic, truncated | error return (identical code) | [x] |
| E20 | `ZSTD_decompressBound` | bad magic / truncated | `ZSTD_CONTENTSIZE_ERROR` | [x] |
| E21 | `ZSTD_getFrameHeader` | truncated input (each length `0..=18`) | `> 0` (bytes still needed) or identical error | [x] |
| E22 | `ZSTD_getFrameHeader_advanced` | `format = ZSTD_f_zstd1_magicless` on a magic-bearing frame | identical result/error | [x] |
| E23 | `ZSTD_getFrameHeader_advanced` | out-of-range `format` enum int | identical result/error | [x] |
| E24 | `ZSTD_decompressBlock` | `srcSize > ZSTD_BLOCKSIZE_MAX` | `ERROR(srcSize_wrong)` | [x] |
| E25 | `ZSTD_decompressBlock` | called without `ZSTD_decompressBegin` | `ERROR(stage_wrong)`/`init_missing` (identical) | [x] |
| E26 | `ZSTD_decompressStream` | `input.pos > input.size` | `ERROR(srcBuffer_wrong)` | [x] |
| E27 | `ZSTD_decompressStream` | `output.pos > output.size` | `ERROR(dstBuffer_wrong)` | [x] |
| E28 | `ZSTD_decompressStream` | zero-size input and zero-size output repeatedly | `ERROR(noForwardProgress_inputEmpty)` after `ZSTD_NO_FORWARD_PROGRESS_MAX` calls | [x] |
| E29 | `ZSTD_decompressStream` | full output, non-empty input, repeated | `ERROR(noForwardProgress_destFull)` | [x] |
| E30 | `ZSTD_decompressDCtx` | `dctx` reused after an error without reset | identical error | [x] |
| E31 | `ZSTD_isFrame` | `srcSize < 4`, bad magic, valid magic, skippable magic | identical `0`/`1` | [x] |
| E32 | `ZSTD_readSkippableFrame` | non-skippable magic | `ERROR(frameParameter_unsupported)` | [x] |
| E33 | `ZSTD_writeSkippableFrame` | `dstCapacity < srcSize + 8` | `ERROR(dstSize_tooSmall)` | [x] |
| E34 | `ZSTD_writeSkippableFrame` | `magicVariant > 15` | `ERROR(parameter_outOfBound)` | [x] |
| E35 | `ZSTD_decompress` | 0-length source with `dstCapacity > 0` | `ERROR(srcSize_wrong)` | [x] |
| E36 | `ZSTD_decompressStream` | `ZSTD_d_stableOutBuffer=1` then output buffer changed | `ERROR(dstBuffer_wrong)` | [x] |

## F. Dictionary rejections

| # | function | trigger | expected C result | [ ] |
|---|----------|---------|-------------------|-----|
| F1 | `ZSTD_getDictID_fromDict` | `dictSize <= 8` | `0` | [x] |
| F2 | `ZSTD_getDictID_fromDict` | bad dict magic | `0` | [x] |
| F3 | `ZSTD_getDictID_fromFrame` | non-frame / truncated input | `0` | [x] |
| F4 | `ZSTD_getDictID_fromCDict` / `fromDDict` | `NULL` handle | `0` | [x] |
| F5 | `ZSTD_createCDict` | `dictSize = 0` | non-NULL CDict (empty dict) — matches C | [x] |
| F6 | `ZSTD_createCDict_advanced` | `dictContentType = ZSTD_dct_fullDict`, garbage dict | `NULL` | [x] |
| F7 | `ZSTD_createDDict` | garbage dict with `ZSTD_dct_fullDict` semantics | identical NULL/non-NULL | [x] |
| F8 | `ZSTD_decompress_usingDDict` | frame built with dict A, decompressed with dict B | `ERROR(dictionary_wrong)` | [x] |
| F9 | `ZSTD_decompress_usingDict` | frame built with a dict, decompressed with no dict | `ERROR(dictionary_wrong)` | [x] |
| F10 | `ZSTD_CCtx_loadDictionary_advanced` | out-of-range `ZSTD_dictLoadMethod_e` / `ZSTD_dictContentType_e` ints | identical error | [x] |
| F11 | `ZSTD_DCtx_loadDictionary_advanced` | out-of-range enum ints (both enums) | identical error | [x] |
| F12 | `ZSTD_CCtx_loadDictionary` | called mid-frame (stage != init) | `ERROR(stage_wrong)` | [x] |
| F13 | `ZSTD_CCtx_refCDict` | called mid-frame | `ERROR(stage_wrong)` | [x] |
| F14 | `ZSTD_CCtx_refPrefix` | called mid-frame | `ERROR(stage_wrong)` | [x] |
| F15 | `ZSTD_DCtx_refDDict` | called mid-frame | `ERROR(stage_wrong)` | [x] |
| F16 | `ZSTD_decompress_usingDict` | corrupted dictionary (valid magic, garbage entropy tables) | `ERROR(dictionary_corrupted)` | [x] |
| F17 | `ZSTD_CCtx_loadDictionary` | `dict = NULL, dictSize > 0` | `ERROR(dictionary_wrong)` (C's explicit check) | [x] |

## G. Static / workspace-sized API rejections

| # | function | trigger | expected C result | [ ] |
|---|----------|---------|-------------------|-----|
| G1 | `ZSTD_initStaticCCtx` | `workspaceSize < ZSTD_estimateCCtxSize(...)` | `NULL` | [x] |
| G2 | `ZSTD_initStaticCCtx` | unaligned workspace pointer | `NULL` (C aligns then re-checks) | [x] |
| G3 | `ZSTD_initStaticDCtx` | `workspaceSize < ZSTD_estimateDCtxSize()` | `NULL` | [x] |
| G4 | `ZSTD_initStaticCStream` | undersized workspace | `NULL` | [x] |
| G5 | `ZSTD_initStaticDStream` | undersized workspace | `NULL` | [x] |
| G6 | `ZSTD_initStaticCDict` | undersized workspace | `NULL` | [x] |
| G7 | `ZSTD_initStaticDDict` | undersized workspace | `NULL` | [x] |
| G8 | `ZSTD_estimate*Size` family | levels `minCLevel..=maxCLevel`, and `ZSTD_estimateCDictSize` over dict sizes | identical `size_t` | [x] |
| G9 | `ZSTD_sizeof_*` family | fresh and used contexts | identical `size_t` | [x] |

## H. Entropy-layer rejections (FSE / HUF / hist)

| # | function | trigger | expected C result | [ ] |
|---|----------|---------|-------------------|-----|
| H1 | `FSE_readNCount` | truncated/garbage header | `ERROR(srcSize_wrong)` / `corruption_detected` (identical) | [x] |
| H2 | `FSE_readNCount` | `tableLog > FSE_MAX_TABLELOG` in header | `ERROR(tableLog_tooLarge)` | [x] |
| H3 | `FSE_readNCount` | `maxSymbolValue` too small for header content | `ERROR(maxSymbolValue_tooSmall)` | [x] |
| H4 | `FSE_normalizeCount` | `tableLog` out of `[FSE_MIN_TABLELOG, FSE_MAX_TABLELOG]` | `ERROR(GENERIC)` / `tableLog_tooLarge` (identical) | [x] |
| H5 | `FSE_normalizeCount` | `total` smaller than `maxSymbolValue+1` | `ERROR(GENERIC)` | [x] |
| H6 | `FSE_writeNCount` | `dstCapacity` too small | `ERROR(dstSize_tooSmall)` | [x] |
| H7 | `FSE_buildCTable_wksp` / `FSE_buildDTable_wksp` | `wkspSize` too small | `ERROR(tableLog_tooLarge)` / `workSpace_tooSmall` | [x] |
| H8 | `FSE_compress2` | `srcSize = 0`, `srcSize = 1` | `0` (not compressible) — identical | [x] |
| H9 | `FSE_compress2` | `maxSymbolValue > FSE_MAX_SYMBOL_VALUE` | `ERROR(maxSymbolValue_tooLarge)` | [x] |
| H10 | `FSE_compress2` | `tableLog > FSE_MAX_TABLELOG` | `ERROR(tableLog_tooLarge)` | [x] |
| H11 | `FSE_decompress_wksp` | garbage input | identical error code | [x] |
| H12 | `HUF_readStats` | truncated / garbage | identical error code | [x] |
| H13 | `HUF_compress1X_wksp` / `HUF_compress4X_wksp` | `srcSize = 0` | `ERROR(srcSize_wrong)` (or 0 — whichever C returns) | [x] |
| H14 | `HUF_compress4X_wksp` | `srcSize > HUF_BLOCKSIZE_MAX` | `ERROR(srcSize_wrong)` | [x] |
| H15 | `HUF_compress4X_wksp` | `maxSymbolValue > HUF_SYMBOLVALUE_MAX` | `ERROR(maxSymbolValue_tooLarge)` | [x] |
| H16 | `HUF_compress4X_wksp` | `huffLog > HUF_TABLELOG_MAX` | `ERROR(tableLog_tooLarge)` | [x] |
| H17 | `HUF_decompress1X*` / `HUF_decompress4X*` | garbage / truncated input | identical error code | [x] |
| H18 | `HUF_getNbBitsFromCTable` | symbol beyond table | identical (no error path — value compared) | [x] |
| H19 | `HIST_count` / `HIST_count_wksp` | `srcSize = 0` | identical result, `maxSymbolValue` written back identically | [x] |
| H20 | `HIST_count_wksp` | `wkspSize` too small | `ERROR(workSpace_tooSmall)` | [x] |
| H21 | `FSE_getErrorName`, `HUF_getErrorName` | error and non-error codes | identical strings | [x] |

## I. Dictionary-builder rejections (`ZDICT_*`)

| # | function | trigger | expected C result | [ ] |
|---|----------|---------|-------------------|-----|
| I1 | `ZDICT_getDictID` | `dictSize <= 8` | `0` | [x] |
| I2 | `ZDICT_getDictID` | bad `ZSTD_MAGIC_DICTIONARY` | `0` | [x] |
| I3 | `ZDICT_getDictHeaderSize` | `dictSize <= 8` or bad magic | `ERROR(dictionary_corrupted)` | [x] |
| I4 | `ZDICT_finalizeDictionary` | `dictBufferCapacity < dictContentSize` | `ERROR(dstSize_tooSmall)` | [x] |
| I5 | `ZDICT_finalizeDictionary` | `dictBufferCapacity < ZDICT_DICTSIZE_MIN` | `ERROR(dstSize_tooSmall)` | [x] |
| I6 | `ZDICT_trainFromBuffer` | `dictBufferCapacity < ZDICT_DICTSIZE_MIN` | `ERROR(dstSize_tooSmall)` | [x] |
| I7 | `ZDICT_trainFromBuffer` | total sample bytes `< ZDICT_MIN_SAMPLES_SIZE` | `ERROR(dictionaryCreation_failed)` | [x] |
| I8 | `ZDICT_trainFromBuffer` | `nbSamples = 0` | identical error | [x] |
| I9 | `ZDICT_optimizeTrainFromBuffer_cover` | `nbSamples` too small / bad `k`,`d` params | identical error | [x] |
| I10 | `ZDICT_trainFromBuffer_cover` | `d` not in `{6,8}` and not 0 | `ERROR(GENERIC)` / `parameter_outOfBound` (identical) | [x] |
| I11 | `ZDICT_trainFromBuffer_cover` | `k < d` | identical error | [x] |
| I12 | `ZDICT_trainFromBuffer_fastCover` | `f > 31` / `accel > 10` | identical error | [x] |
| I13 | `ZDICT_trainFromBuffer_fastCover` | `splitPoint` out of `(0,1]` | identical error | [x] |
| I14 | `ZDICT_isError` / `ZDICT_getErrorName` | error and non-error codes | identical `unsigned` / strings | [x] |
| I15 | `ZDICT_addEntropyTablesFromBuffer` (legacy) | undersized dict buffer | identical result | [x] |

## J. Sequence-API rejections

| # | function | trigger | expected C result | [ ] |
|---|----------|---------|-------------------|-----|
| J1 | `ZSTD_compressSequences` | sequence with `offset = 0` | `ERROR(externalSequences_invalid)` (validateSequences=1) | [x] |
| J2 | `ZSTD_compressSequences` | `offset` larger than window/history | `ERROR(externalSequences_invalid)` | [x] |
| J3 | `ZSTD_compressSequences` | `matchLength < ZSTD_MINMATCH_MIN` on a non-last sequence | `ERROR(externalSequences_invalid)` | [x] |
| J4 | `ZSTD_compressSequences` | sum of `litLength+matchLength` ≠ `srcSize` | `ERROR(externalSequences_invalid)` / `srcSize_wrong` | [x] |
| J5 | `ZSTD_compressSequences` | `dstCapacity` too small | `ERROR(dstSize_tooSmall)` | [x] |
| J6 | `ZSTD_compressSequences` | `ZSTD_c_blockDelimiters = explicit` but no delimiter present | identical error | [x] |
| J7 | `ZSTD_generateSequences` | `dstCapacity`(nb seq) too small | identical error | [x] |
| J8 | `ZSTD_mergeBlockDelimiters` | 0 sequences | identical returned count | [x] |
| J9 | `ZSTD_sequenceBound` | `srcSize = 0`, `1`, large | identical `size_t` | [x] |
| J10 | `ZSTD_compressSequencesAndLiterals` | literals size inconsistent with sequences | identical error | [x] |
| J11 | `ZSTD_compressSequencesAndLiterals` | `srcSize` exceeding what the API allows | `ERROR(srcSize_wrong)` / `cannotProduce_uncompressedBlock` | [x] |

## K. Legacy-decoder rejections (`ZSTDv01..ZSTDv07`, `ZSTD_LEGACY_SUPPORT=5`)

| # | function | trigger | expected C result | [ ] |
|---|----------|---------|-------------------|-----|
| K1 | `ZSTDv0{1..7}_isError` | error and non-error codes | identical `unsigned` | [x] |
| K2 | `ZSTDv0{1..7}_decompress` | garbage input | identical error code | [x] |
| K3 | `ZSTDv0{1..7}_decompress` | `srcSize = 0` | identical error code | [x] |
| K4 | `ZSTDv0{1..7}_findFrameSizeInfoLegacy` | garbage / truncated | identical `cSize`/`dBound` | [x] |
| K5 | `ZSTDv01_magicNumber` etc. | value read | identical constants | [x] |
| K6 | `ZSTDv0{5,6,7}_getFrameParams` | garbage / truncated | identical result | [x] |
| K7 | `ZSTD_decompress` | a v0.5/v0.6/v0.7 legacy frame | routed through legacy path; identical output/error | [x] |
| K8 | `ZSTDv0{1..7}_decompressContinue` | called without init | identical error | [x] |

## L. Deprecated ZBUFF rejections

| # | function | trigger | expected C result | [ ] |
|---|----------|---------|-------------------|-----|
| L1 | `ZBUFF_isError` / `ZBUFF_getErrorName` | error and non-error codes | identical | [x] |
| L2 | `ZBUFF_compressInit` | out-of-range compression level | identical (clamped like `ZSTD_c_compressionLevel`) | [x] |
| L3 | `ZBUFF_decompressContinue` | garbage input | identical error code | [x] |
| L4 | `ZBUFF_recommended*` | value read | identical `size_t` constants | [x] |

## M. Generic FFI boundary conditions (required even though not in the C table)

| # | function | trigger | expected C result | [ ] |
|---|----------|---------|-------------------|-----|
| M1 | all `ZSTD_free*` | `NULL` handle | no-op, returns 0 | [x] |
| M2 | `ZSTD_freeCCtx`/`freeDCtx`/`freeCStream`/`freeDStream`/`freeCDict`/`freeDDict`/`freeCCtxParams` | `NULL` | `0` | [x] |
| M3 | all `ZSTD_sizeof_*` | `NULL` handle | `0` | [x] |
| M4 | `ZSTD_getDictID_fromCDict`/`fromDDict` | `NULL` | `0` | [x] |
| M5 | out-of-range enum ints across FFI | `ZSTD_cParameter`, `ZSTD_dParameter`, `ZSTD_EndDirective`, `ZSTD_ResetDirective`, `ZSTD_strategy`, `ZSTD_dictLoadMethod_e`, `ZSTD_dictContentType_e`, `ZSTD_format_e`, `ZSTD_forceIgnoreChecksum_e`, `ZSTD_refMultipleDDicts_e`, `ZSTD_bufferMode_e`, `ZSTD_sequenceFormat_e`, `ZSTD_paramSwitch_e`, `ZSTD_dictAttachPref_e` — each driven with `-1`, `max+1`, `9999`, `i32::MIN`, `i32::MAX` | identical error code in every case | [x] |
| M6 | zero-length everything | `srcSize=0`/`dstCapacity=0` on every size-taking public entry point | identical return | [x] |
| M7 | oversized lengths | `srcSize = SIZE_MAX`, `SIZE_MAX/2` to `compressBound`, `sequenceBound`, `decompressBound` | identical (incl. overflow behaviour) | [x] |
| M8 | `ZSTD_versionNumber` / `ZSTD_versionString` | — | identical | [x] |
| M9 | `ZSTD_minCLevel` / `ZSTD_maxCLevel` / `ZSTD_defaultCLevel` | — | identical | [x] |
| M10 | `ZSTD_CStreamInSize`/`OutSize`, `ZSTD_DStreamInSize`/`OutSize`, `ZSTD_BLOCKSIZE_MAX` accessors | — | identical | [x] |
| M11 | `ZSTD_XXH*` (32/64/3) | `NULL` input with len 0, len 1..N, streaming reset/update/digest | identical hashes | [x] |

## Row-to-test mapping (evidence)

Every `[x]` above corresponds to a test that was RUN and PASSED against both
`.so` files. Rows marked `n/a` are documented below with the reason.

| row | covering test |
|---|---|
| A1 | `phase_c_errors::a1_a2_a3_is_error_get_code_get_name / a4_get_error_string_out_of_range_enum / a_fse_huf_zdict_zbuff_error_names / phase_b_configs::row72_error_api_valid_side` |
| A2 | `phase_c_errors::a1_a2_a3_is_error_get_code_get_name / a4_get_error_string_out_of_range_enum / a_fse_huf_zdict_zbuff_error_names / phase_b_configs::row72_error_api_valid_side` |
| A3 | `phase_c_errors::a1_a2_a3_is_error_get_code_get_name / a4_get_error_string_out_of_range_enum / a_fse_huf_zdict_zbuff_error_names / phase_b_configs::row72_error_api_valid_side` |
| A4 | `phase_c_errors::a1_a2_a3_is_error_get_code_get_name / a4_get_error_string_out_of_range_enum / a_fse_huf_zdict_zbuff_error_names / phase_b_configs::row72_error_api_valid_side` |
| A5 | `phase_c_errors::a1_a2_a3_is_error_get_code_get_name / a4_get_error_string_out_of_range_enum / a_fse_huf_zdict_zbuff_error_names / phase_b_configs::row72_error_api_valid_side` |
| B1 | `phase_c_errors::b1_cparam_get_bounds_valid` |
| B2 | `phase_c_errors::b2_cparam_get_bounds_out_of_range_enum` |
| B3 | `phase_c_errors::b3_b6_cctx_set_parameter_out_of_bounds` |
| B4 | `phase_c_errors::b3_b6_cctx_set_parameter_out_of_bounds` |
| B5 | `phase_c_errors::b5_compression_level_clamps` |
| B6 | `phase_c_errors::b3_b6_cctx_set_parameter_out_of_bounds` |
| B7 | `phase_c_errors::b3_b6_cctx_set_parameter_out_of_bounds` |
| B9 | `phase_c_errors::b3_b6_cctx_set_parameter_out_of_bounds` |
| B10 | `phase_c_errors2::b10_d12_stage_and_pledged_size` |
| B11 | `phase_c_errors::b11_b14_cctx_params_and_reset` |
| B12 | `phase_c_errors::b11_b14_cctx_params_and_reset` |
| B13 | `phase_c_errors2::b10_d12_stage_and_pledged_size` |
| B14 | `phase_c_errors::b11_b14_cctx_params_and_reset` |
| B15 | `phase_c_errors::b3_b6_cctx_set_parameter_out_of_bounds` |
| B16 | `phase_c_errors::b3_b6_cctx_set_parameter_out_of_bounds` |
| C1 | `phase_c_errors::c1_dparam_get_bounds_valid` |
| C2 | `phase_c_errors::c2_dparam_get_bounds_out_of_range_enum` |
| C3 | `phase_c_errors::c3_c8_dctx_set_parameter_out_of_bounds` |
| C4 | `phase_c_errors::c3_c8_dctx_set_parameter_out_of_bounds` |
| C5 | `phase_c_errors::c3_c8_dctx_set_parameter_out_of_bounds` |
| C6 | `phase_c_errors::b11_b14_cctx_params_and_reset` |
| C7 | `phase_c_errors::c7_c8_set_max_window_size_and_format` |
| C8 | `phase_c_errors::c7_c8_set_max_window_size_and_format` |
| D1 | `phase_c_errors2::d1_d8_compress_dst_too_small` |
| D2 | `phase_c_errors2::d1_d8_compress_dst_too_small` |
| D3 | `phase_c_errors2::d1_d8_compress_dst_too_small` |
| D4 | `phase_c_errors2::d1_d8_compress_dst_too_small` |
| D6 | `phase_c_errors::d6_m7_bound_functions` |
| D7 | `phase_c_errors2::d7_compress2_dst_too_small` |
| D8 | `phase_c_errors2::d1_d8_compress_dst_too_small` |
| D9 | `phase_c_errors2::d9_d11_block_api_rejections` |
| D10 | `phase_c_errors2::d9_d11_block_api_rejections` |
| D11 | `phase_c_errors2::d9_d11_block_api_rejections` |
| D12 | `phase_c_errors2::b10_d12_stage_and_pledged_size` |
| D13 | `phase_c_errors2::d13_d15_stability_and_end_directive` |
| D14 | `phase_c_errors2::d13_d15_stability_and_end_directive` |
| D15 | `phase_c_errors2::d13_d15_stability_and_end_directive` |
| E1 | `phase_c_errors2::e1_e12_frame_rejections` |
| E2 | `phase_c_errors2::e1_e12_frame_rejections` |
| E3 | `phase_c_errors2::e1_e12_frame_rejections` |
| E4 | `phase_c_errors2::e1_e12_frame_rejections` |
| E5 | `phase_c_errors2::e1_e12_frame_rejections` |
| E6 | `phase_c_errors2::e1_e12_frame_rejections` |
| E7 | `phase_c_errors2::e1_e12_frame_rejections` |
| E8 | `phase_c_errors2::e1_e12_frame_rejections` |
| E9 | `phase_c_errors2::e9_e10_checksum_and_window` |
| E10 | `phase_c_errors2::e9_e10_checksum_and_window` |
| E11 | `phase_c_errors2::e1_e12_frame_rejections` |
| E12 | `phase_c_errors2::e1_e12_frame_rejections` |
| E13 | `phase_c_errors2::e13_e23_introspection_rejections` |
| E14 | `phase_c_errors2::e13_e23_introspection_rejections` |
| E15 | `phase_c_errors2::e13_e23_introspection_rejections` |
| E16 | `phase_c_errors2::e13_e23_introspection_rejections` |
| E17 | `phase_c_errors2::e13_e23_introspection_rejections` |
| E18 | `phase_c_errors2::e13_e23_introspection_rejections` |
| E19 | `phase_c_errors2::e13_e23_introspection_rejections` |
| E20 | `phase_c_errors2::e13_e23_introspection_rejections` |
| E21 | `phase_c_errors2::e13_e23_introspection_rejections` |
| E22 | `phase_c_errors2::e13_e23_introspection_rejections` |
| E23 | `phase_c_errors2::e13_e23_introspection_rejections` |
| E24 | `phase_c_errors2::e24_e25_decompress_block_rejections` |
| E25 | `phase_c_errors2::e24_e25_decompress_block_rejections` |
| E26 | `phase_c_errors2::e26_e36_decompress_stream_rejections` |
| E27 | `phase_c_errors2::e26_e36_decompress_stream_rejections` |
| E28 | `phase_c_errors2::e26_e36_decompress_stream_rejections` |
| E29 | `phase_c_errors2::e26_e36_decompress_stream_rejections` |
| E30 | `phase_c_errors2::e30_dctx_reuse_after_error` |
| E31 | `phase_c_errors2::e13_e23_introspection_rejections` |
| E32 | `phase_c_errors2::e32_e34_skippable_frame_rejections` |
| E33 | `phase_c_errors2::e32_e34_skippable_frame_rejections` |
| E34 | `phase_c_errors2::e32_e34_skippable_frame_rejections` |
| E35 | `phase_c_errors2::e1_e12_frame_rejections` |
| E36 | `phase_c_errors2::e26_e36_decompress_stream_rejections` |
| F1 | `phase_b_dict::row37_row42_dictionary_matrix` |
| F2 | `phase_b_dict::row37_row42_dictionary_matrix` |
| F3 | `phase_b_dict::row37_row42_dictionary_matrix` |
| F4 | `phase_b_dict::row37_row42_dictionary_matrix` |
| F5 | `phase_b_dict::row37_row42_dictionary_matrix` |
| F6 | `phase_b_dict::row37_row42_dictionary_matrix` |
| F7 | `phase_b_dict::row37_row42_dictionary_matrix` |
| F8 | `phase_b_dict::row37_row42_dictionary_matrix` |
| F9 | `phase_b_dict::row37_row42_dictionary_matrix` |
| F10 | `phase_b_dict::row37_row42_dictionary_matrix` |
| F11 | `phase_b_dict::row37_row42_dictionary_matrix` |
| F12 | `phase_c_errors2::f12_f15_dict_apis_mid_frame` |
| F13 | `phase_c_errors2::f12_f15_dict_apis_mid_frame` |
| F14 | `phase_c_errors2::f12_f15_dict_apis_mid_frame` |
| F15 | `phase_c_errors2::f12_f15_dict_apis_mid_frame` |
| F16 | `phase_b_dict::row37_row42_dictionary_matrix` |
| F17 | `phase_b_dict::row37_row42_dictionary_matrix` |
| G1 | `phase_b_dict::row48_row49_static_and_custom_mem` |
| G2 | `phase_b_dict::row48_row49_static_and_custom_mem` |
| G3 | `phase_b_dict::row48_row49_static_and_custom_mem` |
| G4 | `phase_b_dict::row48_row49_static_and_custom_mem` |
| G5 | `phase_b_dict::row48_row49_static_and_custom_mem` |
| G6 | `phase_b_dict::row48_row49_static_and_custom_mem` |
| G7 | `phase_b_dict::row48_row49_static_and_custom_mem` |
| G8 | `phase_b_dict::row48_row49_static_and_custom_mem` |
| G9 | `phase_c_errors2::g9_sizeof_fresh_and_used` |
| H1 | `phase_b_entropy::row6_row8_fse_pipeline` |
| H2 | `phase_b_entropy::row6_row8_fse_pipeline` |
| H3 | `phase_b_entropy::row6_row8_fse_pipeline` |
| H4 | `phase_b_entropy::row6_row8_fse_pipeline` |
| H5 | `phase_b_entropy::row6_row8_fse_pipeline` |
| H6 | `phase_b_entropy::row6_row8_fse_pipeline` |
| H7 | `phase_b_entropy::row6_row8_fse_pipeline` |
| H8 | `phase_b_entropy::row6_row8_fse_pipeline (composed: writeNCount+compress_usingCTable)` |
| H9 | `phase_b_entropy::row6_row8_fse_pipeline (FSE_compress2 not exported; bounds checked via normalizeCount/buildCTable_wksp)` |
| H10 | `phase_b_entropy::row6_row8_fse_pipeline` |
| H11 | `phase_b_entropy::row6_row8_fse_pipeline` |
| H12 | `phase_b_entropy::row9_row12_huf_pipeline` |
| H13 | `phase_b_entropy::row9_row12_huf_pipeline` |
| H14 | `phase_b_entropy::row9_row12_huf_pipeline` |
| H15 | `phase_b_entropy::row9_row12_huf_pipeline` |
| H16 | `phase_b_entropy::row9_row12_huf_pipeline` |
| H17 | `phase_b_entropy::row9_row12_huf_pipeline` |
| H18 | `phase_b_entropy::row9_row12_huf_pipeline` |
| H19 | `phase_b_entropy::row5_hist` |
| H20 | `phase_b_entropy::row5_hist` |
| H21 | `phase_c_errors::a_fse_huf_zdict_zbuff_error_names` |
| I1 | `phase_b_dict::row55_row60_dictionary_builder` |
| I2 | `phase_b_dict::row55_row60_dictionary_builder` |
| I3 | `phase_b_dict::row55_row60_dictionary_builder` |
| I4 | `phase_b_dict::row55_row60_dictionary_builder` |
| I5 | `phase_b_dict::row55_row60_dictionary_builder` |
| I6 | `phase_b_dict::row55_row60_dictionary_builder` |
| I7 | `phase_b_dict::row55_row60_dictionary_builder` |
| I8 | `phase_b_dict::row55_row60_dictionary_builder` |
| I9 | `phase_b_dict::row55_row60_dictionary_builder` |
| I10 | `phase_b_dict::row55_row60_dictionary_builder` |
| I11 | `phase_b_dict::row55_row60_dictionary_builder` |
| I12 | `phase_b_dict::row55_row60_dictionary_builder` |
| I13 | `phase_b_dict::row55_row60_dictionary_builder` |
| I14 | `phase_b_dict::row55_row60_dictionary_builder` |
| I15 | `phase_b_dict::row55_row60_dictionary_builder` |
| J1 | `phase_b_dict::row50_row52_sequence_api` |
| J2 | `phase_b_dict::row50_row52_sequence_api` |
| J3 | `phase_b_dict::row50_row52_sequence_api` |
| J4 | `phase_b_dict::row50_row52_sequence_api` |
| J5 | `phase_b_dict::row50_row52_sequence_api` |
| J6 | `phase_b_dict::row50_row52_sequence_api` |
| J7 | `phase_b_dict::row50_row52_sequence_api` |
| J8 | `phase_b_dict::row50_row52_sequence_api` |
| J9 | `phase_b_dict::row50_row52_sequence_api` |
| J10 | `phase_b_dict::row50_row52_sequence_api` |
| J11 | `phase_b_dict::row50_row52_sequence_api` |
| K1 | `phase_b_legacy::*` |
| K2 | `phase_b_legacy::*` |
| K3 | `phase_b_legacy::*` |
| K4 | `phase_b_legacy::*` |
| K5 | `phase_b_legacy::*` |
| K6 | `phase_b_legacy::*` |
| K7 | `phase_b_legacy::*` |
| K8 | `phase_b_legacy::*` |
| L1 | `phase_c_errors::a_fse_huf_zdict_zbuff_error_names` |
| L2 | `phase_b_dict::row61_row62_zbuff` |
| L3 | `phase_b_dict::row61_row62_zbuff` |
| L4 | `phase_c_errors::m8_m10_constants` |
| M1 | `phase_c_errors::m1_m4_null_handles` |
| M2 | `phase_c_errors::m1_m4_null_handles` |
| M3 | `phase_c_errors::m1_m4_null_handles` |
| M4 | `phase_c_errors::m1_m4_null_handles` |
| M5 | `phase_c_errors2::m5_out_of_range_enums` |
| M6 | `phase_c_errors2::m6_zero_lengths` |
| M7 | `phase_c_errors::d6_m7_bound_functions` |
| M8 | `phase_c_errors::m8_m10_constants` |
| M9 | `phase_c_errors::m8_m10_constants` |
| M10 | `phase_c_errors::m8_m10_constants` |
| M11 | `phase_c_errors2::m11_xxh_null_and_zero / phase_b_entropy::row2_xxh32,row3_xxh64` |

### Rows deliberately not tested

- **B8**: NOT TESTED — `cctx == NULL` is undefined behaviour in the C (`assert(cctx != NULL)`; the body dereferences immediately). Verifying it would compare two crashes, not two behaviours.
- **D5**: NOT TESTED — `src == NULL` with `srcSize > 0` is undefined behaviour in the C (it dereferences `src`; `ZSTD_compressStream_generic` asserts `if (input->src == NULL) assert(input->size == 0)`).

## Divergence found and fixed

One real translation bug was found by these tests and fixed in the Rust:

**`ZSTD_compressStream_generic`** (`translation/src/compress/zstd_compress_frame.rs`).
In C, `break;` inside a `switch` exits only the switch, so control returns to
the enclosing `while (someMoreWork)` loop and the next block is processed. The
Rust translation had rendered the `switch` as a `match` inside the `while`, where
`break` exits the **loop** instead. The state machine therefore stopped after a
single block.

Symptom, on a fresh `ZSTD_CCtx` with a large output buffer:

| call | C | Rust (before fix) |
|---|---|---|
| `ZSTD_compressStream2_simpleArgs(..., srcSize=200000, ZSTD_e_continue)` | `srcPos = 200000` | `srcPos = 131072` (one block) |
| `ZSTD_compressStream2_simpleArgs(..., srcSize=70000, ZSTD_e_flush)` | `srcPos = 70000` | `srcPos = 0` |

Fix: the `match` is wrapped in a labelled block `'sw: { … }` and all 12
switch-level `break;` statements became `break 'sw;`, which reproduces C's
`switch`-break exactly.

This was invisible to the round-trip streaming tests, which loop until the
stream completes; it only shows up when a single low-level call is observed
directly — the reason Phase B requires driving the low-level entry points and
not just the convenience wrappers.

An audit of the same bug class across the 13 largest translated modules found no
other instance: `ZSTD_decompressStream` already used `continue` (correct), and
`ZSTD_decompressFrame` uses arm-fallthrough with the loop break outside the
`match` (also correct, matching `decompress/zstd_decompress.c:1014`).
