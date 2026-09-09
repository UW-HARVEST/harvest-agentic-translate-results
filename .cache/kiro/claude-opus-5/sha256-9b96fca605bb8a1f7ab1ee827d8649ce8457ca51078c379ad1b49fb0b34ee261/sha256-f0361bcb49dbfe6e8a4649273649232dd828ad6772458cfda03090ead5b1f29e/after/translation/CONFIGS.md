# CONFIGS.md — Configuration-surface table (valid inputs)

Mirror of `ERRORS.md` for VALID inputs. Axes are derived mechanically from the
branches the C source actually takes, not from a guess at what matters.

## Axes extracted from the C

**Runtime option axes** (`ZSTD_cParam_getBounds` in `c_src/src/compress/zstd_compress.c`,
39 cases; `ZSTD_dParam_getBounds` in `c_src/src/decompress/zstd_decompress.c`, 8 cases):

| axis | values the C branches on | source |
|---|---|---|
| `ZSTD_c_compressionLevel` | `minCLevel()..=maxCLevel()` (−131072..22), incl. 0 ⇒ default | `clevels.h` table lookup |
| `ZSTD_c_strategy` | `ZSTD_fast(1) dfast(2) greedy(3) lazy(4) lazy2(5) btlazy2(6) btopt(7) btultra(8) btultra2(9)` | dispatch in `ZSTD_compressBlock_internal`; 9 distinct match finders |
| `ZSTD_c_windowLog` | `10..=31`, and `forceMaxWindow` | `zstd_compress.c` |
| `ZSTD_c_hashLog` / `chainLog` / `searchLog` | bounds from `getBounds` | table sizing in `zstd_cwksp.h` |
| `ZSTD_c_minMatch` | `3..=7` (mls dispatch: 4/5/6/7 template instantiations) | `ZSTD_selectBlockCompressor` |
| `ZSTD_c_targetLength` | `0..=131072` | `zstd_opt.c` |
| `ZSTD_c_contentSizeFlag` | 0 / 1 → frame header shape | `ZSTD_writeFrameHeader` |
| `ZSTD_c_checksumFlag` | 0 / 1 → 4-byte XXH64 trailer | `ZSTD_compressEnd` |
| `ZSTD_c_dictIDFlag` | 0 / 1 → dictID in header | `ZSTD_writeFrameHeader` |
| `ZSTD_c_enableLongDistanceMatching` | `auto(0) enable(1) disable(2)` | `zstd_ldm.c` on/off |
| `ZSTD_c_ldmHashLog/ldmMinMatch/ldmBucketSizeLog/ldmHashRateLog` | bounds from `getBounds` | `zstd_ldm.c` |
| `ZSTD_c_format` | `ZSTD_f_zstd1(0)`, `ZSTD_f_zstd1_magicless(1)` | magic emitted or not |
| `ZSTD_c_literalCompressionMode` | `auto / enable / disable` | `zstd_compress_literals.c` raw vs huff |
| `ZSTD_c_targetCBlockSize` | `0` (off) or `1340..=131072` (superblock path) | `zstd_compress_superblock.c` |
| `ZSTD_c_blockSplitterLevel` | `0..=6` | `zstd_preSplit.c` |
| `ZSTD_c_splitAfterSequences` | `auto / enable / disable` | post-block-split path |
| `ZSTD_c_useRowMatchFinder` | `auto / enable / disable` | `zstd_lazy.c` row-hash vs chain finder |
| `ZSTD_c_maxBlockSize` | `1024..=131072` | block loop bound |
| `ZSTD_c_srcSizeHint` | `0`, small, large | parameter adaptation |
| `ZSTD_c_forceAttachDict` | `dictDefaultAttach / dictForceAttach / dictForceCopy / dictForceLoad` | `ZSTD_resetCCtx_usingCDict` |
| `ZSTD_c_enableDedicatedDictSearch` | 0 / 1 | DDS table build |
| `ZSTD_c_stableInBuffer` / `stableOutBuffer` | `buffered / stable` | `ZSTD_compressStream2` buffer handling |
| `ZSTD_c_blockDelimiters` | `noBlockDelimiters / explicitBlockDelimiters` | `ZSTD_compressSequences` |
| `ZSTD_c_validateSequences` | 0 / 1 | sequence validation |
| `ZSTD_c_repcodeResolution` | `auto / enable / disable` | sequence repcode search |
| `ZSTD_c_prefetchCDictTables` | `auto / enable / disable` | CDict prefetch |
| `ZSTD_c_deterministicRefPrefix` | 0 / 1 | `refPrefix` behaviour |
| `ZSTD_c_rsyncable` / `nbWorkers` / `jobSize` / `overlapLog` | MT off in this build (`upperBound = 0`) | `ZSTD_MULTITHREAD` not defined |
| `ZSTD_d_windowLogMax` | `10..=31` | decode window check |
| `ZSTD_d_format` | `zstd1 / zstd1_magicless` | must mirror encoder |
| `ZSTD_d_stableOutBuffer` | `buffered / stable` | `ZSTD_decompressStream` |
| `ZSTD_d_forceIgnoreChecksum` | `validateChecksum / ignoreChecksum` | trailer check |
| `ZSTD_d_refMultipleDDicts` | `refSingleDDict / refMultipleDDicts` | DDict hash set |
| `ZSTD_d_disableHuffmanAssembly` | 0 / 1 | `huf_decompress.c` path |
| `ZSTD_d_maxBlockSize` | `1024..=131072` | decode buffer sizing |

