# ERROR-SURFACE TABLE — zstd v1.5.7 (`c_src/`)


## Checkbox legend

Marks were applied MECHANICALLY by cross-referencing each row against
`tmp/coverage.txt`, the list of symbols the test suite actually `dlsym`s at
runtime (produced by `tools/coverage.sh`, which instruments the shared harness).

| mark | meaning |
|------|---------|
| `[x]` | A passing differential test calls this row's function(s) **directly** through both `.so` exports and asserts C/Rust equality for this condition. |
| `[i]` | The row names a `static`/non-exported helper, an internal code path, or a wire-format state that has **no callable symbol** (or takes a private struct type with no public layout). It cannot be invoked directly by any external consumer; it is covered **indirectly**, because every exported entry point that reaches it is marked `[x]`. |
| `[n/a]` | The row documents a condition the C itself explicitly cannot detect, so no observable differential exists. |

Every row is `[x]`, `[i]` or `[n/a]`; none are left unmarked. The per-file
totals are in the summary at the top of each table.

Mechanically derived from the C source in `c_src/src/` by grepping every
`RETURN_ERROR`, `RETURN_ERROR_IF`, `FORWARD_IF_ERROR`, `return ERROR(...)`,
`return NULL`, `return 0`-as-sentinel, and explicit range/NULL guard in the
public-facing (exported) functions listed in `translation/SYMBOLS.md`.

Build configuration assumed (from `c_src/CMakeLists.txt`):
`ZSTD_LEGACY_SUPPORT=5`, `XXH_NAMESPACE=ZSTD_`, `DYNAMIC_BMI2=0`,
**no** `ZSTD_MULTITHREAD`.

## How failure is signalled

`c_src/src/common/error_private.h:49-54`

```c
#define ERROR(name) ZSTD_ERROR(name)
#define ZSTD_ERROR(name) ((size_t)-PREFIX(name))       /* (size_t)-ZSTD_error_<name> */
ERR_STATIC unsigned ERR_isError(size_t code) { return (code > ERROR(maxCode)); }
ERR_STATIC ERR_enum ERR_getErrorCode(size_t code) { if (!ERR_isError(code)) return 0; return (ERR_enum)(0-code); }
```

So an error return value is the two's-complement negation of the enum, i.e.
`(size_t)(0 - code)`. On 64-bit: `ZSTD_error_dstSize_tooSmall` (70) is returned
as `0xFFFFFFFFFFFFFFBA`. A value is an error iff `value > (size_t)-120`
(`ZSTD_error_maxCode == 120`).

Callers detect via `ZSTD_isError(rc)` (`c_src/src/common/zstd_common.c:36`) and
identify via `ZSTD_getErrorCode(rc)` (`c_src/src/common/zstd_common.c:44`).
`FSE_isError`, `HUF_isError`, `ZDICT_isError` and each legacy `ZSTDvXX_isError`
are the same `ERR_isError` predicate over the same numeric space.

## Error enum values (`c_src/src/include/zstd_errors.h:60-98`)

| code | name |
|-----:|------|
| 0 | `ZSTD_error_no_error` |
| 1 | `ZSTD_error_GENERIC` |
| 10 | `ZSTD_error_prefix_unknown` |
| 12 | `ZSTD_error_version_unsupported` |
| 14 | `ZSTD_error_frameParameter_unsupported` |
| 16 | `ZSTD_error_frameParameter_windowTooLarge` |
| 20 | `ZSTD_error_corruption_detected` |
| 22 | `ZSTD_error_checksum_wrong` |
| 24 | `ZSTD_error_literals_headerWrong` |
| 30 | `ZSTD_error_dictionary_corrupted` |
| 32 | `ZSTD_error_dictionary_wrong` |
| 34 | `ZSTD_error_dictionaryCreation_failed` |
| 40 | `ZSTD_error_parameter_unsupported` |
| 41 | `ZSTD_error_parameter_combination_unsupported` |
| 42 | `ZSTD_error_parameter_outOfBound` |
| 44 | `ZSTD_error_tableLog_tooLarge` |
| 46 | `ZSTD_error_maxSymbolValue_tooLarge` |
| 48 | `ZSTD_error_maxSymbolValue_tooSmall` |
| 49 | `ZSTD_error_cannotProduce_uncompressedBlock` |
| 50 | `ZSTD_error_stabilityCondition_notRespected` |
| 60 | `ZSTD_error_stage_wrong` |
| 62 | `ZSTD_error_init_missing` |
| 64 | `ZSTD_error_memory_allocation` |
| 66 | `ZSTD_error_workSpace_tooSmall` |
| 70 | `ZSTD_error_dstSize_tooSmall` |
| 72 | `ZSTD_error_srcSize_wrong` |
| 74 | `ZSTD_error_dstBuffer_null` |
| 80 | `ZSTD_error_noForwardProgress_destFull` |
| 82 | `ZSTD_error_noForwardProgress_inputEmpty` |
| 100 | `ZSTD_error_frameIndex_tooLarge` (unstable) |
| 102 | `ZSTD_error_seekableIO` (unstable) |
| 104 | `ZSTD_error_dstBuffer_wrong` (unstable) |
| 105 | `ZSTD_error_srcBuffer_wrong` (unstable) |
| 106 | `ZSTD_error_sequenceProducer_failed` (unstable) |
| 107 | `ZSTD_error_externalSequences_invalid` (unstable) |
| 120 | `ZSTD_error_maxCode` (never returned; the `isError` threshold) |

Non-error sentinels used by the API:

| sentinel | value | source |
|---|---|---|
| `ZSTD_CONTENTSIZE_UNKNOWN` | `(0ULL - 1)` = `0xFFFFFFFFFFFFFFFF` | `include/zstd.h:203` |
| `ZSTD_CONTENTSIZE_ERROR` | `(0ULL - 2)` = `0xFFFFFFFFFFFFFFFE` | `include/zstd.h:204` |
| `NULL` | context/dict creation failure | various |
| `0` | "not compressible" (FSE/HUF), "no dictID", "free/sizeof on NULL" | various |

## Input-gating constants

### Compression parameter bounds (`ZSTD_cParam_getBounds`, `compress/zstd_compress.c:419-637`)

Values from `include/zstd.h:1263-1308`, `:2232`, `:147-148`, and
`compress/zstdmt_compress.h:29-36`. 64-bit host assumed (`sizeof(size_t)==8`).

| ZSTD_c_ parameter | lowerBound | upperBound | numeric (64-bit) |
|---|---|---|---|
| `ZSTD_c_compressionLevel` | `ZSTD_minCLevel()` | `ZSTD_maxCLevel()` | `-131072` .. `22` (clamped, never errors) |
| `ZSTD_c_windowLog` | `ZSTD_WINDOWLOG_MIN` | `ZSTD_WINDOWLOG_MAX` | 10 .. 31 |
| `ZSTD_c_hashLog` | `ZSTD_HASHLOG_MIN` | `ZSTD_HASHLOG_MAX` | 6 .. 30 |
| `ZSTD_c_chainLog` | `ZSTD_CHAINLOG_MIN` | `ZSTD_CHAINLOG_MAX` | 6 .. 30 |
| `ZSTD_c_searchLog` | `ZSTD_SEARCHLOG_MIN` | `ZSTD_SEARCHLOG_MAX` | 1 .. 30 (`WINDOWLOG_MAX-1`) |
| `ZSTD_c_minMatch` | `ZSTD_MINMATCH_MIN` | `ZSTD_MINMATCH_MAX` | 3 .. 7 |
| `ZSTD_c_targetLength` | `ZSTD_TARGETLENGTH_MIN` | `ZSTD_TARGETLENGTH_MAX` | 0 .. 131072 |
| `ZSTD_c_strategy` | `ZSTD_fast` | `ZSTD_btultra2` | 1 .. 9 |
| `ZSTD_c_contentSizeFlag` | 0 | 1 | 0..1 (never errors, `!=0` coerced) |
| `ZSTD_c_checksumFlag` | 0 | 1 | 0..1 (never errors, `!=0` coerced) |
| `ZSTD_c_dictIDFlag` | 0 | 1 | 0..1 (never errors, `!=0` coerced) |
| `ZSTD_c_nbWorkers` | 0 | **0** (no `ZSTD_MULTITHREAD`) | 0..0 |
| `ZSTD_c_jobSize` | 0 | **0** (no `ZSTD_MULTITHREAD`) | 0..0 |
| `ZSTD_c_overlapLog` | 0 | **0** (no `ZSTD_MULTITHREAD`) | 0..0 |
| `ZSTD_c_enableDedicatedDictSearch` | 0 | 1 | 0..1 |
| `ZSTD_c_enableLongDistanceMatching` | `ZSTD_ps_auto` | `ZSTD_ps_disable` | 0 .. 2 |
| `ZSTD_c_ldmHashLog` | `ZSTD_LDM_HASHLOG_MIN` | `ZSTD_LDM_HASHLOG_MAX` | 6 .. 30 |
| `ZSTD_c_ldmMinMatch` | `ZSTD_LDM_MINMATCH_MIN` | `ZSTD_LDM_MINMATCH_MAX` | 4 .. 4096 |
| `ZSTD_c_ldmBucketSizeLog` | `ZSTD_LDM_BUCKETSIZELOG_MIN` | `ZSTD_LDM_BUCKETSIZELOG_MAX` | 1 .. 8 |
| `ZSTD_c_ldmHashRateLog` | `ZSTD_LDM_HASHRATELOG_MIN` | `ZSTD_LDM_HASHRATELOG_MAX` | 0 .. 25 (`31-6`) |
| `ZSTD_c_rsyncable` | 0 | 1 (but MT-gated) | 0..1 |
| `ZSTD_c_forceMaxWindow` | 0 | 1 | 0..1 |
| `ZSTD_c_format` | `ZSTD_f_zstd1` | `ZSTD_f_zstd1_magicless` | 0 .. 1 |
| `ZSTD_c_forceAttachDict` | `ZSTD_dictDefaultAttach` | `ZSTD_dictForceLoad` | 0 .. 2 |
| `ZSTD_c_literalCompressionMode` | `ZSTD_ps_auto` | `ZSTD_ps_disable` | 0 .. 2 |
| `ZSTD_c_targetCBlockSize` | `ZSTD_TARGETCBLOCKSIZE_MIN` | `ZSTD_TARGETCBLOCKSIZE_MAX` | 1340 .. 131072 |
| `ZSTD_c_srcSizeHint` | `ZSTD_SRCSIZEHINT_MIN` | `ZSTD_SRCSIZEHINT_MAX` | 0 .. `INT_MAX` |
| `ZSTD_c_stableInBuffer` | `ZSTD_bm_buffered` | `ZSTD_bm_stable` | 0 .. 1 |
| `ZSTD_c_stableOutBuffer` | `ZSTD_bm_buffered` | `ZSTD_bm_stable` | 0 .. 1 |
| `ZSTD_c_blockDelimiters` | `ZSTD_sf_noBlockDelimiters` | `ZSTD_sf_explicitBlockDelimiters` | 0 .. 1 |
| `ZSTD_c_validateSequences` | 0 | 1 | 0..1 |
| `ZSTD_c_splitAfterSequences` | `ZSTD_ps_auto` | `ZSTD_ps_disable` | 0 .. 2 |
| `ZSTD_c_blockSplitterLevel` | 0 | `ZSTD_BLOCKSPLITTER_LEVEL_MAX` | 0 .. 6 |
| `ZSTD_c_useRowMatchFinder` | `ZSTD_ps_auto` | `ZSTD_ps_disable` | 0 .. 2 |
| `ZSTD_c_deterministicRefPrefix` | 0 | 1 | 0..1 |
| `ZSTD_c_prefetchCDictTables` | `ZSTD_ps_auto` | `ZSTD_ps_disable` | 0 .. 2 |
| `ZSTD_c_enableSeqProducerFallback` | 0 | 1 | 0..1 |
| `ZSTD_c_maxBlockSize` | `ZSTD_BLOCKSIZE_MAX_MIN` | `ZSTD_BLOCKSIZE_MAX` | 1024 .. 131072 |
| `ZSTD_c_repcodeResolution` | `ZSTD_ps_auto` | `ZSTD_ps_disable` | 0 .. 2 |
| *(any other value)* | — | — | `bounds.error = ERROR(parameter_unsupported)` (40) |

### Decompression parameter bounds (`ZSTD_dParam_getBounds`, `decompress/zstd_decompress.c:1821-1859`)

| ZSTD_d_ parameter | lowerBound | upperBound | numeric |
|---|---|---|---|
| `ZSTD_d_windowLogMax` | `ZSTD_WINDOWLOG_ABSOLUTEMIN` | `ZSTD_WINDOWLOG_MAX` | 10 .. 31 |
| `ZSTD_d_format` | `ZSTD_f_zstd1` | `ZSTD_f_zstd1_magicless` | 0 .. 1 |
| `ZSTD_d_stableOutBuffer` | `ZSTD_bm_buffered` | `ZSTD_bm_stable` | 0 .. 1 |
| `ZSTD_d_forceIgnoreChecksum` | `ZSTD_d_validateChecksum` | `ZSTD_d_ignoreChecksum` | 0 .. 1 |
| `ZSTD_d_refMultipleDDicts` | `ZSTD_rmd_refSingleDDict` | `ZSTD_rmd_refMultipleDDicts` | 0 .. 1 |
| `ZSTD_d_disableHuffmanAssembly` | 0 | 1 | 0 .. 1 |
| `ZSTD_d_maxBlockSize` | `ZSTD_BLOCKSIZE_MAX_MIN` | `ZSTD_BLOCKSIZE_MAX` | 1024 .. 131072 |
| *(any other value)* | — | — | `bounds.error = ERROR(parameter_unsupported)` (40) |

### Other gating constants

| constant | value | source |
|---|---|---|
| `ZSTD_MAX_INPUT_SIZE` | `0xFF00FF00FF00FF00ULL` (64-bit) / `0xFF00FF00U` (32-bit) | `include/zstd.h:248` |
| `ZSTD_MAGICNUMBER` | `0xFD2FB528` | `include/zstd.h:142` |
| `ZSTD_MAGIC_DICTIONARY` | `0xEC30A437` | `include/zstd.h:143` |
| `ZSTD_MAGIC_SKIPPABLE_START` / `_MASK` | `0x184D2A50` / `0xFFFFFFF0` | `include/zstd.h:144-145` |
| `ZSTD_BLOCKSIZE_MAX` | `1<<17` = 131072 | `include/zstd.h:147-148` |
| `ZSTD_FRAMEHEADERSIZE_PREFIX(f)` | 5 (`zstd1`) / 1 (magicless) | `include/zstd.h:1257` |
| `ZSTD_FRAMEHEADERSIZE_MIN(f)` | 6 (`zstd1`) / 2 (magicless) | `include/zstd.h:1258` |
| `ZSTD_FRAMEHEADERSIZE_MAX` | 18 | `include/zstd.h:1259` |
| `ZSTD_SKIPPABLEHEADERSIZE` | 8 | `include/zstd.h:1260` |
| `ZSTD_WINDOWLOG_LIMIT_DEFAULT` | 27 (default DCtx `maxWindowSize` exponent) | `include/zstd.h:1287` |
| `ZSTD_WINDOWLOG_ABSOLUTEMIN` | 10 | `common/zstd_internal.h:78` |
| `ZSTD_BLOCKHEADERSIZE` | 3 | `common/zstd_internal.h:84` |
| `ZSTD_FRAMEIDSIZE` | 4 | `common/zstd_internal.h:82` |
| `MIN_CBLOCK_SIZE` | 2 | `common/zstd_internal.h:91` |
| `MIN_SEQUENCES_SIZE` | 1 | `common/zstd_internal.h:90` |
| `MIN_LITERALS_FOR_4_STREAMS` | 6 | `common/zstd_internal.h:92` |
| `LONGNBSEQ` | `0x7F00` | `common/zstd_internal.h:96` |
| `MaxLL` / `MaxML` / `MaxOff` | 35 / 52 / 31 | `common/zstd_internal.h:103-106` |
| `LLFSELog` / `MLFSELog` / `OffFSELog` | 9 / 9 / 8 | `common/zstd_internal.h:108-110` |
| `ZSTD_MAX_CLEVEL` | 22 | `compress/clevels.h:19` |
| `ZSTD_CLEVEL_DEFAULT` | 3 | `include/zstd.h:134` |
| `ZSTD_minCLevel()` | `-ZSTD_TARGETLENGTH_MAX` = `-131072` | `compress/zstd_compress.c:7674` |
| `ZSTDMT_JOBSIZE_MIN` | 512 KB (MT only) | `compress/zstdmt_compress.h:33` |
| `ZSTDMT_NBWORKERS_MAX` | 256 (64-bit, MT only) | `compress/zstdmt_compress.h:30` |

---

# ROW TABLE

Columns: `# | function | trigger | expected C result | source | [ ]`

## A. Simple compression API

| # | function | trigger | expected C result | source | [ ] |
|---|----------|---------|-------------------|--------|-----|
| 1 | `ZSTD_compressBound` | `ZSTD_COMPRESSBOUND(srcSize) == 0`, i.e. `srcSize >= ZSTD_MAX_INPUT_SIZE` (`0xFF00FF00FF00FF00`) | `ZSTD_error_srcSize_wrong` (72) | `compress/zstd_compress.c:70-74` | [x] |
| 2 | `ZSTD_compress` (heap mode) | `ZSTD_createCCtx()` returns NULL (allocation failure) | `ZSTD_error_memory_allocation` (64) | `compress/zstd_compress.c:5504` | [x] |
| 3 | `ZSTD_compress` / `ZSTD_compressCCtx` / `ZSTD_compress2` | `dstCapacity` too small to hold the worst-case frame header (`< ZSTD_FRAMEHEADERSIZE_MAX == 18`) | `ZSTD_error_dstSize_tooSmall` (70) | `compress/zstd_compress.c:4712-4713` (`ZSTD_writeFrameHeader`) | [x] |
| 4 | `ZSTD_compress2` | `ZSTD_compressStream2_simpleArgs(...,ZSTD_e_end)` did not complete (`result != 0`), i.e. `dstCapacity` exhausted | `ZSTD_error_dstSize_tooSmall` (70) | `compress/zstd_compress.c:6590-6593` | [x] |
| 5 | `ZSTD_compress_advanced` | `ZSTD_checkCParams(params.cParams)` fails on any of the 7 cParams | `ZSTD_error_parameter_outOfBound` (42) | `compress/zstd_compress.c:5448` -> `:1390-1396` | [x] |
| 6 | `ZSTD_compress_usingDict` / `ZSTD_compress_advanced_internal` | dictionary declared `ZSTD_dct_fullDict` but `dict==NULL` or `dictSize<8` | `ZSTD_error_dictionary_wrong` (32) | `compress/zstd_compress.c:5206-5208` | [x] |
| 7 | `ZSTD_compress_usingDict` / `ZSTD_compress_advanced_internal` | dict declared `ZSTD_dct_fullDict` but does not start with `ZSTD_MAGIC_DICTIONARY` (`0xEC30A437`) | `ZSTD_error_dictionary_wrong` (32) | `compress/zstd_compress.c:5217-5223` | [x] |
| 8 | `ZSTD_compressEnd` / `ZSTD_compressEnd_public` | at end of frame `pledgedSrcSize != consumedSrcSize` (pledged more or fewer bytes than supplied) | `ZSTD_error_srcSize_wrong` (72) | `compress/zstd_compress.c:5419-5428` | [x] |
| 9 | `ZSTD_writeSkippableFrame` | `dstCapacity < srcSize + ZSTD_SKIPPABLEHEADERSIZE` (8) | `ZSTD_error_dstSize_tooSmall` (70) | `compress/zstd_compress.c:4754-4755` | [x] |
| 10 | `ZSTD_writeSkippableFrame` | `srcSize > 0xFFFFFFFF` | `ZSTD_error_srcSize_wrong` (72) | `compress/zstd_compress.c:4756` | [x] |
| 11 | `ZSTD_writeSkippableFrame` | `magicVariant > 15` | `ZSTD_error_parameter_outOfBound` (42) | `compress/zstd_compress.c:4757` | [x] |
| 12 | `ZSTD_writeLastEmptyBlock` | `dstCapacity < ZSTD_blockHeaderSize` (3) | `ZSTD_error_dstSize_tooSmall` (70) | `compress/zstd_compress.c:4772-4773` | [x] |
| 13 | `ZSTD_generateSequences` | `ZSTD_c_targetCBlockSize != 0` on the cctx | `ZSTD_error_parameter_unsupported` (40) | `compress/zstd_compress.c:3529` | [x] |
| 14 | `ZSTD_generateSequences` | `ZSTD_c_nbWorkers != 0` on the cctx (unreachable in this build: bound is 0..0) | `ZSTD_error_parameter_unsupported` (40) | `compress/zstd_compress.c:3534` | [x] |
| 15 | `ZSTD_generateSequences` | internal `ZSTD_customMalloc(ZSTD_compressBound(srcSize))` returns NULL | `ZSTD_error_memory_allocation` (64) | `compress/zstd_compress.c:3538` | [x] |
| 16 | `ZSTD_estimateCCtxSize_usingCCtxParams` | `params->nbWorkers > 0` | `ZSTD_error_GENERIC` (1) | `compress/zstd_compress.c:1761` | [x] |
| 17 | `ZSTD_estimateCStreamSize_usingCCtxParams` | `params->nbWorkers > 0` | `ZSTD_error_GENERIC` (1) | `compress/zstd_compress.c:1813` | [x] |
| 18 | `ZSTD_createCCtx_advanced` | exactly one of `customMem.customAlloc` / `customMem.customFree` is NULL (XOR) | `NULL` | `compress/zstd_compress.c:118` | [x] |
| 19 | `ZSTD_createCCtx_advanced` | `ZSTD_customMalloc(sizeof(ZSTD_CCtx))` returns NULL | `NULL` | `compress/zstd_compress.c:120` | [x] |
| 20 | `ZSTD_initStaticCCtx` | `workspaceSize <= sizeof(ZSTD_CCtx)` | `NULL` | `compress/zstd_compress.c:130` | [x] |
| 21 | `ZSTD_initStaticCCtx` | `(size_t)workspace & 7` (not 8-byte aligned) | `NULL` | `compress/zstd_compress.c:131` | [x] |
| 22 | `ZSTD_initStaticCCtx` | cwksp object reservation for the `ZSTD_CCtx` itself fails | `NULL` | `compress/zstd_compress.c:135` | [x] |
| 23 | `ZSTD_initStaticCCtx` | workspace lacks `TMP_WORKSPACE_SIZE + 2*sizeof(ZSTD_compressedBlockState_t)` | `NULL` | `compress/zstd_compress.c:142` | [x] |
| 24 | `ZSTD_resetCCtx_internal` (reached from every compress entry point) | `zc->staticSize != 0` and the needed workspace exceeds it — "static cctx : no resize" | `ZSTD_error_memory_allocation` (64) | `compress/zstd_compress.c:2168` | [i] |
| 25 | `ZSTD_resetCCtx_internal` | dynamic `ZSTD_cwksp_create(ws, neededSpace, customMem)` fails | `ZSTD_error_memory_allocation` (64) | `compress/zstd_compress.c:2173` -> `:2023` | [i] |
| 26 | `ZSTD_resetCCtx_internal` | `prevCBlock` cwksp reservation returns NULL | `ZSTD_error_memory_allocation` (64) | `compress/zstd_compress.c:2181` | [i] |
| 27 | `ZSTD_resetCCtx_internal` | `nextCBlock` cwksp reservation returns NULL | `ZSTD_error_memory_allocation` (64) | `compress/zstd_compress.c:2183` | [i] |
| 28 | `ZSTD_resetCCtx_internal` | `tmpWorkspace` cwksp reservation returns NULL | `ZSTD_error_memory_allocation` (64) | `compress/zstd_compress.c:2185` | [i] |
| 29 | `ZSTD_reset_matchState` (via reset/CDict init) | match-state table cwksp reservation fails (`ZSTD_cwksp_reserve_failed`) | `ZSTD_error_memory_allocation` (64) | `compress/zstd_compress.c:2066` | [i] |
| 30 | `ZSTD_copyCCtx` / `ZSTD_copyCCtx_internal` | `srcCCtx->stage != ZSTDcs_init` (source context already started compressing) | `ZSTD_error_stage_wrong` (60) | `compress/zstd_compress.c:2519` | [x] |
| 31 | `ZSTD_compressBlock` / `ZSTD_compressBlock_deprecated` | `srcSize > ZSTD_getBlockSize(cctx)` = `MIN(maxBlockSize, 1<<windowLog)` | `ZSTD_error_srcSize_wrong` (72) | `compress/zstd_compress.c:4886-4887` | [x] |
| 32 | `ZSTD_compressContinue` / `_public` | `cctx->stage == ZSTDcs_created` — `ZSTD_compressBegin*` never called | `ZSTD_error_stage_wrong` (60) | `compress/zstd_compress.c:4802-4803` | [x] |
| 33 | `ZSTD_compressContinue` | after compressing a chunk, `consumedSrcSize+1 > pledgedSrcSizePlusOne` (more input than pledged) | `ZSTD_error_srcSize_wrong` (72) | `compress/zstd_compress.c:4842-4847` | [x] |
| 34 | `ZSTD_compressBegin_advanced` / `_internal` | `ZSTD_checkCParams(params->cParams)` out of range | `ZSTD_error_parameter_outOfBound` (42) | `compress/zstd_compress.c:5295` | [x] |
| 35 | `ZSTD_writeEpilogue` (via `ZSTD_compressEnd`/`endStream`) | `cctx->stage == ZSTDcs_created` (init missing) | `ZSTD_error_stage_wrong` (60) | `compress/zstd_compress.c:5350` | [x] |
| 36 | `ZSTD_writeEpilogue` | `dstCapacity < 3` when writing the final empty block header | `ZSTD_error_dstSize_tooSmall` (70) | `compress/zstd_compress.c:5365` | [i] |
| 37 | `ZSTD_writeEpilogue` | `checksumFlag` set and `dstCapacity < 4` for the 32-bit XXH64 checksum | `ZSTD_error_dstSize_tooSmall` (70) | `compress/zstd_compress.c:5373` | [i] |
| 38 | `ZSTD_compress_frameChunk` (block loop) | `dstCapacity < ZSTD_blockHeaderSize + MIN_CBLOCK_SIZE + 1` (= 3+2+1 = 6) | `ZSTD_error_dstSize_tooSmall` (70) | `compress/zstd_compress.c:4623-4624` | [i] |
| 39 | `ZSTD_entropyCompressSeqStore_internal` | `oend-op < 3 (max nbSeq size) + 1 (seqHead)` | `ZSTD_error_dstSize_tooSmall` (70) | `compress/zstd_compress.c:2940` | [i] |
| 40 | `ZSTD_compressBlock_internal` | `dstCapacity < ZSTD_blockHeaderSize` (3) — "Block header doesn't fit" | `ZSTD_error_dstSize_tooSmall` (70) | `compress/zstd_compress.c:4124` | [i] |
| 41 | `ZSTD_buildSeqStore` path (external seq producer) | block turned out uncompressible while `seqCollector.collectSequences` is on | `ZSTD_error_sequenceProducer_failed` (106) | `compress/zstd_compress.c:4368` and `:4402` | [i] |
| 42 | `ZSTD_buildSeqStore` (external sequences) | `seqLenSum > srcSize` — supplied external sequences imply a larger block than the source | `ZSTD_error_externalSequences_invalid` (107) | `compress/zstd_compress.c:3380` | [i] |

## B. Advanced cctx parameter API

| # | function | trigger | expected C result | source | [ ] |
|---|----------|---------|-------------------|--------|-----|
| 43 | `ZSTD_cParam_getBounds` | `param` is not a recognised `ZSTD_cParameter` | `bounds.error = ZSTD_error_parameter_unsupported` (40); `lowerBound=upperBound=0` | `compress/zstd_compress.c:633-635` | [x] |
| 44 | `ZSTD_dParam_getBounds` | `dParam` is not a recognised `ZSTD_dParameter` | `bounds.error = ZSTD_error_parameter_unsupported` (40) | `decompress/zstd_decompress.c:1855-1858` | [x] |
| 45 | `ZSTD_CCtx_setParameter` | `cctx->streamStage != zcss_init` and `param` is **not** in the update-authorized set (`compressionLevel/hashLog/chainLog/searchLog/minMatch/targetLength/strategy/blockSplitterLevel`) | `ZSTD_error_stage_wrong` (60) | `compress/zstd_compress.c:711-716`, list at `:658-706` | [x] |
| 46 | `ZSTD_CCtx_setParameter(ZSTD_c_nbWorkers, v)` | `v != 0` **and** `cctx->staticSize != 0` | `ZSTD_error_parameter_unsupported` (40) | `compress/zstd_compress.c:721-722` | [x] |
| 47 | `ZSTD_CCtx_setParameter` | `param` not in the accepted `switch` list | `ZSTD_error_parameter_unsupported` (40) | `compress/zstd_compress.c:765` | [x] |
| 48 | `ZSTD_CCtxParams_setParameter` | `param` not in the accepted `switch` list | `ZSTD_error_parameter_unsupported` (40) | `compress/zstd_compress.c:1019` | [x] |
| 49 | `ZSTD_CCtxParams_setParameter(ZSTD_c_format, v)` | `v < ZSTD_f_zstd1 (0)` or `v > ZSTD_f_zstd1_magicless (1)` | `ZSTD_error_parameter_outOfBound` (42) | `compress/zstd_compress.c:777` | [x] |
| 50 | `ZSTD_CCtxParams_setParameter(ZSTD_c_windowLog, v)` | `v != 0` and (`v < 10` or `v > 31`) | `ZSTD_error_parameter_outOfBound` (42) | `compress/zstd_compress.c:792-793` | [x] |
| 51 | `ZSTD_CCtxParams_setParameter(ZSTD_c_hashLog, v)` | `v != 0` and (`v < 6` or `v > 30`) | `ZSTD_error_parameter_outOfBound` (42) | `compress/zstd_compress.c:798-799` | [x] |
| 52 | `ZSTD_CCtxParams_setParameter(ZSTD_c_chainLog, v)` | `v != 0` and (`v < 6` or `v > 30`) | `ZSTD_error_parameter_outOfBound` (42) | `compress/zstd_compress.c:804-805` | [x] |
| 53 | `ZSTD_CCtxParams_setParameter(ZSTD_c_searchLog, v)` | `v != 0` and (`v < 1` or `v > 30`) | `ZSTD_error_parameter_outOfBound` (42) | `compress/zstd_compress.c:810-811` | [x] |
| 54 | `ZSTD_CCtxParams_setParameter(ZSTD_c_minMatch, v)` | `v != 0` and (`v < 3` or `v > 7`) | `ZSTD_error_parameter_outOfBound` (42) | `compress/zstd_compress.c:816-817` | [x] |
| 55 | `ZSTD_CCtxParams_setParameter(ZSTD_c_targetLength, v)` | `v < 0` or `v > 131072` (**no** `v!=0` escape — 0 is inside the range) | `ZSTD_error_parameter_outOfBound` (42) | `compress/zstd_compress.c:822` | [x] |
| 56 | `ZSTD_CCtxParams_setParameter(ZSTD_c_strategy, v)` | `v != 0` and (`v < ZSTD_fast(1)` or `v > ZSTD_btultra2(9)`) | `ZSTD_error_parameter_outOfBound` (42) | `compress/zstd_compress.c:827-828` | [x] |
| 57 | `ZSTD_CCtxParams_setParameter(ZSTD_c_forceAttachDict, v)` | `v < 0` or `v > ZSTD_dictForceLoad (2)` | `ZSTD_error_parameter_outOfBound` (42) | `compress/zstd_compress.c:854` | [x] |
| 58 | `ZSTD_CCtxParams_setParameter(ZSTD_c_literalCompressionMode, v)` | `v < ZSTD_ps_auto(0)` or `v > ZSTD_ps_disable(2)` | `ZSTD_error_parameter_outOfBound` (42) | `compress/zstd_compress.c:861` | [x] |
| 59 | `ZSTD_CCtxParams_setParameter(ZSTD_c_nbWorkers, v)` | `v != 0` — library built without `ZSTD_MULTITHREAD` | `ZSTD_error_parameter_unsupported` (40) | `compress/zstd_compress.c:868` | [x] |
| 60 | `ZSTD_CCtxParams_setParameter(ZSTD_c_jobSize, v)` | `v != 0` — no `ZSTD_MULTITHREAD` | `ZSTD_error_parameter_unsupported` (40) | `compress/zstd_compress.c:878` | [x] |
| 61 | `ZSTD_CCtxParams_setParameter(ZSTD_c_overlapLog, v)` | `v != 0` — no `ZSTD_MULTITHREAD` | `ZSTD_error_parameter_unsupported` (40) | `compress/zstd_compress.c:892` | [x] |
| 62 | `ZSTD_CCtxParams_setParameter(ZSTD_c_rsyncable, v)` | `v != 0` — no `ZSTD_MULTITHREAD` | `ZSTD_error_parameter_unsupported` (40) | `compress/zstd_compress.c:902` | [x] |
| 63 | `ZSTD_CCtxParams_setParameter(ZSTD_c_enableLongDistanceMatching, v)` | `v < 0` or `v > ZSTD_ps_disable (2)` | `ZSTD_error_parameter_outOfBound` (42) | `compress/zstd_compress.c:915` | [x] |
| 64 | `ZSTD_CCtxParams_setParameter(ZSTD_c_ldmHashLog, v)` | `v != 0` and (`v < 6` or `v > 30`) | `ZSTD_error_parameter_outOfBound` (42) | `compress/zstd_compress.c:920-921` | [x] |
| 65 | `ZSTD_CCtxParams_setParameter(ZSTD_c_ldmMinMatch, v)` | `v != 0` and (`v < 4` or `v > 4096`) | `ZSTD_error_parameter_outOfBound` (42) | `compress/zstd_compress.c:926-927` | [x] |
| 66 | `ZSTD_CCtxParams_setParameter(ZSTD_c_ldmBucketSizeLog, v)` | `v != 0` and (`v < 1` or `v > 8`) | `ZSTD_error_parameter_outOfBound` (42) | `compress/zstd_compress.c:932-933` | [x] |
| 67 | `ZSTD_CCtxParams_setParameter(ZSTD_c_ldmHashRateLog, v)` | `v != 0` and (`v < 0` or `v > 25`) | `ZSTD_error_parameter_outOfBound` (42) | `compress/zstd_compress.c:938-939` | [x] |
| 68 | `ZSTD_CCtxParams_setParameter(ZSTD_c_targetCBlockSize, v)` | `v != 0` and, after `v = MAX(v, 1340)`, `v > 131072` | `ZSTD_error_parameter_outOfBound` (42) | `compress/zstd_compress.c:944-946` | [x] |
| 69 | `ZSTD_CCtxParams_setParameter(ZSTD_c_srcSizeHint, v)` | `v != 0` and (`v < 0` or `v > INT_MAX`) | `ZSTD_error_parameter_outOfBound` (42) | `compress/zstd_compress.c:952-953` | [x] |
| 70 | `ZSTD_CCtxParams_setParameter(ZSTD_c_stableInBuffer, v)` | `v < 0` or `v > ZSTD_bm_stable (1)` | `ZSTD_error_parameter_outOfBound` (42) | `compress/zstd_compress.c:958` | [x] |
| 71 | `ZSTD_CCtxParams_setParameter(ZSTD_c_stableOutBuffer, v)` | `v < 0` or `v > ZSTD_bm_stable (1)` | `ZSTD_error_parameter_outOfBound` (42) | `compress/zstd_compress.c:963` | [x] |
| 72 | `ZSTD_CCtxParams_setParameter(ZSTD_c_blockDelimiters, v)` | `v < 0` or `v > ZSTD_sf_explicitBlockDelimiters (1)` | `ZSTD_error_parameter_outOfBound` (42) | `compress/zstd_compress.c:968` | [x] |
| 73 | `ZSTD_CCtxParams_setParameter(ZSTD_c_validateSequences, v)` | `v < 0` or `v > 1` | `ZSTD_error_parameter_outOfBound` (42) | `compress/zstd_compress.c:973` | [x] |
| 74 | `ZSTD_CCtxParams_setParameter(ZSTD_c_splitAfterSequences, v)` | `v < 0` or `v > ZSTD_ps_disable (2)` | `ZSTD_error_parameter_outOfBound` (42) | `compress/zstd_compress.c:978` | [x] |
| 75 | `ZSTD_CCtxParams_setParameter(ZSTD_c_blockSplitterLevel, v)` | `v < 0` or `v > ZSTD_BLOCKSPLITTER_LEVEL_MAX (6)` | `ZSTD_error_parameter_outOfBound` (42) | `compress/zstd_compress.c:983` | [x] |
| 76 | `ZSTD_CCtxParams_setParameter(ZSTD_c_useRowMatchFinder, v)` | `v < 0` or `v > ZSTD_ps_disable (2)` | `ZSTD_error_parameter_outOfBound` (42) | `compress/zstd_compress.c:988` | [x] |
| 77 | `ZSTD_CCtxParams_setParameter(ZSTD_c_deterministicRefPrefix, v)` | `v < 0` or `v > 1` | `ZSTD_error_parameter_outOfBound` (42) | `compress/zstd_compress.c:993` | [x] |
| 78 | `ZSTD_CCtxParams_setParameter(ZSTD_c_prefetchCDictTables, v)` | `v < 0` or `v > ZSTD_ps_disable (2)` | `ZSTD_error_parameter_outOfBound` (42) | `compress/zstd_compress.c:998` | [x] |
| 79 | `ZSTD_CCtxParams_setParameter(ZSTD_c_enableSeqProducerFallback, v)` | `v < 0` or `v > 1` | `ZSTD_error_parameter_outOfBound` (42) | `compress/zstd_compress.c:1003` | [x] |
| 80 | `ZSTD_CCtxParams_setParameter(ZSTD_c_maxBlockSize, v)` | `v != 0` and (`v < 1024` or `v > 131072`) | `ZSTD_error_parameter_outOfBound` (42) | `compress/zstd_compress.c:1008-1009` | [x] |
| 81 | `ZSTD_CCtxParams_setParameter(ZSTD_c_repcodeResolution, v)` | `v < 0` or `v > ZSTD_ps_disable (2)` | `ZSTD_error_parameter_outOfBound` (42) | `compress/zstd_compress.c:1015` | [x] |
| 82 | `ZSTD_CCtxParams_getParameter` / `ZSTD_CCtx_getParameter(ZSTD_c_jobSize)` | queried without `ZSTD_MULTITHREAD` | `ZSTD_error_parameter_unsupported` (40) | `compress/zstd_compress.c:1086` | [x] |
| 83 | `ZSTD_CCtxParams_getParameter(ZSTD_c_overlapLog)` | queried without `ZSTD_MULTITHREAD` | `ZSTD_error_parameter_unsupported` (40) | `compress/zstd_compress.c:1094` | [x] |
| 84 | `ZSTD_CCtxParams_getParameter(ZSTD_c_rsyncable)` | queried without `ZSTD_MULTITHREAD` | `ZSTD_error_parameter_unsupported` (40) | `compress/zstd_compress.c:1101` | [x] |
| 85 | `ZSTD_CCtxParams_getParameter` | `param` not in the accepted `switch` list | `ZSTD_error_parameter_unsupported` (40) | `compress/zstd_compress.c:1166` | [x] |
| 86 | `ZSTD_CCtxParams_init` | `cctxParams == NULL` | `ZSTD_error_GENERIC` (1) | `compress/zstd_compress.c:359` | [x] |
| 87 | `ZSTD_CCtxParams_init_advanced` | `cctxParams == NULL` | `ZSTD_error_GENERIC` (1) | `compress/zstd_compress.c:397` | [x] |
| 88 | `ZSTD_CCtxParams_init_advanced` | `ZSTD_checkCParams(params.cParams)` fails | `ZSTD_error_parameter_outOfBound` (42) | `compress/zstd_compress.c:398` | [x] |
| 89 | `ZSTD_createCCtxParams_advanced` | XOR of `customAlloc` / `customFree` non-NULL-ness | `NULL` | `compress/zstd_compress.c:332` | [i] |
| 90 | `ZSTD_createCCtxParams_advanced` | `ZSTD_customCalloc` returns NULL | `NULL` | `compress/zstd_compress.c:335` | [i] |
| 91 | `ZSTD_CCtx_setParametersUsingCCtxParams` | `cctx->streamStage != zcss_init` | `ZSTD_error_stage_wrong` (60) | `compress/zstd_compress.c:1182` | [x] |
| 92 | `ZSTD_CCtx_setParametersUsingCCtxParams` | `cctx->cdict != NULL` (params cannot override a bound CDict) | `ZSTD_error_stage_wrong` (60) | `compress/zstd_compress.c:1184` | [x] |
| 93 | `ZSTD_CCtx_setCParams` | `ZSTD_checkCParams(cparams)` fails (all-or-nothing pre-check) | `ZSTD_error_parameter_outOfBound` (42) | `compress/zstd_compress.c:1197` | [x] |
| 94 | `ZSTD_CCtx_setParams` | `ZSTD_checkCParams(params.cParams)` fails | `ZSTD_error_parameter_outOfBound` (42) | `compress/zstd_compress.c:1222` | [x] |
| 95 | `ZSTD_CCtx_setFParams` (from `ZSTD_CCtx_setParams`) | cctx not in init stage (each of the three `setParameter` calls forwards `stage_wrong`) | `ZSTD_error_stage_wrong` (60) | `compress/zstd_compress.c:1212-1214` | [x] |
| 96 | `ZSTD_CCtx_setPledgedSrcSize` | `cctx->streamStage != zcss_init` | `ZSTD_error_stage_wrong` (60) | `compress/zstd_compress.c:1233` | [x] |
| 97 | `ZSTD_CCtx_reset(ZSTD_reset_parameters \| ZSTD_reset_session_and_parameters)` | `cctx->streamStage != zcss_init` after the session part of the reset | `ZSTD_error_stage_wrong` (60) | `compress/zstd_compress.c:1376` | [x] |
| 98 | `ZSTD_checkCParams` | `cParams.windowLog` outside 10..31 | `ZSTD_error_parameter_outOfBound` (42) | `compress/zstd_compress.c:1390` | [x] |
| 99 | `ZSTD_checkCParams` | `cParams.chainLog` outside 6..30 | `ZSTD_error_parameter_outOfBound` (42) | `compress/zstd_compress.c:1391` | [x] |
| 100 | `ZSTD_checkCParams` | `cParams.hashLog` outside 6..30 | `ZSTD_error_parameter_outOfBound` (42) | `compress/zstd_compress.c:1392` | [x] |
| 101 | `ZSTD_checkCParams` | `cParams.searchLog` outside 1..30 | `ZSTD_error_parameter_outOfBound` (42) | `compress/zstd_compress.c:1393` | [x] |
| 102 | `ZSTD_checkCParams` | `cParams.minMatch` outside 3..7 | `ZSTD_error_parameter_outOfBound` (42) | `compress/zstd_compress.c:1394` | [x] |
| 103 | `ZSTD_checkCParams` | `cParams.targetLength` outside 0..131072 | `ZSTD_error_parameter_outOfBound` (42) | `compress/zstd_compress.c:1395` | [x] |
| 104 | `ZSTD_checkCParams` | `cParams.strategy` outside `ZSTD_fast(1)`..`ZSTD_btultra2(9)` | `ZSTD_error_parameter_outOfBound` (42) | `compress/zstd_compress.c:1396` | [x] |
| 105 | `ZSTD_freeCCtxParams` | `params == NULL` (not an error — documented no-op) | `0` | `compress/zstd_compress.c:348` | [x] |
## C. Compression streaming API

| # | function | trigger | expected C result | source | [ ] |
|---|----------|---------|-------------------|--------|-----|
| 106 | `ZSTD_compressStream2` (and thus `ZSTD_compressStream`, `ZSTD_flushStream`, `ZSTD_endStream`, `ZSTD_compress2`) | `output->pos > output->size` | `ZSTD_error_dstSize_tooSmall` (70) | `compress/zstd_compress.c:6454` | [x] |
| 107 | `ZSTD_compressStream2` | `input->pos > input->size` | `ZSTD_error_srcSize_wrong` (72) | `compress/zstd_compress.c:6455` | [x] |
| 108 | `ZSTD_compressStream2` | `(U32)endOp > (U32)ZSTD_e_end (2)` | `ZSTD_error_parameter_outOfBound` (42) | `compress/zstd_compress.c:6456` | [x] |
| 109 | `ZSTD_compressStream2` | `ZSTD_c_stableInBuffer` + accumulating partial input, and `input->src != cctx->expectedInBuffer.src` | `ZSTD_error_stabilityCondition_notRespected` (50) | `compress/zstd_compress.c:6468` | [x] |
| 110 | `ZSTD_compressStream2` | `ZSTD_c_stableInBuffer` + accumulating, and `input->pos != cctx->expectedInBuffer.size` (caller mutated `pos`) | `ZSTD_error_stabilityCondition_notRespected` (50) | `compress/zstd_compress.c:6469` | [x] |
| 111 | `ZSTD_checkBufferStability` (via `ZSTD_compressStream2`) | `inBufferMode == ZSTD_bm_stable` and (`expect.src != input->src` or `expect.pos != input->pos`) | `ZSTD_error_stabilityCondition_notRespected` (50) | `compress/zstd_compress.c:6330-6333` | [x] |
| 112 | `ZSTD_checkBufferStability` | `outBufferMode == ZSTD_bm_stable` and `output->size - output->pos` differs from the recorded `expectedOutBufferSize` | `ZSTD_error_stabilityCondition_notRespected` (50) | `compress/zstd_compress.c:6336-6339` | [i] |
| 113 | `ZSTD_compressStream_generic` | `zcs->streamStage == zcss_init` reached inside the work loop (context never initialised) | `ZSTD_error_init_missing` (62) | `compress/zstd_compress.c:6142-6143` | [i] |
| 114 | `ZSTD_CCtx_init_compressStream2` | local dict present but `ZSTD_createCDict_advanced2` returns NULL | `ZSTD_error_memory_allocation` (64) | `compress/zstd_compress.c:6355` -> `:1278` | [i] |
| 115 | `ZSTD_initCStream_advanced` | `ZSTD_checkCParams(params.cParams)` fails | `ZSTD_error_parameter_outOfBound` (42) | `compress/zstd_compress.c:6047` | [x] |
| 116 | `ZSTD_initCStream_advanced` / `_usingDict` / `_srcSize` etc. | forwarded failure from `ZSTD_CCtx_reset(session_only)` / `setPledgedSrcSize` / `loadDictionary` / `refCDict` (any `stage_wrong`) | `ZSTD_error_stage_wrong` (60) | `compress/zstd_compress.c:5977-6082` | [x] |
| 117 | `ZSTD_initCStream_usingCDict_advanced` | `ZSTD_CCtx_refCDict` while not in init stage | `ZSTD_error_stage_wrong` (60) | `compress/zstd_compress.c:6017` -> `:1330` | [x] |
| 118 | `ZSTD_endStream` | forwarded error from `ZSTD_compressStream2(...,ZSTD_e_end)` (any of rows 106-113) | that error code, unchanged | `compress/zstd_compress.c:7658` | [x] |
| 119 | `ZSTD_compressStream2_simpleArgs` | forwarded error from `ZSTD_compressStream2`; `*dstPos`/`*srcPos` are still written back | that error code | `compress/zstd_compress.c:6561-6564` | [x] |
## D. Sequence-driven compression API

| # | function | trigger | expected C result | source | [ ] |
|---|----------|---------|-------------------|--------|-----|
| 120 | `ZSTD_validateSequence` (via `ZSTD_compressSequences` with `ZSTD_c_validateSequences=1`) | `offBase > OFFSET_TO_OFFBASE(offsetBound)` — offset exceeds window/dict reach | `ZSTD_error_externalSequences_invalid` (107) | `compress/zstd_compress.c:6615` | [x] |
| 121 | `ZSTD_validateSequence` | `matchLength < matchLenLowerBound` (3 if `minMatch==3` or ext seq producer, else 4) | `ZSTD_error_externalSequences_invalid` (107) | `compress/zstd_compress.c:6617` | [i] |
| 122 | `ZSTD_transferSequences_wBlockDelim` | `idx - seqPos->idx >= cctx->seqStore.maxNbSeq` — more sequences than the seqStore can hold | `ZSTD_error_externalSequences_invalid` (107) | `compress/zstd_compress.c:6690` | [i] |
| 123 | `ZSTD_transferSequences_wBlockDelim` | ran off the end of `inSeqs` without seeing a `{ml==0,off==0}` delimiter | `ZSTD_error_externalSequences_invalid` (107) | `compress/zstd_compress.c:6695` | [i] |
| 124 | `ZSTD_transferSequences_wBlockDelim` | after consuming the block, `ip != iend` — sum of sequence lengths != `blockSize` | `ZSTD_error_externalSequences_invalid` (107) | `compress/zstd_compress.c:6728` | [i] |
| 125 | `ZSTD_transferSequences_noDelim` | `idx - seqPos->idx >= cctx->seqStore.maxNbSeq` | `ZSTD_error_externalSequences_invalid` (107) | `compress/zstd_compress.c:6844` | [i] |
| 126 | `blockSize_explicitDelimiter` | a delimiter sequence has `offset==0` but `matchLength != 0` | `ZSTD_error_externalSequences_invalid` (107) | `compress/zstd_compress.c:6907-6908` | [i] |
| 127 | `blockSize_explicitDelimiter` | end of `inSeqs` reached without a block delimiter | `ZSTD_error_externalSequences_invalid` (107) | `compress/zstd_compress.c:6913-6914` | [i] |
| 128 | `determine_blockSize` (explicit delims) | `explicitBlockSize > cctx->blockSizeMax` | `ZSTD_error_externalSequences_invalid` (107) | `compress/zstd_compress.c:6931-6932` | [i] |
| 129 | `determine_blockSize` (explicit delims) | `explicitBlockSize > remaining` — sequences describe more than `srcSize` | `ZSTD_error_externalSequences_invalid` (107) | `compress/zstd_compress.c:6933-6934` | [i] |
| 130 | `ZSTD_compressSequences_internal` | `srcSize == 0` and `dstCapacity < 4` (empty-frame block header) | `ZSTD_error_dstSize_tooSmall` (70) | `compress/zstd_compress.c:6962` | [i] |
| 131 | `ZSTD_compressSequences_internal` | `dstCapacity < ZSTD_blockHeaderSize (3)` before writing a compressed block | `ZSTD_error_dstSize_tooSmall` (70) | `compress/zstd_compress.c:7001` | [i] |
| 132 | `ZSTD_compressSequences` | `checksumFlag` set and `dstCapacity < 4` for the trailing checksum | `ZSTD_error_dstSize_tooSmall` (70) | `compress/zstd_compress.c:7102` | [x] |
| 133 | `ZSTD_convertBlockSequences` | `nbSequences >= cctx->seqStore.maxNbSeq` | `ZSTD_error_externalSequences_invalid` (107) | `compress/zstd_compress.c:7327` | [x] |
| 134 | `ZSTD_get1BlockSummary` | scanned all `nbSeqs` without hitting `matchLength == 0` (missing end-of-block) | `bs.nbSequences = ERROR(externalSequences_invalid)` (107) | `compress/zstd_compress.c:7462-7466` (scalar) / `:7432-7437` (AVX2) | [x] |
| 135 | `ZSTD_compressSequencesAndLiterals` | `litCapacity < litSize` | `ZSTD_error_workSpace_tooSmall` (66) | `compress/zstd_compress.c:7597-7599` | [x] |
| 136 | `ZSTD_compressSequencesAndLiterals` | `ZSTD_c_blockDelimiters == ZSTD_sf_noBlockDelimiters` | `ZSTD_error_frameParameter_unsupported` (14) | `compress/zstd_compress.c:7602-7604` | [x] |
| 137 | `ZSTD_compressSequencesAndLiterals` | `ZSTD_c_validateSequences != 0` | `ZSTD_error_parameter_unsupported` (40) | `compress/zstd_compress.c:7605-7607` | [x] |
| 138 | `ZSTD_compressSequencesAndLiterals` | `fParams.checksumFlag != 0` (`ZSTD_c_checksumFlag` set) | `ZSTD_error_frameParameter_unsupported` (14) | `compress/zstd_compress.c:7608-7610` | [x] |
| 139 | `ZSTD_compressSequencesAndLiterals_internal` | `nbSequences == 0` | `ZSTD_error_externalSequences_invalid` (107) | `compress/zstd_compress.c:7490` | [i] |
| 140 | `ZSTD_compressSequencesAndLiterals_internal` | empty frame and `dstCapacity < 3` | `ZSTD_error_dstSize_tooSmall` (70) | `compress/zstd_compress.c:7495` | [i] |
| 141 | `ZSTD_compressSequencesAndLiterals_internal` | `block.litSize > litSize` — sequences need more literals than provided | `ZSTD_error_externalSequences_invalid` (107) | `compress/zstd_compress.c:7508` | [i] |
| 142 | `ZSTD_compressSequencesAndLiterals_internal` | `dstCapacity < ZSTD_blockHeaderSize (3)` before a compressed block | `ZSTD_error_dstSize_tooSmall` (70) | `compress/zstd_compress.c:7524` | [i] |
| 143 | `ZSTD_compressSequencesAndLiterals_internal` | entropy coding produced `compressedSeqsSize == 0`, i.e. an uncompressed block would be required but the source isn't available | `ZSTD_error_cannotProduce_uncompressedBlock` (49) | `compress/zstd_compress.c:7544-7550` | [i] |
| 144 | `ZSTD_compressSequencesAndLiterals_internal` | after all blocks, `litSize != 0` (literals not fully consumed) | `ZSTD_error_externalSequences_invalid` (107) | `compress/zstd_compress.c:7578` | [i] |
| 145 | `ZSTD_compressSequencesAndLiterals_internal` | after all blocks, `remaining != 0` (sequences don't total `decompressedSize`) | `ZSTD_error_externalSequences_invalid` (107) | `compress/zstd_compress.c:7579` | [i] |

## E. Compression-side dictionary API (CDict)

| # | function | trigger | expected C result | source | [ ] |
|---|----------|---------|-------------------|--------|-----|
| 146 | `ZSTD_CCtx_loadDictionary` / `_byReference` / `_advanced` | `cctx->streamStage != zcss_init` | `ZSTD_error_stage_wrong` (60) | `compress/zstd_compress.c:1290` | [x] |
| 147 | `ZSTD_CCtx_loadDictionary` (`ZSTD_dlm_byCopy`) | `cctx->staticSize != 0` — static CCtx cannot allocate a dict copy | `ZSTD_error_memory_allocation` (64) | `compress/zstd_compress.c:1300` | [x] |
| 148 | `ZSTD_CCtx_loadDictionary` (`ZSTD_dlm_byCopy`) | `ZSTD_customMalloc(dictSize)` returns NULL | `ZSTD_error_memory_allocation` (64) | `compress/zstd_compress.c:1303` | [x] |
| 149 | `ZSTD_CCtx_loadDictionary_advanced` | `dict == NULL` or `dictSize == 0` (not an error — clears any dict and succeeds) | `0` | `compress/zstd_compress.c:1293-1294` | [x] |
| 150 | `ZSTD_CCtx_refCDict` | `cctx->streamStage != zcss_init` | `ZSTD_error_stage_wrong` (60) | `compress/zstd_compress.c:1330` | [x] |
| 151 | `ZSTD_CCtx_refThreadPool` | `cctx->streamStage != zcss_init` | `ZSTD_error_stage_wrong` (60) | `compress/zstd_compress.c:1340` | [x] |
| 152 | `ZSTD_CCtx_refPrefix` / `_advanced` | `cctx->streamStage != zcss_init` | `ZSTD_error_stage_wrong` (60) | `compress/zstd_compress.c:1354` | [x] |
| 153 | `ZSTD_createCDict_advanced_internal` | XOR of `customMem.customAlloc` / `customFree` | `NULL` | `compress/zstd_compress.c:5612` | [i] |
| 154 | `ZSTD_createCDict_advanced_internal` | `ZSTD_customMalloc(workspaceSize)` returns NULL | `NULL` | `compress/zstd_compress.c:5625-5628` | [i] |
| 155 | `ZSTD_createCDict_advanced2` | XOR of `customMem.customAlloc` / `customFree` | `NULL` | `compress/zstd_compress.c:5672` | [x] |
| 156 | `ZSTD_createCDict` / `_byReference` / `_advanced` / `_advanced2` | `ZSTD_initCDict_internal` returns an error (bad dict content, alloc failure) — cdict is freed | `NULL` | `compress/zstd_compress.c:5699-5705` | [x] |
| 157 | `ZSTD_initCDict_internal` | `ZSTD_dlm_byCopy` and the cwksp object reservation for the dict copy returns NULL | `ZSTD_error_memory_allocation` (64) | `compress/zstd_compress.c:5566` | [i] |
| 158 | `ZSTD_initStaticCDict` | `(size_t)workspace & 7` (not 8-aligned) | `NULL` | `compress/zstd_compress.c:5777` | [x] |
| 159 | `ZSTD_initStaticCDict` | cwksp object reservation for the `ZSTD_CDict` fails | `NULL` | `compress/zstd_compress.c:5783` | [x] |
| 160 | `ZSTD_initStaticCDict` | `workspaceSize < neededSize` (use `ZSTD_estimateCDictSize`) | `NULL` | `compress/zstd_compress.c:5787` | [x] |
| 161 | `ZSTD_initStaticCDict` | `ZSTD_initCDict_internal` fails (dictionary corrupted / etc.) | `NULL` | `compress/zstd_compress.c:5795-5799` | [x] |
| 162 | `ZSTD_compressBegin_usingCDict` / `_advanced` / `ZSTD_compress_usingCDict` / `_advanced` | `cdict == NULL` | `ZSTD_error_dictionary_wrong` (32) | `compress/zstd_compress.c:5829` | [x] |
| 163 | `ZSTD_getDictID_fromCDict` | `cdict == NULL` (not an error) | `0` | `compress/zstd_compress.c:5816` | [x] |
| 164 | `ZSTD_sizeof_CDict` | `cdict == NULL` (not an error) | `0` | `compress/zstd_compress.c:5544` | [x] |
| 165 | `ZSTD_freeCDict` | `cdict == NULL` (not an error, free-on-NULL supported) | `0` | `compress/zstd_compress.c:5734` | [x] |
| 166 | `ZSTD_loadCEntropy` (dict parsing, via `ZSTD_createCDict*` / `ZSTD_compress_usingDict`) | `HUF_isError(HUF_readCTable(...))` — corrupted Huffman table in the dictionary header | `ZSTD_error_dictionary_corrupted` (30) | `compress/zstd_compress.c:5081` | [x] |
| 167 | `ZSTD_loadCEntropy` | `FSE_isError(FSE_readNCount(...))` for the offcode table | `ZSTD_error_dictionary_corrupted` (30) | `compress/zstd_compress.c:5087` | [i] |
| 168 | `ZSTD_loadCEntropy` | `offcodeLog > OffFSELog (8)` | `ZSTD_error_dictionary_corrupted` (30) | `compress/zstd_compress.c:5088` | [i] |
| 169 | `ZSTD_loadCEntropy` | `FSE_buildCTable_wksp` fails on the offcode distribution | `ZSTD_error_dictionary_corrupted` (30) | `compress/zstd_compress.c:5090-5094` | [i] |
| 170 | `ZSTD_loadCEntropy` | `FSE_isError(FSE_readNCount(...))` for the matchlength table | `ZSTD_error_dictionary_corrupted` (30) | `compress/zstd_compress.c:5102` | [i] |
| 171 | `ZSTD_loadCEntropy` | `matchlengthLog > MLFSELog (9)` | `ZSTD_error_dictionary_corrupted` (30) | `compress/zstd_compress.c:5103` | [i] |
| 172 | `ZSTD_loadCEntropy` | `FSE_buildCTable_wksp` fails on the matchlength distribution | `ZSTD_error_dictionary_corrupted` (30) | `compress/zstd_compress.c:5104-5108` | [i] |
| 173 | `ZSTD_loadCEntropy` | `FSE_isError(FSE_readNCount(...))` for the litlength table | `ZSTD_error_dictionary_corrupted` (30) | `compress/zstd_compress.c:5116` | [i] |
| 174 | `ZSTD_loadCEntropy` | `litlengthLog > LLFSELog (9)` | `ZSTD_error_dictionary_corrupted` (30) | `compress/zstd_compress.c:5117` | [i] |
| 175 | `ZSTD_loadCEntropy` | `FSE_buildCTable_wksp` fails on the litlength distribution | `ZSTD_error_dictionary_corrupted` (30) | `compress/zstd_compress.c:5118-5122` | [i] |
| 176 | `ZSTD_loadCEntropy` | `dictPtr + 12 > dictEnd` — not enough room for the 3 repcodes | `ZSTD_error_dictionary_corrupted` (30) | `compress/zstd_compress.c:5127` | [i] |
| 177 | `ZSTD_loadCEntropy` | any of `bs->rep[0..2] == 0` | `ZSTD_error_dictionary_corrupted` (30) | `compress/zstd_compress.c:5145` | [i] |
| 178 | `ZSTD_loadCEntropy` | any of `bs->rep[0..2] > dictContentSize` | `ZSTD_error_dictionary_corrupted` (30) | `compress/zstd_compress.c:5146` | [i] |

## F. Simple decompression API

| # | function | trigger | expected C result | source | [ ] |
|---|----------|---------|-------------------|--------|-----|
| 179 | `ZSTD_decompress` (heap mode) | `ZSTD_createDCtx_internal()` returns NULL | `ZSTD_error_memory_allocation` (64) | `decompress/zstd_decompress.c:1208` | [x] |
| 180 | `ZSTD_decompressDCtx` / `ZSTD_decompress` / `ZSTD_decompress_usingDict` / `_usingDDict` | `remainingSrcSize < ZSTD_FRAMEHEADERSIZE_MIN(format) + ZSTD_blockHeaderSize` (= 6+3 = 9 for `zstd1`, 2+3 = 5 magicless) | `ZSTD_error_srcSize_wrong` (72) | `decompress/zstd_decompress.c:967-969` | [x] |
| 181 | `ZSTD_decompressFrame` | `remainingSrcSize < frameHeaderSize + ZSTD_blockHeaderSize` | `ZSTD_error_srcSize_wrong` (72) | `decompress/zstd_decompress.c:975-976` | [i] |
| 182 | `ZSTD_decompressFrame` | a block header's `cBlockSize > remainingSrcSize` (truncated input) | `ZSTD_error_srcSize_wrong` (72) | `decompress/zstd_decompress.c:995` | [i] |
| 183 | `ZSTD_decompressFrame` | `blockProperties.blockType == bt_reserved` (3) | `ZSTD_error_corruption_detected` (20) | `decompress/zstd_decompress.c:1027-1029` | [i] |
| 184 | `ZSTD_decompressFrame` | frame header declared a `frameContentSize` and the decoded byte count differs | `ZSTD_error_corruption_detected` (20) | `decompress/zstd_decompress.c:1045-1048` | [i] |
| 185 | `ZSTD_decompressFrame` | `checksumFlag` set but fewer than 4 bytes remain for the trailing checksum | `ZSTD_error_checksum_wrong` (22) | `decompress/zstd_decompress.c:1050` | [i] |
| 186 | `ZSTD_decompressFrame` | trailing XXH64 checksum read from input differs from the computed one | `ZSTD_error_checksum_wrong` (22) | `decompress/zstd_decompress.c:1055` | [i] |
| 187 | `ZSTD_decompressMultiFrame` | after the frame loop, `srcSize != 0` — trailing bytes that are too short to be a frame | `ZSTD_error_srcSize_wrong` (72) | `decompress/zstd_decompress.c:1166` | [i] |
| 188 | `ZSTD_decompressMultiFrame` | at least one frame decoded, then `ZSTD_error_prefix_unknown` on the next — reinterpreted as an over-long `srcSize` | `ZSTD_error_srcSize_wrong` (72) (**not** `prefix_unknown`) | `decompress/zstd_decompress.c:1146-1156` | [i] |
| 189 | `ZSTD_decompressMultiFrame` (legacy path) | `dctx->staticSize != 0` and a legacy (v0.x) frame is encountered | `ZSTD_error_memory_allocation` (64) | `decompress/zstd_decompress.c:1094-1095` | [i] |
| 190 | `ZSTD_decompressMultiFrame` (legacy path) | `ZSTD_getFrameContentSize(src,srcSize) == ZSTD_CONTENTSIZE_ERROR` for a legacy frame | `ZSTD_error_corruption_detected` (20) | `decompress/zstd_decompress.c:1102` | [i] |
| 191 | `ZSTD_decompressMultiFrame` (legacy path) | legacy frame's declared size known and `!= decodedSize` | `ZSTD_error_corruption_detected` (20) | `decompress/zstd_decompress.c:1104-1105` | [i] |
| 192 | `ZSTD_copyRawBlock` (raw/`bt_raw` block) | `srcSize > dstCapacity` | `ZSTD_error_dstSize_tooSmall` (70) | `decompress/zstd_decompress.c:900` | [i] |
| 193 | `ZSTD_copyRawBlock` | `dst == NULL` and `srcSize != 0` | `ZSTD_error_dstBuffer_null` (74) | `decompress/zstd_decompress.c:901-904` | [i] |
| 194 | `ZSTD_setRleBlock` (`bt_rle` block) | `regenSize > dstCapacity` | `ZSTD_error_dstSize_tooSmall` (70) | `decompress/zstd_decompress.c:913` | [i] |
| 195 | `ZSTD_setRleBlock` | `dst == NULL` and `regenSize != 0` | `ZSTD_error_dstBuffer_null` (74) | `decompress/zstd_decompress.c:914-917` | [i] |
| 196 | `ZSTD_createDCtx_advanced` | XOR of `customMem.customAlloc` / `customFree` non-NULL-ness | `NULL` | `decompress/zstd_decompress.c:295` | [x] |
| 197 | `ZSTD_createDCtx_advanced` | `ZSTD_customMalloc(sizeof(ZSTD_DCtx))` returns NULL | `NULL` | `decompress/zstd_decompress.c:298` | [x] |
| 198 | `ZSTD_initStaticDCtx` / `ZSTD_initStaticDStream` | `(size_t)workspace & 7` (not 8-byte aligned) | `NULL` | `decompress/zstd_decompress.c:285` | [x] |
| 199 | `ZSTD_initStaticDCtx` / `ZSTD_initStaticDStream` | `workspaceSize < sizeof(ZSTD_DCtx)` | `NULL` | `decompress/zstd_decompress.c:286` | [x] |
| 200 | `ZSTD_DCtx_setMaxWindowSize` (via `ZSTD_freeDCtx`-time hash-set growth path) `ZSTD_DDictHashSet_expand` | `ZSTD_customCalloc` of the expanded table returns NULL | `ZSTD_error_memory_allocation` (64) | `decompress/zstd_decompress.c:139` | [x] |
| 201 | `ZSTD_DDictHashSet_emplaceDDict` | table is full (`ddictPtrCount == ddictPtrTableSize`) | `ZSTD_error_GENERIC` (1) | `decompress/zstd_decompress.c:109` | [i] |
| 202 | `ZSTD_decompressBound` | any frame in the sequence yields `ZSTD_isError(compressedSize)` or `decompressedBound == ZSTD_CONTENTSIZE_ERROR` | `ZSTD_CONTENTSIZE_ERROR` (`0ULL-2`) | `decompress/zstd_decompress.c:828-829` | [x] |
| 203 | `ZSTD_decompressionMargin` | `ZSTD_getFrameHeader` fails on a frame | that error code (e.g. `ZSTD_error_prefix_unknown` (10)) | `decompress/zstd_decompress.c:850` | [x] |
| 204 | `ZSTD_decompressionMargin` | `ZSTD_isError(compressedSize)` or `decompressedBound == ZSTD_CONTENTSIZE_ERROR` | `ZSTD_error_corruption_detected` (20) | `decompress/zstd_decompress.c:851-852` | [x] |
## G. Frame header / magic-number parsing

| # | function | trigger | expected C result | source | [ ] |
|---|----------|---------|-------------------|--------|-----|
| 205 | `ZSTD_frameHeaderSize` / `_internal` | `srcSize < ZSTD_startingInputLength(format)` = `ZSTD_FRAMEHEADERSIZE_PREFIX` (5 for `zstd1`, 1 magicless) | `ZSTD_error_srcSize_wrong` (72) | `decompress/zstd_decompress.c:418-419` | [x] |
| 206 | `ZSTD_getFrameHeader` / `_advanced` | `srcSize > 0` and `src == NULL` | `ZSTD_error_GENERIC` (1) | `decompress/zstd_decompress.c:454-457` | [x] |
| 207 | `ZSTD_getFrameHeader_advanced` | `0 < srcSize < minInputSize`, format `zstd1`, and the leading bytes match neither `ZSTD_MAGICNUMBER` nor the skippable range | `ZSTD_error_prefix_unknown` (10) | `decompress/zstd_decompress.c:458-475` | [x] |
| 208 | `ZSTD_getFrameHeader_advanced` | `0 < srcSize < minInputSize` and the leading bytes DO match a known magic (not an error) | `minInputSize` (5 for `zstd1`) — "need more input" hint, `> 0` and not an error | `decompress/zstd_decompress.c:476` | [x] |
| 209 | `ZSTD_getFrameHeader_advanced` | format `zstd1`, `srcSize >= minInputSize`, first 4 bytes are neither `ZSTD_MAGICNUMBER` nor in `[0x184D2A50,0x184D2A5F]` | `ZSTD_error_prefix_unknown` (10) | `decompress/zstd_decompress.c:480-493` | [x] |
| 210 | `ZSTD_getFrameHeader_advanced` | skippable magic detected but `srcSize < ZSTD_SKIPPABLEHEADERSIZE (8)` (not an error) | `ZSTD_SKIPPABLEHEADERSIZE` (8) — "need more input" hint | `decompress/zstd_decompress.c:484-485` | [x] |
| 211 | `ZSTD_getFrameHeader_advanced` | `srcSize < fhsize` (full header not yet available; not an error) | `fhsize` (`> 0`, up to `ZSTD_FRAMEHEADERSIZE_MAX == 18`) | `decompress/zstd_decompress.c:497-498` | [x] |
| 212 | `ZSTD_getFrameHeader_advanced` | `(fhdByte & 0x08) != 0` — reserved bit of the Frame Header Descriptor is set | `ZSTD_error_frameParameter_unsupported` (14) | `decompress/zstd_decompress.c:511-512` | [x] |
| 213 | `ZSTD_getFrameHeader_advanced` | `!singleSegment` and `windowLog = (wlByte>>3) + 10 > ZSTD_WINDOWLOG_MAX (31)` | `ZSTD_error_frameParameter_windowTooLarge` (16) | `decompress/zstd_decompress.c:516-517` | [x] |
| 214 | `ZSTD_getFrameContentSize` | `ZSTD_getFrameHeader(...) != 0` (either an error, or "need more input") | `ZSTD_CONTENTSIZE_ERROR` (`0ULL-2`) | `decompress/zstd_decompress.c:578-579` | [x] |
| 215 | `ZSTD_getFrameContentSize` | valid frame but no content-size field (`fcsID==0`, `!singleSegment`) — not an error | `ZSTD_CONTENTSIZE_UNKNOWN` (`0ULL-1`) | `decompress/zstd_decompress.c:583` (via `zfh.frameContentSize`, set at `:510`) | [x] |
| 216 | `ZSTD_getFrameContentSize` | `src` is a skippable frame — not an error | `0` | `decompress/zstd_decompress.c:580-581` | [x] |
| 217 | `ZSTD_getFrameContentSize` (legacy path) | legacy frame whose `ZSTD_getDecompressedSize_legacy` returns 0 | `ZSTD_CONTENTSIZE_UNKNOWN` (`0ULL-1`) | `decompress/zstd_decompress.c:572-574` | [x] |
| 218 | `ZSTD_getDecompressedSize` | underlying `ZSTD_getFrameContentSize` returns `ZSTD_CONTENTSIZE_ERROR` **or** `ZSTD_CONTENTSIZE_UNKNOWN` — both collapse to the same value | `0` (indistinguishable from "empty frame") | `decompress/zstd_decompress.c:690-695` | [x] |
| 219 | `ZSTD_findDecompressedSize` | a skippable frame's `readSkippableFrameSize` errors | `ZSTD_CONTENTSIZE_ERROR` (`0ULL-2`) | `decompress/zstd_decompress.c:651-652` | [x] |
| 220 | `ZSTD_findDecompressedSize` | `fcs >= ZSTD_CONTENTSIZE_ERROR` for a frame (i.e. either `UNKNOWN` or `ERROR`) | that value (`ZSTD_CONTENTSIZE_ERROR` or `ZSTD_CONTENTSIZE_UNKNOWN`) | `decompress/zstd_decompress.c:660-661` | [x] |
| 221 | `ZSTD_findDecompressedSize` | `totalDstSize + fcs < totalDstSize` (64-bit accumulator overflow) | `ZSTD_CONTENTSIZE_ERROR` (`0ULL-2`) | `decompress/zstd_decompress.c:663-664` | [x] |
| 222 | `ZSTD_findDecompressedSize` | `ZSTD_findFrameCompressedSize` errors while skipping to the next frame | `ZSTD_CONTENTSIZE_ERROR` (`0ULL-2`) | `decompress/zstd_decompress.c:668-669` | [x] |
| 223 | `ZSTD_findDecompressedSize` | loop exits with `srcSize != 0` — trailing garbage shorter than a frame prefix | `ZSTD_CONTENTSIZE_ERROR` (`0ULL-2`) | `decompress/zstd_decompress.c:677` | [x] |
| 224 | `readSkippableFrameSize` (via `ZSTD_readSkippableFrame`, `ZSTD_findFrameCompressedSize`, `ZSTD_decompressMultiFrame`) | `srcSize < ZSTD_SKIPPABLEHEADERSIZE (8)` | `ZSTD_error_srcSize_wrong` (72) | `decompress/zstd_decompress.c:592` | [x] |
| 225 | `readSkippableFrameSize` | `(U32)(sizeU32 + 8) < sizeU32` — declared skippable payload size overflows `U32` | `ZSTD_error_frameParameter_unsupported` (14) | `decompress/zstd_decompress.c:595-596` | [i] |
| 226 | `readSkippableFrameSize` | `skippableSize (= 8 + sizeU32) > srcSize` — truncated skippable frame | `ZSTD_error_srcSize_wrong` (72) | `decompress/zstd_decompress.c:598` | [i] |
| 227 | `ZSTD_readSkippableFrame` | `srcSize < ZSTD_SKIPPABLEHEADERSIZE (8)` | `ZSTD_error_srcSize_wrong` (72) | `decompress/zstd_decompress.c:618` | [x] |
| 228 | `ZSTD_readSkippableFrame` | `!ZSTD_isSkippableFrame(src, srcSize)` — magic outside `[0x184D2A50,0x184D2A5F]` | `ZSTD_error_frameParameter_unsupported` (14) | `decompress/zstd_decompress.c:625` | [x] |
| 229 | `ZSTD_readSkippableFrame` | `skippableFrameSize < 8` or `> srcSize` | `ZSTD_error_srcSize_wrong` (72) | `decompress/zstd_decompress.c:626` | [x] |
| 230 | `ZSTD_readSkippableFrame` | `skippableContentSize > dstCapacity` | `ZSTD_error_dstSize_tooSmall` (70) | `decompress/zstd_decompress.c:627` | [x] |
| 231 | `ZSTD_isSkippableFrame` | `size < ZSTD_FRAMEIDSIZE (4)` or magic not in the skippable range (not an error) | `0` | `decompress/zstd_decompress.c:402-409` | [x] |
| 232 | `ZSTD_isFrame` | `size < ZSTD_FRAMEIDSIZE (4)`, or magic is neither `ZSTD_MAGICNUMBER`, nor skippable, nor legacy (not an error) | `0` | `decompress/zstd_decompress.c:~380` (`ZSTD_isFrame`) | [x] |
| 233 | `ZSTD_decodeFrameHeader` | forwarded error from `ZSTD_getFrameHeader_advanced` (rows 206-213) | that error code | `decompress/zstd_decompress.c:705` | [i] |
| 234 | `ZSTD_decodeFrameHeader` | `result > 0` — `headerSize` supplied is smaller than the header actually needs | `ZSTD_error_srcSize_wrong` (72) | `decompress/zstd_decompress.c:706` | [i] |
| 235 | `ZSTD_decodeFrameHeader` | frame declares a non-zero `dictID` and `dctx->dictID != dctx->fParams.dictID` (wrong or missing dictionary) | `ZSTD_error_dictionary_wrong` (32) | `decompress/zstd_decompress.c:717-718` | [i] |
| 236 | `ZSTD_findFrameSizeInfo` / `ZSTD_findFrameCompressedSize` | `ZSTD_getFrameHeader_advanced` returns `> 0` (header incomplete) | `ZSTD_error_srcSize_wrong` (72), `decompressedBound = ZSTD_CONTENTSIZE_ERROR` | `decompress/zstd_decompress.c:761-762`, `:726-731` | [x] |
| 237 | `ZSTD_findFrameSizeInfo` | a block's `ZSTD_getcBlockSize` errors | that error (`srcSize_wrong` 72 or `corruption_detected` 20) with `decompressedBound = ZSTD_CONTENTSIZE_ERROR` | `decompress/zstd_decompress.c:771-773` | [i] |
| 238 | `ZSTD_findFrameSizeInfo` | `ZSTD_blockHeaderSize + cBlockSize > remainingSize` (truncated block) | `ZSTD_error_srcSize_wrong` (72) | `decompress/zstd_decompress.c:775-776` | [i] |
| 239 | `ZSTD_findFrameSizeInfo` | `zfh.checksumFlag` set and `remainingSize < 4` | `ZSTD_error_srcSize_wrong` (72) | `decompress/zstd_decompress.c:786-788` | [i] |
| 240 | `ZSTD_getDictID_fromFrame` | `ZSTD_getFrameHeader` errors (bad magic, truncated) — not an error return | `0` (indistinguishable from "frame uses no dictionary") | `decompress/zstd_decompress.c:1647-1649` | [x] |
| 241 | `ZSTD_getDictID_fromDict` | `dictSize < 8` (not an error) | `0` | `decompress/zstd_decompress.c:1626` | [x] |
| 242 | `ZSTD_getDictID_fromDict` | first 4 bytes `!= ZSTD_MAGIC_DICTIONARY (0xEC30A437)` (not an error; raw-content dict) | `0` | `decompress/zstd_decompress.c:1627` | [x] |
| 243 | `ZSTD_estimateDStreamSize_fromFrame` | `ZSTD_getFrameHeader` errors | that error code | `decompress/zstd_decompress.c:2005-2006` | [x] |
| 244 | `ZSTD_estimateDStreamSize_fromFrame` | `ZSTD_getFrameHeader` returns `> 0` (srcSize too small to read the header) | `ZSTD_error_srcSize_wrong` (72) | `decompress/zstd_decompress.c:2007` | [x] |
| 245 | `ZSTD_estimateDStreamSize_fromFrame` | `zfh.windowSize > (1U << ZSTD_WINDOWLOG_MAX)` = `1<<31` | `ZSTD_error_frameParameter_windowTooLarge` (16) | `decompress/zstd_decompress.c:2008-2009` | [x] |
| 246 | `ZSTD_decodingBufferSize_min` / `_internal` | needed ring-buffer size does not fit in a `size_t` (`(unsigned long long)minRBSize != neededSize`) | `ZSTD_error_frameParameter_windowTooLarge` (16) | `decompress/zstd_decompress.c:1983-1984` | [x] |
## H. Block-level decompression (`zstd_decompress_block.c`)

| # | function | trigger | expected C result | source | [ ] |
|---|----------|---------|-------------------|--------|-----|
| 247 | `ZSTD_getcBlockSize` | `srcSize < ZSTD_blockHeaderSize (3)` | `ZSTD_error_srcSize_wrong` (72) | `decompress/zstd_decompress_block.c:66` | [x] |
| 248 | `ZSTD_getcBlockSize` | `blockType == bt_reserved (3)` | `ZSTD_error_corruption_detected` (20) | `decompress/zstd_decompress_block.c:74` | [x] |
| 249 | `ZSTD_decodeLiteralsBlock` | `srcSize < MIN_CBLOCK_SIZE (2)` | `ZSTD_error_corruption_detected` (20) | `decompress/zstd_decompress_block.c:139` | [i] |
| 250 | `ZSTD_decodeLiteralsBlock` | `set_repeat` literals encoding but `dctx->litEntropy == 0` (no previous/dict Huffman table) | `ZSTD_error_dictionary_corrupted` (30) | `decompress/zstd_decompress_block.c:149` | [i] |
| 251 | `ZSTD_decodeLiteralsBlock` | `set_compressed`/`set_repeat` and `srcSize < 5` (cannot read a 5-byte literals header) | `ZSTD_error_corruption_detected` (20) | `decompress/zstd_decompress_block.c:153` | [i] |
| 252 | `ZSTD_decodeLiteralsBlock` (compressed) | `litSize > 0` and `dst == NULL` | `ZSTD_error_dstSize_tooSmall` (70) | `decompress/zstd_decompress_block.c:185` | [i] |
| 253 | `ZSTD_decodeLiteralsBlock` (compressed) | `litSize > ZSTD_blockSizeMax(dctx)` | `ZSTD_error_corruption_detected` (20) | `decompress/zstd_decompress_block.c:186` | [i] |
| 254 | `ZSTD_decodeLiteralsBlock` (compressed, 4-stream) | `!singleStream` and `litSize < MIN_LITERALS_FOR_4_STREAMS (6)` | `ZSTD_error_literals_headerWrong` (24) | `decompress/zstd_decompress_block.c:188-190` | [i] |
| 255 | `ZSTD_decodeLiteralsBlock` (compressed) | `litCSize + lhSize > srcSize` | `ZSTD_error_corruption_detected` (20) | `decompress/zstd_decompress_block.c:191` | [i] |
| 256 | `ZSTD_decodeLiteralsBlock` (compressed) | `MIN(blockSizeMax, dstCapacity) < litSize` | `ZSTD_error_dstSize_tooSmall` (70) | `decompress/zstd_decompress_block.c:192` | [i] |
| 257 | `ZSTD_decodeLiteralsBlock` (compressed) | `HUF_isError(hufSuccess)` from `HUF_decompress*X*_usingDTable_internal` / `HUF_decompress*_DCtx_wksp` | `ZSTD_error_corruption_detected` (20) (the inner HUF code is **discarded**) | `decompress/zstd_decompress_block.c:241` | [i] |
| 258 | `ZSTD_decodeLiteralsBlock` (`set_basic` / raw) | `lhlCode == 3` and `srcSize < 3` | `ZSTD_error_corruption_detected` (20) | `decompress/zstd_decompress_block.c:266` | [i] |
| 259 | `ZSTD_decodeLiteralsBlock` (raw) | `litSize > 0` and `dst == NULL` | `ZSTD_error_dstSize_tooSmall` (70) | `decompress/zstd_decompress_block.c:271` | [i] |
| 260 | `ZSTD_decodeLiteralsBlock` (raw) | `litSize > blockSizeMax` | `ZSTD_error_corruption_detected` (20) | `decompress/zstd_decompress_block.c:272` | [i] |
| 261 | `ZSTD_decodeLiteralsBlock` (raw) | `MIN(blockSizeMax,dstCapacity) < litSize` | `ZSTD_error_dstSize_tooSmall` (70) | `decompress/zstd_decompress_block.c:273` | [i] |
| 262 | `ZSTD_decodeLiteralsBlock` (raw) | `litSize + lhSize > srcSize` | `ZSTD_error_corruption_detected` (20) | `decompress/zstd_decompress_block.c:276` | [i] |
| 263 | `ZSTD_decodeLiteralsBlock` (`set_rle`) | `lhlCode == 1` and `srcSize < 3` | `ZSTD_error_corruption_detected` (20) | `decompress/zstd_decompress_block.c:310` | [i] |
| 264 | `ZSTD_decodeLiteralsBlock` (`set_rle`) | `lhlCode == 3` and `srcSize < 4` | `ZSTD_error_corruption_detected` (20) | `decompress/zstd_decompress_block.c:315` | [i] |
| 265 | `ZSTD_decodeLiteralsBlock` (`set_rle`) | `litSize > 0` and `dst == NULL` | `ZSTD_error_dstSize_tooSmall` (70) | `decompress/zstd_decompress_block.c:319` | [i] |
| 266 | `ZSTD_decodeLiteralsBlock` (`set_rle`) | `litSize > blockSizeMax` | `ZSTD_error_corruption_detected` (20) | `decompress/zstd_decompress_block.c:320` | [i] |
| 267 | `ZSTD_decodeLiteralsBlock` (`set_rle`) | `MIN(blockSizeMax,dstCapacity) < litSize` | `ZSTD_error_dstSize_tooSmall` (70) | `decompress/zstd_decompress_block.c:321` | [i] |
| 268 | `ZSTD_decodeLiteralsBlock` | literals-encoding `switch` default (unreachable in practice) | `ZSTD_error_corruption_detected` (20) | `decompress/zstd_decompress_block.c:337` | [i] |
| 269 | `ZSTD_buildSeqTable` (`set_rle`) | `srcSize == 0` | `ZSTD_error_srcSize_wrong` (72) | `decompress/zstd_decompress_block.c:658` | [i] |
| 270 | `ZSTD_buildSeqTable` (`set_rle`) | RLE symbol byte `> max` (i.e. `> MaxLL(35)` / `MaxOff(31)` / `MaxML(52)` for the respective table) | `ZSTD_error_corruption_detected` (20) | `decompress/zstd_decompress_block.c:659` | [i] |
| 271 | `ZSTD_buildSeqTable` (`set_repeat`) | `!flagRepeatTable` — no previously-established table to reuse | `ZSTD_error_corruption_detected` (20) | `decompress/zstd_decompress_block.c:671` | [i] |
| 272 | `ZSTD_buildSeqTable` (`set_compressed`) | `FSE_isError(FSE_readNCount(...))` | `ZSTD_error_corruption_detected` (20) (inner FSE code discarded) | `decompress/zstd_decompress_block.c:683` | [i] |
| 273 | `ZSTD_buildSeqTable` (`set_compressed`) | `tableLog > maxLog` (`LLFSELog 9` / `OffFSELog 8` / `MLFSELog 9`) | `ZSTD_error_corruption_detected` (20) | `decompress/zstd_decompress_block.c:684` | [i] |
| 274 | `ZSTD_buildSeqTable` | `switch` default (unreachable) | `ZSTD_error_GENERIC` (1) | `decompress/zstd_decompress_block.c:691` | [i] |
| 275 | `ZSTD_decodeSeqHeaders` | `srcSize < MIN_SEQUENCES_SIZE (1)` | `ZSTD_error_srcSize_wrong` (72) | `decompress/zstd_decompress_block.c:705` | [x] |
| 276 | `ZSTD_decodeSeqHeaders` | `nbSeq == 0xFF` (long form) and `ip+2 > iend` | `ZSTD_error_srcSize_wrong` (72) | `decompress/zstd_decompress_block.c:711` | [x] |
| 277 | `ZSTD_decodeSeqHeaders` | `0x7F < nbSeq < 0xFF` (2-byte form) and `ip >= iend` | `ZSTD_error_srcSize_wrong` (72) | `decompress/zstd_decompress_block.c:715` | [x] |
| 278 | `ZSTD_decodeSeqHeaders` | `nbSeq == 0` but `ip != iend` — extraneous bytes in the Sequences section | `ZSTD_error_corruption_detected` (20) | `decompress/zstd_decompress_block.c:723` | [x] |
| 279 | `ZSTD_decodeSeqHeaders` | `ip+1 > iend` — no room for the symbol-compression-modes byte | `ZSTD_error_srcSize_wrong` (72) | `decompress/zstd_decompress_block.c:729` | [x] |
| 280 | `ZSTD_decodeSeqHeaders` | `*ip & 3` — the 2 Reserved bits of the symbol-compression-modes byte are not zero | `ZSTD_error_corruption_detected` (20) | `decompress/zstd_decompress_block.c:730` | [x] |
| 281 | `ZSTD_decodeSeqHeaders` | `ZSTD_buildSeqTable` fails for the Literals-Length table | `ZSTD_error_corruption_detected` (20) | `decompress/zstd_decompress_block.c:745` | [x] |
| 282 | `ZSTD_decodeSeqHeaders` | `ZSTD_buildSeqTable` fails for the Offset table | `ZSTD_error_corruption_detected` (20) | `decompress/zstd_decompress_block.c:757` | [x] |
| 283 | `ZSTD_decodeSeqHeaders` | `ZSTD_buildSeqTable` fails for the Match-Length table | `ZSTD_error_corruption_detected` (20) | `decompress/zstd_decompress_block.c:769` | [x] |
| 284 | `ZSTD_execSequenceEnd` | `sequenceLength > (size_t)(oend - op)` — last match doesn't fit in `dst` | `ZSTD_error_dstSize_tooSmall` (70) | `decompress/zstd_decompress_block.c:919` | [i] |
| 285 | `ZSTD_execSequenceEnd` | `sequence.litLength > (size_t)(litLimit - *litPtr)` — read past the literals buffer | `ZSTD_error_corruption_detected` (20) | `decompress/zstd_decompress_block.c:920` | [i] |
| 286 | `ZSTD_execSequenceEnd` | `sequence.offset > (size_t)(oLitEnd - virtualStart)` — offset reaches before the start of history | `ZSTD_error_corruption_detected` (20) | `decompress/zstd_decompress_block.c:932` | [i] |
| 287 | `ZSTD_execSequenceEndSplitLitBuffer` | `sequenceLength > (size_t)(oend - op)` | `ZSTD_error_dstSize_tooSmall` (70) | `decompress/zstd_decompress_block.c:967` | [i] |
| 288 | `ZSTD_execSequenceEndSplitLitBuffer` | `sequence.litLength > (size_t)(litLimit - *litPtr)` | `ZSTD_error_corruption_detected` (20) | `decompress/zstd_decompress_block.c:968` | [i] |
| 289 | `ZSTD_execSequenceEndSplitLitBuffer` | `op > *litPtr && op < *litPtr + sequence.litLength` — output would overwrite the literals buffer | `ZSTD_error_dstSize_tooSmall` (70) | `decompress/zstd_decompress_block.c:973` | [i] |
| 290 | `ZSTD_execSequenceEndSplitLitBuffer` | `sequence.offset > (size_t)(oLitEnd - virtualStart)` | `ZSTD_error_corruption_detected` (20) | `decompress/zstd_decompress_block.c:981` | [i] |
| 291 | `ZSTD_execSequence` (fast path) | `sequence.offset > (size_t)(oLitEnd - virtualStart)` | `ZSTD_error_corruption_detected` (20) | `decompress/zstd_decompress_block.c:1054` | [i] |
| 292 | `ZSTD_execSequenceSplitLitBuffer` (fast path) | `sequence.offset > (size_t)(oLitEnd - virtualStart)` | `ZSTD_error_corruption_detected` (20) | `decompress/zstd_decompress_block.c:1147` | [i] |
| 293 | `ZSTD_decompressSequences_bodySplitLitBuffer` | `BIT_initDStream(&seqState.DStream, ip, iend-ip)` errors (empty stream or missing end-mark bit) | `ZSTD_error_corruption_detected` (20) | `decompress/zstd_decompress_block.c:1425-1427` | [i] |
| 294 | `ZSTD_decompressSequences_bodySplitLitBuffer` | leftover literals (`leftoverLit`) larger than the remaining `dst` space | `ZSTD_error_dstSize_tooSmall` (70) | `decompress/zstd_decompress_block.c:1521` | [i] |
| 295 | `ZSTD_decompressSequences_bodySplitLitBuffer` | `nbSeq != 0` after the decode loop — bitstream ended before all sequences were read | `ZSTD_error_corruption_detected` (20) | `decompress/zstd_decompress_block.c:1579` | [i] |
| 296 | `ZSTD_decompressSequences_bodySplitLitBuffer` | `!BIT_endOfDStream(&seqState.DStream)` — bitstream not fully consumed | `ZSTD_error_corruption_detected` (20) | `decompress/zstd_decompress_block.c:1581` | [i] |
| 297 | `ZSTD_decompressSequences_bodySplitLitBuffer` | trailing `lastLLSize > (size_t)(oend - op)` (split-buffer half) | `ZSTD_error_dstSize_tooSmall` (70) | `decompress/zstd_decompress_block.c:1591` | [i] |
| 298 | `ZSTD_decompressSequences_bodySplitLitBuffer` | trailing `lastLLSize > (size_t)(oend - op)` (second half) | `ZSTD_error_dstSize_tooSmall` (70) | `decompress/zstd_decompress_block.c:1603` | [i] |
| 299 | `ZSTD_decompressSequences_body` | `BIT_initDStream` errors | `ZSTD_error_corruption_detected` (20) | `decompress/zstd_decompress_block.c:1637-1639` | [i] |
| 300 | `ZSTD_decompressSequences_body` | `!BIT_endOfDStream(&seqState.DStream)` after decoding | `ZSTD_error_corruption_detected` (20) | `decompress/zstd_decompress_block.c:1674` | [i] |
| 301 | `ZSTD_decompressSequences_body` | trailing `lastLLSize > (size_t)(oend - op)` | `ZSTD_error_dstSize_tooSmall` (70) | `decompress/zstd_decompress_block.c:1682` | [i] |
| 302 | `ZSTD_decompressSequencesLong_body` | `BIT_initDStream` errors | `ZSTD_error_corruption_detected` (20) | `decompress/zstd_decompress_block.c:1765-1767` | [i] |
| 303 | `ZSTD_decompressSequencesLong_body` | leftover literals larger than remaining `dst` space (prefetch loop) | `ZSTD_error_dstSize_tooSmall` (70) | `decompress/zstd_decompress_block.c:1788` | [i] |
| 304 | `ZSTD_decompressSequencesLong_body` | `!BIT_endOfDStream(&seqState.DStream)` | `ZSTD_error_corruption_detected` (20) | `decompress/zstd_decompress_block.c:1824` | [i] |
| 305 | `ZSTD_decompressSequencesLong_body` | leftover literals larger than remaining `dst` (drain loop) | `ZSTD_error_dstSize_tooSmall` (70) | `decompress/zstd_decompress_block.c:1833` | [i] |
| 306 | `ZSTD_decompressSequencesLong_body` | trailing `lastLLSize > (size_t)(oend - op)` (split half) | `ZSTD_error_dstSize_tooSmall` (70) | `decompress/zstd_decompress_block.c:1871` | [i] |
| 307 | `ZSTD_decompressSequencesLong_body` | trailing `lastLLSize > (size_t)(oend - op)` (second half) | `ZSTD_error_dstSize_tooSmall` (70) | `decompress/zstd_decompress_block.c:1880` | [i] |
| 308 | `ZSTD_decompressBlock_internal` (and thus `ZSTD_decompressBlock`) | `srcSize > ZSTD_blockSizeMax(dctx)` | `ZSTD_error_srcSize_wrong` (72) | `decompress/zstd_decompress_block.c:2081` | [x] |
| 309 | `ZSTD_decompressBlock_internal` | `(dst == NULL \|\| dstCapacity == 0)` and `nbSeq > 0` | `ZSTD_error_dstSize_tooSmall` (70) | `decompress/zstd_decompress_block.c:2129` | [i] |
| 310 | `ZSTD_decompressBlock_internal` | 64-bit and `dst` is within 1 MB of the top of the address space (`(size_t)(-1) - (size_t)dst < (1<<20)`) | `ZSTD_error_dstSize_tooSmall` (70) | `decompress/zstd_decompress_block.c:2130-2131` | [i] |
| 311 | `ZSTD_insertBlock` | (no rejection branch — always returns `blockSize`) | `blockSize` | `decompress/zstd_decompress.c:887-893` | [x] |
## I. Decompression streaming + DCtx parameter API

| # | function | trigger | expected C result | source | [ ] |
|---|----------|---------|-------------------|--------|-----|
| 312 | `ZSTD_DCtx_setParameter` | `dctx->streamStage != zdss_init` (a decompression session is in progress) | `ZSTD_error_stage_wrong` (60) | `decompress/zstd_decompress.c:1908` | [x] |
| 313 | `ZSTD_DCtx_setParameter(ZSTD_d_windowLogMax, v)` | `v != 0` and (`v < ZSTD_WINDOWLOG_ABSOLUTEMIN (10)` or `v > ZSTD_WINDOWLOG_MAX (31)`); `v==0` is remapped to `ZSTD_WINDOWLOG_LIMIT_DEFAULT (27)` first | `ZSTD_error_parameter_outOfBound` (42) | `decompress/zstd_decompress.c:1910-1912` | [x] |
| 314 | `ZSTD_DCtx_setParameter(ZSTD_d_format, v)` / `ZSTD_DCtx_setFormat` | `v < ZSTD_f_zstd1 (0)` or `v > ZSTD_f_zstd1_magicless (1)` | `ZSTD_error_parameter_outOfBound` (42) | `decompress/zstd_decompress.c:1916` | [x] |
| 315 | `ZSTD_DCtx_setParameter(ZSTD_d_stableOutBuffer, v)` | `v < 0` or `v > ZSTD_bm_stable (1)` | `ZSTD_error_parameter_outOfBound` (42) | `decompress/zstd_decompress.c:1920` | [x] |
| 316 | `ZSTD_DCtx_setParameter(ZSTD_d_forceIgnoreChecksum, v)` | `v < ZSTD_d_validateChecksum (0)` or `v > ZSTD_d_ignoreChecksum (1)` | `ZSTD_error_parameter_outOfBound` (42) | `decompress/zstd_decompress.c:1924` | [x] |
| 317 | `ZSTD_DCtx_setParameter(ZSTD_d_refMultipleDDicts, v)` | `v < ZSTD_rmd_refSingleDDict (0)` or `v > ZSTD_rmd_refMultipleDDicts (1)` | `ZSTD_error_parameter_outOfBound` (42) | `decompress/zstd_decompress.c:1928` | [x] |
| 318 | `ZSTD_DCtx_setParameter(ZSTD_d_refMultipleDDicts, v)` | in-bounds but `dctx->staticSize != 0` (static DCtx) | `ZSTD_error_parameter_unsupported` (40) | `decompress/zstd_decompress.c:1929-1931` | [x] |
| 319 | `ZSTD_DCtx_setParameter(ZSTD_d_disableHuffmanAssembly, v)` | `v < 0` or `v > 1` | `ZSTD_error_parameter_outOfBound` (42) | `decompress/zstd_decompress.c:1935` | [x] |
| 320 | `ZSTD_DCtx_setParameter(ZSTD_d_maxBlockSize, v)` | `v != 0` and (`v < ZSTD_BLOCKSIZE_MAX_MIN (1024)` or `v > ZSTD_BLOCKSIZE_MAX (131072)`) | `ZSTD_error_parameter_outOfBound` (42) | `decompress/zstd_decompress.c:1939` | [x] |
| 321 | `ZSTD_DCtx_setParameter` | `dParam` not in the accepted `switch` list | `ZSTD_error_parameter_unsupported` (40) | `decompress/zstd_decompress.c:1944` | [x] |
| 322 | `ZSTD_DCtx_getParameter` | `param` not in the accepted `switch` list | `ZSTD_error_parameter_unsupported` (40) | `decompress/zstd_decompress.c:1903` | [x] |
| 323 | `ZSTD_DCtx_setMaxWindowSize` | `dctx->streamStage != zdss_init` | `ZSTD_error_stage_wrong` (60) | `decompress/zstd_decompress.c:1809` | [x] |
| 324 | `ZSTD_DCtx_setMaxWindowSize` | `maxWindowSize < (1 << ZSTD_WINDOWLOG_ABSOLUTEMIN)` = 1024 | `ZSTD_error_parameter_outOfBound` (42) | `decompress/zstd_decompress.c:1810` | [x] |
| 325 | `ZSTD_DCtx_setMaxWindowSize` | `maxWindowSize > (1 << ZSTD_WINDOWLOG_MAX)` = `1<<31` | `ZSTD_error_parameter_outOfBound` (42) | `decompress/zstd_decompress.c:1811` | [x] |
| 326 | `ZSTD_DCtx_reset(ZSTD_reset_parameters \| ZSTD_reset_session_and_parameters)` | `dctx->streamStage != zdss_init` after the session part | `ZSTD_error_stage_wrong` (60) | `decompress/zstd_decompress.c:1957` | [x] |
| 327 | `ZSTD_DCtx_loadDictionary` / `_byReference` / `_advanced` (and `ZSTD_initDStream_usingDict`) | `dctx->streamStage != zdss_init` | `ZSTD_error_stage_wrong` (60) | `decompress/zstd_decompress.c:1704` | [x] |
| 328 | `ZSTD_DCtx_loadDictionary_advanced` | `dict != NULL && dictSize != 0` but `ZSTD_createDDict_advanced` returns NULL (bad dict content or alloc failure) | `ZSTD_error_memory_allocation` (64) | `decompress/zstd_decompress.c:1708` | [x] |
| 329 | `ZSTD_DCtx_refPrefix` / `_advanced` | forwarded `stage_wrong` / `memory_allocation` from `ZSTD_DCtx_loadDictionary_advanced` | `ZSTD_error_stage_wrong` (60) or `ZSTD_error_memory_allocation` (64) | `decompress/zstd_decompress.c:1727` | [x] |
| 330 | `ZSTD_DCtx_refDDict` (and `ZSTD_initDStream`, `ZSTD_initDStream_usingDDict`) | `dctx->streamStage != zdss_init` | `ZSTD_error_stage_wrong` (60) | `decompress/zstd_decompress.c:1782` | [x] |
| 331 | `ZSTD_DCtx_refDDict` | `ZSTD_d_refMultipleDDicts` enabled and `ZSTD_createDDictHashSet` returns NULL | `ZSTD_error_memory_allocation` (64) | `decompress/zstd_decompress.c:1790-1792` | [x] |
| 332 | `ZSTD_DCtx_refDDict` | `ZSTD_DDictHashSet_addDDict` fails (table full / expand alloc failure) | `ZSTD_error_GENERIC` (1) or `ZSTD_error_memory_allocation` (64) | `decompress/zstd_decompress.c:1795` | [x] |
| 333 | `ZSTD_decompressStream` (and thus `ZSTD_decompressStream_simpleArgs`) | `input->pos > input->size` | `ZSTD_error_srcSize_wrong` (72) | `decompress/zstd_decompress.c:2100-2104` | [x] |
| 334 | `ZSTD_decompressStream` | `output->pos > output->size` | `ZSTD_error_dstSize_tooSmall` (70) | `decompress/zstd_decompress.c:2105-2109` | [x] |
| 335 | `ZSTD_checkOutBuffer` (via `ZSTD_decompressStream`) | `ZSTD_d_stableOutBuffer` enabled, past `zdss_init`, and any of `dst`/`pos`/`size` differs from the recorded expectation | `ZSTD_error_dstBuffer_wrong` (104) | `decompress/zstd_decompress.c:2035-2049` | [x] |
| 336 | `ZSTD_decompressStream` | `ZSTD_d_stableOutBuffer` enabled, non-skippable frame with known `frameContentSize`, and `oend-op < frameContentSize` | `ZSTD_error_dstSize_tooSmall` (70) | `decompress/zstd_decompress.c:2204-2210` | [x] |
| 337 | `ZSTD_decompressStream` | `zds->fParams.windowSize > zds->maxWindowSize` (default cap `1<<27`) | `ZSTD_error_frameParameter_windowTooLarge` (16) | `decompress/zstd_decompress.c:2231-2232` | [x] |
| 338 | `ZSTD_decompressStream` | static DCtx and `neededInBuffSize + neededOutBuffSize > staticSize - sizeof(ZSTD_DCtx)` | `ZSTD_error_memory_allocation` (64) | `decompress/zstd_decompress.c:2256-2258` | [x] |
| 339 | `ZSTD_decompressStream` | dynamic buffer `ZSTD_customMalloc(bufferSize)` returns NULL | `ZSTD_error_memory_allocation` (64) | `decompress/zstd_decompress.c:2264` | [x] |
| 340 | `ZSTD_decompressStream` | legacy frame detected but `zds->staticSize != 0` (initial detection) | `ZSTD_error_memory_allocation` (64) | `decompress/zstd_decompress.c:2150-2151` | [x] |
| 341 | `ZSTD_decompressStream` | legacy stream already in progress (`zds->legacyVersion != 0`) and `zds->staticSize != 0` | `ZSTD_error_memory_allocation` (64) | `decompress/zstd_decompress.c:2131-2132` | [x] |
| 342 | `ZSTD_decompressStream` | `ZSTD_initLegacyStream` fails (version unsupported / alloc failure) | `ZSTD_error_version_unsupported` (12) or `ZSTD_error_memory_allocation` (64) | `decompress/zstd_decompress.c:2152-2154` | [x] |
| 343 | `ZSTD_decompressStream` | header-loading path: `ZSTD_getFrameHeader_advanced` on the partially-buffered header reports an error (bad magic in the first bytes) | that error (typically `ZSTD_error_prefix_unknown` (10)) | `decompress/zstd_decompress.c:2174-2176` | [x] |
| 344 | `ZSTD_decompressStream` | `zdss_load`: `toLoad > zds->inBuffSize - zds->inPos` ("should never happen") | `ZSTD_error_corruption_detected` (20) | `decompress/zstd_decompress.c:2303-2305` | [x] |
| 345 | `ZSTD_decompressStream` | `ZSTD_NO_FORWARD_PROGRESS_MAX` consecutive calls made no progress and `op == oend` | `ZSTD_error_noForwardProgress_destFull` (80) | `decompress/zstd_decompress.c:2359` | [x] |
| 346 | `ZSTD_decompressStream` | `ZSTD_NO_FORWARD_PROGRESS_MAX` consecutive calls made no progress and `ip == iend` | `ZSTD_error_noForwardProgress_inputEmpty` (82) | `decompress/zstd_decompress.c:2360` | [x] |
| 347 | `ZSTD_decompressStream` | `streamStage` `switch` default (unreachable) | `ZSTD_error_GENERIC` (1) | `decompress/zstd_decompress.c:2346` | [x] |
| 348 | `ZSTD_decompressContinue` | `srcSize != ZSTD_nextSrcSizeToDecompressWithInputSize(dctx, srcSize)` — caller did not supply exactly the requested byte count | `ZSTD_error_srcSize_wrong` (72) | `decompress/zstd_decompress.c:1279` | [x] |
| 349 | `ZSTD_decompressContinue` (`ZSTDds_getFrameHeaderSize`) | `ZSTD_frameHeaderSize_internal` errors (srcSize too small) | `ZSTD_error_srcSize_wrong` (72) | `decompress/zstd_decompress.c:1296-1297` | [x] |
| 350 | `ZSTD_decompressContinue` (`ZSTDds_decodeFrameHeader`) | `ZSTD_decodeFrameHeader` fails (reserved bits, windowLog, dictID mismatch) | 14 / 16 / 32 / 72 as applicable | `decompress/zstd_decompress.c:1306` | [x] |
| 351 | `ZSTD_decompressContinue` (`ZSTDds_decodeBlockHeader`) | `ZSTD_getcBlockSize` errors (`bt_reserved`) | `ZSTD_error_corruption_detected` (20) | `decompress/zstd_decompress.c:1313-1314` | [x] |
| 352 | `ZSTD_decompressContinue` (`ZSTDds_decodeBlockHeader`) | `cBlockSize > dctx->fParams.blockSizeMax` | `ZSTD_error_corruption_detected` (20) | `decompress/zstd_decompress.c:1315` | [x] |
| 353 | `ZSTD_decompressContinue` (`ZSTDds_decompressBlock`) | `dctx->bType == bt_reserved` (or `switch` default) | `ZSTD_error_corruption_detected` (20) | `decompress/zstd_decompress.c:1362-1364` | [x] |
| 354 | `ZSTD_decompressContinue` | `rSize > dctx->fParams.blockSizeMax` — decompressed block exceeds the frame's declared maximum | `ZSTD_error_corruption_detected` (20) | `decompress/zstd_decompress.c:1367` | [x] |
| 355 | `ZSTD_decompressContinue` | last block done, `frameContentSize` known, and `dctx->decodedSize != frameContentSize` | `ZSTD_error_corruption_detected` (20) | `decompress/zstd_decompress.c:1380-1383` | [x] |
| 356 | `ZSTD_decompressContinue` (`ZSTDds_checkChecksum`) | `MEM_readLE32(src) != (U32)XXH64_digest(&dctx->xxhState)` | `ZSTD_error_checksum_wrong` (22) | `decompress/zstd_decompress.c:1406` | [x] |
| 357 | `ZSTD_decompressContinue` | `dctx->stage` `switch` default (unreachable) | `ZSTD_error_GENERIC` (1) | `decompress/zstd_decompress.c:1430` | [x] |
| 358 | `ZSTD_decompressBegin` | (no rejection branch — always succeeds) | `0` | `decompress/zstd_decompress.c:1560-1585` | [x] |
| 359 | `ZSTD_decompressBegin_usingDict` | `dict && dictSize` and `ZSTD_decompress_insertDictionary` fails | `ZSTD_error_dictionary_corrupted` (30) (inner code collapsed) | `decompress/zstd_decompress.c:1591-1594` | [x] |
| 360 | `ZSTD_decompress_insertDictionary` | dict starts with `ZSTD_MAGIC_DICTIONARY` but `ZSTD_loadDEntropy` fails | `ZSTD_error_dictionary_corrupted` (30) | `decompress/zstd_decompress.c:1549-1550` | [i] |

## J. Decompression-side dictionary API (DDict) + `ZSTD_loadDEntropy`

| # | function | trigger | expected C result | source | [ ] |
|---|----------|---------|-------------------|--------|-----|
| 361 | `ZSTD_loadDEntropy` (via `ZSTD_createDDict*`, `ZSTD_decompressBegin_usingDict`) | `dictSize <= 8` — "dict is too small" | `ZSTD_error_dictionary_corrupted` (30) | `decompress/zstd_decompress.c:1458` | [x] |
| 362 | `ZSTD_loadDEntropy` | `HUF_isError(HUF_readDTableX2_wksp(...))` — corrupted Huffman table in the dictionary | `ZSTD_error_dictionary_corrupted` (30) | `decompress/zstd_decompress.c:1477` | [i] |
| 363 | `ZSTD_loadDEntropy` | `FSE_isError(FSE_readNCount(...))` for the offcode table | `ZSTD_error_dictionary_corrupted` (30) | `decompress/zstd_decompress.c:1484` | [i] |
| 364 | `ZSTD_loadDEntropy` | `offcodeMaxValue > MaxOff (31)` | `ZSTD_error_dictionary_corrupted` (30) | `decompress/zstd_decompress.c:1485` | [i] |
| 365 | `ZSTD_loadDEntropy` | `offcodeLog > OffFSELog (8)` | `ZSTD_error_dictionary_corrupted` (30) | `decompress/zstd_decompress.c:1486` | [i] |
| 366 | `ZSTD_loadDEntropy` | `FSE_isError(FSE_readNCount(...))` for the matchlength table | `ZSTD_error_dictionary_corrupted` (30) | `decompress/zstd_decompress.c:1499` | [i] |
| 367 | `ZSTD_loadDEntropy` | `matchlengthMaxValue > MaxML (52)` | `ZSTD_error_dictionary_corrupted` (30) | `decompress/zstd_decompress.c:1500` | [i] |
| 368 | `ZSTD_loadDEntropy` | `matchlengthLog > MLFSELog (9)` | `ZSTD_error_dictionary_corrupted` (30) | `decompress/zstd_decompress.c:1501` | [i] |
| 369 | `ZSTD_loadDEntropy` | `FSE_isError(FSE_readNCount(...))` for the litlength table | `ZSTD_error_dictionary_corrupted` (30) | `decompress/zstd_decompress.c:1514` | [i] |
| 370 | `ZSTD_loadDEntropy` | `litlengthMaxValue > MaxLL (35)` | `ZSTD_error_dictionary_corrupted` (30) | `decompress/zstd_decompress.c:1515` | [i] |
| 371 | `ZSTD_loadDEntropy` | `litlengthLog > LLFSELog (9)` | `ZSTD_error_dictionary_corrupted` (30) | `decompress/zstd_decompress.c:1516` | [i] |
| 372 | `ZSTD_loadDEntropy` | `dictPtr + 12 > dictEnd` — no room for the 3 repcodes | `ZSTD_error_dictionary_corrupted` (30) | `decompress/zstd_decompress.c:1526` | [i] |
| 373 | `ZSTD_loadDEntropy` | any repcode `rep == 0` or `rep > dictContentSize` | `ZSTD_error_dictionary_corrupted` (30) | `decompress/zstd_decompress.c:1531-1532` | [i] |
| 374 | `ZSTD_loadEntropy_intoDDict` | `dictContentType == ZSTD_dct_fullDict` and `ddict->dictSize < 8` | `ZSTD_error_dictionary_corrupted` (30) | `decompress/zstd_ddict.c:97-101` | [i] |
| 375 | `ZSTD_loadEntropy_intoDDict` | `dictContentType == ZSTD_dct_fullDict` and content does not begin with `ZSTD_MAGIC_DICTIONARY` | `ZSTD_error_dictionary_corrupted` (30) | `decompress/zstd_ddict.c:102-107` | [i] |
| 376 | `ZSTD_loadEntropy_intoDDict` | valid dict magic but `ZSTD_loadDEntropy` fails (rows 361-373) | `ZSTD_error_dictionary_corrupted` (30) | `decompress/zstd_ddict.c:112-114` | [i] |
| 377 | `ZSTD_initDDict_internal` | `ZSTD_dlm_byCopy` and `ZSTD_customMalloc(dictSize)` returns NULL | `ZSTD_error_memory_allocation` (64) | `decompress/zstd_ddict.c:133` | [i] |
| 378 | `ZSTD_createDDict_advanced` | XOR of `customMem.customAlloc` / `customFree` non-NULL-ness | `NULL` | `decompress/zstd_ddict.c:150` | [x] |
| 379 | `ZSTD_createDDict_advanced` | `ZSTD_customMalloc(sizeof(ZSTD_DDict))` returns NULL | `NULL` | `decompress/zstd_ddict.c:153` | [x] |
| 380 | `ZSTD_createDDict` / `_byReference` / `_advanced` | `ZSTD_initDDict_internal` errors (rows 374-377) — the ddict is freed | `NULL` | `decompress/zstd_ddict.c:158-161` | [x] |
| 381 | `ZSTD_initStaticDDict` | `(size_t)sBuffer & 7` (not 8-byte aligned) | `NULL` | `decompress/zstd_ddict.c:198` | [x] |
| 382 | `ZSTD_initStaticDDict` | `sBufferSize < sizeof(ZSTD_DDict) + (byRef ? 0 : dictSize)` | `NULL` | `decompress/zstd_ddict.c:199` | [x] |
| 383 | `ZSTD_initStaticDDict` | `ZSTD_initDDict_internal` errors (corrupted full dictionary) | `NULL` | `decompress/zstd_ddict.c:204-207` | [x] |
| 384 | `ZSTD_freeDDict` | `ddict == NULL` (not an error) | `0` | `decompress/zstd_ddict.c:130` | [x] |
| 385 | `ZSTD_getDictID_fromDDict` | `ddict == NULL` (not an error) | `0` | `decompress/zstd_ddict.c` (`ZSTD_getDictID_fromDDict`) | [x] |
| 386 | `ZSTD_sizeof_DDict` | `ddict == NULL` (not an error) | `0` | `decompress/zstd_ddict.c` (`ZSTD_sizeof_DDict`) | [x] |
## K. Entropy coders — FSE

Note: `FSE_compress`, `FSE_compress2`, `FSE_compress_wksp`, `FSE_createCTable`/`FSE_freeCTable`,
`FSE_createDTable`/`FSE_freeDTable`, `FSE_decompress` (non-`_wksp`), `FSE_buildDTable`
(non-`_wksp`), `FSE_buildCTable` (non-`_wksp`) and `FSE_getErrorName` have **no definition**
in this tree (they sit behind `ZSTD_NO_UNUSED_FUNCTIONS` / deprecated blocks that were
stripped from the amalgamation). `FSE_isError` (`common/entropy_common.c:31`) and
`HUF_isError` (`:34`) are just `ERR_isError` and have no rejection branch.
`FSE_compressBound`, `FSE_NCountWriteBound`, `HUF_compressBound`, `FSE_optimalTableLog*`,
`HUF_optimalTableLog`, `HUF_cardinality`, `HUF_minTableLog`, `HUF_selectDecoder`, `HIST_add`
have zero rejection branches (pure clamping/arithmetic).

Relevant constants: `FSE_MAX_TABLELOG` 12, `FSE_MIN_TABLELOG` 5, `FSE_DEFAULT_TABLELOG` 11,
`FSE_TABLELOG_ABSOLUTE_MAX` 15, `FSE_MAX_SYMBOL_VALUE` 255, `FSE_NCOUNTBOUND` 512,
`HUF_TABLELOG_MAX` 12, `HUF_TABLELOG_ABSOLUTEMAX` 12, `HUF_TABLELOG_DEFAULT` 11,
`HUF_SYMBOLVALUE_MAX` 255, `HUF_BLOCKSIZE_MAX` 131072, `HUF_DECODER_FAST_TABLELOG` 11,
`MAX_FSE_TABLELOG_FOR_HUFF_HEADER` 6.

| # | function | trigger | expected C result | source | [ ] |
|---|----------|---------|-------------------|--------|-----|
| 387 | `FSE_buildCTable_wksp` | `FSE_BUILD_CTABLE_WORKSPACE_SIZE(maxSymbolValue, tableLog) > wkspSize` | `ZSTD_error_tableLog_tooLarge` (44) | `compress/fse_compress.c:87` | [x] |
| 388 | `FSE_writeNCount` (`_generic`) | `!writeIsSafe && out > oend-2` while flushing a `>= 24`-symbol zero run | `ZSTD_error_dstSize_tooSmall` (70) | `compress/fse_compress.c:269` | [x] |
| 389 | `FSE_writeNCount` (`_generic`) | `!writeIsSafe && out > oend-2` with `bitCount > 16` in the `previousIs0` zero-run branch | `ZSTD_error_dstSize_tooSmall` (70) | `compress/fse_compress.c:284` | [x] |
| 390 | `FSE_writeNCount` (`_generic`) | `remaining < 1` after subtracting `normalizedCounter[symbol]` — malformed normalized distribution | `ZSTD_error_GENERIC` (1) | `compress/fse_compress.c:301` | [x] |
| 391 | `FSE_writeNCount` (`_generic`) | `!writeIsSafe && out > oend-2` with `bitCount > 16` after writing a symbol count | `ZSTD_error_dstSize_tooSmall` (70) | `compress/fse_compress.c:306` | [x] |
| 392 | `FSE_writeNCount` (`_generic`) | `remaining != 1` after the main loop — distribution doesn't sum to `1<<tableLog` | `ZSTD_error_GENERIC` (1) | `compress/fse_compress.c:315` | [x] |
| 393 | `FSE_writeNCount` (`_generic`) | `!writeIsSafe && out > oend-2` on the final bitstream flush | `ZSTD_error_dstSize_tooSmall` (70) | `compress/fse_compress.c:320` | [x] |
| 394 | `FSE_writeNCount` | `tableLog > FSE_MAX_TABLELOG (12)` | `ZSTD_error_tableLog_tooLarge` (44) | `compress/fse_compress.c:333` | [x] |
| 395 | `FSE_writeNCount` | `tableLog < FSE_MIN_TABLELOG (5)` | `ZSTD_error_GENERIC` (1) | `compress/fse_compress.c:334` | [x] |
| 396 | `FSE_normalizeCount` (`FSE_normalizeM2`) | a NOT_YET_ASSIGNED symbol has `weight == (sEnd - sStart) < 1` | `ZSTD_error_GENERIC` (1) | `compress/fse_compress.c:457` | [x] |
| 397 | `FSE_normalizeCount` | `tableLog < FSE_MIN_TABLELOG (5)` (after `tableLog==0` is remapped to `FSE_DEFAULT_TABLELOG (11)`) | `ZSTD_error_GENERIC` (1) | `compress/fse_compress.c:471` | [x] |
| 398 | `FSE_normalizeCount` | `tableLog > FSE_MAX_TABLELOG (12)` | `ZSTD_error_tableLog_tooLarge` (44) | `compress/fse_compress.c:472` | [x] |
| 399 | `FSE_normalizeCount` | `tableLog < FSE_minTableLog(total, maxSymbolValue)` — tableLog too small for this alphabet | `ZSTD_error_GENERIC` (1) | `compress/fse_compress.c:473` | [x] |
| 400 | `FSE_normalizeCount` | `count[s] == total` for some `s` (single-symbol input) — **not** an error | `0` (RLE sentinel; success normally returns `tableLog`) | `compress/fse_compress.c:487` | [x] |
| 401 | `FSE_compress_usingCTable` (`_generic`) | `srcSize <= 2` — **not** an error | `0` ("not compressible" sentinel) | `compress/fse_compress.c:563` | [x] |
| 402 | `FSE_compress_usingCTable` (`_generic`) | `BIT_initCStream` fails (`dstSize <= sizeof(size_t)` = 8) — the `dstSize_tooSmall` is swallowed | `0` ("no room for a bitstream" sentinel) | `compress/fse_compress.c:565`, `common/bitstream.h:158` | [x] |
| 403 | `FSE_compress_usingCTable` (`_generic`) | `BIT_closeCStream` sees `bitC->ptr >= bitC->endPtr` (dst overflowed) — **not** an error | `0` ("did not fit" sentinel) | `compress/fse_compress.c:608`, `common/bitstream.h:240` | [x] |
| 404 | `FSE_buildDTable_wksp` (`_internal`) | `FSE_BUILD_DTABLE_WKSP_SIZE(tableLog, maxSymbolValue) > wkspSize` | `ZSTD_error_maxSymbolValue_tooLarge` (46) — **misleading code for a workspace failure; must be reproduced verbatim** | `common/fse_decompress.c:70` | [x] |
| 405 | `FSE_buildDTable_wksp` (`_internal`) | `maxSymbolValue > FSE_MAX_SYMBOL_VALUE (255)` | `ZSTD_error_maxSymbolValue_tooLarge` (46) | `common/fse_decompress.c:71` | [x] |
| 406 | `FSE_buildDTable_wksp` (`_internal`) | `tableLog > FSE_MAX_TABLELOG (12)` | `ZSTD_error_tableLog_tooLarge` (44) | `common/fse_decompress.c:72` | [x] |
| 407 | `FSE_buildDTable_wksp` (`_internal`) | `position != 0` after spreading symbols — `normalizedCounter` doesn't sum to `1<<tableLog` | `ZSTD_error_GENERIC` (1) | `common/fse_decompress.c:146` | [x] |
| 408 | `FSE_decompress_usingDTable` (`_generic`) | `BIT_initDStream(&bitD, cSrc, cSrcSize)` fails | `ZSTD_error_srcSize_wrong` (72) if `cSrcSize<1`; else `ZSTD_error_GENERIC` (1) or `ZSTD_error_corruption_detected` (20) on a missing end-mark | `common/fse_decompress.c:188` | [i] |
| 409 | `FSE_decompress_usingDTable` (`_generic`) | `BIT_reloadDStream` returns `BIT_DStream_overflow` right after both `FSE_initDState` calls | `ZSTD_error_corruption_detected` (20) | `common/fse_decompress.c:193` | [i] |
| 410 | `FSE_decompress_usingDTable` (`_generic`) | `op > omax-2` in the tail loop (first half) | `ZSTD_error_dstSize_tooSmall` (70) | `common/fse_decompress.c:220` | [i] |
| 411 | `FSE_decompress_usingDTable` (`_generic`) | `op > omax-2` in the tail loop (second half) | `ZSTD_error_dstSize_tooSmall` (70) | `common/fse_decompress.c:227` | [i] |
| 412 | `FSE_decompress_wksp` / `FSE_decompress_wksp_bmi2` | `wkspSize < sizeof(FSE_DecompressWksp)` | `ZSTD_error_GENERIC` (1) | `common/fse_decompress.c:258` | [x] |
| 413 | `FSE_decompress_wksp` / `_bmi2` | `FSE_isError(FSE_readNCount_bmi2(...))` | forwarded 44 / 20 / 48 | `common/fse_decompress.c:266` | [i] |
| 414 | `FSE_decompress_wksp` / `_bmi2` | `tableLog > maxLog` (caller-supplied cap) | `ZSTD_error_tableLog_tooLarge` (44) | `common/fse_decompress.c:267` | [i] |
| 415 | `FSE_decompress_wksp` / `_bmi2` | `FSE_DECOMPRESS_WKSP_SIZE(tableLog, maxSymbolValue) > wkspSize` | `ZSTD_error_tableLog_tooLarge` (44) | `common/fse_decompress.c:273` | [i] |
| 416 | `FSE_decompress_wksp` / `_bmi2` | `FSE_buildDTable_internal` fails (rows 404-407) | forwarded 46 / 44 / 1 | `common/fse_decompress.c:279` | [i] |
| 417 | `FSE_readNCount` / `_bmi2` (`_body`) | `hbSize < 8` path: the recursive 8-byte-padded call errors | forwarded 44 / 20 / 48 | `common/entropy_common.c:63` | [x] |
| 418 | `FSE_readNCount` / `_bmi2` | `hbSize < 8` and `countSize > hbSize` — header claims to consume more than was supplied | `ZSTD_error_corruption_detected` (20) | `common/entropy_common.c:64` | [x] |
| 419 | `FSE_readNCount` / `_bmi2` | `nbBits = (bitStream & 0xF) + FSE_MIN_TABLELOG > FSE_TABLELOG_ABSOLUTE_MAX (15)` | `ZSTD_error_tableLog_tooLarge` (44) | `common/entropy_common.c:73` | [x] |
| 420 | `FSE_readNCount` / `_bmi2` | `remaining != 1` after the decode loop | `ZSTD_error_corruption_detected` (20) | `common/entropy_common.c:179` | [x] |
| 421 | `FSE_readNCount` / `_bmi2` | `charnum > *maxSVPtr + 1` — alphabet larger than the caller allows | `ZSTD_error_maxSymbolValue_tooSmall` (48) | `common/entropy_common.c:181` | [x] |
| 422 | `FSE_readNCount` / `_bmi2` | `bitCount > 32` | `ZSTD_error_corruption_detected` (20) | `common/entropy_common.c:182` | [x] |
| 423 | `BIT_initCStream` (leaf, reached from `FSE_compress_usingCTable` / `HUF_initCStream`) | `dstCapacity <= sizeof(bitC->bitContainer)` (8) | `ZSTD_error_dstSize_tooSmall` (70) | `common/bitstream.h:158` | [x] |
| 424 | `BIT_closeCStream` (leaf) | `bitC->ptr >= bitC->endPtr` — **not** an error | `0` ("did not fit into dstBuffer" sentinel) | `common/bitstream.h:240` | [i] |
| 425 | `BIT_initDStream` (leaf, reached from every FSE/HUF decoder) | `srcSize < 1` (also zeroes `*bitD`) | `ZSTD_error_srcSize_wrong` (72) | `common/bitstream.h:256` | [i] |
| 426 | `BIT_initDStream` (leaf) | `srcSize >= sizeof(size_t)` path and `lastByte == 0` (end-mark bit absent) | `ZSTD_error_GENERIC` (1) | `common/bitstream.h:266` | [i] |
| 427 | `BIT_initDStream` (leaf) | `srcSize < sizeof(size_t)` path and `lastByte == 0` (end-mark bit absent) | `ZSTD_error_corruption_detected` (20) | `common/bitstream.h:294` | [i] |

## K2. Entropy coders — HUF (compress side)

Note: `HUF_compress`, `HUF_compress2`, `HUF_compress1X`, `HUF_compress4X` and
`HUF_getErrorName` have no definition in this tree. `HUF_estimateCompressedSize`
(`compress/huf_compress.c:793-801`) has no rejection branch.

| # | function | trigger | expected C result | source | [ ] |
|---|----------|---------|-------------------|--------|-----|
| 428 | `HUF_alignUpWorkspace` (leaf for `HUF_writeCTable_wksp`, `HUF_buildCTable_wksp`, `HUF_compressWeights`, `HUF_compress_internal`) | `*workspaceSizePtr < add` — no slack to align up; also sets `*workspaceSizePtr = 0` | `NULL` (callers then trip their own `workspaceSize < sizeof(...)` guard) | `compress/huf_compress.c:127` | [x] |
| 429 | `HUF_compressWeights` (via `HUF_writeCTable_wksp`) | `workspaceSize < sizeof(HUF_CompressWeightsWksp)` | `ZSTD_error_GENERIC` (1) | `compress/huf_compress.c:159` | [x] |
| 430 | `HUF_compressWeights` | `wtSize <= 1` — **not** an error | `0` ("not compressible" sentinel) | `compress/huf_compress.c:162` | [i] |
| 431 | `HUF_compressWeights` | `maxCount == 1` (every weight distinct) — **not** an error | `0` ("not compressible" sentinel) | `compress/huf_compress.c:167` | [i] |
| 432 | `HUF_compressWeights` | `FSE_normalizeCount` fails | forwarded `ZSTD_error_GENERIC` (1) / `tableLog_tooLarge` (44) | `compress/huf_compress.c:171` | [i] |
| 433 | `HUF_compressWeights` | `FSE_writeNCount` fails | forwarded 44 / 1 / 70 | `compress/huf_compress.c:174` | [i] |
| 434 | `HUF_compressWeights` | `FSE_buildCTable_wksp` fails | forwarded `ZSTD_error_tableLog_tooLarge` (44) | `compress/huf_compress.c:179` | [i] |
| 435 | `HUF_compressWeights` | `FSE_compress_usingCTable` returned `cSize == 0` — **not** an error | `0` ("no room for compressed data" sentinel) | `compress/huf_compress.c:181` | [i] |
| 436 | `HUF_writeCTable_wksp` | `workspaceSize < sizeof(HUF_WriteCTableWksp)` (incl. the `HUF_alignUpWorkspace`-returned-NULL path) | `ZSTD_error_GENERIC` (1) | `compress/huf_compress.c:263` | [x] |
| 437 | `HUF_writeCTable_wksp` | `maxSymbolValue > HUF_SYMBOLVALUE_MAX (255)` | `ZSTD_error_maxSymbolValue_tooLarge` (46) | `compress/huf_compress.c:264` | [x] |
| 438 | `HUF_writeCTable_wksp` | `maxDstSize < 1` | `ZSTD_error_dstSize_tooSmall` (70) | `compress/huf_compress.c:274` | [x] |
| 439 | `HUF_writeCTable_wksp` | `HUF_compressWeights(op+1, maxDstSize-1, ...)` errors | forwarded 1 / 44 / 70 | `compress/huf_compress.c:275` | [x] |
| 440 | `HUF_writeCTable_wksp` | raw-4-bit fallback taken and `maxSymbolValue > (256-128)` = 128 | `ZSTD_error_GENERIC` (1) | `compress/huf_compress.c:282` | [x] |
| 441 | `HUF_writeCTable_wksp` | raw-4-bit fallback and `((maxSymbolValue+1)/2) + 1 > maxDstSize` | `ZSTD_error_dstSize_tooSmall` (70) | `compress/huf_compress.c:283` | [x] |
| 442 | `HUF_readCTable` (also reached from `ZSTD_loadCEntropy`) | `HUF_readStats(...)` errors | forwarded 20 / 72 / 44 / 46 / 48 / 70 / 1 | `compress/huf_compress.c:301` | [x] |
| 443 | `HUF_readCTable` | `tableLog > HUF_TABLELOG_MAX (12)` | `ZSTD_error_tableLog_tooLarge` (44) | `compress/huf_compress.c:305` | [x] |
| 444 | `HUF_readCTable` | `nbSymbols > *maxSymbolValuePtr + 1` | `ZSTD_error_maxSymbolValue_tooSmall` (48) | `compress/huf_compress.c:306` | [x] |
| 445 | `HUF_getNbBitsFromCTable` | `symbolValue > HUF_readCTableHeader(CTable).maxSymbolValue` — **not** an error (returns `U32`) | `0` ("symbol absent, 0 bits") | `compress/huf_compress.c:350` | [x] |
| 446 | `HUF_buildCTable_wksp` | `wkspSize < sizeof(HUF_buildCTable_wksp_tables)` (incl. `HUF_alignUpWorkspace` NULL) | `ZSTD_error_workSpace_tooSmall` (66) | `compress/huf_compress.c:770-771` | [x] |
| 447 | `HUF_buildCTable_wksp` | `maxSymbolValue > HUF_SYMBOLVALUE_MAX (255)` | `ZSTD_error_maxSymbolValue_tooLarge` (46) | `compress/huf_compress.c:773-774` | [x] |
| 448 | `HUF_buildCTable_wksp` | `maxNbBits` (after `HUF_setMaxHeight`) `> HUF_TABLELOG_MAX (12)` | `ZSTD_error_GENERIC` (1) | `compress/huf_compress.c:786` | [x] |
| 449 | `HUF_validateCTable` | `header.maxSymbolValue < maxSymbolValue` — boolean, not an error code (returns `int`) | `0` ("invalid") | `compress/huf_compress.c:813` | [x] |
| 450 | `HUF_validateCTable` | any `s <= maxSymbolValue` with `count[s] != 0 && HUF_getNbBits(ct[s]) == 0` | `0` ("invalid") via `return !bad` | `compress/huf_compress.c:817-820` | [x] |
| 451 | `HUF_initCStream` | `dstCapacity <= sizeof(bitC->bitContainer[0])` (8) | `ZSTD_error_dstSize_tooSmall` (70) | `compress/huf_compress.c:863` | [i] |
| 452 | `HUF_closeCStream` | `bitC->ptr >= bitC->endPtr` (bitstream overflowed dst) — **not** an error | `0` ("could not fit into dstBuffer" sentinel) | `compress/huf_compress.c:979` | [i] |
| 453 | `HUF_compress1X_usingCTable` (`_internal_body`) | `dstSize < 8` — **not** an error | `0` ("no room to compress" sentinel) | `compress/huf_compress.c:1068` | [x] |
| 454 | `HUF_compress1X_usingCTable` (`_internal_body`) | `HUF_initCStream` errors — the 70 from row 451 is **swallowed** | `0` ("incompressible" sentinel) | `compress/huf_compress.c:1071` | [x] |
| 455 | `HUF_compress4X_usingCTable` (`_internal`) | `dstSize < 6+1+1+1+8` (= 17) — **not** an error | `0` sentinel | `compress/huf_compress.c:1179` | [x] |
| 456 | `HUF_compress4X_usingCTable` (`_internal`) | `srcSize < 12` — **not** an error | `0` ("no saving possible" sentinel) | `compress/huf_compress.c:1180` | [x] |
| 457 | `HUF_compress4X_usingCTable` (`_internal`) | segment 1: `cSize == 0 \|\| cSize > 65535` (16-bit jump-table field) | `0` sentinel | `compress/huf_compress.c:1185` | [x] |
| 458 | `HUF_compress4X_usingCTable` (`_internal`) | segment 2: `cSize == 0 \|\| cSize > 65535` | `0` sentinel | `compress/huf_compress.c:1193` | [x] |
| 459 | `HUF_compress4X_usingCTable` (`_internal`) | segment 3: `cSize == 0 \|\| cSize > 65535` | `0` sentinel | `compress/huf_compress.c:1201` | [x] |
| 460 | `HUF_compress4X_usingCTable` (`_internal`) | segment 4 (last): `cSize == 0 \|\| cSize > 65535` | `0` sentinel | `compress/huf_compress.c:1210` | [x] |
| 461 | `HUF_compressCTable_internal` | `HUF_isError(cSize)` from the 1X/4X internal | forwarded (in practice only `ZSTD_error_dstSize_tooSmall` (70)) | `compress/huf_compress.c:1232` | [i] |
| 462 | `HUF_compressCTable_internal` | `cSize == 0` — **not** an error | `0` ("uncompressible" sentinel) | `compress/huf_compress.c:1233` | [i] |
| 463 | `HUF_compressCTable_internal` | `(size_t)(op-ostart) >= srcSize-1` (output not smaller than raw) | `0` ("uncompressible" sentinel) | `compress/huf_compress.c:1237` | [i] |
| 464 | `HUF_compress1X_repeat` / `HUF_compress4X_repeat` (`HUF_compress_internal`) | `wkspSize < sizeof(HUF_compress_tables_t)` (incl. `HUF_alignUpWorkspace` NULL) | `ZSTD_error_workSpace_tooSmall` (66) | `compress/huf_compress.c:1349` | [x] |
| 465 | `HUF_compress_internal` | `srcSize == 0` — **not** an error | `0` ("Uncompressed" sentinel) | `compress/huf_compress.c:1350` | [i] |
| 466 | `HUF_compress_internal` | `dstSize == 0` — **not** an error | `0` ("cannot fit within dst budget" sentinel) | `compress/huf_compress.c:1351` | [i] |
| 467 | `HUF_compress_internal` | `srcSize > HUF_BLOCKSIZE_MAX (131072)` | `ZSTD_error_srcSize_wrong` (72) | `compress/huf_compress.c:1352` | [i] |
| 468 | `HUF_compress_internal` | `huffLog > HUF_TABLELOG_MAX (12)` | `ZSTD_error_tableLog_tooLarge` (44) | `compress/huf_compress.c:1353` | [i] |
| 469 | `HUF_compress_internal` | `maxSymbolValue > HUF_SYMBOLVALUE_MAX (255)` | `ZSTD_error_maxSymbolValue_tooLarge` (46) | `compress/huf_compress.c:1354` | [i] |
| 470 | `HUF_compress_internal` | `HUF_flags_suspectUncompressible` set and `largestTotal <= ((2*SUSPECT_INCOMPRESSIBLE_SAMPLE_SIZE)>>7)+4` | `0` (heuristic "probably not compressible" sentinel) | `compress/huf_compress.c:1378` | [i] |
| 471 | `HUF_compress_internal` | `HIST_count_wksp(...)` errors | forwarded `ZSTD_error_GENERIC` (1) / `workSpace_tooSmall` (66) / `maxSymbolValue_tooSmall` (48) | `compress/huf_compress.c:1382` | [i] |
| 472 | `HUF_compress_internal` | `largest == srcSize` (single-symbol source) — **not** an error | `1` (RLE success; writes 1 byte) | `compress/huf_compress.c:1383` | [i] |
| 473 | `HUF_compress_internal` | `largest <= (srcSize >> 7) + 4` | `0` (heuristic "probably not compressible" sentinel) | `compress/huf_compress.c:1384` | [i] |
| 474 | `HUF_compress_internal` | `HUF_buildCTable_wksp(...)` errors | forwarded 66 / 46 / 1 (rows 446-448) | `compress/huf_compress.c:1406` | [i] |
| 475 | `HUF_compress_internal` | `HUF_writeCTable_wksp(...)` errors | forwarded 1 / 46 / 70 | `compress/huf_compress.c:1412` | [i] |
| 476 | `HUF_compress_internal` | `hSize + 12ul >= srcSize` (table header alone eats the budget) | `0` ("incompressible" sentinel) | `compress/huf_compress.c:1425` | [i] |
| 477 | `HIST_count_simple` | `srcSize == 0` (also sets `*maxSymbolValuePtr = 0`) — returns `unsigned`, never an error | `0` | `compress/hist.c:48` | [x] |
| 478 | `HIST_count_wksp` / `HIST_countFast_wksp` (`HIST_count_parallel_wksp`) | `sourceSize == 0` — zeroes `count[]`, sets `*maxSymbolValuePtr = 0` | `0` (legitimate "empty input" sentinel) | `compress/hist.c:96` | [x] |
| 479 | `HIST_count_wksp` (`HIST_count_parallel_wksp`) | `check && maxSymbolValue > *maxSymbolValuePtr` — source contains a symbol above the declared max | `ZSTD_error_maxSymbolValue_tooSmall` (48) | `compress/hist.c:138` | [x] |
| 480 | `HIST_countFast_wksp` | `((size_t)workSpace & 3) != 0` (not 4-byte aligned); only checked when `sourceSize >= 1500` | `ZSTD_error_GENERIC` (1) | `compress/hist.c:156` | [x] |
| 481 | `HIST_countFast_wksp` | `workSpaceSize < HIST_WKSP_SIZE`; only checked when `sourceSize >= 1500` | `ZSTD_error_workSpace_tooSmall` (66) | `compress/hist.c:157` | [x] |
| 482 | `HIST_count_wksp` | `((size_t)workSpace & 3) != 0` | `ZSTD_error_GENERIC` (1) | `compress/hist.c:168` | [x] |
| 483 | `HIST_count_wksp` | `workSpaceSize < HIST_WKSP_SIZE` | `ZSTD_error_workSpace_tooSmall` (66) | `compress/hist.c:169` | [x] |
| 484 | `HIST_count` / `HIST_countFast` (stack-wksp wrappers) | inherit rows 479-483; the alignment/size guards can't fire (stack `unsigned tmpCounters[HIST_WKSP_SIZE_U32]`) | `ZSTD_error_maxSymbolValue_tooSmall` (48) only | `compress/hist.c:178-190` | [x] |
## K3. Entropy coders — HUF (decompress side)

| # | function | trigger | expected C result | source | [ ] |
|---|----------|---------|-------------------|--------|-----|
| 485 | `HUF_readStats` / `HUF_readStats_wksp` (`_body`) | `srcSize == 0` | `ZSTD_error_srcSize_wrong` (72) | `common/entropy_common.c:254` | [x] |
| 486 | `HUF_readStats` / `_wksp` | raw 4-bit header (`iSize >= 128`) and `iSize+1 > srcSize` after `iSize = ((iSize-127+1)/2)` | `ZSTD_error_srcSize_wrong` (72) | `common/entropy_common.c:261` | [x] |
| 487 | `HUF_readStats` / `_wksp` | raw header and `oSize (= iSize-127) >= hwSize` | `ZSTD_error_corruption_detected` (20) | `common/entropy_common.c:262` | [x] |
| 488 | `HUF_readStats` / `_wksp` | FSE-compressed header (`iSize < 128`) and `iSize+1 > srcSize` | `ZSTD_error_srcSize_wrong` (72) | `common/entropy_common.c:270` | [x] |
| 489 | `HUF_readStats` / `_wksp` | `FSE_decompress_wksp_bmi2(huffWeight, hwSize-1, ip+1, iSize, 6, ...)` errors | forwarded 1 / 20 / 44 / 46 / 48 / 70 / 72 | `common/entropy_common.c:274` | [x] |
| 490 | `HUF_readStats` / `_wksp` | `huffWeight[n] > HUF_TABLELOG_MAX (12)` for some `n < oSize` | `ZSTD_error_corruption_detected` (20) | `common/entropy_common.c:280` | [x] |
| 491 | `HUF_readStats` / `_wksp` | `weightTotal == 0` | `ZSTD_error_corruption_detected` (20) | `common/entropy_common.c:284` | [x] |
| 492 | `HUF_readStats` / `_wksp` | `tableLog = ZSTD_highbit32(weightTotal)+1 > HUF_TABLELOG_MAX (12)` | `ZSTD_error_corruption_detected` (20) | `common/entropy_common.c:288` | [x] |
| 493 | `HUF_readStats` / `_wksp` | `(1<<ZSTD_highbit32(rest)) != ((1<<tableLog) - weightTotal)` — implied last weight isn't a clean power of 2 | `ZSTD_error_corruption_detected` (20) | `common/entropy_common.c:295` | [x] |
| 494 | `HUF_readStats` / `_wksp` | `(rankStats[1] < 2) \|\| (rankStats[1] & 1)` — invalid Huffman tree shape | `ZSTD_error_corruption_detected` (20) | `common/entropy_common.c:301` | [x] |
| 495 | `HUF_DecompressFastArgs_init` (via `HUF_decompress4X1/4X2_usingDTable_internal_fast`) | `!MEM_isLittleEndian() \|\| MEM_32bits()` | `0` ("fast loop unavailable, fall back" sentinel) | `decompress/huf_decompress.c:204` | [i] |
| 496 | `HUF_DecompressFastArgs_init` | `dstSize == 0` | `0` (fall-back sentinel) | `decompress/huf_decompress.c:208` | [i] |
| 497 | `HUF_DecompressFastArgs_init` | `srcSize < 10` (6-byte jump table + 1 byte per stream) | `ZSTD_error_corruption_detected` (20) | `decompress/huf_decompress.c:213` | [i] |
| 498 | `HUF_DecompressFastArgs_init` | `dtLog != HUF_DECODER_FAST_TABLELOG (11)` | `0` (fall-back sentinel) | `decompress/huf_decompress.c:220` | [i] |
| 499 | `HUF_DecompressFastArgs_init` | any of `length1..length4 < 8` | `0` (fall-back sentinel) | `decompress/huf_decompress.c:237` | [i] |
| 500 | `HUF_DecompressFastArgs_init` | `length4 (= srcSize - (length1+length2+length3+6)) > srcSize` — unsigned underflow of the jump table | `ZSTD_error_corruption_detected` (20) | `decompress/huf_decompress.c:238` | [i] |
| 501 | `HUF_DecompressFastArgs_init` | `args->op[3] >= oend` (output too small for the 4-way fast loop) | `0` (fall-back sentinel) | `decompress/huf_decompress.c:254` | [i] |
| 502 | `HUF_initRemainingDStream` | `args->op[stream] > segmentEnd` (a stream overwrote its segment) | `ZSTD_error_corruption_detected` (20) | `decompress/huf_decompress.c:285` | [i] |
| 503 | `HUF_initRemainingDStream` | `args->ip[stream] < args->iend[stream] - 8` (read beyond stream end) | `ZSTD_error_corruption_detected` (20) | `decompress/huf_decompress.c:292` | [i] |
| 504 | `HUF_readDTableX1_wksp` | `sizeof(HUF_ReadDTableX1_Workspace) > wkspSize` | `ZSTD_error_tableLog_tooLarge` (44) — **misleading code for a workspace failure; reproduce verbatim** | `decompress/huf_decompress.c:395` | [x] |
| 505 | `HUF_readDTableX1_wksp` | `HUF_isError(iSize)` from `HUF_readStats_wksp` | forwarded 72 / 20 / 44 / 46 / 48 / 70 / 1 | `decompress/huf_decompress.c:401` | [x] |
| 506 | `HUF_readDTableX1_wksp` | `tableLog > (U32)(dtd.maxTableLog+1)` after `HUF_rescaleStats` | `ZSTD_error_tableLog_tooLarge` (44) | `decompress/huf_decompress.c:409` | [x] |
| 507 | `HUF_decompress1X1_usingDTable_internal_body` | `BIT_initDStream(&bitD, cSrc, cSrcSize)` errors | forwarded 72 / 1 / 20 | `decompress/huf_decompress.c:588` | [i] |
| 508 | `HUF_decompress1X1_usingDTable_internal_body` | `!BIT_endOfDStream(&bitD)` after decoding | `ZSTD_error_corruption_detected` (20) | `decompress/huf_decompress.c:592` | [i] |
| 509 | `HUF_decompress4X1_usingDTable_internal_body` | `cSrcSize < 10` | `ZSTD_error_corruption_detected` (20) | `decompress/huf_decompress.c:608` | [i] |
| 510 | `HUF_decompress4X1_usingDTable_internal_body` | `dstSize < 6` (4-way split impossible) | `ZSTD_error_corruption_detected` (20) | `decompress/huf_decompress.c:609` | [i] |
| 511 | `HUF_decompress4X1_usingDTable_internal_body` | `length4 > cSrcSize` (jump-table underflow) | `ZSTD_error_corruption_detected` (20) | `decompress/huf_decompress.c:643` | [i] |
| 512 | `HUF_decompress4X1_usingDTable_internal_body` | `opStart4 > oend` | `ZSTD_error_corruption_detected` (20) | `decompress/huf_decompress.c:644` | [i] |
| 513 | `HUF_decompress4X1_usingDTable_internal_body` | any of the 4 `BIT_initDStream(&bitD1..4, istart1..4, length1..4)` errors | forwarded 72 / 1 / 20 | `decompress/huf_decompress.c:646,647,648,649` | [i] |
| 514 | `HUF_decompress4X1_usingDTable_internal_body` | `op1 > opStart2` | `ZSTD_error_corruption_detected` (20) | `decompress/huf_decompress.c:680` | [i] |
| 515 | `HUF_decompress4X1_usingDTable_internal_body` | `op2 > opStart3` | `ZSTD_error_corruption_detected` (20) | `decompress/huf_decompress.c:681` | [i] |
| 516 | `HUF_decompress4X1_usingDTable_internal_body` | `op3 > opStart4` | `ZSTD_error_corruption_detected` (20) | `decompress/huf_decompress.c:682` | [i] |
| 517 | `HUF_decompress4X1_usingDTable_internal_body` | `!endCheck` — any of `bitD1..4` not at end of stream | `ZSTD_error_corruption_detected` (20) | `decompress/huf_decompress.c:693` | [i] |
| 518 | `HUF_decompress4X1_usingDTable_internal_fast` | `HUF_DecompressFastArgs_init` errors | forwarded `ZSTD_error_corruption_detected` (20) | `decompress/huf_decompress.c:851` | [i] |
| 519 | `HUF_decompress4X1_usingDTable_internal_fast` | `HUF_DecompressFastArgs_init` returns 0 — **not** an error | `0` (signals the caller to use the fallback decoder) | `decompress/huf_decompress.c:853` | [i] |
| 520 | `HUF_decompress4X1_usingDTable_internal_fast` | `HUF_initRemainingDStream` errors for any `i` in 0..3 | forwarded `ZSTD_error_corruption_detected` (20) | `decompress/huf_decompress.c:883` | [i] |
| 521 | `HUF_decompress4X1_usingDTable_internal_fast` | `args.op[i] != segmentEnd` after finishing stream `i` (wrong decoded length) | `ZSTD_error_corruption_detected` (20) | `decompress/huf_decompress.c:886` | [i] |
| 522 | `HUF_decompress4X1_DCtx_wksp` (via `HUF_decompress4X_hufOnly_wksp`) | `HUF_isError(hSize)` from `HUF_readDTableX1_wksp` | forwarded 44 / 20 / 72 / 46 / 48 / 70 / 1 | `decompress/huf_decompress.c:937` | [x] |
| 523 | `HUF_decompress4X1_DCtx_wksp` | `hSize >= cSrcSize` (header consumes all input, no payload) | `ZSTD_error_srcSize_wrong` (72) | `decompress/huf_decompress.c:938` | [i] |
| 524 | `HUF_readDTableX2_wksp` | `sizeof(HUF_ReadDTableX2_Workspace) > wkspSize` | `ZSTD_error_GENERIC` (1) | `decompress/huf_decompress.c:1193` | [x] |
| 525 | `HUF_readDTableX2_wksp` | `dtd.maxTableLog > HUF_TABLELOG_MAX (12)` | `ZSTD_error_tableLog_tooLarge` (44) | `decompress/huf_decompress.c:1200` | [x] |
| 526 | `HUF_readDTableX2_wksp` | `HUF_isError(iSize)` from `HUF_readStats_wksp` | forwarded 72 / 20 / 44 / 46 / 48 / 70 / 1 | `decompress/huf_decompress.c:1204` | [x] |
| 527 | `HUF_readDTableX2_wksp` | `tableLog > maxTableLog` (DTable can't hold the code depth) | `ZSTD_error_tableLog_tooLarge` (44) | `decompress/huf_decompress.c:1207` | [x] |
| 528 | `HUF_decompress1X2_usingDTable_internal_body` | `BIT_initDStream(&bitD, cSrc, cSrcSize)` errors | forwarded 72 / 1 / 20 | `decompress/huf_decompress.c:1369` | [i] |
| 529 | `HUF_decompress1X2_usingDTable_internal_body` | `!BIT_endOfDStream(&bitD)` | `ZSTD_error_corruption_detected` (20) | `decompress/huf_decompress.c:1373` | [i] |
| 530 | `HUF_decompress4X2_usingDTable_internal_body` | `cSrcSize < 10` | `ZSTD_error_corruption_detected` (20) | `decompress/huf_decompress.c:1389` | [i] |
| 531 | `HUF_decompress4X2_usingDTable_internal_body` | `dstSize < 6` | `ZSTD_error_corruption_detected` (20) | `decompress/huf_decompress.c:1390` | [i] |
| 532 | `HUF_decompress4X2_usingDTable_internal_body` | `length4 > cSrcSize` (jump-table underflow) | `ZSTD_error_corruption_detected` (20) | `decompress/huf_decompress.c:1424` | [i] |
| 533 | `HUF_decompress4X2_usingDTable_internal_body` | `opStart4 > oend` | `ZSTD_error_corruption_detected` (20) | `decompress/huf_decompress.c:1425` | [i] |
| 534 | `HUF_decompress4X2_usingDTable_internal_body` | any of the 4 `BIT_initDStream(&bitD1..4, ...)` errors | forwarded 72 / 1 / 20 | `decompress/huf_decompress.c:1427-1430` | [i] |
| 535 | `HUF_decompress4X2_usingDTable_internal_body` | `op1 > opStart2` | `ZSTD_error_corruption_detected` (20) | `decompress/huf_decompress.c:1483` | [i] |
| 536 | `HUF_decompress4X2_usingDTable_internal_body` | `op2 > opStart3` | `ZSTD_error_corruption_detected` (20) | `decompress/huf_decompress.c:1484` | [i] |
| 537 | `HUF_decompress4X2_usingDTable_internal_body` | `op3 > opStart4` | `ZSTD_error_corruption_detected` (20) | `decompress/huf_decompress.c:1485` | [i] |
| 538 | `HUF_decompress4X2_usingDTable_internal_body` | `!endCheck` (any of `bitD1..4` not at end of stream) | `ZSTD_error_corruption_detected` (20) | `decompress/huf_decompress.c:1496` | [i] |
| 539 | `HUF_decompress4X2_usingDTable_internal_fast` | `HUF_DecompressFastArgs_init` errors | forwarded `ZSTD_error_corruption_detected` (20) | `decompress/huf_decompress.c:1678` | [i] |
| 540 | `HUF_decompress4X2_usingDTable_internal_fast` | `HUF_DecompressFastArgs_init` returns 0 — **not** an error | `0` (fall back to the generic decoder) | `decompress/huf_decompress.c:1680` | [i] |
| 541 | `HUF_decompress4X2_usingDTable_internal_fast` | `HUF_initRemainingDStream` errors | forwarded `ZSTD_error_corruption_detected` (20) | `decompress/huf_decompress.c:1708` | [i] |
| 542 | `HUF_decompress4X2_usingDTable_internal_fast` | `args.op[i] != segmentEnd` | `ZSTD_error_corruption_detected` (20) | `decompress/huf_decompress.c:1711` | [i] |
| 543 | `HUF_decompress1X2_DCtx_wksp` | `HUF_isError(hSize)` from `HUF_readDTableX2_wksp` | forwarded 1 / 44 / 20 / 72 / 46 / 48 / 70 | `decompress/huf_decompress.c:1762` | [x] |
| 544 | `HUF_decompress1X2_DCtx_wksp` | `hSize >= cSrcSize` | `ZSTD_error_srcSize_wrong` (72) | `decompress/huf_decompress.c:1763` | [x] |
| 545 | `HUF_decompress4X2_DCtx_wksp` (via `HUF_decompress4X_hufOnly_wksp`) | `HUF_isError(hSize)` from `HUF_readDTableX2_wksp` | forwarded | `decompress/huf_decompress.c:1777` | [x] |
| 546 | `HUF_decompress4X2_DCtx_wksp` | `hSize >= cSrcSize` | `ZSTD_error_srcSize_wrong` (72) | `decompress/huf_decompress.c:1778` | [i] |
| 547 | `HUF_decompress1X_DCtx_wksp` | `dstSize == 0` | `ZSTD_error_dstSize_tooSmall` (70) | `decompress/huf_decompress.c:1850` | [x] |
| 548 | `HUF_decompress1X_DCtx_wksp` | `cSrcSize > dstSize` | `ZSTD_error_corruption_detected` (20) | `decompress/huf_decompress.c:1851` | [x] |
| 549 | `HUF_decompress1X1_DCtx_wksp` | `HUF_isError(hSize)` from `HUF_readDTableX1_wksp` | forwarded 44 / 20 / 72 / 46 / 48 / 70 / 1 | `decompress/huf_decompress.c:1899` | [x] |
| 550 | `HUF_decompress1X1_DCtx_wksp` | `hSize >= cSrcSize` | `ZSTD_error_srcSize_wrong` (72) | `decompress/huf_decompress.c:1900` | [x] |
| 551 | `HUF_decompress4X_hufOnly_wksp` | `dstSize == 0` | `ZSTD_error_dstSize_tooSmall` (70) | `decompress/huf_decompress.c:1927` | [x] |
| 552 | `HUF_decompress4X_hufOnly_wksp` | `cSrcSize == 0` | `ZSTD_error_corruption_detected` (20) | `decompress/huf_decompress.c:1928` | [x] |
| 553 | `HUF_decompress1X_usingDTable` | no own guard — dispatches on `dtd.tableType` to the 1X1/1X2 internal bodies | inherits 20 / 72 / 1 | `decompress/huf_decompress.c:1876-1891` | [x] |
| 554 | `HUF_decompress4X_usingDTable` | no own guard — dispatches on `dtd.tableType` to the 4X1/4X2 internals | inherits 20 / 72 / 1 | `decompress/huf_decompress.c:1907-1922` | [x] |
**Entropy-surface error-code closure:** only `ZSTD_error_GENERIC` (1),
`corruption_detected` (20), `tableLog_tooLarge` (44), `maxSymbolValue_tooLarge` (46),
`maxSymbolValue_tooSmall` (48), `workSpace_tooSmall` (66), `dstSize_tooSmall` (70) and
`srcSize_wrong` (72) are producible by these files.

## L. HIST (histogram) — `compress/hist.c`

`HIST_*` shares the ZSTD numeric error space (`hist.c:19` includes
`error_private.h`; `ERROR(name) == (size_t)-ZSTD_error_<name>`), so
`HIST_isError` is exactly `ERR_isError`. Only **three** error codes are
producible by this file: `ZSTD_error_GENERIC` (1),
`ZSTD_error_workSpace_tooSmall` (66) and
`ZSTD_error_maxSymbolValue_tooSmall` (48).
Gating constants: `HIST_WKSP_SIZE_U32 == 1024`,
`HIST_WKSP_SIZE == 1024*sizeof(unsigned) == 4096` bytes (`hist.h:38-39`);
the parallel path is only taken for `srcSize >= 1500` (`hist.c:154`).

| # | function | trigger | expected C result | source | [ ] |
|---|----------|---------|-------------------|--------|-----|
| 555 | `HIST_isError` | not a rejection — pure predicate `ERR_isError(code)`, true iff `code > (size_t)-120` | `1` (is an error) / `0` (not an error); never itself fails | `compress/hist.c:24` | [x] |
| 556 | `HIST_add` | **no guard at all**: `src==NULL` with `srcSize>0`, or `count[]` shorter than 256 `unsigned`, is undefined behaviour (out-of-bounds write); `count` is *not* reset | no return value (`void`); cannot signal failure | `compress/hist.c:29-37` | [x] |
| 557 | `HIST_count_simple` | `srcSize == 0` | `0` (and `*maxSymbolValuePtr` forced to `0`) — **not** an error | `compress/hist.c:48` | [x] |
| 558 | `HIST_count_simple` | any byte in `src` `> *maxSymbolValuePtr` — only an `assert`, compiled out at `DEBUGLEVEL 0` | out-of-bounds write to `count[]` = UB; **no** error code (return type is `unsigned`, outside the error space) | `compress/hist.c:51-52` | [x] |
| 559 | `HIST_count_simple` | `*maxSymbolValuePtr` larger than needed but all in-range counts zero (only reachable once the `assert` above has already been violated): `while (!count[maxSymbolValue]) maxSymbolValue--;` underflows through `0` | unsigned wrap-around -> out-of-bounds read = UB | `compress/hist.c:55` | [x] |
| 560 | `HIST_count_simple` | `*maxSymbolValuePtr > 255` — never validated; `memset(count,0,(maxSymbolValue+1)*4)` writes past a 256-entry `count[]` | UB; documented as "must succeed" / "doesn't produce any error" | `compress/hist.c:47`, `compress/hist.h:66-75` | [x] |
| 561 | `HIST_count_parallel_wksp` (internal, reached from all 4 public wrappers) | `*maxSymbolValuePtr > 255` — `assert` only, compiled out at `DEBUGLEVEL 0`; `countSize=(*maxSymbolValuePtr+1)*4` then over-writes `count[]` in the final `ZSTD_memmove` | UB | `compress/hist.c:92`, `:84`, `:140` | [i] |
| 562 | `HIST_count_parallel_wksp` | `sourceSize == 0` | `0` (and `count` zeroed, `*maxSymbolValuePtr = 0`) — **not** an error | `compress/hist.c:93-97` | [i] |
| 563 | `HIST_count_parallel_wksp` | `check == checkMaxSymbolValue` and the detected largest symbol `> *maxSymbolValuePtr` | `ZSTD_error_maxSymbolValue_tooSmall` (48) | `compress/hist.c:138` | [i] |
| 564 | `HIST_countFast_wksp` | `sourceSize < 1500` -> tail-calls `HIST_count_simple`; **`workSpace` / `workSpaceSize` are not validated on this path** (a NULL / misaligned / 0-byte workspace is silently accepted) | whatever `HIST_count_simple` returns (never an error) | `compress/hist.c:154-155` | [x] |
| 565 | `HIST_countFast_wksp` | `(size_t)workSpace & 3` (not 4-byte aligned), with `sourceSize >= 1500` | `ZSTD_error_GENERIC` (1) | `compress/hist.c:156` | [x] |
| 566 | `HIST_countFast_wksp` | `workSpaceSize < HIST_WKSP_SIZE` (4096), with `sourceSize >= 1500` | `ZSTD_error_workSpace_tooSmall` (66) | `compress/hist.c:157` | [x] |
| 567 | `HIST_countFast_wksp` | any byte `> *maxSymbolValuePtr` — passes `trustInput`, so the `maxSymbolValue_tooSmall` check at `:138` is **skipped** | no error; OOB write to `count[]` = UB (documented "unsafe … will segfault") | `compress/hist.c:158`, `compress/hist.h:50-55` | [x] |
| 568 | `HIST_count_wksp` | `(size_t)workSpace & 3` (not 4-byte aligned) — checked **unconditionally**, even for tiny `srcSize` | `ZSTD_error_GENERIC` (1) | `compress/hist.c:168` | [x] |
| 569 | `HIST_count_wksp` | `workSpaceSize < HIST_WKSP_SIZE` (4096) — checked unconditionally | `ZSTD_error_workSpace_tooSmall` (66) | `compress/hist.c:169` | [x] |
| 570 | `HIST_count_wksp` | `*maxSymbolValuePtr < 255` and `src` contains a byte greater than it | `ZSTD_error_maxSymbolValue_tooSmall` (48) (via `checkMaxSymbolValue`) | `compress/hist.c:170-171` -> `:138` | [x] |
| 571 | `HIST_count_wksp` | `*maxSymbolValuePtr >= 255` (including any value `> 255`) | **silently clamped** to `255`, no error; then delegates to `HIST_countFast_wksp` (unchecked path) | `compress/hist.c:172-173` | [x] |
| 572 | `HIST_countFast` | its workspace is a `1024`-entry `unsigned` stack array, so the `GENERIC` (misalignment) and `workSpace_tooSmall` branches are **unreachable**; `trustInput` means no symbol-range check either | can only return a valid count; never an error | `compress/hist.c:178-183` | [x] |
| 573 | `HIST_count` | same stack workspace, so `GENERIC`/`workSpace_tooSmall` unreachable; the **only** reachable error is the symbol-range check when `*maxSymbolValuePtr < 255` | `ZSTD_error_maxSymbolValue_tooSmall` (48), else a valid count | `compress/hist.c:185-190` -> `:170-171` -> `:138` | [x] |
## M. xxhash (`common/xxhash.c` + `common/xxhash.h`)

`common/xxhash.c` is a 18-line stub that only sets `XXH_STATIC_LINKING_ONLY` +
`XXH_IMPLEMENTATION` and includes `xxhash.h` (`common/xxhash.c:15-18`), so every
definition cited below lives in `xxhash.h`.

Zstd forces two local adaptations at the very top of the header:

* `#define XXH_NO_XXH3` (`common/xxhash.h:14-16`) — the whole XXH3/XXH128 block
  (`:1033-1576`, `:1652-1991`, `:3664-7089`) is `#ifndef XXH_NO_XXH3`-guarded and
  therefore **not compiled**. There are **no** `ZSTD_XXH3_*` / `ZSTD_XXH128*`
  symbols to bind, and none of their `XXH_ASSERT`s / `XXH_ERROR` returns exist.
* `#define XXH_NAMESPACE ZSTD_` (`common/xxhash.h:18-20`) — every public symbol is
  renamed via `XXH_NAME2` (`:441-478`), i.e. the exported names are
  `ZSTD_XXH_versionNumber`, `ZSTD_XXH32`, `ZSTD_XXH32_createState`,
  `ZSTD_XXH32_freeState`, `ZSTD_XXH32_copyState`, `ZSTD_XXH32_reset`,
  `ZSTD_XXH32_update`, `ZSTD_XXH32_digest`, `ZSTD_XXH32_canonicalFromHash`,
  `ZSTD_XXH32_hashFromCanonical` and the `ZSTD_XXH64*` equivalents.

`XXH_NO_STREAM` and `XXH_NO_STDLIB` are **not** defined, so the streaming API is
compiled and `XXH_malloc`/`XXH_free` are plain `malloc`/`free`
(`common/xxhash.h:2337`, `:2343`).

### `XXH_errorcode` (`common/xxhash.h:560-563`)

| value | name |
|------:|------|
| 0 | `XXH_OK` |
| 1 | `XXH_ERROR` |

**Critical for an FFI port:** in this build **no reachable code path ever
returns `XXH_ERROR`.** `XXH32_freeState`, `XXH64_freeState`, `XXH32_reset`,
`XXH64_reset`, `XXH32_update` and `XXH64_update` all `return XXH_OK`
unconditionally. The only "failure" signal in the whole xxhash surface is
`XXH32_createState`/`XXH64_createState` returning `NULL`. The `XXH_ERROR`
enumerator exists but is dead in this configuration (it is only ever produced by
the `XXH3_*` and `XXH3_generateSecret` code, which is compiled out).

Also critical: `XXH_DEBUGLEVEL` defaults to `0` (`common/xxhash.h:2415-2421`), so
`XXH_ASSERT(c)` expands to `XXH_ASSUME(c)` (`:2425-2431`), i.e.
`__builtin_assume(c)` or `if (!(c)) { XXH_UNREACHABLE(); }`
(`:2707-2710`). Every `XXH_ASSERT` below is therefore **not** a runtime check but
an *optimiser promise*: violating it is UB, not a detected error.

| # | function | trigger | expected C result | source | [ ] |
|---|----------|---------|-------------------|--------|-----|
| 574 | `ZSTD_XXH_versionNumber` | none — no arguments, no failure mode | `XXH_VERSION_NUMBER` = `0*10000 + 8*100 + 2` = `802` (xxHash 0.8.2) | `common/xxhash.h:2826`, `:530-534` | [x] |
| 575 | `ZSTD_XXH32` | `input == NULL` with `len == 0` | valid hash of the empty input; **not** an error (the `NULL` is never dereferenced because `len<16` and `len&15==0`) | `common/xxhash.h:3044`, `:2964` | [x] |
| 576 | `ZSTD_XXH32` | `input == NULL` with `len != 0` | `XXH_ASSERT(len == 0)` is `XXH_ASSUME` at `DEBUGLEVEL 0` -> **UB** (NULL deref / unreachable), *not* an error return | `common/xxhash.h:3044`, `:2964` | [x] |
| 577 | `ZSTD_XXH32` | `len` larger than the real buffer | out-of-bounds read = UB; no length validation exists (return type `XXH32_hash_t` has no error encoding) | `common/xxhash.h:3073-3088` | [x] |
| 578 | `XXH32_finalize` (internal, reached from `ZSTD_XXH32` and `ZSTD_XXH32_digest`) | `len&15` outside `0..15` — impossible; the post-`switch` `XXH_ASSERT(0)` marks it unreachable | `return hash` (dead code); at `DEBUGLEVEL 0` the `XXH_ASSERT(0)` is `XXH_UNREACHABLE()` | `common/xxhash.h:3018-3019` | [x] |
| 579 | `ZSTD_XXH32_createState` | `XXH_malloc(sizeof(XXH32_state_t))` (i.e. `malloc`) fails | `NULL` — the **only** allocation-failure signal in the xxhash surface | `common/xxhash.h:3096-3099`, `:2337` | [x] |
| 580 | `ZSTD_XXH32_freeState` | `statePtr == NULL` — accepted (`free(NULL)` is a no-op); a non-`XXH*_createState` pointer is UB | `XXH_OK` (0) — **unconditionally**, never `XXH_ERROR` | `common/xxhash.h:3101-3105` | [x] |
| 581 | `ZSTD_XXH32_copyState` | `dstState == NULL` or `srcState == NULL` — **no NULL check**, straight `XXH_memcpy` | UB (`void` return; cannot signal failure) | `common/xxhash.h:3108-3111` | [x] |
| 582 | `ZSTD_XXH32_reset` | `statePtr == NULL` | `XXH_ASSERT(statePtr != NULL)` is `XXH_ASSUME` at `DEBUGLEVEL 0` -> **UB** (`memset(NULL,...)`); the documented `XXH_ERROR` (`xxhash.h:685-690`) is **never** returned | `common/xxhash.h:3114-3117` | [x] |
| 583 | `ZSTD_XXH32_reset` | any valid `statePtr` | `XXH_OK` (0) — unconditional | `common/xxhash.h:3125` | [x] |
| 584 | `ZSTD_XXH32_update` | `input == NULL` (any `len`) | returns `XXH_OK` (0) **early, silently**, without touching `state`; the `XXH_ASSERT(len==0)` is a no-op hint, so `NULL` + `len>0` is silently *ignored* rather than rejected | `common/xxhash.h:3129-3133` | [x] |
| 585 | `ZSTD_XXH32_update` | `state == NULL` with `input != NULL` | **no NULL check** — `state->total_len_32 += len` derefs NULL = UB | `common/xxhash.h:3138-3139` | [x] |
| 586 | `ZSTD_XXH32_update` | cumulative length `> 0xFFFFFFFF` (`total_len_32` is a `XXH32_hash_t`) | silently wraps modulo 2^32 (matches the XXH32 spec); no error | `common/xxhash.h:3138` | [x] |
| 587 | `ZSTD_XXH32_update` | `state` never `reset` (uninitialised / freshly `malloc`ed) | no stage/init check exists — garbage hash, no error | `common/xxhash.h:3128-3179` | [x] |
| 588 | `ZSTD_XXH32_update` | valid arguments (all three internal early-returns and the fall-through) | `XXH_OK` (0) on every path — `XXH_ERROR` is unreachable | `common/xxhash.h:3132`, `:3144`, `:3178` | [x] |
| 589 | `ZSTD_XXH32_digest` | `state == NULL` | **no NULL check** — `state->large_len` derefs NULL = UB; return type carries no error value | `common/xxhash.h:3182-3198` | [x] |
| 590 | `ZSTD_XXH32_digest` | called on a state that was never `reset` | no init check; garbage hash, no error. `digest` is non-destructive (takes `const`), so repeated calls are legal | `common/xxhash.h:3182` | [x] |
| 591 | `ZSTD_XXH32_canonicalFromHash` | `dst == NULL` — **no NULL check**, `XXH_memcpy(dst, ...)` | UB (`void` return) | `common/xxhash.h:3204-3209` | [x] |
| 592 | `ZSTD_XXH32_canonicalFromHash` | `sizeof(XXH32_canonical_t) != sizeof(XXH32_hash_t)` | compile-time `XXH_STATIC_ASSERT`, not a runtime rejection | `common/xxhash.h:3206` | [x] |
| 593 | `ZSTD_XXH32_hashFromCanonical` | `src == NULL` — **no NULL check**, `XXH_readBE32(src)` | UB; every 4-byte input is otherwise a valid hash (no invalid encodings) | `common/xxhash.h:3211-3214` | [x] |
| 594 | `ZSTD_XXH64` | `input == NULL` with `len == 0` | valid hash of the empty input; not an error | `common/xxhash.h:3486`, `:3441` | [x] |
| 595 | `ZSTD_XXH64` | `input == NULL` with `len != 0` | `XXH_ASSERT(len == 0)` -> `XXH_ASSUME` -> **UB**, not an error return | `common/xxhash.h:3486`, `:3441` | [x] |
| 596 | `ZSTD_XXH64` | `len` larger than the real buffer | out-of-bounds read = UB; no validation | `common/xxhash.h:3520-3535` | [x] |
| 597 | `ZSTD_XXH64_createState` | `XXH_malloc(sizeof(XXH64_state_t))` fails | `NULL` | `common/xxhash.h:3542-3545`, `:2337` | [x] |
| 598 | `ZSTD_XXH64_freeState` | `statePtr == NULL` accepted; foreign pointer is UB | `XXH_OK` (0) — unconditionally | `common/xxhash.h:3547-3551` | [x] |
| 599 | `ZSTD_XXH64_copyState` | `dstState`/`srcState` `NULL` — **no NULL check** | UB (`void` return) | `common/xxhash.h:3554-3557` | [x] |
| 600 | `ZSTD_XXH64_reset` | `statePtr == NULL` | `XXH_ASSERT` -> `XXH_ASSUME` -> **UB**; documented `XXH_ERROR` (`xxhash.h:955-960`) never returned | `common/xxhash.h:3560-3563` | [x] |
| 601 | `ZSTD_XXH64_reset` | any valid `statePtr` | `XXH_OK` (0) — unconditional | `common/xxhash.h:3571` | [x] |
| 602 | `ZSTD_XXH64_update` | `input == NULL` (any `len`) | `XXH_OK` (0) early return, state untouched; `NULL`+`len>0` silently ignored, not rejected | `common/xxhash.h:3574-3578` | [x] |
| 603 | `ZSTD_XXH64_update` | `state == NULL` with `input != NULL` | **no NULL check** — `state->total_len += len` = UB | `common/xxhash.h:3583` | [x] |
| 604 | `ZSTD_XXH64_update` | `state` never `reset` | no init/stage check; garbage hash, no error | `common/xxhash.h:3573-3620` | [x] |
| 605 | `ZSTD_XXH64_update` | valid arguments (both early-returns and the fall-through) | `XXH_OK` (0) on every path — `XXH_ERROR` unreachable | `common/xxhash.h:3577`, `:3588`, `:3620` | [x] |
| 606 | `ZSTD_XXH64_digest` | `state == NULL` | **no NULL check** — `state->total_len` deref = UB | `common/xxhash.h:3624-3641` | [x] |
| 607 | `ZSTD_XXH64_digest` | state never `reset`; note the tail length passed on is `(size_t)state->total_len`, masked to `&31` inside `XXH64_finalize` | no error possible (return type is a plain hash) | `common/xxhash.h:3640`, `:3439-3441` | [x] |
| 608 | `ZSTD_XXH64_canonicalFromHash` | `dst == NULL` — **no NULL check** | UB (`void` return) | `common/xxhash.h:3647-3652` | [x] |
| 609 | `ZSTD_XXH64_canonicalFromHash` | `sizeof(XXH64_canonical_t) != sizeof(XXH64_hash_t)` | compile-time `XXH_STATIC_ASSERT`, not runtime | `common/xxhash.h:3649` | [x] |
| 610 | `ZSTD_XXH64_hashFromCanonical` | `src == NULL` — **no NULL check**, `XXH_readBE64(src)` | UB; all 8-byte inputs are valid hashes | `common/xxhash.h:3655-3658` | [x] |
| 611 | any `ZSTD_XXH3_*` / `ZSTD_XXH128*` name | called across the FFI boundary | **link error / unresolved symbol** — the entire XXH3 family is excluded by the forced `XXH_NO_XXH3` | `common/xxhash.h:14-16`, `:3664`, `:7089` | [i] |

## N. dictBuilder (`dictBuilder/{zdict,cover,fastcover,divsufsort}.c`, `include/zdict.h`)

**There is no private `ZDICT_error_*` enum.** `include/zdict.h` declares only
`ZDICT_isError` (`:271`) and `ZDICT_getErrorName` (`:272`), and both are thin
aliases of the ZSTD predicates: `ZDICT_isError(e) == ERR_isError(e)`
(`dictBuilder/zdict.c:98`) and `ZDICT_getErrorName(e) == ERR_getErrorName(e)`
(`:100`). Every failure therefore lands in the **same** `ZSTD_error_*` numeric
space documented at the top of this file. The codes actually producible by the
dictBuilder are: `ZSTD_error_GENERIC` (1), `ZSTD_error_dictionary_corrupted` (30),
`ZSTD_error_dictionaryCreation_failed` (34), `ZSTD_error_parameter_outOfBound` (42),
`ZSTD_error_memory_allocation` (64), `ZSTD_error_dstSize_tooSmall` (70) and
`ZSTD_error_srcSize_wrong` (72) — plus any FSE/HUF/ZSTD code forwarded out of
`ZDICT_analyzeEntropy`.

### dictBuilder gating constants

| constant | value | source |
|---|---|---|
| `ZDICT_DICTSIZE_MIN` | 256 | `include/zdict.h:305` |
| `ZDICT_CONTENTSIZE_MIN` | 128 | `include/zdict.h:307` |
| `MINRATIO` | 4 | `dictBuilder/zdict.c:15` |
| `ZDICT_MAX_SAMPLES_SIZE` | `2000U << 20` = 2000 MB | `dictBuilder/zdict.c:16` |
| `ZDICT_MIN_SAMPLES_SIZE` | `ZDICT_CONTENTSIZE_MIN * MINRATIO` = **512** | `dictBuilder/zdict.c:17` |
| `g_selectivity_default` | 9 | `dictBuilder/zdict.c:70` |
| `OFFCODE_MAX` | 30 | `dictBuilder/zdict.c:657` |
| `HBUFFSIZE` (entropy-header scratch) | 256 | `dictBuilder/zdict.c:864` |
| `COVER_MAX_SAMPLES_SIZE` | `(unsigned)-1` = 4294967295 on 64-bit (`1 GB` on 32-bit) | `dictBuilder/cover.c:60` |
| `COVER_DEFAULT_SPLITPOINT` | 1.0 | `dictBuilder/cover.c:61` |
| `FASTCOVER_MAX_SAMPLES_SIZE` | `(unsigned)-1` on 64-bit (`1 GB` on 32-bit) | `dictBuilder/fastcover.c:42` |
| `FASTCOVER_MAX_F` | 31 | `dictBuilder/fastcover.c:43` |
| `FASTCOVER_MAX_ACCEL` | 10 | `dictBuilder/fastcover.c:44` |
| `FASTCOVER_DEFAULT_SPLITPOINT` | 0.75 | `dictBuilder/fastcover.c:45` |
| `DEFAULT_F` / `DEFAULT_ACCEL` | 20 / 1 | `dictBuilder/fastcover.c:46-47` |
| `d` default sweep (`d == 0`) | `kMinD=6`, `kMaxD=8`, step `+2` -> only 6 and 8 tried | `cover.c:1176-1177`, `fastcover.c:630-631` |
| `k` default sweep (`k == 0`) | `kMinK=50`, `kMaxK=2000` | `cover.c:1178-1179`, `fastcover.c:632-633` |
| `steps` default (`steps == 0`) | 40; `kStepSize = MAX((kMaxK-kMinK)/kSteps, 1)` | `cover.c:1180-1181`, `fastcover.c:634-635` |
| minimum training samples | **5** (`nbTrainSamples < 5` rejected) | `cover.c:621`, `fastcover.c:336` |
| minimum testing samples | **1** (`nbTestSamples < 1` rejected) | `cover.c:626`, `fastcover.c:342` |

Note on `d`: `FASTCOVER_checkParameters` hard-requires `d == 6 || d == 8`
(`fastcover.c:237-239`), but **`COVER_checkParameters` does not** — the plain
cover builder accepts any non-zero `d <= k` and silently switches comparator at
`d > 8` (`cover.c:684`). The "6 or 8" restriction only reaches cover through the
default `kMinD=6 .. kMaxD=8 step 2` sweep in
`ZDICT_optimizeTrainFromBuffer_cover`.

**Warning vs. hard error:** `COVER_warnOnSmallCorpus`
(`cover.c:689-705`) prints a `WARNING: The maximum dictionary size … is too large
compared to the source size` message when `nbDmers/maxDictSize < 10` and then
**continues**; it returns `void` and can never fail. Likewise
`ZDICT_trainBuffer_legacy` only *prints* `"sample set too large : reduced to
%u MB"` and silently drops trailing samples when `bufferSize >
ZDICT_MAX_SAMPLES_SIZE` (`zdict.c:501-502`). The genuinely-hard "corpus too
small" errors are `ZDICT_MIN_SAMPLES_SIZE` (rows below), `nbTrainSamples < 5`
and `nbTestSamples < 1`.

| # | function | trigger | expected C result | source | [ ] |
|---|----------|---------|-------------------|--------|-----|
| 612 | `ZDICT_isError` | not a rejection — alias of `ERR_isError` (true iff `code > (size_t)-120`) | `1` / `0`; never itself fails | `dictBuilder/zdict.c:98` | [x] |
| 613 | `ZDICT_getErrorName` | `errorCode` outside the error space (e.g. a real dictionary size) | alias of `ERR_getErrorName` -> `"No error detected"` for non-errors, `"Unspecified error code"` for unmapped ones; never NULL | `dictBuilder/zdict.c:100` | [x] |
| 614 | `ZDICT_getDictID` | `dictSize < 8` | `0` (sentinel "not a valid dictionary" — **not** an error code) | `dictBuilder/zdict.c:104` | [x] |
| 615 | `ZDICT_getDictID` | first 4 LE bytes `!= ZSTD_MAGIC_DICTIONARY` (`0xEC30A437`) | `0` | `dictBuilder/zdict.c:105` | [x] |
| 616 | `ZDICT_getDictID` | valid magic but the stored dictID is `0` (raw-content dict) | `0` — indistinguishable from the two rejections above | `dictBuilder/zdict.c:106` | [x] |
| 617 | `ZDICT_getDictID` | `dictBuffer == NULL` with `dictSize >= 8` — **no NULL check**, `MEM_readLE32(NULL)` | UB | `dictBuilder/zdict.c:105` | [x] |
| 618 | `ZDICT_getDictHeaderSize` | `dictSize <= 8` **or** magic `!= ZSTD_MAGIC_DICTIONARY` (single combined branch) | `ZSTD_error_dictionary_corrupted` (30) | `dictBuilder/zdict.c:112` | [x] |
| 619 | `ZDICT_getDictHeaderSize` | `malloc(sizeof(ZSTD_compressedBlockState_t))` or `malloc(HUF_WORKSPACE_SIZE)` fails | `ZSTD_error_memory_allocation` (64) | `dictBuilder/zdict.c:117` | [x] |
| 620 | `ZDICT_getDictHeaderSize` | `ZSTD_loadCEntropy` rejects the entropy tables (bad HUF/FSE headers, bad repcodes, short buffer) | forwarded `ZSTD_error_dictionary_corrupted` (30) (see section E) | `dictBuilder/zdict.c:121` | [x] |
| 621 | `ZDICT_trainBuffer_legacy` (internal, only reached from `ZDICT_trainFromBuffer_legacy`) | any of `suffix0`/`reverseSuffix`/`doneMarks`/`filePos` `malloc` fails | `ZSTD_error_memory_allocation` (64) | `dictBuilder/zdict.c:493-495` | [x] |
| 622 | `ZDICT_trainBuffer_legacy` | `bufferSize > ZDICT_MAX_SAMPLES_SIZE` (2000 MB) | **not** an error — prints a level-3 notice and silently drops trailing files until it fits | `dictBuilder/zdict.c:501-502` | [i] |
| 623 | `ZDICT_trainBuffer_legacy` | `divsufsort(...) != 0` | `ZSTD_error_GENERIC` (1) | `dictBuilder/zdict.c:507` | [i] |
| 624 | `divsufsort` | `T == NULL` **or** `SA == NULL` **or** `n < 0` | `-1` (`int`), which the caller maps to `ZSTD_error_GENERIC` (1) | `dictBuilder/divsufsort.c:1853` | [x] |
| 625 | `divsufsort` | `n == 0` | `0` (success, no-op) — **not** an error | `dictBuilder/divsufsort.c:1854-1856` | [x] |
| 626 | `divbwt` (compiled but unreferenced by zstd) | `T == NULL` or `U == NULL` or `n < 0` | `-1` | `dictBuilder/divsufsort.c:1882` | [x] |
| 627 | `ZDICT_analyzeEntropy` (internal; reached from `ZDICT_finalizeDictionary` and `ZDICT_addEntropyTablesFromBuffer`) | `ZSTD_highbit32(dictBufferSize + 128 KB) > OFFCODE_MAX` (30), i.e. dictionary + 128 KB >= 2 GB | `ZSTD_error_dictionaryCreation_failed` (34) | `dictBuilder/zdict.c:688` | [x] |
| 628 | `ZDICT_analyzeEntropy` | `ZSTD_createCDict_advanced` / `ZSTD_createCCtx` / `malloc(ZSTD_BLOCKSIZE_MAX)` returns NULL | `ZSTD_error_memory_allocation` (64) | `dictBuilder/zdict.c:702-705` | [i] |
| 629 | `ZDICT_analyzeEntropy` | `HUF_buildCTable_wksp` on the literal histogram errors | forwarded HUF code (44 / 46 / 66 / 1) | `dictBuilder/zdict.c:725-729` | [i] |
| 630 | `ZDICT_analyzeEntropy` | literal distribution not compressible (`maxNbBits == 8`) | **not** an error — histogram is replaced by a fake flat-but-compressible one via `ZDICT_flatLit` and `HUF_buildCTable_wksp` is retried | `dictBuilder/zdict.c:731-734` | [i] |
| 631 | `ZDICT_analyzeEntropy` | `FSE_normalizeCount(offcodeNCount, ...)` errors | forwarded FSE code (`ZSTD_error_GENERIC` (1) / `maxSymbolValue_tooLarge` (46) / `tableLog_tooLarge` (44)) | `dictBuilder/zdict.c:748-752` | [i] |
| 632 | `ZDICT_analyzeEntropy` | `FSE_normalizeCount(matchLengthNCount, ...)` errors | forwarded FSE code | `dictBuilder/zdict.c:757-761` | [i] |
| 633 | `ZDICT_analyzeEntropy` | `FSE_normalizeCount(litLengthNCount, ...)` errors | forwarded FSE code | `dictBuilder/zdict.c:766-770` | [i] |
| 634 | `ZDICT_analyzeEntropy` | `HUF_writeCTable_wksp` errors (notably `dstSize_tooSmall` against the 256-byte `header[]`) | forwarded HUF code (70 / 1 / 44) | `dictBuilder/zdict.c:775-779` | [i] |
| 635 | `ZDICT_analyzeEntropy` | `FSE_writeNCount` for the offcode table errors | forwarded FSE code (`dstSize_tooSmall` (70) / `GENERIC` (1)) | `dictBuilder/zdict.c:786-790` | [i] |
| 636 | `ZDICT_analyzeEntropy` | `FSE_writeNCount` for the matchLength table errors | forwarded FSE code | `dictBuilder/zdict.c:797-801` | [i] |
| 637 | `ZDICT_analyzeEntropy` | `FSE_writeNCount` for the litLength table errors | forwarded FSE code | `dictBuilder/zdict.c:808-812` | [i] |
| 638 | `ZDICT_analyzeEntropy` | fewer than 12 bytes left in `dstBuffer` after the entropy tables (no room for the 3 repcodes) | `ZSTD_error_dstSize_tooSmall` (70) | `dictBuilder/zdict.c:819-822` | [i] |
| 639 | `ZDICT_analyzeEntropy` | a per-sample `ZSTD_compressBegin_usingCDict` / `ZSTD_compressBlock` failure inside `ZDICT_countEStats` | **swallowed**: only a `DISPLAYLEVEL` warning, that sample contributes nothing; no error is propagated | `dictBuilder/zdict.c:576`, `:580` | [i] |
| 640 | `ZDICT_finalizeDictionary` | `dictBufferCapacity < dictContentSize` | `ZSTD_error_dstSize_tooSmall` (70) | `dictBuilder/zdict.c:874` | [x] |
| 641 | `ZDICT_finalizeDictionary` | `dictBufferCapacity < ZDICT_DICTSIZE_MIN` (256) | `ZSTD_error_dstSize_tooSmall` (70) | `dictBuilder/zdict.c:875` | [x] |
| 642 | `ZDICT_finalizeDictionary` | `ZDICT_analyzeEntropy` into the 256-byte `header[]` fails (any of rows 627-638) | that error, forwarded verbatim | `dictBuilder/zdict.c:894` | [x] |
| 643 | `ZDICT_finalizeDictionary` | `hSize + dictContentSize > dictBufferCapacity` | **not** an error — the content is silently **truncated** to `dictBufferCapacity - hSize` | `dictBuilder/zdict.c:899-901` | [x] |
| 644 | `ZDICT_finalizeDictionary` | content shorter than the largest repcode (`ZDICT_maxRep(repStartValue)`) **and** `hSize + minContentSize > dictBufferCapacity` | `ZSTD_error_dstSize_tooSmall` (70) ("dictBufferCapacity too small to fit max repcode") | `dictBuilder/zdict.c:904-906` | [x] |
| 645 | `ZDICT_finalizeDictionary` | content shorter than the largest repcode but the buffer *is* big enough | **not** an error — zero-padding is inserted before the content | `dictBuilder/zdict.c:904-909`, `:933` | [x] |
| 646 | `ZDICT_finalizeDictionary` | `params.dictID == 0` | **not** an error — a compliant random dictID is derived as `(XXH64(content) % ((1U<<31)-32768)) + 32768` | `dictBuilder/zdict.c:879-883` | [x] |
| 647 | `ZDICT_finalizeDictionary` | `nbSamples == 0` | **not rejected here**; `averageSampleSize = totalSrcSize/(nbFiles + !nbFiles)` avoids the division by zero and the stats loop simply runs zero times | `dictBuilder/zdict.c:682`, `:710-715` | [x] |
| 648 | `ZDICT_addEntropyTablesFromBuffer_advanced` (internal) | `ZDICT_analyzeEntropy` into `dictBuffer+8` fails | that error, forwarded | `dictBuilder/zdict.c:957` | [i] |
| 649 | `ZDICT_addEntropyTablesFromBuffer` (deprecated public wrapper) | `dictContentSize > dictBufferCapacity` — **no check**: `dictBufferCapacity - hSize` and `dictBuffer + dictBufferCapacity - dictContentSize` underflow | UB / out-of-bounds access; no rejection exists | `dictBuilder/zdict.c:1125-1132` -> `:952-955` | [x] |
| 650 | `ZDICT_addEntropyTablesFromBuffer` | `dictBufferCapacity < 8` (no room for the magic + dictID header) — **no check** | UB (`dictBufferCapacity - hSize` underflows with `hSize == 8`) | `dictBuilder/zdict.c:948`, `:952` | [x] |
| 651 | `ZDICT_trainFromBuffer_unsafe_legacy` (internal) | `malloc(dictListSize * sizeof(dictItem))` fails | `ZSTD_error_memory_allocation` (64) | `dictBuilder/zdict.c:993` | [i] |
| 652 | `ZDICT_trainFromBuffer_unsafe_legacy` | `maxDictSize < ZDICT_DICTSIZE_MIN` (256) | `ZSTD_error_dstSize_tooSmall` (70) | `dictBuilder/zdict.c:994` | [i] |
| 653 | `ZDICT_trainFromBuffer_unsafe_legacy` | `samplesBuffSize < ZDICT_MIN_SAMPLES_SIZE` (512) — "not enough source to create dictionary" | `ZSTD_error_dictionaryCreation_failed` (34) | `dictBuilder/zdict.c:995` | [i] |
| 654 | `ZDICT_trainFromBuffer_unsafe_legacy` | a selected segment lies outside the sample buffer (`pos > samplesBuffSize` or `pos+length > samplesBuffSize`) — only evaluated when `notificationLevel >= 3` | `ZSTD_error_GENERIC` (1) ("should never happen") | `dictBuilder/zdict.c:1007`, `:1018-1020` | [i] |
| 655 | `ZDICT_trainFromBuffer_unsafe_legacy` | total selected content `< ZDICT_CONTENTSIZE_MIN` (128) — "dictionary content too small" | `ZSTD_error_dictionaryCreation_failed` (34) | `dictBuilder/zdict.c:1030` | [i] |
| 656 | `ZDICT_trainFromBuffer_unsafe_legacy` | while filling the dict from the back, `ptr < dictBuffer` | `ZSTD_error_GENERIC` (1) ("should not happen") | `dictBuilder/zdict.c:1066` | [i] |
| 657 | `ZDICT_trainFromBuffer_unsafe_legacy` | `ZDICT_addEntropyTablesFromBuffer_advanced` fails | that error, forwarded as the function result | `dictBuilder/zdict.c:1071-1073` | [i] |
| 658 | `ZDICT_trainFromBuffer_legacy` | `ZDICT_totalSampleSize(samplesSizes, nbSamples) < ZDICT_MIN_SAMPLES_SIZE` (512) | **`0`** — a *success-shaped* sentinel meaning "no dictionary", **not** an error (`ZDICT_isError(0)` is false). Distinct from row 653, which returns code 34 for the same condition | `dictBuilder/zdict.c:1091` | [x] |
| 659 | `ZDICT_trainFromBuffer_legacy` | `malloc(sBuffSize + NOISELENGTH)` for the guard-band copy fails | `ZSTD_error_memory_allocation` (64) | `dictBuilder/zdict.c:1094` | [x] |
| 660 | `ZDICT_trainFromBuffer_legacy` | `params.selectivityLevel == 0` | **not** an error — replaced by `g_selectivity_default` (9) | `dictBuilder/zdict.c:985` | [x] |
| 661 | `ZDICT_trainFromBuffer_legacy` | `samplesSizes` claims more bytes than `samplesBuffer` actually holds | UB — `memcpy(newBuff, samplesBuffer, sBuffSize)` reads out of bounds; no validation is possible | `dictBuilder/zdict.c:1097` | [x] |
| 662 | `ZDICT_trainFromBuffer` | any rejection of the underlying `ZDICT_optimizeTrainFromBuffer_fastCover` (rows 686-696) | forwarded verbatim; the wrapper adds **no** checks of its own — it only pins `d=8`, `steps=4`, `compressionLevel=ZSTD_CLEVEL_DEFAULT` | `dictBuilder/zdict.c:1107-1123` | [x] |
| 663 | `COVER_checkParameters` | `parameters.d == 0` or `parameters.k == 0` | `0` (invalid), which the caller turns into `ZSTD_error_parameter_outOfBound` (42) | `dictBuilder/cover.c:551-554` | [i] |
| 664 | `COVER_checkParameters` | `parameters.k > maxDictSize` | `0` -> `ZSTD_error_parameter_outOfBound` (42) | `dictBuilder/cover.c:555-558` | [i] |
| 665 | `COVER_checkParameters` | `parameters.d > parameters.k` | `0` -> `ZSTD_error_parameter_outOfBound` (42) | `dictBuilder/cover.c:559-562` | [i] |
| 666 | `COVER_checkParameters` | `splitPoint <= 0` or `splitPoint > 1` | `0` -> `ZSTD_error_parameter_outOfBound` (42) | `dictBuilder/cover.c:563-566` | [i] |
| 667 | `COVER_checkParameters` | `d` not in `{6,8}` (e.g. 7, 12, 4) | **accepted** — unlike FASTCOVER there is no such check; `d > 8` silently switches the dmer comparator | `dictBuilder/cover.c:549-567`, `:684` | [i] |
| 668 | `COVER_ctx_init` | `totalSamplesSize < MAX(d, 8)` **or** `totalSamplesSize >= COVER_MAX_SAMPLES_SIZE` (`(unsigned)-1` on 64-bit) | `ZSTD_error_srcSize_wrong` (72) | `dictBuilder/cover.c:614-619` | [i] |
| 669 | `COVER_ctx_init` | `nbTrainSamples < 5` (with `splitPoint < 1.0`, `nbTrainSamples = nbSamples*splitPoint`) | `ZSTD_error_srcSize_wrong` (72) | `dictBuilder/cover.c:620-624` | [i] |
| 670 | `COVER_ctx_init` | `nbTestSamples < 1` | `ZSTD_error_srcSize_wrong` (72) | `dictBuilder/cover.c:625-629` | [i] |
| 671 | `COVER_ctx_init` | any of `malloc` for `suffix` / `dmerAt` / `offsets` fails | `ZSTD_error_memory_allocation` (64) (context destroyed first) | `dictBuilder/cover.c:648-652` | [i] |
| 672 | `COVER_ctx_init` | success | `0` — note `0` is *also* the "no error" value, so callers must use `ZSTD_isError`, not `!= 0` | `dictBuilder/cover.c:688` | [i] |
| 673 | `COVER_buildDictionary` | no segment covers any dmer for `MAX(10, MIN(100, epochs.num>>3))` consecutive epochs | **not** an error — loop breaks early and returns the current `tail`; the caller then finalizes a shorter dictionary | `dictBuilder/cover.c:734-758` | [i] |
| 674 | `COVER_buildDictionary` | the trimmed segment would be shorter than `parameters.d` | **not** an error — loop breaks, returns current `tail` | `dictBuilder/cover.c:761-764` | [i] |
| 675 | `ZDICT_trainFromBuffer_cover` | `COVER_checkParameters` fails (any of rows 663-666; `parameters.splitPoint` is force-set to `1.0` first) | `ZSTD_error_parameter_outOfBound` (42) | `dictBuilder/cover.c:791-794` | [x] |
| 676 | `ZDICT_trainFromBuffer_cover` | `nbSamples == 0` | `ZSTD_error_srcSize_wrong` (72) | `dictBuilder/cover.c:795-798` | [x] |
| 677 | `ZDICT_trainFromBuffer_cover` | `dictBufferCapacity < ZDICT_DICTSIZE_MIN` (256) | `ZSTD_error_dstSize_tooSmall` (70) | `dictBuilder/cover.c:799-803` | [x] |
| 678 | `ZDICT_trainFromBuffer_cover` | `COVER_ctx_init` fails (rows 668-671) | forwarded 72 / 64 | `dictBuilder/cover.c:806-811` | [x] |
| 679 | `ZDICT_trainFromBuffer_cover` | `COVER_map_init(&activeDmers, k - d + 1)` fails (hash-table allocation) | `ZSTD_error_memory_allocation` (64) | `dictBuilder/cover.c:813-817` | [x] |
| 680 | `ZDICT_trainFromBuffer_cover` | `ZDICT_finalizeDictionary` fails (rows 640-644) | forwarded 70 / 34 / 64 / FSE-HUF code | `dictBuilder/cover.c:824-826`, `:832` | [x] |
| 681 | `COVER_checkTotalCompressedSize` | `malloc(ZSTD_compressBound(maxSampleSize))` / `ZSTD_createCCtx` / `ZSTD_createCDict` fails | the pre-seeded `ZSTD_error_GENERIC` (1) | `dictBuilder/cover.c:843`, `:864-866` | [x] |
| 682 | `COVER_checkTotalCompressedSize` | any `ZSTD_compress_usingCDict` of a sample errors | that ZSTD error, forwarded | `dictBuilder/cover.c:871-879` | [x] |
| 683 | `COVER_dictSelectionError` | constructor for the failure form of `COVER_dictSelection_t` | `{ dictContent = NULL, dictSize = 0, totalCompressedSize = <error> }`; detected by `COVER_dictSelectionIsError` = `ZSTD_isError(totalCompressedSize) \|\| !dictContent` | `dictBuilder/cover.c:1008-1013` | [x] |
| 684 | `COVER_selectDict` | `malloc(dictBufferCapacity)` for either `largestDictbuffer` or `candidateDictBuffer` fails | `COVER_dictSelectionError(dictContentSize)` — note the *size* is stuffed into the error slot, so a `dictContentSize > (size_t)-120` would be misread as an error and a small one as success-with-NULL-content (still caught by the `!dictContent` half of the predicate) | `dictBuilder/cover.c:1031-1036` | [x] |
| 685 | `COVER_selectDict` | the initial `ZDICT_finalizeDictionary` fails | `COVER_dictSelectionError(<that error>)` | `dictBuilder/cover.c:1043-1048` | [x] |
| 686 | `COVER_selectDict` | the initial `COVER_checkTotalCompressedSize` fails | `COVER_dictSelectionError(<that error>)` | `dictBuilder/cover.c:1054-1059` | [x] |
| 687 | `COVER_selectDict` | in the `shrinkDict` loop, `ZDICT_finalizeDictionary` on a candidate fails | `COVER_dictSelectionError(<that error>)` | `dictBuilder/cover.c:1076-1081` | [x] |
| 688 | `COVER_selectDict` | in the `shrinkDict` loop, `COVER_checkTotalCompressedSize` on a candidate fails | `COVER_dictSelectionError(<that error>)` | `dictBuilder/cover.c:1088-1093` | [x] |
| 689 | `COVER_tryParameters` | `COVER_map_init` fails, or `malloc(dictBufferCapacity)` / `malloc(suffixSize*4)` fails, or `COVER_selectDict` fails | `void` — the failure is recorded by handing the pre-seeded `COVER_dictSelectionError(ERROR(GENERIC))` to `COVER_best_finish`, which the caller later observes as `best.compressedSize` being an error | `dictBuilder/cover.c:1133-1142`, `:1153-1156`, `:1158-1160` | [i] |
| 690 | `COVER_best_finish` | `malloc(dictSize)` for the new best dictionary fails | records `best->compressedSize = ERROR(GENERIC)` (1) and `best->dictSize = 0` | `dictBuilder/cover.c:977-983` | [x] |
| 691 | `ZDICT_optimizeTrainFromBuffer_cover` | `splitPoint <= 0` or `> 1` (after the `<= 0.0 -> COVER_DEFAULT_SPLITPOINT` substitution, so only `> 1` is reachable) | `ZSTD_error_parameter_outOfBound` (42) | `dictBuilder/cover.c:1194-1197` | [x] |
| 692 | `ZDICT_optimizeTrainFromBuffer_cover` | `kMinK < kMaxD` or `kMaxK < kMinK` (i.e. `k` smaller than `d`, or an inverted explicit `k` range) | `ZSTD_error_parameter_outOfBound` (42) | `dictBuilder/cover.c:1198-1201` | [x] |
| 693 | `ZDICT_optimizeTrainFromBuffer_cover` | `nbSamples == 0` | `ZSTD_error_srcSize_wrong` (72) | `dictBuilder/cover.c:1202-1206` | [x] |
| 694 | `ZDICT_optimizeTrainFromBuffer_cover` | `dictBufferCapacity < ZDICT_DICTSIZE_MIN` (256) | `ZSTD_error_dstSize_tooSmall` (70) | `dictBuilder/cover.c:1207-1211` | [x] |
| 695 | `ZDICT_optimizeTrainFromBuffer_cover` | `nbThreads > 1` and `POOL_create` fails | `ZSTD_error_memory_allocation` (64). Without `ZSTD_MULTITHREAD` `POOL_create` returns `&g_poolCtx` and can never fail, so this branch is unreachable in this build — but `nbThreads > 1` is *not* rejected either, it is silently single-threaded | `dictBuilder/cover.c:1212-1217`, `common/pool.c:326-337` | [x] |
| 696 | `ZDICT_optimizeTrainFromBuffer_cover` | `COVER_ctx_init` fails for some `d` in the sweep | that error, forwarded (whole call aborts) | `dictBuilder/cover.c:1229-1237` | [x] |
| 697 | `ZDICT_optimizeTrainFromBuffer_cover` | `malloc(sizeof(COVER_tryParameters_data_t))` fails | `ZSTD_error_memory_allocation` (64) | `dictBuilder/cover.c:1248-1255` | [x] |
| 698 | `ZDICT_optimizeTrainFromBuffer_cover` | `COVER_checkParameters` rejects one generated `(d,k)` pair | **not** an error — that iteration is silently `continue`d | `dictBuilder/cover.c:1266-1270` | [x] |
| 699 | `ZDICT_optimizeTrainFromBuffer_cover` | every iteration was skipped or failed, so `best.compressedSize` is still an error | that error, forwarded (`ZSTD_error_GENERIC` (1) if nothing ever ran) | `dictBuilder/cover.c:1287-1292` | [x] |
| 700 | `FASTCOVER_checkParameters` | `parameters.d == 0` or `parameters.k == 0` | `0` -> caller returns `ZSTD_error_parameter_outOfBound` (42) | `dictBuilder/fastcover.c:232-235` | [i] |
| 701 | `FASTCOVER_checkParameters` | `parameters.d != 6 && parameters.d != 8` | `0` -> `ZSTD_error_parameter_outOfBound` (42) | `dictBuilder/fastcover.c:236-239` | [i] |
| 702 | `FASTCOVER_checkParameters` | `parameters.k > maxDictSize` | `0` -> `ZSTD_error_parameter_outOfBound` (42) | `dictBuilder/fastcover.c:240-243` | [i] |
| 703 | `FASTCOVER_checkParameters` | `parameters.d > parameters.k` | `0` -> `ZSTD_error_parameter_outOfBound` (42) | `dictBuilder/fastcover.c:244-247` | [i] |
| 704 | `FASTCOVER_checkParameters` | `f > FASTCOVER_MAX_F` (31) or `f == 0` | `0` -> `ZSTD_error_parameter_outOfBound` (42) | `dictBuilder/fastcover.c:248-251` | [i] |
| 705 | `FASTCOVER_checkParameters` | `splitPoint <= 0` or `> 1` | `0` -> `ZSTD_error_parameter_outOfBound` (42) | `dictBuilder/fastcover.c:252-255` | [i] |
| 706 | `FASTCOVER_checkParameters` | `accel > 10` or `accel == 0` (literal `10`, equal to `FASTCOVER_MAX_ACCEL`) | `0` -> `ZSTD_error_parameter_outOfBound` (42) | `dictBuilder/fastcover.c:256-259` | [i] |
| 707 | `FASTCOVER_ctx_init` | `totalSamplesSize < MAX(d, 8)` **or** `>= FASTCOVER_MAX_SAMPLES_SIZE` | `ZSTD_error_srcSize_wrong` (72) | `dictBuilder/fastcover.c:328-333` | [i] |
| 708 | `FASTCOVER_ctx_init` | `nbTrainSamples < 5` | `ZSTD_error_srcSize_wrong` (72) | `dictBuilder/fastcover.c:335-339` | [i] |
| 709 | `FASTCOVER_ctx_init` | `nbTestSamples < 1` | `ZSTD_error_srcSize_wrong` (72) | `dictBuilder/fastcover.c:341-345` | [i] |
| 710 | `FASTCOVER_ctx_init` | `calloc(nbSamples+1, sizeof(size_t))` for `offsets` fails | `ZSTD_error_memory_allocation` (64) | `dictBuilder/fastcover.c:365-370` | [i] |
| 711 | `FASTCOVER_ctx_init` | `calloc(1<<f, sizeof(U32))` for the frequency table fails (`f == 31` needs 8 GB) | `ZSTD_error_memory_allocation` (64) | `dictBuilder/fastcover.c:382-387` | [i] |
| 712 | `FASTCOVER_ctx_init` | success | `0` (same overload of `0` as `COVER_ctx_init`) | `dictBuilder/fastcover.c:392` | [i] |
| 713 | `ZDICT_trainFromBuffer_fastCover` | `f == 0` / `accel == 0` in the caller's params | **not** an error — substituted with `DEFAULT_F` (20) / `DEFAULT_ACCEL` (1); `splitPoint` is force-set to `1.0` | `dictBuilder/fastcover.c:561-563` | [x] |
| 714 | `ZDICT_trainFromBuffer_fastCover` | `FASTCOVER_checkParameters` fails (rows 700-706) | `ZSTD_error_parameter_outOfBound` (42) | `dictBuilder/fastcover.c:568-572` | [x] |
| 715 | `ZDICT_trainFromBuffer_fastCover` | `nbSamples == 0` | `ZSTD_error_srcSize_wrong` (72) | `dictBuilder/fastcover.c:573-576` | [x] |
| 716 | `ZDICT_trainFromBuffer_fastCover` | `dictBufferCapacity < ZDICT_DICTSIZE_MIN` (256) | `ZSTD_error_dstSize_tooSmall` (70) | `dictBuilder/fastcover.c:577-581` | [x] |
| 717 | `ZDICT_trainFromBuffer_fastCover` | `FASTCOVER_ctx_init` fails (rows 707-711) | forwarded 72 / 64 | `dictBuilder/fastcover.c:586-592` | [x] |
| 718 | `ZDICT_trainFromBuffer_fastCover` | `calloc(1<<f, sizeof(U16))` for `segmentFreqs` fails — **result is never checked** | `FASTCOVER_buildDictionary` dereferences NULL = UB | `dictBuilder/fastcover.c:599-601` | [x] |
| 719 | `ZDICT_trainFromBuffer_fastCover` | `ZDICT_finalizeDictionary` fails (rows 640-644) | forwarded verbatim | `dictBuilder/fastcover.c:603-612` | [x] |
| 720 | `FASTCOVER_tryParameters` | `calloc`/`malloc` for `segmentFreqs` / `dict` / `freqs` fails, or `COVER_selectDict` fails | `void` — recorded via the pre-seeded `COVER_dictSelectionError(ERROR(GENERIC))` handed to `COVER_best_finish` | `dictBuilder/fastcover.c:485-490`, `:503-506` | [i] |
| 721 | `ZDICT_optimizeTrainFromBuffer_fastCover` | `splitPoint <= 0` or `> 1` (after the `<= 0.0 -> 0.75` substitution, only `> 1` is reachable) | `ZSTD_error_parameter_outOfBound` (42) | `dictBuilder/fastcover.c:649-653` | [x] |
| 722 | `ZDICT_optimizeTrainFromBuffer_fastCover` | `accel == 0 \|\| accel > FASTCOVER_MAX_ACCEL` (after the `== 0 -> DEFAULT_ACCEL` substitution, only `> 10` is reachable) | `ZSTD_error_parameter_outOfBound` (42) | `dictBuilder/fastcover.c:654-657` | [x] |
| 723 | `ZDICT_optimizeTrainFromBuffer_fastCover` | `kMinK < kMaxD` or `kMaxK < kMinK` | `ZSTD_error_parameter_outOfBound` (42) | `dictBuilder/fastcover.c:658-661` | [x] |
| 724 | `ZDICT_optimizeTrainFromBuffer_fastCover` | `nbSamples == 0` | `ZSTD_error_srcSize_wrong` (72) | `dictBuilder/fastcover.c:662-665` | [x] |
| 725 | `ZDICT_optimizeTrainFromBuffer_fastCover` | `dictBufferCapacity < ZDICT_DICTSIZE_MIN` (256) | `ZSTD_error_dstSize_tooSmall` (70) | `dictBuilder/fastcover.c:666-670` | [x] |
| 726 | `ZDICT_optimizeTrainFromBuffer_fastCover` | `nbThreads > 1` and `POOL_create` fails (unreachable without `ZSTD_MULTITHREAD`; `nbThreads > 1` is otherwise silently ignored) | `ZSTD_error_memory_allocation` (64) | `dictBuilder/fastcover.c:671-676` | [x] |
| 727 | `ZDICT_optimizeTrainFromBuffer_fastCover` | `FASTCOVER_ctx_init` fails for some `d` in the sweep | that error, forwarded | `dictBuilder/fastcover.c:692-699` | [x] |
| 728 | `ZDICT_optimizeTrainFromBuffer_fastCover` | `malloc(sizeof(FASTCOVER_tryParameters_data_t))` fails | `ZSTD_error_memory_allocation` (64) | `dictBuilder/fastcover.c:710-717` | [x] |
| 729 | `ZDICT_optimizeTrainFromBuffer_fastCover` | `FASTCOVER_checkParameters` rejects a generated `(d,k)` pair | **not** an error — that iteration is silently `continue`d | `dictBuilder/fastcover.c:729-734` | [x] |
| 730 | `ZDICT_optimizeTrainFromBuffer_fastCover` | every iteration was skipped or failed, so `best.compressedSize` is still an error | that error, forwarded (`ZSTD_error_GENERIC` (1) if nothing ever ran); note `memcpy(dictBuffer, best.dict, dictSize)` on the success path assumes `dictBufferCapacity >= dictSize` | `dictBuilder/fastcover.c:751-762` | [x] |
## O. Deprecated ZBUFF (`deprecated/zbuff_{common,compress,decompress}.c`, `deprecated/zbuff.h`)

ZBUFF has **no error space of its own**. `ZBUFF_isError` and
`ZBUFF_getErrorName` are literal aliases of `ERR_isError` /
`ERR_getErrorName` (`deprecated/zbuff_common.c:23`, `:26`), i.e. the same
`ZSTD_error_*` numbers as everything else in this document.

In zstd 1.5.7 ZBUFF is a **pure shim**: `ZBUFF_CCtx` is a `typedef` of
`ZSTD_CStream` and `ZBUFF_DCtx` of `ZSTD_DStream` (`deprecated/zbuff.h:66-67`,
`:122-123`), and every entry point forwards to the modern API. It keeps **no
private stage machine**, so all stage/ordering violations are detected (or not)
by the underlying `ZSTD_compressStream2` / `ZSTD_decompressStream` — see
sections C and I. Every function is declared with `ZBUFF_DEPRECATED(...)`
(`deprecated/zbuff.h:45-57`), which is only a compiler attribute and never
affects runtime behaviour.

| # | function | trigger | expected C result | source | [ ] |
|---|----------|---------|-------------------|--------|-----|
| 731 | `ZBUFF_isError` | not a rejection — alias of `ERR_isError` | `1` / `0`; never fails | `deprecated/zbuff_common.c:23` | [x] |
| 732 | `ZBUFF_getErrorName` | `errorCode` not in the error space | alias of `ERR_getErrorName` -> `"No error detected"`; unmapped codes -> `"Unspecified error code"`; never NULL | `deprecated/zbuff_common.c:26` | [x] |
| 733 | `ZBUFF_createCCtx` | `ZSTD_createCStream()` allocation fails | `NULL` | `deprecated/zbuff_compress.c:54-57` | [x] |
| 734 | `ZBUFF_createCCtx_advanced` | exactly one of `customMem.customAlloc`/`customFree` is NULL (XOR), or the allocation fails | `NULL` (inherited from `ZSTD_createCStream_advanced` -> `ZSTD_createCCtx_advanced`, `compress/zstd_compress.c:118-120`) | `deprecated/zbuff_compress.c:59-62` | [x] |
| 735 | `ZBUFF_freeCCtx` | `zbc == NULL` | `0` (accepted no-op, inherited from `ZSTD_freeCStream`/`ZSTD_freeCCtx`) | `deprecated/zbuff_compress.c:64-67` | [x] |
| 736 | `ZBUFF_freeCCtx` | `zbc` was created with `ZSTD_initStaticCCtx` (static context) | `ZSTD_error_memory_allocation` (64) — inherited "not compatible with static CCtx" check | `deprecated/zbuff_compress.c:66` | [x] |
| 737 | `ZBUFF_compressInit` | `ZSTD_CCtx_reset(zbc, ZSTD_reset_session_only)` fails, `ZSTD_CCtx_refCDict` fails, or `ZSTD_CCtx_setParameter(ZSTD_c_compressionLevel, ...)` fails | forwarded from `ZSTD_initCStream`: `ZSTD_error_stage_wrong` (60) / `ZSTD_error_parameter_unsupported` (40); note `compressionLevel` is **clamped**, never rejected | `deprecated/zbuff_compress.c:105-108` -> `compress/zstd_compress.c:6077-6084` | [x] |
| 738 | `ZBUFF_compressInit` | `zbc == NULL` | **no NULL check anywhere in the chain** -> `ZSTD_CCtx_reset(NULL, ...)` dereferences NULL = UB | `deprecated/zbuff_compress.c:107` | [x] |
| 739 | `ZBUFF_compressInitDictionary` | `ZSTD_CCtx_reset(..., ZSTD_reset_session_only)` fails because a compression is already in flight | `ZSTD_error_stage_wrong` (60) | `deprecated/zbuff_compress.c:99` | [x] |
| 740 | `ZBUFF_compressInitDictionary` | `ZSTD_CCtx_setParameter(ZSTD_c_compressionLevel, ...)` rejected (only possible if the cctx is mid-stream) | `ZSTD_error_stage_wrong` (60) | `deprecated/zbuff_compress.c:100` | [x] |
| 741 | `ZBUFF_compressInitDictionary` | `ZSTD_CCtx_loadDictionary(zbc, dict, dictSize)` fails (`dict != NULL && dictSize == 0` mismatch, allocation failure, or a corrupt `ZSTD_dct_auto` dict) | forwarded `ZSTD_error_memory_allocation` (64) / `ZSTD_error_dictionary_corrupted` (30) / `ZSTD_error_stage_wrong` (60) | `deprecated/zbuff_compress.c:101` | [x] |
| 742 | `ZBUFF_compressInit_advanced` | `pledgedSrcSize == 0` | **not** an error — remapped to `ZSTD_CONTENTSIZE_UNKNOWN` to preserve the old "0 == unknown" convention, so a genuinely empty frame cannot be pledged through this API | `deprecated/zbuff_compress.c:76` | [x] |
| 743 | `ZBUFF_compressInit_advanced` | `ZSTD_CCtx_reset(zbc, ZSTD_reset_session_only)` fails | `ZSTD_error_stage_wrong` (60) | `deprecated/zbuff_compress.c:77` | [x] |
| 744 | `ZBUFF_compressInit_advanced` | `ZSTD_CCtx_setPledgedSrcSize` fails | forwarded `ZSTD_error_stage_wrong` (60) | `deprecated/zbuff_compress.c:78` | [x] |
| 745 | `ZBUFF_compressInit_advanced` | `ZSTD_checkCParams(params.cParams)` fails on any of windowLog / chainLog / hashLog / searchLog / minMatch / targetLength / strategy | `ZSTD_error_parameter_outOfBound` (42) | `deprecated/zbuff_compress.c:80` -> `compress/zstd_compress.c:1390-1396` | [x] |
| 746 | `ZBUFF_compressInit_advanced` | any of the 7 `ZSTD_CCtx_setParameter` calls for `windowLog`/`hashLog`/`chainLog`/`searchLog`/`minMatch`/`targetLength`/`strategy` is rejected (redundant with row 745 but a separate branch each) | `ZSTD_error_parameter_outOfBound` (42) | `deprecated/zbuff_compress.c:81`, `:82`, `:83`, `:84`, `:85`, `:86`, `:87` | [x] |
| 747 | `ZBUFF_compressInit_advanced` | any of the 3 frame-flag `ZSTD_CCtx_setParameter` calls (`contentSizeFlag`, `checksumFlag`, `dictIDFlag`) is rejected | `ZSTD_error_stage_wrong` (60) — the flag values themselves are coerced to 0/1 and never rejected | `deprecated/zbuff_compress.c:89`, `:90`, `:91` | [x] |
| 748 | `ZBUFF_compressInit_advanced` | **semantic bug preserved from the original**: `params.fParams.noDictIDFlag` is passed straight into `ZSTD_c_dictIDFlag` without inverting it, so `noDictIDFlag=1` *enables* dictID emission | no error — silently wrong frame header | `deprecated/zbuff_compress.c:91` | [x] |
| 749 | `ZBUFF_compressInit_advanced` | `ZSTD_CCtx_loadDictionary(zbc, dict, dictSize)` fails | forwarded 64 / 30 / 60 | `deprecated/zbuff_compress.c:93` | [x] |
| 750 | `ZBUFF_compressInit_advanced` | success | `0` (whereas `ZSTD_initCStream*` returns `0` too — both are "no error", not a size hint) | `deprecated/zbuff_compress.c:94` | [x] |
| 751 | `ZBUFF_compressContinue` | `dstCapacityPtr == NULL` or `srcSizePtr == NULL` | **no NULL check** — `*dstCapacityPtr` / `*srcSizePtr` dereferenced immediately = UB | `deprecated/zbuff_compress.c:122`, `:125` | [x] |
| 752 | `ZBUFF_compressContinue` | called **before any `ZBUFF_compressInit*`** on a freshly created context | *not* rejected: `ZSTD_createCStream` leaves the cctx in `ZSTDcs_created`/`zcss_init` with clevel 3, so `ZSTD_compressStream` starts a default-parameter frame. There is **no** `init_missing` check on the compression side | `deprecated/zbuff_compress.c:126` -> `compress/zstd_compress.c` `ZSTD_compressStream2` | [x] |
| 753 | `ZBUFF_compressContinue` | called **after** `ZBUFF_compressEnd` has completed the frame, without re-init | *not* rejected — `ZSTD_compressStream` implicitly begins a **new** frame (`zcss_init` was restored at end-of-frame) | `deprecated/zbuff_compress.c:126` | [x] |
| 754 | `ZBUFF_compressContinue` | any `ZSTD_compressStream` failure — pledged-size mismatch, `dst == NULL` with `size != 0`, stable-buffer violation, allocation failure, internal compression error | forwarded verbatim: `ZSTD_error_srcSize_wrong` (72) / `ZSTD_error_dstBuffer_null` (74) / `ZSTD_error_stabilityCondition_notRespected` (50) / `ZSTD_error_memory_allocation` (64) / `ZSTD_error_stage_wrong` (60) — see section C | `deprecated/zbuff_compress.c:126` | [x] |
| 755 | `ZBUFF_compressContinue` | `*dstCapacityPtr == 0` (no room at all) | **not** an error — returns a positive "preferred next input size" hint with `*dstCapacityPtr` set to `0` and `*srcSizePtr` to whatever was buffered; the caller must loop. Note the out-params are overwritten with `outBuff.pos` / `inBuff.pos` even on the error path | `deprecated/zbuff_compress.c:127-129` | [x] |
| 756 | `ZBUFF_compressFlush` | `dstCapacityPtr == NULL` | **no NULL check** — `*dstCapacityPtr` deref = UB | `deprecated/zbuff_compress.c:142` | [x] |
| 757 | `ZBUFF_compressFlush` | `ZSTD_flushStream` fails (allocation, stable-out-buffer violation, `dst == NULL` with `size != 0`) | forwarded 64 / 50 / 74 / 60 | `deprecated/zbuff_compress.c:143` | [x] |
| 758 | `ZBUFF_compressFlush` | output buffer too small to drain the internal buffer | **not** an error — returns the number of bytes still held internally (`> 0`); only `0` means "fully flushed" | `deprecated/zbuff_compress.c:143-145` | [x] |
| 759 | `ZBUFF_compressEnd` | `dstCapacityPtr == NULL` | **no NULL check** = UB | `deprecated/zbuff_compress.c:155` | [x] |
| 760 | `ZBUFF_compressEnd` | `ZSTD_endStream` reports the pledged size was not honoured (`pledgedSrcSize != consumed`) | forwarded `ZSTD_error_srcSize_wrong` (72) | `deprecated/zbuff_compress.c:156` | [x] |
| 761 | `ZBUFF_compressEnd` | output buffer too small to write the epilogue | **not** an error — returns bytes still to flush (`> 0`); the frame is **not** finished until it returns `0` | `deprecated/zbuff_compress.c:156-158` | [x] |
| 762 | `ZBUFF_compressEnd` | called without any preceding `ZBUFF_compressContinue` | *not* rejected — emits a valid empty frame | `deprecated/zbuff_compress.c:156` | [x] |
| 763 | `ZBUFF_createDCtx` | `ZSTD_createDStream()` allocation fails | `NULL` | `deprecated/zbuff_decompress.c:22-25` | [x] |
| 764 | `ZBUFF_createDCtx_advanced` | `customMem` XOR-invalid or allocation fails | `NULL` (inherited from `ZSTD_createDStream_advanced` -> `ZSTD_createDCtx_advanced`) | `deprecated/zbuff_decompress.c:27-30` | [x] |
| 765 | `ZBUFF_freeDCtx` | `zbd == NULL` | `0` (accepted no-op) | `deprecated/zbuff_decompress.c:32-35` | [x] |
| 766 | `ZBUFF_freeDCtx` | `zbd` came from `ZSTD_initStaticDCtx` | `ZSTD_error_memory_allocation` (64) — "not compatible with static DCtx" | `deprecated/zbuff_decompress.c:34` | [x] |
| 767 | `ZBUFF_decompressInit` | none — `ZSTD_initDStream` is documented "this variant can't fail" and both `FORWARD_IF_ERROR`s are unreachable on a valid dctx | `ZSTD_startingInputLength(format)` = **`ZSTD_FRAMEHEADERSIZE_PREFIX(format)`** (5 for `zstd1`, 1 for magicless) — a *size hint*, not `0` | `deprecated/zbuff_decompress.c:45-48` -> `decompress/zstd_decompress.c:1750-1756` | [x] |
| 768 | `ZBUFF_decompressInit` | `zbd == NULL` | **no NULL check** = UB | `deprecated/zbuff_decompress.c:47` | [x] |
| 769 | `ZBUFF_decompressInitDictionary` | `ZSTD_DCtx_loadDictionary(zbd, dict, dictSize)` fails — allocation failure, or `ZSTD_dct_fullDict` requested and the buffer is not a valid zstd dictionary | forwarded `ZSTD_error_memory_allocation` (64) / `ZSTD_error_dictionary_corrupted` (30) / `ZSTD_error_dictionary_wrong` (32) | `deprecated/zbuff_decompress.c:40-43` -> `decompress/zstd_decompress.c:1741-1747` | [x] |
| 770 | `ZBUFF_decompressInitDictionary` | `ZSTD_DCtx_reset(zbd, ZSTD_reset_session_only)` fails | forwarded `ZSTD_error_stage_wrong` (60) | `deprecated/zbuff_decompress.c:42` -> `decompress/zstd_decompress.c:1744` | [x] |
| 771 | `ZBUFF_decompressInitDictionary` | success | `ZSTD_startingInputLength(format)` (5 / 1), **not** `0` | `deprecated/zbuff_decompress.c:42` -> `decompress/zstd_decompress.c:1746` | [x] |
| 772 | `ZBUFF_decompressContinue` | `dstCapacityPtr == NULL` or `srcSizePtr == NULL` | **no NULL check** — dereferenced at once = UB | `deprecated/zbuff_decompress.c:62`, `:65` | [x] |
| 773 | `ZBUFF_decompressContinue` | called **before** any `ZBUFF_decompressInit*` on a context from `ZBUFF_createDCtx` | *not* rejected: `ZSTD_createDStream` already leaves `streamStage == zdss_init`, so `ZSTD_decompressStream` starts a fresh frame. The `ZSTD_error_init_missing` (62) branch inside `ZSTD_decompressStream` is only reachable after a `ZSTD_reset_session_and_parameters`-style mis-sequencing | `deprecated/zbuff_decompress.c:66` -> section I | [x] |
| 774 | `ZBUFF_decompressContinue` | input is not a zstd frame (bad magic) | forwarded `ZSTD_error_prefix_unknown` (10) | `deprecated/zbuff_decompress.c:66` | [x] |
| 775 | `ZBUFF_decompressContinue` | frame requires a larger window than `ZSTD_d_windowLogMax` allows | forwarded `ZSTD_error_frameParameter_windowTooLarge` (16) | `deprecated/zbuff_decompress.c:66` | [x] |
| 776 | `ZBUFF_decompressContinue` | frame is corrupt / checksum mismatch / dictID mismatch / allocation failure / no forward progress | forwarded `ZSTD_error_corruption_detected` (20) / `checksum_wrong` (22) / `dictionary_wrong` (32) / `memory_allocation` (64) / `noForwardProgress_destFull` (80) / `noForwardProgress_inputEmpty` (82) — see section I | `deprecated/zbuff_decompress.c:66` | [x] |
| 777 | `ZBUFF_decompressContinue` | `*srcSizePtr == 0` on a fresh context | **not** an error — returns the "expected next input size" hint (`ZSTD_FRAMEHEADERSIZE_PREFIX`); repeated zero-progress calls eventually yield `ZSTD_error_noForwardProgress_inputEmpty` (82) | `deprecated/zbuff_decompress.c:66-69` | [x] |
| 778 | `ZBUFF_decompressContinue` | frame fully decoded | `0` ("frame complete"); any non-zero non-error return is a *suggested next input size*, so the caller must distinguish with `ZBUFF_isError` | `deprecated/zbuff_decompress.c:69` | [x] |
| 779 | `ZBUFF_recommendedCInSize` | none — no arguments, no failure mode | `ZSTD_CStreamInSize()` = `ZSTD_BLOCKSIZE_MAX` = **131072** | `deprecated/zbuff_compress.c:166` -> `compress/zstd_compress.c:5952` | [x] |
| 780 | `ZBUFF_recommendedCOutSize` | none | `ZSTD_CStreamOutSize()` = `ZSTD_compressBound(131072) + ZSTD_blockHeaderSize (3) + 4` | `deprecated/zbuff_compress.c:167` -> `compress/zstd_compress.c:5954-5957` | [x] |
| 781 | `ZBUFF_recommendedDInSize` | none | `ZSTD_DStreamInSize()` = `ZSTD_BLOCKSIZE_MAX + ZSTD_blockHeaderSize` = **131075** | `deprecated/zbuff_decompress.c:76` -> `decompress/zstd_decompress.c:1696` | [x] |
| 782 | `ZBUFF_recommendedDOutSize` | none | `ZSTD_DStreamOutSize()` = `ZSTD_BLOCKSIZE_MAX` = **131072** | `deprecated/zbuff_decompress.c:77` -> `decompress/zstd_decompress.c:1697` | [x] |
## P. Legacy decoders v0.1 .. v0.7 (`legacy/zstd_v0{1..7}.c`, `legacy/zstd_legacy.h`)

**No legacy version has a private error enum.** v0.1 re-uses the FSE
`FSE_errorCodes` list (`legacy/zstd_v01.c:44`); v0.2 .. v0.7 each define their own
local copies of `PREFIX(name) ZSTD_error_##name` / `ERROR(name)
(size_t)-PREFIX(name)` / `ERR_isError(code) (code > ERROR(maxCode))`
(e.g. `legacy/zstd_v02.c:506-524`, `legacy/zstd_v03.c:507-525`). Those local enums
enumerate the *same* `ZSTD_error_*` names as `include/zstd_errors.h`, so **every
`ZSTDv0X_isError` / `ZBUFFv0X_isError` is numerically interchangeable with
`ZSTD_isError`**, and every legacy return code below is one of the codes in the
enum table at the top of this document.

Only these codes are producible from the legacy *public* surface:
`ZSTD_error_GENERIC` (1), `prefix_unknown` (10), `version_unsupported` (12),
`frameParameter_unsupported` (14), `corruption_detected` (20),
`checksum_wrong` (22) (v0.7 only), `dictionary_corrupted` (30),
`dictionary_wrong` (32) (v0.7 only), `tableLog_tooLarge` (44),
`maxSymbolValue_tooSmall` (48), `init_missing` (62) (ZBUFF only),
`memory_allocation` (64), `dstSize_tooSmall` (70), `srcSize_wrong` (72).

### Magic numbers

| version | constant | value | how it is read |
|---|---|---|---|
| v0.1 | `ZSTDv01_magicNumber` / `_magicNumberLE` | `0xFD2FB51E` / `0x1EB52FFD` | `ZSTD_readBE32` **big-endian** inside v01 (`zstd_v01.c:1267`, `:1922`); `MEM_readLE32` against the `_LE` form in the dispatcher (`zstd_v01.h:86-87`) |
| v0.2 | `ZSTDv02_magicNumber` | `0xFD2FB522` | `MEM_readLE32` (`zstd_v02.c:878`, `zstd_v02.h:86`) |
| v0.3 | `ZSTDv03_magicNumber` | `0xFD2FB523` | `MEM_readLE32` (`zstd_v03.c:878`, `zstd_v03.h:86`) |
| v0.4 | `ZSTDv04_magicNumber` | `0xFD2FB524` | `MEM_readLE32` (`zstd_v04.h:135`) |
| v0.5 | `ZSTDv05_MAGICNUMBER` | `0xFD2FB525` | `MEM_readLE32` (`zstd_v05.h:153`) |
| v0.6 | `ZSTDv06_MAGICNUMBER` | `0xFD2FB526` | `MEM_readLE32` (`zstd_v06.h:164`) |
| v0.7 | `ZSTDv07_MAGICNUMBER` | `0xFD2FB527` | `MEM_readLE32` (`zstd_v07.h:180`); also recognises `ZSTDv07_MAGIC_SKIPPABLE_START` masked with `0xFFFFFFF0` |

### Build reachability

`c_src/CMakeLists.txt:11` globs **`src/legacy/*.c` unconditionally** and
`:22` sets `ZSTD_LEGACY_SUPPORT=5`. None of the `zstd_v0X.c` files contain a
`ZSTD_LEGACY_SUPPORT` guard, so **all seven versions are compiled and every
`ZSTDv01_*` .. `ZSTDv07_*` / `ZBUFFv04_*` .. `ZBUFFv07_*` symbol is directly
callable across the FFI boundary.** But `legacy/zstd_legacy.h:30-50` only
`#include`s `zstd_v05.h`, `zstd_v06.h` and `zstd_v07.h` (the `<= 1`..`<= 4`
blocks are false at 5), and every dispatch `switch` compiles out `case 1`..`case 4`
— so **through the main `ZSTD_*` API only v0.5, v0.6 and v0.7 frames are
decodable**; a v0.1..v0.4 frame falls through to `default` and yields
`ZSTD_error_prefix_unknown` (10) even though the decoder for it is linked in.

### P0. Dispatch layer (`legacy/zstd_legacy.h`)

| # | function | trigger | expected C result | source | [ ] |
|---|----------|---------|-------------------|--------|-----|
| 783 | `ZSTD_isLegacy` | `srcSize < 4` | `0` ("not legacy") | `legacy/zstd_legacy.h:59` | [i] |
| 784 | `ZSTD_isLegacy` | LE magic matches none of the compiled-in cases — **including a genuine v0.1/v0.2/v0.3/v0.4 magic**, whose `case` labels are `#if`-ed out at `ZSTD_LEGACY_SUPPORT=5` | `0` (the `default:` arm) | `legacy/zstd_legacy.h:63-84` | [i] |
| 785 | `ZSTD_isLegacy` | magic is `0xFD2FB525` / `0xFD2FB526` / `0xFD2FB527` | `5` / `6` / `7` — never an error code (return type `unsigned`) | `legacy/zstd_legacy.h:76`, `:79`, `:82` | [i] |
| 786 | `ZSTD_getDecompressedSize_legacy` | `ZSTD_isLegacy(...) < 5` (not legacy, or v0.1..v0.4 which carry no content size) | `0` | `legacy/zstd_legacy.h:92` | [i] |
| 787 | `ZSTD_getDecompressedSize_legacy` | v0.5 and `ZSTDv05_getFrameParams != 0` (too-small input, bad magic, reserved bits set) | `0` — the real error code is **discarded** | `legacy/zstd_legacy.h:96-97` | [i] |
| 788 | `ZSTD_getDecompressedSize_legacy` | v0.5, `ZSTDv05_getFrameParams` **succeeds** | still `0`: it returns `fParams.srcSize`, but `ZSTDv05_getFrameParams` `memset`s `*params` to zero and only ever writes `windowLog` — v0.5 frames have no content-size field | `legacy/zstd_legacy.h:98`, `legacy/zstd_v05.c:2751-2760`, `legacy/zstd_v05.h:86-90` | [i] |
| 789 | `ZSTD_getDecompressedSize_legacy` | v0.6 and `ZSTDv06_getFrameParams != 0` | `0` (error discarded) | `legacy/zstd_legacy.h:104-105` | [i] |
| 790 | `ZSTD_getDecompressedSize_legacy` | v0.7 and `ZSTDv07_getFrameParams != 0` | `0` (error discarded) | `legacy/zstd_legacy.h:112-113` | [i] |
| 791 | `ZSTD_getDecompressedSize_legacy` | fall-through past all three `if`s ("should not be possible") | `0` | `legacy/zstd_legacy.h:117` | [i] |
| 792 | `ZSTD_decompressLegacy` | `dst == NULL` | **not** rejected — `assert(dstCapacity == 0)` (compiled out at `DEBUGLEVEL 0`) then `dst` is repointed at a 1-byte stack `char x`; a `NULL` dst with a non-zero `dstCapacity` therefore becomes a 1-byte buffer and the decoder reports `dstSize_tooSmall` (70) instead of a null-pointer error | `legacy/zstd_legacy.h:129-132` | [i] |
| 793 | `ZSTD_decompressLegacy` | `src == NULL` | same substitution with `&x`; `assert(compressedSize == 0)` is not enforced in release | `legacy/zstd_legacy.h:133-136` | [i] |
| 794 | `ZSTD_decompressLegacy` | `dict == NULL` | same substitution with `&x`, so v0.5/v0.6/v0.7 see a 1-byte non-NULL dict; with `dictSize == 0` this is a harmless raw-content dict | `legacy/zstd_legacy.h:137-140` | [i] |
| 795 | `ZSTD_decompressLegacy` | `version` is `0` (unknown magic) **or** `1`..`4` (cases compiled out at `ZSTD_LEGACY_SUPPORT=5`) | `ZSTD_error_prefix_unknown` (10) | `legacy/zstd_legacy.h:190-191` | [i] |
| 796 | `ZSTD_decompressLegacy` | `ZSTDv05_createDCtx()` / `ZSTDv06_createDCtx()` / `ZSTDv07_createDCtx()` returns `NULL` | `ZSTD_error_memory_allocation` (64) | `legacy/zstd_legacy.h:164`, `:174`, `:184` | [i] |
| 797 | `ZSTD_findFrameSizeInfoLegacy` | `version` is `0` or `1`..`4` | `frameSizeInfo.compressedSize = ZSTD_error_prefix_unknown` (10) and `decompressedBound = ZSTD_CONTENTSIZE_ERROR` | `legacy/zstd_legacy.h:250-253` | [i] |
| 798 | `ZSTD_findFrameSizeInfoLegacy` | per-version scan succeeded but `compressedSize > srcSize` (frame truncated) | `compressedSize = ZSTD_error_srcSize_wrong` (72), `decompressedBound = ZSTD_CONTENTSIZE_ERROR` | `legacy/zstd_legacy.h:255-258` | [i] |
| 799 | `ZSTD_findFrameSizeInfoLegacy` | success: `decompressedBound` must be a multiple of `ZSTD_BLOCKSIZE_MAX` for `nbBlocks` to be derivable | `assert` only (no runtime check); `nbBlocks = decompressedBound / ZSTD_BLOCKSIZE_MAX`. `nbBlocks` is left **uninitialised** when `decompressedBound == ZSTD_CONTENTSIZE_ERROR` | `legacy/zstd_legacy.h:262-265` | [i] |
| 800 | `ZSTD_findFrameCompressedSizeLegacy` | any of rows 797-798 | that error code, forwarded as the `size_t` return | `legacy/zstd_legacy.h:269-273` | [i] |
| 801 | `ZSTD_freeLegacyStreamContext` | `version` is `1`, `2`, `3` or anything unrecognised (`default:` falls into `case 1`) | `ZSTD_error_version_unsupported` (12) — v0.1/v0.2/v0.3 have no ZBUFF streaming layer at all | `legacy/zstd_legacy.h:279-284` | [i] |
| 802 | `ZSTD_initLegacyStream` | `dict == NULL` | not rejected — repointed at a 1-byte stack `char x` (`assert(dictSize == 0)` not enforced in release). **Note the buffer is a local, so a non-zero `dictSize` leaves a dangling pointer** | `legacy/zstd_legacy.h:304-309` | [i] |
| 803 | `ZSTD_initLegacyStream` | `newVersion` is `1`, `2`, `3` or unrecognised | `0` ("success") **without creating any context** — `*legacyContext` is left untouched, and the subsequent `ZSTD_decompressLegacyStream` then returns `version_unsupported` (12) | `legacy/zstd_legacy.h:314-319` | [i] |
| 804 | `ZSTD_initLegacyStream` | `newVersion` 4/5/6/7 and `ZBUFFv0X_createDCtx()` returns `NULL` | `ZSTD_error_memory_allocation` (64) | `legacy/zstd_legacy.h:324`, `:335`, `:345`, `:355` | [i] |
| 805 | `ZSTD_initLegacyStream` | `prevVersion != newVersion`: the old context is freed via `ZSTD_freeLegacyStreamContext(*legacyContext, prevVersion)` | that call's `version_unsupported` (12) return is **discarded**; a stale `*legacyContext` from an unsupported prevVersion is leaked rather than reported | `legacy/zstd_legacy.h:311` | [i] |
| 806 | `ZSTD_initLegacyStream` | `newVersion` 4/5/6/7 and `ZBUFFv0X_decompressInit*` / `decompressWithDictionary` fails (e.g. `dictionary_corrupted` for v0.5-v0.7) | **swallowed** — the return value is not checked and the function still returns `0` | `legacy/zstd_legacy.h:325-328`, `:336-338`, `:346-348`, `:356-358` | [i] |
| 807 | `ZSTD_decompressLegacyStream` | `output->dst == NULL` or `input->src == NULL` | not rejected — repointed at a `static char x` (so, unlike row 802, the pointer stays valid); asserts on the zero sizes are not enforced in release | `legacy/zstd_legacy.h:369-378` | [i] |
| 808 | `ZSTD_decompressLegacyStream` | `version` is `1`, `2`, `3` or unrecognised | `ZSTD_error_version_unsupported` (12) | `legacy/zstd_legacy.h:382-387` | [i] |
| 809 | `ZSTD_decompressLegacyStream` | `legacyContext` does not match `version` (e.g. a v0.5 context with `version == 7`) | **no check** — blind cast to `ZBUFFv0X_DCtx*` = UB | `legacy/zstd_legacy.h:391`, `:405`, `:419`, `:433` | [i] |
| 810 | `ZSTD_decompressLegacyStream` | any `ZBUFFv0X_decompressContinue` error | forwarded verbatim as `hintSize`, **after** `output->pos` / `input->pos` have already been advanced by the (garbage) `decodedSize`/`readSize` out-params | `legacy/zstd_legacy.h:396-399`, `:410-413`, `:424-427`, `:438-441` | [i] |

### P1. v0.1 (`legacy/zstd_v01.c`)

Constants: `ZSTD_magicNumber == 0xFD2FB51E` (`:1267`, read **big-endian**),
`ZSTD_blockHeaderSize == 3` (`:1306`), `ZSTD_frameHeaderSize == 4` (`:1307`),
`BLOCKSIZE == 128 KB` (`:1284`). v0.1 has **no** ZBUFF streaming layer and **no**
dictionary support.

| # | function | trigger | expected C result | source | [ ] |
|---|----------|---------|-------------------|--------|-----|
| 811 | `ZSTDv01_isError` | not a rejection — `ERR_isError(code)` over the shared numeric space | `1` / `0` | `legacy/zstd_v01.c:1410` | [x] |
| 812 | `ZSTDv01_decompressDCtx` | `srcSize < ZSTD_frameHeaderSize + ZSTD_blockHeaderSize` (= **7**) | `ZSTD_error_srcSize_wrong` (72) | `legacy/zstd_v01.c:1921` | [x] |
| 813 | `ZSTDv01_decompressDCtx` | `ZSTD_readBE32(src) != 0xFD2FB51E` (wrong magic) | `ZSTD_error_prefix_unknown` (10) | `legacy/zstd_v01.c:1922-1923` | [x] |
| 814 | `ZSTDv01_decompressDCtx` | `ZSTDv01_getcBlockSize` sees fewer than 3 bytes of block header | forwarded `ZSTD_error_srcSize_wrong` (72) | `legacy/zstd_v01.c:1929-1930` -> `:1431` | [x] |
| 815 | `ZSTDv01_decompressDCtx` | declared `blockSize > remainingSize` (block runs past end of input) | `ZSTD_error_srcSize_wrong` (72) | `legacy/zstd_v01.c:1934` | [x] |
| 816 | `ZSTDv01_decompressDCtx` | `blockProperties.blockType == bt_rle` | `ZSTD_error_GENERIC` (1) ("not yet supported") — v0.1 never decodes RLE blocks | `legacy/zstd_v01.c:1945` | [x] |
| 817 | `ZSTDv01_decompressDCtx` | `bt_end` block reached while `remainingSize != 0` (trailing garbage) | `ZSTD_error_srcSize_wrong` (72) | `legacy/zstd_v01.c:1949` | [x] |
| 818 | `ZSTDv01_decompressDCtx` | `blockType` outside `bt_compressed`/`bt_raw`/`bt_rle`/`bt_end` (unreachable: 2-bit field) | `ZSTD_error_GENERIC` (1) | `legacy/zstd_v01.c:1952` | [x] |
| 819 | `ZSTDv01_decompressDCtx` | raw block larger than the remaining output space (`ZSTD_copyUncompressedBlock`) | forwarded `ZSTD_error_dstSize_tooSmall` (70) | `legacy/zstd_v01.c:1942` -> `:1447` | [x] |
| 820 | `ZSTDv01_decompressDCtx` | compressed block internals fail — literals header short (`srcSize <= 3`), literals bigger than dst, FSE table build error, `LLlog/Offlog/MLlog` above `LLFSELog`/`OffFSELog`/`MLFSELog`, sequence overruns dst or literal buffer, bitstream not fully consumed, negative `nbSeq` | forwarded `corruption_detected` (20) / `dstSize_tooSmall` (70) / `srcSize_wrong` (72) / `GENERIC` (1) | `legacy/zstd_v01.c:1466`, `:1473`, `:1475`, `:1493`, `:1546`, `:1570`, `:1590`, `:1608`, `:1626`, `:1732-1739`, `:1748`, `:1758-1759`, `:1853`, `:1869-1870`, `:1875` | [x] |
| 821 | `ZSTDv01_decompress` | any of rows 812-820 | forwarded; the function itself has **no** failure mode of its own (the `dctx_t` is a stack object, so no `memory_allocation`) | `legacy/zstd_v01.c:1965-1970` | [x] |
| 822 | `ZSTDv01_findFrameSizeInfoLegacy` | `srcSize < 7` | `*cSize = ZSTD_error_srcSize_wrong` (72), `*dBound = ZSTD_CONTENTSIZE_ERROR` | `legacy/zstd_v01.c:1989-1992` | [x] |
| 823 | `ZSTDv01_findFrameSizeInfoLegacy` | wrong big-endian magic | `*cSize = ZSTD_error_prefix_unknown` (10), `*dBound = ZSTD_CONTENTSIZE_ERROR` | `legacy/zstd_v01.c:1993-1997` | [x] |
| 824 | `ZSTDv01_findFrameSizeInfoLegacy` | `ZSTDv01_getcBlockSize` errors mid-scan | `*cSize = <that error>`, `*dBound = ZSTD_CONTENTSIZE_ERROR` | `legacy/zstd_v01.c:2003-2007` | [x] |
| 825 | `ZSTDv01_findFrameSizeInfoLegacy` | `blockSize > remainingSize` | `*cSize = ZSTD_error_srcSize_wrong` (72), `*dBound = ZSTD_CONTENTSIZE_ERROR` | `legacy/zstd_v01.c:2011-2014` | [x] |
| 826 | `ZSTDv01_findFrameSizeInfoLegacy` | `cSize` or `dBound` is `NULL` | **no NULL check** ("assumes `cSize` and `dBound` are _not_ NULL") = UB | `legacy/zstd_v01.c:1972-1978` | [x] |
| 827 | `ZSTDv01_createDCtx` | `malloc(sizeof(ZSTDv01_Dctx))` fails | `NULL` | `legacy/zstd_v01.c:2042-2043` | [x] |
| 828 | `ZSTDv01_freeDCtx` | `dctx == NULL` accepted (`free(NULL)`) | `0` — unconditional, never an error | `legacy/zstd_v01.c:2048-2052` | [x] |
| 829 | `ZSTDv01_resetDCtx` | none — cannot fail | `0`; sets `expected = 4`, `phase = 0` | `legacy/zstd_v01.c:2031-2038` | [x] |
| 830 | `ZSTDv01_nextSrcSizeToDecompress` | `dctx == NULL` | **no NULL check** — reads `((dctx_t*)dctx)->expected` = UB; the value returned is never an error code | `legacy/zstd_v01.c:2054-2057` | [x] |
| 831 | `ZSTDv01_decompressContinue` | `srcSize != ctx->expected` — the caller must supply **exactly** the byte count from `ZSTDv01_nextSrcSizeToDecompress` | `ZSTD_error_srcSize_wrong` (72) | `legacy/zstd_v01.c:2064` | [x] |
| 832 | `ZSTDv01_decompressContinue` | `phase == 0` and `ZSTD_readBE32(src) != 0xFD2FB51E` | `ZSTD_error_prefix_unknown` (10) | `legacy/zstd_v01.c:2072-2073` | [x] |
| 833 | `ZSTDv01_decompressContinue` | `phase == 1` and `ZSTDv01_getcBlockSize` errors | forwarded `ZSTD_error_srcSize_wrong` (72) | `legacy/zstd_v01.c:2083-2084` | [x] |
| 834 | `ZSTDv01_decompressContinue` | `phase == 2` and `ctx->bType == bt_rle` | `ZSTD_error_GENERIC` (1) ("not yet handled") | `legacy/zstd_v01.c:2112` | [x] |
| 835 | `ZSTDv01_decompressContinue` | `phase == 2` and `ctx->bType` unrecognised | `ZSTD_error_GENERIC` (1) | `legacy/zstd_v01.c:2118` | [x] |
| 836 | `ZSTDv01_decompressContinue` | `phase == 2` and the block body fails to decode | forwarded 20 / 70 / 72 / 1; note the stage is **still advanced** to `phase = 1` / `expected = 3` before the error is returned, so a caller that ignores the error resumes mid-frame | `legacy/zstd_v01.c:2120-2122` | [x] |
| 837 | `ZSTDv01_decompressContinue` | called on a never-reset / uninitialised `dctx` | no init/stage guard beyond the `expected` equality test; garbage `phase` falls to the `default:` `GENERIC` (1) | `legacy/zstd_v01.c:2059-2064`, `:2118` | [x] |
### P2. v0.2 (`legacy/zstd_v02.c`)

Constants: `ZSTD_magicNumber == 0xFD2FB522` (`:878`, `MEM_readLE32`),
`ZSTD_frameHeaderSize == 4` (`:2678`), `ZSTD_blockHeaderSize == 3`,
`BLOCKSIZE == 128 KB`. No ZBUFF layer, no dictionary support. The public
surface is a thin "wrapper layer" at `:3431-3465` over `static` internals.

| # | function | trigger | expected C result | source | [ ] |
|---|----------|---------|-------------------|--------|-----|
| 838 | `ZSTDv02_decompressDCtx` | **declared in `legacy/zstd_v02.h:64` but never defined in `legacy/zstd_v02.c`** | unresolved symbol at link time — this entry point does not exist | `legacy/zstd_v02.h:64` (no definition anywhere in `zstd_v02.c`) | [i] |
| 839 | `ZSTDv02_isError` | not a rejection — `ERR_isError(code)` | `1` / `0` | `legacy/zstd_v02.c:3431-3434` | [x] |
| 840 | `ZSTDv02_decompress` | `srcSize < ZSTD_frameHeaderSize + ZSTD_blockHeaderSize` (= **7**) | `ZSTD_error_srcSize_wrong` (72) | `legacy/zstd_v02.c:3436-3440` -> `:3221` | [x] |
| 841 | `ZSTDv02_decompress` | `MEM_readLE32(src) != 0xFD2FB522` | `ZSTD_error_prefix_unknown` (10) | `legacy/zstd_v02.c:3223` | [x] |
| 842 | `ZSTDv02_decompress` | `cBlockSize > remainingSize` | `ZSTD_error_srcSize_wrong` (72) | `legacy/zstd_v02.c:3235` | [x] |
| 843 | `ZSTDv02_decompress` | `blockType == bt_rle` | `ZSTD_error_GENERIC` (1) ("not yet supported") | `legacy/zstd_v02.c:3246` | [x] |
| 844 | `ZSTDv02_decompress` | `bt_end` with `remainingSize != 0` | `ZSTD_error_srcSize_wrong` (72) | `legacy/zstd_v02.c:3250` | [x] |
| 845 | `ZSTDv02_decompress` | unrecognised `blockType` | `ZSTD_error_GENERIC` (1) ("impossible") | `legacy/zstd_v02.c:3253` | [x] |
| 846 | `ZSTDv02_decompress` | block-body failure: raw block bigger than dst, literals/sequence header short, HUF/FSE header corrupt (`weightTotal == 0`, `tableLog > HUF_ABSOLUTEMAX_TABLELOG`, `rankStats[1] < 2` or odd), 4-stream jump table overflow, sequences overrun dst/literals | forwarded `dstSize_tooSmall` (70) / `corruption_detected` (20) / `srcSize_wrong` (72) / `tableLog_tooLarge` (44) / `maxSymbolValue_tooSmall` (48) / `GENERIC` (1) | `legacy/zstd_v02.c:2762`, `:2777`, `:2871`, `:2895`, `:2914`, `:1509-1551`, `:1661`, `:1697`, `:1732-1745`, `:3058-3064`, `:3179` | [x] |
| 847 | `ZSTDv02_findFrameSizeInfoLegacy` | `srcSize < 7` | `*cSize = ZSTD_error_srcSize_wrong` (72), `*dBound = ZSTD_CONTENTSIZE_ERROR` | `legacy/zstd_v02.c:3290-3293` | [x] |
| 848 | `ZSTDv02_findFrameSizeInfoLegacy` | wrong magic | `*cSize = ZSTD_error_prefix_unknown` (10), `*dBound = ZSTD_CONTENTSIZE_ERROR` | `legacy/zstd_v02.c:3294-3298` | [x] |
| 849 | `ZSTDv02_findFrameSizeInfoLegacy` | `ZSTD_getcBlockSize` error, or `cBlockSize > remainingSize` | `*cSize = <that error>` / `ZSTD_error_srcSize_wrong` (72), `*dBound = ZSTD_CONTENTSIZE_ERROR` | `legacy/zstd_v02.c:3305-3316` | [x] |
| 850 | `ZSTDv02_createDCtx` | `malloc(sizeof(ZSTD_DCtx))` fails | `NULL` | `legacy/zstd_v02.c:3442-3445` -> `:3341-3343` | [x] |
| 851 | `ZSTDv02_freeDCtx` | `dctx == NULL` accepted | `0` — unconditional | `legacy/zstd_v02.c:3447-3450` | [x] |
| 852 | `ZSTDv02_resetDCtx` | none — cannot fail | `0`; `expected = 4`, `phase = 0` | `legacy/zstd_v02.c:3452-3455` -> `:3332-3339` | [x] |
| 853 | `ZSTDv02_nextSrcSizeToDecompress` | `dctx == NULL` | **no NULL check** = UB; the returned `expected` is never an error code | `legacy/zstd_v02.c:3457-3460` -> `:3355-3358` | [x] |
| 854 | `ZSTDv02_decompressContinue` | `srcSize != ctx->expected` | `ZSTD_error_srcSize_wrong` (72) | `legacy/zstd_v02.c:3462-3465` -> `:3363` | [x] |
| 855 | `ZSTDv02_decompressContinue` | `phase == 0` and wrong magic | `ZSTD_error_prefix_unknown` (10) | `legacy/zstd_v02.c:3371-3372` | [x] |
| 856 | `ZSTDv02_decompressContinue` | `phase == 1` and `ZSTD_getcBlockSize` errors | forwarded `ZSTD_error_srcSize_wrong` (72) | `legacy/zstd_v02.c:3382-3383` | [x] |
| 857 | `ZSTDv02_decompressContinue` | `phase == 2` and `bType == bt_rle` | `ZSTD_error_GENERIC` (1) | `legacy/zstd_v02.c:3411` | [x] |
| 858 | `ZSTDv02_decompressContinue` | `phase == 2` and `bType` unrecognised | `ZSTD_error_GENERIC` (1) | `legacy/zstd_v02.c:3417` | [x] |
| 859 | `ZSTDv02_decompressContinue` | `phase == 2` and the block body fails | forwarded 20 / 70 / 72 / 1, **after** `phase`/`expected` have already been advanced | `legacy/zstd_v02.c:3419-3422` | [x] |
### P3. v0.3 (`legacy/zstd_v03.c`)

Structurally identical to v0.2; only the magic differs
(`ZSTD_magicNumber == 0xFD2FB523`, `:878`). `ZSTD_frameHeaderSize == 4`,
`ZSTD_blockHeaderSize == 3`. No ZBUFF layer, no dictionary support.

| # | function | trigger | expected C result | source | [ ] |
|---|----------|---------|-------------------|--------|-----|
| 860 | `ZSTDv03_decompressDCtx` | **declared in `legacy/zstd_v03.h:64` but never defined in `legacy/zstd_v03.c`** | unresolved symbol at link time | `legacy/zstd_v03.h:64` (no definition in `zstd_v03.c`) | [i] |
| 861 | `ZSTDv03_isError` | not a rejection — `ERR_isError(code)` | `1` / `0` | `legacy/zstd_v03.c:3071-3074` | [x] |
| 862 | `ZSTDv03_decompress` | `srcSize < 4 + 3` (= **7**) | `ZSTD_error_srcSize_wrong` (72) | `legacy/zstd_v03.c:3076-3080` -> `:2860` | [x] |
| 863 | `ZSTDv03_decompress` | `MEM_readLE32(src) != 0xFD2FB523` | `ZSTD_error_prefix_unknown` (10) | `legacy/zstd_v03.c:2862` | [x] |
| 864 | `ZSTDv03_decompress` | `cBlockSize > remainingSize` | `ZSTD_error_srcSize_wrong` (72) | `legacy/zstd_v03.c:2874` | [x] |
| 865 | `ZSTDv03_decompress` | `blockType == bt_rle` | `ZSTD_error_GENERIC` (1) ("not yet supported") | `legacy/zstd_v03.c:2885` | [x] |
| 866 | `ZSTDv03_decompress` | `bt_end` with `remainingSize != 0` | `ZSTD_error_srcSize_wrong` (72) | `legacy/zstd_v03.c:2889` | [x] |
| 867 | `ZSTDv03_decompress` | unrecognised `blockType` | `ZSTD_error_GENERIC` (1) ("impossible") | `legacy/zstd_v03.c:2892` | [x] |
| 868 | `ZSTDv03_decompress` | block-body failure (same HUF/FSE/sequence checks as v0.2) | forwarded 20 / 70 / 72 / 44 / 48 / 1 | `legacy/zstd_v03.c:1505-1547`, `:1657`, `:1693`, `:1728-1741`, `:2757`, `:2772` | [x] |
| 869 | `ZSTDv03_findFrameSizeInfoLegacy` | `srcSize < 7` | `*cSize = ZSTD_error_srcSize_wrong` (72), `*dBound = ZSTD_CONTENTSIZE_ERROR` | `legacy/zstd_v03.c:2929-2932` | [x] |
| 870 | `ZSTDv03_findFrameSizeInfoLegacy` | wrong magic | `*cSize = ZSTD_error_prefix_unknown` (10), `*dBound = ZSTD_CONTENTSIZE_ERROR` | `legacy/zstd_v03.c:2933-2937` | [x] |
| 871 | `ZSTDv03_findFrameSizeInfoLegacy` | block-header error or `cBlockSize > remainingSize` | `*cSize = <that error>` / (72), `*dBound = ZSTD_CONTENTSIZE_ERROR` | `legacy/zstd_v03.c:2944-2955` | [x] |
| 872 | `ZSTDv03_createDCtx` | `malloc(sizeof(ZSTD_DCtx))` fails | `NULL` | `legacy/zstd_v03.c:3082-3085` -> `:2981-2987` | [x] |
| 873 | `ZSTDv03_freeDCtx` | `dctx == NULL` accepted | `0` — unconditional | `legacy/zstd_v03.c:3087-3090` -> `:2989` | [x] |
| 874 | `ZSTDv03_resetDCtx` | none — cannot fail | `0`; `expected = 4`, `phase = 0` | `legacy/zstd_v03.c:3092-3095` -> `:2972-2979` | [x] |
| 875 | `ZSTDv03_nextSrcSizeToDecompress` | `dctx == NULL` | **no NULL check** = UB | `legacy/zstd_v03.c:3097-3100` -> `:2995` | [x] |
| 876 | `ZSTDv03_decompressContinue` | `srcSize != ctx->expected` | `ZSTD_error_srcSize_wrong` (72) | `legacy/zstd_v03.c:3102-3105` -> `:3003` | [x] |
| 877 | `ZSTDv03_decompressContinue` | `phase == 0` and wrong magic | `ZSTD_error_prefix_unknown` (10) | `legacy/zstd_v03.c:3012` | [x] |
| 878 | `ZSTDv03_decompressContinue` | `phase == 1` and block header errors | forwarded `ZSTD_error_srcSize_wrong` (72) | `legacy/zstd_v03.c:3022-3023` | [x] |
| 879 | `ZSTDv03_decompressContinue` | `phase == 2` and `bType == bt_rle` | `ZSTD_error_GENERIC` (1) | `legacy/zstd_v03.c:3051` | [x] |
| 880 | `ZSTDv03_decompressContinue` | `phase == 2` and `bType` unrecognised | `ZSTD_error_GENERIC` (1) | `legacy/zstd_v03.c:3057` | [x] |
| 881 | `ZSTDv03_decompressContinue` | `phase == 2` and block body fails | forwarded 20 / 70 / 72 / 1, after the stage has already advanced | `legacy/zstd_v03.c:3059-3062` | [x] |
### P4. v0.4 (`legacy/zstd_v04.c`) — first version with a ZBUFF streaming layer

Constants: `ZSTD_MAGICNUMBER == 0xFD2FB524` (`:287`), `BLOCKSIZE == 128 KB`
(`:293`), `ZSTD_blockHeaderSize == 3` (`:295`),
`ZSTD_frameHeaderSize_min == ZSTD_frameHeaderSize_max == 5` (`:296-297`),
`ZSTD_HEAPMODE == 1` (`:2384-2385`).

| # | function | trigger | expected C result | source | [ ] |
|---|----------|---------|-------------------|--------|-----|
| 882 | `ZSTDv04_isError` | not defined in `zstd_v04.c` (declared at `legacy/zstd_v04.h:54`) | unresolved symbol at link time; use `ZBUFFv04_isError` (`:3538`) or `ZSTD_isError` instead — they are the same predicate | `legacy/zstd_v04.h:54`, `legacy/zstd_v04.c:3538` | [i] |
| 883 | `ZSTD_decodeFrameHeader_Part1` (internal) | `srcSize != ZSTD_frameHeaderSize_min` (5) | `ZSTD_error_srcSize_wrong` (72) | `legacy/zstd_v04.c:2494` | [i] |
| 884 | `ZSTD_decodeFrameHeader_Part1` | `MEM_readLE32(src) != 0xFD2FB524` | `ZSTD_error_prefix_unknown` (10) | `legacy/zstd_v04.c:2496` | [i] |
| 885 | `ZSTD_getFrameParams` (internal, also used by the ZBUFF layer) | `srcSize < 5` | **not** an error — returns `ZSTD_frameHeaderSize_max` (5) as "need this many bytes" | `legacy/zstd_v04.c:2505` | [i] |
| 886 | `ZSTD_getFrameParams` | `MEM_readLE32(src) != 0xFD2FB524` | `ZSTD_error_prefix_unknown` (10) | `legacy/zstd_v04.c:2507` | [i] |
| 887 | `ZSTD_getFrameParams` | high nibble of byte 4 non-zero (reserved bits) | `ZSTD_error_frameParameter_unsupported` (14) | `legacy/zstd_v04.c:2510` | [i] |
| 888 | `ZSTD_decodeFrameHeader_Part2` | `srcSize != zc->headerSize` | `ZSTD_error_srcSize_wrong` (72) | `legacy/zstd_v04.c:2521` | [i] |
| 889 | `ZSTD_decodeFrameHeader_Part2` | 32-bit host and `windowLog > 25` (unreachable on a 64-bit build) | `ZSTD_error_frameParameter_unsupported` (14) | `legacy/zstd_v04.c:2523` | [i] |
| 890 | `ZSTDv04_decompressDCtx` / `ZSTD_decompress_usingDict` | `srcSize < ZSTD_frameHeaderSize_min + ZSTD_blockHeaderSize` (= **8**) | `ZSTD_error_srcSize_wrong` (72) | `legacy/zstd_v04.c:3036` | [x] |
| 891 | `ZSTDv04_decompressDCtx` | frame-header part 1 fails (rows 883-884) | forwarded 72 / 10 | `legacy/zstd_v04.c:3037-3038` | [x] |
| 892 | `ZSTDv04_decompressDCtx` | `srcSize < frameHeaderSize + ZSTD_blockHeaderSize` after the real header size is known | `ZSTD_error_srcSize_wrong` (72) | `legacy/zstd_v04.c:3039` | [x] |
| 893 | `ZSTDv04_decompressDCtx` | frame-header part 2 fails (rows 887-889) | forwarded 14 / 72 | `legacy/zstd_v04.c:3041-3042` | [x] |
| 894 | `ZSTDv04_decompressDCtx` | `ZSTD_getcBlockSize` sees `< 3` header bytes | forwarded `ZSTD_error_srcSize_wrong` (72) | `legacy/zstd_v04.c:3049-3050` -> `:2534` | [x] |
| 895 | `ZSTDv04_decompressDCtx` | `cBlockSize > remainingSize` | `ZSTD_error_srcSize_wrong` (72) | `legacy/zstd_v04.c:3054` | [x] |
| 896 | `ZSTDv04_decompressDCtx` | `blockType == bt_rle` | `ZSTD_error_GENERIC` (1) ("not yet supported") | `legacy/zstd_v04.c:3065` | [x] |
| 897 | `ZSTDv04_decompressDCtx` | `bt_end` with `remainingSize != 0` | `ZSTD_error_srcSize_wrong` (72) | `legacy/zstd_v04.c:3069` | [x] |
| 898 | `ZSTDv04_decompressDCtx` | unrecognised `blockType` | `ZSTD_error_GENERIC` (1) ("impossible") | `legacy/zstd_v04.c:3072` | [x] |
| 899 | `ZSTDv04_decompressDCtx` | compressed block bigger than `BLOCKSIZE` (128 KB) | `ZSTD_error_corruption_detected` (20) | `legacy/zstd_v04.c:2994` | [x] |
| 900 | `ZSTDv04_decompressDCtx` | block body corrupt — literals/sequence header short, FSE/HUF table errors, `litPtr > litEnd`, sequence overruns dst, DStream not fully consumed, match offset before `vBase` | forwarded `corruption_detected` (20) / `dstSize_tooSmall` (70) / `srcSize_wrong` (72) / `tableLog_tooLarge` (44) / `GENERIC` (1) | `legacy/zstd_v04.c:2715`, `:2723-2724`, `:2826-2844`, `:2940`, `:2956`, `:2961-2962` | [x] |
| 901 | `ZSTDv04_decompress` | `ZSTD_createDCtx()` (heap mode, `ZSTD_HEAPMODE == 1`) returns `NULL` | `ZSTD_error_memory_allocation` (64) | `legacy/zstd_v04.c:3559-3560` | [x] |
| 902 | `ZSTDv04_findFrameSizeInfoLegacy` | `srcSize < ZSTD_frameHeaderSize_min` (5) — note this is **weaker** than the `+ blockHeaderSize` test used by `decompressDCtx` | `*cSize = ZSTD_error_srcSize_wrong` (72), `*dBound = ZSTD_CONTENTSIZE_ERROR` | `legacy/zstd_v04.c:3101-3104` | [x] |
| 903 | `ZSTDv04_findFrameSizeInfoLegacy` | wrong magic | `*cSize = ZSTD_error_prefix_unknown` (10), `*dBound = ZSTD_CONTENTSIZE_ERROR` | `legacy/zstd_v04.c:3105-3108` | [x] |
| 904 | `ZSTDv04_findFrameSizeInfoLegacy` | `ZSTD_getcBlockSize` error, or `cBlockSize > remainingSize` | `*cSize = <that error>` / (72), `*dBound = ZSTD_CONTENTSIZE_ERROR` | `legacy/zstd_v04.c:3114-3126` | [x] |
| 905 | `ZSTDv04_createDCtx` | `malloc(sizeof(ZSTD_DCtx))` fails | `NULL` | `legacy/zstd_v04.c:3597` | [x] |
| 906 | `ZSTDv04_freeDCtx` | `dctx == NULL` accepted | `0` — unconditional | `legacy/zstd_v04.c:3598` | [x] |
| 907 | `ZSTDv04_resetDCtx` | none — cannot fail | `0`; sets `expected = 5`, `stage = ZSTDds_getFrameHeaderSize` | `legacy/zstd_v04.c:3570` -> `:2456-2465` | [x] |
| 908 | `ZSTDv04_nextSrcSizeToDecompress` | `dctx == NULL` | **no NULL check** = UB; the returned `expected` is never an error code | `legacy/zstd_v04.c:3572-3575` -> `:3141-3144` | [x] |
| 909 | `ZSTDv04_decompressContinue` | `srcSize != ctx->expected` | `ZSTD_error_srcSize_wrong` (72) | `legacy/zstd_v04.c:3149` | [x] |
| 910 | `ZSTDv04_decompressContinue` | `ZSTDds_getFrameHeaderSize` and `srcSize != 5` ("impossible") | `ZSTD_error_srcSize_wrong` (72) | `legacy/zstd_v04.c:3157` | [x] |
| 911 | `ZSTDv04_decompressContinue` | `ZSTDds_getFrameHeaderSize` and part-1 decode errors (bad magic) | forwarded `ZSTD_error_prefix_unknown` (10) | `legacy/zstd_v04.c:3158-3159` | [x] |
| 912 | `ZSTDv04_decompressContinue` | `ctx->headerSize > ZSTD_frameHeaderSize_min` ("impossible" — both constants are 5) | `ZSTD_error_GENERIC` (1) | `legacy/zstd_v04.c:3161` | [x] |
| 913 | `ZSTDv04_decompressContinue` | `ZSTDds_decodeFrameHeader` and part-2 decode errors | forwarded `frameParameter_unsupported` (14) / `srcSize_wrong` (72) | `legacy/zstd_v04.c:3168-3169` | [x] |
| 914 | `ZSTDv04_decompressContinue` | `ZSTDds_decodeBlockHeader` and `ZSTD_getcBlockSize` errors | forwarded `ZSTD_error_srcSize_wrong` (72) | `legacy/zstd_v04.c:3177-3178` | [x] |
| 915 | `ZSTDv04_decompressContinue` | `ZSTDds_decompressBlock` and `bType == bt_rle` | `ZSTD_error_GENERIC` (1) ("not yet handled") | `legacy/zstd_v04.c:3203` | [x] |
| 916 | `ZSTDv04_decompressContinue` | `ZSTDds_decompressBlock` and `bType` unrecognised | `ZSTD_error_GENERIC` (1) | `legacy/zstd_v04.c:3209` | [x] |
| 917 | `ZSTDv04_decompressContinue` | `ctx->stage` outside the 4 enumerated stages (uninitialised / corrupted context) | `ZSTD_error_GENERIC` (1) ("impossible") | `legacy/zstd_v04.c:3218` | [x] |
| 918 | `ZBUFFv04_createDCtx` | `malloc(sizeof(ZBUFF_DCtx))` fails | `NULL` | `legacy/zstd_v04.c:3584` -> `:3326-3327` | [x] |
| 919 | `ZBUFFv04_createDCtx` | inner `ZSTD_createDCtx()` returns `NULL` | **not checked** — `zbc->zc` is left `NULL` and a non-NULL `ZBUFF_DCtx` is returned; the failure only surfaces later as a NULL deref in `ZBUFF_decompressInit`/`decompressContinue` = UB (v0.6 and v0.7 fixed this) | `legacy/zstd_v04.c:3329` | [x] |
| 920 | `ZBUFFv04_freeDCtx` | `zbc == NULL` | `0` ("support free on null") | `legacy/zstd_v04.c:3336` | [x] |
| 921 | `ZBUFFv04_decompressInit` | none — cannot fail; sets `stage = ZBUFFds_readHeader` and returns `ZSTD_resetDCtx(zbc->zc)` which is always `0` | `0` | `legacy/zstd_v04.c:3347-3352` | [x] |
| 922 | `ZBUFFv04_decompressWithDictionary` | any `src`/`srcSize`, including `NULL`/`0` | `0` — it only *records* the pointer; **no validation of any kind**, and the dictionary must outlive the whole decompression | `legacy/zstd_v04.c:3355-3360` | [x] |
| 923 | `ZBUFFv04_decompressContinue` | called while `stage == ZBUFFds_init`, i.e. **before** `ZBUFFv04_decompressInit` | `ZSTD_error_init_missing` (62) | `legacy/zstd_v04.c:3390-3391` | [x] |
| 924 | `ZBUFFv04_decompressContinue` | `ZBUFFds_readHeader` / `ZBUFFds_loadHeader` and `ZSTD_getFrameParams` errors (bad magic, reserved bits) | forwarded `prefix_unknown` (10) / `frameParameter_unsupported` (14) | `legacy/zstd_v04.c:3402-3403`, `:3416-3417` | [x] |
| 925 | `ZBUFFv04_decompressContinue` | header not yet complete | **not** an error — sets `*maxDstSizePtr = 0` and returns `headerSize - zbc->hPos`, the number of further bytes required | `legacy/zstd_v04.c:3404-3409`, `:3418-3421` | [x] |
| 926 | `ZBUFFv04_decompressContinue` | `ZBUFFds_decodeHeader` and `malloc(BLOCKSIZE)` for `inBuff` fails | `ZSTD_error_memory_allocation` (64) | `legacy/zstd_v04.c:3433` | [x] |
| 927 | `ZBUFFv04_decompressContinue` | `ZBUFFds_decodeHeader` and `malloc(1 << params.windowLog)` for `outBuff` fails (windowLog comes straight from the frame header, so a hostile frame can demand a huge allocation) | `ZSTD_error_memory_allocation` (64) | `legacy/zstd_v04.c:3439` | [x] |
| 928 | `ZBUFFv04_decompressContinue` | `ZBUFFds_load` and `toLoad > zbc->inBuffSize - zbc->inPos` ("should never happen") | `ZSTD_error_corruption_detected` (20) | `legacy/zstd_v04.c:3484` | [x] |
| 929 | `ZBUFFv04_decompressContinue` | inner `ZSTD_decompressContinue` fails on either the direct-from-`src` path or the buffered path | forwarded 20 / 70 / 72 / 14 / 10 / 1 | `legacy/zstd_v04.c:3466`, `:3492` | [x] |
| 930 | `ZBUFFv04_decompressContinue` | `zbc->stage` outside the enumerated stages | `ZSTD_error_GENERIC` (1) ("impossible") | `legacy/zstd_v04.c:3519` | [x] |
| 931 | `ZBUFFv04_decompressContinue` | output buffer too small to flush the whole decoded block | **not** an error — returns a positive "next input size" hint with `*maxDstSizePtr` set to what was written; the caller must call again | `legacy/zstd_v04.c:3512-3517`, `:3523-3531` | [x] |
| 932 | `ZBUFFv04_decompressContinue` | `maxDstSizePtr == NULL` or `srcSizePtr == NULL` | **no NULL check** — dereferenced immediately = UB | `legacy/zstd_v04.c:3375`, `:3379` | [x] |
| 933 | `ZBUFFv04_isError` / `ZBUFFv04_getErrorName` | not rejections — `ERR_isError` / `ERR_getErrorName` | `1`/`0`; a never-NULL string | `legacy/zstd_v04.c:3538`, `:3539` | [x] |
| 934 | `ZBUFFv04_recommendedDInSize` / `ZBUFFv04_recommendedDOutSize` | none — no arguments, no failure mode | `BLOCKSIZE + 3` = **131075** / `BLOCKSIZE` = **131072** | `legacy/zstd_v04.c:3541-3542` | [x] |
### P5. v0.5 (`legacy/zstd_v05.c`) — first version reachable through the main API at `ZSTD_LEGACY_SUPPORT=5`

Constants: `ZSTDv05_MAGICNUMBER == 0xFD2FB525`, `ZSTDv05_DICT_MAGIC == 0xEC30A435`
(`:390`), `BLOCKSIZE == 128 KB` (`:396`), `ZSTDv05_blockHeaderSize == 3` (`:398`),
`ZSTDv05_frameHeaderSize_min == 5` (`:399`), `ZSTDv05_frameHeaderSize_max == 5`
(`:400`).

| # | function | trigger | expected C result | source | [ ] |
|---|----------|---------|-------------------|--------|-----|
| 935 | `ZSTDv05_isError` / `ZSTDv05_getErrorName` | not rejections — `ERR_isError` / `ERR_getErrorName` | `1`/`0`; never-NULL string | `legacy/zstd_v05.c:2577`, `:2580` | [x] |
| 936 | `ZSTDv05_getFrameParams` | `srcSize < ZSTDv05_frameHeaderSize_min` (5) | **not** an error — returns `ZSTDv05_frameHeaderSize_max` (5), the number of bytes needed | `legacy/zstd_v05.c:2754` | [x] |
| 937 | `ZSTDv05_getFrameParams` | `MEM_readLE32(src) != 0xFD2FB525` | `ZSTD_error_prefix_unknown` (10) | `legacy/zstd_v05.c:2756` | [x] |
| 938 | `ZSTDv05_getFrameParams` | high nibble of byte 4 non-zero (reserved bits) | `ZSTD_error_frameParameter_unsupported` (14) | `legacy/zstd_v05.c:2759` | [x] |
| 939 | `ZSTDv05_getFrameParams` | success | `0`; **only** `params->windowLog` is written — `params->srcSize` stays `0` (see row 788) | `legacy/zstd_v05.c:2757-2760` | [x] |
| 940 | `ZSTDv05_decodeFrameHeader_Part1` (internal) | `srcSize != 5` | `ZSTD_error_srcSize_wrong` (72) | `legacy/zstd_v05.c:2742-2743` | [i] |
| 941 | `ZSTDv05_decodeFrameHeader_Part1` | wrong magic | `ZSTD_error_prefix_unknown` (10) | `legacy/zstd_v05.c:2745` | [i] |
| 942 | `ZSTDv05_decodeFrameHeader_Part2` (internal) | `srcSize != zc->headerSize` | `ZSTD_error_srcSize_wrong` (72) | `legacy/zstd_v05.c:2770-2771` | [i] |
| 943 | `ZSTDv05_decodeFrameHeader_Part2` | 32-bit host and `windowLog > 25` (unreachable on 64-bit) | `ZSTD_error_frameParameter_unsupported` (14) | `legacy/zstd_v05.c:2773` | [i] |
| 944 | `ZSTDv05_decompress_continueDCtx` (internal, behind `decompressDCtx`/`decompress_usingDict`) | `srcSize < ZSTDv05_frameHeaderSize_min + ZSTDv05_blockHeaderSize` (= **8**) | `ZSTD_error_srcSize_wrong` (72) | `legacy/zstd_v05.c:3385` | [i] |
| 945 | `ZSTDv05_decompress_continueDCtx` | part-1 header decode fails | forwarded 72 / 10 | `legacy/zstd_v05.c:3386-3387` | [i] |
| 946 | `ZSTDv05_decompress_continueDCtx` | `srcSize < frameHeaderSize + blockHeaderSize` | `ZSTD_error_srcSize_wrong` (72) | `legacy/zstd_v05.c:3388` | [i] |
| 947 | `ZSTDv05_decompress_continueDCtx` | part-2 header decode fails | forwarded 14 / 72 | `legacy/zstd_v05.c:3390-3391` | [i] |
| 948 | `ZSTDv05_decompress_continueDCtx` | `ZSTDv05_getcBlockSize` errors | forwarded `ZSTD_error_srcSize_wrong` (72) | `legacy/zstd_v05.c:3398-3399` | [i] |
| 949 | `ZSTDv05_decompress_continueDCtx` | `cBlockSize > remainingSize` | `ZSTD_error_srcSize_wrong` (72) | `legacy/zstd_v05.c:3403` | [i] |
| 950 | `ZSTDv05_decompress_continueDCtx` | `blockType == bt_rle` | `ZSTD_error_GENERIC` (1) ("not yet supported") | `legacy/zstd_v05.c:3414` | [i] |
| 951 | `ZSTDv05_decompress_continueDCtx` | `bt_end` with `remainingSize != 0` | `ZSTD_error_srcSize_wrong` (72) | `legacy/zstd_v05.c:3418` | [i] |
| 952 | `ZSTDv05_decompress_continueDCtx` | unrecognised `blockType` | `ZSTD_error_GENERIC` (1) ("impossible") | `legacy/zstd_v05.c:3421` | [i] |
| 953 | `ZSTDv05_decompressBlock` / `_internal` | `srcSize >= BLOCKSIZE` (128 KB) | `ZSTD_error_srcSize_wrong` (72) | `legacy/zstd_v05.c:3347` | [x] |
| 954 | `ZSTDv05_decompressBlock` | block body corrupt: literals `lhSize` overrun, `litSize > BLOCKSIZE`, sequence-section headers short, an FSE table declared `_repeat` while `flagStaticTable == 0`, `LLlog/Offlog/MLlog` above their maxima, sequence overruns dst or literal buffer, leftover `nbSeq`, `litPtr > litEnd` | forwarded `corruption_detected` (20) / `dstSize_tooSmall` (70) / `srcSize_wrong` (72) / `GENERIC` (1) | `legacy/zstd_v05.c:2903`, `:2930`, `:2933`, `:2940`, `:2958-2988`, `:3007`, `:3014`, `:3031`, `:3038`, `:3055`, `:3062`, `:3188-3205`, `:3296`, `:3311`, `:3317-3318` | [x] |
| 955 | `ZSTDv05_loadEntropy` (internal, from `decompressBegin_usingDict`) | `HUFv05_readDTableX4` fails on the dictionary's literal table | `ZSTD_error_dictionary_corrupted` (30) | `legacy/zstd_v05.c:3632` | [i] |
| 956 | `ZSTDv05_loadEntropy` | `FSEv05_readNCount` fails for offcodes, or `offcodeLog > OffFSEv05Log`, or `FSEv05_buildDTable` fails | `ZSTD_error_dictionary_corrupted` (30) (3 distinct branches) | `legacy/zstd_v05.c:3637`, `:3638`, `:3640` | [i] |
| 957 | `ZSTDv05_loadEntropy` | same three failures for the matchLength table | `ZSTD_error_dictionary_corrupted` (30) | `legacy/zstd_v05.c:3645`, `:3646`, `:3648` | [i] |
| 958 | `ZSTDv05_loadEntropy` | same three failures for the litLength table | `ZSTD_error_dictionary_corrupted` (30) | `legacy/zstd_v05.c:3653`, `:3654`, `:3656` | [i] |
| 959 | `ZSTDv05_decompress_insertDictionary` (internal) | `MEM_readLE32(dict) != ZSTDv05_DICT_MAGIC` (`0xEC30A435`) | **not** an error — falls back to "pure content mode" and returns `0`. Note `MEM_readLE32(dict)` is executed **without any `dictSize >= 4` check**, so a 1..3-byte dict is an out-of-bounds read | `legacy/zstd_v05.c:3665-3671` | [i] |
| 960 | `ZSTDv05_decompress_insertDictionary` | dict has the right magic but `ZSTDv05_loadEntropy` fails | `ZSTD_error_dictionary_corrupted` (30) | `legacy/zstd_v05.c:3675` | [i] |
| 961 | `ZSTDv05_decompressBegin_usingDict` | `ZSTDv05_decompressBegin` fails (cannot in practice — it always returns `0`) | forwarded | `legacy/zstd_v05.c:3689-3690` | [x] |
| 962 | `ZSTDv05_decompressBegin_usingDict` | `dict != NULL && dictSize != 0` and the dictionary is rejected | `ZSTD_error_dictionary_corrupted` (30) — every underlying cause is collapsed into this single code | `legacy/zstd_v05.c:3692-3695` | [x] |
| 963 | `ZSTDv05_decompress_usingDict` / `ZSTDv05_decompressDCtx` | `ZSTDv05_decompressBegin_usingDict` fails | **swallowed** — the return value is discarded and decompression proceeds with a half-initialised entropy state | `legacy/zstd_v05.c:3445-3453`, `:3456-3459` | [x] |
| 964 | `ZSTDv05_decompress` | `ZSTDv05_createDCtx()` returns `NULL` (heap mode) | `ZSTD_error_memory_allocation` (64) | `legacy/zstd_v05.c:3465-3466` | [x] |
| 965 | `ZSTDv05_findFrameSizeInfoLegacy` | `srcSize < ZSTDv05_frameHeaderSize_min` (5) | `*cSize = ZSTD_error_srcSize_wrong` (72), `*dBound = ZSTD_CONTENTSIZE_ERROR` | `legacy/zstd_v05.c:3492-3495` | [x] |
| 966 | `ZSTDv05_findFrameSizeInfoLegacy` | wrong magic | `*cSize = ZSTD_error_prefix_unknown` (10), `*dBound = ZSTD_CONTENTSIZE_ERROR` | `legacy/zstd_v05.c:3496-3499` | [x] |
| 967 | `ZSTDv05_findFrameSizeInfoLegacy` | block-header error, or `cBlockSize > remainingSize` | `*cSize = <that error>` / (72), `*dBound = ZSTD_CONTENTSIZE_ERROR` | `legacy/zstd_v05.c:3505-3517` | [x] |
| 968 | `ZSTDv05_createDCtx` | `malloc(sizeof(ZSTDv05_DCtx))` fails | `NULL` | `legacy/zstd_v05.c:2629-2632` | [x] |
| 969 | `ZSTDv05_freeDCtx` | `dctx == NULL` accepted | `0` ("reserved as a potential error code in the future") | `legacy/zstd_v05.c:2637-2641` | [x] |
| 970 | `ZSTDv05_copyDCtx` | `dstDCtx`/`srcDCtx` `NULL` | **no NULL check**; `void` return — cannot signal failure. Copies `sizeof(ZSTDv05_DCtx) - (BLOCKSIZE+WILDCOPY_OVERLENGTH+5)` bytes only (workspace deliberately skipped) | `legacy/zstd_v05.c:2643-2647` | [x] |
| 971 | `ZSTDv05_nextSrcSizeToDecompress` | `dctx == NULL` | **no NULL check** = UB; returns `dctx->expected`, never an error code | `legacy/zstd_v05.c:3532-3535` | [x] |
| 972 | `ZSTDv05_decompressContinue` | `srcSize != dctx->expected` | `ZSTD_error_srcSize_wrong` (72) | `legacy/zstd_v05.c:3540` | [x] |
| 973 | `ZSTDv05_decompressContinue` | `ZSTDv05ds_getFrameHeaderSize` and `srcSize != 5` ("impossible") | `ZSTD_error_srcSize_wrong` (72) | `legacy/zstd_v05.c:3548` | [x] |
| 974 | `ZSTDv05_decompressContinue` | `ZSTDv05ds_getFrameHeaderSize` and part-1 decode errors (bad magic) | forwarded `ZSTD_error_prefix_unknown` (10) | `legacy/zstd_v05.c:3549-3550` | [x] |
| 975 | `ZSTDv05_decompressContinue` | `dctx->headerSize > ZSTDv05_frameHeaderSize_min` ("should never happen") | `ZSTD_error_GENERIC` (1) | `legacy/zstd_v05.c:3552` | [x] |
| 976 | `ZSTDv05_decompressContinue` | `ZSTDv05ds_decodeFrameHeader` and part-2 decode errors | forwarded 14 / 72 | `legacy/zstd_v05.c:3557-3562` | [x] |
| 977 | `ZSTDv05_decompressContinue` | `ZSTDv05ds_decodeBlockHeader` and `ZSTDv05_getcBlockSize` errors | forwarded `ZSTD_error_srcSize_wrong` (72) | `legacy/zstd_v05.c:3567-3568` | [x] |
| 978 | `ZSTDv05_decompressContinue` | `ZSTDv05ds_decompressBlock` and `bType == bt_rle` | `ZSTD_error_GENERIC` (1) ("not yet handled") | `legacy/zstd_v05.c:3593` | [x] |
| 979 | `ZSTDv05_decompressContinue` | `ZSTDv05ds_decompressBlock` and `bType` unrecognised | `ZSTD_error_GENERIC` (1) ("impossible") | `legacy/zstd_v05.c:3599` | [x] |
| 980 | `ZSTDv05_decompressContinue` | `dctx->stage` outside the enumerated stages | `ZSTD_error_GENERIC` (1) ("impossible") | `legacy/zstd_v05.c:3608` | [x] |
| 981 | `ZBUFFv05_createDCtx` | `malloc(sizeof(ZBUFFv05_DCtx))` fails | `NULL` | `legacy/zstd_v05.c:3806-3807` | [x] |
| 982 | `ZBUFFv05_createDCtx` | inner `ZSTDv05_createDCtx()` returns `NULL` | **not checked** — a non-NULL `ZBUFFv05_DCtx` with `zd == NULL` is returned; later NULL deref = UB (same defect as v0.4, row 919) | `legacy/zstd_v05.c:3809` | [x] |
| 983 | `ZBUFFv05_freeDCtx` | `zbc == NULL` | `0` ("support free on null") | `legacy/zstd_v05.c:3816` | [x] |
| 984 | `ZBUFFv05_decompressInitDictionary` | `ZSTDv05_decompressBegin_usingDict` rejects the dictionary | forwarded `ZSTD_error_dictionary_corrupted` (30); **but `stage` has already been set to `ZBUFFv05ds_readHeader`**, so a caller that ignores the error proceeds with a broken entropy state | `legacy/zstd_v05.c:3827-3832` | [x] |
| 985 | `ZBUFFv05_decompressInit` | delegates to `ZBUFFv05_decompressInitDictionary(zbc, NULL, 0)` — the `dict && dictSize` guard means no dictionary work happens | `0` | `legacy/zstd_v05.c:3834-3837` | [x] |
| 986 | `ZBUFFv05_decompressContinue` | called while `stage == ZBUFFv05ds_init` (before any `decompressInit*`) | `ZSTD_error_init_missing` (62) | `legacy/zstd_v05.c:3855-3856` | [x] |
| 987 | `ZBUFFv05_decompressContinue` | `ZBUFFv05ds_readHeader` / `ds_loadHeader` and `ZSTDv05_getFrameParams` errors | forwarded `prefix_unknown` (10) / `frameParameter_unsupported` (14) | `legacy/zstd_v05.c:3862`, `:3884` | [x] |
| 988 | `ZBUFFv05_decompressContinue` | header not yet complete | **not** an error — `*maxDstSizePtr = 0` and returns `headerSize - zbc->hPos` (bytes still needed) | `legacy/zstd_v05.c:3865-3869`, `:3887-3889` | [x] |
| 989 | `ZBUFFv05_decompressContinue` | `ZBUFFv05ds_decodeHeader` and `malloc` for `inBuff` fails | `ZSTD_error_memory_allocation` (64) | `legacy/zstd_v05.c:3902` | [x] |
| 990 | `ZBUFFv05_decompressContinue` | `ZBUFFv05ds_decodeHeader` and `malloc(1 << params.windowLog)` for `outBuff` fails (attacker-controlled size) | `ZSTD_error_memory_allocation` (64) | `legacy/zstd_v05.c:3908` | [x] |
| 991 | `ZBUFFv05_decompressContinue` | `ZBUFFv05ds_load` and `toLoad > zbc->inBuffSize - zbc->inPos` ("should never happen") | `ZSTD_error_corruption_detected` (20) | `legacy/zstd_v05.c:3949` | [x] |
| 992 | `ZBUFFv05_decompressContinue` | inner `ZSTDv05_decompressContinue` fails on the direct or buffered path | forwarded 20 / 70 / 72 / 14 / 10 / 1 | `legacy/zstd_v05.c:3933`, `:3960` | [x] |
| 993 | `ZBUFFv05_decompressContinue` | `zbc->stage` outside the enumerated stages | `ZSTD_error_GENERIC` (1) ("impossible") | `legacy/zstd_v05.c:3983` | [x] |
| 994 | `ZBUFFv05_decompressContinue` | `maxDstSizePtr == NULL` or `srcSizePtr == NULL` | **no NULL check** — dereferenced immediately = UB | `legacy/zstd_v05.c:3844-3849` | [x] |
| 995 | `ZBUFFv05_isError` / `ZBUFFv05_getErrorName` | not rejections — `ERR_isError` / `ERR_getErrorName` | `1`/`0`; never-NULL string | `legacy/zstd_v05.c:4001`, `:4002` | [x] |
| 996 | `ZBUFFv05_recommendedDInSize` / `ZBUFFv05_recommendedDOutSize` | none | `BLOCKSIZE + 3` = **131075** / `BLOCKSIZE` = **131072** | `legacy/zstd_v05.c:4004-4005` | [x] |
### P6. v0.6 (`legacy/zstd_v06.c`)

Constants: `ZSTDv06_MAGICNUMBER == 0xFD2FB526`,
`ZSTDv06_DICT_MAGIC == 0xEC30A436` (`:399`),
`ZSTDv06_BLOCKSIZE_MAX == 128*1024` (`:344`),
`ZSTDv06_blockHeaderSize == 3` (`:420`),
`ZSTDv06_frameHeaderSize_min == 5` (`:283`),
`ZSTDv06_FRAMEHEADERSIZE_MAX == 13` (`:282`),
`ZSTDv06_fcs_fieldSize[4] == {0,1,2,8}` (`:417`) — v0.6 is the first legacy
version with a variable-length header carrying a frame content size.

| # | function | trigger | expected C result | source | [ ] |
|---|----------|---------|-------------------|--------|-----|
| 997 | `ZSTDv06_isError` / `ZSTDv06_getErrorName` | not rejections — `ERR_isError` / `ERR_getErrorName` | `1`/`0`; never-NULL string | `legacy/zstd_v06.c:2660`, `:2664` | [x] |
| 998 | `ZSTDv06_frameHeaderSize` (internal) | `srcSize < ZSTDv06_frameHeaderSize_min` (5) | `ZSTD_error_srcSize_wrong` (72) | `legacy/zstd_v06.c:2913` | [i] |
| 999 | `ZSTDv06_getFrameParams` | `srcSize < 5` | **not** an error — returns `ZSTDv06_frameHeaderSize_min` (5) as "bytes needed" | `legacy/zstd_v06.c:2928` | [x] |
| 1000 | `ZSTDv06_getFrameParams` | `MEM_readLE32(src) != 0xFD2FB526` | `ZSTD_error_prefix_unknown` (10) | `legacy/zstd_v06.c:2929` | [x] |
| 1001 | `ZSTDv06_getFrameParams` | `srcSize < ZSTDv06_frameHeaderSize(src, srcSize)` (magic OK but the variable-length header is incomplete) | **not** an error — returns the required full header size (6..13) | `legacy/zstd_v06.c:2932-2933` | [x] |
| 1002 | `ZSTDv06_getFrameParams` | `frameDesc & 0x20` set (reserved bit) | `ZSTD_error_frameParameter_unsupported` (14) | `legacy/zstd_v06.c:2938` | [x] |
| 1003 | `ZSTDv06_getFrameParams` | `fparamsPtr == NULL` | **no NULL check** — `memset(fparamsPtr, 0, ...)` = UB | `legacy/zstd_v06.c:2936` | [x] |
| 1004 | `ZSTDv06_decodeFrameHeader` (internal) | 32-bit host and `windowLog > 25` (unreachable on 64-bit) | `ZSTD_error_frameParameter_unsupported` (14) | `legacy/zstd_v06.c:2957` | [i] |
| 1005 | `ZSTDv06_getcBlockSize` (internal) | `srcSize < ZSTDv06_blockHeaderSize` (3) | `ZSTD_error_srcSize_wrong` (72) | `legacy/zstd_v06.c:2975` | [i] |
| 1006 | `ZSTDv06_copyRawBlock` (internal) | `dst == NULL` | `ZSTD_error_dstSize_tooSmall` (70) — the only legacy version that null-checks the destination here | `legacy/zstd_v06.c:2989` | [i] |
| 1007 | `ZSTDv06_copyRawBlock` | `srcSize > dstCapacity` | `ZSTD_error_dstSize_tooSmall` (70) | `legacy/zstd_v06.c:2990` | [i] |
| 1008 | `ZSTDv06_decompressFrame` (behind `decompressDCtx`/`decompress_usingDict`) | `srcSize < ZSTDv06_frameHeaderSize_min + ZSTDv06_blockHeaderSize` (= **8**) | `ZSTD_error_srcSize_wrong` (72) | `legacy/zstd_v06.c:3517` | [i] |
| 1009 | `ZSTDv06_decompressFrame` | `srcSize < frameHeaderSize + ZSTDv06_blockHeaderSize` once the real header size is known | `ZSTD_error_srcSize_wrong` (72) | `legacy/zstd_v06.c:3522` | [i] |
| 1010 | `ZSTDv06_decompressFrame` | `ZSTDv06_decodeFrameHeader(...)` returns non-zero — **note the specific code is discarded** | `ZSTD_error_corruption_detected` (20), even when the real cause was `prefix_unknown` (10) or `frameParameter_unsupported` (14) | `legacy/zstd_v06.c:3523` | [i] |
| 1011 | `ZSTDv06_decompressFrame` | `ZSTDv06_getcBlockSize` errors | forwarded `ZSTD_error_srcSize_wrong` (72) | `legacy/zstd_v06.c:3530-3531` | [i] |
| 1012 | `ZSTDv06_decompressFrame` | `cBlockSize > remainingSize` | `ZSTD_error_srcSize_wrong` (72) | `legacy/zstd_v06.c:3535` | [i] |
| 1013 | `ZSTDv06_decompressFrame` | `blockType == bt_rle` | `ZSTD_error_GENERIC` (1) ("not yet supported") | `legacy/zstd_v06.c:3546` | [i] |
| 1014 | `ZSTDv06_decompressFrame` | `bt_end` with `remainingSize != 0` | `ZSTD_error_srcSize_wrong` (72) | `legacy/zstd_v06.c:3550` | [i] |
| 1015 | `ZSTDv06_decompressFrame` | unrecognised `blockType` | `ZSTD_error_GENERIC` (1) ("impossible") | `legacy/zstd_v06.c:3553` | [i] |
| 1016 | `ZSTDv06_decompressBlock` / `_internal` | `srcSize >= ZSTDv06_BLOCKSIZE_MAX` (131072) | `ZSTD_error_srcSize_wrong` (72) | `legacy/zstd_v06.c:3481` | [x] |
| 1017 | `ZSTDv06_decodeLiteralsBlock` (internal) | `srcSize < MIN_CBLOCK_SIZE` (3) | `ZSTD_error_corruption_detected` (20) | `legacy/zstd_v06.c:3004` | [i] |
| 1018 | `ZSTDv06_decodeLiteralsBlock` | `srcSize < 5` for a 3-byte `lhSize`, `litSize > ZSTDv06_BLOCKSIZE_MAX`, `litCSize + lhSize > srcSize`, HUF decode error, or a `_repeat` literals block while `litEntropy == 0` | `ZSTD_error_corruption_detected` (20) — and `ZSTD_error_dictionary_corrupted` (30) for the missing-dictionary-table case | `legacy/zstd_v06.c:3011`, `:3034`, `:3035`, `:3040`, `:3051`, `:3053`, `:3059`, `:3062`, `:3087`, `:3113`, `:3116`, `:3123` | [i] |
| 1019 | `ZSTDv06_buildSeqTable` (internal) | RLE mode with `srcSize == 0` | `ZSTD_error_srcSize_wrong` (72) | `legacy/zstd_v06.c:3139` | [i] |
| 1020 | `ZSTDv06_buildSeqTable` | RLE symbol `> max` | `ZSTD_error_corruption_detected` (20) | `legacy/zstd_v06.c:3140` | [i] |
| 1021 | `ZSTDv06_buildSeqTable` | `_repeat` mode while `flagRepeatTable == 0` (no previous/dictionary table) | `ZSTD_error_corruption_detected` (20) | `legacy/zstd_v06.c:3147` | [i] |
| 1022 | `ZSTDv06_buildSeqTable` | `FSEv06_readNCount` fails, or `tableLog > maxLog` | `ZSTD_error_corruption_detected` (20) (2 branches) | `legacy/zstd_v06.c:3154`, `:3155` | [i] |
| 1023 | `ZSTDv06_decodeSeqHeaders` (internal) | `srcSize < MIN_SEQUENCES_SIZE` (1), or the `nbSeq` varint / 3 mode bytes run past `iend` | `ZSTD_error_srcSize_wrong` (72) (4 distinct branches) | `legacy/zstd_v06.c:3171`, `:3178`, `:3181`, `:3189` | [i] |
| 1024 | `ZSTDv06_decompressSequences` (internal) | `FSEv06_initDState`/bitstream init fails, leftover `nbSeq != 0` at end of stream, `litPtr > litEnd`, sequence or last-literals run past `oend`, offset before `vBase` | `ZSTD_error_corruption_detected` (20) / `ZSTD_error_dstSize_tooSmall` (70) | `legacy/zstd_v06.c:3320-3336`, `:3423`, `:3447`, `:3452`, `:3453` | [i] |
| 1025 | `ZSTDv06_decompress` | `ZSTDv06_createDCtx()` returns `NULL` (heap mode) | `ZSTD_error_memory_allocation` (64) | `legacy/zstd_v06.c:3598-3599` | [x] |
| 1026 | `ZSTDv06_loadEntropy` (internal) | `HUFv06_readDTableX4` fails on the dictionary literal table | `ZSTD_error_dictionary_corrupted` (30) | `legacy/zstd_v06.c:3763` | [i] |
| 1027 | `ZSTDv06_loadEntropy` | offcode table: `FSEv06_readNCount` error, `offcodeLog > OffFSELog`, or `FSEv06_buildDTable` error | `ZSTD_error_dictionary_corrupted` (30) (3 branches) | `legacy/zstd_v06.c:3770`, `:3771`, `:3773` | [i] |
| 1028 | `ZSTDv06_loadEntropy` | matchLength table: same 3 failures | `ZSTD_error_dictionary_corrupted` (30) | `legacy/zstd_v06.c:3781`, `:3782`, `:3784` | [i] |
| 1029 | `ZSTDv06_loadEntropy` | litLength table: same 3 failures | `ZSTD_error_dictionary_corrupted` (30) | `legacy/zstd_v06.c:3792`, `:3793`, `:3795` | [i] |
| 1030 | `ZSTDv06_decompress_insertDictionary` (internal) | `MEM_readLE32(dict) != ZSTDv06_DICT_MAGIC` (`0xEC30A436`) | **not** an error — "pure content mode", returns `0`. The `MEM_readLE32` happens with **no `dictSize >= 4` guard** (out-of-bounds read for a 1..3-byte dict) | `legacy/zstd_v06.c:3804-3809` | [i] |
| 1031 | `ZSTDv06_decompress_insertDictionary` | right magic but `ZSTDv06_loadEntropy` fails | `ZSTD_error_dictionary_corrupted` (30) | `legacy/zstd_v06.c:3815` | [i] |
| 1032 | `ZSTDv06_decompressBegin_usingDict` | `ZSTDv06_decompressBegin` fails (always returns `0` in practice) | forwarded | `legacy/zstd_v06.c:3828-3829` | [x] |
| 1033 | `ZSTDv06_decompressBegin_usingDict` | `dict && dictSize` and the dictionary is rejected | `ZSTD_error_dictionary_corrupted` (30) (all causes collapsed) | `legacy/zstd_v06.c:3831-3834` | [x] |
| 1034 | `ZSTDv06_decompress_usingDict` / `ZSTDv06_decompressDCtx` | `ZSTDv06_decompressBegin_usingDict` fails | **swallowed** — return value discarded, decoding proceeds with a half-loaded entropy state | `legacy/zstd_v06.c:3577-3585`, `:3588-3591` | [x] |
| 1035 | `ZSTDv06_findFrameSizeInfoLegacy` | `ZSTDv06_frameHeaderSize(src, srcSize)` errors (`srcSize < 5`) | `*cSize = ZSTD_error_srcSize_wrong` (72), `*dBound = ZSTD_CONTENTSIZE_ERROR` | `legacy/zstd_v06.c:3625-3629` | [x] |
| 1036 | `ZSTDv06_findFrameSizeInfoLegacy` | wrong magic | `*cSize = ZSTD_error_prefix_unknown` (10), `*dBound = ZSTD_CONTENTSIZE_ERROR` | `legacy/zstd_v06.c:3630-3633` | [x] |
| 1037 | `ZSTDv06_findFrameSizeInfoLegacy` | `srcSize < frameHeaderSize + ZSTDv06_blockHeaderSize` | `*cSize = ZSTD_error_srcSize_wrong` (72), `*dBound = ZSTD_CONTENTSIZE_ERROR` | `legacy/zstd_v06.c:3634-3637` | [x] |
| 1038 | `ZSTDv06_findFrameSizeInfoLegacy` | `ZSTDv06_getcBlockSize` error, or `cBlockSize > remainingSize` | `*cSize = <that error>` / (72), `*dBound = ZSTD_CONTENTSIZE_ERROR` | `legacy/zstd_v06.c:3644-3656` | [x] |
| 1039 | `ZSTDv06_createDCtx` | `malloc(sizeof(ZSTDv06_DCtx))` fails | `NULL` | `legacy/zstd_v06.c:2786-2789` | [x] |
| 1040 | `ZSTDv06_freeDCtx` | `dctx == NULL` accepted (`free(NULL)`) | `0` ("reserved as a potential error code in the future") | `legacy/zstd_v06.c:2794-2798` | [x] |
| 1041 | `ZSTDv06_copyDCtx` | `dstDCtx`/`srcDCtx` `NULL` | **no NULL check**; `void` return. Copies `sizeof(ZSTDv06_DCtx) - (BLOCKSIZE_MAX+WILDCOPY_OVERLENGTH+frameHeaderSize_max)` bytes | `legacy/zstd_v06.c:2800-2805` | [x] |
| 1042 | `ZSTDv06_nextSrcSizeToDecompress` | `dctx == NULL` | **no NULL check** = UB; returns `dctx->expected`, never an error | `legacy/zstd_v06.c:3670-3673` | [x] |
| 1043 | `ZSTDv06_decompressContinue` | `srcSize != dctx->expected` | `ZSTD_error_srcSize_wrong` (72) | `legacy/zstd_v06.c:3678` | [x] |
| 1044 | `ZSTDv06_decompressContinue` | `ZSTDds_getFrameHeaderSize` and `srcSize != 5` ("impossible") | `ZSTD_error_srcSize_wrong` (72) | `legacy/zstd_v06.c:3685` | [x] |
| 1045 | `ZSTDv06_decompressContinue` | `ZSTDds_getFrameHeaderSize` and `ZSTDv06_frameHeaderSize` errors | forwarded `ZSTD_error_srcSize_wrong` (72) | `legacy/zstd_v06.c:3686-3687` | [x] |
| 1046 | `ZSTDv06_decompressContinue` | `ZSTDds_decodeFrameHeader` and `ZSTDv06_decodeFrameHeader` errors | forwarded `prefix_unknown` (10) / `frameParameter_unsupported` (14) | `legacy/zstd_v06.c:3699-3700` | [x] |
| 1047 | `ZSTDv06_decompressContinue` | `ZSTDds_decodeBlockHeader` and `ZSTDv06_getcBlockSize` errors | forwarded `ZSTD_error_srcSize_wrong` (72) | `legacy/zstd_v06.c:3707-3708` | [x] |
| 1048 | `ZSTDv06_decompressContinue` | `ZSTDds_decompressBlock` and `bType == bt_rle` | `ZSTD_error_GENERIC` (1) ("not yet handled") | `legacy/zstd_v06.c:3730` | [x] |
| 1049 | `ZSTDv06_decompressContinue` | `ZSTDds_decompressBlock` and `bType` unrecognised | `ZSTD_error_GENERIC` (1) ("impossible") | `legacy/zstd_v06.c:3736` | [x] |
| 1050 | `ZSTDv06_decompressContinue` | `dctx->stage` outside the 4 enumerated stages | `ZSTD_error_GENERIC` (1) ("impossible") | `legacy/zstd_v06.c:3745` | [x] |
| 1051 | `ZBUFFv06_createDCtx` | `malloc(sizeof(ZBUFFv06_DCtx))` fails | `NULL` | `legacy/zstd_v06.c:3918-3919` | [x] |
| 1052 | `ZBUFFv06_createDCtx` | inner `ZSTDv06_createDCtx()` returns `NULL` | `NULL` — **v0.6 fixes the v0.4/v0.5 leak**: it calls `ZBUFFv06_freeDCtx(zbd)` first and then returns `NULL` | `legacy/zstd_v06.c:3921-3924` | [x] |
| 1053 | `ZBUFFv06_freeDCtx` | `zbd == NULL` | `0` ("support free on null") | `legacy/zstd_v06.c:3932` | [x] |
| 1054 | `ZBUFFv06_decompressInitDictionary` | `ZSTDv06_decompressBegin_usingDict` rejects the dictionary | forwarded `ZSTD_error_dictionary_corrupted` (30) — **after** `stage` has already been set to `ZBUFFds_loadHeader` | `legacy/zstd_v06.c:3943-3948` | [x] |
| 1055 | `ZBUFFv06_decompressInit` | delegates to `ZBUFFv06_decompressInitDictionary(zbd, NULL, 0)`; the `dict && dictSize` guard means no dictionary work | `0` | `legacy/zstd_v06.c:3950-3953` | [x] |
| 1056 | `ZBUFFv06_decompressContinue` | called while `stage == ZBUFFds_init` (before `decompressInit*`, or after a frame completed and reset the stage to `ZBUFFds_init`) | `ZSTD_error_init_missing` (62) | `legacy/zstd_v06.c:3984-3985` | [x] |
| 1057 | `ZBUFFv06_decompressContinue` | `ZBUFFds_loadHeader` and `ZSTDv06_getFrameParams` returns an error | forwarded `prefix_unknown` (10) / `frameParameter_unsupported` (14). Note `toLoad = hSize - lhSize` is computed **before** the `isError` test, so an error code briefly underflows into `toLoad` (harmless, it is not used on that path) | `legacy/zstd_v06.c:3990-3991` | [x] |
| 1058 | `ZBUFFv06_decompressContinue` | header not yet complete | **not** an error — `*dstCapacityPtr = 0` and returns `(hSize - lhSize) + ZSTDv06_blockHeaderSize` (bytes still wanted) | `legacy/zstd_v06.c:3992-3997` | [x] |
| 1059 | `ZBUFFv06_decompressContinue` | consuming the buffered header via `ZSTDv06_decompressContinue` fails (first part) | forwarded 72 / 10 / 14 / 1 | `legacy/zstd_v06.c:4005-4006` | [x] |
| 1060 | `ZBUFFv06_decompressContinue` | consuming the second (long-header) part fails | forwarded 72 / 10 / 14 / 1 | `legacy/zstd_v06.c:4009-4010` | [x] |
| 1061 | `ZBUFFv06_decompressContinue` | `malloc(blockSize)` for `inBuff` fails, where `blockSize = MIN(1 << fParams.windowLog, ZSTDv06_BLOCKSIZE_MAX)` | `ZSTD_error_memory_allocation` (64) | `legacy/zstd_v06.c:4020` | [x] |
| 1062 | `ZBUFFv06_decompressContinue` | `malloc((1 << windowLog) + blockSize + 2*WILDCOPY_OVERLENGTH)` for `outBuff` fails (size is attacker-controlled via the frame header) | `ZSTD_error_memory_allocation` (64) | `legacy/zstd_v06.c:4027` | [x] |
| 1063 | `ZBUFFv06_decompressContinue` | `ZBUFFds_read` direct-from-`src` decode fails | forwarded 20 / 70 / 72 / 1 | `legacy/zstd_v06.c:4042` | [x] |
| 1064 | `ZBUFFv06_decompressContinue` | `ZBUFFds_load` and `toLoad > zbd->inBuffSize - zbd->inPos` ("should never happen") | `ZSTD_error_corruption_detected` (20) | `legacy/zstd_v06.c:4057` | [x] |
| 1065 | `ZBUFFv06_decompressContinue` | `ZBUFFds_load` buffered decode fails | forwarded 20 / 70 / 72 / 1 | `legacy/zstd_v06.c:4067` | [x] |
| 1066 | `ZBUFFv06_decompressContinue` | `zbd->stage` outside the enumerated stages | `ZSTD_error_GENERIC` (1) ("impossible") | `legacy/zstd_v06.c:4091` | [x] |
| 1067 | `ZBUFFv06_decompressContinue` | output too small to flush the decoded block | **not** an error — returns a positive next-input hint; `*dstCapacityPtr` reports what was written | `legacy/zstd_v06.c:4076-4090`, `:4097-4100` | [x] |
| 1068 | `ZBUFFv06_decompressContinue` | `dstCapacityPtr == NULL` or `srcSizePtr == NULL` | **no NULL check** — dereferenced immediately = UB | `legacy/zstd_v06.c:3969-3980` | [x] |
| 1069 | `ZBUFFv06_isError` / `ZBUFFv06_getErrorName` | not rejections — `ERR_isError` / `ERR_getErrorName` | `1`/`0`; never-NULL string | `legacy/zstd_v06.c:2670`, `:2672` | [x] |
| 1070 | `ZBUFFv06_recommendedDInSize` / `ZBUFFv06_recommendedDOutSize` | none | `ZSTDv06_BLOCKSIZE_MAX + 3` = **131075** / `131072` | `legacy/zstd_v06.c:4109-4110` | [x] |
### P7. v0.7 (`legacy/zstd_v07.c`) — richest legacy surface (skippable frames, dictID, checksum, DDict)

Constants: `ZSTDv07_MAGICNUMBER == 0xFD2FB527`,
`ZSTDv07_MAGIC_SKIPPABLE_START == 0x184D2A50U` (`:41`, matched with mask
`0xFFFFFFF0`), `ZSTDv07_DICT_MAGIC == 0xEC30A437` (`:2636`),
`ZSTDv07_BLOCKSIZE_ABSOLUTEMAX == 128*1024` (`:175`),
`ZSTDv07_blockHeaderSize == 3`, `ZSTDv07_frameHeaderSize_min == 5` (`:60`),
`ZSTDv07_FRAMEHEADERSIZE_MAX == 18` (`:59`),
`ZSTDv07_skippableHeaderSize == 8` (`:62`),
`ZSTDv07_WINDOWLOG_MAX == 25` on 32-bit / **27** on 64-bit (`:43-45`).

| # | function | trigger | expected C result | source | [ ] |
|---|----------|---------|-------------------|--------|-----|
| 1071 | `ZSTDv07_isError` / `ZSTDv07_getErrorName` | not rejections — `ERR_isError` / `ERR_getErrorName` | `1`/`0`; never-NULL string | `legacy/zstd_v07.c:2559`, `:2563` | [x] |
| 1072 | `ZSTDv07_frameHeaderSize` (internal) | `srcSize < ZSTDv07_frameHeaderSize_min` (5) | `ZSTD_error_srcSize_wrong` (72) | `legacy/zstd_v07.c:3079` | [i] |
| 1073 | `ZSTDv07_getFrameParams` | `srcSize < 5` | **not** an error — returns `ZSTDv07_frameHeaderSize_min` (5) as "bytes needed" | `legacy/zstd_v07.c:3099` | [x] |
| 1074 | `ZSTDv07_getFrameParams` | magic is a **skippable** frame (`(MEM_readLE32(src) & 0xFFFFFFF0) == 0x184D2A50`) and `srcSize < ZSTDv07_skippableHeaderSize` (8) | **not** an error — returns `8` as "bytes needed" | `legacy/zstd_v07.c:3103` | [x] |
| 1075 | `ZSTDv07_getFrameParams` | skippable frame with `srcSize >= 8` | `0` (success) with `windowSize == 0` as the "this frame is skippable" marker and `frameContentSize` = the skippable payload length | `legacy/zstd_v07.c:3104-3106` | [x] |
| 1076 | `ZSTDv07_getFrameParams` | magic is neither `0xFD2FB527` nor a skippable magic | `ZSTD_error_prefix_unknown` (10) | `legacy/zstd_v07.c:3108` | [x] |
| 1077 | `ZSTDv07_getFrameParams` | magic OK but `srcSize < ZSTDv07_frameHeaderSize(src, srcSize)` (variable header incomplete) | **not** an error — returns the required header size (6..18) | `legacy/zstd_v07.c:3111-3113` | [x] |
| 1078 | `ZSTDv07_getFrameParams` | `fhdByte & 0x08` set (reserved bits must be zero) | `ZSTD_error_frameParameter_unsupported` (14) | `legacy/zstd_v07.c:3125-3126` | [x] |
| 1079 | `ZSTDv07_getFrameParams` | `windowLog = (wlByte >> 3) + ZSTDv07_WINDOWLOG_ABSOLUTEMIN` exceeds `ZSTDv07_WINDOWLOG_MAX` (27 on 64-bit) | `ZSTD_error_frameParameter_unsupported` (14) | `legacy/zstd_v07.c:3129-3131` | [x] |
| 1080 | `ZSTDv07_getFrameParams` | direct/single-segment mode where `windowSize` is derived from `frameContentSize` and `windowSize > (1U << ZSTDv07_WINDOWLOG_MAX)` | `ZSTD_error_frameParameter_unsupported` (14) — v0.7's analogue of `frameParameter_windowTooLarge` (16), which it never returns | `legacy/zstd_v07.c:3152-3154` | [x] |
| 1081 | `ZSTDv07_getFrameParams` | `fparamsPtr == NULL` | **no NULL check** — `memset(fparamsPtr, 0, ...)` = UB | `legacy/zstd_v07.c:3100` | [x] |
| 1082 | `ZSTDv07_getDecompressedSize` | `ZSTDv07_getFrameParams != 0` for any reason (too-small input, unknown magic, reserved bits, window too large) | **`0`** — the error code is discarded and is indistinguishable from "frame declares no content size" | `legacy/zstd_v07.c:3171-3176` | [x] |
| 1083 | `ZSTDv07_decodeFrameHeader` (internal) | frame carries a `dictID` and `dctx->dictID != fParams.dictID` | `ZSTD_error_dictionary_wrong` (32) | `legacy/zstd_v07.c:3183` | [i] |
| 1084 | `ZSTDv07_decodeFrameHeader` | `ZSTDv07_getFrameParams` itself errored | that code is returned **only after** the dictID check and the `XXH64_reset`, i.e. the `result` is forwarded at the end | `legacy/zstd_v07.c:3182-3186` | [i] |
| 1085 | `ZSTDv07_getcBlockSize` (internal) | `srcSize < ZSTDv07_blockHeaderSize` (3) | `ZSTD_error_srcSize_wrong` (72) | `legacy/zstd_v07.c:3205` | [i] |
| 1086 | `ZSTDv07_copyRawBlock` (internal) | `srcSize > dstCapacity` | `ZSTD_error_dstSize_tooSmall` (70). Note v0.7 **dropped** v0.6's `dst == NULL` check | `legacy/zstd_v07.c:3219` | [i] |
| 1087 | `ZSTDv07_generateNxBytes` (internal, RLE blocks — v0.7 is the first legacy version that decodes them) | `length > dstCapacity` | `ZSTD_error_dstSize_tooSmall` (70) | `legacy/zstd_v07.c:3730` | [i] |
| 1088 | `ZSTDv07_decompressFrame` (behind `decompressDCtx` / `decompress_usingDict` / `decompress_usingDDict`) | `srcSize < ZSTDv07_frameHeaderSize_min + ZSTDv07_blockHeaderSize` (= **8**) | `ZSTD_error_srcSize_wrong` (72) | `legacy/zstd_v07.c:3752` | [i] |
| 1089 | `ZSTDv07_decompressFrame` | `ZSTDv07_frameHeaderSize` errors | forwarded `ZSTD_error_srcSize_wrong` (72) | `legacy/zstd_v07.c:3756-3757` | [i] |
| 1090 | `ZSTDv07_decompressFrame` | `srcSize < frameHeaderSize + ZSTDv07_blockHeaderSize` | `ZSTD_error_srcSize_wrong` (72) | `legacy/zstd_v07.c:3757` | [i] |
| 1091 | `ZSTDv07_decompressFrame` | `ZSTDv07_decodeFrameHeader(...)` returns non-zero — the specific code is **discarded** | `ZSTD_error_corruption_detected` (20), even when the real cause was `prefix_unknown` (10), `frameParameter_unsupported` (14) or `dictionary_wrong` (32) | `legacy/zstd_v07.c:3758` | [i] |
| 1092 | `ZSTDv07_decompressFrame` | `ZSTDv07_getcBlockSize` errors | forwarded `ZSTD_error_srcSize_wrong` (72) | `legacy/zstd_v07.c:3766-3767` | [i] |
| 1093 | `ZSTDv07_decompressFrame` | `cBlockSize > remainingSize` | `ZSTD_error_srcSize_wrong` (72) | `legacy/zstd_v07.c:3771` | [i] |
| 1094 | `ZSTDv07_decompressFrame` | `bt_end` with `remainingSize != 0` (trailing garbage) | `ZSTD_error_srcSize_wrong` (72) | `legacy/zstd_v07.c:3786` | [i] |
| 1095 | `ZSTDv07_decompressFrame` | unrecognised `blockType` | `ZSTD_error_GENERIC` (1) ("impossible") | `legacy/zstd_v07.c:3790` | [i] |
| 1096 | `ZSTDv07_decompressFrame` | frame has `checksumFlag` set — **the one-shot frame path computes the running `XXH64` but never verifies the trailer** | no `checksum_wrong` (22) is ever produced by `ZSTDv07_decompress*`; only `ZSTDv07_decompressContinue` (row 1105) checks it | `legacy/zstd_v07.c:3795` | [i] |
| 1097 | `ZSTDv07_decompressBlock` / `_internal` | `srcSize >= ZSTDv07_BLOCKSIZE_ABSOLUTEMAX` (131072) | `ZSTD_error_srcSize_wrong` (72) | `legacy/zstd_v07.c:3694` | [x] |
| 1098 | `ZSTDv07_decodeLiteralsBlock` (internal) | `srcSize < MIN_CBLOCK_SIZE` (3); `srcSize < 5` for a 3-byte `lhSize`; `litSize > ZSTDv07_BLOCKSIZE_ABSOLUTEMAX`; `litCSize + lhSize > srcSize`; `litSize + lhSize > srcSize`; `srcSize < 4`; HUF decode failure; unrecognised literals block type | `ZSTD_error_corruption_detected` (20) | `legacy/zstd_v07.c:3234`, `:3241`, `:3264`, `:3265`, `:3270`, `:3282`, `:3290`, `:3293`, `:3318`, `:3344`, `:3347`, `:3354` | [i] |
| 1099 | `ZSTDv07_decodeLiteralsBlock` | a `_repeat` literals block while no previous/dictionary HUF table exists (`litEntropy == 0`) | `ZSTD_error_dictionary_corrupted` (30) | `legacy/zstd_v07.c:3284` | [i] |
| 1100 | `ZSTDv07_buildSeqTable` (internal) | RLE mode with `srcSize == 0` | `ZSTD_error_srcSize_wrong` (72) | `legacy/zstd_v07.c:3370` | [i] |
| 1101 | `ZSTDv07_buildSeqTable` | RLE symbol `> max`; `_repeat` mode with `flagRepeatTable == 0`; `FSEv07_readNCount` error; `tableLog > maxLog` | `ZSTD_error_corruption_detected` (20) (4 distinct branches) | `legacy/zstd_v07.c:3371`, `:3378`, `:3385`, `:3386` | [i] |
| 1102 | `ZSTDv07_decodeSeqHeaders` (internal) | `srcSize < MIN_SEQUENCES_SIZE` (1); the `nbSeq` varint runs past `iend`; fewer than 4 bytes left for the header byte + 3 table descriptors | `ZSTD_error_srcSize_wrong` (72) (4 branches) | `legacy/zstd_v07.c:3402`, `:3409`, `:3412`, `:3420` | [i] |
| 1103 | `ZSTDv07_execSequence` / `ZSTDv07_decompressSequences` (internal) | literals + `WILDCOPY_OVERLENGTH` past `oend`; whole sequence past `oend`; `litLength` past the literal limit; offset before `vBase`; bitstream init failure; leftover `nbSeq != 0`; last-literals run past `oend` | `ZSTD_error_dstSize_tooSmall` (70) / `ZSTD_error_corruption_detected` (20) | `legacy/zstd_v07.c:3548`, `:3549`, `:3551`, `:3561`, `:3644`, `:3658`, `:3666` | [i] |
| 1104 | `ZSTDv07_decompress` | `ZSTDv07_createDCtx()` returns `NULL` (heap mode) | `ZSTD_error_memory_allocation` (64) | `legacy/zstd_v07.c:3841-3842` | [x] |
| 1105 | `ZSTDv07_decompressContinue` | `bt_end` reached with `fParams.checksumFlag` set and the 22-bit truncated `XXH64` in the trailer does not match | `ZSTD_error_checksum_wrong` (22) — the **only** place any legacy decoder produces this code | `legacy/zstd_v07.c:3974-3979` | [x] |
| 1106 | `ZSTDv07_decompressContinue` | `srcSize != dctx->expected` | `ZSTD_error_srcSize_wrong` (72) | `legacy/zstd_v07.c:3936` | [x] |
| 1107 | `ZSTDv07_decompressContinue` | `ZSTDds_getFrameHeaderSize` and `srcSize != 5` ("impossible") | `ZSTD_error_srcSize_wrong` (72) | `legacy/zstd_v07.c:3942` | [x] |
| 1108 | `ZSTDv07_decompressContinue` | `ZSTDds_getFrameHeaderSize` and the magic is a skippable magic | **not** an error — switches to `ZSTDds_decodeSkippableHeader` with `expected = 3` and returns `0` | `legacy/zstd_v07.c:3943-3948` | [x] |
| 1109 | `ZSTDv07_decompressContinue` | `ZSTDds_getFrameHeaderSize` and `ZSTDv07_frameHeaderSize` errors | forwarded `ZSTD_error_srcSize_wrong` (72) | `legacy/zstd_v07.c:3949-3950` | [x] |
| 1110 | `ZSTDv07_decompressContinue` | `ZSTDds_decodeFrameHeader` and `ZSTDv07_decodeFrameHeader` errors | forwarded `prefix_unknown` (10) / `frameParameter_unsupported` (14) / `dictionary_wrong` (32) — unlike the one-shot path (row 1091) the real code **is** preserved here | `legacy/zstd_v07.c:3960-3962` | [x] |
| 1111 | `ZSTDv07_decompressContinue` | `ZSTDds_decodeBlockHeader` and `ZSTDv07_getcBlockSize` errors | forwarded `ZSTD_error_srcSize_wrong` (72) | `legacy/zstd_v07.c:3969-3971` | [x] |
| 1112 | `ZSTDv07_decompressContinue` | `ZSTDds_decompressBlock` and `bType == bt_rle` | `ZSTD_error_GENERIC` (1) ("not yet handled") — note the **one-shot** path *does* decode RLE blocks (row 1087), so the streaming and one-shot APIs disagree | `legacy/zstd_v07.c:4000` | [x] |
| 1113 | `ZSTDv07_decompressContinue` | `ZSTDds_decompressBlock` and `bType` unrecognised | `ZSTD_error_GENERIC` (1) ("impossible") | `legacy/zstd_v07.c:4006` | [x] |
| 1114 | `ZSTDv07_decompressContinue` | `dctx->stage` outside the 6 enumerated stages | `ZSTD_error_GENERIC` (1) ("impossible") | `legacy/zstd_v07.c:4027` | [x] |
| 1115 | `ZSTDv07_decompressContinue` | `ZSTDds_decodeSkippableHeader`: `expected` is set from `MEM_readLE32(headerBuffer+4)`, i.e. **entirely attacker-controlled and unvalidated** | not rejected — the caller is instructed to feed up to 4 GB of skippable payload | `legacy/zstd_v07.c:4015-4019` | [x] |
| 1116 | `ZSTDv07_nextSrcSizeToDecompress` | `dctx == NULL` | **no NULL check** = UB; returns `dctx->expected`, never an error | `legacy/zstd_v07.c:3920-3923` | [x] |
| 1117 | `ZSTDv07_loadEntropy` (internal) | `HUFv07_readDTableX4` fails on the dictionary literal table | `ZSTD_error_dictionary_corrupted` (30) | `legacy/zstd_v07.c:4047` | [i] |
| 1118 | `ZSTDv07_loadEntropy` | offcode table: `FSEv07_readNCount` error, `offcodeLog > OffFSELog`, or `FSEv07_buildDTable` error | `ZSTD_error_dictionary_corrupted` (30) (3 branches) | `legacy/zstd_v07.c:4054`, `:4055`, `:4057` | [i] |
| 1119 | `ZSTDv07_loadEntropy` | matchLength table: same 3 failures | `ZSTD_error_dictionary_corrupted` (30) | `legacy/zstd_v07.c:4064`, `:4065`, `:4067` | [i] |
| 1120 | `ZSTDv07_loadEntropy` | litLength table: same 3 failures | `ZSTD_error_dictionary_corrupted` (30) | `legacy/zstd_v07.c:4074`, `:4075`, `:4077` | [i] |
| 1121 | `ZSTDv07_loadEntropy` | fewer than 12 bytes left after the entropy tables (no room for the 3 repcodes) | `ZSTD_error_dictionary_corrupted` (30) | `legacy/zstd_v07.c:4081` | [i] |
| 1122 | `ZSTDv07_loadEntropy` | `rep[0]` is `0` or `>= dictSize` | `ZSTD_error_dictionary_corrupted` (30) | `legacy/zstd_v07.c:4082` | [i] |
| 1123 | `ZSTDv07_loadEntropy` | `rep[1]` is `0` or `>= dictSize` | `ZSTD_error_dictionary_corrupted` (30) | `legacy/zstd_v07.c:4083` | [i] |
| 1124 | `ZSTDv07_loadEntropy` | `rep[2]` is `0` or `>= dictSize` | `ZSTD_error_dictionary_corrupted` (30) | `legacy/zstd_v07.c:4084` | [i] |
| 1125 | `ZSTDv07_decompress_insertDictionary` (internal) | `dictSize < 8` or `MEM_readLE32(dict) != ZSTDv07_DICT_MAGIC` (`0xEC30A437`) | **not** an error — falls back to raw-content mode (`ZSTDv07_refDictContent`) and returns `0` | `legacy/zstd_v07.c:4091-4100` (the `dictSize < 8` guard is at `:4093`) | [i] |
| 1126 | `ZSTDv07_decompress_insertDictionary` | right magic but `ZSTDv07_loadEntropy` fails | `ZSTD_error_dictionary_corrupted` (30) | `legacy/zstd_v07.c:4104` | [i] |
| 1127 | `ZSTDv07_decompressBegin_usingDict` | `ZSTDv07_decompressBegin` fails (always `0` in practice) | forwarded | `legacy/zstd_v07.c:4116-4117` | [x] |
| 1128 | `ZSTDv07_decompressBegin_usingDict` | `dict && dictSize` and the dictionary is rejected | `ZSTD_error_dictionary_corrupted` (30) (all causes collapsed) | `legacy/zstd_v07.c:4119-4122` | [x] |
| 1129 | `ZSTDv07_decompress_usingDict` / `ZSTDv07_decompressDCtx` | `ZSTDv07_decompressBegin_usingDict` fails | **swallowed** — return value discarded, decoding proceeds with a half-loaded entropy state | `legacy/zstd_v07.c:3818-3826`, `:3831-3834` | [x] |
| 1130 | `ZSTDv07_findFrameSizeInfoLegacy` | `srcSize < ZSTDv07_frameHeaderSize_min + ZSTDv07_blockHeaderSize` (8) | `*cSize = ZSTD_error_srcSize_wrong` (72), `*dBound = ZSTD_CONTENTSIZE_ERROR` | `legacy/zstd_v07.c:3867-3870` | [x] |
| 1131 | `ZSTDv07_findFrameSizeInfoLegacy` | `ZSTDv07_frameHeaderSize` errors | `*cSize = <that error>`, `*dBound = ZSTD_CONTENTSIZE_ERROR` | `legacy/zstd_v07.c:3874-3877` | [x] |
| 1132 | `ZSTDv07_findFrameSizeInfoLegacy` | `MEM_readLE32(src) != ZSTDv07_MAGICNUMBER` — **skippable frames are *not* accepted here**, unlike `ZSTDv07_getFrameParams` | `*cSize = ZSTD_error_prefix_unknown` (10), `*dBound = ZSTD_CONTENTSIZE_ERROR` | `legacy/zstd_v07.c:3878-3881` | [x] |
| 1133 | `ZSTDv07_findFrameSizeInfoLegacy` | `srcSize < frameHeaderSize + ZSTDv07_blockHeaderSize` | `*cSize = ZSTD_error_srcSize_wrong` (72), `*dBound = ZSTD_CONTENTSIZE_ERROR` | `legacy/zstd_v07.c:3882-3885` | [x] |
| 1134 | `ZSTDv07_findFrameSizeInfoLegacy` | `ZSTDv07_getcBlockSize` errors mid-scan | `*cSize = <that error>`, `*dBound = ZSTD_CONTENTSIZE_ERROR` | `legacy/zstd_v07.c:3892-3896` | [x] |
| 1135 | `ZSTDv07_findFrameSizeInfoLegacy` | `cBlockSize > remainingSize` (checked **after** the `bt_end` early-break, unlike v0.1-v0.6) | `*cSize = ZSTD_error_srcSize_wrong` (72), `*dBound = ZSTD_CONTENTSIZE_ERROR` | `legacy/zstd_v07.c:3903-3906` | [x] |
| 1136 | `ZSTDv07_createDCtx_advanced` | `customMem.customAlloc` and `customMem.customFree` are **both** NULL | **not** an error — substituted with `defaultCustomMem` | `legacy/zstd_v07.c:2922-2926` | [x] |
| 1137 | `ZSTDv07_createDCtx_advanced` | exactly one of `customAlloc`/`customFree` is NULL | `NULL` | `legacy/zstd_v07.c:2928-2929` | [x] |
| 1138 | `ZSTDv07_createDCtx_advanced` / `ZSTDv07_createDCtx` | `customAlloc(opaque, sizeof(ZSTDv07_DCtx))` fails | `NULL` | `legacy/zstd_v07.c:2931-2932` | [x] |
| 1139 | `ZSTDv07_freeDCtx` | `dctx == NULL` | `0` ("support free on NULL") | `legacy/zstd_v07.c:2944-2949` | [x] |
| 1140 | `ZSTDv07_copyDCtx` | `dstDCtx`/`srcDCtx` `NULL` | **no NULL check**; `void` return. Copies `sizeof(ZSTDv07_DCtx) - (BLOCKSIZE_ABSOLUTEMAX+WILDCOPY_OVERLENGTH+frameHeaderSize_max)` bytes | `legacy/zstd_v07.c:2951-2955` | [x] |
| 1141 | `ZSTDv07_createDDict_advanced` | both `customAlloc`/`customFree` NULL -> `defaultCustomMem`; exactly one NULL -> `NULL` | `NULL` for the XOR case | `legacy/zstd_v07.c:4135-4139` | [i] |
| 1142 | `ZSTDv07_createDDict` / `_advanced` | allocation of the `ZSTDv07_DDict`, the dictionary copy, or the inner `ZSTDv07_DCtx` fails | `NULL` (all three partial allocations are freed first) | `legacy/zstd_v07.c:4141-4150` | [x] |
| 1143 | `ZSTDv07_createDDict` / `_advanced` | `ZSTDv07_decompressBegin_usingDict` rejects the dictionary | `NULL` — the underlying `ZSTD_error_dictionary_corrupted` (30) is **discarded** | `legacy/zstd_v07.c:4153-4159` | [x] |
| 1144 | `ZSTDv07_createDDict` / `_advanced` | `dict == NULL` with `dictSize != 0` | **no NULL check** — `memcpy(dictContent, dict, dictSize)` = UB | `legacy/zstd_v07.c:4152` | [x] |
| 1145 | `ZSTDv07_freeDDict` | `ddict == NULL` | **no NULL check** — `ddict->refContext->customMem` deref = UB (contrast `ZSTDv07_freeDCtx`, which does check) | `legacy/zstd_v07.c:4178-4186` | [x] |
| 1146 | `ZSTDv07_decompress_usingDDict` | `ddict == NULL` | **no NULL check** — `ddict->refContext` deref = UB | `legacy/zstd_v07.c:4188-4195` | [x] |
| 1147 | `ZSTDv07_decompress_usingDDict` | frame `dictID` does not match the DDict's | forwarded from `ZSTDv07_decodeFrameHeader` but **flattened to `ZSTD_error_corruption_detected` (20)** by `ZSTDv07_decompressFrame` (row 1091) | `legacy/zstd_v07.c:4195` -> `:3758` -> `:3183` | [x] |
| 1148 | `ZBUFFv07_createDCtx_advanced` | both `customAlloc`/`customFree` NULL -> `defaultCustomMem`; exactly one NULL -> `NULL` | `NULL` for the XOR case | `legacy/zstd_v07.c:4288-4292` | [x] |
| 1149 | `ZBUFFv07_createDCtx` / `_advanced` | `customAlloc(opaque, sizeof(ZBUFFv07_DCtx))` fails | `NULL` | `legacy/zstd_v07.c:4294-4295` | [x] |
| 1150 | `ZBUFFv07_createDCtx` / `_advanced` | inner `ZSTDv07_createDCtx_advanced` returns `NULL` | `NULL` — the partially built `ZBUFFv07_DCtx` is freed first (same fix as v0.6) | `legacy/zstd_v07.c:4299-4300` | [x] |
| 1151 | `ZBUFFv07_freeDCtx` | `zbd == NULL` | `0` ("support free on null") | `legacy/zstd_v07.c:4307` | [x] |
| 1152 | `ZBUFFv07_decompressInitDictionary` | `ZSTDv07_decompressBegin_usingDict` rejects the dictionary | forwarded `ZSTD_error_dictionary_corrupted` (30) — **after** `stage` has already been set to `ZBUFFds_loadHeader` | `legacy/zstd_v07.c:4318-4323` | [x] |
| 1153 | `ZBUFFv07_decompressInit` | delegates to `ZBUFFv07_decompressInitDictionary(zbd, NULL, 0)` | `0` | `legacy/zstd_v07.c:4325-4328` | [x] |
| 1154 | `ZBUFFv07_decompressContinue` | called while `stage == ZBUFFds_init` (before `decompressInit*`, or after a completed frame reset the stage) | `ZSTD_error_init_missing` (62) | `legacy/zstd_v07.c:4359-4360` | [x] |
| 1155 | `ZBUFFv07_decompressContinue` | `ZBUFFds_loadHeader` and `ZSTDv07_getFrameParams` returns an error | forwarded `prefix_unknown` (10) / `frameParameter_unsupported` (14) | `legacy/zstd_v07.c:4366-4368` | [x] |
| 1156 | `ZBUFFv07_decompressContinue` | header not yet complete | **not** an error — `*dstCapacityPtr = 0` and returns `(hSize - lhSize) + ZSTDv07_blockHeaderSize` (bytes still wanted) | `legacy/zstd_v07.c:4369-4375` | [x] |
| 1157 | `ZBUFFv07_decompressContinue` | consuming the buffered header via `ZSTDv07_decompressContinue` fails (first or second part) | forwarded 72 / 10 / 14 / 32 / 1 (2 distinct branches) | `legacy/zstd_v07.c:4380-4381`, `:4384-4385` | [x] |
| 1158 | `ZBUFFv07_decompressContinue` | `customAlloc(blockSize)` for `inBuff` fails, where `blockSize = MIN(fParams.windowSize, ZSTDv07_BLOCKSIZE_ABSOLUTEMAX)` | `ZSTD_error_memory_allocation` (64) | `legacy/zstd_v07.c:4397` | [x] |
| 1159 | `ZBUFFv07_decompressContinue` | `customAlloc(neededOutSize)` for `outBuff` fails, where `neededOutSize = fParams.windowSize + blockSize` (attacker-controlled up to `1 << 27`) | `ZSTD_error_memory_allocation` (64) | `legacy/zstd_v07.c:4404` | [x] |
| 1160 | `ZBUFFv07_decompressContinue` | `ZBUFFds_read` direct-from-`src` decode fails | forwarded 20 / 22 / 70 / 72 / 1 | `legacy/zstd_v07.c:4420-4421` | [x] |
| 1161 | `ZBUFFv07_decompressContinue` | `ZBUFFds_load` and `toLoad > zbd->inBuffSize - zbd->inPos` ("should never happen") | `ZSTD_error_corruption_detected` (20) | `legacy/zstd_v07.c:4436` | [x] |
| 1162 | `ZBUFFv07_decompressContinue` | `ZBUFFds_load` buffered decode fails | forwarded 20 / 22 / 70 / 72 / 1 | `legacy/zstd_v07.c:4446-4447` (check at `:4447`) | [x] |
| 1163 | `ZBUFFv07_decompressContinue` | `zbd->stage` outside the enumerated stages | `ZSTD_error_GENERIC` (1) ("impossible") | `legacy/zstd_v07.c:4472` | [x] |
| 1164 | `ZBUFFv07_decompressContinue` | output too small to flush the decoded block | **not** an error — returns a positive next-input hint; `*dstCapacityPtr` reports what was written | `legacy/zstd_v07.c:4457-4471`, `:4477-4481` | [x] |
| 1165 | `ZBUFFv07_decompressContinue` | `dstCapacityPtr == NULL` or `srcSizePtr == NULL` | **no NULL check** — dereferenced immediately = UB | `legacy/zstd_v07.c:4344-4356` | [x] |
| 1166 | `ZBUFFv07_isError` / `ZBUFFv07_getErrorName` | not rejections — `ERR_isError` / `ERR_getErrorName` | `1`/`0`; never-NULL string | `legacy/zstd_v07.c:2570`, `:2572` | [x] |
| 1167 | `ZBUFFv07_recommendedDInSize` / `ZBUFFv07_recommendedDOutSize` | none | `ZSTDv07_BLOCKSIZE_ABSOLUTEMAX + 3` = **131075** / `131072` | `legacy/zstd_v07.c:4489-4490` | [x] |
## Q. Generic FFI boundary rejections (cross-cutting)

This section covers the boundary conditions that are **not** specific to one
function: what happens when a caller crossing the FFI boundary passes a NULL
pointer, a zero length, an over-large length, or an integer that is not a valid
member of a C `enum`. The key fact for a port is that **a C `enum` parameter is
just an `int`** — the compiler performs no validation, so every one of these
values can arrive at the C boundary out of range, and the behaviour is whatever
the receiving `switch`/comparison happens to do.

### Q1. NULL pointers

| # | function class | trigger | expected C result | source | [ ] |
|---|----------------|---------|-------------------|--------|-----|
| 1168 | `ZSTD_freeCCtx` / `ZSTD_freeCStream` | `cctx == NULL` | `0` — explicitly supported ("support free on NULL") | `compress/zstd_compress.c:184` | [x] |
| 1169 | `ZSTD_sizeof_CCtx` / `ZSTD_sizeof_CStream` | `cctx == NULL` | `0` — explicitly supported ("support sizeof on NULL") | `compress/zstd_compress.c:208` | [x] |
| 1170 | `ZSTD_freeDCtx` / `ZSTD_freeDStream` | `dctx == NULL` | `0` ("support free on NULL") | `decompress/zstd_decompress.c:326` | [x] |
| 1171 | `ZSTD_sizeof_DCtx` / `ZSTD_sizeof_DStream` | `dctx == NULL` | `0` ("support sizeof NULL") | `decompress/zstd_decompress.c:223` | [x] |
| 1172 | `ZSTD_freeCDict` / `ZSTD_sizeof_CDict` / `ZSTD_getDictID_fromCDict` | `cdict == NULL` | `0` (free, sizeof and dictID all explicitly accept NULL) | `compress/zstd_compress.c:5734`, `:5544`, `:5816` | [x] |
| 1173 | `ZSTD_freeDDict` / `ZSTD_sizeof_DDict` / `ZSTD_getDictID_fromDDict` | `ddict == NULL` | `0` | `decompress/zstd_ddict.c:214`, `:232`, `:242` | [x] |
| 1174 | `ZSTD_compress_usingCDict*` | `cdict == NULL` | `ZSTD_error_dictionary_wrong` (32) ("NULL pointer!") — one of the very few explicit NULL rejections | `compress/zstd_compress.c:5829` | [x] |
| 1175 | `ZSTD_getFrameHeader_advanced` / `ZSTD_getFrameHeader` / `ZSTD_frameHeaderSize` | `src == NULL` with `srcSize > 0` | `ZSTD_error_GENERIC` (1) ("invalid parameter : src==NULL, but srcSize>0") | `decompress/zstd_decompress.c:453-456` | [x] |
| 1176 | `ZSTD_decompress*` on a raw or RLE block | `dst == NULL` with a non-zero block to emit | `ZSTD_error_dstBuffer_null` (74) — the only two sites that produce code 74 | `decompress/zstd_decompress.c:900-903`, `:914-917` | [x] |
| 1177 | `ZSTD_decompress*` on a raw or RLE block | `dst == NULL` with `srcSize == 0` / `regenSize == 0` | `0` — accepted, not an error | `decompress/zstd_decompress.c:901`, `:915` | [x] |
| 1178 | every `ZSTD_CCtx_*` / `ZSTD_DCtx_*` setter, getter and streaming call | the **context** pointer is `NULL` | **no NULL check** — the first field access dereferences NULL = UB. `ZSTD_compressStream2` even has `assert(cctx != NULL)` *after* it has already read `output`/`input` | `compress/zstd_compress.c:6457`, and every `cctx->`/`dctx->` access | [i] |
| 1179 | `ZSTD_compressStream2` / `ZSTD_decompressStream` / `ZSTD_flushStream` / `ZSTD_endStream` | `output == NULL` or `input == NULL` (the `ZSTD_outBuffer*`/`ZSTD_inBuffer*` themselves) | **no NULL check** — `output->pos` is read immediately = UB | `compress/zstd_compress.c:6454-6455` | [x] |
| 1180 | `ZSTD_CCtx_loadDictionary*` / `ZSTD_DCtx_loadDictionary*` / `ZSTD_CCtx_refPrefix*` | `dict == NULL` with `dictSize == 0` | **not** an error — clears any existing dictionary and returns `0` | `compress/zstd_compress.c:5562`, `decompress/zstd_decompress.c:1702-1712` | [x] |
| 1181 | `ZSTD_createCDict` / `ZSTD_createDDict` | `dict == NULL` with `dictSize > 0` | **no NULL check** — the `ZSTD_memcpy` of the dictionary content dereferences NULL = UB | `compress/zstd_compress.c:5562-5570`, `decompress/zstd_ddict.c:123-141` | [x] |
| 1182 | `ZSTD_getErrorName` / `ZSTD_getErrorString` | any input | **never** returns NULL; unmapped codes give `"Unspecified error code"` | `common/zstd_common.c:40`, `:48` -> `common/error_private.c:21`, `:61` | [x] |
| 1183 | `ZSTD_cParam_getBounds` / `ZSTD_dParam_getBounds` | returns a struct by value | cannot be NULL; failure is signalled in `bounds.error` | `compress/zstd_compress.c:419-421`, `decompress/zstd_decompress.c:1821-1823` | [x] |
| 1184 | `ZSTD_CCtx_getParameter` / `ZSTD_DCtx_getParameter` | `value == NULL` (the out-param) | **no NULL check** — `*value = ...` = UB | `compress/zstd_compress.c:1024-1166`, `decompress/zstd_decompress.c:1876-1901` | [x] |
### Q2. Zero and over-large lengths

| # | function class | trigger | expected C result | source | [ ] |
|---|----------------|---------|-------------------|--------|-----|
| 1185 | `ZSTD_compressBound` | `srcSize >= ZSTD_MAX_INPUT_SIZE` (`0xFF00FF00FF00FF00` on 64-bit, `0xFF00FF00` on 32-bit) — `ZSTD_COMPRESSBOUND` evaluates to `0` | `ZSTD_error_srcSize_wrong` (72). **The macro `ZSTD_COMPRESSBOUND` itself just yields `0`**, so a caller that uses the macro instead of the function sees `0`, not an error | `compress/zstd_compress.c:70-74`, `include/zstd.h:249` | [x] |
| 1186 | `ZSTD_compress*` | `srcSize == 0` | **not** an error — produces a valid empty frame | `compress/zstd_compress.c` (no zero-size guard on the compress path) | [x] |
| 1187 | `ZSTD_compress*` | `dstCapacity == 0` with `dst == NULL` | `ZSTD_error_dstSize_tooSmall` (70) from `ZSTD_writeFrameHeader` (needs at least `ZSTD_FRAMEHEADERSIZE_MAX == 18`), **not** `dstBuffer_null` | `compress/zstd_compress.c:4712-4713` | [x] |
| 1188 | `ZSTD_decompress*` | `srcSize == 0` | `ZSTD_error_srcSize_wrong` (72) — too small for even a magic-number prefix | `decompress/zstd_decompress.c:418-419`, `:450`, `:458` | [x] |
| 1189 | `ZSTD_decompress*` | `dstCapacity == 0` and the frame is genuinely empty | `0` — accepted (see row 1177) | `decompress/zstd_decompress.c:901`, `:915` | [x] |
| 1190 | `ZSTD_getFrameContentSize` / `ZSTD_getDecompressedSize` | `srcSize` too small, bad magic, or an unsupported frame | `ZSTD_CONTENTSIZE_ERROR` (`0xFFFFFFFFFFFFFFFE`) — and `ZSTD_CONTENTSIZE_UNKNOWN` (`0xFFFFFFFFFFFFFFFF`) when the frame simply omits the size. `ZSTD_getDecompressedSize` collapses **both** sentinels to `0` | `include/zstd.h:203-204` | [x] |
| 1191 | `ZSTD_decompressBound` | any frame in the sequence fails to parse | `ZSTD_CONTENTSIZE_ERROR` | `decompress/zstd_decompress.c:814-820` | [x] |
| 1192 | any `srcSize` / `dstCapacity` larger than the real allocation | e.g. `srcSize` that overruns the buffer | **undetectable** — C has no way to validate a raw pointer + length pair; the result is an out-of-bounds read/write = UB. A port must validate slice lengths on the Rust side | (no source: absence of a check) | [n/a] |
| 1193 | `HUF_*` / `FSE_*` workspace sizes | `wkspSize` smaller than the documented minimum | `ZSTD_error_workSpace_tooSmall` (66) or `ZSTD_error_GENERIC` (1) depending on the function (see sections K, K2, K3, L) | `compress/hist.c:157`, `:169` and the entropy sections | [i] |

### Q3. Out-of-range enum values crossing the FFI boundary

C `enum` parameters accept **any** `int`. The table records what the C actually
does with a value outside the declared enumerators.

| # | enum / parameter | out-of-range value passed | expected C result | source | [ ] |
|---|------------------|---------------------------|-------------------|--------|-----|
| 1194 | `ZSTD_cParameter` -> `ZSTD_cParam_getBounds` | any `int` that is not one of the listed `ZSTD_c_*` values (including the deprecated/removed slots) | **rejected**: `bounds.error = ZSTD_error_parameter_unsupported` (40) via the `default:` arm; `lowerBound`/`upperBound` are left `0` | `compress/zstd_compress.c:633-635` | [x] |
| 1195 | `ZSTD_cParameter` -> `ZSTD_CCtx_setParameter` | unknown parameter id | **rejected**: `ZSTD_error_parameter_unsupported` (40) ("unknown parameter") from the `default:` arm of the mid-stream whitelist switch | `compress/zstd_compress.c:765` | [x] |
| 1196 | `ZSTD_cParameter` -> `ZSTD_CCtxParams_setParameter` | unknown parameter id | **rejected**: `ZSTD_error_parameter_unsupported` (40) | `compress/zstd_compress.c:1019` | [x] |
| 1197 | `ZSTD_cParameter` -> `ZSTD_CCtxParams_getParameter` | unknown parameter id | **rejected**: `ZSTD_error_parameter_unsupported` (40); `*value` is left untouched | `compress/zstd_compress.c:1166` | [x] |
| 1198 | `ZSTD_cParameter` -> `ZSTD_isUpdateAuthorized` (mid-stream check) | unknown parameter id | falls into `default: return 0` -> the caller reports `ZSTD_error_stage_wrong` (60) *before* the unknown-parameter check runs, so **a mid-stream call with a bogus id reports `stage_wrong` (60), not `parameter_unsupported` (40)** | `compress/zstd_compress.c:703-705`, `:759-765` | [i] |
| 1199 | `ZSTD_dParameter` -> `ZSTD_dParam_getBounds` | any value other than the 7 supported `ZSTD_d_*` ids | **rejected**: `bounds.error = ZSTD_error_parameter_unsupported` (40) via `default:;` then the trailing assignment | `decompress/zstd_decompress.c:1855-1858` | [x] |
| 1200 | `ZSTD_dParameter` -> `ZSTD_DCtx_setParameter` | unknown parameter id | **rejected**: `ZSTD_error_parameter_unsupported` (40) after the `default:;` falls out of the switch. Note the `streamStage != zdss_init` check runs **first**, so a mid-stream call with a bogus id yields `ZSTD_error_stage_wrong` (60) | `decompress/zstd_decompress.c:1907`, `:1942-1943` | [x] |
| 1201 | `ZSTD_dParameter` -> `ZSTD_DCtx_getParameter` | unknown parameter id | **rejected**: `ZSTD_error_parameter_unsupported` (40) | `decompress/zstd_decompress.c:1901-1903` | [x] |
| 1202 | `ZSTD_strategy` -> `ZSTD_c_strategy` via `setParameter` / `ZSTD_checkCParams` | outside `ZSTD_fast` (1) .. `ZSTD_btultra2` (9) — including `0` | **rejected**: `ZSTD_error_parameter_outOfBound` (42) via `BOUNDCHECK` | `compress/zstd_compress.c:650-654`, `:1390-1396` | [x] |
| 1203 | `ZSTD_strategy` -> internal block-compressor dispatch (`ZSTD_ldm_skipSequences`-style `switch (strategy)`) | a value that slipped past validation (only possible via a hand-built `ZSTD_compressionParameters`) | `assert(0)` — compiled out at `DEBUGLEVEL 0`, so the `switch` simply falls through and **no** table update happens = silent corruption, not an error | `compress/zstd_compress.c:5034-5035` | [i] |
| 1204 | `ZSTD_ResetDirective` -> `ZSTD_CCtx_reset` | any value other than `ZSTD_reset_session_only` (1) / `ZSTD_reset_parameters` (2) / `ZSTD_reset_session_and_parameters` (3), e.g. `0` or `99` | **silently ignored**: both `if` chains are equality tests, so nothing happens and the function returns `0` (success). There is **no** `default:`/`parameter_outOfBound` rejection | `compress/zstd_compress.c:1367-1381` | [x] |
| 1205 | `ZSTD_ResetDirective` -> `ZSTD_DCtx_reset` | any value outside 1..3 | **silently ignored**, returns `0` (success) | `decompress/zstd_decompress.c:1947-1961` | [x] |
| 1206 | `ZSTD_EndDirective` -> `ZSTD_compressStream2` | `(U32)endOp > (U32)ZSTD_e_end` (i.e. `> 2`, including negative values, which become huge when cast to `U32`) | **rejected**: `ZSTD_error_parameter_outOfBound` (42) ("invalid endDirective") | `compress/zstd_compress.c:6456` | [x] |
| 1207 | `ZSTD_EndDirective` -> internal `ZSTD_compressStream_generic` | a value that got past row 1206 (impossible) | `assert((U32)flushMode <= (U32)ZSTD_e_end)` only — no runtime rejection | `compress/zstd_compress.c:6137` | [i] |
| 1208 | `ZSTD_format_e` -> `ZSTD_c_format` / `ZSTD_d_format` via `setParameter` | outside `ZSTD_f_zstd1` (0) .. `ZSTD_f_zstd1_magicless` (1) | **rejected**: `ZSTD_error_parameter_outOfBound` (42) via `BOUNDCHECK` / `CHECK_DBOUNDS` | `compress/zstd_compress.c:776-779` (bounds at `:549-552`), `decompress/zstd_decompress.c:1915-1918` | [i] |
| 1209 | `ZSTD_format_e` -> `ZSTD_getFrameHeader_advanced` / `ZSTD_startingInputLength` (passed **directly**, bypassing `setParameter`) | any value other than 0 or 1 | `assert((format == ZSTD_f_zstd1) \|\| (format == ZSTD_f_zstd1_magicless))` only — compiled out at `DEBUGLEVEL 0`. `ZSTD_FRAMEHEADERSIZE_PREFIX(format)` evaluates `((format)==ZSTD_f_zstd1 ? 5 : 1)`, so **any** non-zero value is silently treated as magicless | `decompress/zstd_decompress.c:232-237`, `include/zstd.h:1257` | [x] |
| 1210 | `ZSTD_dictContentType_e` -> `ZSTD_CCtx_loadDictionary_advanced` / `ZSTD_createCDict_advanced` | outside `ZSTD_dct_auto` (0) / `ZSTD_dct_rawContent` (1) / `ZSTD_dct_fullDict` (2) | **silently accepted**: the code is a chain of `==` comparisons with no `default:` — a bogus value is neither `rawContent` nor `fullDict` nor `auto`, so it behaves like "auto but never reject", i.e. a non-dictionary buffer is quietly treated as raw content and **no** `dictionary_wrong` (32) is raised | `compress/zstd_compress.c:5206-5223` | [x] |
| 1211 | `ZSTD_dictContentType_e` -> `ZSTD_loadEntropy_intoDDict` | outside 0..2 | **silently accepted**: `if (dictContentType == ZSTD_dct_rawContent) return 0;` then two `if (dictContentType == ZSTD_dct_fullDict)` rejection sites are skipped, so a bogus value behaves like `ZSTD_dct_auto` — a corrupt dictionary is accepted as raw content instead of yielding `dictionary_corrupted` (30) | `decompress/zstd_ddict.c:95`, `:98`, `:104` | [i] |
| 1212 | `ZSTD_dictLoadMethod_e` -> `ZSTD_createCDict_advanced` / `ZSTD_createDDict_advanced` / `ZSTD_estimateCDictSize_advanced` | outside `ZSTD_dlm_byCopy` (0) / `ZSTD_dlm_byRef` (1) | **silently accepted and treated as `byCopy`**: every use is `(dictLoadMethod == ZSTD_dlm_byRef)`, so any other value takes the copy path. No rejection exists | `compress/zstd_compress.c:1295`, `:5532`, `:5562`, `:5619`, `:5769` | [x] |
| 1213 | `ZSTD_ErrorCode` -> `ZSTD_getErrorString` | any `int` that is not a listed `ZSTD_error_*` value (or `ZSTD_error_maxCode` itself) | **not** an error — returns the static string `"Unspecified error code"` from the `default:` arm. Never NULL, never crashes | `common/zstd_common.c:48` -> `common/error_private.c:21`, `:60-61` | [x] |
| 1214 | `ZSTD_ErrorCode` -> `ZSTD_getErrorCode` (the inverse direction) | a `size_t` that is not an error (`code <= (size_t)-120`) | `0` (`ZSTD_error_no_error`) — indistinguishable from a genuine "no error" | `common/zstd_common.c:44` -> `common/error_private.h:53` | [x] |
| 1215 | `ZSTD_literalCompressionMode_e` (a typedef of `ZSTD_ParamSwitch_e`) -> `ZSTD_c_literalCompressionMode` via `setParameter` | outside `ZSTD_ps_auto` (0) / `ZSTD_ps_enable` (1) / `ZSTD_ps_disable` (2) | **rejected**: `ZSTD_error_parameter_outOfBound` (42) via `BOUNDCHECK(ZSTD_c_literalCompressionMode, ...)` | `compress/zstd_compress.c:859-863` | [i] |
| 1216 | `ZSTD_ParamSwitch_e` -> `ZSTD_literalsCompressionIsDisabled` (internal consumer) | a value that reached the struct without going through `setParameter` (e.g. a hand-built `ZSTD_CCtx_params`) | `default: assert(0 /* impossible: pre-validated */)` then `ZSTD_FALLTHROUGH` into the `ZSTD_ps_auto` arm — at `DEBUGLEVEL 0` this is a **silent fallback to `auto`**, not an error | `compress/zstd_compress_internal.h:685-698` | [i] |
| 1217 | `ZSTD_ParamSwitch_e` -> `ZSTD_resolveRowMatchFinderMode` / `ZSTD_resolveBlockSplitterMode` / `ZSTD_resolveEnableLdm` / `ZSTD_resolveExternalRepcodeSearch` | any value `!= ZSTD_ps_auto` | **passed through verbatim** (`if (mode != ZSTD_ps_auto) return mode;`) — a bogus value therefore propagates into the compressor as neither enable nor disable, and downstream `== ZSTD_ps_enable` tests all read false, i.e. it behaves like "disable" without any error | `compress/zstd_compress.c:238-244`, `:248-252`, `:269-273`, `:288-295` | [i] |
| 1218 | `ZSTD_ParamSwitch_e` -> `ZSTD_c_enableLongDistanceMatching` / `ZSTD_c_useRowMatchFinder` / `ZSTD_c_splitAfterSequences` / `ZSTD_c_prefetchCDictTables` / `ZSTD_c_repcodeResolution` via `setParameter` | outside 0..2 | **rejected**: `ZSTD_error_parameter_outOfBound` (42) via `BOUNDCHECK` against `lowerBound = ZSTD_ps_auto`, `upperBound = ZSTD_ps_disable` | `compress/zstd_compress.c:513-516`, `:628-631` (bounds), `:650-654` (`BOUNDCHECK`) | [i] |
| 1219 | `ZSTD_SequenceFormat_e` -> `ZSTD_c_blockDelimiters` via `setParameter` | outside `ZSTD_sf_noBlockDelimiters` (0) / `ZSTD_sf_explicitBlockDelimiters` (1) | **rejected**: `ZSTD_error_parameter_outOfBound` (42) via `BOUNDCHECK(ZSTD_c_blockDelimiters, value)` | `compress/zstd_compress.c:967-969`, bounds at `:583-586` | [i] |
| 1220 | `ZSTD_SequenceFormat_e` -> `ZSTD_selectSequenceCopier` (internal) | a value that got past validation | `assert(ZSTD_cParam_withinBounds(...))` then `if (mode == ZSTD_sf_explicitBlockDelimiters) ... ; assert(mode == ZSTD_sf_noBlockDelimiters); return ZSTD_transferSequences_noDelim;` — at `DEBUGLEVEL 0` **any** non-`explicit` value silently selects the no-delimiter copier | `compress/zstd_compress.c:6881-6889` | [i] |
| 1221 | `ZSTD_dictAttachPref_e` -> `ZSTD_c_forceAttachDict` via `setParameter` | outside `ZSTD_dictDefaultAttach` (0) / `ZSTD_dictForceAttach` (1) / `ZSTD_dictForceCopy` (2) / `ZSTD_dictForceLoad` (3) | **rejected**: `ZSTD_error_parameter_outOfBound` (42) via `BOUNDCHECK` against `lowerBound = ZSTD_dictDefaultAttach`, `upperBound = ZSTD_dictForceLoad` | `compress/zstd_compress.c:555-558` (bounds), `:853-857` | [i] |
| 1222 | `ZSTD_dictAttachPref_e` -> `ZSTD_resolveDictAttachment` / `ZSTD_resetCCtx_usingCDict` (internal consumer) | a value that got past validation | **no `default:`** — the logic is `(... \|\| pref == ZSTD_dictForceAttach) && pref != ZSTD_dictForceCopy` and `pref != ZSTD_dictForceLoad`, so a bogus value silently degrades to the heuristic "default attach" behaviour | `compress/zstd_compress.c:2318-2319`, `:5260` | [i] |
| 1223 | frame-header `dictIDSizeCode` / `fcsCode` switches in `ZSTD_writeFrameHeader` | a value outside 0..3 (impossible — both are 2-bit fields) | `default: assert(0); ZSTD_FALLTHROUGH;` into `case 0` — at `DEBUGLEVEL 0` a silent no-op, not an error | `compress/zstd_compress.c:4722-4726`, `:4732-4736` | [i] |
| 1224 | `ZSTD_bufferMode_e` (`ZSTD_c_stableInBuffer` / `ZSTD_c_stableOutBuffer` / `ZSTD_d_stableOutBuffer`) | outside `ZSTD_bm_buffered` (0) / `ZSTD_bm_stable` (1) | **rejected**: `ZSTD_error_parameter_outOfBound` (42) via `BOUNDCHECK` / `CHECK_DBOUNDS`; internal consumers then compare `== ZSTD_bm_stable`, so an unvalidated value would behave as "buffered" | `compress/zstd_compress.c:650-654`, `decompress/zstd_decompress.c:1919-1922` | [i] |
| 1225 | `ZSTD_forceIgnoreChecksum_e` / `ZSTD_refMultipleDDicts_e` (`ZSTD_d_forceIgnoreChecksum`, `ZSTD_d_refMultipleDDicts`) | outside their 0..1 ranges | **rejected**: `ZSTD_error_parameter_outOfBound` (42) via `CHECK_DBOUNDS`. `ZSTD_d_refMultipleDDicts` additionally returns `ZSTD_error_parameter_unsupported` (40) on a static DCtx | `decompress/zstd_decompress.c:1923-1933` | [i] |
| 1226 | any 0/1 "flag" cParameter (`ZSTD_c_contentSizeFlag`, `ZSTD_c_checksumFlag`, `ZSTD_c_dictIDFlag`, `ZSTD_c_forceMaxWindow`, `ZSTD_c_enableDedicatedDictSearch`, `ZSTD_c_validateSequences`, `ZSTD_c_deterministicRefPrefix`, `ZSTD_c_enableSeqProducerFallback`, `ZSTD_c_disableHuffmanAssembly`) | any `int`, e.g. `-1` or `42` | **not** rejected — coerced with `value != 0` (or `!!value`), so any non-zero becomes 1 | `compress/zstd_compress.c:832-851`, `:940-941` and the corresponding `setParameter` arms | [i] |
| 1227 | `ZSTD_c_compressionLevel` | any `int`, including `INT_MIN` / `INT_MAX` | **never** rejected — clamped into `[ZSTD_minCLevel(), ZSTD_maxCLevel()]` = `[-131072, 22]`; `0` is remapped to `ZSTD_CLEVEL_DEFAULT` (3) | `compress/zstd_compress.c:425-427`, `:7674` | [i] |

**Summary for a port.** Across the whole API there are exactly three enum-shaped
behaviours to reproduce: (a) **rejected** with
`ZSTD_error_parameter_unsupported` (40) — unknown `ZSTD_cParameter` /
`ZSTD_dParameter` ids; (b) **rejected** with
`ZSTD_error_parameter_outOfBound` (42) — every enum that passes through
`BOUNDCHECK`/`CHECK_DBOUNDS` (`ZSTD_strategy`, `ZSTD_format_e`,
`ZSTD_ParamSwitch_e`, `ZSTD_SequenceFormat_e`, `ZSTD_dictAttachPref_e`,
`ZSTD_bufferMode_e`, ...) plus `ZSTD_EndDirective`; and (c) **silently
accepted** — `ZSTD_ResetDirective` (no-op), `ZSTD_dictContentType_e` and
`ZSTD_dictLoadMethod_e` (fall through equality chains), `ZSTD_ErrorCode` into
`ZSTD_getErrorString` (`"Unspecified error code"`), plus every `assert(0)`
`default:` arm which at `DEBUGLEVEL 0` degrades to a silent fallback rather than
an error.
