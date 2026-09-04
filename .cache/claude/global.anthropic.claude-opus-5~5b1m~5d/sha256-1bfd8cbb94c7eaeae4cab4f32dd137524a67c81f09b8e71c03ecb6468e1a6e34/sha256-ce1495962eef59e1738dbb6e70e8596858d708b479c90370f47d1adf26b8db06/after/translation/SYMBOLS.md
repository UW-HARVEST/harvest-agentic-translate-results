# SYMBOLS.md - dynamic symbol parity, C `.so` vs Rust `.so`

Generated mechanically from:

```
nm -D --defined-only c_src/build/libzstd.so
nm -D --defined-only translation/target/release/libzstd.so
```

* C  exported (defined, dynamic): **615**
* Rust exported (defined, dynamic): **615**
* **Missing from Rust: 0**
* Extra in Rust (not in C): 0

## MISSING

_None._ Every symbol exported by the C `.so` is exported by the
Rust `.so` with the exact same name (including the macro-generated
`XXH_NAMESPACE=ZSTD_` names and the legacy `FSEv05_`/`HUFv06_`/`ZSTDv07_` renames).

## EXTRA in Rust

_None._

## Undefined (imported) symbols

Rust non-libc undefined symbols: _none_

C non-libc undefined symbols: `ZSTD_trace_compress_begin`, `ZSTD_trace_compress_end`,
`ZSTD_trace_decompress_begin`, `ZSTD_trace_decompress_end` (weak, never defined -> always NULL).
The Rust port hard-codes these hooks to `None` (see `src/zstd_trace.rs`), which is
behaviourally identical, so they do not appear as imports.

## Feature combinations (Phase D)

`translation/Cargo.toml` declares **no `[features]` table**, and
`grep -rn 'cfg(feature' src/` finds **0** hits, so the crate has exactly
**one** build configuration. The only `#[cfg(...)]` in the whole crate are
5 `target_arch = "x86_64"` guards (`src/zstd_internal.rs::ZSTD_cpuid`,
`src/compress/zstd_ldm.rs::PREFETCH_L1`,
`src/compress/zstd_lazy.rs` SSE2 row-hash helpers), which mirror the C's own
`#if defined(__x86_64__)` guards; the target is x86-64 little-endian per
`PORTING_GUIDE.md`.

`translation/run_all_features.sh` extracts the feature list from `Cargo.toml`
mechanically (never hard-coded), builds the cross-product of
`--no-default-features [--features ...]`, and for each combination runs
`cargo check`, `cargo build --release`, the `nm -D` symbol diff and the whole
`cargo test --release` suite. With no features declared it reports exactly one
combination (`default`).

## Full symbol list

### common/entropy (71 symbols)

| symbol | type | in C | in Rust |
|---|---|---|---|
| `ERR_getErrorString` | T | yes | yes |
| `FSE_NCountWriteBound` | T | yes | yes |
| `FSE_buildCTable_rle` | T | yes | yes |
| `FSE_buildCTable_wksp` | T | yes | yes |
| `FSE_buildDTable_wksp` | T | yes | yes |
| `FSE_compressBound` | T | yes | yes |
| `FSE_compress_usingCTable` | T | yes | yes |
| `FSE_decompress_wksp_bmi2` | T | yes | yes |
| `FSE_getErrorName` | T | yes | yes |
| `FSE_isError` | T | yes | yes |
| `FSE_normalizeCount` | T | yes | yes |
| `FSE_optimalTableLog` | T | yes | yes |
| `FSE_optimalTableLog_internal` | T | yes | yes |
| `FSE_readNCount` | T | yes | yes |
| `FSE_readNCount_bmi2` | T | yes | yes |
| `FSE_versionNumber` | T | yes | yes |
| `FSE_writeNCount` | T | yes | yes |
| `HIST_add` | T | yes | yes |
| `HIST_count` | T | yes | yes |
| `HIST_countFast` | T | yes | yes |
| `HIST_countFast_wksp` | T | yes | yes |
| `HIST_count_simple` | T | yes | yes |
| `HIST_count_wksp` | T | yes | yes |
| `HIST_isError` | T | yes | yes |
| `HUF_buildCTable_wksp` | T | yes | yes |
| `HUF_cardinality` | T | yes | yes |
| `HUF_compress1X_repeat` | T | yes | yes |
| `HUF_compress1X_usingCTable` | T | yes | yes |
| `HUF_compress4X_repeat` | T | yes | yes |
| `HUF_compress4X_usingCTable` | T | yes | yes |
| `HUF_compressBound` | T | yes | yes |
| `HUF_decompress1X1_DCtx_wksp` | T | yes | yes |
| `HUF_decompress1X2_DCtx_wksp` | T | yes | yes |
| `HUF_decompress1X_DCtx_wksp` | T | yes | yes |
| `HUF_decompress1X_usingDTable` | T | yes | yes |
| `HUF_decompress4X_hufOnly_wksp` | T | yes | yes |
| `HUF_decompress4X_usingDTable` | T | yes | yes |
| `HUF_estimateCompressedSize` | T | yes | yes |
| `HUF_getErrorName` | T | yes | yes |
| `HUF_getNbBitsFromCTable` | T | yes | yes |
| `HUF_isError` | T | yes | yes |
| `HUF_minTableLog` | T | yes | yes |
| `HUF_optimalTableLog` | T | yes | yes |
| `HUF_readCTable` | T | yes | yes |
| `HUF_readCTableHeader` | T | yes | yes |
| `HUF_readDTableX1_wksp` | T | yes | yes |
| `HUF_readDTableX2_wksp` | T | yes | yes |
| `HUF_readStats` | T | yes | yes |
| `HUF_readStats_wksp` | T | yes | yes |
| `HUF_selectDecoder` | T | yes | yes |
| `HUF_validateCTable` | T | yes | yes |
| `HUF_writeCTable_wksp` | T | yes | yes |
| `ZSTD_XXH32` | T | yes | yes |
| `ZSTD_XXH32_canonicalFromHash` | T | yes | yes |
| `ZSTD_XXH32_copyState` | T | yes | yes |
| `ZSTD_XXH32_createState` | T | yes | yes |
| `ZSTD_XXH32_digest` | T | yes | yes |
| `ZSTD_XXH32_freeState` | T | yes | yes |
| `ZSTD_XXH32_hashFromCanonical` | T | yes | yes |
| `ZSTD_XXH32_reset` | T | yes | yes |
| `ZSTD_XXH32_update` | T | yes | yes |
| `ZSTD_XXH64` | T | yes | yes |
| `ZSTD_XXH64_canonicalFromHash` | T | yes | yes |
| `ZSTD_XXH64_copyState` | T | yes | yes |
| `ZSTD_XXH64_createState` | T | yes | yes |
| `ZSTD_XXH64_digest` | T | yes | yes |
| `ZSTD_XXH64_freeState` | T | yes | yes |
| `ZSTD_XXH64_hashFromCanonical` | T | yes | yes |
| `ZSTD_XXH64_reset` | T | yes | yes |
| `ZSTD_XXH64_update` | T | yes | yes |
| `ZSTD_XXH_versionNumber` | T | yes | yes |