**Input-shape axes** the C special-cases:

| shape | values | why the C distinguishes it |
|---|---|---|
| size | `0, 1, 2, 3, 4, 7, 8, 15, 16, 31, 63, 127, 128, 255, 256, 1023, 1024, 4096, 65535, 65536, 131071, 131072, 131073, 300000` | `< MINMATCH`, single-block vs multi-block at `ZSTD_BLOCKSIZE_MAX`, `HUF_BLOCKSIZE_MAX` |
| entropy | all-zeros, single repeated byte, 2-symbol alphabet, 256-symbol uniform random, text-like, highly repetitive, incompressible random | RLE block / raw block / huffman-compressed literals / FSE table modes (`set_basic/set_rle/set_compressed/set_repeat`) |
| dictionary | none, raw-content prefix, `ZDICT_trainFromBuffer` dict, `ZDICT_finalizeDictionary` dict, cover dict, fastcover dict | `ZSTD_dct_auto/rawContent/fullDict` |
| streaming chunking | one-shot, 1-byte chunks, 7-byte chunks, `CStreamInSize()` chunks, uneven chunk sequence | `ZSTD_compressStream2` / `ZSTD_decompressStream` state machine |
| byte order / width | `MEM_read*`/`MEM_write*` LE helpers, 32-bit vs 64-bit `size_t` | `mem.h` |

## The table

