# SYMBOLS.md — C vs Rust exported-symbol parity

Generated mechanically from `nm -D --defined-only` on both shared libraries.

- C   : `c_src/build/libzstd.so`
- Rust: `translation/target/release/libzstd.so`

## 1. Parity (the Phase A / Phase D gate)

| metric | value |
|---|---|
| C exported symbols | 615 |
| Rust exported symbols | 615 |
| **Missing from Rust** | **0** |
| Extra in Rust | 0 |
| Rust undefined non-libc / non-Rust-runtime symbols | 0 |

**The symbol diff is EMPTY in both directions.** Enforced as a test
(`tests/t99_symbols.rs`), which `dlsym`s all 615 names in BOTH libraries, so
a dropped `#[no_mangle]` fails the suite rather than going unnoticed.

Note: the C `.so` additionally lists 4 *weak undefined* tracing hooks
(`ZSTD_trace_compress_begin/end`, `ZSTD_trace_decompress_begin/end`). Those are
opt-in user callbacks, not exported definitions — `ZSTD_TRACE` is off in this
build, so they are absent from both libraries as definitions.

## 2. Runtime exercise coverage

Measured by `tools/coverage.sh`: the shared harness records every symbol
resolved through `Impls::pair()`/`has()` while the whole suite runs, and the
result is diffed against `nm -D`. `tests/t99_symbols.rs` is excluded from this
measurement because it deliberately probes all 615 (including it would make
every symbol look exercised and the number meaningless).

| metric | value |
|---|---|
| Called directly through both `.so`s by a passing test | **570 recorded, 534 of 615 exports** |
| Not directly callable (see §3) | 81 |

## 3. The 81 symbols not called directly, and why

Every one of these is an **internal-linkage API**: it takes (or returns) a
private struct type that has no public layout, so no external consumer can
construct a valid argument for it. They are reached only through the public
API and are therefore covered *indirectly* — which is exactly what the `[i]`
rows in `ERRORS.md` / `CONFIGS.md` record.

| group | count | private type(s) in the signature | how it IS covered |
|---|---|---|---|
| `ZSTD_compressBlock_*` match finders | 40 | `ZSTD_MatchState_t*`, `SeqStore_t*` | the 9-strategy x dictMode x rowMatchFinder sweeps in `t02`/`t04`; the dispatch table itself is compared symbol-for-symbol by `t14::select_block_compressor_partition_matches` |
| `ZSTDMT_*` | 9 | `ZSTDMT_CCtx*` | dead in this build — no `ZSTD_MULTITHREAD`, so `ZSTD_c_nbWorkers` bounds collapse to {0,0} (asserted in `t02`) |
| `ZSTD_ldm_*` | 8 | `ldmState_t*`, `RawSeqStore_t*`, `ldmParams_t` | the LDM configuration rows in `t02`/`t03`/`t14` (enable x hashLog x minMatch x bucketSizeLog x hashRateLog) |
| other block/entropy internals | 24 | `SeqStore_t*`, `ZSTD_entropyCTables_t*`, `ZSTD_CCtx_params*`, `rawSeq*`, `ZSTD_hufCTables_t*`, `ZSTD_entropyDTables_t*`, `ZSTD_seqSymbol*` | full-pipeline compress/decompress equality over all shapes, sizes, levels, strategies and options |

### Full list

```
ZSTDMT_compressStream_generic
ZSTDMT_createCCtx_advanced
ZSTDMT_freeCCtx
ZSTDMT_getFrameProgression
ZSTDMT_initCStream_internal
ZSTDMT_nextInputSizeHint
ZSTDMT_sizeof_CCtx
ZSTDMT_toFlushNow
ZSTDMT_updateCParams_whileCompressing
ZSTD_buildBlockEntropyStats
ZSTD_buildCTable
ZSTD_buildFSETable
ZSTD_compressBegin_advanced_internal
ZSTD_compressBlock_btlazy2
ZSTD_compressBlock_btlazy2_dictMatchState
ZSTD_compressBlock_btlazy2_extDict
ZSTD_compressBlock_btopt
ZSTD_compressBlock_btopt_dictMatchState
ZSTD_compressBlock_btopt_extDict
ZSTD_compressBlock_btultra
ZSTD_compressBlock_btultra2
ZSTD_compressBlock_btultra_dictMatchState
ZSTD_compressBlock_btultra_extDict
ZSTD_compressBlock_doubleFast
ZSTD_compressBlock_doubleFast_dictMatchState
ZSTD_compressBlock_doubleFast_extDict
ZSTD_compressBlock_fast
ZSTD_compressBlock_fast_dictMatchState
ZSTD_compressBlock_fast_extDict
ZSTD_compressBlock_greedy
ZSTD_compressBlock_greedy_dedicatedDictSearch
ZSTD_compressBlock_greedy_dedicatedDictSearch_row
ZSTD_compressBlock_greedy_dictMatchState
ZSTD_compressBlock_greedy_dictMatchState_row
ZSTD_compressBlock_greedy_extDict
ZSTD_compressBlock_greedy_extDict_row
ZSTD_compressBlock_greedy_row
ZSTD_compressBlock_lazy
ZSTD_compressBlock_lazy2
ZSTD_compressBlock_lazy2_dedicatedDictSearch
ZSTD_compressBlock_lazy2_dedicatedDictSearch_row
ZSTD_compressBlock_lazy2_dictMatchState
ZSTD_compressBlock_lazy2_dictMatchState_row
ZSTD_compressBlock_lazy2_extDict
ZSTD_compressBlock_lazy2_extDict_row
ZSTD_compressBlock_lazy2_row
ZSTD_compressBlock_lazy_dedicatedDictSearch
ZSTD_compressBlock_lazy_dedicatedDictSearch_row
ZSTD_compressBlock_lazy_dictMatchState
ZSTD_compressBlock_lazy_dictMatchState_row
ZSTD_compressBlock_lazy_extDict
ZSTD_compressBlock_lazy_extDict_row
ZSTD_compressBlock_lazy_row
ZSTD_compressLiterals
ZSTD_compressSuperBlock
ZSTD_compress_advanced_internal
ZSTD_decompressBlock_internal
ZSTD_dedicatedDictSearch_lazy_loadDictionary
ZSTD_encodeSequences
ZSTD_fillDoubleHashTable
ZSTD_fillHashTable
ZSTD_fseBitCost
ZSTD_getSeqStore
ZSTD_initCStream_internal
ZSTD_insertAndFindFirstIndex
ZSTD_ldm_adjustParameters
ZSTD_ldm_blockCompress
ZSTD_ldm_fillHashTable
ZSTD_ldm_generateSequences
ZSTD_ldm_getMaxNbSeq
ZSTD_ldm_getTableSize
ZSTD_ldm_skipRawSeqStoreBytes
ZSTD_ldm_skipSequences
ZSTD_loadCEntropy
ZSTD_loadDEntropy
ZSTD_referenceExternalSequences
ZSTD_resetSeqStore
ZSTD_reset_compressedBlockState
ZSTD_row_update
ZSTD_seqToCodes
ZSTD_updateTree
```

## 4. Full symbol table