### deprecated (zbuff) (21 symbols)

| symbol | type | in C | in Rust |
|---|---|---|---|
| `ZBUFF_compressContinue` | T | yes | yes |
| `ZBUFF_compressEnd` | T | yes | yes |
| `ZBUFF_compressFlush` | T | yes | yes |
| `ZBUFF_compressInit` | T | yes | yes |
| `ZBUFF_compressInitDictionary` | T | yes | yes |
| `ZBUFF_compressInit_advanced` | T | yes | yes |
| `ZBUFF_createCCtx` | T | yes | yes |
| `ZBUFF_createCCtx_advanced` | T | yes | yes |
| `ZBUFF_createDCtx` | T | yes | yes |
| `ZBUFF_createDCtx_advanced` | T | yes | yes |
| `ZBUFF_decompressContinue` | T | yes | yes |
| `ZBUFF_decompressInit` | T | yes | yes |
| `ZBUFF_decompressInitDictionary` | T | yes | yes |
| `ZBUFF_freeCCtx` | T | yes | yes |
| `ZBUFF_freeDCtx` | T | yes | yes |
| `ZBUFF_getErrorName` | T | yes | yes |
| `ZBUFF_isError` | T | yes | yes |
| `ZBUFF_recommendedCInSize` | T | yes | yes |
| `ZBUFF_recommendedCOutSize` | T | yes | yes |
| `ZBUFF_recommendedDInSize` | T | yes | yes |
| `ZBUFF_recommendedDOutSize` | T | yes | yes |

### dictBuilder (27 symbols)

| symbol | type | in C | in Rust |
|---|---|---|---|
| `COVER_best_destroy` | T | yes | yes |
| `COVER_best_finish` | T | yes | yes |
| `COVER_best_init` | T | yes | yes |
| `COVER_best_start` | T | yes | yes |
| `COVER_best_wait` | T | yes | yes |
| `COVER_checkTotalCompressedSize` | T | yes | yes |
| `COVER_computeEpochs` | T | yes | yes |
| `COVER_dictSelectionError` | T | yes | yes |
| `COVER_dictSelectionFree` | T | yes | yes |
| `COVER_dictSelectionIsError` | T | yes | yes |
| `COVER_selectDict` | T | yes | yes |
| `COVER_sum` | T | yes | yes |
| `COVER_warnOnSmallCorpus` | T | yes | yes |
| `ZDICT_addEntropyTablesFromBuffer` | T | yes | yes |
| `ZDICT_finalizeDictionary` | T | yes | yes |
| `ZDICT_getDictHeaderSize` | T | yes | yes |
| `ZDICT_getDictID` | T | yes | yes |
| `ZDICT_getErrorName` | T | yes | yes |
| `ZDICT_isError` | T | yes | yes |
| `ZDICT_optimizeTrainFromBuffer_cover` | T | yes | yes |
| `ZDICT_optimizeTrainFromBuffer_fastCover` | T | yes | yes |
| `ZDICT_trainFromBuffer` | T | yes | yes |
| `ZDICT_trainFromBuffer_cover` | T | yes | yes |
| `ZDICT_trainFromBuffer_fastCover` | T | yes | yes |
| `ZDICT_trainFromBuffer_legacy` | T | yes | yes |
| `divbwt` | T | yes | yes |
| `divsufsort` | T | yes | yes |

### legacy (v0.1-v0.7) (206 symbols)