Test file: `translation/tests/phase_b_configs.rs`.
Every row calls BOTH `.so` files through `libloading` with **many randomized
inputs** (fixed seed `0x5EED_2718`, ≥16 payloads per row unless noted) and
asserts byte-identical output buffers, return codes, and any written-back
out-params.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| 1 | `ZSTD_versionNumber`, `ZSTD_versionString`, `ZSTD_minCLevel`, `ZSTD_maxCLevel`, `ZSTD_defaultCLevel`, `ZSTD_compressBound`, `ZSTD_CStreamInSize`, `ZSTD_CStreamOutSize`, `ZSTD_DStreamInSize`, `ZSTD_DStreamOutSize` | pure constants + `compressBound` over the full size axis incl. `SIZE_MAX` | [x] |
| 2 | `ZSTD_XXH32`, `ZSTD_XXH32_reset/update/digest/copyState`, `ZSTD_XXH32_canonicalFromHash`, `ZSTD_XXH32_hashFromCanonical` | seeds `{0,1,0xDEADBEEF}` × sizes `0..=1024` × chunked and one-shot | [x] |
| 3 | `ZSTD_XXH64`, `ZSTD_XXH64_reset/update/digest/copyState`, canonical helpers | seeds `{0,1,u64::MAX}` × sizes `0..=1024` × chunked and one-shot | [x] |
| 4 | `ZSTD_XXH_versionNumber` | XXH3/XXH128 are **not exported** by this build: `nm -D` on the C `.so` shows only `ZSTD_XXH32*`, `ZSTD_XXH64*` and `ZSTD_XXH_versionNumber` (`xxhash.c` keeps XXH3 `static`). Row verifies the version accessor and asserts XXH3 is absent from BOTH `.so` files. | [x] |
| 5 | `HIST_count`, `HIST_count_simple`, `HIST_count_wksp`, `HIST_isError` | all 7 entropy shapes × sizes over the size axis × `maxSymbolValue` in `{15,63,127,255}` | [x] |
| 6 | `FSE_normalizeCount`, `FSE_writeNCount`, `FSE_readNCount`, `FSE_readNCount_bmi2`, `FSE_optimalTableLog`, `FSE_optimalTableLog_internal`, `FSE_NCountWriteBound` | randomized count tables × `tableLog` `5..=12` × `maxSymbolValue` `1..=255` (round-trip write→read) | [x] |
| 7 | `FSE_buildCTable_wksp`, `FSE_buildCTable_rle`, `FSE_buildCTable_raw`, `FSE_compress_usingCTable`, `FSE_compress2`, `FSE_compress_wksp`, `FSE_compressBound` | all entropy shapes × `tableLog` `5..=12` × `maxSymbolValue` `{15,63,255}` | [x] |
| 8 | `FSE_buildDTable_wksp`, `FSE_buildDTable_rle`, `FSE_buildDTable_raw`, `FSE_decompress_usingDTable`, `FSE_decompress_wksp`, `FSE_decompress_wksp_bmi2`, `FSE_getErrorName` | decode of every buffer produced in row 7 (round-trip) | [x] |
| 9 | `HUF_compress`, `HUF_compress1X`, `HUF_compress1X_wksp`, `HUF_compress1X_repeat`, `HUF_compress1X_usingCTable`, `HUF_compress1X_usingCTable_bmi2` | 1-stream mode × all entropy shapes × sizes `1..=HUF_BLOCKSIZE_MAX` × `huffLog` `5..=12` | [x] |
| 10 | `HUF_compress2`, `HUF_compress4X_wksp`, `HUF_compress4X_repeat`, `HUF_compress4X_usingCTable`, `HUF_compress4X_usingCTable_bmi2`, `HUF_compressBound` | 4-stream mode × all entropy shapes × sizes × `huffLog` `5..=12`; `repeat` = `HUF_repeat_none/check/valid` | [x] |
| 11 | `HUF_buildCTable`, `HUF_buildCTable_wksp`, `HUF_writeCTable`, `HUF_writeCTable_wksp`, `HUF_readCTable`, `HUF_estimateCompressedSize`, `HUF_validateCTable`, `HUF_getNbBitsFromCTable`, `HUF_optimalTableLog`, `HUF_cardinality`, `HUF_minTableLog`, `HUF_readStats`, `HUF_readStats_wksp` | randomized histograms × `maxSymbolValue` `{15,63,255}` × `tableLog` `5..=12` | [x] |
| 12 | `HUF_decompress`, `HUF_decompress1X1`, `HUF_decompress1X2`, `HUF_decompress4X1`, `HUF_decompress4X2`, `_DCtx`, `_DCtx_wksp`, `_usingDTable*`, `HUF_readDTableX1/X2(_wksp)`, `HUF_selectDecoder`, `HUF_decompress1X_DCtx_wksp`, `HUF_decompress4X_hufOnly_wksp`, `HUF_decompress1X_usingDTable`, `HUF_decompress4X_usingDTable` | decode of every buffer from rows 9–10, both X1 (single-symbol) and X2 (double-symbol) decoders, `bmi2` flag 0/1 | [x] |
| 13 | `ZSTD_compress`, `ZSTD_decompress` (simple one-shot) | levels `minCLevel..=maxCLevel` (all, incl. negative and 0) × all 7 entropy shapes × full size axis | [x] |
| 14 | `ZSTD_compressCCtx`, `ZSTD_decompressDCtx` | same as row 13, with context reuse across iterations (no reset) | [x] |
| 15 | `ZSTD_compress2` + `ZSTD_CCtx_setParameter(strategy)` | each of the 9 strategies × entropy shapes × sizes; `windowLog`/`hashLog`/`chainLog`/`searchLog` at their bounds for each strategy | [x] |
| 16 | `ZSTD_compress2` + `ZSTD_c_minMatch` | `minMatch` `3..=7` × each strategy that accepts it × entropy shapes (exercises every mls template) | [x] |
| 17 | `ZSTD_compress2` + `ZSTD_c_useRowMatchFinder` | `auto/enable/disable` × strategies `greedy..btlazy2` × sizes (row-hash vs chain-table finders) | [x] |
| 18 | `ZSTD_compress2` + `ZSTD_c_enableLongDistanceMatching` | `auto/enable/disable` × `ldmHashLog`/`ldmMinMatch`/`ldmBucketSizeLog`/`ldmHashRateLog` at min/mid/max × long repetitive inputs (≥256 KB) | [x] |
| 19 | `ZSTD_compress2` + `ZSTD_c_literalCompressionMode` | `auto/enable/disable` × entropy shapes (raw vs huffman literal blocks) | [x] |
| 20 | `ZSTD_compress2` + `ZSTD_c_targetCBlockSize` | `0`, `ZSTD_TARGETCBLOCKSIZE_MIN`, mid, `ZSTD_TARGETCBLOCKSIZE_MAX` × sizes ≥ 128 KB (superblock path) | [x] |
| 21 | `ZSTD_compress2` + `ZSTD_c_blockSplitterLevel` and `ZSTD_c_splitAfterSequences` | splitter level `0..=6` × `splitAfterSequences` `auto/enable/disable` × mixed-entropy inputs (entropy changes mid-buffer) | [x] |
| 22 | `ZSTD_compress2` + `ZSTD_c_maxBlockSize` | `1024`, `4096`, `65536`, `131072` × sizes spanning multiple blocks | [x] |
| 23 | `ZSTD_compress2` + frame-header flags | `contentSizeFlag` × `checksumFlag` × `dictIDFlag` (2³ combos) × with/without `ZSTD_CCtx_setPledgedSrcSize` | [x] |
| 24 | `ZSTD_compress2` / `ZSTD_decompressDCtx` + `ZSTD_c_format` / `ZSTD_d_format` | `zstd1` and `zstd1_magicless`, encoder and decoder matched | [x] |
| 25 | `ZSTD_compress2` + `ZSTD_c_windowLog` + `ZSTD_c_forceMaxWindow`; decoder `ZSTD_d_windowLogMax` | `windowLog` `10..=27` × `forceMaxWindow` 0/1 × inputs larger and smaller than the window | [x] |
| 26 | `ZSTD_compress2` + `ZSTD_c_srcSizeHint`, `ZSTD_c_deterministicRefPrefix` | hint `{0, exact, 10×, 1/10×}` × `deterministicRefPrefix` 0/1 | [x] |
| 27 | `ZSTD_compressStream`, `ZSTD_flushStream`, `ZSTD_endStream`, `ZSTD_initCStream` | levels × chunk patterns (1-byte, 7-byte, `CStreamInSize`, uneven) × entropy shapes | [x] |
| 28 | `ZSTD_compressStream2` with all `ZSTD_EndDirective` values | `ZSTD_e_continue` / `ZSTD_e_flush` / `ZSTD_e_end` interleaved × chunk patterns × restricted output buffer sizes (1, 7, 1024, `CStreamOutSize`) | [x] |
| 29 | `ZSTD_compressStream2_simpleArgs`, `ZSTD_decompressStream_simpleArgs` | same chunking axis, verifying returned `srcPos`/`dstPos` | [x] |
| 30 | `ZSTD_compressStream2` + `ZSTD_c_stableInBuffer` / `stableOutBuffer` | `buffered/stable` × `stable/buffered` (4 combos) respecting the stability contract | [x] |
| 31 | `ZSTD_decompressStream`, `ZSTD_initDStream`, `ZSTD_resetDStream`, `ZSTD_nextSrcSizeToDecompress`, `ZSTD_DStreamInSize` | decode of every stream from rows 27–30 × output chunk patterns × `ZSTD_d_stableOutBuffer` 0/1 | [x] |
| 32 | `ZSTD_decompressStream` + `ZSTD_d_forceIgnoreChecksum`, `ZSTD_d_disableHuffmanAssembly`, `ZSTD_d_maxBlockSize` | checksum validate/ignore × huffman-assembly 0/1 × `maxBlockSize` `{1024,65536,131072}` | [x] |
| 33 | `ZSTD_CCtx_reset`, `ZSTD_DCtx_reset`, `ZSTD_CCtx_setParametersUsingCCtxParams` | all 3 `ZSTD_ResetDirective` values × reuse of a context across a sequence of different configurations | [x] |
| 34 | `ZSTD_createCCtxParams`, `ZSTD_CCtxParams_init`, `ZSTD_CCtxParams_init_advanced`, `ZSTD_CCtxParams_setParameter`, `ZSTD_CCtxParams_getParameter`, `ZSTD_CCtxParams_reset`, `ZSTD_freeCCtxParams` | every valid param at lower/mid/upper bound, applied via params object then compressed | [x] |
| 35 | `ZSTD_getParams`, `ZSTD_getCParams`, `ZSTD_adjustCParams`, `ZSTD_checkCParams` | levels `minCLevel..=maxCLevel` × `srcSizeHint` `{0,1,1KB,1MB,1GB,unknown}` × `dictSize` `{0,1KB,1MB}` — all struct fields compared | [x] |
| 36 | `ZSTD_compress_advanced`, `ZSTD_compress_usingDict`, `ZSTD_compress_usingCDict`, `ZSTD_compress_usingCDict_advanced` | `ZSTD_parameters` from row 35 × dict shapes × entropy shapes | [x] |
| 37 | `ZSTD_createCDict`, `ZSTD_createCDict_advanced`, `ZSTD_createCDict_advanced2`, `ZSTD_sizeof_CDict`, `ZSTD_getDictID_fromCDict`, `ZSTD_freeCDict` | `ZSTD_dlm_byCopy/byRef` × `ZSTD_dct_auto/rawContent/fullDict` × levels × dict shapes | [x] |
| 38 | `ZSTD_CCtx_refCDict` + `ZSTD_c_forceAttachDict` | `dictDefaultAttach/dictForceAttach/dictForceCopy/dictForceLoad` × `ZSTD_c_prefetchCDictTables` `auto/enable/disable` × src sizes (attach vs copy heuristic depends on size) | [x] |
| 39 | `ZSTD_CCtx_loadDictionary`, `_byReference`, `_advanced`, `ZSTD_CCtx_refPrefix`, `_advanced` | all load-method × content-type combos × dict shapes | [x] |
| 40 | `ZSTD_createDDict`, `_byReference`, `_advanced`, `ZSTD_initStaticDDict`, `ZSTD_sizeof_DDict`, `ZSTD_getDictID_fromDDict`, `ZSTD_freeDDict`, `ZSTD_DCtx_refDDict`, `ZSTD_DCtx_loadDictionary*`, `ZSTD_DCtx_refPrefix*` | mirror of rows 37/39 on the decode side × `ZSTD_d_refMultipleDDicts` single/multiple | [x] |
| 41 | `ZSTD_decompress_usingDict`, `ZSTD_decompress_usingDDict`, `ZSTD_getDictID_fromDict`, `ZSTD_getDictID_fromFrame` | decode of every dict-compressed frame from rows 36–39 | [x] |
| 42 | `ZSTD_initCStream_usingDict`, `_advanced`, `_usingCDict`, `_usingCDict_advanced`, `_srcSize`, `ZSTD_initDStream_usingDict`, `_usingDDict`, `ZSTD_resetCStream` | legacy streaming init variants × dict shapes × levels | [x] |
| 43 | `ZSTD_compressBegin`, `_usingDict`, `_advanced`, `_usingCDict`, `_usingCDict_advanced`, `_srcSize`, `ZSTD_copyCCtx`, `ZSTD_compressContinue`, `ZSTD_compressEnd`, `ZSTD_getBlockSize` | **lowest-level block API**: manual frame construction, block sizes `1..=ZSTD_getBlockSize()`, `copyCCtx` mid-stream | [x] |
| 44 | `ZSTD_compressBlock`, `ZSTD_decompressBlock`, `ZSTD_insertBlock`, `ZSTD_decompressBegin`, `_usingDict`, `_usingDDict`, `ZSTD_copyDCtx`, `ZSTD_decompressContinue`, `ZSTD_nextSrcSizeToDecompress`, `ZSTD_nextInputType` | **raw block API**: no frame header; block sizes across the size axis × entropy shapes × dict prefix | [x] |
| 45 | `ZSTD_getFrameHeader`, `_advanced`, `ZSTD_frameHeaderSize`, `ZSTD_getFrameContentSize`, `ZSTD_getDecompressedSize`, `ZSTD_findDecompressedSize`, `ZSTD_findFrameCompressedSize`, `ZSTD_decompressBound`, `ZSTD_isFrame` | every frame from rows 13–30, all header shapes (contentSize 0/1/2/8-byte encodings, dictID 0/1/2/4-byte, single-segment flag) | [x] |
| 46 | `ZSTD_decompressMultiFrame` path: `ZSTD_decompress` over concatenated frames; `ZSTD_writeSkippableFrame`, `ZSTD_readSkippableFrame`, `ZSTD_isSkippableFrame` | 1/2/5 concatenated frames × skippable frames interleaved × `magicVariant` `0..=15` | [x] |
| 47 | `ZSTD_estimateCCtxSize`, `_usingCParams`, `_usingCCtxParams`, `ZSTD_estimateCStreamSize*`, `ZSTD_estimateDCtxSize`, `ZSTD_estimateDStreamSize`, `_fromFrame`, `ZSTD_estimateCDictSize`, `_advanced`, `ZSTD_estimateDDictSize` | levels × cParams from row 35 × dict sizes | [x] |
| 48 | `ZSTD_initStaticCCtx`, `ZSTD_initStaticCStream`, `ZSTD_initStaticDCtx`, `ZSTD_initStaticDStream`, `ZSTD_initStaticCDict` | workspace exactly `estimate*Size()`, and larger; then a full compress/decompress round trip through the static context | [x] |
| 49 | `ZSTD_customMem` variants: `ZSTD_createCCtx_advanced`, `ZSTD_createCStream_advanced`, `ZSTD_createDCtx_advanced`, `ZSTD_createDStream_advanced`, `ZSTD_createCDict_advanced`, `ZSTD_createDDict_advanced`, `ZSTD_createCCtxParams_advanced` | custom malloc/free callbacks (counted) × null customMem × round trip | [x] |
| 50 | `ZSTD_generateSequences`, `ZSTD_mergeBlockDelimiters`, `ZSTD_sequenceBound`, `ZSTD_getSequences` | levels × entropy shapes × sizes; sequence arrays compared element-by-element | [x] |
| 51 | `ZSTD_compressSequences` + `ZSTD_c_blockDelimiters` + `ZSTD_c_validateSequences` + `ZSTD_c_repcodeResolution` | `noBlockDelimiters/explicitBlockDelimiters` × `validateSequences` 0/1 × `repcodeResolution` `auto/enable/disable` × sequences from row 50 | [x] |
| 52 | `ZSTD_compressSequencesAndLiterals` | same option cross-product as row 51, with an explicit literals buffer | [x] |
| 53 | `ZSTD_CCtx_setParameter` full sweep + round trip | for each of the 39 valid `ZSTD_cParameter`: set `lowerBound`, mid, `upperBound`, then compress and decompress; assert identical frames and identical `ZSTD_CCtx_getParameter` read-back | [x] |
| 54 | `ZSTD_DCtx_setParameter` full sweep + round trip | for each of the 8 valid `ZSTD_dParameter`: set `lowerBound`, mid, `upperBound`, then decode a matching frame | [x] |
| 55 | `ZDICT_trainFromBuffer`, `ZDICT_getDictID`, `ZDICT_getDictHeaderSize`, `ZDICT_isError`, `ZDICT_getErrorName` | randomized sample corpora: `nbSamples` `{8, 64, 512}` × sample sizes × `dictBufferCapacity` `{ZDICT_DICTSIZE_MIN, 4KB, 64KB, 110KB}` | [x] |
| 56 | `ZDICT_trainFromBuffer_cover`, `ZDICT_optimizeTrainFromBuffer_cover` | `k` `{16,50,200,1024}` × `d` `{6,8}` × `steps` `{0,4,8}` × `splitPoint` `{0,0.75,1.0}` × `shrinkDict` 0/1 × `nbThreads` 1 | [x] |
| 57 | `ZDICT_trainFromBuffer_fastCover`, `ZDICT_optimizeTrainFromBuffer_fastCover` | `k`,`d` as row 56 × `f` `{15,20,23,25}` × `accel` `{1,2,5,10}` × `splitPoint` × `shrinkDict` 0/1 | [x] |
| 58 | `ZDICT_finalizeDictionary`, `ZDICT_addEntropyTablesFromBuffer`, `ZDICT_trainFromBuffer_legacy` | dict content from rows 55–57 × `ZDICT_params_t` `{compressionLevel 0/1/9/19, notificationLevel 0, dictID 0/explicit}` | [x] |
| 59 | `divsufsort` (internal, exported): reached via `ZDICT_trainFromBuffer_legacy` | randomized corpora incl. highly repetitive (suffix-array worst cases) | [x] |
| 60 | Round trip: dict from rows 55–58 used for `ZSTD_compress_usingDict` → `ZSTD_decompress_usingDict` | each trained dict × levels × entropy shapes | [x] |
| 61 | `ZBUFF_createCCtx`, `_advanced`, `ZBUFF_compressInit`, `_advanced`, `_usingDict`, `ZBUFF_compressContinue`, `ZBUFF_compressFlush`, `ZBUFF_compressEnd`, `ZBUFF_freeCCtx`, `ZBUFF_recommendedCInSize`, `ZBUFF_recommendedCOutSize` | levels × chunk patterns × dict shapes (deprecated streaming API) | [x] |
| 62 | `ZBUFF_createDCtx`, `_advanced`, `ZBUFF_decompressInit`, `_usingDict`, `ZBUFF_decompressContinue`, `ZBUFF_freeDCtx`, `ZBUFF_recommendedDInSize`, `ZBUFF_recommendedDOutSize`, `ZBUFF_isError`, `ZBUFF_getErrorName` | decode of every stream from row 61 | [x] |
| 63 | `ZSTDv01_decompress`, `ZSTDv01_decompressDCtx`, `ZSTDv01_createDCtx`, `_freeDCtx`, `_resetDCtx`, `_decompressContinue`, `_nextSrcSizeToDecompress`, `_findFrameSizeInfoLegacy`, `_magicNumber`, `_isError` | legacy v0.1 frames (fixture bytes) × truncations × one-shot and continue paths | [x] |
| 64 | `ZSTDv02_*` full family | same axes as row 63 | [x] |
| 65 | `ZSTDv03_*` full family | same axes as row 63 | [x] |
| 66 | `ZSTDv04_*` full family (incl. `ZBUFFv04_*` streaming) | same axes as row 63 × ZBUFF chunk patterns | [x] |
| 67 | `ZSTDv05_*` full family (incl. `ZBUFFv05_*`, `ZSTDv05_getFrameParams`) | same axes as row 63 × ZBUFF chunk patterns | [x] |
| 68 | `ZSTDv06_*` full family (incl. `ZBUFFv06_*`, `ZSTDv06_getFrameParams`) | same axes as row 63 × ZBUFF chunk patterns | [x] |
| 69 | `ZSTDv07_*` full family (incl. `ZBUFFv07_*`, `ZSTDv07_getFrameParams`, dict variants) | same axes as row 63 × ZBUFF chunk patterns × dict | [x] |
| 70 | `ZSTD_decompress` on legacy frames (`zstd_legacy.h` dispatch, `ZSTD_LEGACY_SUPPORT=5`) | v0.5/v0.6/v0.7 fixtures fed to the modern entry point | [x] |
| 71 | `POOL_create`, `_create_advanced`, `_free`, `_add`, `_tryAdd`, `_sizeof`, `_resize`, `_joinJobs`, `ZSTD_customCalloc/Malloc/Free` | thread counts `{1,2,4}` × queue sizes `{1,4,16}` — MT disabled build, so the single-thread fallback path | [x] |
| 72 | `ZSTD_getErrorName`, `ZSTD_getErrorString`, `ZSTD_getErrorCode`, `ZSTD_isError`, `ERR_getErrorString` | codes `0..=200` (valid-side companion to `ERRORS.md` A1–A5) | [x] |