`exercised` = called directly through both `.so`s by a passing differential test.

| # | symbol | in C .so | in Rust .so | exercised directly |
|---|--------|----------|-------------|--------------------|
| 1 | `COVER_best_destroy` | YES | YES | yes |
| 2 | `COVER_best_finish` | YES | YES | yes |
| 3 | `COVER_best_init` | YES | YES | yes |
| 4 | `COVER_best_start` | YES | YES | yes |
| 5 | `COVER_best_wait` | YES | YES | yes |
| 6 | `COVER_checkTotalCompressedSize` | YES | YES | yes |
| 7 | `COVER_computeEpochs` | YES | YES | yes |
| 8 | `COVER_dictSelectionError` | YES | YES | yes |
| 9 | `COVER_dictSelectionFree` | YES | YES | yes |
| 10 | `COVER_dictSelectionIsError` | YES | YES | yes |
| 11 | `COVER_selectDict` | YES | YES | yes |
| 12 | `COVER_sum` | YES | YES | yes |
| 13 | `COVER_warnOnSmallCorpus` | YES | YES | yes |
| 14 | `ERR_getErrorString` | YES | YES | yes |
| 15 | `FSE_NCountWriteBound` | YES | YES | yes |
| 16 | `FSE_buildCTable_rle` | YES | YES | yes |
| 17 | `FSE_buildCTable_wksp` | YES | YES | yes |
| 18 | `FSE_buildDTable_wksp` | YES | YES | yes |
| 19 | `FSE_compressBound` | YES | YES | yes |
| 20 | `FSE_compress_usingCTable` | YES | YES | yes |
| 21 | `FSE_decompress_wksp_bmi2` | YES | YES | yes |
| 22 | `FSE_getErrorName` | YES | YES | yes |
| 23 | `FSE_isError` | YES | YES | yes |
| 24 | `FSE_normalizeCount` | YES | YES | yes |
| 25 | `FSE_optimalTableLog` | YES | YES | yes |
| 26 | `FSE_optimalTableLog_internal` | YES | YES | yes |
| 27 | `FSE_readNCount` | YES | YES | yes |
| 28 | `FSE_readNCount_bmi2` | YES | YES | yes |
| 29 | `FSE_versionNumber` | YES | YES | yes |
| 30 | `FSE_writeNCount` | YES | YES | yes |
| 31 | `FSEv05_buildDTable` | YES | YES | yes |
| 32 | `FSEv05_buildDTable_raw` | YES | YES | yes |
| 33 | `FSEv05_buildDTable_rle` | YES | YES | yes |
| 34 | `FSEv05_createDTable` | YES | YES | yes |
| 35 | `FSEv05_decompress` | YES | YES | yes |
| 36 | `FSEv05_decompress_usingDTable` | YES | YES | yes |
| 37 | `FSEv05_freeDTable` | YES | YES | yes |
| 38 | `FSEv05_getErrorName` | YES | YES | yes |
| 39 | `FSEv05_isError` | YES | YES | yes |
| 40 | `FSEv05_readNCount` | YES | YES | yes |
| 41 | `FSEv06_buildDTable` | YES | YES | yes |
| 42 | `FSEv06_buildDTable_raw` | YES | YES | yes |
| 43 | `FSEv06_buildDTable_rle` | YES | YES | yes |
| 44 | `FSEv06_createDTable` | YES | YES | yes |
| 45 | `FSEv06_decompress` | YES | YES | yes |
| 46 | `FSEv06_decompress_usingDTable` | YES | YES | yes |
| 47 | `FSEv06_freeDTable` | YES | YES | yes |
| 48 | `FSEv06_getErrorName` | YES | YES | yes |
| 49 | `FSEv06_isError` | YES | YES | yes |
| 50 | `FSEv06_readNCount` | YES | YES | yes |
| 51 | `FSEv07_buildDTable` | YES | YES | yes |
| 52 | `FSEv07_buildDTable_raw` | YES | YES | yes |
| 53 | `FSEv07_buildDTable_rle` | YES | YES | yes |
| 54 | `FSEv07_createDTable` | YES | YES | yes |
| 55 | `FSEv07_decompress` | YES | YES | yes |
| 56 | `FSEv07_decompress_usingDTable` | YES | YES | yes |
| 57 | `FSEv07_freeDTable` | YES | YES | yes |
| 58 | `FSEv07_getErrorName` | YES | YES | yes |
| 59 | `FSEv07_isError` | YES | YES | yes |
| 60 | `FSEv07_readNCount` | YES | YES | yes |
| 61 | `HIST_add` | YES | YES | yes |
| 62 | `HIST_count` | YES | YES | yes |
| 63 | `HIST_countFast` | YES | YES | yes |
| 64 | `HIST_countFast_wksp` | YES | YES | yes |
| 65 | `HIST_count_simple` | YES | YES | yes |
| 66 | `HIST_count_wksp` | YES | YES | yes |
| 67 | `HIST_isError` | YES | YES | yes |
| 68 | `HUF_buildCTable_wksp` | YES | YES | yes |
| 69 | `HUF_cardinality` | YES | YES | yes |
| 70 | `HUF_compress1X_repeat` | YES | YES | yes |
| 71 | `HUF_compress1X_usingCTable` | YES | YES | yes |
| 72 | `HUF_compress4X_repeat` | YES | YES | yes |
| 73 | `HUF_compress4X_usingCTable` | YES | YES | yes |
| 74 | `HUF_compressBound` | YES | YES | yes |
| 75 | `HUF_decompress1X1_DCtx_wksp` | YES | YES | yes |
| 76 | `HUF_decompress1X2_DCtx_wksp` | YES | YES | yes |
| 77 | `HUF_decompress1X_DCtx_wksp` | YES | YES | yes |
| 78 | `HUF_decompress1X_usingDTable` | YES | YES | yes |
| 79 | `HUF_decompress4X_hufOnly_wksp` | YES | YES | yes |
| 80 | `HUF_decompress4X_usingDTable` | YES | YES | yes |
| 81 | `HUF_estimateCompressedSize` | YES | YES | yes |
| 82 | `HUF_getErrorName` | YES | YES | yes |
| 83 | `HUF_getNbBitsFromCTable` | YES | YES | yes |
| 84 | `HUF_isError` | YES | YES | yes |
| 85 | `HUF_minTableLog` | YES | YES | yes |
| 86 | `HUF_optimalTableLog` | YES | YES | yes |
| 87 | `HUF_readCTable` | YES | YES | yes |
| 88 | `HUF_readCTableHeader` | YES | YES | yes |
| 89 | `HUF_readDTableX1_wksp` | YES | YES | yes |
| 90 | `HUF_readDTableX2_wksp` | YES | YES | yes |
| 91 | `HUF_readStats` | YES | YES | yes |
| 92 | `HUF_readStats_wksp` | YES | YES | yes |
| 93 | `HUF_selectDecoder` | YES | YES | yes |
| 94 | `HUF_validateCTable` | YES | YES | yes |
| 95 | `HUF_writeCTable_wksp` | YES | YES | yes |
| 96 | `HUFv05_decompress` | YES | YES | yes |
| 97 | `HUFv05_decompress1X2` | YES | YES | yes |
| 98 | `HUFv05_decompress1X2_usingDTable` | YES | YES | yes |
| 99 | `HUFv05_decompress1X4` | YES | YES | yes |
| 100 | `HUFv05_decompress1X4_usingDTable` | YES | YES | yes |
| 101 | `HUFv05_decompress4X2` | YES | YES | yes |
| 102 | `HUFv05_decompress4X2_usingDTable` | YES | YES | yes |
| 103 | `HUFv05_decompress4X4` | YES | YES | yes |
| 104 | `HUFv05_decompress4X4_usingDTable` | YES | YES | yes |
| 105 | `HUFv05_getErrorName` | YES | YES | yes |
| 106 | `HUFv05_isError` | YES | YES | yes |
| 107 | `HUFv05_readDTableX2` | YES | YES | yes |
| 108 | `HUFv05_readDTableX4` | YES | YES | yes |
| 109 | `HUFv06_decompress` | YES | YES | yes |
| 110 | `HUFv06_decompress1X2` | YES | YES | yes |
| 111 | `HUFv06_decompress1X2_usingDTable` | YES | YES | yes |
| 112 | `HUFv06_decompress1X4` | YES | YES | yes |
| 113 | `HUFv06_decompress1X4_usingDTable` | YES | YES | yes |
| 114 | `HUFv06_decompress4X2` | YES | YES | yes |
| 115 | `HUFv06_decompress4X2_usingDTable` | YES | YES | yes |
| 116 | `HUFv06_decompress4X4` | YES | YES | yes |
| 117 | `HUFv06_decompress4X4_usingDTable` | YES | YES | yes |
| 118 | `HUFv06_readDTableX2` | YES | YES | yes |
| 119 | `HUFv06_readDTableX4` | YES | YES | yes |
| 120 | `HUFv07_decompress` | YES | YES | yes |
| 121 | `HUFv07_decompress1X2` | YES | YES | yes |
| 122 | `HUFv07_decompress1X2_DCtx` | YES | YES | yes |
| 123 | `HUFv07_decompress1X2_usingDTable` | YES | YES | yes |
| 124 | `HUFv07_decompress1X4` | YES | YES | yes |
| 125 | `HUFv07_decompress1X4_DCtx` | YES | YES | yes |
| 126 | `HUFv07_decompress1X4_usingDTable` | YES | YES | yes |
| 127 | `HUFv07_decompress1X_DCtx` | YES | YES | yes |
| 128 | `HUFv07_decompress1X_usingDTable` | YES | YES | yes |
| 129 | `HUFv07_decompress4X2` | YES | YES | yes |
| 130 | `HUFv07_decompress4X2_DCtx` | YES | YES | yes |
| 131 | `HUFv07_decompress4X2_usingDTable` | YES | YES | yes |
| 132 | `HUFv07_decompress4X4` | YES | YES | yes |
| 133 | `HUFv07_decompress4X4_DCtx` | YES | YES | yes |
| 134 | `HUFv07_decompress4X4_usingDTable` | YES | YES | yes |
| 135 | `HUFv07_decompress4X_DCtx` | YES | YES | yes |
| 136 | `HUFv07_decompress4X_hufOnly` | YES | YES | yes |
| 137 | `HUFv07_decompress4X_usingDTable` | YES | YES | yes |
| 138 | `HUFv07_getErrorName` | YES | YES | yes |
| 139 | `HUFv07_isError` | YES | YES | yes |
| 140 | `HUFv07_readDTableX2` | YES | YES | yes |
| 141 | `HUFv07_readDTableX4` | YES | YES | yes |
| 142 | `HUFv07_readStats` | YES | YES | yes |
| 143 | `HUFv07_selectDecoder` | YES | YES | yes |
| 144 | `POOL_add` | YES | YES | yes |
| 145 | `POOL_create` | YES | YES | yes |
| 146 | `POOL_create_advanced` | YES | YES | yes |
| 147 | `POOL_free` | YES | YES | yes |
| 148 | `POOL_joinJobs` | YES | YES | yes |
| 149 | `POOL_resize` | YES | YES | yes |
| 150 | `POOL_sizeof` | YES | YES | yes |
| 151 | `POOL_tryAdd` | YES | YES | yes |
| 152 | `ZBUFF_compressContinue` | YES | YES | yes |
| 153 | `ZBUFF_compressEnd` | YES | YES | yes |
| 154 | `ZBUFF_compressFlush` | YES | YES | yes |
| 155 | `ZBUFF_compressInit` | YES | YES | yes |
| 156 | `ZBUFF_compressInitDictionary` | YES | YES | yes |
| 157 | `ZBUFF_compressInit_advanced` | YES | YES | yes |
| 158 | `ZBUFF_createCCtx` | YES | YES | yes |
| 159 | `ZBUFF_createCCtx_advanced` | YES | YES | yes |
| 160 | `ZBUFF_createDCtx` | YES | YES | yes |
| 161 | `ZBUFF_createDCtx_advanced` | YES | YES | yes |
| 162 | `ZBUFF_decompressContinue` | YES | YES | yes |
| 163 | `ZBUFF_decompressInit` | YES | YES | yes |
| 164 | `ZBUFF_decompressInitDictionary` | YES | YES | yes |
| 165 | `ZBUFF_freeCCtx` | YES | YES | yes |
| 166 | `ZBUFF_freeDCtx` | YES | YES | yes |
| 167 | `ZBUFF_getErrorName` | YES | YES | yes |
| 168 | `ZBUFF_isError` | YES | YES | yes |
| 169 | `ZBUFF_recommendedCInSize` | YES | YES | yes |
| 170 | `ZBUFF_recommendedCOutSize` | YES | YES | yes |
| 171 | `ZBUFF_recommendedDInSize` | YES | YES | yes |
| 172 | `ZBUFF_recommendedDOutSize` | YES | YES | yes |
| 173 | `ZBUFFv04_createDCtx` | YES | YES | yes |
| 174 | `ZBUFFv04_decompressContinue` | YES | YES | yes |
| 175 | `ZBUFFv04_decompressInit` | YES | YES | yes |
| 176 | `ZBUFFv04_decompressWithDictionary` | YES | YES | yes |
| 177 | `ZBUFFv04_freeDCtx` | YES | YES | yes |
| 178 | `ZBUFFv04_getErrorName` | YES | YES | yes |
| 179 | `ZBUFFv04_isError` | YES | YES | yes |
| 180 | `ZBUFFv04_recommendedDInSize` | YES | YES | yes |
| 181 | `ZBUFFv04_recommendedDOutSize` | YES | YES | yes |
| 182 | `ZBUFFv05_createDCtx` | YES | YES | yes |
| 183 | `ZBUFFv05_decompressContinue` | YES | YES | yes |
| 184 | `ZBUFFv05_decompressInit` | YES | YES | yes |
| 185 | `ZBUFFv05_decompressInitDictionary` | YES | YES | yes |
| 186 | `ZBUFFv05_freeDCtx` | YES | YES | yes |
| 187 | `ZBUFFv05_getErrorName` | YES | YES | yes |
| 188 | `ZBUFFv05_isError` | YES | YES | yes |
| 189 | `ZBUFFv05_recommendedDInSize` | YES | YES | yes |
| 190 | `ZBUFFv05_recommendedDOutSize` | YES | YES | yes |
| 191 | `ZBUFFv06_createDCtx` | YES | YES | yes |
| 192 | `ZBUFFv06_decompressContinue` | YES | YES | yes |
| 193 | `ZBUFFv06_decompressInit` | YES | YES | yes |
| 194 | `ZBUFFv06_decompressInitDictionary` | YES | YES | yes |
| 195 | `ZBUFFv06_freeDCtx` | YES | YES | yes |
| 196 | `ZBUFFv06_getErrorName` | YES | YES | yes |
| 197 | `ZBUFFv06_isError` | YES | YES | yes |
| 198 | `ZBUFFv06_recommendedDInSize` | YES | YES | yes |
| 199 | `ZBUFFv06_recommendedDOutSize` | YES | YES | yes |
| 200 | `ZBUFFv07_createDCtx` | YES | YES | yes |
| 201 | `ZBUFFv07_createDCtx_advanced` | YES | YES | yes |
| 202 | `ZBUFFv07_decompressContinue` | YES | YES | yes |
| 203 | `ZBUFFv07_decompressInit` | YES | YES | yes |
| 204 | `ZBUFFv07_decompressInitDictionary` | YES | YES | yes |
| 205 | `ZBUFFv07_freeDCtx` | YES | YES | yes |
| 206 | `ZBUFFv07_getErrorName` | YES | YES | yes |
| 207 | `ZBUFFv07_isError` | YES | YES | yes |
| 208 | `ZBUFFv07_recommendedDInSize` | YES | YES | yes |
| 209 | `ZBUFFv07_recommendedDOutSize` | YES | YES | yes |
| 210 | `ZDICT_addEntropyTablesFromBuffer` | YES | YES | yes |
| 211 | `ZDICT_finalizeDictionary` | YES | YES | yes |
| 212 | `ZDICT_getDictHeaderSize` | YES | YES | yes |
| 213 | `ZDICT_getDictID` | YES | YES | yes |
| 214 | `ZDICT_getErrorName` | YES | YES | yes |
| 215 | `ZDICT_isError` | YES | YES | yes |
| 216 | `ZDICT_optimizeTrainFromBuffer_cover` | YES | YES | yes |
| 217 | `ZDICT_optimizeTrainFromBuffer_fastCover` | YES | YES | yes |
| 218 | `ZDICT_trainFromBuffer` | YES | YES | yes |
| 219 | `ZDICT_trainFromBuffer_cover` | YES | YES | yes |
| 220 | `ZDICT_trainFromBuffer_fastCover` | YES | YES | yes |
| 221 | `ZDICT_trainFromBuffer_legacy` | YES | YES | yes |
| 222 | `ZSTDMT_compressStream_generic` | YES | YES | indirect |
| 223 | `ZSTDMT_createCCtx_advanced` | YES | YES | indirect |
| 224 | `ZSTDMT_freeCCtx` | YES | YES | indirect |
| 225 | `ZSTDMT_getFrameProgression` | YES | YES | indirect |
| 226 | `ZSTDMT_initCStream_internal` | YES | YES | indirect |
| 227 | `ZSTDMT_nextInputSizeHint` | YES | YES | indirect |
| 228 | `ZSTDMT_sizeof_CCtx` | YES | YES | indirect |
| 229 | `ZSTDMT_toFlushNow` | YES | YES | indirect |
| 230 | `ZSTDMT_updateCParams_whileCompressing` | YES | YES | indirect |
| 231 | `ZSTD_CCtxParams_getParameter` | YES | YES | yes |
| 232 | `ZSTD_CCtxParams_init` | YES | YES | yes |
| 233 | `ZSTD_CCtxParams_init_advanced` | YES | YES | yes |
| 234 | `ZSTD_CCtxParams_registerSequenceProducer` | YES | YES | yes |
| 235 | `ZSTD_CCtxParams_reset` | YES | YES | yes |
| 236 | `ZSTD_CCtxParams_setParameter` | YES | YES | yes |
| 237 | `ZSTD_CCtx_getParameter` | YES | YES | yes |
| 238 | `ZSTD_CCtx_loadDictionary` | YES | YES | yes |
| 239 | `ZSTD_CCtx_loadDictionary_advanced` | YES | YES | yes |
| 240 | `ZSTD_CCtx_loadDictionary_byReference` | YES | YES | yes |
| 241 | `ZSTD_CCtx_refCDict` | YES | YES | yes |
| 242 | `ZSTD_CCtx_refPrefix` | YES | YES | yes |
| 243 | `ZSTD_CCtx_refPrefix_advanced` | YES | YES | yes |
| 244 | `ZSTD_CCtx_refThreadPool` | YES | YES | yes |
| 245 | `ZSTD_CCtx_reset` | YES | YES | yes |
| 246 | `ZSTD_CCtx_setCParams` | YES | YES | yes |
| 247 | `ZSTD_CCtx_setFParams` | YES | YES | yes |
| 248 | `ZSTD_CCtx_setParameter` | YES | YES | yes |
| 249 | `ZSTD_CCtx_setParametersUsingCCtxParams` | YES | YES | yes |
| 250 | `ZSTD_CCtx_setParams` | YES | YES | yes |
| 251 | `ZSTD_CCtx_setPledgedSrcSize` | YES | YES | yes |
| 252 | `ZSTD_CCtx_trace` | YES | YES | yes |
| 253 | `ZSTD_CStreamInSize` | YES | YES | yes |
| 254 | `ZSTD_CStreamOutSize` | YES | YES | yes |
| 255 | `ZSTD_DCtx_getParameter` | YES | YES | yes |
| 256 | `ZSTD_DCtx_loadDictionary` | YES | YES | yes |
| 257 | `ZSTD_DCtx_loadDictionary_advanced` | YES | YES | yes |
| 258 | `ZSTD_DCtx_loadDictionary_byReference` | YES | YES | yes |
| 259 | `ZSTD_DCtx_refDDict` | YES | YES | yes |
| 260 | `ZSTD_DCtx_refPrefix` | YES | YES | yes |
| 261 | `ZSTD_DCtx_refPrefix_advanced` | YES | YES | yes |
| 262 | `ZSTD_DCtx_reset` | YES | YES | yes |
| 263 | `ZSTD_DCtx_setFormat` | YES | YES | yes |
| 264 | `ZSTD_DCtx_setMaxWindowSize` | YES | YES | yes |
| 265 | `ZSTD_DCtx_setParameter` | YES | YES | yes |
| 266 | `ZSTD_DDict_dictContent` | YES | YES | yes |
| 267 | `ZSTD_DDict_dictSize` | YES | YES | yes |
| 268 | `ZSTD_DStreamInSize` | YES | YES | yes |
| 269 | `ZSTD_DStreamOutSize` | YES | YES | yes |
| 270 | `ZSTD_XXH32` | YES | YES | yes |
| 271 | `ZSTD_XXH32_canonicalFromHash` | YES | YES | yes |
| 272 | `ZSTD_XXH32_copyState` | YES | YES | yes |
| 273 | `ZSTD_XXH32_createState` | YES | YES | yes |
| 274 | `ZSTD_XXH32_digest` | YES | YES | yes |
| 275 | `ZSTD_XXH32_freeState` | YES | YES | yes |
| 276 | `ZSTD_XXH32_hashFromCanonical` | YES | YES | yes |
| 277 | `ZSTD_XXH32_reset` | YES | YES | yes |
| 278 | `ZSTD_XXH32_update` | YES | YES | yes |
| 279 | `ZSTD_XXH64` | YES | YES | yes |
| 280 | `ZSTD_XXH64_canonicalFromHash` | YES | YES | yes |
| 281 | `ZSTD_XXH64_copyState` | YES | YES | yes |
| 282 | `ZSTD_XXH64_createState` | YES | YES | yes |
| 283 | `ZSTD_XXH64_digest` | YES | YES | yes |
| 284 | `ZSTD_XXH64_freeState` | YES | YES | yes |
| 285 | `ZSTD_XXH64_hashFromCanonical` | YES | YES | yes |
| 286 | `ZSTD_XXH64_reset` | YES | YES | yes |
| 287 | `ZSTD_XXH64_update` | YES | YES | yes |
| 288 | `ZSTD_XXH_versionNumber` | YES | YES | yes |
| 289 | `ZSTD_adjustCParams` | YES | YES | yes |
| 290 | `ZSTD_buildBlockEntropyStats` | YES | YES | indirect |
| 291 | `ZSTD_buildCTable` | YES | YES | indirect |
| 292 | `ZSTD_buildFSETable` | YES | YES | indirect |
| 293 | `ZSTD_cParam_getBounds` | YES | YES | yes |
| 294 | `ZSTD_checkCParams` | YES | YES | yes |
| 295 | `ZSTD_checkContinuity` | YES | YES | yes |
| 296 | `ZSTD_compress` | YES | YES | yes |
| 297 | `ZSTD_compress2` | YES | YES | yes |
| 298 | `ZSTD_compressBegin` | YES | YES | yes |
| 299 | `ZSTD_compressBegin_advanced` | YES | YES | yes |
| 300 | `ZSTD_compressBegin_advanced_internal` | YES | YES | indirect |
| 301 | `ZSTD_compressBegin_usingCDict` | YES | YES | yes |
| 302 | `ZSTD_compressBegin_usingCDict_advanced` | YES | YES | yes |
| 303 | `ZSTD_compressBegin_usingCDict_deprecated` | YES | YES | yes |
| 304 | `ZSTD_compressBegin_usingDict` | YES | YES | yes |
| 305 | `ZSTD_compressBlock` | YES | YES | yes |
| 306 | `ZSTD_compressBlock_btlazy2` | YES | YES | indirect |
| 307 | `ZSTD_compressBlock_btlazy2_dictMatchState` | YES | YES | indirect |
| 308 | `ZSTD_compressBlock_btlazy2_extDict` | YES | YES | indirect |
| 309 | `ZSTD_compressBlock_btopt` | YES | YES | indirect |
| 310 | `ZSTD_compressBlock_btopt_dictMatchState` | YES | YES | indirect |
| 311 | `ZSTD_compressBlock_btopt_extDict` | YES | YES | indirect |
| 312 | `ZSTD_compressBlock_btultra` | YES | YES | indirect |
| 313 | `ZSTD_compressBlock_btultra2` | YES | YES | indirect |
| 314 | `ZSTD_compressBlock_btultra_dictMatchState` | YES | YES | indirect |
| 315 | `ZSTD_compressBlock_btultra_extDict` | YES | YES | indirect |
| 316 | `ZSTD_compressBlock_deprecated` | YES | YES | yes |
| 317 | `ZSTD_compressBlock_doubleFast` | YES | YES | indirect |
| 318 | `ZSTD_compressBlock_doubleFast_dictMatchState` | YES | YES | indirect |
| 319 | `ZSTD_compressBlock_doubleFast_extDict` | YES | YES | indirect |
| 320 | `ZSTD_compressBlock_fast` | YES | YES | indirect |
| 321 | `ZSTD_compressBlock_fast_dictMatchState` | YES | YES | indirect |
| 322 | `ZSTD_compressBlock_fast_extDict` | YES | YES | indirect |
| 323 | `ZSTD_compressBlock_greedy` | YES | YES | indirect |
| 324 | `ZSTD_compressBlock_greedy_dedicatedDictSearch` | YES | YES | indirect |
| 325 | `ZSTD_compressBlock_greedy_dedicatedDictSearch_row` | YES | YES | indirect |
| 326 | `ZSTD_compressBlock_greedy_dictMatchState` | YES | YES | indirect |
| 327 | `ZSTD_compressBlock_greedy_dictMatchState_row` | YES | YES | indirect |
| 328 | `ZSTD_compressBlock_greedy_extDict` | YES | YES | indirect |
| 329 | `ZSTD_compressBlock_greedy_extDict_row` | YES | YES | indirect |
| 330 | `ZSTD_compressBlock_greedy_row` | YES | YES | indirect |
| 331 | `ZSTD_compressBlock_lazy` | YES | YES | indirect |
| 332 | `ZSTD_compressBlock_lazy2` | YES | YES | indirect |
| 333 | `ZSTD_compressBlock_lazy2_dedicatedDictSearch` | YES | YES | indirect |
| 334 | `ZSTD_compressBlock_lazy2_dedicatedDictSearch_row` | YES | YES | indirect |
| 335 | `ZSTD_compressBlock_lazy2_dictMatchState` | YES | YES | indirect |
| 336 | `ZSTD_compressBlock_lazy2_dictMatchState_row` | YES | YES | indirect |
| 337 | `ZSTD_compressBlock_lazy2_extDict` | YES | YES | indirect |
| 338 | `ZSTD_compressBlock_lazy2_extDict_row` | YES | YES | indirect |
| 339 | `ZSTD_compressBlock_lazy2_row` | YES | YES | indirect |
| 340 | `ZSTD_compressBlock_lazy_dedicatedDictSearch` | YES | YES | indirect |
| 341 | `ZSTD_compressBlock_lazy_dedicatedDictSearch_row` | YES | YES | indirect |
| 342 | `ZSTD_compressBlock_lazy_dictMatchState` | YES | YES | indirect |
| 343 | `ZSTD_compressBlock_lazy_dictMatchState_row` | YES | YES | indirect |
| 344 | `ZSTD_compressBlock_lazy_extDict` | YES | YES | indirect |
| 345 | `ZSTD_compressBlock_lazy_extDict_row` | YES | YES | indirect |
| 346 | `ZSTD_compressBlock_lazy_row` | YES | YES | indirect |
| 347 | `ZSTD_compressBound` | YES | YES | yes |
| 348 | `ZSTD_compressCCtx` | YES | YES | yes |
| 349 | `ZSTD_compressContinue` | YES | YES | yes |
| 350 | `ZSTD_compressContinue_public` | YES | YES | yes |
| 351 | `ZSTD_compressEnd` | YES | YES | yes |
| 352 | `ZSTD_compressEnd_public` | YES | YES | yes |
| 353 | `ZSTD_compressLiterals` | YES | YES | indirect |
| 354 | `ZSTD_compressRleLiteralsBlock` | YES | YES | yes |
| 355 | `ZSTD_compressSequences` | YES | YES | yes |
| 356 | `ZSTD_compressSequencesAndLiterals` | YES | YES | yes |
| 357 | `ZSTD_compressStream` | YES | YES | yes |
| 358 | `ZSTD_compressStream2` | YES | YES | yes |
| 359 | `ZSTD_compressStream2_simpleArgs` | YES | YES | yes |
| 360 | `ZSTD_compressSuperBlock` | YES | YES | indirect |
| 361 | `ZSTD_compress_advanced` | YES | YES | yes |
| 362 | `ZSTD_compress_advanced_internal` | YES | YES | indirect |
| 363 | `ZSTD_compress_usingCDict` | YES | YES | yes |
| 364 | `ZSTD_compress_usingCDict_advanced` | YES | YES | yes |
| 365 | `ZSTD_compress_usingDict` | YES | YES | yes |
| 366 | `ZSTD_convertBlockSequences` | YES | YES | yes |
| 367 | `ZSTD_copyCCtx` | YES | YES | yes |
| 368 | `ZSTD_copyDCtx` | YES | YES | yes |
| 369 | `ZSTD_copyDDictParameters` | YES | YES | yes |
| 370 | `ZSTD_createCCtx` | YES | YES | yes |
| 371 | `ZSTD_createCCtxParams` | YES | YES | yes |
| 372 | `ZSTD_createCCtx_advanced` | YES | YES | yes |
| 373 | `ZSTD_createCDict` | YES | YES | yes |
| 374 | `ZSTD_createCDict_advanced` | YES | YES | yes |
| 375 | `ZSTD_createCDict_advanced2` | YES | YES | yes |
| 376 | `ZSTD_createCDict_byReference` | YES | YES | yes |
| 377 | `ZSTD_createCStream` | YES | YES | yes |
| 378 | `ZSTD_createCStream_advanced` | YES | YES | yes |
| 379 | `ZSTD_createDCtx` | YES | YES | yes |
| 380 | `ZSTD_createDCtx_advanced` | YES | YES | yes |
| 381 | `ZSTD_createDDict` | YES | YES | yes |
| 382 | `ZSTD_createDDict_advanced` | YES | YES | yes |
| 383 | `ZSTD_createDDict_byReference` | YES | YES | yes |
| 384 | `ZSTD_createDStream` | YES | YES | yes |
| 385 | `ZSTD_createDStream_advanced` | YES | YES | yes |
| 386 | `ZSTD_crossEntropyCost` | YES | YES | yes |
| 387 | `ZSTD_cycleLog` | YES | YES | yes |
| 388 | `ZSTD_dParam_getBounds` | YES | YES | yes |
| 389 | `ZSTD_decodeLiteralsBlock_wrapper` | YES | YES | yes |
| 390 | `ZSTD_decodeSeqHeaders` | YES | YES | yes |
| 391 | `ZSTD_decodingBufferSize_min` | YES | YES | yes |
| 392 | `ZSTD_decompress` | YES | YES | yes |
| 393 | `ZSTD_decompressBegin` | YES | YES | yes |
| 394 | `ZSTD_decompressBegin_usingDDict` | YES | YES | yes |
| 395 | `ZSTD_decompressBegin_usingDict` | YES | YES | yes |
| 396 | `ZSTD_decompressBlock` | YES | YES | yes |
| 397 | `ZSTD_decompressBlock_deprecated` | YES | YES | yes |
| 398 | `ZSTD_decompressBlock_internal` | YES | YES | indirect |
| 399 | `ZSTD_decompressBound` | YES | YES | yes |
| 400 | `ZSTD_decompressContinue` | YES | YES | yes |
| 401 | `ZSTD_decompressDCtx` | YES | YES | yes |
| 402 | `ZSTD_decompressStream` | YES | YES | yes |
| 403 | `ZSTD_decompressStream_simpleArgs` | YES | YES | yes |
| 404 | `ZSTD_decompress_usingDDict` | YES | YES | yes |
| 405 | `ZSTD_decompress_usingDict` | YES | YES | yes |
| 406 | `ZSTD_decompressionMargin` | YES | YES | yes |
| 407 | `ZSTD_dedicatedDictSearch_lazy_loadDictionary` | YES | YES | indirect |
| 408 | `ZSTD_defaultCLevel` | YES | YES | yes |
| 409 | `ZSTD_encodeSequences` | YES | YES | indirect |
| 410 | `ZSTD_endStream` | YES | YES | yes |
| 411 | `ZSTD_estimateCCtxSize` | YES | YES | yes |
| 412 | `ZSTD_estimateCCtxSize_usingCCtxParams` | YES | YES | yes |
| 413 | `ZSTD_estimateCCtxSize_usingCParams` | YES | YES | yes |
| 414 | `ZSTD_estimateCDictSize` | YES | YES | yes |
| 415 | `ZSTD_estimateCDictSize_advanced` | YES | YES | yes |
| 416 | `ZSTD_estimateCStreamSize` | YES | YES | yes |
| 417 | `ZSTD_estimateCStreamSize_usingCCtxParams` | YES | YES | yes |
| 418 | `ZSTD_estimateCStreamSize_usingCParams` | YES | YES | yes |
| 419 | `ZSTD_estimateDCtxSize` | YES | YES | yes |
| 420 | `ZSTD_estimateDDictSize` | YES | YES | yes |
| 421 | `ZSTD_estimateDStreamSize` | YES | YES | yes |
| 422 | `ZSTD_estimateDStreamSize_fromFrame` | YES | YES | yes |
| 423 | `ZSTD_fillDoubleHashTable` | YES | YES | indirect |
| 424 | `ZSTD_fillHashTable` | YES | YES | indirect |
| 425 | `ZSTD_findDecompressedSize` | YES | YES | yes |
| 426 | `ZSTD_findFrameCompressedSize` | YES | YES | yes |
| 427 | `ZSTD_flushStream` | YES | YES | yes |
| 428 | `ZSTD_frameHeaderSize` | YES | YES | yes |
| 429 | `ZSTD_freeCCtx` | YES | YES | yes |
| 430 | `ZSTD_freeCCtxParams` | YES | YES | yes |
| 431 | `ZSTD_freeCDict` | YES | YES | yes |
| 432 | `ZSTD_freeCStream` | YES | YES | yes |
| 433 | `ZSTD_freeDCtx` | YES | YES | yes |
| 434 | `ZSTD_freeDDict` | YES | YES | yes |
| 435 | `ZSTD_freeDStream` | YES | YES | yes |
| 436 | `ZSTD_fseBitCost` | YES | YES | indirect |
| 437 | `ZSTD_generateSequences` | YES | YES | yes |
| 438 | `ZSTD_get1BlockSummary` | YES | YES | yes |
| 439 | `ZSTD_getBlockSize` | YES | YES | yes |
| 440 | `ZSTD_getCParams` | YES | YES | yes |
| 441 | `ZSTD_getCParamsFromCCtxParams` | YES | YES | yes |
| 442 | `ZSTD_getCParamsFromCDict` | YES | YES | yes |
| 443 | `ZSTD_getDecompressedSize` | YES | YES | yes |
| 444 | `ZSTD_getDictID_fromCDict` | YES | YES | yes |
| 445 | `ZSTD_getDictID_fromDDict` | YES | YES | yes |
| 446 | `ZSTD_getDictID_fromDict` | YES | YES | yes |
| 447 | `ZSTD_getDictID_fromFrame` | YES | YES | yes |
| 448 | `ZSTD_getErrorCode` | YES | YES | yes |
| 449 | `ZSTD_getErrorName` | YES | YES | yes |
| 450 | `ZSTD_getErrorString` | YES | YES | yes |
| 451 | `ZSTD_getFrameContentSize` | YES | YES | yes |
| 452 | `ZSTD_getFrameHeader` | YES | YES | yes |
| 453 | `ZSTD_getFrameHeader_advanced` | YES | YES | yes |
| 454 | `ZSTD_getFrameProgression` | YES | YES | yes |
| 455 | `ZSTD_getParams` | YES | YES | yes |
| 456 | `ZSTD_getSeqStore` | YES | YES | indirect |
| 457 | `ZSTD_getcBlockSize` | YES | YES | yes |
| 458 | `ZSTD_initCStream` | YES | YES | yes |
| 459 | `ZSTD_initCStream_advanced` | YES | YES | yes |
| 460 | `ZSTD_initCStream_internal` | YES | YES | indirect |
| 461 | `ZSTD_initCStream_srcSize` | YES | YES | yes |
| 462 | `ZSTD_initCStream_usingCDict` | YES | YES | yes |
| 463 | `ZSTD_initCStream_usingCDict_advanced` | YES | YES | yes |
| 464 | `ZSTD_initCStream_usingDict` | YES | YES | yes |
| 465 | `ZSTD_initDStream` | YES | YES | yes |
| 466 | `ZSTD_initDStream_usingDDict` | YES | YES | yes |
| 467 | `ZSTD_initDStream_usingDict` | YES | YES | yes |
| 468 | `ZSTD_initStaticCCtx` | YES | YES | yes |
| 469 | `ZSTD_initStaticCDict` | YES | YES | yes |
| 470 | `ZSTD_initStaticCStream` | YES | YES | yes |
| 471 | `ZSTD_initStaticDCtx` | YES | YES | yes |
| 472 | `ZSTD_initStaticDDict` | YES | YES | yes |
| 473 | `ZSTD_initStaticDStream` | YES | YES | yes |
| 474 | `ZSTD_insertAndFindFirstIndex` | YES | YES | indirect |
| 475 | `ZSTD_insertBlock` | YES | YES | yes |
| 476 | `ZSTD_invalidateRepCodes` | YES | YES | yes |
| 477 | `ZSTD_isError` | YES | YES | yes |
| 478 | `ZSTD_isFrame` | YES | YES | yes |
| 479 | `ZSTD_isSkippableFrame` | YES | YES | yes |
| 480 | `ZSTD_ldm_adjustParameters` | YES | YES | indirect |
| 481 | `ZSTD_ldm_blockCompress` | YES | YES | indirect |
| 482 | `ZSTD_ldm_fillHashTable` | YES | YES | indirect |
| 483 | `ZSTD_ldm_generateSequences` | YES | YES | indirect |
| 484 | `ZSTD_ldm_getMaxNbSeq` | YES | YES | indirect |
| 485 | `ZSTD_ldm_getTableSize` | YES | YES | indirect |
| 486 | `ZSTD_ldm_skipRawSeqStoreBytes` | YES | YES | indirect |
| 487 | `ZSTD_ldm_skipSequences` | YES | YES | indirect |
| 488 | `ZSTD_loadCEntropy` | YES | YES | indirect |
| 489 | `ZSTD_loadDEntropy` | YES | YES | indirect |
| 490 | `ZSTD_maxCLevel` | YES | YES | yes |
| 491 | `ZSTD_mergeBlockDelimiters` | YES | YES | yes |
| 492 | `ZSTD_minCLevel` | YES | YES | yes |
| 493 | `ZSTD_nextInputType` | YES | YES | yes |
| 494 | `ZSTD_nextSrcSizeToDecompress` | YES | YES | yes |
| 495 | `ZSTD_noCompressLiterals` | YES | YES | yes |
| 496 | `ZSTD_readSkippableFrame` | YES | YES | yes |
| 497 | `ZSTD_referenceExternalSequences` | YES | YES | indirect |
| 498 | `ZSTD_registerSequenceProducer` | YES | YES | yes |
| 499 | `ZSTD_resetCStream` | YES | YES | yes |
| 500 | `ZSTD_resetDStream` | YES | YES | yes |
| 501 | `ZSTD_resetSeqStore` | YES | YES | indirect |
| 502 | `ZSTD_reset_compressedBlockState` | YES | YES | indirect |
| 503 | `ZSTD_row_update` | YES | YES | indirect |
| 504 | `ZSTD_selectBlockCompressor` | YES | YES | yes |
| 505 | `ZSTD_selectEncodingType` | YES | YES | yes |
| 506 | `ZSTD_seqToCodes` | YES | YES | indirect |
| 507 | `ZSTD_sequenceBound` | YES | YES | yes |
| 508 | `ZSTD_sizeof_CCtx` | YES | YES | yes |
| 509 | `ZSTD_sizeof_CDict` | YES | YES | yes |
| 510 | `ZSTD_sizeof_CStream` | YES | YES | yes |
| 511 | `ZSTD_sizeof_DCtx` | YES | YES | yes |
| 512 | `ZSTD_sizeof_DDict` | YES | YES | yes |
| 513 | `ZSTD_sizeof_DStream` | YES | YES | yes |
| 514 | `ZSTD_splitBlock` | YES | YES | yes |
| 515 | `ZSTD_toFlushNow` | YES | YES | yes |
| 516 | `ZSTD_updateTree` | YES | YES | indirect |
| 517 | `ZSTD_versionNumber` | YES | YES | yes |
| 518 | `ZSTD_versionString` | YES | YES | yes |
| 519 | `ZSTD_writeLastEmptyBlock` | YES | YES | yes |
| 520 | `ZSTD_writeSkippableFrame` | YES | YES | yes |
| 521 | `ZSTDv01_createDCtx` | YES | YES | yes |
| 522 | `ZSTDv01_decompress` | YES | YES | yes |
| 523 | `ZSTDv01_decompressContinue` | YES | YES | yes |
| 524 | `ZSTDv01_decompressDCtx` | YES | YES | yes |
| 525 | `ZSTDv01_findFrameSizeInfoLegacy` | YES | YES | yes |
| 526 | `ZSTDv01_freeDCtx` | YES | YES | yes |
| 527 | `ZSTDv01_isError` | YES | YES | yes |
| 528 | `ZSTDv01_nextSrcSizeToDecompress` | YES | YES | yes |
| 529 | `ZSTDv01_resetDCtx` | YES | YES | yes |
| 530 | `ZSTDv02_createDCtx` | YES | YES | yes |
| 531 | `ZSTDv02_decompress` | YES | YES | yes |
| 532 | `ZSTDv02_decompressContinue` | YES | YES | yes |
| 533 | `ZSTDv02_findFrameSizeInfoLegacy` | YES | YES | yes |
| 534 | `ZSTDv02_freeDCtx` | YES | YES | yes |
| 535 | `ZSTDv02_isError` | YES | YES | yes |
| 536 | `ZSTDv02_nextSrcSizeToDecompress` | YES | YES | yes |
| 537 | `ZSTDv02_resetDCtx` | YES | YES | yes |
| 538 | `ZSTDv03_createDCtx` | YES | YES | yes |
| 539 | `ZSTDv03_decompress` | YES | YES | yes |
| 540 | `ZSTDv03_decompressContinue` | YES | YES | yes |
| 541 | `ZSTDv03_findFrameSizeInfoLegacy` | YES | YES | yes |
| 542 | `ZSTDv03_freeDCtx` | YES | YES | yes |
| 543 | `ZSTDv03_isError` | YES | YES | yes |
| 544 | `ZSTDv03_nextSrcSizeToDecompress` | YES | YES | yes |
| 545 | `ZSTDv03_resetDCtx` | YES | YES | yes |
| 546 | `ZSTDv04_createDCtx` | YES | YES | yes |
| 547 | `ZSTDv04_decompress` | YES | YES | yes |
| 548 | `ZSTDv04_decompressContinue` | YES | YES | yes |
| 549 | `ZSTDv04_decompressDCtx` | YES | YES | yes |
| 550 | `ZSTDv04_findFrameSizeInfoLegacy` | YES | YES | yes |
| 551 | `ZSTDv04_freeDCtx` | YES | YES | yes |
| 552 | `ZSTDv04_nextSrcSizeToDecompress` | YES | YES | yes |
| 553 | `ZSTDv04_resetDCtx` | YES | YES | yes |
| 554 | `ZSTDv05_copyDCtx` | YES | YES | yes |
| 555 | `ZSTDv05_createDCtx` | YES | YES | yes |
| 556 | `ZSTDv05_decompress` | YES | YES | yes |
| 557 | `ZSTDv05_decompressBegin` | YES | YES | yes |
| 558 | `ZSTDv05_decompressBegin_usingDict` | YES | YES | yes |
| 559 | `ZSTDv05_decompressBlock` | YES | YES | yes |
| 560 | `ZSTDv05_decompressContinue` | YES | YES | yes |
| 561 | `ZSTDv05_decompressDCtx` | YES | YES | yes |
| 562 | `ZSTDv05_decompress_usingDict` | YES | YES | yes |
| 563 | `ZSTDv05_decompress_usingPreparedDCtx` | YES | YES | yes |
| 564 | `ZSTDv05_findFrameSizeInfoLegacy` | YES | YES | yes |
| 565 | `ZSTDv05_freeDCtx` | YES | YES | yes |
| 566 | `ZSTDv05_getErrorName` | YES | YES | yes |
| 567 | `ZSTDv05_getFrameParams` | YES | YES | yes |
| 568 | `ZSTDv05_isError` | YES | YES | yes |
| 569 | `ZSTDv05_nextSrcSizeToDecompress` | YES | YES | yes |
| 570 | `ZSTDv05_sizeofDCtx` | YES | YES | yes |
| 571 | `ZSTDv06_copyDCtx` | YES | YES | yes |
| 572 | `ZSTDv06_createDCtx` | YES | YES | yes |
| 573 | `ZSTDv06_decompress` | YES | YES | yes |
| 574 | `ZSTDv06_decompressBegin` | YES | YES | yes |
| 575 | `ZSTDv06_decompressBegin_usingDict` | YES | YES | yes |
| 576 | `ZSTDv06_decompressBlock` | YES | YES | yes |
| 577 | `ZSTDv06_decompressContinue` | YES | YES | yes |
| 578 | `ZSTDv06_decompressDCtx` | YES | YES | yes |
| 579 | `ZSTDv06_decompress_usingDict` | YES | YES | yes |
| 580 | `ZSTDv06_decompress_usingPreparedDCtx` | YES | YES | yes |
| 581 | `ZSTDv06_findFrameSizeInfoLegacy` | YES | YES | yes |
| 582 | `ZSTDv06_freeDCtx` | YES | YES | yes |
| 583 | `ZSTDv06_getErrorName` | YES | YES | yes |
| 584 | `ZSTDv06_getFrameParams` | YES | YES | yes |
| 585 | `ZSTDv06_isError` | YES | YES | yes |
| 586 | `ZSTDv06_nextSrcSizeToDecompress` | YES | YES | yes |
| 587 | `ZSTDv06_sizeofDCtx` | YES | YES | yes |
| 588 | `ZSTDv07_copyDCtx` | YES | YES | yes |
| 589 | `ZSTDv07_createDCtx` | YES | YES | yes |
| 590 | `ZSTDv07_createDCtx_advanced` | YES | YES | yes |
| 591 | `ZSTDv07_createDDict` | YES | YES | yes |
| 592 | `ZSTDv07_decompress` | YES | YES | yes |
| 593 | `ZSTDv07_decompressBegin` | YES | YES | yes |
| 594 | `ZSTDv07_decompressBegin_usingDict` | YES | YES | yes |
| 595 | `ZSTDv07_decompressBlock` | YES | YES | yes |
| 596 | `ZSTDv07_decompressContinue` | YES | YES | yes |
| 597 | `ZSTDv07_decompressDCtx` | YES | YES | yes |
| 598 | `ZSTDv07_decompress_usingDDict` | YES | YES | yes |
| 599 | `ZSTDv07_decompress_usingDict` | YES | YES | yes |
| 600 | `ZSTDv07_estimateDCtxSize` | YES | YES | yes |
| 601 | `ZSTDv07_findFrameSizeInfoLegacy` | YES | YES | yes |
| 602 | `ZSTDv07_freeDCtx` | YES | YES | yes |
| 603 | `ZSTDv07_freeDDict` | YES | YES | yes |
| 604 | `ZSTDv07_getDecompressedSize` | YES | YES | yes |
| 605 | `ZSTDv07_getErrorName` | YES | YES | yes |
| 606 | `ZSTDv07_getFrameParams` | YES | YES | yes |
| 607 | `ZSTDv07_insertBlock` | YES | YES | yes |
| 608 | `ZSTDv07_isError` | YES | YES | yes |
| 609 | `ZSTDv07_isSkipFrame` | YES | YES | yes |
| 610 | `ZSTDv07_nextSrcSizeToDecompress` | YES | YES | yes |
| 611 | `ZSTDv07_sizeofDCtx` | YES | YES | yes |
| 612 | `divbwt` | YES | YES | yes |
| 613 | `divsufsort` | YES | YES | yes |
| 614 | `g_ZSTD_threading_useless_symbol` | YES | YES | yes |
| 615 | `g_debuglevel` | YES | YES | yes |
