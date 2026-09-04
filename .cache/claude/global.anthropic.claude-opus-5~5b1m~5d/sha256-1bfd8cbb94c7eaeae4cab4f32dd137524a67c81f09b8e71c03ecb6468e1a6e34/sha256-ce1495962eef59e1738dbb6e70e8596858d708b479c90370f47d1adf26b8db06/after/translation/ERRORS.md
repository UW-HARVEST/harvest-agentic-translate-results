# ERRORS.md - the ERROR-SURFACE TABLE

Mechanically extracted from every rejection site in `c_src/src/**.{c,h}` with
`wk/extract2.py`: every `RETURN_ERROR(...)`, `RETURN_ERROR_IF(...)`,
`return ERROR(...)`, `return NULL;`, `return -1;` and every `... = ERROR(...)`
assignment. `FORWARD_IF_ERROR` sites are *propagation*, not distinct
rejections, so they are excluded (the error they forward is a row at its
origin). One row per distinct rejection site.

**Total rows: 1269**

## Notes on `assert()`

The CMake build defines `DEBUGLEVEL` nowhere, so `common/debug.h` expands
`assert(x)` to `((void)0)`: the 966 `assert()` sites in the library are
**no-ops** and are therefore not rejections. The single exception is
`dictBuilder/divsufsort.c`, which `#include <assert.h>` directly (it is the
only object file that imports `__assert_fail`). Those asserts are
*internal* invariants of the suffix-sort over a `[0,n)` byte array reached
only from `ZDICT_trainFromBuffer*`; they cannot be tripped from the public
API with any input, so the Rust port dropping them is not observable.
They are listed as rows 1-2 of the `dictBuilder/divsufsort.c` section only
because the extractor found two `return -1;` guards there.

## Error code frequency

| error code | sites |
|---|---|
| `corruption_detected` | 364 |
| `srcSize_wrong` | 202 |
| `dstSize_tooSmall` | 130 |
| `GENERIC` | 127 |
| `NULL` | 80 |
| `dictionary_corrupted` | 73 |
| `memory_allocation` | 69 |
| `tableLog_tooLarge` | 45 |
| `parameter_unsupported` | 19 |
| `stage_wrong` | 18 |
| `externalSequences_invalid` | 18 |
| `prefix_unknown` | 16 |
| `frameParameter_unsupported` | 14 |
| `parameter_outOfBound` | 13 |
| `maxSymbolValue_tooLarge` | 11 |
| `maxSymbolValue_tooSmall` | 9 |
| `-1` | 6 |
| `workSpace_tooSmall` | 5 |
| `sequenceProducer_failed` | 5 |
| `dictionary_wrong` | 5 |
| `init_missing` | 5 |
| `(computed)` | 4 |
| `err` | 4 |
| `stabilityCondition_notRespected` | 4 |
| `frameParameter_windowTooLarge` | 4 |
| `checksum_wrong` | 4 |
| `parameter_combination_unsupported` | 3 |
| `dictionaryCreation_failed` | 3 |
| `dstBuffer_null` | 2 |
| `version_unsupported` | 2 |
| `cannotProduce_uncompressedBlock` | 1 |
| `dstBuffer_wrong` | 1 |
| `noForwardProgress_destFull` | 1 |
| `noForwardProgress_inputEmpty` | 1 |
| `literals_headerWrong` | 1 |

## Covering differential tests

| covering test (file in `translation/tests/`) | rows |
|---|---|
| `phase_b_legacy` | 743 |
| `phase_c_decompress` | 196 |
| `phase_c_compress` | 130 |
| `phase_c_entropy` | 63 |
| `phase_c_dictbuilder` | 63 |
| `phase_c_misc` | 53 |
| `phase_c_params` | 21 |

## Rows

Legend: `expected C result` is the value the C function returns when the
trigger holds. `ZSTD_error_X` means the function returns the `size_t`
sentinel `(size_t)-X` (i.e. `ZSTD_isError() != 0` and
`ZSTD_getErrorCode() == ZSTD_error_X`).

### `common/bitstream.h`

| # | function | trigger (the exact invalid input/condition) | expected C result | covering test | [x] |
|---|----------|---------------------------------------------|-------------------|---------------|-----|
| 1 | `BIT_initCStream` (L158) | `return ERROR(dstSize_tooSmall)` | `ZSTD_error_dstSize_tooSmall` | `phase_c_entropy` | [x] |
| 2 | `BIT_initDStream` (L256) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_c_entropy` | [x] |
| 3 | `BIT_initDStream` (L266) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_c_entropy` | [x] |
| 4 | `BIT_initDStream` (L294) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_c_entropy` | [x] |

### `common/entropy_common.c`

| # | function | trigger (the exact invalid input/condition) | expected C result | covering test | [x] |
|---|----------|---------------------------------------------|-------------------|---------------|-----|
| 5 | `FSE_readNCount_body` (L64) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_c_entropy` | [x] |
| 6 | `FSE_readNCount_body` (L73) | `return ERROR(tableLog_tooLarge)` | `ZSTD_error_tableLog_tooLarge` | `phase_c_entropy` | [x] |
| 7 | `FSE_readNCount_body` (L179) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_c_entropy` | [x] |
| 8 | `FSE_readNCount_body` (L181) | `return ERROR(maxSymbolValue_tooSmall)` | `ZSTD_error_maxSymbolValue_tooSmall` | `phase_c_entropy` | [x] |
| 9 | `FSE_readNCount_body` (L182) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_c_entropy` | [x] |
| 10 | `HUF_readStats` (L254) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_c_entropy` | [x] |
| 11 | `HUF_readStats` (L261) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_c_entropy` | [x] |
| 12 | `HUF_readStats` (L262) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_c_entropy` | [x] |
| 13 | `HUF_readStats` (L270) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_c_entropy` | [x] |
| 14 | `HUF_readStats` (L280) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_c_entropy` | [x] |
| 15 | `HUF_readStats` (L284) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_c_entropy` | [x] |
| 16 | `HUF_readStats` (L288) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_c_entropy` | [x] |
| 17 | `HUF_readStats` (L295) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_c_entropy` | [x] |
| 18 | `HUF_readStats` (L301) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_c_entropy` | [x] |

### `common/error_private.h`

| # | function | trigger (the exact invalid input/condition) | expected C result | covering test | [x] |
|---|----------|---------------------------------------------|-------------------|---------------|-----|
| 19 | `_force_has_format_string` (L111) | `RETURN_ERROR()` | computed error | `phase_c_misc` | [x] |
| 20 | `_force_has_format_string` (L113) | `cond` | `ZSTD_error_err` | `phase_c_misc` | [x] |
| 21 | `_force_has_format_string` (L121) | `return ERROR(err)` | `ZSTD_error_err` | `phase_c_misc` | [x] |
| 22 | `_force_has_format_string` (L130) | `RETURN_ERROR(err, ...)` | `ZSTD_error_err` | `phase_c_misc` | [x] |
| 23 | `_force_has_format_string` (L137) | `return ERROR(err)` | `ZSTD_error_err` | `phase_c_misc` | [x] |

### `common/fse_decompress.c`

| # | function | trigger (the exact invalid input/condition) | expected C result | covering test | [x] |
|---|----------|---------------------------------------------|-------------------|---------------|-----|
| 24 | `FSE_buildDTable_internal` (L70) | `return ERROR(maxSymbolValue_tooLarge)` | `ZSTD_error_maxSymbolValue_tooLarge` | `phase_c_entropy` | [x] |
| 25 | `FSE_buildDTable_internal` (L71) | `return ERROR(maxSymbolValue_tooLarge)` | `ZSTD_error_maxSymbolValue_tooLarge` | `phase_c_entropy` | [x] |
| 26 | `FSE_buildDTable_internal` (L72) | `return ERROR(tableLog_tooLarge)` | `ZSTD_error_tableLog_tooLarge` | `phase_c_entropy` | [x] |
| 27 | `FSE_buildDTable_internal` (L146) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_c_entropy` | [x] |
| 28 | `FSE_decompress_usingDTable_generic` (L193) | `BIT_reloadDStream(&bitD)==BIT_DStream_overflow` | `ZSTD_error_corruption_detected` | `phase_c_entropy` | [x] |
| 29 | `FSE_decompress_usingDTable_generic` (L220) | `return ERROR(dstSize_tooSmall)` | `ZSTD_error_dstSize_tooSmall` | `phase_c_entropy` | [x] |
| 30 | `FSE_decompress_usingDTable_generic` (L227) | `return ERROR(dstSize_tooSmall)` | `ZSTD_error_dstSize_tooSmall` | `phase_c_entropy` | [x] |
| 31 | `FSE_decompress_wksp_body` (L258) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_c_entropy` | [x] |
| 32 | `FSE_decompress_wksp_body` (L267) | `return ERROR(tableLog_tooLarge)` | `ZSTD_error_tableLog_tooLarge` | `phase_c_entropy` | [x] |
| 33 | `FSE_decompress_wksp_body` (L273) | `return ERROR(tableLog_tooLarge)` | `ZSTD_error_tableLog_tooLarge` | `phase_c_entropy` | [x] |

### `common/pool.c`

| # | function | trigger (the exact invalid input/condition) | expected C result | covering test | [x] |
|---|----------|---------------------------------------------|-------------------|---------------|-----|
| 34 | `POOL_thread` (L69) | `return NULL;` | `NULL` | `phase_c_misc` | [x] |
| 35 | `POOL_create_advanced` (L120) | `return NULL;` | `NULL` | `phase_c_misc` | [x] |
| 36 | `POOL_create_advanced` (L123) | `return NULL;` | `NULL` | `phase_c_misc` | [x] |
| 37 | `POOL_create_advanced` (L139) | `return NULL;` | `NULL` | `phase_c_misc` | [x] |
| 38 | `POOL_create_advanced` (L147) | `return NULL;` | `NULL` | `phase_c_misc` | [x] |
| 39 | `POOL_create_advanced` (L154) | `return NULL;` | `NULL` | `phase_c_misc` | [x] |

### `common/threading.c`

| # | function | trigger (the exact invalid input/condition) | expected C result | covering test | [x] |
|---|----------|---------------------------------------------|-------------------|---------------|-----|
| 40 | `ZSTD_pthread_create` (L76) | `return -1;` | `-1` | `phase_c_misc` | [x] |
| 41 | `ZSTD_pthread_create` (L86) | `return -1;` | `-1` | `phase_c_misc` | [x] |
| 42 | `ZSTD_pthread_create` (L91) | `return -1;` | `-1` | `phase_c_misc` | [x] |

### `common/xxhash.h`

| # | function | trigger (the exact invalid input/condition) | expected C result | covering test | [x] |
|---|----------|---------------------------------------------|-------------------|---------------|-----|
| 43 | `XXH_malloc` (L2315) | `return NULL;` | `NULL` | `phase_c_misc` | [x] |
| 44 | `XXH_alignedMalloc` (L6116) | `return NULL;` | `NULL` | `phase_c_misc` | [x] |
| 45 | `XXH3_createState` (L6146) | `return NULL;` | `NULL` | `phase_c_misc` | [x] |

### `compress/fse_compress.c`

| # | function | trigger (the exact invalid input/condition) | expected C result | covering test | [x] |
|---|----------|---------------------------------------------|-------------------|---------------|-----|
| 46 | `FSE_buildCTable_wksp` (L87) | `return ERROR(tableLog_tooLarge)` | `ZSTD_error_tableLog_tooLarge` | `phase_c_entropy` | [x] |
| 47 | `FSE_NCountWriteBound` (L269) | `return ERROR(dstSize_tooSmall)` | `ZSTD_error_dstSize_tooSmall` | `phase_c_entropy` | [x] |
| 48 | `FSE_NCountWriteBound` (L284) | `return ERROR(dstSize_tooSmall)` | `ZSTD_error_dstSize_tooSmall` | `phase_c_entropy` | [x] |
| 49 | `FSE_NCountWriteBound` (L301) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_c_entropy` | [x] |
| 50 | `FSE_NCountWriteBound` (L306) | `return ERROR(dstSize_tooSmall)` | `ZSTD_error_dstSize_tooSmall` | `phase_c_entropy` | [x] |
| 51 | `FSE_NCountWriteBound` (L315) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_c_entropy` | [x] |
| 52 | `FSE_NCountWriteBound` (L320) | `return ERROR(dstSize_tooSmall)` | `ZSTD_error_dstSize_tooSmall` | `phase_c_entropy` | [x] |
| 53 | `FSE_writeNCount` (L333) | `return ERROR(tableLog_tooLarge)` | `ZSTD_error_tableLog_tooLarge` | `phase_c_entropy` | [x] |
| 54 | `FSE_writeNCount` (L334) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_c_entropy` | [x] |
| 55 | `FSE_normalizeM2` (L457) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_c_entropy` | [x] |
| 56 | `FSE_normalizeCount` (L471) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_c_entropy` | [x] |
| 57 | `FSE_normalizeCount` (L472) | `return ERROR(tableLog_tooLarge)` | `ZSTD_error_tableLog_tooLarge` | `phase_c_entropy` | [x] |
| 58 | `FSE_normalizeCount` (L473) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_c_entropy` | [x] |

### `compress/hist.c`

| # | function | trigger (the exact invalid input/condition) | expected C result | covering test | [x] |
|---|----------|---------------------------------------------|-------------------|---------------|-----|
| 59 | `HIST_count_parallel_wksp` (L138) | `return ERROR(maxSymbolValue_tooSmall)` | `ZSTD_error_maxSymbolValue_tooSmall` | `phase_c_entropy` | [x] |
| 60 | `HIST_countFast_wksp` (L156) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_c_entropy` | [x] |
| 61 | `HIST_countFast_wksp` (L157) | `return ERROR(workSpace_tooSmall)` | `ZSTD_error_workSpace_tooSmall` | `phase_c_entropy` | [x] |
| 62 | `HIST_count_wksp` (L168) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_c_entropy` | [x] |
| 63 | `HIST_count_wksp` (L169) | `return ERROR(workSpace_tooSmall)` | `ZSTD_error_workSpace_tooSmall` | `phase_c_entropy` | [x] |

### `compress/huf_compress.c`

| # | function | trigger (the exact invalid input/condition) | expected C result | covering test | [x] |
|---|----------|---------------------------------------------|-------------------|---------------|-----|
| 64 | `HUF_alignUpWorkspace` (L127) | `return NULL;` | `NULL` | `phase_c_entropy` | [x] |
| 65 | `HUF_alignUpWorkspace` (L159) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_c_entropy` | [x] |
| 66 | `HUF_writeCTable_wksp` (L263) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_c_entropy` | [x] |
| 67 | `HUF_writeCTable_wksp` (L264) | `return ERROR(maxSymbolValue_tooLarge)` | `ZSTD_error_maxSymbolValue_tooLarge` | `phase_c_entropy` | [x] |
| 68 | `HUF_writeCTable_wksp` (L274) | `return ERROR(dstSize_tooSmall)` | `ZSTD_error_dstSize_tooSmall` | `phase_c_entropy` | [x] |
| 69 | `HUF_writeCTable_wksp` (L282) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_c_entropy` | [x] |
| 70 | `HUF_writeCTable_wksp` (L283) | `return ERROR(dstSize_tooSmall)` | `ZSTD_error_dstSize_tooSmall` | `phase_c_entropy` | [x] |
| 71 | `HUF_readCTable` (L305) | `return ERROR(tableLog_tooLarge)` | `ZSTD_error_tableLog_tooLarge` | `phase_c_entropy` | [x] |
| 72 | `HUF_readCTable` (L306) | `return ERROR(maxSymbolValue_tooSmall)` | `ZSTD_error_maxSymbolValue_tooSmall` | `phase_c_entropy` | [x] |
| 73 | `HUF_buildCTableFromTree` (L771) | `return ERROR(workSpace_tooSmall)` | `ZSTD_error_workSpace_tooSmall` | `phase_c_entropy` | [x] |
| 74 | `HUF_buildCTableFromTree` (L774) | `return ERROR(maxSymbolValue_tooLarge)` | `ZSTD_error_maxSymbolValue_tooLarge` | `phase_c_entropy` | [x] |
| 75 | `HUF_buildCTableFromTree` (L786) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_c_entropy` | [x] |
| 76 | `HUF_initCStream` (L863) | `return ERROR(dstSize_tooSmall)` | `ZSTD_error_dstSize_tooSmall` | `phase_c_entropy` | [x] |
| 77 | `HUF_optimalTableLog` (L1349) | `return ERROR(workSpace_tooSmall)` | `ZSTD_error_workSpace_tooSmall` | `phase_c_entropy` | [x] |
| 78 | `HUF_optimalTableLog` (L1352) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_c_entropy` | [x] |
| 79 | `HUF_optimalTableLog` (L1353) | `return ERROR(tableLog_tooLarge)` | `ZSTD_error_tableLog_tooLarge` | `phase_c_entropy` | [x] |
| 80 | `HUF_optimalTableLog` (L1354) | `return ERROR(maxSymbolValue_tooLarge)` | `ZSTD_error_maxSymbolValue_tooLarge` | `phase_c_entropy` | [x] |

### `compress/zstd_compress.c`