| symbol | type | in C | in Rust |
|---|---|---|---|
| `FSEv05_buildDTable` | T | yes | yes |
| `FSEv05_buildDTable_raw` | T | yes | yes |
| `FSEv05_buildDTable_rle` | T | yes | yes |
| `FSEv05_createDTable` | T | yes | yes |
| `FSEv05_decompress` | T | yes | yes |
| `FSEv05_decompress_usingDTable` | T | yes | yes |
| `FSEv05_freeDTable` | T | yes | yes |
| `FSEv05_getErrorName` | T | yes | yes |
| `FSEv05_isError` | T | yes | yes |
| `FSEv05_readNCount` | T | yes | yes |
| `FSEv06_buildDTable` | T | yes | yes |
| `FSEv06_buildDTable_raw` | T | yes | yes |
| `FSEv06_buildDTable_rle` | T | yes | yes |
| `FSEv06_createDTable` | T | yes | yes |
| `FSEv06_decompress` | T | yes | yes |
| `FSEv06_decompress_usingDTable` | T | yes | yes |
| `FSEv06_freeDTable` | T | yes | yes |
| `FSEv06_getErrorName` | T | yes | yes |
| `FSEv06_isError` | T | yes | yes |
| `FSEv06_readNCount` | T | yes | yes |
| `FSEv07_buildDTable` | T | yes | yes |
| `FSEv07_buildDTable_raw` | T | yes | yes |
| `FSEv07_buildDTable_rle` | T | yes | yes |
| `FSEv07_createDTable` | T | yes | yes |
| `FSEv07_decompress` | T | yes | yes |
| `FSEv07_decompress_usingDTable` | T | yes | yes |
| `FSEv07_freeDTable` | T | yes | yes |
| `FSEv07_getErrorName` | T | yes | yes |
| `FSEv07_isError` | T | yes | yes |
| `FSEv07_readNCount` | T | yes | yes |
| `HUFv05_decompress` | T | yes | yes |
| `HUFv05_decompress1X2` | T | yes | yes |
| `HUFv05_decompress1X2_usingDTable` | T | yes | yes |
| `HUFv05_decompress1X4` | T | yes | yes |
| `HUFv05_decompress1X4_usingDTable` | T | yes | yes |
| `HUFv05_decompress4X2` | T | yes | yes |
| `HUFv05_decompress4X2_usingDTable` | T | yes | yes |
| `HUFv05_decompress4X4` | T | yes | yes |
| `HUFv05_decompress4X4_usingDTable` | T | yes | yes |
| `HUFv05_getErrorName` | T | yes | yes |
| `HUFv05_isError` | T | yes | yes |
| `HUFv05_readDTableX2` | T | yes | yes |
| `HUFv05_readDTableX4` | T | yes | yes |
| `HUFv06_decompress` | T | yes | yes |
| `HUFv06_decompress1X2` | T | yes | yes |
| `HUFv06_decompress1X2_usingDTable` | T | yes | yes |
| `HUFv06_decompress1X4` | T | yes | yes |
| `HUFv06_decompress1X4_usingDTable` | T | yes | yes |
| `HUFv06_decompress4X2` | T | yes | yes |
| `HUFv06_decompress4X2_usingDTable` | T | yes | yes |
| `HUFv06_decompress4X4` | T | yes | yes |
| `HUFv06_decompress4X4_usingDTable` | T | yes | yes |
| `HUFv06_readDTableX2` | T | yes | yes |
| `HUFv06_readDTableX4` | T | yes | yes |
| `HUFv07_decompress` | T | yes | yes |
| `HUFv07_decompress1X2` | T | yes | yes |
| `HUFv07_decompress1X2_DCtx` | T | yes | yes |
| `HUFv07_decompress1X2_usingDTable` | T | yes | yes |
| `HUFv07_decompress1X4` | T | yes | yes |
| `HUFv07_decompress1X4_DCtx` | T | yes | yes |
| `HUFv07_decompress1X4_usingDTable` | T | yes | yes |
| `HUFv07_decompress1X_DCtx` | T | yes | yes |
| `HUFv07_decompress1X_usingDTable` | T | yes | yes |
| `HUFv07_decompress4X2` | T | yes | yes |
| `HUFv07_decompress4X2_DCtx` | T | yes | yes |
| `HUFv07_decompress4X2_usingDTable` | T | yes | yes |
| `HUFv07_decompress4X4` | T | yes | yes |
| `HUFv07_decompress4X4_DCtx` | T | yes | yes |
| `HUFv07_decompress4X4_usingDTable` | T | yes | yes |
| `HUFv07_decompress4X_DCtx` | T | yes | yes |
| `HUFv07_decompress4X_hufOnly` | T | yes | yes |
| `HUFv07_decompress4X_usingDTable` | T | yes | yes |
| `HUFv07_getErrorName` | T | yes | yes |
| `HUFv07_isError` | T | yes | yes |
| `HUFv07_readDTableX2` | T | yes | yes |
| `HUFv07_readDTableX4` | T | yes | yes |
| `HUFv07_readStats` | T | yes | yes |
| `HUFv07_selectDecoder` | T | yes | yes |
| `ZBUFFv04_createDCtx` | T | yes | yes |
| `ZBUFFv04_decompressContinue` | T | yes | yes |
| `ZBUFFv04_decompressInit` | T | yes | yes |
| `ZBUFFv04_decompressWithDictionary` | T | yes | yes |
| `ZBUFFv04_freeDCtx` | T | yes | yes |
| `ZBUFFv04_getErrorName` | T | yes | yes |
| `ZBUFFv04_isError` | T | yes | yes |
| `ZBUFFv04_recommendedDInSize` | T | yes | yes |
| `ZBUFFv04_recommendedDOutSize` | T | yes | yes |
| `ZBUFFv05_createDCtx` | T | yes | yes |
| `ZBUFFv05_decompressContinue` | T | yes | yes |
| `ZBUFFv05_decompressInit` | T | yes | yes |
| `ZBUFFv05_decompressInitDictionary` | T | yes | yes |
| `ZBUFFv05_freeDCtx` | T | yes | yes |
| `ZBUFFv05_getErrorName` | T | yes | yes |
| `ZBUFFv05_isError` | T | yes | yes |
| `ZBUFFv05_recommendedDInSize` | T | yes | yes |
| `ZBUFFv05_recommendedDOutSize` | T | yes | yes |
| `ZBUFFv06_createDCtx` | T | yes | yes |
| `ZBUFFv06_decompressContinue` | T | yes | yes |
| `ZBUFFv06_decompressInit` | T | yes | yes |
| `ZBUFFv06_decompressInitDictionary` | T | yes | yes |
| `ZBUFFv06_freeDCtx` | T | yes | yes |
| `ZBUFFv06_getErrorName` | T | yes | yes |
| `ZBUFFv06_isError` | T | yes | yes |
| `ZBUFFv06_recommendedDInSize` | T | yes | yes |
| `ZBUFFv06_recommendedDOutSize` | T | yes | yes |
| `ZBUFFv07_createDCtx` | T | yes | yes |
| `ZBUFFv07_createDCtx_advanced` | T | yes | yes |
| `ZBUFFv07_decompressContinue` | T | yes | yes |
| `ZBUFFv07_decompressInit` | T | yes | yes |
| `ZBUFFv07_decompressInitDictionary` | T | yes | yes |
| `ZBUFFv07_freeDCtx` | T | yes | yes |
| `ZBUFFv07_getErrorName` | T | yes | yes |
| `ZBUFFv07_isError` | T | yes | yes |
| `ZBUFFv07_recommendedDInSize` | T | yes | yes |
| `ZBUFFv07_recommendedDOutSize` | T | yes | yes |
| `ZSTDv01_createDCtx` | T | yes | yes |
| `ZSTDv01_decompress` | T | yes | yes |
| `ZSTDv01_decompressContinue` | T | yes | yes |
| `ZSTDv01_decompressDCtx` | T | yes | yes |
| `ZSTDv01_findFrameSizeInfoLegacy` | T | yes | yes |
| `ZSTDv01_freeDCtx` | T | yes | yes |
| `ZSTDv01_isError` | T | yes | yes |
| `ZSTDv01_nextSrcSizeToDecompress` | T | yes | yes |
| `ZSTDv01_resetDCtx` | T | yes | yes |
| `ZSTDv02_createDCtx` | T | yes | yes |
| `ZSTDv02_decompress` | T | yes | yes |
| `ZSTDv02_decompressContinue` | T | yes | yes |
| `ZSTDv02_findFrameSizeInfoLegacy` | T | yes | yes |
| `ZSTDv02_freeDCtx` | T | yes | yes |
| `ZSTDv02_isError` | T | yes | yes |
| `ZSTDv02_nextSrcSizeToDecompress` | T | yes | yes |
| `ZSTDv02_resetDCtx` | T | yes | yes |
| `ZSTDv03_createDCtx` | T | yes | yes |
| `ZSTDv03_decompress` | T | yes | yes |
| `ZSTDv03_decompressContinue` | T | yes | yes |
| `ZSTDv03_findFrameSizeInfoLegacy` | T | yes | yes |
| `ZSTDv03_freeDCtx` | T | yes | yes |
| `ZSTDv03_isError` | T | yes | yes |
| `ZSTDv03_nextSrcSizeToDecompress` | T | yes | yes |
| `ZSTDv03_resetDCtx` | T | yes | yes |
| `ZSTDv04_createDCtx` | T | yes | yes |
| `ZSTDv04_decompress` | T | yes | yes |
| `ZSTDv04_decompressContinue` | T | yes | yes |
| `ZSTDv04_decompressDCtx` | T | yes | yes |
| `ZSTDv04_findFrameSizeInfoLegacy` | T | yes | yes |
| `ZSTDv04_freeDCtx` | T | yes | yes |
| `ZSTDv04_nextSrcSizeToDecompress` | T | yes | yes |
| `ZSTDv04_resetDCtx` | T | yes | yes |
| `ZSTDv05_copyDCtx` | T | yes | yes |
| `ZSTDv05_createDCtx` | T | yes | yes |
| `ZSTDv05_decompress` | T | yes | yes |
| `ZSTDv05_decompressBegin` | T | yes | yes |
| `ZSTDv05_decompressBegin_usingDict` | T | yes | yes |
| `ZSTDv05_decompressBlock` | T | yes | yes |
| `ZSTDv05_decompressContinue` | T | yes | yes |
| `ZSTDv05_decompressDCtx` | T | yes | yes |
| `ZSTDv05_decompress_usingDict` | T | yes | yes |
| `ZSTDv05_decompress_usingPreparedDCtx` | T | yes | yes |
| `ZSTDv05_findFrameSizeInfoLegacy` | T | yes | yes |
| `ZSTDv05_freeDCtx` | T | yes | yes |
| `ZSTDv05_getErrorName` | T | yes | yes |
| `ZSTDv05_getFrameParams` | T | yes | yes |
| `ZSTDv05_isError` | T | yes | yes |
| `ZSTDv05_nextSrcSizeToDecompress` | T | yes | yes |
| `ZSTDv05_sizeofDCtx` | T | yes | yes |
| `ZSTDv06_copyDCtx` | T | yes | yes |
| `ZSTDv06_createDCtx` | T | yes | yes |
| `ZSTDv06_decompress` | T | yes | yes |
| `ZSTDv06_decompressBegin` | T | yes | yes |
| `ZSTDv06_decompressBegin_usingDict` | T | yes | yes |
| `ZSTDv06_decompressBlock` | T | yes | yes |
| `ZSTDv06_decompressContinue` | T | yes | yes |
| `ZSTDv06_decompressDCtx` | T | yes | yes |
| `ZSTDv06_decompress_usingDict` | T | yes | yes |
| `ZSTDv06_decompress_usingPreparedDCtx` | T | yes | yes |
| `ZSTDv06_findFrameSizeInfoLegacy` | T | yes | yes |
| `ZSTDv06_freeDCtx` | T | yes | yes |
| `ZSTDv06_getErrorName` | T | yes | yes |
| `ZSTDv06_getFrameParams` | T | yes | yes |
| `ZSTDv06_isError` | T | yes | yes |
| `ZSTDv06_nextSrcSizeToDecompress` | T | yes | yes |
| `ZSTDv06_sizeofDCtx` | T | yes | yes |
| `ZSTDv07_copyDCtx` | T | yes | yes |
| `ZSTDv07_createDCtx` | T | yes | yes |
| `ZSTDv07_createDCtx_advanced` | T | yes | yes |
| `ZSTDv07_createDDict` | T | yes | yes |
| `ZSTDv07_decompress` | T | yes | yes |
| `ZSTDv07_decompressBegin` | T | yes | yes |
| `ZSTDv07_decompressBegin_usingDict` | T | yes | yes |
| `ZSTDv07_decompressBlock` | T | yes | yes |
| `ZSTDv07_decompressContinue` | T | yes | yes |
| `ZSTDv07_decompressDCtx` | T | yes | yes |
| `ZSTDv07_decompress_usingDDict` | T | yes | yes |
| `ZSTDv07_decompress_usingDict` | T | yes | yes |
| `ZSTDv07_estimateDCtxSize` | T | yes | yes |
| `ZSTDv07_findFrameSizeInfoLegacy` | T | yes | yes |
| `ZSTDv07_freeDCtx` | T | yes | yes |
| `ZSTDv07_freeDDict` | T | yes | yes |
| `ZSTDv07_getDecompressedSize` | T | yes | yes |
| `ZSTDv07_getErrorName` | T | yes | yes |
| `ZSTDv07_getFrameParams` | T | yes | yes |
| `ZSTDv07_insertBlock` | T | yes | yes |
| `ZSTDv07_isError` | T | yes | yes |
| `ZSTDv07_isSkipFrame` | T | yes | yes |
| `ZSTDv07_nextSrcSizeToDecompress` | T | yes | yes |
| `ZSTDv07_sizeofDCtx` | T | yes | yes |