## Feature combinations (Phase D)

`translation/Cargo.toml` declares **no `[features]` section**, so the only
configuration is the default (empty) feature set. Verified mechanically:

```
$ grep -n "^\[features\]" translation/Cargo.toml   # -> no match
```

The C build likewise has a single configuration
(`ZSTD_LEGACY_SUPPORT=5`, `XXH_NAMESPACE=ZSTD_`, `DYNAMIC_BMI2=0`, no
`ZSTD_MULTITHREAD`). Tests are still run under
`--no-default-features` and the default set to confirm equivalence.

`[lib] crate-type = ["cdylib"]` and there is no `[[bin]]` target, so there is
**no driver binary** to diff on stdout.

## Row-to-test mapping (evidence)

Every `[x]` above corresponds to a test that was RUN and PASSED against both
`.so` files, under BOTH the default and `--no-default-features` configurations.

| row | covering test(s) |
|---|---|
| 1 | `phase_c_errors::m8_m10_constants, d6_m7_bound_functions; phase_b_coverage::row1_size_and_margin_helpers` |
| 2 | `phase_b_entropy::row2_xxh32` |
| 3 | `phase_b_entropy::row3_xxh64` |
| 4 | `phase_b_entropy::row4_xxh_version_and_no_xxh3` |
| 5 | `phase_b_entropy::row5_hist, row5b_hist_add` |
| 6 | `phase_b_entropy::row6_row8_fse_pipeline` |
| 7 | `phase_b_entropy::row6_row8_fse_pipeline` |
| 8 | `phase_b_entropy::row6_row8_fse_pipeline` |
| 9 | `phase_b_entropy::row9_row12_huf_pipeline` |
| 10 | `phase_b_entropy::row9_row12_huf_pipeline` |
| 11 | `phase_b_entropy::row9_row12_huf_pipeline` |
| 12 | `phase_b_entropy::row9_row12_huf_pipeline` |
| 13 | `phase_b_configs::row13_simple_compress_decompress_all_levels_shapes_sizes, row13b_randomized_payloads_per_level` |
| 14 | `phase_b_configs::row14_context_reuse_no_reset` |
| 15 | `phase_b_configs::row15_all_strategies, row15b_strategy_x_search_params` |
| 16 | `phase_b_configs::row16_min_match_all_mls_templates` |
| 17 | `phase_b_configs::row17_row_match_finder` |
| 18 | `phase_b_advanced::row18_long_distance_matching` |
| 19 | `phase_b_configs::row19_literal_compression_mode` |
| 20 | `phase_b_advanced::row20_target_cblock_size_superblock` |
| 21 | `phase_b_advanced::row21_block_splitter_levels` |
| 22 | `phase_b_advanced::row22_max_block_size` |
| 23 | `phase_b_configs::row23_frame_header_flags` |
| 24 | `phase_b_configs::row24_magicless_format_round_trip` |
| 25 | `phase_b_advanced::row25_window_log_force_max_window` |
| 26 | `phase_b_advanced::row26_src_size_hint_and_deterministic_ref_prefix` |
| 27 | `phase_b_advanced::row27_row31_streaming_round_trip, row27b_legacy_stream_api` |
| 28 | `phase_b_advanced::row27_row31_streaming_round_trip` |
| 29 | `phase_b_advanced::row29_simple_args` |
| 30 | `phase_b_advanced::row30_stable_buffers` |
| 31 | `phase_b_advanced::row27_row31_streaming_round_trip` |
| 32 | `phase_b_advanced::row32_decoder_options` |
| 33 | `phase_b_advanced::row33_row34_reset_and_params_object` |
| 34 | `phase_b_advanced::row33_row34_reset_and_params_object; phase_c_errors::b11_b14_cctx_params_and_reset` |
| 35 | `phase_b_configs::row35_get_params_get_cparams_adjust_check` |
| 36 | `phase_b_dict::row37_row42_dictionary_matrix; phase_b_coverage::row36_advanced_compress_variants` |
| 37 | `phase_b_dict::row37_row42_dictionary_matrix; phase_b_coverage::row36_advanced_compress_variants` |
| 38 | `phase_b_dict::row37_row42_dictionary_matrix` |
| 39 | `phase_b_dict::row37_row42_dictionary_matrix` |
| 40 | `phase_b_dict::row37_row42_dictionary_matrix` |
| 41 | `phase_b_dict::row37_row42_dictionary_matrix` |
| 42 | `phase_b_coverage::row42_legacy_stream_inits` |
| 43 | `phase_b_advanced::row43_low_level_frame_api; phase_b_coverage::row43_row44_begin_variants_and_accessors` |
| 44 | `phase_b_advanced::row44_raw_block_api, row44b_decompress_continue_driver; phase_b_coverage::row43_row44_begin_variants_and_accessors` |
| 45 | `phase_b_configs::row23_frame_header_flags (diff_frame_introspection); phase_c_errors2::e13_e23_introspection_rejections` |
| 46 | `phase_b_advanced::row46_multi_frame_and_skippable` |
| 47 | `phase_b_configs::row47_size_estimators; phase_b_coverage::row1_size_and_margin_helpers` |
| 48 | `phase_b_dict::row48_row49_static_and_custom_mem` |
| 49 | `phase_b_dict::row48_row49_static_and_custom_mem` |
| 50 | `phase_b_dict::row50_row52_sequence_api` |
| 51 | `phase_b_dict::row50_row52_sequence_api` |
| 52 | `phase_b_dict::row50_row52_sequence_api` |
| 53 | `phase_b_configs::row53_cparam_sweep_round_trip` |
| 54 | `phase_b_configs::row54_dparam_sweep_round_trip` |
| 55 | `phase_b_dict::row55_row60_dictionary_builder` |
| 56 | `phase_b_dict::row55_row60_dictionary_builder` |
| 57 | `phase_b_dict::row55_row60_dictionary_builder` |
| 58 | `phase_b_dict::row55_row60_dictionary_builder` |
| 59 | `phase_b_coverage::row59_divsufsort (direct) + phase_b_dict::row55_row60_dictionary_builder (via trainFromBuffer_legacy)` |
| 60 | `phase_b_dict::row55_row60_dictionary_builder` |
| 61 | `phase_b_dict::row61_row62_zbuff` |
| 62 | `phase_b_dict::row61_row62_zbuff` |
| 63 | `phase_b_legacy::row63_row69_legacy_oneshot, k1_k5_legacy_error_and_magic, k8_legacy_continue_drivers` |
| 64 | `phase_b_legacy::row63_row69_legacy_oneshot, k1_k5_legacy_error_and_magic, k8_legacy_continue_drivers` |
| 65 | `phase_b_legacy::row63_row69_legacy_oneshot, k1_k5_legacy_error_and_magic, k8_legacy_continue_drivers` |
| 66 | `phase_b_legacy::row63_row69_legacy_oneshot, row66_row69_zbuff_legacy, legacy_extras` |
| 67 | `phase_b_legacy::row63_row69_legacy_oneshot, row66_row69_zbuff_legacy, row67_row69_legacy_dict_and_block_apis, k6_legacy_get_frame_params` |
| 68 | `phase_b_legacy::row63_row69_legacy_oneshot, row66_row69_zbuff_legacy, row67_row69_legacy_dict_and_block_apis, k6_legacy_get_frame_params` |
| 69 | `phase_b_legacy::row63_row69_legacy_oneshot, row66_row69_zbuff_legacy, row67_row69_legacy_dict_and_block_apis, k6_legacy_get_frame_params; phase_b_coverage::legacy_extras` |
| 70 | `phase_b_legacy::row70_k7_legacy_via_modern_api` |
| 71 | `phase_b_dict::row71_pool` |
| 72 | `phase_b_configs::row72_error_api_valid_side; phase_c_errors::a1_a2_a3_is_error_get_code_get_name` |