| # | function | trigger (the exact invalid input/condition) | expected C result | covering test | [x] |
|---|----------|---------------------------------------------|-------------------|---------------|-----|
| 81 | `ZSTD_compressBound` (L72) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_c_compress` | [x] |
| 82 | `ZSTD_createCCtx_advanced` (L118) | `return NULL;` | `NULL` | `phase_c_compress` | [x] |
| 83 | `ZSTD_createCCtx_advanced` (L120) | `return NULL;` | `NULL` | `phase_c_compress` | [x] |
| 84 | `ZSTD_initStaticCCtx` (L130) | `return NULL;` | `NULL` | `phase_c_compress` | [x] |
| 85 | `ZSTD_initStaticCCtx` (L131) | `return NULL;` | `NULL` | `phase_c_compress` | [x] |
| 86 | `ZSTD_initStaticCCtx` (L135) | `return NULL;` | `NULL` | `phase_c_compress` | [x] |
| 87 | `ZSTD_initStaticCCtx` (L142) | `return NULL;` | `NULL` | `phase_c_compress` | [x] |
| 88 | `ZSTD_freeCCtx` (L185) | `cctx->staticSize` | `ZSTD_error_memory_allocation` | `phase_c_compress` | [x] |
| 89 | `ZSTD_createCCtxParams_advanced` (L332) | `return NULL;` | `NULL` | `phase_c_compress` | [x] |
| 90 | `ZSTD_createCCtxParams_advanced` (L335) | `return NULL;` | `NULL` | `phase_c_compress` | [x] |
| 91 | `ZSTD_CCtxParams_init` (L359) | `!cctxParams` | `ZSTD_error_GENERIC` | `phase_c_compress` | [x] |
| 92 | `ZSTD_CCtxParams_init_advanced` (L397) | `!cctxParams` | `ZSTD_error_GENERIC` | `phase_c_compress` | [x] |
| 93 | `ZSTD_cParam_getBounds` (L634) | `= ERROR(parameter_unsupported)` | `ZSTD_error_parameter_unsupported` | `phase_c_params` | [x] |
| 94 | `ZSTD_cParam_clampBounds` (L653) | `!ZSTD_cParam_withinBounds(cParam,val)` | `ZSTD_error_parameter_outOfBound` | `phase_c_params` | [x] |
| 95 | `ZSTD_CCtx_setParameter` (L715) | `RETURN_ERROR(stage_wrong, "can only set params in cctx init stage")` | `ZSTD_error_stage_wrong` | `phase_c_compress` | [x] |
| 96 | `ZSTD_CCtx_setParameter` (L721) | `(value!=0) && cctx->staticSize` | `ZSTD_error_parameter_unsupported` | `phase_c_params` | [x] |
| 97 | `ZSTD_CCtx_setParameter` (L765) | `RETURN_ERROR(parameter_unsupported, "unknown parameter")` | `ZSTD_error_parameter_unsupported` | `phase_c_params` | [x] |
| 98 | `ZSTD_CCtxParams_setParameter` (L868) | `value!=0` | `ZSTD_error_parameter_unsupported` | `phase_c_params` | [x] |
| 99 | `ZSTD_CCtxParams_setParameter` (L878) | `value!=0` | `ZSTD_error_parameter_unsupported` | `phase_c_params` | [x] |
| 100 | `ZSTD_CCtxParams_setParameter` (L892) | `value!=0` | `ZSTD_error_parameter_unsupported` | `phase_c_params` | [x] |
| 101 | `ZSTD_CCtxParams_setParameter` (L902) | `value!=0` | `ZSTD_error_parameter_unsupported` | `phase_c_params` | [x] |
| 102 | `ZSTD_CCtxParams_setParameter` (L1019) | `RETURN_ERROR(parameter_unsupported, "unknown parameter")` | `ZSTD_error_parameter_unsupported` | `phase_c_params` | [x] |
| 103 | `ZSTD_CCtxParams_getParameter` (L1086) | `RETURN_ERROR(parameter_unsupported, "not compiled with multithreading")` | `ZSTD_error_parameter_unsupported` | `phase_c_params` | [x] |
| 104 | `ZSTD_CCtxParams_getParameter` (L1094) | `RETURN_ERROR(parameter_unsupported, "not compiled with multithreading")` | `ZSTD_error_parameter_unsupported` | `phase_c_params` | [x] |
| 105 | `ZSTD_CCtxParams_getParameter` (L1101) | `RETURN_ERROR(parameter_unsupported, "not compiled with multithreading")` | `ZSTD_error_parameter_unsupported` | `phase_c_params` | [x] |
| 106 | `ZSTD_CCtxParams_getParameter` (L1166) | `RETURN_ERROR(parameter_unsupported, "unknown parameter")` | `ZSTD_error_parameter_unsupported` | `phase_c_params` | [x] |
| 107 | `ZSTD_CCtx_setParametersUsingCCtxParams` (L1182) | `cctx->streamStage != zcss_init` | `ZSTD_error_stage_wrong` | `phase_c_compress` | [x] |
| 108 | `ZSTD_CCtx_setParametersUsingCCtxParams` (L1184) | `cctx->cdict` | `ZSTD_error_stage_wrong` | `phase_c_compress` | [x] |
| 109 | `ZSTD_CCtx_setPledgedSrcSize` (L1233) | `cctx->streamStage != zcss_init` | `ZSTD_error_stage_wrong` | `phase_c_compress` | [x] |
| 110 | `ZSTD_initLocalDict` (L1278) | `!dl->cdict` | `ZSTD_error_memory_allocation` | `phase_c_compress` | [x] |
| 111 | `ZSTD_CCtx_loadDictionary_advanced` (L1290) | `cctx->streamStage != zcss_init` | `ZSTD_error_stage_wrong` | `phase_c_compress` | [x] |
| 112 | `ZSTD_CCtx_loadDictionary_advanced` (L1300) | `cctx->staticSize` | `ZSTD_error_memory_allocation` | `phase_c_compress` | [x] |
| 113 | `ZSTD_CCtx_loadDictionary_advanced` (L1303) | `dictBuffer==NULL` | `ZSTD_error_memory_allocation` | `phase_c_compress` | [x] |
| 114 | `ZSTD_CCtx_refCDict` (L1330) | `cctx->streamStage != zcss_init` | `ZSTD_error_stage_wrong` | `phase_c_compress` | [x] |
| 115 | `ZSTD_CCtx_refThreadPool` (L1340) | `cctx->streamStage != zcss_init` | `ZSTD_error_stage_wrong` | `phase_c_compress` | [x] |
| 116 | `ZSTD_CCtx_refPrefix_advanced` (L1354) | `cctx->streamStage != zcss_init` | `ZSTD_error_stage_wrong` | `phase_c_compress` | [x] |
| 117 | `ZSTD_CCtx_reset` (L1376) | `cctx->streamStage != zcss_init` | `ZSTD_error_stage_wrong` | `phase_c_compress` | [x] |
| 118 | `ZSTD_estimateCCtxSize_usingCCtxParams` (L1761) | `params->nbWorkers > 0` | `ZSTD_error_GENERIC` | `phase_c_compress` | [x] |
| 119 | `ZSTD_estimateCStreamSize_usingCCtxParams` (L1813) | `params->nbWorkers > 0` | `ZSTD_error_GENERIC` | `phase_c_compress` | [x] |
| 120 | `ZSTD_advanceHashSalt` (L2023) | `ZSTD_cwksp_reserve_failed(ws)` | `ZSTD_error_memory_allocation` | `phase_c_compress` | [x] |
| 121 | `ZSTD_advanceHashSalt` (L2066) | `ZSTD_cwksp_reserve_failed(ws)` | `ZSTD_error_memory_allocation` | `phase_c_compress` | [x] |
| 122 | `ZSTD_resetCCtx_internal` (L2168) | `zc->staticSize` | `ZSTD_error_memory_allocation` | `phase_c_compress` | [x] |
| 123 | `ZSTD_resetCCtx_internal` (L2181) | `zc->blockState.prevCBlock == NULL` | `ZSTD_error_memory_allocation` | `phase_c_compress` | [x] |
| 124 | `ZSTD_resetCCtx_internal` (L2183) | `zc->blockState.nextCBlock == NULL` | `ZSTD_error_memory_allocation` | `phase_c_compress` | [x] |
| 125 | `ZSTD_resetCCtx_internal` (L2185) | `zc->tmpWorkspace == NULL` | `ZSTD_error_memory_allocation` | `phase_c_compress` | [x] |
| 126 | `ZSTD_copyCCtx_internal` (L2519) | `srcCCtx->stage!=ZSTDcs_init` | `ZSTD_error_stage_wrong` | `phase_c_compress` | [x] |
| 127 | `ZSTD_blockSplitterEnabled` (L2940) | `(oend-op) < 3 /*max nbSeq Size*/ + 1 /*seqHead*/` | `ZSTD_error_dstSize_tooSmall` | `phase_c_compress` | [x] |
| 128 | `ZSTD_blockSplitterEnabled` (L3026) | `= ERROR(dstSize_tooSmall)` | `ZSTD_error_dstSize_tooSmall` | `phase_c_compress` | [x] |
| 129 | `ZSTD_postProcessSequenceProducerResult` (L3177) | `nbExternalSeqs > outSeqsCapacity` | `ZSTD_error_sequenceProducer_failed` | `phase_c_compress` | [x] |
| 130 | `ZSTD_postProcessSequenceProducerResult` (L3184) | `nbExternalSeqs == 0 && srcSize > 0` | `ZSTD_error_sequenceProducer_failed` | `phase_c_compress` | [x] |
| 131 | `ZSTD_postProcessSequenceProducerResult` (L3205) | `nbExternalSeqs == outSeqsCapacity` | `ZSTD_error_sequenceProducer_failed` | `phase_c_compress` | [x] |
| 132 | `ZSTD_buildSeqStore` (L3312) | `ZSTD_hasExtSeqProd(&zc->appliedParams)` | `ZSTD_error_parameter_combination_unsupported` | `phase_c_params` | [x] |
| 133 | `ZSTD_buildSeqStore` (L3331) | `ZSTD_hasExtSeqProd(&zc->appliedParams)` | `ZSTD_error_parameter_combination_unsupported` | `phase_c_params` | [x] |
| 134 | `ZSTD_buildSeqStore` (L3380) | `seqLenSum > srcSize` | `ZSTD_error_externalSequences_invalid` | `phase_c_compress` | [x] |
| 135 | `ZSTD_copyBlockSequences` (L3445) | `nbOutSequences > (size_t)(seqCollector->maxSequences - seqCollector->seqIndex)` | `ZSTD_error_dstSize_tooSmall` | `phase_c_compress` | [x] |
| 136 | `ZSTD_generateSequences` (L3529) | `targetCBlockSize != 0` | `ZSTD_error_parameter_unsupported` | `phase_c_params` | [x] |
| 137 | `ZSTD_generateSequences` (L3534) | `nbWorkers != 0` | `ZSTD_error_parameter_unsupported` | `phase_c_params` | [x] |
| 138 | `ZSTD_generateSequences` (L3538) | `dst == NULL` | `ZSTD_error_memory_allocation` | `phase_c_compress` | [x] |
| 139 | `ZSTD_deriveSeqStoreChunk` (L4124) | `dstCapacity < ZSTD_blockHeaderSize` | `ZSTD_error_dstSize_tooSmall` | `phase_c_compress` | [x] |
| 140 | `ZSTD_deriveBlockSplits` (L4368) | `zc->seqCollector.collectSequences` | `ZSTD_error_sequenceProducer_failed` | `phase_c_compress` | [x] |
| 141 | `ZSTD_deriveBlockSplits` (L4402) | `zc->seqCollector.collectSequences` | `ZSTD_error_sequenceProducer_failed` | `phase_c_compress` | [x] |
| 142 | `ZSTD_compressBlock_targetCBlockSize_body` (L4487) | `= ERROR(dstSize_tooSmall)` | `ZSTD_error_dstSize_tooSmall` | `phase_c_compress` | [x] |
| 143 | `ZSTD_compress_frameChunk` (L4623) | `dstCapacity < ZSTD_blockHeaderSize + MIN_CBLOCK_SIZE + 1` | `ZSTD_error_dstSize_tooSmall` | `phase_c_compress` | [x] |
| 144 | `ZSTD_writeFrameHeader` (L4712) | `dstCapacity < ZSTD_FRAMEHEADERSIZE_MAX` | `ZSTD_error_dstSize_tooSmall` | `phase_c_compress` | [x] |
| 145 | `ZSTD_writeSkippableFrame` (L4754) | `dstCapacity < srcSize + ZSTD_SKIPPABLEHEADERSIZE /* Skippable frame overhead */` | `ZSTD_error_dstSize_tooSmall` | `phase_c_compress` | [x] |
| 146 | `ZSTD_writeSkippableFrame` (L4756) | `srcSize > (unsigned)0xFFFFFFFF` | `ZSTD_error_srcSize_wrong` | `phase_c_compress` | [x] |
| 147 | `ZSTD_writeSkippableFrame` (L4757) | `magicVariant > 15` | `ZSTD_error_parameter_outOfBound` | `phase_c_params` | [x] |
| 148 | `ZSTD_writeLastEmptyBlock` (L4772) | `dstCapacity < ZSTD_blockHeaderSize` | `ZSTD_error_dstSize_tooSmall` | `phase_c_compress` | [x] |
| 149 | `ZSTD_compressContinue_internal` (L4802) | `cctx->stage==ZSTDcs_created` | `ZSTD_error_stage_wrong` | `phase_c_compress` | [x] |
| 150 | `ZSTD_compressContinue_internal` (L4842) | `cctx->consumedSrcSize+1 > cctx->pledgedSrcSizePlusOne` | `ZSTD_error_srcSize_wrong` | `phase_c_compress` | [x] |
| 151 | `ZSTD_compressBlock_deprecated` (L4887) | `srcSize > blockSizeMax` | `ZSTD_error_srcSize_wrong` | `phase_c_compress` | [x] |
| 152 | `ZSTD_loadCEntropy` (L5081) | `HUF_isError(hufHeaderSize)` | `ZSTD_error_dictionary_corrupted` | `phase_c_compress` | [x] |
| 153 | `ZSTD_loadCEntropy` (L5087) | `FSE_isError(offcodeHeaderSize)` | `ZSTD_error_dictionary_corrupted` | `phase_c_compress` | [x] |
| 154 | `ZSTD_loadCEntropy` (L5088) | `offcodeLog > OffFSELog` | `ZSTD_error_dictionary_corrupted` | `phase_c_compress` | [x] |
| 155 | `ZSTD_loadCEntropy` (L5090) | `FSE_isError(FSE_buildCTable_wksp( bs->entropy.fse.offcodeCTable, offcodeNCount, MaxOff, offcodeLog, workspace, HUF_WO...` | computed error | `phase_c_compress` | [x] |
| 156 | `ZSTD_loadCEntropy` (L5102) | `FSE_isError(matchlengthHeaderSize)` | `ZSTD_error_dictionary_corrupted` | `phase_c_compress` | [x] |
| 157 | `ZSTD_loadCEntropy` (L5103) | `matchlengthLog > MLFSELog` | `ZSTD_error_dictionary_corrupted` | `phase_c_compress` | [x] |
| 158 | `ZSTD_loadCEntropy` (L5104) | `FSE_isError(FSE_buildCTable_wksp( bs->entropy.fse.matchlengthCTable, matchlengthNCount, matchlengthMaxValue, matchlen...` | computed error | `phase_c_compress` | [x] |
| 159 | `ZSTD_loadCEntropy` (L5116) | `FSE_isError(litlengthHeaderSize)` | `ZSTD_error_dictionary_corrupted` | `phase_c_compress` | [x] |
| 160 | `ZSTD_loadCEntropy` (L5117) | `litlengthLog > LLFSELog` | `ZSTD_error_dictionary_corrupted` | `phase_c_compress` | [x] |
| 161 | `ZSTD_loadCEntropy` (L5118) | `FSE_isError(FSE_buildCTable_wksp( bs->entropy.fse.litlengthCTable, litlengthNCount, litlengthMaxValue, litlengthLog, ...` | computed error | `phase_c_compress` | [x] |
| 162 | `ZSTD_loadCEntropy` (L5127) | `dictPtr+12 > dictEnd` | `ZSTD_error_dictionary_corrupted` | `phase_c_compress` | [x] |
| 163 | `ZSTD_loadCEntropy` (L5145) | `bs->rep[u] == 0` | `ZSTD_error_dictionary_corrupted` | `phase_c_compress` | [x] |
| 164 | `ZSTD_loadCEntropy` (L5146) | `bs->rep[u] > dictContentSize` | `ZSTD_error_dictionary_corrupted` | `phase_c_compress` | [x] |
| 165 | `ZSTD_loadZstdDictionary` (L5207) | `dictContentType == ZSTD_dct_fullDict` | `ZSTD_error_dictionary_wrong` | `phase_c_compress` | [x] |
| 166 | `ZSTD_loadZstdDictionary` (L5223) | `dictContentType == ZSTD_dct_fullDict` | `ZSTD_error_dictionary_wrong` | `phase_c_compress` | [x] |
| 167 | `ZSTD_writeEpilogue` (L5350) | `cctx->stage == ZSTDcs_created` | `ZSTD_error_stage_wrong` | `phase_c_compress` | [x] |
| 168 | `ZSTD_writeEpilogue` (L5365) | `dstCapacity<3` | `ZSTD_error_dstSize_tooSmall` | `phase_c_compress` | [x] |
| 169 | `ZSTD_writeEpilogue` (L5373) | `dstCapacity<4` | `ZSTD_error_dstSize_tooSmall` | `phase_c_compress` | [x] |
| 170 | `ZSTD_compressEnd_public` (L5422) | `cctx->pledgedSrcSizePlusOne != cctx->consumedSrcSize+1` | `ZSTD_error_srcSize_wrong` | `phase_c_compress` | [x] |
| 171 | `ZSTD_compress` (L5504) | `!cctx` | `ZSTD_error_memory_allocation` | `phase_c_compress` | [x] |
| 172 | `ZSTD_initCDict_internal` (L5566) | `!internalBuffer` | `ZSTD_error_memory_allocation` | `phase_c_compress` | [x] |
| 173 | `ZSTD_initCDict_internal` (L5612) | `return NULL;` | `NULL` | `phase_c_compress` | [x] |
| 174 | `ZSTD_initCDict_internal` (L5627) | `return NULL;` | `NULL` | `phase_c_compress` | [x] |
| 175 | `ZSTD_createCDict_advanced2` (L5672) | `return NULL;` | `NULL` | `phase_c_compress` | [x] |
| 176 | `ZSTD_createCDict_advanced2` (L5704) | `return NULL;` | `NULL` | `phase_c_compress` | [x] |
| 177 | `ZSTD_initStaticCDict` (L5777) | `return NULL;` | `NULL` | `phase_c_compress` | [x] |
| 178 | `ZSTD_initStaticCDict` (L5783) | `return NULL;` | `NULL` | `phase_c_compress` | [x] |
| 179 | `ZSTD_initStaticCDict` (L5787) | `return NULL;` | `NULL` | `phase_c_compress` | [x] |
| 180 | `ZSTD_initStaticCDict` (L5799) | `return NULL;` | `NULL` | `phase_c_compress` | [x] |
| 181 | `ZSTD_compressBegin_usingCDict_internal` (L5829) | `cdict==NULL` | `ZSTD_error_dictionary_wrong` | `phase_c_compress` | [x] |
| 182 | `ZSTD_compressStream_generic` (L6143) | `RETURN_ERROR(init_missing, "call ZSTD_initCStream() first!")` | `ZSTD_error_init_missing` | `phase_c_compress` | [x] |
| 183 | `ZSTD_checkBufferStability` (L6333) | `RETURN_ERROR(stabilityCondition_notRespected, "ZSTD_c_stableInBuffer enabled but input differs!")` | `ZSTD_error_stabilityCondition_notRespected` | `phase_c_compress` | [x] |
| 184 | `ZSTD_checkBufferStability` (L6339) | `RETURN_ERROR(stabilityCondition_notRespected, "ZSTD_c_stableOutBuffer enabled but output size differs!")` | `ZSTD_error_stabilityCondition_notRespected` | `phase_c_compress` | [x] |
| 185 | `ZSTD_CCtx_init_compressStream2` (L6386) | `ZSTD_hasExtSeqProd(&params) && params.nbWorkers >= 1` | `ZSTD_error_parameter_combination_unsupported` | `phase_c_params` | [x] |
| 186 | `ZSTD_CCtx_init_compressStream2` (L6404) | `cctx->mtctx == NULL` | `ZSTD_error_memory_allocation` | `phase_c_compress` | [x] |
| 187 | `ZSTD_compressStream2` (L6454) | `output->pos > output->size` | `ZSTD_error_dstSize_tooSmall` | `phase_c_compress` | [x] |
| 188 | `ZSTD_compressStream2` (L6455) | `input->pos > input->size` | `ZSTD_error_srcSize_wrong` | `phase_c_compress` | [x] |
| 189 | `ZSTD_compressStream2` (L6456) | `(U32)endOp > (U32)ZSTD_e_end` | `ZSTD_error_parameter_outOfBound` | `phase_c_params` | [x] |
| 190 | `ZSTD_compressStream2` (L6468) | `input->src != cctx->expectedInBuffer.src` | `ZSTD_error_stabilityCondition_notRespected` | `phase_c_compress` | [x] |
| 191 | `ZSTD_compressStream2` (L6469) | `input->pos != cctx->expectedInBuffer.size` | `ZSTD_error_stabilityCondition_notRespected` | `phase_c_compress` | [x] |
| 192 | `ZSTD_compress2` (L6592) | `RETURN_ERROR(dstSize_tooSmall, "")` | `ZSTD_error_dstSize_tooSmall` | `phase_c_compress` | [x] |
| 193 | `ZSTD_compress2` (L6615) | `offBase > OFFSET_TO_OFFBASE(offsetBound)` | `ZSTD_error_externalSequences_invalid` | `phase_c_compress` | [x] |
| 194 | `ZSTD_compress2` (L6617) | `matchLength < matchLenLowerBound` | `ZSTD_error_externalSequences_invalid` | `phase_c_compress` | [x] |
| 195 | `ZSTD_finalizeOffBase` (L6690) | `idx - seqPos->idx >= cctx->seqStore.maxNbSeq` | `ZSTD_error_externalSequences_invalid` | `phase_c_compress` | [x] |
| 196 | `ZSTD_finalizeOffBase` (L6695) | `idx == inSeqsSize` | `ZSTD_error_externalSequences_invalid` | `phase_c_compress` | [x] |
| 197 | `ZSTD_finalizeOffBase` (L6728) | `ip != iend` | `ZSTD_error_externalSequences_invalid` | `phase_c_compress` | [x] |
| 198 | `ZSTD_finalizeOffBase` (L6844) | `idx - seqPos->idx >= cctx->seqStore.maxNbSeq` | `ZSTD_error_externalSequences_invalid` | `phase_c_compress` | [x] |
| 199 | `ZSTD_selectSequenceCopier` (L6908) | `RETURN_ERROR(externalSequences_invalid, "delimiter format error : both matchlength and offset must be == 0")` | `ZSTD_error_externalSequences_invalid` | `phase_c_compress` | [x] |
| 200 | `ZSTD_selectSequenceCopier` (L6914) | `RETURN_ERROR(externalSequences_invalid, "Reached end of sequences without finding a block delimiter")` | `ZSTD_error_externalSequences_invalid` | `phase_c_compress` | [x] |
| 201 | `determine_blockSize` (L6932) | `RETURN_ERROR(externalSequences_invalid, "sequences incorrectly define a too large block")` | `ZSTD_error_externalSequences_invalid` | `phase_c_compress` | [x] |
| 202 | `determine_blockSize` (L6934) | `RETURN_ERROR(externalSequences_invalid, "sequences define a frame longer than source")` | `ZSTD_error_externalSequences_invalid` | `phase_c_compress` | [x] |
| 203 | `determine_blockSize` (L6962) | `dstCapacity<4` | `ZSTD_error_dstSize_tooSmall` | `phase_c_compress` | [x] |
| 204 | `determine_blockSize` (L7001) | `dstCapacity < ZSTD_blockHeaderSize` | `ZSTD_error_dstSize_tooSmall` | `phase_c_compress` | [x] |
| 205 | `ZSTD_compressSequences` (L7102) | `dstCapacity<4` | `ZSTD_error_dstSize_tooSmall` | `phase_c_compress` | [x] |
| 206 | `ZSTD_convertBlockSequences` (L7327) | `nbSequences >= cctx->seqStore.maxNbSeq` | `ZSTD_error_externalSequences_invalid` | `phase_c_compress` | [x] |
| 207 | `ZSTD_get1BlockSummary` (L7435) | `= ERROR(externalSequences_invalid)` | `ZSTD_error_externalSequences_invalid` | `phase_c_compress` | [x] |
| 208 | `ZSTD_get1BlockSummary` (L7464) | `= ERROR(externalSequences_invalid)` | `ZSTD_error_externalSequences_invalid` | `phase_c_compress` | [x] |
| 209 | `ZSTD_get1BlockSummary` (L7490) | `nbSequences == 0` | `ZSTD_error_externalSequences_invalid` | `phase_c_compress` | [x] |
| 210 | `ZSTD_get1BlockSummary` (L7495) | `dstCapacity<3` | `ZSTD_error_dstSize_tooSmall` | `phase_c_compress` | [x] |
| 211 | `ZSTD_get1BlockSummary` (L7508) | `block.litSize > litSize` | `ZSTD_error_externalSequences_invalid` | `phase_c_compress` | [x] |
| 212 | `ZSTD_get1BlockSummary` (L7524) | `dstCapacity < ZSTD_blockHeaderSize` | `ZSTD_error_dstSize_tooSmall` | `phase_c_compress` | [x] |
| 213 | `ZSTD_get1BlockSummary` (L7550) | `RETURN_ERROR(cannotProduce_uncompressedBlock, "ZSTD_compressSequencesAndLiterals cannot generate an uncompressed block")` | `ZSTD_error_cannotProduce_uncompressedBlock` | `phase_c_compress` | [x] |
| 214 | `ZSTD_get1BlockSummary` (L7578) | `litSize != 0` | `ZSTD_error_externalSequences_invalid` | `phase_c_compress` | [x] |
| 215 | `ZSTD_get1BlockSummary` (L7579) | `remaining != 0` | `ZSTD_error_externalSequences_invalid` | `phase_c_compress` | [x] |
| 216 | `ZSTD_get1BlockSummary` (L7598) | `RETURN_ERROR(workSpace_tooSmall, "literals buffer is not large enough: must be at least 8 bytes larger than litSize (...` | `ZSTD_error_workSpace_tooSmall` | `phase_c_compress` | [x] |
| 217 | `ZSTD_get1BlockSummary` (L7603) | `RETURN_ERROR(frameParameter_unsupported, "This mode is only compatible with explicit delimiters")` | `ZSTD_error_frameParameter_unsupported` | `phase_c_compress` | [x] |
| 218 | `ZSTD_get1BlockSummary` (L7606) | `RETURN_ERROR(parameter_unsupported, "This mode is not compatible with Sequence validation")` | `ZSTD_error_parameter_unsupported` | `phase_c_params` | [x] |
| 219 | `ZSTD_get1BlockSummary` (L7609) | `RETURN_ERROR(frameParameter_unsupported, "this mode is not compatible with frame checksum")` | `ZSTD_error_frameParameter_unsupported` | `phase_c_compress` | [x] |

### `compress/zstd_compress_internal.h`

| # | function | trigger (the exact invalid input/condition) | expected C result | covering test | [x] |
|---|----------|---------------------------------------------|-------------------|---------------|-----|
| 220 | `ZSTD_cParam_withinBounds` (L654) | `srcSize + ZSTD_blockHeaderSize > dstCapacity` | `ZSTD_error_dstSize_tooSmall` | `phase_c_compress` | [x] |
| 221 | `ZSTD_cParam_withinBounds` (L666) | `dstCapacity < 4` | `ZSTD_error_dstSize_tooSmall` | `phase_c_compress` | [x] |

### `compress/zstd_compress_literals.c`

| # | function | trigger (the exact invalid input/condition) | expected C result | covering test | [x] |
|---|----------|---------------------------------------------|-------------------|---------------|-----|
| 222 | `ZSTD_noCompressLiterals` (L46) | `srcSize + flSize > dstCapacity` | `ZSTD_error_dstSize_tooSmall` | `phase_c_compress` | [x] |
| 223 | `ZSTD_compressLiterals` (L161) | `dstCapacity < lhSize+1` | `ZSTD_error_dstSize_tooSmall` | `phase_c_compress` | [x] |

### `compress/zstd_compress_sequences.c`

| # | function | trigger (the exact invalid input/condition) | expected C result | covering test | [x] |
|---|----------|---------------------------------------------|-------------------|---------------|-----|
| 224 | `ZSTD_fseBitCost` (L117) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_c_compress` | [x] |
| 225 | `ZSTD_fseBitCost` (L127) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_c_compress` | [x] |
| 226 | `ZSTD_crossEntropyCost` (L258) | `dstCapacity==0` | `ZSTD_error_dstSize_tooSmall` | `phase_c_compress` | [x] |
| 227 | `ZSTD_crossEntropyCost` (L286) | `RETURN_ERROR(GENERIC, "impossible to reach")` | `ZSTD_error_GENERIC` | `phase_c_compress` | [x] |
| 228 | `ZSTD_crossEntropyCost` (L303) | `ERR_isError(BIT_initCStream(&blockStream, dst, dstCapacity))` | `ZSTD_error_dstSize_tooSmall` | `phase_c_compress` | [x] |
| 229 | `ZSTD_crossEntropyCost` (L379) | `streamSize==0` | `ZSTD_error_dstSize_tooSmall` | `phase_c_compress` | [x] |

### `compress/zstd_compress_superblock.c`

| # | function | trigger (the exact invalid input/condition) | expected C result | covering test | [x] |
|---|----------|---------------------------------------------|-------------------|---------------|-----|
| 230 | `?` (L181) | `(oend-op) < 3 /*max nbSeq Size*/ + 1 /*seqHead*/` | `ZSTD_error_dstSize_tooSmall` | `phase_c_compress` | [x] |

### `compress/zstd_cwksp.h`

| # | function | trigger (the exact invalid input/condition) | expected C result | covering test | [x] |
|---|----------|---------------------------------------------|-------------------|---------------|-----|
| 231 | `ZSTD_cwksp_initialAllocStart` (L302) | `return NULL;` | `NULL` | `phase_c_misc` | [x] |
| 232 | `ZSTD_cwksp_initialAllocStart` (L334) | `objectEnd > ws->workspaceEnd` | `ZSTD_error_memory_allocation` | `phase_c_misc` | [x] |
| 233 | `ZSTD_cwksp_owns_buffer` (L365) | `return NULL;` | `NULL` | `phase_c_misc` | [x] |
| 234 | `ZSTD_cwksp_reserve_table` (L457) | `return NULL;` | `NULL` | `phase_c_misc` | [x] |
| 235 | `ZSTD_cwksp_reserve_table` (L472) | `return NULL;` | `NULL` | `phase_c_misc` | [x] |
| 236 | `ZSTD_cwksp_reserve_object` (L512) | `return NULL;` | `NULL` | `phase_c_misc` | [x] |
| 237 | `ZSTD_cwksp_reserve_object_aligned` (L538) | `return NULL;` | `NULL` | `phase_c_misc` | [x] |
| 238 | `ZSTD_cwksp_create` (L692) | `workspace == NULL` | `ZSTD_error_memory_allocation` | `phase_c_misc` | [x] |

### `compress/zstd_ldm.c`

| # | function | trigger (the exact invalid input/condition) | expected C result | covering test | [x] |
|---|----------|---------------------------------------------|-------------------|---------------|-----|
| 239 | `ZSTD_ldm_generateSequences_internal` (L479) | `return ERROR(dstSize_tooSmall)` | `ZSTD_error_dstSize_tooSmall` | `phase_c_compress` | [x] |

### `compress/zstdmt_compress.c`

| # | function | trigger (the exact invalid input/condition) | expected C result | covering test | [x] |
|---|----------|---------------------------------------------|-------------------|---------------|-----|
| 240 | `ZSTDMT_createBufferPool` (L126) | `return NULL;` | `NULL` | `phase_c_misc` | [x] |
| 241 | `ZSTDMT_createBufferPool` (L129) | `return NULL;` | `NULL` | `phase_c_misc` | [x] |
| 242 | `ZSTDMT_createBufferPool` (L134) | `return NULL;` | `NULL` | `phase_c_misc` | [x] |
| 243 | `ZSTDMT_expandBufferPool` (L173) | `return NULL;` | `NULL` | `phase_c_misc` | [x] |
| 244 | `ZSTDMT_createSeqPool` (L337) | `return NULL;` | `NULL` | `phase_c_misc` | [x] |
| 245 | `ZSTDMT_createCCtxPool` (L386) | `return NULL;` | `NULL` | `phase_c_misc` | [x] |
| 246 | `ZSTDMT_createCCtxPool` (L389) | `return NULL;` | `NULL` | `phase_c_misc` | [x] |
| 247 | `ZSTDMT_createCCtxPool` (L395) | `return NULL;` | `NULL` | `phase_c_misc` | [x] |
| 248 | `ZSTDMT_createCCtxPool` (L399) | `return NULL;` | `NULL` | `phase_c_misc` | [x] |
| 249 | `ZSTDMT_expandCCtxPool` (L408) | `return NULL;` | `NULL` | `phase_c_misc` | [x] |
| 250 | `ZSTDMT_createJobsTable` (L916) | `return NULL;` | `NULL` | `phase_c_misc` | [x] |
| 251 | `ZSTDMT_createJobsTable` (L924) | `return NULL;` | `NULL` | `phase_c_misc` | [x] |
| 252 | `ZSTDMT_expandJobsTable` (L935) | `return ERROR(memory_allocation)` | `ZSTD_error_memory_allocation` | `phase_c_misc` | [x] |
| 253 | `ZSTDMT_createCCtx_advanced_internal` (L957) | `return NULL;` | `NULL` | `phase_c_misc` | [x] |
| 254 | `ZSTDMT_createCCtx_advanced_internal` (L961) | `return NULL;` | `NULL` | `phase_c_misc` | [x] |
| 255 | `ZSTDMT_createCCtx_advanced_internal` (L964) | `return NULL;` | `NULL` | `phase_c_misc` | [x] |
| 256 | `ZSTDMT_createCCtx_advanced_internal` (L986) | `return NULL;` | `NULL` | `phase_c_misc` | [x] |
| 257 | `ZSTDMT_createCCtx_advanced` (L1000) | `return NULL;` | `NULL` | `phase_c_misc` | [x] |
| 258 | `ZSTDMT_resize` (L1080) | `return ERROR(memory_allocation)` | `ZSTD_error_memory_allocation` | `phase_c_misc` | [x] |
| 259 | `ZSTDMT_resize` (L1083) | `return ERROR(memory_allocation)` | `ZSTD_error_memory_allocation` | `phase_c_misc` | [x] |
| 260 | `ZSTDMT_resize` (L1085) | `return ERROR(memory_allocation)` | `ZSTD_error_memory_allocation` | `phase_c_misc` | [x] |
| 261 | `ZSTDMT_resize` (L1087) | `return ERROR(memory_allocation)` | `ZSTD_error_memory_allocation` | `phase_c_misc` | [x] |
| 262 | `ZSTDMT_initCStream_internal` (L1283) | `return ERROR(memory_allocation)` | `ZSTD_error_memory_allocation` | `phase_c_misc` | [x] |
| 263 | `ZSTDMT_initCStream_internal` (L1334) | `return ERROR(memory_allocation)` | `ZSTD_error_memory_allocation` | `phase_c_misc` | [x] |
| 264 | `ZSTDMT_initCStream_internal` (L1365) | `return ERROR(memory_allocation)` | `ZSTD_error_memory_allocation` | `phase_c_misc` | [x] |
| 265 | `ZSTDMT_initCStream_internal` (L1373) | `return ERROR(memory_allocation)` | `ZSTD_error_memory_allocation` | `phase_c_misc` | [x] |
| 266 | `ZSTDMT_writeLastEmptyBlock` (L1393) | `= ERROR(memory_allocation)` | `ZSTD_error_memory_allocation` | `phase_c_misc` | [x] |
| 267 | `ZSTDMT_compressStream_generic` (L1866) | `return ERROR(stage_wrong)` | `ZSTD_error_stage_wrong` | `phase_c_misc` | [x] |

### `decompress/huf_decompress.c`

| # | function | trigger (the exact invalid input/condition) | expected C result | covering test | [x] |
|---|----------|---------------------------------------------|-------------------|---------------|-----|
| 268 | `HUF_DecompressFastArgs_init` (L213) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_c_decompress` | [x] |
| 269 | `HUF_DecompressFastArgs_init` (L238) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_c_decompress` | [x] |
| 270 | `HUF_initRemainingDStream` (L285) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_c_decompress` | [x] |
| 271 | `HUF_initRemainingDStream` (L292) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_c_decompress` | [x] |
| 272 | `HUF_readDTableX1_wksp` (L395) | `return ERROR(tableLog_tooLarge)` | `ZSTD_error_tableLog_tooLarge` | `phase_c_decompress` | [x] |
| 273 | `HUF_readDTableX1_wksp` (L409) | `return ERROR(tableLog_tooLarge)` | `ZSTD_error_tableLog_tooLarge` | `phase_c_decompress` | [x] |
| 274 | `HUF_readDTableX1_wksp` (L592) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_c_decompress` | [x] |
| 275 | `HUF_readDTableX1_wksp` (L608) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_c_decompress` | [x] |
| 276 | `HUF_readDTableX1_wksp` (L609) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_c_decompress` | [x] |
| 277 | `HUF_readDTableX1_wksp` (L643) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_c_decompress` | [x] |
| 278 | `HUF_readDTableX1_wksp` (L644) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_c_decompress` | [x] |
| 279 | `HUF_readDTableX1_wksp` (L680) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_c_decompress` | [x] |
| 280 | `HUF_readDTableX1_wksp` (L681) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_c_decompress` | [x] |
| 281 | `HUF_readDTableX1_wksp` (L682) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_c_decompress` | [x] |
| 282 | `HUF_readDTableX1_wksp` (L693) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_c_decompress` | [x] |
| 283 | `HUF_decompress4X1_usingDTable_internal_fast_c_loop` (L886) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_c_decompress` | [x] |
| 284 | `HUF_decompress4X1_DCtx_wksp` (L938) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_c_decompress` | [x] |
| 285 | `HUF_readDTableX2_wksp` (L1193) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_c_decompress` | [x] |
| 286 | `HUF_readDTableX2_wksp` (L1200) | `return ERROR(tableLog_tooLarge)` | `ZSTD_error_tableLog_tooLarge` | `phase_c_decompress` | [x] |
| 287 | `HUF_readDTableX2_wksp` (L1207) | `return ERROR(tableLog_tooLarge)` | `ZSTD_error_tableLog_tooLarge` | `phase_c_decompress` | [x] |
| 288 | `HUF_readDTableX2_wksp` (L1373) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_c_decompress` | [x] |
| 289 | `HUF_readDTableX2_wksp` (L1389) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_c_decompress` | [x] |
| 290 | `HUF_readDTableX2_wksp` (L1390) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_c_decompress` | [x] |
| 291 | `HUF_readDTableX2_wksp` (L1424) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_c_decompress` | [x] |
| 292 | `HUF_readDTableX2_wksp` (L1425) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_c_decompress` | [x] |
| 293 | `HUF_readDTableX2_wksp` (L1483) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_c_decompress` | [x] |
| 294 | `HUF_readDTableX2_wksp` (L1484) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_c_decompress` | [x] |
| 295 | `HUF_readDTableX2_wksp` (L1485) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_c_decompress` | [x] |
| 296 | `HUF_readDTableX2_wksp` (L1496) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_c_decompress` | [x] |
| 297 | `HUF_decompress4X2_usingDTable_internal_fast_c_loop` (L1711) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_c_decompress` | [x] |
| 298 | `HUF_decompress1X2_DCtx_wksp` (L1763) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_c_decompress` | [x] |
| 299 | `HUF_decompress4X2_DCtx_wksp` (L1778) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_c_decompress` | [x] |
| 300 | `HUF_decompress1X_DCtx_wksp` (L1850) | `return ERROR(dstSize_tooSmall)` | `ZSTD_error_dstSize_tooSmall` | `phase_c_decompress` | [x] |
| 301 | `HUF_decompress1X_DCtx_wksp` (L1851) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_c_decompress` | [x] |
| 302 | `HUF_decompress1X1_DCtx_wksp` (L1900) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_c_decompress` | [x] |
| 303 | `HUF_decompress4X_hufOnly_wksp` (L1927) | `return ERROR(dstSize_tooSmall)` | `ZSTD_error_dstSize_tooSmall` | `phase_c_decompress` | [x] |
| 304 | `HUF_decompress4X_hufOnly_wksp` (L1928) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_c_decompress` | [x] |

### `decompress/zstd_ddict.c`

| # | function | trigger (the exact invalid input/condition) | expected C result | covering test | [x] |
|---|----------|---------------------------------------------|-------------------|---------------|-----|
| 305 | `ZSTD_copyDDictParameters` (L99) | `return ERROR(dictionary_corrupted)` | `ZSTD_error_dictionary_corrupted` | `phase_c_dictbuilder` | [x] |
| 306 | `ZSTD_copyDDictParameters` (L105) | `return ERROR(dictionary_corrupted)` | `ZSTD_error_dictionary_corrupted` | `phase_c_dictbuilder` | [x] |
| 307 | `ZSTD_copyDDictParameters` (L112) | `ZSTD_isError(ZSTD_loadDEntropy( &ddict->entropy, ddict->dictContent, ddict->dictSize))` | `ZSTD_error_dictionary_corrupted` | `phase_c_dictbuilder` | [x] |
| 308 | `ZSTD_initDDict_internal` (L133) | `return ERROR(memory_allocation)` | `ZSTD_error_memory_allocation` | `phase_c_dictbuilder` | [x] |
| 309 | `ZSTD_createDDict_advanced` (L150) | `return NULL;` | `NULL` | `phase_c_dictbuilder` | [x] |
| 310 | `ZSTD_createDDict_advanced` (L153) | `return NULL;` | `NULL` | `phase_c_dictbuilder` | [x] |
| 311 | `ZSTD_createDDict_advanced` (L160) | `return NULL;` | `NULL` | `phase_c_dictbuilder` | [x] |
| 312 | `ZSTD_initStaticDDict` (L198) | `return NULL;` | `NULL` | `phase_c_dictbuilder` | [x] |
| 313 | `ZSTD_initStaticDDict` (L199) | `return NULL;` | `NULL` | `phase_c_dictbuilder` | [x] |
| 314 | `ZSTD_initStaticDDict` (L207) | `return NULL;` | `NULL` | `phase_c_dictbuilder` | [x] |

### `decompress/zstd_decompress.c`

| # | function | trigger (the exact invalid input/condition) | expected C result | covering test | [x] |
|---|----------|---------------------------------------------|-------------------|---------------|-----|
| 315 | `ZSTD_DDictHashSet_emplaceDDict` (L109) | `hashSet->ddictPtrCount == hashSet->ddictPtrTableSize` | `ZSTD_error_GENERIC` | `phase_c_decompress` | [x] |
| 316 | `ZSTD_DDictHashSet_expand` (L139) | `!newTable` | `ZSTD_error_memory_allocation` | `phase_c_decompress` | [x] |
| 317 | `ZSTD_createDDictHashSet` (L182) | `return NULL;` | `NULL` | `phase_c_decompress` | [x] |
| 318 | `ZSTD_createDDictHashSet` (L186) | `return NULL;` | `NULL` | `phase_c_decompress` | [x] |
| 319 | `ZSTD_initStaticDCtx` (L285) | `return NULL;` | `NULL` | `phase_c_decompress` | [x] |
| 320 | `ZSTD_initStaticDCtx` (L286) | `return NULL;` | `NULL` | `phase_c_decompress` | [x] |
| 321 | `ZSTD_createDCtx_internal` (L295) | `return NULL;` | `NULL` | `phase_c_decompress` | [x] |
| 322 | `ZSTD_createDCtx_internal` (L298) | `return NULL;` | `NULL` | `phase_c_decompress` | [x] |
| 323 | `ZSTD_freeDCtx` (L327) | `dctx->staticSize` | `ZSTD_error_memory_allocation` | `phase_c_decompress` | [x] |
| 324 | `ZSTD_frameHeaderSize_internal` (L419) | `srcSize < minInputSize` | `ZSTD_error_srcSize_wrong` | `phase_c_decompress` | [x] |
| 325 | `ZSTD_getFrameHeader_advanced` (L456) | `src==NULL` | `ZSTD_error_GENERIC` | `phase_c_decompress` | [x] |
| 326 | `ZSTD_getFrameHeader_advanced` (L473) | `RETURN_ERROR(prefix_unknown, "first bytes don't correspond to any supported magic number")` | `ZSTD_error_prefix_unknown` | `phase_c_decompress` | [x] |
| 327 | `ZSTD_getFrameHeader_advanced` (L493) | `RETURN_ERROR(prefix_unknown, "")` | `ZSTD_error_prefix_unknown` | `phase_c_decompress` | [x] |
| 328 | `ZSTD_getFrameHeader_advanced` (L511) | `(fhdByte & 0x08) != 0` | `ZSTD_error_frameParameter_unsupported` | `phase_c_decompress` | [x] |
| 329 | `ZSTD_getFrameHeader_advanced` (L517) | `windowLog > ZSTD_WINDOWLOG_MAX` | `ZSTD_error_frameParameter_windowTooLarge` | `phase_c_decompress` | [x] |
| 330 | `readSkippableFrameSize` (L592) | `srcSize < ZSTD_SKIPPABLEHEADERSIZE` | `ZSTD_error_srcSize_wrong` | `phase_c_decompress` | [x] |
| 331 | `readSkippableFrameSize` (L595) | `(U32)(sizeU32 + ZSTD_SKIPPABLEHEADERSIZE) < sizeU32` | `ZSTD_error_frameParameter_unsupported` | `phase_c_decompress` | [x] |
| 332 | `readSkippableFrameSize` (L598) | `skippableSize > srcSize` | `ZSTD_error_srcSize_wrong` | `phase_c_decompress` | [x] |
| 333 | `ZSTD_readSkippableFrame` (L618) | `srcSize < ZSTD_SKIPPABLEHEADERSIZE` | `ZSTD_error_srcSize_wrong` | `phase_c_decompress` | [x] |
| 334 | `ZSTD_readSkippableFrame` (L625) | `!ZSTD_isSkippableFrame(src, srcSize)` | `ZSTD_error_frameParameter_unsupported` | `phase_c_decompress` | [x] |
| 335 | `ZSTD_readSkippableFrame` (L626) | `skippableFrameSize < ZSTD_SKIPPABLEHEADERSIZE \|\| skippableFrameSize > srcSize` | `ZSTD_error_srcSize_wrong` | `phase_c_decompress` | [x] |
| 336 | `ZSTD_readSkippableFrame` (L627) | `skippableContentSize > dstCapacity` | `ZSTD_error_dstSize_tooSmall` | `phase_c_decompress` | [x] |
| 337 | `ZSTD_decodeFrameHeader` (L706) | `result>0` | `ZSTD_error_srcSize_wrong` | `phase_c_decompress` | [x] |
| 338 | `ZSTD_decodeFrameHeader` (L717) | `dctx->fParams.dictID && (dctx->dictID != dctx->fParams.dictID)` | `ZSTD_error_dictionary_wrong` | `phase_c_decompress` | [x] |
| 339 | `ZSTD_decompressionMargin` (L852) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_c_decompress` | [x] |
| 340 | `ZSTD_copyRawBlock` (L900) | `srcSize > dstCapacity` | `ZSTD_error_dstSize_tooSmall` | `phase_c_decompress` | [x] |
| 341 | `ZSTD_copyRawBlock` (L903) | `RETURN_ERROR(dstBuffer_null, "")` | `ZSTD_error_dstBuffer_null` | `phase_c_decompress` | [x] |
| 342 | `ZSTD_setRleBlock` (L913) | `regenSize > dstCapacity` | `ZSTD_error_dstSize_tooSmall` | `phase_c_decompress` | [x] |
| 343 | `ZSTD_setRleBlock` (L916) | `RETURN_ERROR(dstBuffer_null, "")` | `ZSTD_error_dstBuffer_null` | `phase_c_decompress` | [x] |
| 344 | `ZSTD_decompressFrame` (L967) | `remainingSrcSize < ZSTD_FRAMEHEADERSIZE_MIN(dctx->format)+ZSTD_blockHeaderSize` | `ZSTD_error_srcSize_wrong` | `phase_c_decompress` | [x] |
| 345 | `ZSTD_decompressFrame` (L975) | `remainingSrcSize < frameHeaderSize+ZSTD_blockHeaderSize` | `ZSTD_error_srcSize_wrong` | `phase_c_decompress` | [x] |
| 346 | `ZSTD_decompressFrame` (L995) | `cBlockSize > remainingSrcSize` | `ZSTD_error_srcSize_wrong` | `phase_c_decompress` | [x] |
| 347 | `ZSTD_decompressFrame` (L1029) | `RETURN_ERROR(corruption_detected, "invalid block type")` | `ZSTD_error_corruption_detected` | `phase_c_decompress` | [x] |
| 348 | `ZSTD_decompressFrame` (L1046) | `(U64)(op-ostart) != dctx->fParams.frameContentSize` | `ZSTD_error_corruption_detected` | `phase_c_decompress` | [x] |
| 349 | `ZSTD_decompressFrame` (L1050) | `remainingSrcSize<4` | `ZSTD_error_checksum_wrong` | `phase_c_decompress` | [x] |
| 350 | `ZSTD_decompressFrame` (L1055) | `checkRead != checkCalc` | `ZSTD_error_checksum_wrong` | `phase_c_decompress` | [x] |
| 351 | `ZSTD_decompressMultiFrame` (L1094) | `dctx->staticSize` | `ZSTD_error_memory_allocation` | `phase_c_decompress` | [x] |
| 352 | `ZSTD_decompressMultiFrame` (L1102) | `expectedSize == ZSTD_CONTENTSIZE_ERROR` | `ZSTD_error_corruption_detected` | `phase_c_decompress` | [x] |
| 353 | `ZSTD_decompressMultiFrame` (L1104) | `expectedSize != decodedSize` | `ZSTD_error_corruption_detected` | `phase_c_decompress` | [x] |
| 354 | `ZSTD_decompressMultiFrame` (L1146) | `(ZSTD_getErrorCode(res) == ZSTD_error_prefix_unknown) && (moreThan1Frame==1)` | `ZSTD_error_srcSize_wrong` | `phase_c_decompress` | [x] |
| 355 | `ZSTD_decompressMultiFrame` (L1166) | `srcSize` | `ZSTD_error_srcSize_wrong` | `phase_c_decompress` | [x] |
| 356 | `ZSTD_getDDict` (L1188) | `return NULL;` | `NULL` | `phase_c_decompress` | [x] |
| 357 | `ZSTD_decompress` (L1208) | `dctx==NULL` | `ZSTD_error_memory_allocation` | `phase_c_decompress` | [x] |
| 358 | `ZSTD_decompressContinue` (L1279) | `srcSize != ZSTD_nextSrcSizeToDecompressWithInputSize(dctx, srcSize)` | `ZSTD_error_srcSize_wrong` | `phase_c_decompress` | [x] |
| 359 | `ZSTD_decompressContinue` (L1315) | `cBlockSize > dctx->fParams.blockSizeMax` | `ZSTD_error_corruption_detected` | `phase_c_decompress` | [x] |
| 360 | `ZSTD_decompressContinue` (L1364) | `RETURN_ERROR(corruption_detected, "invalid block type")` | `ZSTD_error_corruption_detected` | `phase_c_decompress` | [x] |
| 361 | `ZSTD_decompressContinue` (L1367) | `rSize > dctx->fParams.blockSizeMax` | `ZSTD_error_corruption_detected` | `phase_c_decompress` | [x] |
| 362 | `ZSTD_decompressContinue` (L1380) | `dctx->fParams.frameContentSize != ZSTD_CONTENTSIZE_UNKNOWN && dctx->decodedSize != dctx->fParams.frameContentSize` | `ZSTD_error_corruption_detected` | `phase_c_decompress` | [x] |
| 363 | `ZSTD_decompressContinue` (L1406) | `check32 != h32` | `ZSTD_error_checksum_wrong` | `phase_c_decompress` | [x] |
| 364 | `ZSTD_decompressContinue` (L1430) | `RETURN_ERROR(GENERIC, "impossible to reach")` | `ZSTD_error_GENERIC` | `phase_c_decompress` | [x] |
| 365 | `ZSTD_refDictContent` (L1458) | `dictSize <= 8` | `ZSTD_error_dictionary_corrupted` | `phase_c_decompress` | [x] |
| 366 | `ZSTD_refDictContent` (L1477) | `HUF_isError(hSize)` | `ZSTD_error_dictionary_corrupted` | `phase_c_decompress` | [x] |
| 367 | `ZSTD_refDictContent` (L1484) | `FSE_isError(offcodeHeaderSize)` | `ZSTD_error_dictionary_corrupted` | `phase_c_decompress` | [x] |
| 368 | `ZSTD_refDictContent` (L1485) | `offcodeMaxValue > MaxOff` | `ZSTD_error_dictionary_corrupted` | `phase_c_decompress` | [x] |
| 369 | `ZSTD_refDictContent` (L1486) | `offcodeLog > OffFSELog` | `ZSTD_error_dictionary_corrupted` | `phase_c_decompress` | [x] |
| 370 | `ZSTD_refDictContent` (L1499) | `FSE_isError(matchlengthHeaderSize)` | `ZSTD_error_dictionary_corrupted` | `phase_c_decompress` | [x] |
| 371 | `ZSTD_refDictContent` (L1500) | `matchlengthMaxValue > MaxML` | `ZSTD_error_dictionary_corrupted` | `phase_c_decompress` | [x] |
| 372 | `ZSTD_refDictContent` (L1501) | `matchlengthLog > MLFSELog` | `ZSTD_error_dictionary_corrupted` | `phase_c_decompress` | [x] |
| 373 | `ZSTD_refDictContent` (L1514) | `FSE_isError(litlengthHeaderSize)` | `ZSTD_error_dictionary_corrupted` | `phase_c_decompress` | [x] |
| 374 | `ZSTD_refDictContent` (L1515) | `litlengthMaxValue > MaxLL` | `ZSTD_error_dictionary_corrupted` | `phase_c_decompress` | [x] |
| 375 | `ZSTD_refDictContent` (L1516) | `litlengthLog > LLFSELog` | `ZSTD_error_dictionary_corrupted` | `phase_c_decompress` | [x] |
| 376 | `ZSTD_refDictContent` (L1526) | `dictPtr+12 > dictEnd` | `ZSTD_error_dictionary_corrupted` | `phase_c_decompress` | [x] |
| 377 | `ZSTD_refDictContent` (L1531) | `rep==0 \|\| rep > dictContentSize` | `ZSTD_error_dictionary_corrupted` | `phase_c_decompress` | [x] |
| 378 | `ZSTD_decompress_insertDictionary` (L1550) | `ZSTD_isError(eSize)` | `ZSTD_error_dictionary_corrupted` | `phase_c_decompress` | [x] |
| 379 | `ZSTD_decompressBegin_usingDict` (L1592) | `ZSTD_isError(ZSTD_decompress_insertDictionary(dctx, dict, dictSize))` | `ZSTD_error_dictionary_corrupted` | `phase_c_decompress` | [x] |
| 380 | `ZSTD_DCtx_loadDictionary_advanced` (L1704) | `dctx->streamStage != zdss_init` | `ZSTD_error_stage_wrong` | `phase_c_decompress` | [x] |
| 381 | `ZSTD_DCtx_loadDictionary_advanced` (L1708) | `dctx->ddictLocal == NULL` | `ZSTD_error_memory_allocation` | `phase_c_decompress` | [x] |
| 382 | `ZSTD_DCtx_refDDict` (L1782) | `dctx->streamStage != zdss_init` | `ZSTD_error_stage_wrong` | `phase_c_decompress` | [x] |
| 383 | `ZSTD_DCtx_refDDict` (L1791) | `RETURN_ERROR(memory_allocation, "Failed to allocate memory for hash set!")` | `ZSTD_error_memory_allocation` | `phase_c_decompress` | [x] |
| 384 | `ZSTD_DCtx_setMaxWindowSize` (L1809) | `dctx->streamStage != zdss_init` | `ZSTD_error_stage_wrong` | `phase_c_decompress` | [x] |
| 385 | `ZSTD_DCtx_setMaxWindowSize` (L1810) | `maxWindowSize < min` | `ZSTD_error_parameter_outOfBound` | `phase_c_decompress` | [x] |
| 386 | `ZSTD_DCtx_setMaxWindowSize` (L1811) | `maxWindowSize > max` | `ZSTD_error_parameter_outOfBound` | `phase_c_decompress` | [x] |
| 387 | `ZSTD_dParam_getBounds` (L1857) | `= ERROR(parameter_unsupported)` | `ZSTD_error_parameter_unsupported` | `phase_c_decompress` | [x] |
| 388 | `ZSTD_dParam_withinBounds` (L1874) | `!ZSTD_dParam_withinBounds(p, v)` | `ZSTD_error_parameter_outOfBound` | `phase_c_decompress` | [x] |
| 389 | `ZSTD_DCtx_getParameter` (L1903) | `RETURN_ERROR(parameter_unsupported, "")` | `ZSTD_error_parameter_unsupported` | `phase_c_decompress` | [x] |
| 390 | `ZSTD_DCtx_setParameter` (L1908) | `dctx->streamStage != zdss_init` | `ZSTD_error_stage_wrong` | `phase_c_decompress` | [x] |
| 391 | `ZSTD_DCtx_setParameter` (L1930) | `RETURN_ERROR(parameter_unsupported, "Static dctx does not support multiple DDicts!")` | `ZSTD_error_parameter_unsupported` | `phase_c_decompress` | [x] |
| 392 | `ZSTD_DCtx_setParameter` (L1944) | `RETURN_ERROR(parameter_unsupported, "")` | `ZSTD_error_parameter_unsupported` | `phase_c_decompress` | [x] |
| 393 | `ZSTD_DCtx_reset` (L1957) | `dctx->streamStage != zdss_init` | `ZSTD_error_stage_wrong` | `phase_c_decompress` | [x] |
| 394 | `ZSTD_decodingBufferSize_internal` (L1983) | `(unsigned long long)minRBSize != neededSize` | `ZSTD_error_frameParameter_windowTooLarge` | `phase_c_decompress` | [x] |
| 395 | `ZSTD_estimateDStreamSize_fromFrame` (L2007) | `err>0` | `ZSTD_error_srcSize_wrong` | `phase_c_decompress` | [x] |
| 396 | `ZSTD_estimateDStreamSize_fromFrame` (L2008) | `zfh.windowSize > windowSizeMax` | `ZSTD_error_frameParameter_windowTooLarge` | `phase_c_decompress` | [x] |
| 397 | `ZSTD_checkOutBuffer` (L2049) | `RETURN_ERROR(dstBuffer_wrong, "ZSTD_d_stableOutBuffer enabled but output differs!")` | `ZSTD_error_dstBuffer_wrong` | `phase_c_decompress` | [x] |
| 398 | `ZSTD_decompressStream` (L2100) | `input->pos > input->size` | `ZSTD_error_srcSize_wrong` | `phase_c_decompress` | [x] |
| 399 | `ZSTD_decompressStream` (L2105) | `output->pos > output->size` | `ZSTD_error_dstSize_tooSmall` | `phase_c_decompress` | [x] |
| 400 | `ZSTD_decompressStream` (L2131) | `zds->staticSize` | `ZSTD_error_memory_allocation` | `phase_c_decompress` | [x] |
| 401 | `ZSTD_decompressStream` (L2150) | `zds->staticSize` | `ZSTD_error_memory_allocation` | `phase_c_decompress` | [x] |
| 402 | `ZSTD_decompressStream` (L2209) | `RETURN_ERROR(dstSize_tooSmall, "ZSTD_obm_stable passed but ZSTD_outBuffer is too small")` | `ZSTD_error_dstSize_tooSmall` | `phase_c_decompress` | [x] |
| 403 | `ZSTD_decompressStream` (L2231) | `zds->fParams.windowSize > zds->maxWindowSize` | `ZSTD_error_frameParameter_windowTooLarge` | `phase_c_decompress` | [x] |
| 404 | `ZSTD_decompressStream` (L2256) | `bufferSize > zds->staticSize - sizeof(ZSTD_DCtx)` | `ZSTD_error_memory_allocation` | `phase_c_decompress` | [x] |
| 405 | `ZSTD_decompressStream` (L2264) | `zds->inBuff == NULL` | `ZSTD_error_memory_allocation` | `phase_c_decompress` | [x] |
| 406 | `ZSTD_decompressStream` (L2303) | `toLoad > zds->inBuffSize - zds->inPos` | `ZSTD_error_corruption_detected` | `phase_c_decompress` | [x] |
| 407 | `ZSTD_decompressStream` (L2346) | `RETURN_ERROR(GENERIC, "impossible to reach")` | `ZSTD_error_GENERIC` | `phase_c_decompress` | [x] |
| 408 | `ZSTD_decompressStream` (L2359) | `op==oend` | `ZSTD_error_noForwardProgress_destFull` | `phase_c_decompress` | [x] |
| 409 | `ZSTD_decompressStream` (L2360) | `ip==iend` | `ZSTD_error_noForwardProgress_inputEmpty` | `phase_c_decompress` | [x] |

### `decompress/zstd_decompress_block.c`

| # | function | trigger (the exact invalid input/condition) | expected C result | covering test | [x] |
|---|----------|---------------------------------------------|-------------------|---------------|-----|
| 410 | `ZSTD_getcBlockSize` (L66) | `srcSize < ZSTD_blockHeaderSize` | `ZSTD_error_srcSize_wrong` | `phase_c_decompress` | [x] |
| 411 | `ZSTD_getcBlockSize` (L74) | `bpPtr->blockType == bt_reserved` | `ZSTD_error_corruption_detected` | `phase_c_decompress` | [x] |
| 412 | `ZSTD_decodeLiteralsBlock` (L139) | `srcSize < MIN_CBLOCK_SIZE` | `ZSTD_error_corruption_detected` | `phase_c_decompress` | [x] |
| 413 | `ZSTD_decodeLiteralsBlock` (L149) | `dctx->litEntropy==0` | `ZSTD_error_dictionary_corrupted` | `phase_c_decompress` | [x] |
| 414 | `ZSTD_decodeLiteralsBlock` (L153) | `srcSize < 5` | `ZSTD_error_corruption_detected` | `phase_c_decompress` | [x] |
| 415 | `ZSTD_decodeLiteralsBlock` (L185) | `litSize > 0 && dst == NULL` | `ZSTD_error_dstSize_tooSmall` | `phase_c_decompress` | [x] |
| 416 | `ZSTD_decodeLiteralsBlock` (L186) | `litSize > blockSizeMax` | `ZSTD_error_corruption_detected` | `phase_c_decompress` | [x] |
| 417 | `ZSTD_decodeLiteralsBlock` (L188) | `litSize < MIN_LITERALS_FOR_4_STREAMS` | `ZSTD_error_literals_headerWrong` | `phase_c_decompress` | [x] |
| 418 | `ZSTD_decodeLiteralsBlock` (L191) | `litCSize + lhSize > srcSize` | `ZSTD_error_corruption_detected` | `phase_c_decompress` | [x] |
| 419 | `ZSTD_decodeLiteralsBlock` (L192) | `expectedWriteSize < litSize` | `ZSTD_error_dstSize_tooSmall` | `phase_c_decompress` | [x] |
| 420 | `ZSTD_decodeLiteralsBlock` (L241) | `HUF_isError(hufSuccess)` | `ZSTD_error_corruption_detected` | `phase_c_decompress` | [x] |
| 421 | `ZSTD_decodeLiteralsBlock` (L266) | `srcSize<3` | `ZSTD_error_corruption_detected` | `phase_c_decompress` | [x] |
| 422 | `ZSTD_decodeLiteralsBlock` (L271) | `litSize > 0 && dst == NULL` | `ZSTD_error_dstSize_tooSmall` | `phase_c_decompress` | [x] |
| 423 | `ZSTD_decodeLiteralsBlock` (L272) | `litSize > blockSizeMax` | `ZSTD_error_corruption_detected` | `phase_c_decompress` | [x] |
| 424 | `ZSTD_decodeLiteralsBlock` (L273) | `expectedWriteSize < litSize` | `ZSTD_error_dstSize_tooSmall` | `phase_c_decompress` | [x] |
| 425 | `ZSTD_decodeLiteralsBlock` (L276) | `litSize+lhSize > srcSize` | `ZSTD_error_corruption_detected` | `phase_c_decompress` | [x] |
| 426 | `ZSTD_decodeLiteralsBlock` (L310) | `srcSize<3` | `ZSTD_error_corruption_detected` | `phase_c_decompress` | [x] |
| 427 | `ZSTD_decodeLiteralsBlock` (L315) | `srcSize<4` | `ZSTD_error_corruption_detected` | `phase_c_decompress` | [x] |
| 428 | `ZSTD_decodeLiteralsBlock` (L319) | `litSize > 0 && dst == NULL` | `ZSTD_error_dstSize_tooSmall` | `phase_c_decompress` | [x] |
| 429 | `ZSTD_decodeLiteralsBlock` (L320) | `litSize > blockSizeMax` | `ZSTD_error_corruption_detected` | `phase_c_decompress` | [x] |
| 430 | `ZSTD_decodeLiteralsBlock` (L321) | `expectedWriteSize < litSize` | `ZSTD_error_dstSize_tooSmall` | `phase_c_decompress` | [x] |
| 431 | `ZSTD_decodeLiteralsBlock` (L337) | `RETURN_ERROR(corruption_detected, "impossible")` | `ZSTD_error_corruption_detected` | `phase_c_decompress` | [x] |
| 432 | `ZSTD_buildSeqTable` (L658) | `!srcSize` | `ZSTD_error_srcSize_wrong` | `phase_c_decompress` | [x] |
| 433 | `ZSTD_buildSeqTable` (L659) | `(*(const BYTE*)src) > max` | `ZSTD_error_corruption_detected` | `phase_c_decompress` | [x] |
| 434 | `ZSTD_buildSeqTable` (L671) | `!flagRepeatTable` | `ZSTD_error_corruption_detected` | `phase_c_decompress` | [x] |
| 435 | `ZSTD_buildSeqTable` (L683) | `FSE_isError(headerSize)` | `ZSTD_error_corruption_detected` | `phase_c_decompress` | [x] |
| 436 | `ZSTD_buildSeqTable` (L684) | `tableLog > maxLog` | `ZSTD_error_corruption_detected` | `phase_c_decompress` | [x] |
| 437 | `ZSTD_buildSeqTable` (L691) | `RETURN_ERROR(GENERIC, "impossible")` | `ZSTD_error_GENERIC` | `phase_c_decompress` | [x] |
| 438 | `ZSTD_decodeSeqHeaders` (L705) | `srcSize < MIN_SEQUENCES_SIZE` | `ZSTD_error_srcSize_wrong` | `phase_c_decompress` | [x] |
| 439 | `ZSTD_decodeSeqHeaders` (L711) | `ip+2 > iend` | `ZSTD_error_srcSize_wrong` | `phase_c_decompress` | [x] |
| 440 | `ZSTD_decodeSeqHeaders` (L715) | `ip >= iend` | `ZSTD_error_srcSize_wrong` | `phase_c_decompress` | [x] |
| 441 | `ZSTD_decodeSeqHeaders` (L723) | `ip != iend` | `ZSTD_error_corruption_detected` | `phase_c_decompress` | [x] |
| 442 | `ZSTD_decodeSeqHeaders` (L729) | `ip+1 > iend` | `ZSTD_error_srcSize_wrong` | `phase_c_decompress` | [x] |
| 443 | `ZSTD_decodeSeqHeaders` (L730) | `*ip & 3` | `ZSTD_error_corruption_detected` | `phase_c_decompress` | [x] |
| 444 | `ZSTD_decodeSeqHeaders` (L745) | `ZSTD_isError(llhSize)` | `ZSTD_error_corruption_detected` | `phase_c_decompress` | [x] |
| 445 | `ZSTD_decodeSeqHeaders` (L757) | `ZSTD_isError(ofhSize)` | `ZSTD_error_corruption_detected` | `phase_c_decompress` | [x] |
| 446 | `ZSTD_decodeSeqHeaders` (L769) | `ZSTD_isError(mlhSize)` | `ZSTD_error_corruption_detected` | `phase_c_decompress` | [x] |
| 447 | `ZSTD_execSequenceEnd` (L919) | `sequenceLength > (size_t)(oend - op)` | `ZSTD_error_dstSize_tooSmall` | `phase_c_decompress` | [x] |
| 448 | `ZSTD_execSequenceEnd` (L920) | `sequence.litLength > (size_t)(litLimit - *litPtr)` | `ZSTD_error_corruption_detected` | `phase_c_decompress` | [x] |
| 449 | `ZSTD_execSequenceEnd` (L932) | `sequence.offset > (size_t)(oLitEnd - virtualStart)` | `ZSTD_error_corruption_detected` | `phase_c_decompress` | [x] |
| 450 | `ZSTD_execSequenceEndSplitLitBuffer` (L967) | `sequenceLength > (size_t)(oend - op)` | `ZSTD_error_dstSize_tooSmall` | `phase_c_decompress` | [x] |
| 451 | `ZSTD_execSequenceEndSplitLitBuffer` (L968) | `sequence.litLength > (size_t)(litLimit - *litPtr)` | `ZSTD_error_corruption_detected` | `phase_c_decompress` | [x] |
| 452 | `ZSTD_execSequenceEndSplitLitBuffer` (L973) | `op > *litPtr && op < *litPtr + sequence.litLength` | `ZSTD_error_dstSize_tooSmall` | `phase_c_decompress` | [x] |
| 453 | `ZSTD_execSequenceEndSplitLitBuffer` (L981) | `sequence.offset > (size_t)(oLitEnd - virtualStart)` | `ZSTD_error_corruption_detected` | `phase_c_decompress` | [x] |
| 454 | `ZSTD_execSequence` (L1054) | `UNLIKELY(sequence.offset > (size_t)(oLitEnd - virtualStart))` | `ZSTD_error_corruption_detected` | `phase_c_decompress` | [x] |
| 455 | `ZSTD_execSequenceSplitLitBuffer` (L1147) | `UNLIKELY(sequence.offset > (size_t)(oLitEnd - virtualStart))` | `ZSTD_error_corruption_detected` | `phase_c_decompress` | [x] |
| 456 | `ZSTD_assertValidSequence` (L1425) | `ERR_isError(BIT_initDStream(&seqState.DStream, ip, iend-ip))` | `ZSTD_error_corruption_detected` | `phase_c_decompress` | [x] |
| 457 | `ZSTD_assertValidSequence` (L1521) | `leftoverLit > (size_t)(oend - op)` | `ZSTD_error_dstSize_tooSmall` | `phase_c_decompress` | [x] |
| 458 | `ZSTD_assertValidSequence` (L1579) | `nbSeq` | `ZSTD_error_corruption_detected` | `phase_c_decompress` | [x] |
| 459 | `ZSTD_assertValidSequence` (L1581) | `!BIT_endOfDStream(&seqState.DStream)` | `ZSTD_error_corruption_detected` | `phase_c_decompress` | [x] |
| 460 | `ZSTD_assertValidSequence` (L1591) | `lastLLSize > (size_t)(oend - op)` | `ZSTD_error_dstSize_tooSmall` | `phase_c_decompress` | [x] |
| 461 | `ZSTD_assertValidSequence` (L1603) | `lastLLSize > (size_t)(oend-op)` | `ZSTD_error_dstSize_tooSmall` | `phase_c_decompress` | [x] |
| 462 | `ZSTD_assertValidSequence` (L1637) | `ERR_isError(BIT_initDStream(&seqState.DStream, ip, iend - ip))` | `ZSTD_error_corruption_detected` | `phase_c_decompress` | [x] |
| 463 | `ZSTD_assertValidSequence` (L1674) | `!BIT_endOfDStream(&seqState.DStream)` | `ZSTD_error_corruption_detected` | `phase_c_decompress` | [x] |
| 464 | `ZSTD_assertValidSequence` (L1682) | `lastLLSize > (size_t)(oend-op)` | `ZSTD_error_dstSize_tooSmall` | `phase_c_decompress` | [x] |
| 465 | `ZSTD_prefetchMatch` (L1765) | `ERR_isError(BIT_initDStream(&seqState.DStream, ip, iend-ip))` | `ZSTD_error_corruption_detected` | `phase_c_decompress` | [x] |
| 466 | `ZSTD_prefetchMatch` (L1788) | `leftoverLit > (size_t)(oend - op)` | `ZSTD_error_dstSize_tooSmall` | `phase_c_decompress` | [x] |
| 467 | `ZSTD_prefetchMatch` (L1824) | `!BIT_endOfDStream(&seqState.DStream)` | `ZSTD_error_corruption_detected` | `phase_c_decompress` | [x] |
| 468 | `ZSTD_prefetchMatch` (L1833) | `leftoverLit > (size_t)(oend - op)` | `ZSTD_error_dstSize_tooSmall` | `phase_c_decompress` | [x] |
| 469 | `ZSTD_prefetchMatch` (L1871) | `lastLLSize > (size_t)(oend - op)` | `ZSTD_error_dstSize_tooSmall` | `phase_c_decompress` | [x] |
| 470 | `ZSTD_prefetchMatch` (L1880) | `lastLLSize > (size_t)(oend-op)` | `ZSTD_error_dstSize_tooSmall` | `phase_c_decompress` | [x] |
| 471 | `ZSTD_maxShortOffset` (L2081) | `srcSize > ZSTD_blockSizeMax(dctx)` | `ZSTD_error_srcSize_wrong` | `phase_c_decompress` | [x] |
| 472 | `ZSTD_maxShortOffset` (L2129) | `(dst == NULL \|\| dstCapacity == 0) && nbSeq > 0` | `ZSTD_error_dstSize_tooSmall` | `phase_c_decompress` | [x] |
| 473 | `ZSTD_maxShortOffset` (L2130) | `MEM_64bits() && sizeof(size_t) == sizeof(void*) && (size_t)(-1) - (size_t)dst < (size_t)(1 << 20)` | `ZSTD_error_dstSize_tooSmall` | `phase_c_decompress` | [x] |

### `dictBuilder/cover.c`

| # | function | trigger (the exact invalid input/condition) | expected C result | covering test | [x] |
|---|----------|---------------------------------------------|-------------------|---------------|-----|
| 474 | `COVER_cmp8` (L283) | `return -1;` | `-1` | `phase_c_dictbuilder` | [x] |
| 475 | `COVER_ctx_init` (L618) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_c_dictbuilder` | [x] |
| 476 | `COVER_ctx_init` (L623) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_c_dictbuilder` | [x] |
| 477 | `COVER_ctx_init` (L628) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_c_dictbuilder` | [x] |
| 478 | `COVER_ctx_init` (L651) | `return ERROR(memory_allocation)` | `ZSTD_error_memory_allocation` | `phase_c_dictbuilder` | [x] |
| 479 | `ZDICT_trainFromBuffer_cover` (L793) | `return ERROR(parameter_outOfBound)` | `ZSTD_error_parameter_outOfBound` | `phase_c_dictbuilder` | [x] |
| 480 | `ZDICT_trainFromBuffer_cover` (L797) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_c_dictbuilder` | [x] |
| 481 | `ZDICT_trainFromBuffer_cover` (L802) | `return ERROR(dstSize_tooSmall)` | `ZSTD_error_dstSize_tooSmall` | `phase_c_dictbuilder` | [x] |
| 482 | `ZDICT_trainFromBuffer_cover` (L816) | `return ERROR(memory_allocation)` | `ZSTD_error_memory_allocation` | `phase_c_dictbuilder` | [x] |
| 483 | `COVER_checkTotalCompressedSize` (L844) | `= ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_c_dictbuilder` | [x] |
| 484 | `COVER_best_finish` (L977) | `= ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_c_dictbuilder` | [x] |
| 485 | `COVER_tryParameters` (L1129) | `= ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_c_dictbuilder` | [x] |
| 486 | `ZDICT_optimizeTrainFromBuffer_cover` (L1197) | `return ERROR(parameter_outOfBound)` | `ZSTD_error_parameter_outOfBound` | `phase_c_dictbuilder` | [x] |
| 487 | `ZDICT_optimizeTrainFromBuffer_cover` (L1201) | `return ERROR(parameter_outOfBound)` | `ZSTD_error_parameter_outOfBound` | `phase_c_dictbuilder` | [x] |
| 488 | `ZDICT_optimizeTrainFromBuffer_cover` (L1205) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_c_dictbuilder` | [x] |
| 489 | `ZDICT_optimizeTrainFromBuffer_cover` (L1210) | `return ERROR(dstSize_tooSmall)` | `ZSTD_error_dstSize_tooSmall` | `phase_c_dictbuilder` | [x] |
| 490 | `ZDICT_optimizeTrainFromBuffer_cover` (L1215) | `return ERROR(memory_allocation)` | `ZSTD_error_memory_allocation` | `phase_c_dictbuilder` | [x] |
| 491 | `ZDICT_optimizeTrainFromBuffer_cover` (L1253) | `return ERROR(memory_allocation)` | `ZSTD_error_memory_allocation` | `phase_c_dictbuilder` | [x] |

### `dictBuilder/divsufsort.c`

| # | function | trigger (the exact invalid input/condition) | expected C result | covering test | [x] |
|---|----------|---------------------------------------------|-------------------|---------------|-----|
| 492 | `?` (L1853) | `return -1;` | `-1` | `phase_c_dictbuilder` | [x] |
| 493 | `?` (L1882) | `return -1;` | `-1` | `phase_c_dictbuilder` | [x] |

### `dictBuilder/fastcover.c`

| # | function | trigger (the exact invalid input/condition) | expected C result | covering test | [x] |
|---|----------|---------------------------------------------|-------------------|---------------|-----|
| 494 | `FASTCOVER_checkParameters` (L332) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_c_dictbuilder` | [x] |
| 495 | `FASTCOVER_checkParameters` (L338) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_c_dictbuilder` | [x] |
| 496 | `FASTCOVER_checkParameters` (L344) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_c_dictbuilder` | [x] |
| 497 | `FASTCOVER_checkParameters` (L369) | `return ERROR(memory_allocation)` | `ZSTD_error_memory_allocation` | `phase_c_dictbuilder` | [x] |
| 498 | `FASTCOVER_checkParameters` (L386) | `return ERROR(memory_allocation)` | `ZSTD_error_memory_allocation` | `phase_c_dictbuilder` | [x] |
| 499 | `FASTCOVER_tryParameters` (L480) | `= ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_c_dictbuilder` | [x] |
| 500 | `FASTCOVER_tryParameters` (L571) | `return ERROR(parameter_outOfBound)` | `ZSTD_error_parameter_outOfBound` | `phase_c_dictbuilder` | [x] |
| 501 | `FASTCOVER_tryParameters` (L575) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_c_dictbuilder` | [x] |
| 502 | `FASTCOVER_tryParameters` (L580) | `return ERROR(dstSize_tooSmall)` | `ZSTD_error_dstSize_tooSmall` | `phase_c_dictbuilder` | [x] |
| 503 | `FASTCOVER_tryParameters` (L652) | `return ERROR(parameter_outOfBound)` | `ZSTD_error_parameter_outOfBound` | `phase_c_dictbuilder` | [x] |
| 504 | `FASTCOVER_tryParameters` (L656) | `return ERROR(parameter_outOfBound)` | `ZSTD_error_parameter_outOfBound` | `phase_c_dictbuilder` | [x] |
| 505 | `FASTCOVER_tryParameters` (L660) | `return ERROR(parameter_outOfBound)` | `ZSTD_error_parameter_outOfBound` | `phase_c_dictbuilder` | [x] |
| 506 | `FASTCOVER_tryParameters` (L664) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_c_dictbuilder` | [x] |
| 507 | `FASTCOVER_tryParameters` (L669) | `return ERROR(dstSize_tooSmall)` | `ZSTD_error_dstSize_tooSmall` | `phase_c_dictbuilder` | [x] |
| 508 | `FASTCOVER_tryParameters` (L674) | `return ERROR(memory_allocation)` | `ZSTD_error_memory_allocation` | `phase_c_dictbuilder` | [x] |
| 509 | `FASTCOVER_tryParameters` (L715) | `return ERROR(memory_allocation)` | `ZSTD_error_memory_allocation` | `phase_c_dictbuilder` | [x] |

### `dictBuilder/zdict.c`

| # | function | trigger (the exact invalid input/condition) | expected C result | covering test | [x] |
|---|----------|---------------------------------------------|-------------------|---------------|-----|
| 510 | `ZDICT_getDictHeaderSize` (L112) | `return ERROR(dictionary_corrupted)` | `ZSTD_error_dictionary_corrupted` | `phase_c_dictbuilder` | [x] |
| 511 | `ZDICT_getDictHeaderSize` (L117) | `= ERROR(memory_allocation)` | `ZSTD_error_memory_allocation` | `phase_c_dictbuilder` | [x] |
| 512 | `ZDICT_trainBuffer_legacy` (L494) | `= ERROR(memory_allocation)` | `ZSTD_error_memory_allocation` | `phase_c_dictbuilder` | [x] |
| 513 | `ZDICT_trainBuffer_legacy` (L507) | `= ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_c_dictbuilder` | [x] |
| 514 | `ZDICT_analyzeEntropy` (L688) | `= ERROR(dictionaryCreation_failed)` | `ZSTD_error_dictionaryCreation_failed` | `phase_c_dictbuilder` | [x] |
| 515 | `ZDICT_analyzeEntropy` (L703) | `= ERROR(memory_allocation)` | `ZSTD_error_memory_allocation` | `phase_c_dictbuilder` | [x] |
| 516 | `ZDICT_analyzeEntropy` (L820) | `= ERROR(dstSize_tooSmall)` | `ZSTD_error_dstSize_tooSmall` | `phase_c_dictbuilder` | [x] |
| 517 | `ZDICT_finalizeDictionary` (L874) | `return ERROR(dstSize_tooSmall)` | `ZSTD_error_dstSize_tooSmall` | `phase_c_dictbuilder` | [x] |
| 518 | `ZDICT_finalizeDictionary` (L875) | `return ERROR(dstSize_tooSmall)` | `ZSTD_error_dstSize_tooSmall` | `phase_c_dictbuilder` | [x] |
| 519 | `ZDICT_finalizeDictionary` (L905) | `hSize + minContentSize > dictBufferCapacity` | `ZSTD_error_dstSize_tooSmall` | `phase_c_dictbuilder` | [x] |
| 520 | `ZDICT_trainFromBuffer_unsafe_legacy` (L993) | `return ERROR(memory_allocation)` | `ZSTD_error_memory_allocation` | `phase_c_dictbuilder` | [x] |
| 521 | `ZDICT_trainFromBuffer_unsafe_legacy` (L994) | `return ERROR(dstSize_tooSmall)` | `ZSTD_error_dstSize_tooSmall` | `phase_c_dictbuilder` | [x] |
| 522 | `ZDICT_trainFromBuffer_unsafe_legacy` (L995) | `return ERROR(dictionaryCreation_failed)` | `ZSTD_error_dictionaryCreation_failed` | `phase_c_dictbuilder` | [x] |
| 523 | `ZDICT_trainFromBuffer_unsafe_legacy` (L1019) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_c_dictbuilder` | [x] |
| 524 | `ZDICT_trainFromBuffer_unsafe_legacy` (L1030) | `return ERROR(dictionaryCreation_failed)` | `ZSTD_error_dictionaryCreation_failed` | `phase_c_dictbuilder` | [x] |
| 525 | `ZDICT_trainFromBuffer_unsafe_legacy` (L1066) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_c_dictbuilder` | [x] |
| 526 | `ZDICT_trainFromBuffer_legacy` (L1094) | `return ERROR(memory_allocation)` | `ZSTD_error_memory_allocation` | `phase_c_dictbuilder` | [x] |

### `legacy/zstd_legacy.h`

| # | function | trigger (the exact invalid input/condition) | expected C result | covering test | [x] |
|---|----------|---------------------------------------------|-------------------|---------------|-----|
| 527 | `ZSTD_decompressLegacy` (L164) | `return ERROR(memory_allocation)` | `ZSTD_error_memory_allocation` | `phase_b_legacy` | [x] |
| 528 | `ZSTD_decompressLegacy` (L174) | `return ERROR(memory_allocation)` | `ZSTD_error_memory_allocation` | `phase_b_legacy` | [x] |
| 529 | `ZSTD_decompressLegacy` (L184) | `return ERROR(memory_allocation)` | `ZSTD_error_memory_allocation` | `phase_b_legacy` | [x] |
| 530 | `ZSTD_decompressLegacy` (L191) | `return ERROR(prefix_unknown)` | `ZSTD_error_prefix_unknown` | `phase_b_legacy` | [x] |
| 531 | `ZSTD_findFrameSizeInfoLegacy` (L251) | `= ERROR(prefix_unknown)` | `ZSTD_error_prefix_unknown` | `phase_b_legacy` | [x] |
| 532 | `ZSTD_findFrameSizeInfoLegacy` (L256) | `= ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 533 | `ZSTD_freeLegacyStreamContext` (L284) | `return ERROR(version_unsupported)` | `ZSTD_error_version_unsupported` | `phase_b_legacy` | [x] |
| 534 | `ZSTD_initLegacyStream` (L324) | `return ERROR(memory_allocation)` | `ZSTD_error_memory_allocation` | `phase_b_legacy` | [x] |
| 535 | `ZSTD_initLegacyStream` (L335) | `return ERROR(memory_allocation)` | `ZSTD_error_memory_allocation` | `phase_b_legacy` | [x] |
| 536 | `ZSTD_initLegacyStream` (L345) | `return ERROR(memory_allocation)` | `ZSTD_error_memory_allocation` | `phase_b_legacy` | [x] |
| 537 | `ZSTD_initLegacyStream` (L355) | `return ERROR(memory_allocation)` | `ZSTD_error_memory_allocation` | `phase_b_legacy` | [x] |
| 538 | `ZSTD_decompressLegacyStream` (L387) | `return ERROR(version_unsupported)` | `ZSTD_error_version_unsupported` | `phase_b_legacy` | [x] |

### `legacy/zstd_v01.c`

| # | function | trigger (the exact invalid input/condition) | expected C result | covering test | [x] |
|---|----------|---------------------------------------------|-------------------|---------------|-----|
| 539 | `ZSTDv01_getcBlockSize` (L1431) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 540 | `ZSTD_copyUncompressedBlock` (L1447) | `return ERROR(dstSize_tooSmall)` | `ZSTD_error_dstSize_tooSmall` | `phase_b_legacy` | [x] |
| 541 | `ZSTD_decompressLiterals` (L1466) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 542 | `ZSTD_decompressLiterals` (L1473) | `return ERROR(dstSize_tooSmall)` | `ZSTD_error_dstSize_tooSmall` | `phase_b_legacy` | [x] |
| 543 | `ZSTD_decompressLiterals` (L1475) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_b_legacy` | [x] |
| 544 | `ZSTDv01_decodeLiteralsBlock` (L1493) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 545 | `ZSTDv01_decodeLiteralsBlock` (L1506) | `return ERROR(dstSize_tooSmall)` | `ZSTD_error_dstSize_tooSmall` | `phase_b_legacy` | [x] |
| 546 | `ZSTDv01_decodeLiteralsBlock` (L1507) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 547 | `ZSTDv01_decodeLiteralsBlock` (L1527) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_b_legacy` | [x] |
| 548 | `ZSTDv01_decodeSeqHeaders` (L1546) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 549 | `ZSTDv01_decodeSeqHeaders` (L1570) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 550 | `ZSTDv01_decodeSeqHeaders` (L1589) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_b_legacy` | [x] |
| 551 | `ZSTDv01_decodeSeqHeaders` (L1590) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 552 | `ZSTDv01_decodeSeqHeaders` (L1599) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 553 | `ZSTDv01_decodeSeqHeaders` (L1607) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_b_legacy` | [x] |
| 554 | `ZSTDv01_decodeSeqHeaders` (L1608) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 555 | `ZSTDv01_decodeSeqHeaders` (L1617) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 556 | `ZSTDv01_decodeSeqHeaders` (L1625) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_b_legacy` | [x] |
| 557 | `ZSTDv01_decodeSeqHeaders` (L1626) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 558 | `ZSTD_execSequence` (L1732) | `return ERROR(dstSize_tooSmall)` | `ZSTD_error_dstSize_tooSmall` | `phase_b_legacy` | [x] |
| 559 | `ZSTD_execSequence` (L1733) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 560 | `ZSTD_execSequence` (L1735) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 561 | `ZSTD_execSequence` (L1737) | `return ERROR(dstSize_tooSmall)` | `ZSTD_error_dstSize_tooSmall` | `phase_b_legacy` | [x] |
| 562 | `ZSTD_execSequence` (L1738) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 563 | `ZSTD_execSequence` (L1739) | `return ERROR(dstSize_tooSmall)` | `ZSTD_error_dstSize_tooSmall` | `phase_b_legacy` | [x] |
| 564 | `ZSTD_execSequence` (L1748) | `return ERROR(dstSize_tooSmall)` | `ZSTD_error_dstSize_tooSmall` | `phase_b_legacy` | [x] |
| 565 | `ZSTD_execSequence` (L1758) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 566 | `ZSTD_execSequence` (L1759) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 567 | `ZSTD_decompressSequences` (L1853) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 568 | `ZSTD_decompressSequences` (L1869) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 569 | `ZSTD_decompressSequences` (L1870) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 570 | `ZSTD_decompressSequences` (L1875) | `return ERROR(dstSize_tooSmall)` | `ZSTD_error_dstSize_tooSmall` | `phase_b_legacy` | [x] |
| 571 | `ZSTDv01_decompressDCtx` (L1921) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 572 | `ZSTDv01_decompressDCtx` (L1923) | `return ERROR(prefix_unknown)` | `ZSTD_error_prefix_unknown` | `phase_b_legacy` | [x] |
| 573 | `ZSTDv01_decompressDCtx` (L1934) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 574 | `ZSTDv01_decompressDCtx` (L1945) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_b_legacy` | [x] |
| 575 | `ZSTDv01_decompressDCtx` (L1949) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 576 | `ZSTDv01_decompressDCtx` (L1952) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_b_legacy` | [x] |
| 577 | `ZSTDv01_createDCtx` (L2043) | `return NULL;` | `NULL` | `phase_b_legacy` | [x] |
| 578 | `ZSTDv01_decompressContinue` (L2064) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 579 | `ZSTDv01_decompressContinue` (L2073) | `return ERROR(prefix_unknown)` | `ZSTD_error_prefix_unknown` | `phase_b_legacy` | [x] |
| 580 | `ZSTDv01_decompressContinue` (L2112) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_b_legacy` | [x] |
| 581 | `ZSTDv01_decompressContinue` (L2118) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_b_legacy` | [x] |

### `legacy/zstd_v02.c`

| # | function | trigger (the exact invalid input/condition) | expected C result | covering test | [x] |
|---|----------|---------------------------------------------|-------------------|---------------|-----|
| 582 | `BIT_initDStream` (L325) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 583 | `BIT_initDStream` (L334) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_b_legacy` | [x] |
| 584 | `BIT_initDStream` (L360) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_b_legacy` | [x] |
| 585 | `FSE_tableStep` (L1051) | `return ERROR(maxSymbolValue_tooLarge)` | `ZSTD_error_maxSymbolValue_tooLarge` | `phase_b_legacy` | [x] |
| 586 | `FSE_tableStep` (L1052) | `return ERROR(tableLog_tooLarge)` | `ZSTD_error_tableLog_tooLarge` | `phase_b_legacy` | [x] |
| 587 | `FSE_tableStep` (L1082) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_b_legacy` | [x] |
| 588 | `FSE_readNCount` (L1131) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 589 | `FSE_readNCount` (L1134) | `return ERROR(tableLog_tooLarge)` | `ZSTD_error_tableLog_tooLarge` | `phase_b_legacy` | [x] |
| 590 | `FSE_readNCount` (L1169) | `return ERROR(maxSymbolValue_tooSmall)` | `ZSTD_error_maxSymbolValue_tooSmall` | `phase_b_legacy` | [x] |
| 591 | `FSE_readNCount` (L1221) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_b_legacy` | [x] |
| 592 | `FSE_readNCount` (L1225) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 593 | `FSE_buildDTable_raw` (L1261) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_b_legacy` | [x] |
| 594 | `FSE_decompress_usingDTable_generic` (L1340) | `return ERROR(dstSize_tooSmall)` | `ZSTD_error_dstSize_tooSmall` | `phase_b_legacy` | [x] |
| 595 | `FSE_decompress_usingDTable_generic` (L1342) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 596 | `FSE_decompress` (L1369) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 597 | `FSE_decompress` (L1374) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 598 | `HUF_readStats` (L1492) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 599 | `HUF_readStats` (L1509) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 600 | `HUF_readStats` (L1510) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 601 | `HUF_readStats` (L1521) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 602 | `HUF_readStats` (L1531) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 603 | `HUF_readStats` (L1535) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 604 | `HUF_readStats` (L1539) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 605 | `HUF_readStats` (L1545) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 606 | `HUF_readStats` (L1551) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 607 | `HUF_readDTableX2` (L1584) | `return ERROR(tableLog_tooLarge)` | `ZSTD_error_tableLog_tooLarge` | `phase_b_legacy` | [x] |
| 608 | `HUF_decompress4X2_usingDTable` (L1661) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 609 | `HUF_decompress4X2_usingDTable` (L1697) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 610 | `HUF_decompress4X2_usingDTable` (L1732) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 611 | `HUF_decompress4X2_usingDTable` (L1733) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 612 | `HUF_decompress4X2_usingDTable` (L1734) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 613 | `HUF_decompress4X2_usingDTable` (L1745) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 614 | `HUF_decompress4X2` (L1761) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 615 | `HUF_readDTableX4` (L1882) | `return ERROR(tableLog_tooLarge)` | `ZSTD_error_tableLog_tooLarge` | `phase_b_legacy` | [x] |
| 616 | `HUF_readDTableX4` (L1889) | `return ERROR(tableLog_tooLarge)` | `ZSTD_error_tableLog_tooLarge` | `phase_b_legacy` | [x] |
| 617 | `HUF_readDTableX4` (L1893) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_b_legacy` | [x] |
| 618 | `HUF_decompress4X4_usingDTable` (L2023) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 619 | `HUF_decompress4X4_usingDTable` (L2059) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 620 | `HUF_decompress4X4_usingDTable` (L2094) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 621 | `HUF_decompress4X4_usingDTable` (L2095) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 622 | `HUF_decompress4X4_usingDTable` (L2096) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 623 | `HUF_decompress4X4_usingDTable` (L2107) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 624 | `HUF_decompress4X4` (L2122) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 625 | `HUF_readDTableX6` (L2215) | `return ERROR(tableLog_tooLarge)` | `ZSTD_error_tableLog_tooLarge` | `phase_b_legacy` | [x] |
| 626 | `HUF_readDTableX6` (L2222) | `return ERROR(tableLog_tooLarge)` | `ZSTD_error_tableLog_tooLarge` | `phase_b_legacy` | [x] |
| 627 | `HUF_readDTableX6` (L2226) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_b_legacy` | [x] |
| 628 | `HUF_decompress4X6_usingDTable` (L2378) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 629 | `HUF_decompress4X6_usingDTable` (L2416) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 630 | `HUF_decompress4X6_usingDTable` (L2451) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 631 | `HUF_decompress4X6_usingDTable` (L2452) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 632 | `HUF_decompress4X6_usingDTable` (L2453) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 633 | `HUF_decompress4X6_usingDTable` (L2464) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 634 | `HUF_decompress4X6` (L2479) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 635 | `HUF_decompress` (L2526) | `return ERROR(dstSize_tooSmall)` | `ZSTD_error_dstSize_tooSmall` | `phase_b_legacy` | [x] |
| 636 | `HUF_decompress` (L2527) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 637 | `ZSTD_getcBlockSize` (L2762) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 638 | `ZSTD_copyUncompressedBlock` (L2777) | `return ERROR(dstSize_tooSmall)` | `ZSTD_error_dstSize_tooSmall` | `phase_b_legacy` | [x] |
| 639 | `ZSTD_decompressLiterals` (L2795) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 640 | `ZSTD_decompressLiterals` (L2796) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 641 | `ZSTD_decompressLiterals` (L2798) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 642 | `ZSTD_decodeLiteralsBlock` (L2814) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 643 | `ZSTD_decodeLiteralsBlock` (L2833) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 644 | `ZSTD_decodeLiteralsBlock` (L2834) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 645 | `ZSTD_decodeLiteralsBlock` (L2849) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 646 | `ZSTD_decodeSeqHeaders` (L2871) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 647 | `ZSTD_decodeSeqHeaders` (L2895) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 648 | `ZSTD_decodeSeqHeaders` (L2914) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_b_legacy` | [x] |
| 649 | `ZSTD_decodeSeqHeaders` (L2915) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 650 | `ZSTD_decodeSeqHeaders` (L2924) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 651 | `ZSTD_decodeSeqHeaders` (L2933) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_b_legacy` | [x] |
| 652 | `ZSTD_decodeSeqHeaders` (L2934) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 653 | `ZSTD_decodeSeqHeaders` (L2943) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 654 | `ZSTD_decodeSeqHeaders` (L2951) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_b_legacy` | [x] |
| 655 | `ZSTD_decodeSeqHeaders` (L2952) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 656 | `ZSTD_execSequence` (L3058) | `return ERROR(dstSize_tooSmall)` | `ZSTD_error_dstSize_tooSmall` | `phase_b_legacy` | [x] |
| 657 | `ZSTD_execSequence` (L3059) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 658 | `ZSTD_execSequence` (L3061) | `return ERROR(dstSize_tooSmall)` | `ZSTD_error_dstSize_tooSmall` | `phase_b_legacy` | [x] |
| 659 | `ZSTD_execSequence` (L3062) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 660 | `ZSTD_execSequence` (L3064) | `return ERROR(dstSize_tooSmall)` | `ZSTD_error_dstSize_tooSmall` | `phase_b_legacy` | [x] |
| 661 | `ZSTD_execSequence` (L3065) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 662 | `ZSTD_execSequence` (L3077) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 663 | `ZSTD_execSequence` (L3078) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 664 | `ZSTD_execSequence` (L3079) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 665 | `ZSTD_decompressSequences` (L3156) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 666 | `ZSTD_decompressSequences` (L3172) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 667 | `ZSTD_decompressSequences` (L3173) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 668 | `ZSTD_decompressSequences` (L3178) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 669 | `ZSTD_decompressSequences` (L3179) | `return ERROR(dstSize_tooSmall)` | `ZSTD_error_dstSize_tooSmall` | `phase_b_legacy` | [x] |
| 670 | `ZSTD_decompressDCtx` (L3221) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 671 | `ZSTD_decompressDCtx` (L3223) | `return ERROR(prefix_unknown)` | `ZSTD_error_prefix_unknown` | `phase_b_legacy` | [x] |
| 672 | `ZSTD_decompressDCtx` (L3235) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 673 | `ZSTD_decompressDCtx` (L3246) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_b_legacy` | [x] |
| 674 | `ZSTD_decompressDCtx` (L3250) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 675 | `ZSTD_decompressDCtx` (L3253) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_b_legacy` | [x] |
| 676 | `ZSTD_createDCtx` (L3344) | `return NULL;` | `NULL` | `phase_b_legacy` | [x] |
| 677 | `ZSTD_decompressContinue` (L3363) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 678 | `ZSTD_decompressContinue` (L3372) | `return ERROR(prefix_unknown)` | `ZSTD_error_prefix_unknown` | `phase_b_legacy` | [x] |
| 679 | `ZSTD_decompressContinue` (L3411) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_b_legacy` | [x] |
| 680 | `ZSTD_decompressContinue` (L3417) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_b_legacy` | [x] |

### `legacy/zstd_v03.c`

| # | function | trigger (the exact invalid input/condition) | expected C result | covering test | [x] |
|---|----------|---------------------------------------------|-------------------|---------------|-----|
| 681 | `BIT_initDStream` (L327) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 682 | `BIT_initDStream` (L336) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_b_legacy` | [x] |
| 683 | `BIT_initDStream` (L362) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_b_legacy` | [x] |
| 684 | `FSE_tableStep` (L1051) | `return ERROR(maxSymbolValue_tooLarge)` | `ZSTD_error_maxSymbolValue_tooLarge` | `phase_b_legacy` | [x] |
| 685 | `FSE_tableStep` (L1052) | `return ERROR(tableLog_tooLarge)` | `ZSTD_error_tableLog_tooLarge` | `phase_b_legacy` | [x] |
| 686 | `FSE_tableStep` (L1082) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_b_legacy` | [x] |
| 687 | `FSE_readNCount` (L1131) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 688 | `FSE_readNCount` (L1134) | `return ERROR(tableLog_tooLarge)` | `ZSTD_error_tableLog_tooLarge` | `phase_b_legacy` | [x] |
| 689 | `FSE_readNCount` (L1169) | `return ERROR(maxSymbolValue_tooSmall)` | `ZSTD_error_maxSymbolValue_tooSmall` | `phase_b_legacy` | [x] |
| 690 | `FSE_readNCount` (L1221) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_b_legacy` | [x] |
| 691 | `FSE_readNCount` (L1225) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 692 | `FSE_buildDTable_raw` (L1261) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_b_legacy` | [x] |
| 693 | `FSE_decompress_usingDTable_generic` (L1340) | `return ERROR(dstSize_tooSmall)` | `ZSTD_error_dstSize_tooSmall` | `phase_b_legacy` | [x] |
| 694 | `FSE_decompress_usingDTable_generic` (L1342) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 695 | `FSE_decompress` (L1369) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 696 | `FSE_decompress` (L1374) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 697 | `HUF_readStats` (L1488) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 698 | `HUF_readStats` (L1505) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 699 | `HUF_readStats` (L1506) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 700 | `HUF_readStats` (L1517) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 701 | `HUF_readStats` (L1527) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 702 | `HUF_readStats` (L1531) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 703 | `HUF_readStats` (L1535) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 704 | `HUF_readStats` (L1541) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 705 | `HUF_readStats` (L1547) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 706 | `HUF_readDTableX2` (L1580) | `return ERROR(tableLog_tooLarge)` | `ZSTD_error_tableLog_tooLarge` | `phase_b_legacy` | [x] |
| 707 | `HUF_decompress4X2_usingDTable` (L1657) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 708 | `HUF_decompress4X2_usingDTable` (L1693) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 709 | `HUF_decompress4X2_usingDTable` (L1728) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 710 | `HUF_decompress4X2_usingDTable` (L1729) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 711 | `HUF_decompress4X2_usingDTable` (L1730) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 712 | `HUF_decompress4X2_usingDTable` (L1741) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 713 | `HUF_decompress4X2` (L1757) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 714 | `HUF_readDTableX4` (L1878) | `return ERROR(tableLog_tooLarge)` | `ZSTD_error_tableLog_tooLarge` | `phase_b_legacy` | [x] |
| 715 | `HUF_readDTableX4` (L1885) | `return ERROR(tableLog_tooLarge)` | `ZSTD_error_tableLog_tooLarge` | `phase_b_legacy` | [x] |
| 716 | `HUF_readDTableX4` (L1889) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_b_legacy` | [x] |
| 717 | `HUF_decompress4X4_usingDTable` (L2019) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 718 | `HUF_decompress4X4_usingDTable` (L2055) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 719 | `HUF_decompress4X4_usingDTable` (L2090) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 720 | `HUF_decompress4X4_usingDTable` (L2091) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 721 | `HUF_decompress4X4_usingDTable` (L2092) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 722 | `HUF_decompress4X4_usingDTable` (L2103) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 723 | `HUF_decompress4X4` (L2118) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 724 | `HUF_decompress` (L2165) | `return ERROR(dstSize_tooSmall)` | `ZSTD_error_dstSize_tooSmall` | `phase_b_legacy` | [x] |
| 725 | `HUF_decompress` (L2166) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 726 | `ZSTD_getcBlockSize` (L2402) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 727 | `ZSTD_copyUncompressedBlock` (L2417) | `return ERROR(dstSize_tooSmall)` | `ZSTD_error_dstSize_tooSmall` | `phase_b_legacy` | [x] |
| 728 | `ZSTD_decompressLiterals` (L2435) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 729 | `ZSTD_decompressLiterals` (L2436) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 730 | `ZSTD_decompressLiterals` (L2438) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 731 | `ZSTD_decodeLiteralsBlock` (L2454) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 732 | `ZSTD_decodeLiteralsBlock` (L2473) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 733 | `ZSTD_decodeLiteralsBlock` (L2474) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 734 | `ZSTD_decodeLiteralsBlock` (L2489) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 735 | `ZSTD_decodeSeqHeaders` (L2511) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 736 | `ZSTD_decodeSeqHeaders` (L2535) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 737 | `ZSTD_decodeSeqHeaders` (L2554) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_b_legacy` | [x] |
| 738 | `ZSTD_decodeSeqHeaders` (L2555) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 739 | `ZSTD_decodeSeqHeaders` (L2564) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 740 | `ZSTD_decodeSeqHeaders` (L2573) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_b_legacy` | [x] |
| 741 | `ZSTD_decodeSeqHeaders` (L2574) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 742 | `ZSTD_decodeSeqHeaders` (L2583) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 743 | `ZSTD_decodeSeqHeaders` (L2591) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_b_legacy` | [x] |
| 744 | `ZSTD_decodeSeqHeaders` (L2592) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 745 | `ZSTD_execSequence` (L2698) | `return ERROR(dstSize_tooSmall)` | `ZSTD_error_dstSize_tooSmall` | `phase_b_legacy` | [x] |
| 746 | `ZSTD_execSequence` (L2699) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 747 | `ZSTD_execSequence` (L2701) | `return ERROR(dstSize_tooSmall)` | `ZSTD_error_dstSize_tooSmall` | `phase_b_legacy` | [x] |
| 748 | `ZSTD_execSequence` (L2702) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 749 | `ZSTD_execSequence` (L2704) | `return ERROR(dstSize_tooSmall)` | `ZSTD_error_dstSize_tooSmall` | `phase_b_legacy` | [x] |
| 750 | `ZSTD_execSequence` (L2705) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 751 | `ZSTD_execSequence` (L2716) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 752 | `ZSTD_execSequence` (L2717) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 753 | `ZSTD_execSequence` (L2718) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 754 | `ZSTD_decompressSequences` (L2795) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 755 | `ZSTD_decompressSequences` (L2811) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 756 | `ZSTD_decompressSequences` (L2812) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 757 | `ZSTD_decompressSequences` (L2817) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 758 | `ZSTD_decompressSequences` (L2818) | `return ERROR(dstSize_tooSmall)` | `ZSTD_error_dstSize_tooSmall` | `phase_b_legacy` | [x] |
| 759 | `ZSTD_decompressDCtx` (L2860) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 760 | `ZSTD_decompressDCtx` (L2862) | `return ERROR(prefix_unknown)` | `ZSTD_error_prefix_unknown` | `phase_b_legacy` | [x] |
| 761 | `ZSTD_decompressDCtx` (L2874) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 762 | `ZSTD_decompressDCtx` (L2885) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_b_legacy` | [x] |
| 763 | `ZSTD_decompressDCtx` (L2889) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 764 | `ZSTD_decompressDCtx` (L2892) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_b_legacy` | [x] |
| 765 | `ZSTD_createDCtx` (L2984) | `return NULL;` | `NULL` | `phase_b_legacy` | [x] |
| 766 | `ZSTD_decompressContinue` (L3003) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 767 | `ZSTD_decompressContinue` (L3012) | `return ERROR(prefix_unknown)` | `ZSTD_error_prefix_unknown` | `phase_b_legacy` | [x] |
| 768 | `ZSTD_decompressContinue` (L3051) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_b_legacy` | [x] |
| 769 | `ZSTD_decompressContinue` (L3057) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_b_legacy` | [x] |

### `legacy/zstd_v04.c`

| # | function | trigger (the exact invalid input/condition) | expected C result | covering test | [x] |
|---|----------|---------------------------------------------|-------------------|---------------|-----|
| 770 | `BIT_initDStream` (L603) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 771 | `BIT_initDStream` (L612) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_b_legacy` | [x] |
| 772 | `BIT_initDStream` (L632) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_b_legacy` | [x] |
| 773 | `FSE_buildDTable` (L1033) | `return ERROR(maxSymbolValue_tooLarge)` | `ZSTD_error_maxSymbolValue_tooLarge` | `phase_b_legacy` | [x] |
| 774 | `FSE_buildDTable` (L1034) | `return ERROR(tableLog_tooLarge)` | `ZSTD_error_tableLog_tooLarge` | `phase_b_legacy` | [x] |
| 775 | `FSE_buildDTable` (L1065) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_b_legacy` | [x] |
| 776 | `FSE_readNCount` (L1114) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 777 | `FSE_readNCount` (L1117) | `return ERROR(tableLog_tooLarge)` | `ZSTD_error_tableLog_tooLarge` | `phase_b_legacy` | [x] |
| 778 | `FSE_readNCount` (L1152) | `return ERROR(maxSymbolValue_tooSmall)` | `ZSTD_error_maxSymbolValue_tooSmall` | `phase_b_legacy` | [x] |
| 779 | `FSE_readNCount` (L1204) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_b_legacy` | [x] |
| 780 | `FSE_readNCount` (L1208) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 781 | `FSE_buildDTable_raw` (L1246) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_b_legacy` | [x] |
| 782 | `FSE_decompress_usingDTable_generic` (L1325) | `return ERROR(dstSize_tooSmall)` | `ZSTD_error_dstSize_tooSmall` | `phase_b_legacy` | [x] |
| 783 | `FSE_decompress_usingDTable_generic` (L1327) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 784 | `FSE_decompress` (L1357) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 785 | `FSE_decompress` (L1362) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 786 | `HUF_readStats` (L1647) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 787 | `HUF_readStats` (L1664) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 788 | `HUF_readStats` (L1665) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 789 | `HUF_readStats` (L1676) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 790 | `HUF_readStats` (L1686) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 791 | `HUF_readStats` (L1690) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 792 | `HUF_readStats` (L1694) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 793 | `HUF_readStats` (L1700) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 794 | `HUF_readStats` (L1706) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 795 | `HUF_readDTableX2` (L1738) | `return ERROR(tableLog_tooLarge)` | `ZSTD_error_tableLog_tooLarge` | `phase_b_legacy` | [x] |
| 796 | `HUF_decompress4X2_usingDTable` (L1815) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 797 | `HUF_decompress4X2_usingDTable` (L1850) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 798 | `HUF_decompress4X2_usingDTable` (L1885) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 799 | `HUF_decompress4X2_usingDTable` (L1886) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 800 | `HUF_decompress4X2_usingDTable` (L1887) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 801 | `HUF_decompress4X2_usingDTable` (L1898) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 802 | `HUF_decompress4X2` (L1914) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 803 | `HUF_readDTableX4` (L2034) | `return ERROR(tableLog_tooLarge)` | `ZSTD_error_tableLog_tooLarge` | `phase_b_legacy` | [x] |
| 804 | `HUF_readDTableX4` (L2041) | `return ERROR(tableLog_tooLarge)` | `ZSTD_error_tableLog_tooLarge` | `phase_b_legacy` | [x] |
| 805 | `HUF_readDTableX4` (L2045) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_b_legacy` | [x] |
| 806 | `HUF_decompress4X4_usingDTable` (L2173) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 807 | `HUF_decompress4X4_usingDTable` (L2208) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 808 | `HUF_decompress4X4_usingDTable` (L2243) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 809 | `HUF_decompress4X4_usingDTable` (L2244) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 810 | `HUF_decompress4X4_usingDTable` (L2245) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 811 | `HUF_decompress4X4_usingDTable` (L2256) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 812 | `HUF_decompress4X4` (L2271) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 813 | `HUF_decompress` (L2318) | `return ERROR(dstSize_tooSmall)` | `ZSTD_error_dstSize_tooSmall` | `phase_b_legacy` | [x] |
| 814 | `HUF_decompress` (L2319) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 815 | `ZSTD_createDCtx` (L2472) | `return NULL;` | `NULL` | `phase_b_legacy` | [x] |
| 816 | `ZSTD_decodeFrameHeader_Part1` (L2494) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 817 | `ZSTD_decodeFrameHeader_Part1` (L2496) | `return ERROR(prefix_unknown)` | `ZSTD_error_prefix_unknown` | `phase_b_legacy` | [x] |
| 818 | `ZSTD_getFrameParams` (L2507) | `return ERROR(prefix_unknown)` | `ZSTD_error_prefix_unknown` | `phase_b_legacy` | [x] |
| 819 | `ZSTD_getFrameParams` (L2510) | `return ERROR(frameParameter_unsupported)` | `ZSTD_error_frameParameter_unsupported` | `phase_b_legacy` | [x] |
| 820 | `ZSTD_decodeFrameHeader_Part2` (L2521) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 821 | `ZSTD_decodeFrameHeader_Part2` (L2523) | `return ERROR(frameParameter_unsupported)` | `ZSTD_error_frameParameter_unsupported` | `phase_b_legacy` | [x] |
| 822 | `ZSTD_getcBlockSize` (L2534) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 823 | `ZSTD_copyRawBlock` (L2549) | `return ERROR(dstSize_tooSmall)` | `ZSTD_error_dstSize_tooSmall` | `phase_b_legacy` | [x] |
| 824 | `ZSTD_decompressLiterals` (L2567) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 825 | `ZSTD_decompressLiterals` (L2568) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 826 | `ZSTD_decompressLiterals` (L2570) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 827 | `ZSTD_decodeLiteralsBlock` (L2585) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 828 | `ZSTD_decodeLiteralsBlock` (L2604) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 829 | `ZSTD_decodeLiteralsBlock` (L2605) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 830 | `ZSTD_decodeLiteralsBlock` (L2619) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 831 | `ZSTD_decodeLiteralsBlock` (L2626) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 832 | `ZSTD_decodeSeqHeaders` (L2643) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 833 | `ZSTD_decodeSeqHeaders` (L2667) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 834 | `ZSTD_decodeSeqHeaders` (L2686) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_b_legacy` | [x] |
| 835 | `ZSTD_decodeSeqHeaders` (L2687) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 836 | `ZSTD_decodeSeqHeaders` (L2696) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 837 | `ZSTD_decodeSeqHeaders` (L2705) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_b_legacy` | [x] |
| 838 | `ZSTD_decodeSeqHeaders` (L2706) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 839 | `ZSTD_decodeSeqHeaders` (L2715) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 840 | `ZSTD_decodeSeqHeaders` (L2723) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_b_legacy` | [x] |
| 841 | `ZSTD_decodeSeqHeaders` (L2724) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 842 | `ZSTD_execSequence` (L2826) | `return ERROR(dstSize_tooSmall)` | `ZSTD_error_dstSize_tooSmall` | `phase_b_legacy` | [x] |
| 843 | `ZSTD_execSequence` (L2827) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 844 | `ZSTD_execSequence` (L2829) | `return ERROR(dstSize_tooSmall)` | `ZSTD_error_dstSize_tooSmall` | `phase_b_legacy` | [x] |
| 845 | `ZSTD_execSequence` (L2831) | `return ERROR(dstSize_tooSmall)` | `ZSTD_error_dstSize_tooSmall` | `phase_b_legacy` | [x] |
| 846 | `ZSTD_execSequence` (L2832) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 847 | `ZSTD_execSequence` (L2844) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 848 | `ZSTD_decompressSequences` (L2940) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 849 | `ZSTD_decompressSequences` (L2956) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 850 | `ZSTD_decompressSequences` (L2961) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 851 | `ZSTD_decompressSequences` (L2962) | `return ERROR(dstSize_tooSmall)` | `ZSTD_error_dstSize_tooSmall` | `phase_b_legacy` | [x] |
| 852 | `ZSTD_decompressBlock_internal` (L2994) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 853 | `ZSTD_decompress_usingDict` (L3036) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 854 | `ZSTD_decompress_usingDict` (L3039) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 855 | `ZSTD_decompress_usingDict` (L3054) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 856 | `ZSTD_decompress_usingDict` (L3065) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_b_legacy` | [x] |
| 857 | `ZSTD_decompress_usingDict` (L3069) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 858 | `ZSTD_decompress_usingDict` (L3072) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_b_legacy` | [x] |
| 859 | `ZSTD_decompressContinue` (L3149) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 860 | `ZSTD_decompressContinue` (L3157) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 861 | `ZSTD_decompressContinue` (L3161) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_b_legacy` | [x] |
| 862 | `ZSTD_decompressContinue` (L3203) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_b_legacy` | [x] |
| 863 | `ZSTD_decompressContinue` (L3209) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_b_legacy` | [x] |
| 864 | `ZSTD_decompressContinue` (L3218) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_b_legacy` | [x] |
| 865 | `ZBUFF_createDCtx` (L3327) | `return NULL;` | `NULL` | `phase_b_legacy` | [x] |
| 866 | `ZBUFF_decompressContinue` (L3391) | `return ERROR(init_missing)` | `ZSTD_error_init_missing` | `phase_b_legacy` | [x] |
| 867 | `ZBUFF_decompressContinue` (L3433) | `return ERROR(memory_allocation)` | `ZSTD_error_memory_allocation` | `phase_b_legacy` | [x] |
| 868 | `ZBUFF_decompressContinue` (L3439) | `return ERROR(memory_allocation)` | `ZSTD_error_memory_allocation` | `phase_b_legacy` | [x] |
| 869 | `ZBUFF_decompressContinue` (L3484) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 870 | `ZBUFF_decompressContinue` (L3519) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_b_legacy` | [x] |
| 871 | `ZSTDv04_decompress` (L3560) | `return ERROR(memory_allocation)` | `ZSTD_error_memory_allocation` | `phase_b_legacy` | [x] |

### `legacy/zstd_v05.c`

| # | function | trigger (the exact invalid input/condition) | expected C result | covering test | [x] |
|---|----------|---------------------------------------------|-------------------|---------------|-----|
| 872 | `BITv05_initDStream` (L736) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 873 | `BITv05_initDStream` (L744) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_b_legacy` | [x] |
| 874 | `BITv05_initDStream` (L762) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_b_legacy` | [x] |
| 875 | `FSEv05_buildDTable` (L1173) | `return ERROR(maxSymbolValue_tooLarge)` | `ZSTD_error_maxSymbolValue_tooLarge` | `phase_b_legacy` | [x] |
| 876 | `FSEv05_buildDTable` (L1174) | `return ERROR(tableLog_tooLarge)` | `ZSTD_error_tableLog_tooLarge` | `phase_b_legacy` | [x] |
| 877 | `FSEv05_buildDTable` (L1197) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_b_legacy` | [x] |
| 878 | `FSEv05_readNCount` (L1244) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 879 | `FSEv05_readNCount` (L1247) | `return ERROR(tableLog_tooLarge)` | `ZSTD_error_tableLog_tooLarge` | `phase_b_legacy` | [x] |
| 880 | `FSEv05_readNCount` (L1274) | `return ERROR(maxSymbolValue_tooSmall)` | `ZSTD_error_maxSymbolValue_tooSmall` | `phase_b_legacy` | [x] |
| 881 | `FSEv05_readNCount` (L1315) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_b_legacy` | [x] |
| 882 | `FSEv05_readNCount` (L1319) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 883 | `FSEv05_buildDTable_raw` (L1358) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_b_legacy` | [x] |
| 884 | `FSEv05_decompress_usingDTable_generic` (L1434) | `return ERROR(dstSize_tooSmall)` | `ZSTD_error_dstSize_tooSmall` | `phase_b_legacy` | [x] |
| 885 | `FSEv05_decompress_usingDTable_generic` (L1436) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 886 | `FSEv05_decompress` (L1464) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 887 | `FSEv05_decompress` (L1469) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 888 | `HUFv05_readStats` (L1753) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 889 | `HUFv05_readStats` (L1767) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 890 | `HUFv05_readStats` (L1768) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 891 | `HUFv05_readStats` (L1775) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 892 | `HUFv05_readStats` (L1784) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 893 | `HUFv05_readStats` (L1788) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 894 | `HUFv05_readStats` (L1792) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 895 | `HUFv05_readStats` (L1798) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 896 | `HUFv05_readStats` (L1804) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 897 | `HUFv05_readDTableX2` (L1836) | `return ERROR(tableLog_tooLarge)` | `ZSTD_error_tableLog_tooLarge` | `phase_b_legacy` | [x] |
| 898 | `HUFv05_decompress1X2_usingDTable` (L1916) | `return ERROR(dstSize_tooSmall)` | `ZSTD_error_dstSize_tooSmall` | `phase_b_legacy` | [x] |
| 899 | `HUFv05_decompress1X2_usingDTable` (L1923) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 900 | `HUFv05_decompress1X2` (L1936) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 901 | `HUFv05_decompress4X2_usingDTable` (L1950) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 902 | `HUFv05_decompress4X2_usingDTable` (L1984) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 903 | `HUFv05_decompress4X2_usingDTable` (L2017) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 904 | `HUFv05_decompress4X2_usingDTable` (L2018) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 905 | `HUFv05_decompress4X2_usingDTable` (L2019) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 906 | `HUFv05_decompress4X2_usingDTable` (L2030) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 907 | `HUFv05_decompress4X2` (L2046) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 908 | `HUFv05_readDTableX4` (L2160) | `return ERROR(tableLog_tooLarge)` | `ZSTD_error_tableLog_tooLarge` | `phase_b_legacy` | [x] |
| 909 | `HUFv05_readDTableX4` (L2167) | `return ERROR(tableLog_tooLarge)` | `ZSTD_error_tableLog_tooLarge` | `phase_b_legacy` | [x] |
| 910 | `HUFv05_decompress1X4_usingDTable` (L2306) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 911 | `HUFv05_decompress1X4` (L2319) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 912 | `HUFv05_decompress4X4_usingDTable` (L2331) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 913 | `HUFv05_decompress4X4_usingDTable` (L2366) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 914 | `HUFv05_decompress4X4_usingDTable` (L2400) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 915 | `HUFv05_decompress4X4_usingDTable` (L2401) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 916 | `HUFv05_decompress4X4_usingDTable` (L2402) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 917 | `HUFv05_decompress4X4_usingDTable` (L2413) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 918 | `HUFv05_decompress4X4` (L2428) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 919 | `HUFv05_decompress` (L2475) | `return ERROR(dstSize_tooSmall)` | `ZSTD_error_dstSize_tooSmall` | `phase_b_legacy` | [x] |
| 920 | `HUFv05_decompress` (L2476) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 921 | `ZSTDv05_createDCtx` (L2632) | `return NULL;` | `NULL` | `phase_b_legacy` | [x] |
| 922 | `ZSTDv05_decodeFrameHeader_Part1` (L2743) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 923 | `ZSTDv05_decodeFrameHeader_Part1` (L2745) | `return ERROR(prefix_unknown)` | `ZSTD_error_prefix_unknown` | `phase_b_legacy` | [x] |
| 924 | `ZSTDv05_getFrameParams` (L2756) | `return ERROR(prefix_unknown)` | `ZSTD_error_prefix_unknown` | `phase_b_legacy` | [x] |
| 925 | `ZSTDv05_getFrameParams` (L2759) | `return ERROR(frameParameter_unsupported)` | `ZSTD_error_frameParameter_unsupported` | `phase_b_legacy` | [x] |
| 926 | `ZSTDv05_decodeFrameHeader_Part2` (L2771) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 927 | `ZSTDv05_decodeFrameHeader_Part2` (L2773) | `return ERROR(frameParameter_unsupported)` | `ZSTD_error_frameParameter_unsupported` | `phase_b_legacy` | [x] |
| 928 | `ZSTDv05_getcBlockSize` (L2785) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 929 | `ZSTDv05_copyRawBlock` (L2801) | `return ERROR(dstSize_tooSmall)` | `ZSTD_error_dstSize_tooSmall` | `phase_b_legacy` | [x] |
| 930 | `ZSTDv05_copyRawBlock` (L2802) | `return ERROR(dstSize_tooSmall)` | `ZSTD_error_dstSize_tooSmall` | `phase_b_legacy` | [x] |
| 931 | `ZSTDv05_decodeLiteralsBlock` (L2816) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 932 | `ZSTDv05_decodeLiteralsBlock` (L2824) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 933 | `ZSTDv05_decodeLiteralsBlock` (L2847) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 934 | `ZSTDv05_decodeLiteralsBlock` (L2848) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 935 | `ZSTDv05_decodeLiteralsBlock` (L2853) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 936 | `ZSTDv05_decodeLiteralsBlock` (L2866) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 937 | `ZSTDv05_decodeLiteralsBlock` (L2868) | `return ERROR(dictionary_corrupted)` | `ZSTD_error_dictionary_corrupted` | `phase_b_legacy` | [x] |
| 938 | `ZSTDv05_decodeLiteralsBlock` (L2874) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 939 | `ZSTDv05_decodeLiteralsBlock` (L2877) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 940 | `ZSTDv05_decodeLiteralsBlock` (L2903) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 941 | `ZSTDv05_decodeLiteralsBlock` (L2930) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 942 | `ZSTDv05_decodeLiteralsBlock` (L2933) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 943 | `ZSTDv05_decodeLiteralsBlock` (L2940) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 944 | `ZSTDv05_decodeSeqHeaders` (L2958) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 945 | `ZSTDv05_decodeSeqHeaders` (L2964) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 946 | `ZSTDv05_decodeSeqHeaders` (L2968) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 947 | `ZSTDv05_decodeSeqHeaders` (L2973) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 948 | `ZSTDv05_decodeSeqHeaders` (L2978) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 949 | `ZSTDv05_decodeSeqHeaders` (L2988) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 950 | `ZSTDv05_decodeSeqHeaders` (L3007) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 951 | `ZSTDv05_decodeSeqHeaders` (L3013) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_b_legacy` | [x] |
| 952 | `ZSTDv05_decodeSeqHeaders` (L3014) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 953 | `ZSTDv05_decodeSeqHeaders` (L3023) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 954 | `ZSTDv05_decodeSeqHeaders` (L3031) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 955 | `ZSTDv05_decodeSeqHeaders` (L3037) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_b_legacy` | [x] |
| 956 | `ZSTDv05_decodeSeqHeaders` (L3038) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 957 | `ZSTDv05_decodeSeqHeaders` (L3047) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 958 | `ZSTDv05_decodeSeqHeaders` (L3055) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 959 | `ZSTDv05_decodeSeqHeaders` (L3061) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_b_legacy` | [x] |
| 960 | `ZSTDv05_decodeSeqHeaders` (L3062) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 961 | `ZSTDv05_execSequence` (L3188) | `return ERROR(dstSize_tooSmall)` | `ZSTD_error_dstSize_tooSmall` | `phase_b_legacy` | [x] |
| 962 | `ZSTDv05_execSequence` (L3189) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 963 | `ZSTDv05_execSequence` (L3191) | `return ERROR(dstSize_tooSmall)` | `ZSTD_error_dstSize_tooSmall` | `phase_b_legacy` | [x] |
| 964 | `ZSTDv05_execSequence` (L3193) | `return ERROR(dstSize_tooSmall)` | `ZSTD_error_dstSize_tooSmall` | `phase_b_legacy` | [x] |
| 965 | `ZSTDv05_execSequence` (L3194) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 966 | `ZSTDv05_execSequence` (L3205) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 967 | `ZSTDv05_decompressSequences` (L3296) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 968 | `ZSTDv05_decompressSequences` (L3311) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 969 | `ZSTDv05_decompressSequences` (L3317) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 970 | `ZSTDv05_decompressSequences` (L3318) | `return ERROR(dstSize_tooSmall)` | `ZSTD_error_dstSize_tooSmall` | `phase_b_legacy` | [x] |
| 971 | `ZSTDv05_decompressBlock_internal` (L3347) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 972 | `ZSTDv05_decompress_continueDCtx` (L3385) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 973 | `ZSTDv05_decompress_continueDCtx` (L3388) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 974 | `ZSTDv05_decompress_continueDCtx` (L3403) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 975 | `ZSTDv05_decompress_continueDCtx` (L3414) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_b_legacy` | [x] |
| 976 | `ZSTDv05_decompress_continueDCtx` (L3418) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 977 | `ZSTDv05_decompress_continueDCtx` (L3421) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_b_legacy` | [x] |
| 978 | `ZSTDv05_decompress` (L3466) | `return ERROR(memory_allocation)` | `ZSTD_error_memory_allocation` | `phase_b_legacy` | [x] |
| 979 | `ZSTDv05_decompressContinue` (L3540) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 980 | `ZSTDv05_decompressContinue` (L3548) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 981 | `ZSTDv05_decompressContinue` (L3552) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_b_legacy` | [x] |
| 982 | `ZSTDv05_decompressContinue` (L3593) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_b_legacy` | [x] |
| 983 | `ZSTDv05_decompressContinue` (L3599) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_b_legacy` | [x] |
| 984 | `ZSTDv05_decompressContinue` (L3608) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_b_legacy` | [x] |
| 985 | `ZSTDv05_loadEntropy` (L3632) | `return ERROR(dictionary_corrupted)` | `ZSTD_error_dictionary_corrupted` | `phase_b_legacy` | [x] |
| 986 | `ZSTDv05_loadEntropy` (L3637) | `return ERROR(dictionary_corrupted)` | `ZSTD_error_dictionary_corrupted` | `phase_b_legacy` | [x] |
| 987 | `ZSTDv05_loadEntropy` (L3638) | `return ERROR(dictionary_corrupted)` | `ZSTD_error_dictionary_corrupted` | `phase_b_legacy` | [x] |
| 988 | `ZSTDv05_loadEntropy` (L3640) | `return ERROR(dictionary_corrupted)` | `ZSTD_error_dictionary_corrupted` | `phase_b_legacy` | [x] |
| 989 | `ZSTDv05_loadEntropy` (L3645) | `return ERROR(dictionary_corrupted)` | `ZSTD_error_dictionary_corrupted` | `phase_b_legacy` | [x] |
| 990 | `ZSTDv05_loadEntropy` (L3646) | `return ERROR(dictionary_corrupted)` | `ZSTD_error_dictionary_corrupted` | `phase_b_legacy` | [x] |
| 991 | `ZSTDv05_loadEntropy` (L3648) | `return ERROR(dictionary_corrupted)` | `ZSTD_error_dictionary_corrupted` | `phase_b_legacy` | [x] |
| 992 | `ZSTDv05_loadEntropy` (L3653) | `return ERROR(dictionary_corrupted)` | `ZSTD_error_dictionary_corrupted` | `phase_b_legacy` | [x] |
| 993 | `ZSTDv05_loadEntropy` (L3654) | `return ERROR(dictionary_corrupted)` | `ZSTD_error_dictionary_corrupted` | `phase_b_legacy` | [x] |
| 994 | `ZSTDv05_loadEntropy` (L3656) | `return ERROR(dictionary_corrupted)` | `ZSTD_error_dictionary_corrupted` | `phase_b_legacy` | [x] |
| 995 | `ZSTDv05_decompress_insertDictionary` (L3675) | `return ERROR(dictionary_corrupted)` | `ZSTD_error_dictionary_corrupted` | `phase_b_legacy` | [x] |
| 996 | `ZSTDv05_decompressBegin_usingDict` (L3694) | `return ERROR(dictionary_corrupted)` | `ZSTD_error_dictionary_corrupted` | `phase_b_legacy` | [x] |
| 997 | `ZBUFFv05_createDCtx` (L3807) | `return NULL;` | `NULL` | `phase_b_legacy` | [x] |
| 998 | `ZBUFFv05_decompressContinue` (L3856) | `return ERROR(init_missing)` | `ZSTD_error_init_missing` | `phase_b_legacy` | [x] |
| 999 | `ZBUFFv05_decompressContinue` (L3902) | `return ERROR(memory_allocation)` | `ZSTD_error_memory_allocation` | `phase_b_legacy` | [x] |
| 1000 | `ZBUFFv05_decompressContinue` (L3908) | `return ERROR(memory_allocation)` | `ZSTD_error_memory_allocation` | `phase_b_legacy` | [x] |
| 1001 | `ZBUFFv05_decompressContinue` (L3949) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1002 | `ZBUFFv05_decompressContinue` (L3983) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_b_legacy` | [x] |

### `legacy/zstd_v06.c`

| # | function | trigger (the exact invalid input/condition) | expected C result | covering test | [x] |
|---|----------|---------------------------------------------|-------------------|---------------|-----|
| 1003 | `BITv06_initDStream` (L835) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 1004 | `BITv06_initDStream` (L842) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_b_legacy` | [x] |
| 1005 | `BITv06_initDStream` (L859) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_b_legacy` | [x] |
| 1006 | `FSEv06_readNCount` (L1221) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 1007 | `FSEv06_readNCount` (L1224) | `return ERROR(tableLog_tooLarge)` | `ZSTD_error_tableLog_tooLarge` | `phase_b_legacy` | [x] |
| 1008 | `FSEv06_readNCount` (L1251) | `return ERROR(maxSymbolValue_tooSmall)` | `ZSTD_error_maxSymbolValue_tooSmall` | `phase_b_legacy` | [x] |
| 1009 | `FSEv06_readNCount` (L1291) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_b_legacy` | [x] |
| 1010 | `FSEv06_readNCount` (L1295) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 1011 | `FSEv06_buildDTable` (L1413) | `return ERROR(maxSymbolValue_tooLarge)` | `ZSTD_error_maxSymbolValue_tooLarge` | `phase_b_legacy` | [x] |
| 1012 | `FSEv06_buildDTable` (L1414) | `return ERROR(tableLog_tooLarge)` | `ZSTD_error_tableLog_tooLarge` | `phase_b_legacy` | [x] |
| 1013 | `FSEv06_buildDTable` (L1445) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_b_legacy` | [x] |
| 1014 | `FSEv06_buildDTable_raw` (L1497) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_b_legacy` | [x] |
| 1015 | `FSEv06_decompress_usingDTable_generic` (L1557) | `return ERROR(dstSize_tooSmall)` | `ZSTD_error_dstSize_tooSmall` | `phase_b_legacy` | [x] |
| 1016 | `FSEv06_decompress_usingDTable_generic` (L1566) | `return ERROR(dstSize_tooSmall)` | `ZSTD_error_dstSize_tooSmall` | `phase_b_legacy` | [x] |
| 1017 | `FSEv06_decompress` (L1602) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 1018 | `FSEv06_decompress` (L1607) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 1019 | `HUFv06_readStats` (L1807) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 1020 | `HUFv06_readStats` (L1821) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 1021 | `HUFv06_readStats` (L1822) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1022 | `HUFv06_readStats` (L1830) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 1023 | `HUFv06_readStats` (L1839) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1024 | `HUFv06_readStats` (L1843) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1025 | `HUFv06_readStats` (L1847) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1026 | `HUFv06_readStats` (L1854) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1027 | `HUFv06_readStats` (L1860) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1028 | `HUFv06_readDTableX2` (L1967) | `return ERROR(tableLog_tooLarge)` | `ZSTD_error_tableLog_tooLarge` | `phase_b_legacy` | [x] |
| 1029 | `HUFv06_decompress1X2_usingDTable` (L2054) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1030 | `HUFv06_decompress1X2` (L2066) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 1031 | `HUFv06_decompress4X2_usingDTable` (L2080) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1032 | `HUFv06_decompress4X2_usingDTable` (L2114) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1033 | `HUFv06_decompress4X2_usingDTable` (L2147) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1034 | `HUFv06_decompress4X2_usingDTable` (L2148) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1035 | `HUFv06_decompress4X2_usingDTable` (L2149) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1036 | `HUFv06_decompress4X2_usingDTable` (L2160) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1037 | `HUFv06_decompress4X2` (L2175) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 1038 | `HUFv06_readDTableX4` (L2286) | `return ERROR(tableLog_tooLarge)` | `ZSTD_error_tableLog_tooLarge` | `phase_b_legacy` | [x] |
| 1039 | `HUFv06_readDTableX4` (L2293) | `return ERROR(tableLog_tooLarge)` | `ZSTD_error_tableLog_tooLarge` | `phase_b_legacy` | [x] |
| 1040 | `HUFv06_decompress1X4_usingDTable` (L2430) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1041 | `HUFv06_decompress1X4` (L2443) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 1042 | `HUFv06_decompress4X4_usingDTable` (L2455) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1043 | `HUFv06_decompress4X4_usingDTable` (L2489) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1044 | `HUFv06_decompress4X4_usingDTable` (L2523) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1045 | `HUFv06_decompress4X4_usingDTable` (L2524) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1046 | `HUFv06_decompress4X4_usingDTable` (L2525) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1047 | `HUFv06_decompress4X4_usingDTable` (L2536) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1048 | `HUFv06_decompress4X4` (L2551) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 1049 | `HUFv06_decompress` (L2595) | `return ERROR(dstSize_tooSmall)` | `ZSTD_error_dstSize_tooSmall` | `phase_b_legacy` | [x] |
| 1050 | `HUFv06_decompress` (L2596) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1051 | `ZSTDv06_createDCtx` (L2789) | `return NULL;` | `NULL` | `phase_b_legacy` | [x] |
| 1052 | `ZSTDv06_frameHeaderSize` (L2913) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 1053 | `ZSTDv06_getFrameParams` (L2929) | `return ERROR(prefix_unknown)` | `ZSTD_error_prefix_unknown` | `phase_b_legacy` | [x] |
| 1054 | `ZSTDv06_getFrameParams` (L2938) | `return ERROR(frameParameter_unsupported)` | `ZSTD_error_frameParameter_unsupported` | `phase_b_legacy` | [x] |
| 1055 | `ZSTDv06_decodeFrameHeader` (L2957) | `return ERROR(frameParameter_unsupported)` | `ZSTD_error_frameParameter_unsupported` | `phase_b_legacy` | [x] |
| 1056 | `ZSTDv06_getcBlockSize` (L2975) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 1057 | `ZSTDv06_copyRawBlock` (L2989) | `return ERROR(dstSize_tooSmall)` | `ZSTD_error_dstSize_tooSmall` | `phase_b_legacy` | [x] |
| 1058 | `ZSTDv06_copyRawBlock` (L2990) | `return ERROR(dstSize_tooSmall)` | `ZSTD_error_dstSize_tooSmall` | `phase_b_legacy` | [x] |
| 1059 | `ZSTDv06_decodeLiteralsBlock` (L3004) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1060 | `ZSTDv06_decodeLiteralsBlock` (L3011) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1061 | `ZSTDv06_decodeLiteralsBlock` (L3034) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1062 | `ZSTDv06_decodeLiteralsBlock` (L3035) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1063 | `ZSTDv06_decodeLiteralsBlock` (L3040) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1064 | `ZSTDv06_decodeLiteralsBlock` (L3051) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1065 | `ZSTDv06_decodeLiteralsBlock` (L3053) | `return ERROR(dictionary_corrupted)` | `ZSTD_error_dictionary_corrupted` | `phase_b_legacy` | [x] |
| 1066 | `ZSTDv06_decodeLiteralsBlock` (L3059) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1067 | `ZSTDv06_decodeLiteralsBlock` (L3062) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1068 | `ZSTDv06_decodeLiteralsBlock` (L3087) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1069 | `ZSTDv06_decodeLiteralsBlock` (L3113) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1070 | `ZSTDv06_decodeLiteralsBlock` (L3116) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1071 | `ZSTDv06_decodeLiteralsBlock` (L3123) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1072 | `ZSTDv06_buildSeqTable` (L3139) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 1073 | `ZSTDv06_buildSeqTable` (L3140) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1074 | `ZSTDv06_buildSeqTable` (L3147) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1075 | `ZSTDv06_buildSeqTable` (L3154) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1076 | `ZSTDv06_buildSeqTable` (L3155) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1077 | `ZSTDv06_decodeSeqHeaders` (L3171) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 1078 | `ZSTDv06_decodeSeqHeaders` (L3178) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 1079 | `ZSTDv06_decodeSeqHeaders` (L3181) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 1080 | `ZSTDv06_decodeSeqHeaders` (L3189) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 1081 | `ZSTDv06_decodeSeqHeaders` (L3197) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1082 | `ZSTDv06_decodeSeqHeaders` (L3201) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1083 | `ZSTDv06_decodeSeqHeaders` (L3205) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1084 | `ZSTDv06_execSequence` (L3320) | `return ERROR(dstSize_tooSmall)` | `ZSTD_error_dstSize_tooSmall` | `phase_b_legacy` | [x] |
| 1085 | `ZSTDv06_execSequence` (L3321) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1086 | `ZSTDv06_execSequence` (L3323) | `return ERROR(dstSize_tooSmall)` | `ZSTD_error_dstSize_tooSmall` | `phase_b_legacy` | [x] |
| 1087 | `ZSTDv06_execSequence` (L3325) | `return ERROR(dstSize_tooSmall)` | `ZSTD_error_dstSize_tooSmall` | `phase_b_legacy` | [x] |
| 1088 | `ZSTDv06_execSequence` (L3326) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1089 | `ZSTDv06_execSequence` (L3336) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1090 | `ZSTDv06_decompressSequences` (L3423) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1091 | `ZSTDv06_decompressSequences` (L3447) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1092 | `ZSTDv06_decompressSequences` (L3452) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1093 | `ZSTDv06_decompressSequences` (L3453) | `return ERROR(dstSize_tooSmall)` | `ZSTD_error_dstSize_tooSmall` | `phase_b_legacy` | [x] |
| 1094 | `ZSTDv06_decompressBlock_internal` (L3481) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 1095 | `ZSTDv06_decompressFrame` (L3517) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 1096 | `ZSTDv06_decompressFrame` (L3522) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 1097 | `ZSTDv06_decompressFrame` (L3523) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1098 | `ZSTDv06_decompressFrame` (L3535) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 1099 | `ZSTDv06_decompressFrame` (L3546) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_b_legacy` | [x] |
| 1100 | `ZSTDv06_decompressFrame` (L3550) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 1101 | `ZSTDv06_decompressFrame` (L3553) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_b_legacy` | [x] |
| 1102 | `ZSTDv06_decompress` (L3599) | `return ERROR(memory_allocation)` | `ZSTD_error_memory_allocation` | `phase_b_legacy` | [x] |
| 1103 | `ZSTDv06_decompressContinue` (L3678) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 1104 | `ZSTDv06_decompressContinue` (L3685) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 1105 | `ZSTDv06_decompressContinue` (L3730) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_b_legacy` | [x] |
| 1106 | `ZSTDv06_decompressContinue` (L3736) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_b_legacy` | [x] |
| 1107 | `ZSTDv06_decompressContinue` (L3745) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_b_legacy` | [x] |
| 1108 | `ZSTDv06_loadEntropy` (L3763) | `return ERROR(dictionary_corrupted)` | `ZSTD_error_dictionary_corrupted` | `phase_b_legacy` | [x] |
| 1109 | `ZSTDv06_loadEntropy` (L3770) | `return ERROR(dictionary_corrupted)` | `ZSTD_error_dictionary_corrupted` | `phase_b_legacy` | [x] |
| 1110 | `ZSTDv06_loadEntropy` (L3771) | `return ERROR(dictionary_corrupted)` | `ZSTD_error_dictionary_corrupted` | `phase_b_legacy` | [x] |
| 1111 | `ZSTDv06_loadEntropy` (L3773) | `return ERROR(dictionary_corrupted)` | `ZSTD_error_dictionary_corrupted` | `phase_b_legacy` | [x] |
| 1112 | `ZSTDv06_loadEntropy` (L3781) | `return ERROR(dictionary_corrupted)` | `ZSTD_error_dictionary_corrupted` | `phase_b_legacy` | [x] |
| 1113 | `ZSTDv06_loadEntropy` (L3782) | `return ERROR(dictionary_corrupted)` | `ZSTD_error_dictionary_corrupted` | `phase_b_legacy` | [x] |
| 1114 | `ZSTDv06_loadEntropy` (L3784) | `return ERROR(dictionary_corrupted)` | `ZSTD_error_dictionary_corrupted` | `phase_b_legacy` | [x] |
| 1115 | `ZSTDv06_loadEntropy` (L3792) | `return ERROR(dictionary_corrupted)` | `ZSTD_error_dictionary_corrupted` | `phase_b_legacy` | [x] |
| 1116 | `ZSTDv06_loadEntropy` (L3793) | `return ERROR(dictionary_corrupted)` | `ZSTD_error_dictionary_corrupted` | `phase_b_legacy` | [x] |
| 1117 | `ZSTDv06_loadEntropy` (L3795) | `return ERROR(dictionary_corrupted)` | `ZSTD_error_dictionary_corrupted` | `phase_b_legacy` | [x] |
| 1118 | `ZSTDv06_decompress_insertDictionary` (L3815) | `return ERROR(dictionary_corrupted)` | `ZSTD_error_dictionary_corrupted` | `phase_b_legacy` | [x] |
| 1119 | `ZSTDv06_decompressBegin_usingDict` (L3833) | `return ERROR(dictionary_corrupted)` | `ZSTD_error_dictionary_corrupted` | `phase_b_legacy` | [x] |
| 1120 | `ZBUFFv06_createDCtx` (L3919) | `return NULL;` | `NULL` | `phase_b_legacy` | [x] |
| 1121 | `ZBUFFv06_createDCtx` (L3924) | `return NULL;` | `NULL` | `phase_b_legacy` | [x] |
| 1122 | `ZBUFFv06_decompressContinue` (L3985) | `return ERROR(init_missing)` | `ZSTD_error_init_missing` | `phase_b_legacy` | [x] |
| 1123 | `ZBUFFv06_decompressContinue` (L4020) | `return ERROR(memory_allocation)` | `ZSTD_error_memory_allocation` | `phase_b_legacy` | [x] |
| 1124 | `ZBUFFv06_decompressContinue` (L4027) | `return ERROR(memory_allocation)` | `ZSTD_error_memory_allocation` | `phase_b_legacy` | [x] |
| 1125 | `ZBUFFv06_decompressContinue` (L4057) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1126 | `ZBUFFv06_decompressContinue` (L4091) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_b_legacy` | [x] |

### `legacy/zstd_v07.c`

| # | function | trigger (the exact invalid input/condition) | expected C result | covering test | [x] |
|---|----------|---------------------------------------------|-------------------|---------------|-----|
| 1127 | `BITv07_initDStream` (L504) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 1128 | `BITv07_initDStream` (L512) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_b_legacy` | [x] |
| 1129 | `BITv07_initDStream` (L529) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_b_legacy` | [x] |
| 1130 | `FSEv07_readNCount` (L1166) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 1131 | `FSEv07_readNCount` (L1169) | `return ERROR(tableLog_tooLarge)` | `ZSTD_error_tableLog_tooLarge` | `phase_b_legacy` | [x] |
| 1132 | `FSEv07_readNCount` (L1196) | `return ERROR(maxSymbolValue_tooSmall)` | `ZSTD_error_maxSymbolValue_tooSmall` | `phase_b_legacy` | [x] |
| 1133 | `FSEv07_readNCount` (L1236) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_b_legacy` | [x] |
| 1134 | `FSEv07_readNCount` (L1240) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 1135 | `HUFv07_readStats` (L1260) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 1136 | `HUFv07_readStats` (L1274) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 1137 | `HUFv07_readStats` (L1275) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1138 | `HUFv07_readStats` (L1283) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 1139 | `HUFv07_readStats` (L1292) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1140 | `HUFv07_readStats` (L1296) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1141 | `HUFv07_readStats` (L1300) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1142 | `HUFv07_readStats` (L1307) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1143 | `HUFv07_readStats` (L1313) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1144 | `FSEv07_buildDTable` (L1434) | `return ERROR(maxSymbolValue_tooLarge)` | `ZSTD_error_maxSymbolValue_tooLarge` | `phase_b_legacy` | [x] |
| 1145 | `FSEv07_buildDTable` (L1435) | `return ERROR(tableLog_tooLarge)` | `ZSTD_error_tableLog_tooLarge` | `phase_b_legacy` | [x] |
| 1146 | `FSEv07_buildDTable` (L1466) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_b_legacy` | [x] |
| 1147 | `FSEv07_buildDTable_raw` (L1518) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_b_legacy` | [x] |
| 1148 | `FSEv07_decompress_usingDTable_generic` (L1578) | `return ERROR(dstSize_tooSmall)` | `ZSTD_error_dstSize_tooSmall` | `phase_b_legacy` | [x] |
| 1149 | `FSEv07_decompress_usingDTable_generic` (L1587) | `return ERROR(dstSize_tooSmall)` | `ZSTD_error_dstSize_tooSmall` | `phase_b_legacy` | [x] |
| 1150 | `FSEv07_decompress` (L1623) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 1151 | `FSEv07_decompress` (L1628) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 1152 | `HUFv07_readDTableX2` (L1739) | `return ERROR(tableLog_tooLarge)` | `ZSTD_error_tableLog_tooLarge` | `phase_b_legacy` | [x] |
| 1153 | `HUFv07_decompress1X2_usingDTable_internal` (L1831) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1154 | `HUFv07_decompress1X2_usingDTable` (L1842) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_b_legacy` | [x] |
| 1155 | `HUFv07_decompress1X2_DCtx` (L1852) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 1156 | `HUFv07_decompress4X2_usingDTable_internal` (L1871) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1157 | `HUFv07_decompress4X2_usingDTable_internal` (L1904) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1158 | `HUFv07_decompress4X2_usingDTable_internal` (L1937) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1159 | `HUFv07_decompress4X2_usingDTable_internal` (L1938) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1160 | `HUFv07_decompress4X2_usingDTable_internal` (L1939) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1161 | `HUFv07_decompress4X2_usingDTable_internal` (L1950) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1162 | `HUFv07_decompress4X2_usingDTable` (L1964) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_b_legacy` | [x] |
| 1163 | `HUFv07_decompress4X2_DCtx` (L1975) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 1164 | `HUFv07_readDTableX4` (L2095) | `return ERROR(tableLog_tooLarge)` | `ZSTD_error_tableLog_tooLarge` | `phase_b_legacy` | [x] |
| 1165 | `HUFv07_readDTableX4` (L2102) | `return ERROR(tableLog_tooLarge)` | `ZSTD_error_tableLog_tooLarge` | `phase_b_legacy` | [x] |
| 1166 | `HUFv07_decompress1X4_usingDTable_internal` (L2242) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1167 | `HUFv07_decompress1X4_usingDTable` (L2254) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_b_legacy` | [x] |
| 1168 | `HUFv07_decompress1X4_DCtx` (L2264) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 1169 | `HUFv07_decompress4X4_usingDTable_internal` (L2281) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1170 | `HUFv07_decompress4X4_usingDTable_internal` (L2314) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1171 | `HUFv07_decompress4X4_usingDTable_internal` (L2348) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1172 | `HUFv07_decompress4X4_usingDTable_internal` (L2349) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1173 | `HUFv07_decompress4X4_usingDTable_internal` (L2350) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1174 | `HUFv07_decompress4X4_usingDTable_internal` (L2361) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1175 | `HUFv07_decompress4X4_usingDTable` (L2375) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_b_legacy` | [x] |
| 1176 | `HUFv07_decompress4X4_DCtx` (L2386) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 1177 | `HUFv07_decompress` (L2469) | `return ERROR(dstSize_tooSmall)` | `ZSTD_error_dstSize_tooSmall` | `phase_b_legacy` | [x] |
| 1178 | `HUFv07_decompress` (L2470) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1179 | `HUFv07_decompress4X_DCtx` (L2485) | `return ERROR(dstSize_tooSmall)` | `ZSTD_error_dstSize_tooSmall` | `phase_b_legacy` | [x] |
| 1180 | `HUFv07_decompress4X_DCtx` (L2486) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1181 | `HUFv07_decompress4X_hufOnly` (L2499) | `return ERROR(dstSize_tooSmall)` | `ZSTD_error_dstSize_tooSmall` | `phase_b_legacy` | [x] |
| 1182 | `HUFv07_decompress4X_hufOnly` (L2500) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1183 | `HUFv07_decompress1X_DCtx` (L2511) | `return ERROR(dstSize_tooSmall)` | `ZSTD_error_dstSize_tooSmall` | `phase_b_legacy` | [x] |
| 1184 | `HUFv07_decompress1X_DCtx` (L2512) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1185 | `ZSTDv07_createDCtx_advanced` (L2930) | `return NULL;` | `NULL` | `phase_b_legacy` | [x] |
| 1186 | `ZSTDv07_createDCtx_advanced` (L2933) | `return NULL;` | `NULL` | `phase_b_legacy` | [x] |
| 1187 | `ZSTDv07_frameHeaderSize` (L3079) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 1188 | `ZSTDv07_getFrameParams` (L3108) | `return ERROR(prefix_unknown)` | `ZSTD_error_prefix_unknown` | `phase_b_legacy` | [x] |
| 1189 | `ZSTDv07_getFrameParams` (L3126) | `return ERROR(frameParameter_unsupported)` | `ZSTD_error_frameParameter_unsupported` | `phase_b_legacy` | [x] |
| 1190 | `ZSTDv07_getFrameParams` (L3131) | `return ERROR(frameParameter_unsupported)` | `ZSTD_error_frameParameter_unsupported` | `phase_b_legacy` | [x] |
| 1191 | `ZSTDv07_getFrameParams` (L3154) | `return ERROR(frameParameter_unsupported)` | `ZSTD_error_frameParameter_unsupported` | `phase_b_legacy` | [x] |
| 1192 | `ZSTDv07_decodeFrameHeader` (L3186) | `return ERROR(dictionary_wrong)` | `ZSTD_error_dictionary_wrong` | `phase_b_legacy` | [x] |
| 1193 | `ZSTDv07_getcBlockSize` (L3205) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 1194 | `ZSTDv07_copyRawBlock` (L3219) | `return ERROR(dstSize_tooSmall)` | `ZSTD_error_dstSize_tooSmall` | `phase_b_legacy` | [x] |
| 1195 | `ZSTDv07_decodeLiteralsBlock` (L3234) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1196 | `ZSTDv07_decodeLiteralsBlock` (L3241) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1197 | `ZSTDv07_decodeLiteralsBlock` (L3264) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1198 | `ZSTDv07_decodeLiteralsBlock` (L3265) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1199 | `ZSTDv07_decodeLiteralsBlock` (L3270) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1200 | `ZSTDv07_decodeLiteralsBlock` (L3282) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1201 | `ZSTDv07_decodeLiteralsBlock` (L3284) | `return ERROR(dictionary_corrupted)` | `ZSTD_error_dictionary_corrupted` | `phase_b_legacy` | [x] |
| 1202 | `ZSTDv07_decodeLiteralsBlock` (L3290) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1203 | `ZSTDv07_decodeLiteralsBlock` (L3293) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1204 | `ZSTDv07_decodeLiteralsBlock` (L3318) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1205 | `ZSTDv07_decodeLiteralsBlock` (L3344) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1206 | `ZSTDv07_decodeLiteralsBlock` (L3347) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1207 | `ZSTDv07_decodeLiteralsBlock` (L3354) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1208 | `ZSTDv07_buildSeqTable` (L3370) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 1209 | `ZSTDv07_buildSeqTable` (L3371) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1210 | `ZSTDv07_buildSeqTable` (L3378) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1211 | `ZSTDv07_buildSeqTable` (L3385) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1212 | `ZSTDv07_buildSeqTable` (L3386) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1213 | `ZSTDv07_decodeSeqHeaders` (L3402) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 1214 | `ZSTDv07_decodeSeqHeaders` (L3409) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 1215 | `ZSTDv07_decodeSeqHeaders` (L3412) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 1216 | `ZSTDv07_decodeSeqHeaders` (L3420) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 1217 | `ZSTDv07_decodeSeqHeaders` (L3428) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1218 | `ZSTDv07_decodeSeqHeaders` (L3432) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1219 | `ZSTDv07_decodeSeqHeaders` (L3436) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1220 | `ZSTDv07_execSequence` (L3548) | `return ERROR(dstSize_tooSmall)` | `ZSTD_error_dstSize_tooSmall` | `phase_b_legacy` | [x] |
| 1221 | `ZSTDv07_execSequence` (L3549) | `return ERROR(dstSize_tooSmall)` | `ZSTD_error_dstSize_tooSmall` | `phase_b_legacy` | [x] |
| 1222 | `ZSTDv07_execSequence` (L3551) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1223 | `ZSTDv07_execSequence` (L3561) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1224 | `ZSTDv07_decompressSequences` (L3644) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1225 | `ZSTDv07_decompressSequences` (L3658) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1226 | `ZSTDv07_decompressSequences` (L3665) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1227 | `ZSTDv07_decompressSequences` (L3666) | `return ERROR(dstSize_tooSmall)` | `ZSTD_error_dstSize_tooSmall` | `phase_b_legacy` | [x] |
| 1228 | `ZSTDv07_decompressBlock_internal` (L3694) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 1229 | `ZSTDv07_generateNxBytes` (L3730) | `return ERROR(dstSize_tooSmall)` | `ZSTD_error_dstSize_tooSmall` | `phase_b_legacy` | [x] |
| 1230 | `ZSTDv07_decompressFrame` (L3752) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 1231 | `ZSTDv07_decompressFrame` (L3757) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 1232 | `ZSTDv07_decompressFrame` (L3758) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1233 | `ZSTDv07_decompressFrame` (L3771) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 1234 | `ZSTDv07_decompressFrame` (L3786) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 1235 | `ZSTDv07_decompressFrame` (L3790) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_b_legacy` | [x] |
| 1236 | `ZSTDv07_decompress` (L3842) | `return ERROR(memory_allocation)` | `ZSTD_error_memory_allocation` | `phase_b_legacy` | [x] |
| 1237 | `ZSTDv07_decompressContinue` (L3936) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 1238 | `ZSTDv07_decompressContinue` (L3942) | `return ERROR(srcSize_wrong)` | `ZSTD_error_srcSize_wrong` | `phase_b_legacy` | [x] |
| 1239 | `ZSTDv07_decompressContinue` (L3978) | `return ERROR(checksum_wrong)` | `ZSTD_error_checksum_wrong` | `phase_b_legacy` | [x] |
| 1240 | `ZSTDv07_decompressContinue` (L4000) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_b_legacy` | [x] |
| 1241 | `ZSTDv07_decompressContinue` (L4006) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_b_legacy` | [x] |
| 1242 | `ZSTDv07_decompressContinue` (L4027) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_b_legacy` | [x] |
| 1243 | `ZSTDv07_loadEntropy` (L4047) | `return ERROR(dictionary_corrupted)` | `ZSTD_error_dictionary_corrupted` | `phase_b_legacy` | [x] |
| 1244 | `ZSTDv07_loadEntropy` (L4054) | `return ERROR(dictionary_corrupted)` | `ZSTD_error_dictionary_corrupted` | `phase_b_legacy` | [x] |
| 1245 | `ZSTDv07_loadEntropy` (L4055) | `return ERROR(dictionary_corrupted)` | `ZSTD_error_dictionary_corrupted` | `phase_b_legacy` | [x] |
| 1246 | `ZSTDv07_loadEntropy` (L4057) | `return ERROR(dictionary_corrupted)` | `ZSTD_error_dictionary_corrupted` | `phase_b_legacy` | [x] |
| 1247 | `ZSTDv07_loadEntropy` (L4064) | `return ERROR(dictionary_corrupted)` | `ZSTD_error_dictionary_corrupted` | `phase_b_legacy` | [x] |
| 1248 | `ZSTDv07_loadEntropy` (L4065) | `return ERROR(dictionary_corrupted)` | `ZSTD_error_dictionary_corrupted` | `phase_b_legacy` | [x] |
| 1249 | `ZSTDv07_loadEntropy` (L4067) | `return ERROR(dictionary_corrupted)` | `ZSTD_error_dictionary_corrupted` | `phase_b_legacy` | [x] |
| 1250 | `ZSTDv07_loadEntropy` (L4074) | `return ERROR(dictionary_corrupted)` | `ZSTD_error_dictionary_corrupted` | `phase_b_legacy` | [x] |
| 1251 | `ZSTDv07_loadEntropy` (L4075) | `return ERROR(dictionary_corrupted)` | `ZSTD_error_dictionary_corrupted` | `phase_b_legacy` | [x] |
| 1252 | `ZSTDv07_loadEntropy` (L4077) | `return ERROR(dictionary_corrupted)` | `ZSTD_error_dictionary_corrupted` | `phase_b_legacy` | [x] |
| 1253 | `ZSTDv07_loadEntropy` (L4081) | `return ERROR(dictionary_corrupted)` | `ZSTD_error_dictionary_corrupted` | `phase_b_legacy` | [x] |
| 1254 | `ZSTDv07_loadEntropy` (L4082) | `return ERROR(dictionary_corrupted)` | `ZSTD_error_dictionary_corrupted` | `phase_b_legacy` | [x] |
| 1255 | `ZSTDv07_loadEntropy` (L4083) | `return ERROR(dictionary_corrupted)` | `ZSTD_error_dictionary_corrupted` | `phase_b_legacy` | [x] |
| 1256 | `ZSTDv07_loadEntropy` (L4084) | `return ERROR(dictionary_corrupted)` | `ZSTD_error_dictionary_corrupted` | `phase_b_legacy` | [x] |
| 1257 | `ZSTDv07_decompress_insertDictionary` (L4104) | `return ERROR(dictionary_corrupted)` | `ZSTD_error_dictionary_corrupted` | `phase_b_legacy` | [x] |
| 1258 | `ZSTDv07_decompressBegin_usingDict` (L4121) | `return ERROR(dictionary_corrupted)` | `ZSTD_error_dictionary_corrupted` | `phase_b_legacy` | [x] |
| 1259 | `ZSTDv07_createDDict_advanced` (L4140) | `return NULL;` | `NULL` | `phase_b_legacy` | [x] |
| 1260 | `ZSTDv07_createDDict_advanced` (L4150) | `return NULL;` | `NULL` | `phase_b_legacy` | [x] |
| 1261 | `ZSTDv07_createDDict_advanced` (L4159) | `return NULL;` | `NULL` | `phase_b_legacy` | [x] |
| 1262 | `ZBUFFv07_createDCtx_advanced` (L4293) | `return NULL;` | `NULL` | `phase_b_legacy` | [x] |
| 1263 | `ZBUFFv07_createDCtx_advanced` (L4296) | `return NULL;` | `NULL` | `phase_b_legacy` | [x] |
| 1264 | `ZBUFFv07_createDCtx_advanced` (L4300) | `return NULL;` | `NULL` | `phase_b_legacy` | [x] |
| 1265 | `ZBUFFv07_decompressContinue` (L4360) | `return ERROR(init_missing)` | `ZSTD_error_init_missing` | `phase_b_legacy` | [x] |
| 1266 | `ZBUFFv07_decompressContinue` (L4397) | `return ERROR(memory_allocation)` | `ZSTD_error_memory_allocation` | `phase_b_legacy` | [x] |
| 1267 | `ZBUFFv07_decompressContinue` (L4404) | `return ERROR(memory_allocation)` | `ZSTD_error_memory_allocation` | `phase_b_legacy` | [x] |
| 1268 | `ZBUFFv07_decompressContinue` (L4436) | `return ERROR(corruption_detected)` | `ZSTD_error_corruption_detected` | `phase_b_legacy` | [x] |
| 1269 | `ZBUFFv07_decompressContinue` (L4472) | `return ERROR(GENERIC)` | `ZSTD_error_GENERIC` | `phase_b_legacy` | [x] |