### other (10 symbols)

| symbol | type | in C | in Rust |
|---|---|---|---|
| `POOL_add` | T | yes | yes |
| `POOL_create` | T | yes | yes |
| `POOL_create_advanced` | T | yes | yes |
| `POOL_free` | T | yes | yes |
| `POOL_joinJobs` | T | yes | yes |
| `POOL_resize` | T | yes | yes |
| `POOL_sizeof` | T | yes | yes |
| `POOL_tryAdd` | T | yes | yes |
| `g_ZSTD_threading_useless_symbol` | B | yes | yes |
| `g_debuglevel` | B | yes | yes |

### zstd core (271 symbols)

| symbol | type | in C | in Rust |
|---|---|---|---|
| `ZSTD_CCtxParams_getParameter` | T | yes | yes |
| `ZSTD_CCtxParams_init` | T | yes | yes |
| `ZSTD_CCtxParams_init_advanced` | T | yes | yes |
| `ZSTD_CCtxParams_registerSequenceProducer` | T | yes | yes |
| `ZSTD_CCtxParams_reset` | T | yes | yes |
| `ZSTD_CCtxParams_setParameter` | T | yes | yes |
| `ZSTD_CCtx_getParameter` | T | yes | yes |
| `ZSTD_CCtx_loadDictionary` | T | yes | yes |
| `ZSTD_CCtx_loadDictionary_advanced` | T | yes | yes |
| `ZSTD_CCtx_loadDictionary_byReference` | T | yes | yes |
| `ZSTD_CCtx_refCDict` | T | yes | yes |
| `ZSTD_CCtx_refPrefix` | T | yes | yes |
| `ZSTD_CCtx_refPrefix_advanced` | T | yes | yes |
| `ZSTD_CCtx_refThreadPool` | T | yes | yes |
| `ZSTD_CCtx_reset` | T | yes | yes |
| `ZSTD_CCtx_setCParams` | T | yes | yes |
| `ZSTD_CCtx_setFParams` | T | yes | yes |
| `ZSTD_CCtx_setParameter` | T | yes | yes |
| `ZSTD_CCtx_setParametersUsingCCtxParams` | T | yes | yes |
| `ZSTD_CCtx_setParams` | T | yes | yes |
| `ZSTD_CCtx_setPledgedSrcSize` | T | yes | yes |
| `ZSTD_CCtx_trace` | T | yes | yes |
| `ZSTD_CStreamInSize` | T | yes | yes |
| `ZSTD_CStreamOutSize` | T | yes | yes |
| `ZSTD_DCtx_getParameter` | T | yes | yes |
| `ZSTD_DCtx_loadDictionary` | T | yes | yes |
| `ZSTD_DCtx_loadDictionary_advanced` | T | yes | yes |
| `ZSTD_DCtx_loadDictionary_byReference` | T | yes | yes |
| `ZSTD_DCtx_refDDict` | T | yes | yes |
| `ZSTD_DCtx_refPrefix` | T | yes | yes |
| `ZSTD_DCtx_refPrefix_advanced` | T | yes | yes |
| `ZSTD_DCtx_reset` | T | yes | yes |
| `ZSTD_DCtx_setFormat` | T | yes | yes |
| `ZSTD_DCtx_setMaxWindowSize` | T | yes | yes |
| `ZSTD_DCtx_setParameter` | T | yes | yes |
| `ZSTD_DDict_dictContent` | T | yes | yes |
| `ZSTD_DDict_dictSize` | T | yes | yes |
| `ZSTD_DStreamInSize` | T | yes | yes |
| `ZSTD_DStreamOutSize` | T | yes | yes |
| `ZSTD_adjustCParams` | T | yes | yes |
| `ZSTD_buildBlockEntropyStats` | T | yes | yes |
| `ZSTD_buildCTable` | T | yes | yes |
| `ZSTD_buildFSETable` | T | yes | yes |
| `ZSTD_cParam_getBounds` | T | yes | yes |
| `ZSTD_checkCParams` | T | yes | yes |
| `ZSTD_checkContinuity` | T | yes | yes |
| `ZSTD_compress` | T | yes | yes |
| `ZSTD_compress2` | T | yes | yes |
| `ZSTD_compressBegin` | T | yes | yes |
| `ZSTD_compressBegin_advanced` | T | yes | yes |
| `ZSTD_compressBegin_advanced_internal` | T | yes | yes |
| `ZSTD_compressBegin_usingCDict` | T | yes | yes |
| `ZSTD_compressBegin_usingCDict_advanced` | T | yes | yes |
| `ZSTD_compressBegin_usingCDict_deprecated` | T | yes | yes |
| `ZSTD_compressBegin_usingDict` | T | yes | yes |
| `ZSTD_compressBlock` | T | yes | yes |
| `ZSTD_compressBlock_btlazy2` | T | yes | yes |
| `ZSTD_compressBlock_btlazy2_dictMatchState` | T | yes | yes |
| `ZSTD_compressBlock_btlazy2_extDict` | T | yes | yes |
| `ZSTD_compressBlock_btopt` | T | yes | yes |
| `ZSTD_compressBlock_btopt_dictMatchState` | T | yes | yes |
| `ZSTD_compressBlock_btopt_extDict` | T | yes | yes |
| `ZSTD_compressBlock_btultra` | T | yes | yes |
| `ZSTD_compressBlock_btultra2` | T | yes | yes |
| `ZSTD_compressBlock_btultra_dictMatchState` | T | yes | yes |
| `ZSTD_compressBlock_btultra_extDict` | T | yes | yes |
| `ZSTD_compressBlock_deprecated` | T | yes | yes |
| `ZSTD_compressBlock_doubleFast` | T | yes | yes |
| `ZSTD_compressBlock_doubleFast_dictMatchState` | T | yes | yes |
| `ZSTD_compressBlock_doubleFast_extDict` | T | yes | yes |
| `ZSTD_compressBlock_fast` | T | yes | yes |
| `ZSTD_compressBlock_fast_dictMatchState` | T | yes | yes |
| `ZSTD_compressBlock_fast_extDict` | T | yes | yes |
| `ZSTD_compressBlock_greedy` | T | yes | yes |
| `ZSTD_compressBlock_greedy_dedicatedDictSearch` | T | yes | yes |
| `ZSTD_compressBlock_greedy_dedicatedDictSearch_row` | T | yes | yes |
| `ZSTD_compressBlock_greedy_dictMatchState` | T | yes | yes |
| `ZSTD_compressBlock_greedy_dictMatchState_row` | T | yes | yes |
| `ZSTD_compressBlock_greedy_extDict` | T | yes | yes |
| `ZSTD_compressBlock_greedy_extDict_row` | T | yes | yes |
| `ZSTD_compressBlock_greedy_row` | T | yes | yes |
| `ZSTD_compressBlock_lazy` | T | yes | yes |
| `ZSTD_compressBlock_lazy2` | T | yes | yes |
| `ZSTD_compressBlock_lazy2_dedicatedDictSearch` | T | yes | yes |
| `ZSTD_compressBlock_lazy2_dedicatedDictSearch_row` | T | yes | yes |
| `ZSTD_compressBlock_lazy2_dictMatchState` | T | yes | yes |
| `ZSTD_compressBlock_lazy2_dictMatchState_row` | T | yes | yes |
| `ZSTD_compressBlock_lazy2_extDict` | T | yes | yes |
| `ZSTD_compressBlock_lazy2_extDict_row` | T | yes | yes |
| `ZSTD_compressBlock_lazy2_row` | T | yes | yes |
| `ZSTD_compressBlock_lazy_dedicatedDictSearch` | T | yes | yes |
| `ZSTD_compressBlock_lazy_dedicatedDictSearch_row` | T | yes | yes |
| `ZSTD_compressBlock_lazy_dictMatchState` | T | yes | yes |
| `ZSTD_compressBlock_lazy_dictMatchState_row` | T | yes | yes |
| `ZSTD_compressBlock_lazy_extDict` | T | yes | yes |
| `ZSTD_compressBlock_lazy_extDict_row` | T | yes | yes |
| `ZSTD_compressBlock_lazy_row` | T | yes | yes |
| `ZSTD_compressBound` | T | yes | yes |
| `ZSTD_compressCCtx` | T | yes | yes |
| `ZSTD_compressContinue` | T | yes | yes |
| `ZSTD_compressContinue_public` | T | yes | yes |
| `ZSTD_compressEnd` | T | yes | yes |
| `ZSTD_compressEnd_public` | T | yes | yes |
| `ZSTD_compressLiterals` | T | yes | yes |
| `ZSTD_compressRleLiteralsBlock` | T | yes | yes |
| `ZSTD_compressSequences` | T | yes | yes |
| `ZSTD_compressSequencesAndLiterals` | T | yes | yes |
| `ZSTD_compressStream` | T | yes | yes |
| `ZSTD_compressStream2` | T | yes | yes |
| `ZSTD_compressStream2_simpleArgs` | T | yes | yes |
| `ZSTD_compressSuperBlock` | T | yes | yes |
| `ZSTD_compress_advanced` | T | yes | yes |
| `ZSTD_compress_advanced_internal` | T | yes | yes |
| `ZSTD_compress_usingCDict` | T | yes | yes |
| `ZSTD_compress_usingCDict_advanced` | T | yes | yes |
| `ZSTD_compress_usingDict` | T | yes | yes |
| `ZSTD_convertBlockSequences` | T | yes | yes |
| `ZSTD_copyCCtx` | T | yes | yes |
| `ZSTD_copyDCtx` | T | yes | yes |
| `ZSTD_copyDDictParameters` | T | yes | yes |
| `ZSTD_createCCtx` | T | yes | yes |
| `ZSTD_createCCtxParams` | T | yes | yes |
| `ZSTD_createCCtx_advanced` | T | yes | yes |
| `ZSTD_createCDict` | T | yes | yes |
| `ZSTD_createCDict_advanced` | T | yes | yes |
| `ZSTD_createCDict_advanced2` | T | yes | yes |
| `ZSTD_createCDict_byReference` | T | yes | yes |
| `ZSTD_createCStream` | T | yes | yes |
| `ZSTD_createCStream_advanced` | T | yes | yes |
| `ZSTD_createDCtx` | T | yes | yes |
| `ZSTD_createDCtx_advanced` | T | yes | yes |
| `ZSTD_createDDict` | T | yes | yes |
| `ZSTD_createDDict_advanced` | T | yes | yes |
| `ZSTD_createDDict_byReference` | T | yes | yes |
| `ZSTD_createDStream` | T | yes | yes |
| `ZSTD_createDStream_advanced` | T | yes | yes |
| `ZSTD_crossEntropyCost` | T | yes | yes |
| `ZSTD_cycleLog` | T | yes | yes |
| `ZSTD_dParam_getBounds` | T | yes | yes |
| `ZSTD_decodeLiteralsBlock_wrapper` | T | yes | yes |
| `ZSTD_decodeSeqHeaders` | T | yes | yes |
| `ZSTD_decodingBufferSize_min` | T | yes | yes |
| `ZSTD_decompress` | T | yes | yes |
| `ZSTD_decompressBegin` | T | yes | yes |
| `ZSTD_decompressBegin_usingDDict` | T | yes | yes |
| `ZSTD_decompressBegin_usingDict` | T | yes | yes |
| `ZSTD_decompressBlock` | T | yes | yes |
| `ZSTD_decompressBlock_deprecated` | T | yes | yes |
| `ZSTD_decompressBlock_internal` | T | yes | yes |
| `ZSTD_decompressBound` | T | yes | yes |
| `ZSTD_decompressContinue` | T | yes | yes |
| `ZSTD_decompressDCtx` | T | yes | yes |
| `ZSTD_decompressStream` | T | yes | yes |
| `ZSTD_decompressStream_simpleArgs` | T | yes | yes |
| `ZSTD_decompress_usingDDict` | T | yes | yes |
| `ZSTD_decompress_usingDict` | T | yes | yes |
| `ZSTD_decompressionMargin` | T | yes | yes |
| `ZSTD_dedicatedDictSearch_lazy_loadDictionary` | T | yes | yes |
| `ZSTD_defaultCLevel` | T | yes | yes |
| `ZSTD_encodeSequences` | T | yes | yes |
| `ZSTD_endStream` | T | yes | yes |
| `ZSTD_estimateCCtxSize` | T | yes | yes |
| `ZSTD_estimateCCtxSize_usingCCtxParams` | T | yes | yes |
| `ZSTD_estimateCCtxSize_usingCParams` | T | yes | yes |
| `ZSTD_estimateCDictSize` | T | yes | yes |
| `ZSTD_estimateCDictSize_advanced` | T | yes | yes |
| `ZSTD_estimateCStreamSize` | T | yes | yes |
| `ZSTD_estimateCStreamSize_usingCCtxParams` | T | yes | yes |
| `ZSTD_estimateCStreamSize_usingCParams` | T | yes | yes |
| `ZSTD_estimateDCtxSize` | T | yes | yes |
| `ZSTD_estimateDDictSize` | T | yes | yes |
| `ZSTD_estimateDStreamSize` | T | yes | yes |
| `ZSTD_estimateDStreamSize_fromFrame` | T | yes | yes |
| `ZSTD_fillDoubleHashTable` | T | yes | yes |
| `ZSTD_fillHashTable` | T | yes | yes |
| `ZSTD_findDecompressedSize` | T | yes | yes |
| `ZSTD_findFrameCompressedSize` | T | yes | yes |
| `ZSTD_flushStream` | T | yes | yes |
| `ZSTD_frameHeaderSize` | T | yes | yes |
| `ZSTD_freeCCtx` | T | yes | yes |
| `ZSTD_freeCCtxParams` | T | yes | yes |
| `ZSTD_freeCDict` | T | yes | yes |
| `ZSTD_freeCStream` | T | yes | yes |
| `ZSTD_freeDCtx` | T | yes | yes |
| `ZSTD_freeDDict` | T | yes | yes |
| `ZSTD_freeDStream` | T | yes | yes |
| `ZSTD_fseBitCost` | T | yes | yes |
| `ZSTD_generateSequences` | T | yes | yes |
| `ZSTD_get1BlockSummary` | T | yes | yes |
| `ZSTD_getBlockSize` | T | yes | yes |
| `ZSTD_getCParams` | T | yes | yes |
| `ZSTD_getCParamsFromCCtxParams` | T | yes | yes |
| `ZSTD_getCParamsFromCDict` | T | yes | yes |
| `ZSTD_getDecompressedSize` | T | yes | yes |
| `ZSTD_getDictID_fromCDict` | T | yes | yes |
| `ZSTD_getDictID_fromDDict` | T | yes | yes |
| `ZSTD_getDictID_fromDict` | T | yes | yes |
| `ZSTD_getDictID_fromFrame` | T | yes | yes |
| `ZSTD_getErrorCode` | T | yes | yes |
| `ZSTD_getErrorName` | T | yes | yes |
| `ZSTD_getErrorString` | T | yes | yes |
| `ZSTD_getFrameContentSize` | T | yes | yes |
| `ZSTD_getFrameHeader` | T | yes | yes |
| `ZSTD_getFrameHeader_advanced` | T | yes | yes |
| `ZSTD_getFrameProgression` | T | yes | yes |
| `ZSTD_getParams` | T | yes | yes |
| `ZSTD_getSeqStore` | T | yes | yes |
| `ZSTD_getcBlockSize` | T | yes | yes |
| `ZSTD_initCStream` | T | yes | yes |
| `ZSTD_initCStream_advanced` | T | yes | yes |
| `ZSTD_initCStream_internal` | T | yes | yes |
| `ZSTD_initCStream_srcSize` | T | yes | yes |
| `ZSTD_initCStream_usingCDict` | T | yes | yes |
| `ZSTD_initCStream_usingCDict_advanced` | T | yes | yes |
| `ZSTD_initCStream_usingDict` | T | yes | yes |
| `ZSTD_initDStream` | T | yes | yes |
| `ZSTD_initDStream_usingDDict` | T | yes | yes |
| `ZSTD_initDStream_usingDict` | T | yes | yes |
| `ZSTD_initStaticCCtx` | T | yes | yes |
| `ZSTD_initStaticCDict` | T | yes | yes |
| `ZSTD_initStaticCStream` | T | yes | yes |
| `ZSTD_initStaticDCtx` | T | yes | yes |
| `ZSTD_initStaticDDict` | T | yes | yes |
| `ZSTD_initStaticDStream` | T | yes | yes |
| `ZSTD_insertAndFindFirstIndex` | T | yes | yes |
| `ZSTD_insertBlock` | T | yes | yes |
| `ZSTD_invalidateRepCodes` | T | yes | yes |
| `ZSTD_isError` | T | yes | yes |
| `ZSTD_isFrame` | T | yes | yes |
| `ZSTD_isSkippableFrame` | T | yes | yes |
| `ZSTD_ldm_adjustParameters` | T | yes | yes |
| `ZSTD_ldm_blockCompress` | T | yes | yes |
| `ZSTD_ldm_fillHashTable` | T | yes | yes |
| `ZSTD_ldm_generateSequences` | T | yes | yes |
| `ZSTD_ldm_getMaxNbSeq` | T | yes | yes |
| `ZSTD_ldm_getTableSize` | T | yes | yes |
| `ZSTD_ldm_skipRawSeqStoreBytes` | T | yes | yes |
| `ZSTD_ldm_skipSequences` | T | yes | yes |
| `ZSTD_loadCEntropy` | T | yes | yes |
| `ZSTD_loadDEntropy` | T | yes | yes |
| `ZSTD_maxCLevel` | T | yes | yes |
| `ZSTD_mergeBlockDelimiters` | T | yes | yes |
| `ZSTD_minCLevel` | T | yes | yes |
| `ZSTD_nextInputType` | T | yes | yes |
| `ZSTD_nextSrcSizeToDecompress` | T | yes | yes |
| `ZSTD_noCompressLiterals` | T | yes | yes |
| `ZSTD_readSkippableFrame` | T | yes | yes |
| `ZSTD_referenceExternalSequences` | T | yes | yes |
| `ZSTD_registerSequenceProducer` | T | yes | yes |
| `ZSTD_resetCStream` | T | yes | yes |
| `ZSTD_resetDStream` | T | yes | yes |
| `ZSTD_resetSeqStore` | T | yes | yes |
| `ZSTD_reset_compressedBlockState` | T | yes | yes |
| `ZSTD_row_update` | T | yes | yes |
| `ZSTD_selectBlockCompressor` | T | yes | yes |
| `ZSTD_selectEncodingType` | T | yes | yes |
| `ZSTD_seqToCodes` | T | yes | yes |
| `ZSTD_sequenceBound` | T | yes | yes |
| `ZSTD_sizeof_CCtx` | T | yes | yes |
| `ZSTD_sizeof_CDict` | T | yes | yes |
| `ZSTD_sizeof_CStream` | T | yes | yes |
| `ZSTD_sizeof_DCtx` | T | yes | yes |
| `ZSTD_sizeof_DDict` | T | yes | yes |
| `ZSTD_sizeof_DStream` | T | yes | yes |
| `ZSTD_splitBlock` | T | yes | yes |
| `ZSTD_toFlushNow` | T | yes | yes |
| `ZSTD_updateTree` | T | yes | yes |
| `ZSTD_versionNumber` | T | yes | yes |
| `ZSTD_versionString` | T | yes | yes |
| `ZSTD_writeLastEmptyBlock` | T | yes | yes |
| `ZSTD_writeSkippableFrame` | T | yes | yes |

### zstdmt (9 symbols)

| symbol | type | in C | in Rust |
|---|---|---|---|
| `ZSTDMT_compressStream_generic` | T | yes | yes |
| `ZSTDMT_createCCtx_advanced` | T | yes | yes |
| `ZSTDMT_freeCCtx` | T | yes | yes |
| `ZSTDMT_getFrameProgression` | T | yes | yes |
| `ZSTDMT_initCStream_internal` | T | yes | yes |
| `ZSTDMT_nextInputSizeHint` | T | yes | yes |
| `ZSTDMT_sizeof_CCtx` | T | yes | yes |
| `ZSTDMT_toFlushNow` | T | yes | yes |
| `ZSTDMT_updateCParams_whileCompressing` | T | yes | yes |