## Out-of-contract inputs excluded (and why)

While driving the low-level entry points, several inputs turned out to be
**undefined behaviour in the C itself**, not behaviour a translation can be
required to match. Each is excluded at exactly one place, with the C evidence:

| C function | excluded input | why (C evidence) |
|---|---|---|
| `FSE_optimalTableLog`, `FSE_optimalTableLog_internal` | `srcSize <= 1` | `assert(srcSize > 1)` then `ZSTD_highbit32((U32)(srcSize-1))`, i.e. `__builtin_clz(0)` |
| `HUF_minTableLog` | `symbolCardinality == 0` | `ZSTD_highbit32(0)`; `ZSTD_highbit32` itself asserts `val != 0` (`common/bits.h`) |
| `FSE_buildCTable_wksp`, `FSE_buildDTable_wksp` | `tableLog == 0` (which is what `FSE_normalizeCount` returns for the **RLE special case**, `if (count[s] == total) return 0;`) | `1 << (tableLog-1)` and `ZSTD_highbit32(0)`; the C's own callers take the `FSE_buildCTable_rle` path instead |
| `HUF_buildCTable_wksp`, and `HUF_compress*_repeat` via the `HUF_flags_optimalDepth` branch of `HUF_optimalTableLog` | `huffLog < HUF_minTableLog(HUF_cardinality(count, maxSymbolValue))` | `HUF_setMaxHeight` cannot converge and the C indexes past `huffNodeTable`; real callers always pass `HUF_TABLELOG_DEFAULT` (11) or larger |
| `HUF_optimalTableLog` | `srcSize <= 1`, or an all-zero histogram | `assert(srcSize > 1)`; cardinality 0 feeds `HUF_minTableLog(0)` |
| `HIST_count_simple` | `maxSymbolValue` below the true maximum symbol | `assert(*ip <= maxSymbolValue)` then unconditional `count[*ip]++`, and `while (!count[maxSymbolValue]) maxSymbolValue--` underflows |
| `ZSTD_compressSequences` | `dstCapacity < ZSTD_FRAMEHEADERSIZE_MAX` | the C does **not** forward the error from `ZSTD_writeFrameHeader`: `op += frameHeaderSize; assert(frameHeaderSize <= dstCapacity);` so `op` advances by an error code |
| `ZSTD_getDictID_fromDict`, `ZDICT_getDictID` | a claimed `dictSize >= 8` larger than the real buffer | `MEM_readLE32(dict)` runs as soon as `dictSize >= 8` |
| `ZSTDv07_isSkipFrame` | a data pointer | signature is `int ZSTDv07_isSkipFrame(ZSTDv07_DCtx*)` (`legacy/zstd_v07.c:2901`) — takes a CONTEXT |

One genuine C quirk is **reproduced rather than excluded**:
`FSE_BUILD_CTABLE_WORKSPACE_SIZE(maxSymbolValue, tableLog)` under-reports the
real requirement by 2 bytes whenever `(maxSymbolValue + 2 + tableSize)` is odd,
because `4 * ((maxSV+2 + tableSize)/2 + 2)` truncates the `/2`, while the body
needs `2*(maxSV1+1) + tableSize + tableSize + 8`. At exactly the macro value
the C accepts the call and then writes 2 bytes past the workspace. The test
over-ALLOCATES every workspace by 512 bytes while still passing the smaller
size to the function, and asserts the two libraries produce a **byte-identical
workspace image** — so the Rust is confirmed to reproduce the C's overshoot
exactly, including those 2 bytes.
