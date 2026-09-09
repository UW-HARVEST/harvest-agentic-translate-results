# CONFIGS.md — configuration surface (VALID inputs) of the C zstd library

Derived mechanically from `c_src/src` (public headers + the `if`/`switch` branches the C
actually takes). Build config: `ZSTD_LEGACY_SUPPORT=5`, `XXH_NAMESPACE=ZSTD_`,
`DYNAMIC_BMI2=0`, **`ZSTD_MULTITHREAD` NOT defined** (so `nbWorkers`/`jobSize`/
`overlapLog`/`rsyncable` reject non-zero with `parameter_unsupported`).
`CMakeLists.txt` globs *all* of `src/legacy/*.c`, so v01..v07 symbols exist, but
`ZSTD_isLegacy()` only auto-dispatches v05/v06/v07.

## Axes

### A1 — compression level
`ZSTD_minCLevel() = -131072`, `ZSTD_maxCLevel() = 22`, `ZSTD_defaultCLevel() = 3`.
Level is *clamped*, never rejected. `0` → 3. Negative levels use table row 0 and set
`targetLength = -level`. cParam table row selected by
`tableID = (rSize<=256KB) + (rSize<=128KB) + (rSize<=16KB)` → 4 tables (driven by
`srcSizeHint` / pledgedSrcSize / dictSize).

| strategy | value | levels (tableID 0) | representative |
|---|---|---|---|
| ZSTD_fast | 1 | ≤0, 1, 2 | 1, −5, −131072 |
| ZSTD_dfast | 2 | 3, 4 | 3 |
| ZSTD_greedy | 3 | 5 | 5 |
| ZSTD_lazy | 4 | 6, 7 | 6 |
| ZSTD_lazy2 | 5 | 8–12 | 9 |
| ZSTD_btlazy2 | 6 | 13–15 | 13 |
| ZSTD_btopt | 7 | 16, 17 | 16 |
| ZSTD_btultra | 8 | 18 | 18 |
| ZSTD_btultra2 | 9 | 19–22 | 19 |

### A2 — `ZSTD_cParameter` (all 39 accepted ids + bounds)
100 compressionLevel [-131072,22] · 101 windowLog [10,31] · 102 hashLog [6,30] ·
103 chainLog [6,30] · 104 searchLog [1,30] · 105 minMatch [3,7] · 106 targetLength [0,131072] ·
107 strategy [1,9] · 130 targetCBlockSize [1340,131072] · 160 enableLDM [0,2] ·
161 ldmHashLog [6,30] · 162 ldmMinMatch [4,4096] · 163 ldmBucketSizeLog [1,8] ·
164 ldmHashRateLog [0,25] · 200 contentSizeFlag [0,1] · 201 checksumFlag [0,1] ·
202 dictIDFlag [0,1] · 400 nbWorkers [0,0] · 401 jobSize [0,0] · 402 overlapLog [0,0] ·
500 rsyncable · 10 format [0,1] · 1000 forceMaxWindow [0,1] · 1001 forceAttachDict [0,3] ·
1002 literalCompressionMode [0,2] · 1004 srcSizeHint [0,INT_MAX] ·
1005 enableDedicatedDictSearch [0,1] · 1006 stableInBuffer · 1007 stableOutBuffer ·
1008 blockDelimiters [0,1] · 1009 validateSequences · 1010 splitAfterSequences [0,2] ·
1011 useRowMatchFinder [0,2] · 1012 deterministicRefPrefix · 1013 prefetchCDictTables [0,2] ·
1014 enableSeqProducerFallback · 1015 maxBlockSize [1024,131072] ·
1016 repcodeResolution [0,2] · 1017 blockSplitterLevel [0,6].
(1003 = removed `experimentalParam6` → `parameter_unsupported`.)

### A3 — `ZSTD_dParameter`
100 windowLogMax [10,31] (default 27) · 1000 format [0,1] · 1001 stableOutBuffer [0,1] ·
1002 forceIgnoreChecksum [0,1] · 1003 refMultipleDDicts [0,1] · 1004 disableHuffmanAssembly
[0,1] · 1005 maxBlockSize {0} ∪ [1024,131072]. All `ZSTD_DCtx_setParameter` calls require
`streamStage == zdss_init`.

### A4 — frame-format constants
`MAGICNUMBER=0xFD2FB528`, `MAGIC_DICTIONARY=0xEC30A437`,
`MAGIC_SKIPPABLE_START=0x184D2A50` (+0..15), `SKIPPABLEHEADERSIZE=8`,
`FRAMEHEADERSIZE_PREFIX=5` (1 magicless), `_MAX=18`, `BLOCKSIZE_MAX=131072`,
`BLOCKSIZE_MAX_MIN=1024`, `CONTENTSIZE_UNKNOWN=(U64)-1`, `CONTENTSIZE_ERROR=(U64)-2`.

### A5 — input shapes
0, 1, 2, 3, 7, 16, 63, 100, 999, 4096, 65535, **131072** (`BLOCKSIZE_MAX`), 131073,
200000, 262144, 400000, 1 MB; content classes: random/incompressible, RLE (all one byte),
short-period repetitive, text-like, long-distance matches, mixed, sparse; window-wrap
(input ≫ 1<<windowLog); pledgedSrcSize unknown / exact / wrong.

### A6 — streaming shapes
in-chunk ∈ {1, 2, 7, 100, 1000, 4096, 131072, `CStreamInSize()`}; out-chunk ∈ {1, 7, 1000,
`CStreamOutSize()`}; directives `ZSTD_e_continue=0`, `e_flush=1`, `e_end=2` and their
interleavings; old API (`compressStream`/`flushStream`/`endStream`, `resetCStream`,
`initCStream_srcSize/_advanced/_usingDict/_usingCDict`); resets 1/2/3.

### A7 — multi-frame / skippable / magicless
concatenated frames (mixed levels & checksum flags); skippable frames before/between/after
(magic variants 0,1,7,15 × payload 0,1,8,1000); magicless (`c_format=1` + `d_format=1`) and
the mismatched cross-combination.

### A8 — dictionary shapes
none / raw-content (0,1,256,8192,112640 B) / trained (`ZDICT_*`, magic + dictID) / prefix;
`dlm_byCopy|byRef`; `dct_auto|rawContent|fullDict`; `forceAttachDict 0..3`;
dedicated-dict-search; `d_refMultipleDDicts`; `dictIDFlag=0`; static CDict/DDict.

### A9 — dictBuilder axes
`ZDICT_trainFromBuffer` (fastCover optimize, d=8, steps=4, level=3);
`_cover` (k ∈ {16,50,200,1000,2048}, d ∈ {6,8,16}, splitPoint {0→1.0,0.5,0.75,1.0},
shrinkDict {0,1}, shrinkDictMaxRegression {0,1,5}); `_optimizeTrainFromBuffer_cover`
(steps {0→40,1,4,8}); `_fastCover` (f ∈ {0→20,1,6,20,25,31}, accel ∈ {0→1,1,2,5,10});
`_optimizeTrainFromBuffer_fastCover`; `_legacy` (selectivity {0→9,1,5,9,12,31});
`ZDICT_finalizeDictionary` (maxDictSize {256,1024,8192,…}); dictID MUST be pinned non-zero
(0 ⇒ randomized output).

### A10 — entropy / hash / pool axes
FSE tableLog {5,6,9,11,12} (13 → error), maxSymbolValue {1,15,100,255},
useLowProbCount {0,1}; HUF huffLog {5,8,11,12}, flags bitmask {0,1,2,4,8,16,32,48,63},
`HUF_repeat` {none,check,valid}, 1X vs 4X, X1 vs X2; XXH32/64 one-shot and streaming over
lengths {0,1,3,4,7,8,15,16,31,32,33,63,64,127,128,1000,131072} × several seeds, `copyState`,
canonical round-trip; POOL numThreads {0,1,2,4,16} × queueSize {0,1,4}.

---

## Cross-product matrix

Legend for the check column: `[x]` = covered by a passing differential test across
randomized inputs (test names in the last column, see `tests/`).

| # | entry point(s) | configuration (options set + input shape) | [ ] | test |
|---|----------------|-------------------------------------------|-----|------|
| 1 | ZSTD_compressCCtx/decompressDCtx | level 1 (ZSTD_fast), text 300000 B | [x] | cfg_levels_x_shapes_simple_api, cfg_cctx_dctx_oneshot |
| 2 | ZSTD_compressCCtx/decompressDCtx | level 3 (dfast), text 300000 B | [x] | cfg_levels_x_shapes_simple_api |
| 3 | ZSTD_compressCCtx/decompressDCtx | level 5 (greedy), text 300000 B | [x] | cfg_levels_x_shapes_simple_api |
| 4 | ZSTD_compressCCtx/decompressDCtx | level 6 (lazy), text 300000 B | [x] | cfg_levels_x_shapes_simple_api |
| 5 | ZSTD_compressCCtx/decompressDCtx | level 9 (lazy2), text 300000 B | [x] | cfg_levels_x_shapes_simple_api |
| 6 | ZSTD_compressCCtx/decompressDCtx | level 13 (btlazy2), text 300000 B | [x] | cfg_levels_x_shapes_simple_api |
| 7 | ZSTD_compressCCtx/decompressDCtx | level 16 (btopt), text 300000 B | [x] | cfg_all_levels_random_sizes |
| 8 | ZSTD_compressCCtx/decompressDCtx | level 18 (btultra), text 300000 B | [x] | cfg_all_levels_random_sizes |
| 9 | ZSTD_compressCCtx/decompressDCtx | level 19 (btultra2), text 300000 B | [x] | cfg_levels_x_shapes_simple_api |
| 10 | ZSTD_compress | level 22, text 300000 B | [x] | cfg_levels_x_shapes_simple_api |
| 11 | ZSTD_compress | level -1, incompressible 131072 B | [x] | cfg_all_levels_random_sizes |
| 12 | ZSTD_compress | level -5, highly compressible 262144 B | [x] | cfg_levels_x_shapes_simple_api |
| 13 | ZSTD_compress | level -131072 (minCLevel), random 131072 B | [x] | cfg_all_levels_random_sizes |
| 14 | ZSTD_compress | level 0 vs 3 (0 resolves to 3) | [x] | cfg_all_levels_random_sizes, cfg_level_equivalences |
| 15 | ZSTD_compress | level 100 (clamped to 22) vs 22 | [x] | cfg_level_equivalences |
| 16 | ZSTD_compress2 + c_strategy | strategy=1 fast, windowLog 20, text 300000 B | [x] | cfg_all_strategies_manual_params |
| 17 | ZSTD_compress2 + c_strategy | strategy=2 dfast, text 300000 B | [x] | cfg_all_strategies_manual_params |
| 18 | ZSTD_compress2 + c_strategy | strategy=3 greedy, useRowMatchFinder=1, wlog 20 | [x] | cfg_rowmatchfinder_matrix |
| 19 | ZSTD_compress2 + c_strategy | strategy=3 greedy, useRowMatchFinder=2, wlog 20 | [x] | cfg_rowmatchfinder_matrix |
| 20 | ZSTD_compress2 + c_strategy | strategy=4 lazy, rowMF auto, windowLog 14 vs 15 | [x] | cfg_rowmatchfinder_matrix |
| 21 | ZSTD_compress2 + c_strategy | strategy=5 lazy2, rowMF=1, text 300000 B | [x] | cfg_rowmatchfinder_matrix |
| 22 | ZSTD_compress2 + c_strategy | strategy=6 btlazy2, text 300000 B | [x] | cfg_all_strategies_manual_params |
| 23 | ZSTD_compress2 + c_strategy | strategy=7 btopt, targetLength=48, minMatch=3 | [x] | cfg_all_strategies_manual_params |
| 24 | ZSTD_compress2 + c_strategy | strategy=8 btultra, targetLength=999 | [x] | cfg_all_strategies_manual_params |
| 25 | ZSTD_compress2 + c_strategy | strategy=9 btultra2, targetLength=0 | [x] | cfg_all_strategies_manual_params |
| 26 | ZSTD_compress2 | windowLog=10 + 4096 B input (window wrap/extDict) | [x] | cfg_all_strategies_manual_params, cfg_window_wrap |
| 27 | ZSTD_compress2 | windowLog=17, 300000 B (multi wrap, lazy2) | [x] | cfg_window_wrap |
| 28 | ZSTD_compress2 / d_windowLogMax | windowLog=31 then decode with dWLM 31 and default 27 | [x] | cfg_window_wrap, cfg_every_dparam |
| 29 | ZSTD_compress2 | hashLog=6 and 25, strategy=fast, 131072 B | [x] | cfg_every_cparam_at_bounds |
| 30 | ZSTD_compress2 | chainLog=6 and 25, lazy2 + rowMF disabled, 262144 B | [x] | cfg_every_cparam_at_bounds, cfg_rowmatchfinder_matrix |
| 31 | ZSTD_compress2 | searchLog=1 and 12, btopt, 262144 B | [x] | cfg_every_cparam_at_bounds |
| 32 | ZSTD_compress2 | minMatch=3, btultra, 262144 B | [x] | cfg_all_strategies_manual_params |
| 33 | ZSTD_compress2 | minMatch=7, fast, 262144 B | [x] | cfg_all_strategies_manual_params |
| 34 | ZSTD_compress2 | minMatch=4,5,6, greedy, 262144 B | [x] | cfg_all_strategies_manual_params |
| 35 | ZSTD_compress2 | targetLength=0,1,131072, btopt | [x] | cfg_every_cparam_at_bounds |
| 36 | ZSTD_compress2 | contentSizeFlag 0 vs 1 with pledged size known | [x] | cfg_flag_combinations |
| 37 | ZSTD_compress2 | contentSizeFlag=1 with pledged size unknown | [x] | cfg_flag_combinations |
| 38 | ZSTD_compress2/decompressDCtx | checksumFlag=1, 262144 B; corrupt tail | [x] | cfg_flag_combinations, err_checksum_wrong |
| 39 | decompressDCtx | checksum corrupted + d_forceIgnoreChecksum=1 | [x] | err_checksum_ignored |
| 40 | ZSTD_compress2 + CDict | dictIDFlag=0 with trained dict | [x] | cfg_dictID_flag_and_queries |
| 41 | ZSTD_compress2 | format=magicless + contentSizeFlag; decode fmt 1 and fmt 0 | [x] | cfg_flag_combinations, err_magicless_mismatch |
| 42 | ZSTD_compress2 | forceMaxWindow=1, windowLog=20, 1000 B | [x] | cfg_more_experimental_matrix |
| 43 | ZSTD_compress2 | literalCompressionMode=1 (huffman) on random 131072 B | [x] | cfg_misc_experimental_matrix |
| 44 | ZSTD_compress2 | literalCompressionMode=2 (uncompressed) on text | [x] | cfg_misc_experimental_matrix |
| 45 | ZSTD_compress2 | litCompressionMode auto at level -3 vs 3 | [x] | cfg_misc_experimental_matrix |
| 46 | ZSTD_compress2 | enableLDM=1 default params, 2 MB far-apart repeats | [x] | cfg_ldm_matrix |
| 47 | ZSTD_compress2 | LDM + ldmHashLog 6 / 20 | [x] | cfg_ldm_matrix |
| 48 | ZSTD_compress2 | LDM + ldmMinMatch 4 / 64 / 4096 | [x] | cfg_ldm_matrix |
| 49 | ZSTD_compress2 | LDM + ldmBucketSizeLog 1 / 8 | [x] | cfg_ldm_matrix |
| 50 | ZSTD_compress2 | LDM + ldmHashRateLog 0 / 25 | [x] | cfg_ldm_matrix |
| 51 | ZSTD_compress2 | enableLDM=0(auto) btopt wlog 27 vs 26 | [x] | cfg_ldm_matrix |
| 52 | ZSTD_compress2 | targetCBlockSize=1340, 262144 B text (superblock) | [x] | cfg_more_experimental_matrix |
| 53 | ZSTD_compress2 | targetCBlockSize=1000 raised to 1340 (== row 52) | [x] | cfg_targetcblocksize_clamp |
| 54 | ZSTD_compress2 | targetCBlockSize=131072, 262144 B | [x] | cfg_every_cparam_at_bounds |
| 55 | ZSTD_compress2 | targetCBlockSize=2000 + checksum + random 300000 B | [x] | cfg_more_experimental_matrix |
| 56 | ZSTD_compress2 | srcSizeHint=10000 (tableID 3), level 11 | [x] | cfg_srcsizehint_tables |
| 57 | ZSTD_compress2 | srcSizeHint=100000 (tableID 2), level 11 | [x] | cfg_srcsizehint_tables |
| 58 | ZSTD_compress2 | srcSizeHint=200000 (tableID 1), level 11 | [x] | cfg_srcsizehint_tables |
| 59 | ZSTD_compress2 | srcSizeHint=INT_MAX (tableID 0), level 11 | [x] | cfg_srcsizehint_tables, cfg_every_cparam_at_bounds |
| 60 | ZSTD_compress2 | maxBlockSize=1024, 65536 B | [x] | cfg_more_experimental_matrix |
| 61 | ZSTD_compress2/decompressStream | maxBlockSize=32768 + d_maxBlockSize 32768 / 1024 | [x] | cfg_maxblocksize_pairs |
| 62 | ZSTD_compress2 | blockSplitterLevel=0 for fast/lazy2/btultra2, 400000 B | [x] | cfg_misc_experimental_matrix |
| 63 | ZSTD_compress2 | blockSplitterLevel=1 (never presplit) | [x] | cfg_misc_experimental_matrix |
| 64 | ZSTD_compress2 | blockSplitterLevel=2 (splitBlock level 0) | [x] | cfg_misc_experimental_matrix |
| 65 | ZSTD_compress2 | blockSplitterLevel=3,4,5,6 | [x] | cfg_blocksplitter_levels |
| 66 | ZSTD_compress2 | splitAfterSequences 0/1/2 × btopt wlog 17/16 | [x] | cfg_splitaftersequences |
| 67 | ZSTD_compress2 | deterministicRefPrefix=1 + refPrefix 8192 B | [x] | cfg_refPrefix_variants |
| 68 | ZSTD_compress2 | prefetchCDictTables 1 vs 2 with 65536 B CDict | [x] | cfg_refCDict_attach_prefs |
| 69 | ZSTD_compress2 | enableSeqProducerFallback=1, no producer | [x] | cfg_every_cparam_at_bounds |
| 70 | ZSTD_CCtx_setParameter | nbWorkers/jobSize/overlapLog/rsyncable = 1 → unsupported; 0 ok | [x] | cfg_stream2_multithreaded, err_mt_params_unsupported |
| 71 | ZSTD_CCtx_setParameter | param 1003 and 9999 → unsupported; each param ±1 past bounds | [x] | err_cparam_out_of_bound |
| 72 | ZSTD_cParam_getBounds / ZSTD_dParam_getBounds | all params + invalid ids | [x] | cfg_cParam_getBounds_all, cfg_dParam_getBounds_all |
| 73 | ZSTD_CCtx_getParameter / CCtxParams_getParameter | read back every param | [x] | cfg_every_cparam_at_bounds, cfg_cctxparams_api |
| 74 | createCCtxParams+init+setParameter+setParametersUsingCCtxParams+compress2 | level 12, btopt, checksum | [x] | cfg_cctxparams_api |
| 75 | ZSTD_CCtxParams_init_advanced | explicit ZSTD_parameters from getParams(9,300000,0) | [x] | cfg_cctxparams_api |
| 76 | ZSTD_CCtxParams_reset | after level 19 → back to defaults | [x] | cfg_cctxparams_api |
| 77 | getCParams/getParams/adjustCParams/checkCParams/cycleLog | levels × srcSizeHint × dictSize sweep | [x] | cfg_getcparams_sweep |
| 78 | compressStream2/decompressStream | level 3, in 1 B, out 1 B, 4096 B | [x] | cfg_stream2_chunk_matrix |
| 79 | compressStream2/decompressStream | level 3, in 7 B, out 7 B, 100000 B | [x] | cfg_stream2_chunk_matrix |
| 80 | compressStream2/decompressStream | level 9, in 1000, out 1000, 300000 B | [x] | cfg_stream2_chunk_matrix, cfg_stream2_params_matrix |
| 81 | compressStream2 | in-chunk 131072 (exactly one block), out=CStreamOutSize, 262144 B | [x] | cfg_stream2_chunk_matrix |
| 82 | compressStream2 | e_flush every 1000 B chunk, 100000 B | [x] | cfg_stream2_endmode_patterns |
| 83 | compressStream2 | e_flush at byte 1, then continue, then end | [x] | cfg_stream2_endmode_patterns |
| 84 | compressStream2 | e_end with 0-byte input (empty frame) | [x] | cfg_stream2_chunk_matrix |
| 85 | compressStream2 | pledgedSrcSize exact, then N+1 / N-1 → srcSize_wrong | [x] | cfg_stream2_params_matrix, err_pledged_srcsize_wrong |
| 86 | compressStream2 | stableInBuffer=1 whole input once; then violate → stabilityCondition | [x] | cfg_stable_buffers, err_stable_in_violation |
| 87 | compressStream2 | stableOutBuffer=1 with compressBound-size out; then shrink → error | [x] | cfg_stable_buffers, err_stable_out_violation |
| 88 | compressStream2 | stableIn + stableOut + e_end one-shot, 262144 B | [x] | cfg_stable_buffers |
| 89 | compressStream/flushStream/endStream | initCStream(level 5), in 1000, out CStreamOutSize | [x] | cfg_old_cstream_api |
| 90 | ZSTD_initCStream_srcSize + old API | level 7, srcSize exact, in-chunk 7 | [x] | cfg_old_cstream_init_variants |
| 91 | ZSTD_initCStream_advanced + old API | explicit params (checksum, contentSize) + pledged | [x] | cfg_old_cstream_init_variants |
| 92 | initCStream_usingDict / _usingCDict(_advanced) | 8192 B raw dict, level 6 | [x] | cfg_old_cstream_init_variants |
| 93 | ZSTD_resetCStream | 3 frames with pledged {0,1,131072} | [x] | cfg_old_cstream_init_variants |
| 94 | decompressStream | out-chunk 1 B on 262144 B frame with checksum | [x] | cfg_stream2_chunk_matrix |
| 95 | decompressStream | in-chunk 1 B over a 3-frame concatenation | [x] | cfg_stream_decompress_multiframe_and_skippable |
| 96 | decompressStream | d_stableOutBuffer=1 exact buffer, known FCS; unknown FCS path | [x] | cfg_dstream_stable_out |
| 97 | initDStream_usingDict/_usingDDict/resetDStream | dict frame, out-chunk 1000 | [x] | cfg_dstream_init_variants |
| 98 | decompressStream | d_disableHuffmanAssembly 0 vs 1 (identical output) | [x] | cfg_every_dparam, cfg_dstream_init_variants |
| 99 | compressStream2_simpleArgs / decompressStream_simpleArgs | level 6, 100000 B, 4096 chunks | [x] | cfg_simple_args |
| 100 | compressBegin + compressContinue×N + compressEnd | level 5, 131072 B chunks, 400000 B | [x] | cfg_compressBegin_continue_end |
| 101 | compressBegin_advanced + continue/end | explicit params wlog 17 btlazy2 checksum, pledged | [x] | cfg_compressBegin_advanced |
| 102 | compressBegin_usingDict/_usingCDict(_advanced) + continue/end | 8192 B dict, level 9 | [x] | cfg_compressBegin_dict |
| 103 | ZSTD_copyCCtx | copy after compressBegin(level 9), continue on both | [x] | cfg_copyCCtx |
| 104 | ZSTD_compressBlock | srcSize == getBlockSize; +1 → srcSize_wrong | [x] | cfg_block_api, err_block_too_large |
| 105 | compressBlock/decompressBlock | 3 successive 131072 B text blocks | [x] | cfg_block_api |
| 106 | compressBlock/decompressBlock + insertBlock | 131072 B random (raw block) | [x] | cfg_block_api |
| 107 | compressBlock/decompressBlock | 131072 B all-zero (RLE, cSize==1) | [x] | cfg_block_api |
| 108 | compressBlock | 1/7/1000/1024 B blocks; maxBlockSize=1024 variant | [x] | cfg_block_api |
| 109 | decompressBegin+nextSrcSizeToDecompress+decompressContinue+nextInputType | 262144 B, contentSize+checksum; assert full sequences | [x] | cfg_decompressContinue_loop |
| 110 | same low-level loop | magicless frame via DCtx_setFormat, 65536 B | [x] | cfg_decompressContinue_magicless |
| 111 | same low-level loop | frame with raw block (random 131072 B) | [x] | cfg_decompressContinue_raw_rle |
| 112 | same low-level loop | frame with RLE block (all 0x5A, 131072 B) | [x] | cfg_decompressContinue_raw_rle |
| 113 | decompressBegin_usingDict/_usingDDict + continue | 8192 B trained dict, 65536 B | [x] | cfg_decompressBegin_dict |
| 114 | ZSTD_copyDCtx + decompressContinue | copy after header, continue both | [x] | cfg_copyDCtx |
| 115 | decompressBegin + insertBlock + checkContinuity | manual raw block insertion | [x] | cfg_block_api |
| 116 | getFrameHeader/_advanced/frameHeaderSize | all fcsId sizes, dictID sizes, single-segment, srcSize 1..18 | [x] | cfg_getFrameHeader_variants |
| 117 | findDecompressedSize/decompressBound/findFrameCompressedSize | 1 frame ±FCS, 3 frames, +skippable, truncated | [x] | cfg_frame_size_queries |
| 118 | getFrameContentSize/getDecompressedSize | 0-byte frame, unknown FCS, huge FCS, non-frame | [x] | cfg_frame_size_queries |
| 119 | ZSTD_decompressionMargin | single frame w/ checksum, multi-frame, in-place decode | [x] | cfg_decompression_margin |
| 120 | isFrame/isSkippableFrame | zstd1, all 16 skippable magics, legacy magics, magicless, random | [x] | cfg_isframe_isskippable |
| 121 | writeSkippableFrame/readSkippableFrame | variants 0,1,7,15 × payload 0,1,8,1000; variant 16 → outOfBound; small dst | [x] | cfg_skippable_roundtrip, err_skippable |
| 122 | ZSTD_decompress on concatenation | [skip][frame][skip][frame][skip] | [x] | cfg_decompress_concat_skippable |
| 123 | decompressStream on same concatenation | fed 1 byte at a time | [x] | cfg_stream_decompress_multiframe_and_skippable |
| 124 | ZSTD_writeLastEmptyBlock + compressBegin/Continue | hand-assembled frame w/ empty last block | [x] | cfg_write_last_empty_block |
| 125 | compress_usingDict/decompress_usingDict | raw dict 0,1,256,8192,112640 × levels 1,9,19 | [x] | cfg_compress_using_dict_rawcontent |
| 126 | createCDict/compress_usingCDict/createDDict | trained dict 8192 B, level 9 | [x] | cfg_cdict_ddict_levels_and_shapes |
| 127 | createCDict_byReference vs createCDict | same content, level 9 → identical output | [x] | cfg_refCDict_attach_prefs |
| 128 | createCDict_advanced | dlm × dct matrix on raw 8192 B (fullDict → corrupted) | [x] | cfg_cdict_advanced_matrix, err_dict_fullDict_on_raw |
| 129 | createCDict_advanced | dlm × dct matrix on trained dict | [x] | cfg_cdict_advanced_matrix |
| 130 | createCDict_advanced2 | via CCtx_params level 19 + explicit cParams | [x] | cfg_cdict_advanced_matrix |
| 131 | compress_usingCDict_advanced | explicit fParams {1,1,1} | [x] | cfg_cdict_advanced_matrix |
| 132 | CCtx_refCDict + compress2 | forceAttachDict=0, pledged 4096 vs 1000000, greedy | [x] | cfg_refCDict_attach_prefs, cfg_attach_cutoffs |
| 133 | CCtx_refCDict + compress2 | forceAttachDict=1, pledged 1000000 | [x] | cfg_attach_cutoffs |
| 134 | CCtx_refCDict + compress2 | forceAttachDict=2, pledged 4096 | [x] | cfg_attach_cutoffs |
| 135 | CCtx_refCDict + compress2 | forceAttachDict=3, pledged 4096 and unknown | [x] | cfg_attach_cutoffs |
| 136 | CCtx_refCDict + compress2 | forceMaxWindow=1 + default attach, pledged 4096 | [x] | cfg_attach_cutoffs |
| 137 | CCtx_refCDict + compress2 | fast/dfast/lazy2 with pledged just below/above cutoff | [x] | cfg_attach_cutoffs |
| 138 | CCtx_loadDictionary/_byReference/_advanced | all dlm × dct, 8192 B dict, level 9 | [x] | cfg_loadDictionary_variants |
| 139 | CCtx_refPrefix/DCtx_refPrefix | 8192 B prefix, level 6; prefix consumed after one frame | [x] | cfg_refPrefix_variants, cfg_prefix_single_use |
| 140 | refPrefix_advanced | dct_rawContent vs dct_fullDict on trained-dict buffer | [x] | cfg_refPrefix_variants |
| 141 | enableDedicatedDictSearch=1 + CDict | greedy hashLog 20 chainLog 16 vs chainLog 25 | [x] | cfg_dedicated_dict_search |
| 142 | ZSTD_dedicatedDictSearch_lazy_loadDictionary (direct) | greedy/lazy/lazy2 on 65536 B dict | [x] | cfg_dedicated_dict_search |
| 143 | DCtx_refDDict + d_refMultipleDDicts=1 | 2 DDicts distinct dictIDs; unknown dictID → dictionary_wrong | [x] | cfg_ref_multiple_ddicts, err_dictionary_wrong |
| 144 | getDictID_fromDict/_fromCDict/_fromDDict/_fromFrame | trained, raw, 3-byte, dict frame, plain frame | [x] | cfg_cdict_ddict_levels_and_shapes, cfg_dictID_flag_and_queries |
| 145 | createDDict_byReference/_advanced/DDict_dictContent/_dictSize/copyDDictParameters | trained dict, dlm × dct | [x] | cfg_ddict_advanced_matrix |
| 146 | sequenceBound + generateSequences | levels 1,5,9,19 × 1000/131072/300000 B; targetCBlockSize→unsupported | [x] | cfg_generate_sequences, err_generateSequences_targetCBlockSize |
| 147 | generateSequences+mergeBlockDelimiters+compressSequences | noBlockDelimiters, validateSequences=1, 300000 B | [x] | cfg_compress_sequences |
| 148 | generateSequences+compressSequences | explicitBlockDelimiters, validateSequences=0 | [x] | cfg_compress_sequences |
| 149 | compressSequences | repcodeResolution 0/1/2 at level 5 and 12 | [x] | cfg_compress_sequences |
| 150 | compressSequences | checksum+contentSize; invalid seq + validate → corruption_detected | [x] | cfg_compress_sequences, err_sequences_invalid |
| 151 | compressSequencesAndLiterals | explicit delims, litCapacity=litSize+8, 131072 B | [x] | cfg_compress_sequences_and_literals |
| 152 | compressSequencesAndLiterals (negative) | noDelims / validate=1 / checksum=1 / small litCapacity / incompressible | [x] | err_seq_and_literals |
| 153 | convertBlockSequences/getSeqStore/resetSeqStore/seqToCodes/encodeSequences/buildCTable/selectEncodingType/buildBlockEntropyStats | driven from generateSequences output, levels 3 and 19 | [x] | cfg_seqstore_internals |
| 154 | noCompressLiterals/compressRleLiteralsBlock/compressLiterals | literal buffers 0..131072 × random/RLE/text × flags | [x] | cfg_literals_functions |
| 155 | compressSuperBlock / splitBlock | levels 3,19 with targetCBlockSize=1340, 131072 B mixed; splitBlock level 0..4 | [x] | cfg_more_experimental_matrix, cfg_split_block_direct |
| 156 | compress/decompress size sweep | 0,1,7,1000,65536,131072,131073,262144,400000,1 MB at level 3 | [x] | cfg_all_sizes_boundaries |
| 157 | compress2 level 19 size sweep | same size set, text | [x] | cfg_all_sizes_boundaries, cfg_levels_x_shapes_simple_api |
| 158 | levels 3 and 19, size 131072 | all-zero, 1-byte repeat, 2-byte repeat, random, text, 90/10 mix | [x] | cfg_levels_x_shapes_simple_api |
| 159 | ZSTD_compress dst capacity | bound, exact, exact-1 → dstSize_tooSmall, 0 | [x] | err_dst_capacity_compress |
| 160 | ZSTD_decompress dst capacity | exact, +1, -1 → dstSize_tooSmall, 0 w/ empty payload | [x] | err_dst_capacity_decompress |
| 161 | compressBound/decompressBound/decodingBufferSize_min | 0,1,131072,1<<20,1<<30,SIZE_MAX/2 | [x] | cfg_level_query_functions, cfg_bound_functions |
| 162 | estimate* family | levels 1,3,9,19,22 × cParams × dictSize 0,8192,112640 | [x] | cfg_estimate_functions |
| 163 | initStaticCCtx + compressCCtx | workspace = estimate; size-1 → NULL | [x] | cfg_init_static, err_init_static_small |
| 164 | initStaticDCtx + decompressDCtx | workspace = estimate; refMultipleDDicts → unsupported | [x] | cfg_init_static, err_static_refMultipleDDicts |
| 165 | initStaticCStream/DStream/CDict/DDict | exact-estimate workspaces, 65536 B round trip | [x] | cfg_init_static |
| 166 | sizeof_CCtx/DCtx/CStream/DStream/CDict/DDict | after create, after compress, after reset | [x] | cfg_sizeof_functions |
| 167 | getFrameProgression/toFlushNow/get1BlockSummary | 3 points during a 400000 B stream at level 9 | [x] | cfg_frame_progression |
| 168 | ZSTD_CCtx_reset | session_only mid-frame; parameters mid-frame → stage_wrong; both | [x] | cfg_reset_and_pledged_sequences, err_reset_parameters_midframe |
| 169 | ZSTD_DCtx_reset | all 3 directives; setParameter mid-frame → stage_wrong | [x] | err_dctx_setparam_stage_wrong |
| 170 | ZBUFF_compressInit + continue/flush/end | level 3, in 1000, out recommendedCOutSize, 100000 B | [x] | cfg_zbuff_compress |
| 171 | ZBUFF_createCCtx_advanced + compressInit_advanced | explicit params checksum=1, pledged 65536, chunks 7 | [x] | cfg_zbuff_compress_advanced |
| 172 | ZBUFF_compressInitDictionary + continue/flush/end | 8192 B raw dict, level 6 | [x] | cfg_zbuff_dict |
| 173 | ZBUFF_createDCtx(_advanced)+decompressInit+decompressContinue | in 1 B and recommendedDInSize, out 1000 and recommendedDOutSize | [x] | cfg_zbuff_decompress |
| 174 | ZBUFF_decompressInitDictionary + continue | dict frame, out-chunk 7 | [x] | cfg_zbuff_dict |
| 175 | ZBUFF_recommended*Size / isError / getErrorName | codes 0,1,(size_t)-1,(size_t)-72 | [x] | cfg_zbuff_helpers |
| 176 | ZSTDv01_* battery | magic-only, magic+garbage, modern frame, 0/1/3-byte, all-zero 1024 B | [x] | legacy_v01 |
| 177 | ZSTDv02_* battery | same, magic 0xFD2FB522 | [x] | legacy_v02 |
| 178 | ZSTDv03_* battery | same, magic 0xFD2FB523 | [x] | legacy_v03 |
| 179 | ZSTDv04_* + ZBUFFv04_* battery | same, magic 0xFD2FB524 | [x] | legacy_v04 |
| 180 | ZSTDv05_* battery + getFrameParams/decompressBegin/Block/usingDict/copyDCtx/sizeofDCtx | magic 0xFD2FB525; ZSTD_decompress auto-dispatch | [x] | legacy_v05 |
| 181 | ZSTDv06_* + ZBUFFv06_* battery | magic 0xFD2FB526; ZSTD_isFrame | [x] | legacy_v06 |
| 182 | ZSTDv07_* + ZBUFFv07_* battery + DDict/insertBlock/isSkipFrame/estimateDCtxSize | magic 0xFD2FB527 | [x] | legacy_v07 |
| 183 | FSEv05/06/07_* | createDTable(tableLog 5,9,11,12), buildDTable_raw/rle, readNCount, decompress | [x] | legacy_entropy |
| 184 | HUFv05/06/07_* | readDTableX2/X4, decompress 1X/4X variants, readStats, selectDecoder | [x] | legacy_entropy |
| 185 | ZDICT_trainFromBuffer | 200 samples × 512 B text, capacity 4096/16384/112640 | [x] | dictb_train_basic |
| 186 | ZDICT_trainFromBuffer (edge) | 0 samples, 1 sample, total < 512 B, identical samples, random samples | [x] | dictb_train_edge |
| 187 | ZDICT_trainFromBuffer_cover | (k,d) = (50,6),(200,8),(1000,8),(2048,16); splitPoint 0,1.0; level 3 | [x] | dictb_cover |
| 188 | ZDICT_trainFromBuffer_cover | k=200,d=8, splitPoint 0.5/0.75; shrinkDict 1 × regression 0,1,5 | [x] | dictb_cover |
| 189 | ZDICT_trainFromBuffer_cover (invalid) | k=0; d=0; d>k; k>maxDictSize; splitPoint 0.0 / 1.5 | [x] | err_dictb_cover_params |
| 190 | ZDICT_optimizeTrainFromBuffer_cover | k=0,d=0, steps 1,4,8,0(→40); levels 3,9; returned params struct | [x] | dictb_cover_optimize |
| 191 | ZDICT_optimizeTrainFromBuffer_cover | d=6, k=0, steps=4, splitPoint 0.75, nbThreads 1 vs 4 | [x] | dictb_cover_optimize |
| 192 | ZDICT_trainFromBuffer_fastCover | k=200,d=8 × f ∈ {1,6,20,25,31}, accel=1 | [x] | dictb_fastcover |
| 193 | ZDICT_trainFromBuffer_fastCover | k=200,d=8,f=20 × accel ∈ {0,2,5,10} | [x] | dictb_fastcover |
| 194 | ZDICT_trainFromBuffer_fastCover (invalid) | f=32 → error; accel=11 → error; splitPoint ignored | [x] | err_dictb_fastcover_params |
| 195 | ZDICT_optimizeTrainFromBuffer_fastCover | k=0,d=0,f=0,accel=0 × steps {1,4,8,0}; splitPoint 0,0.5,1.0 | [x] | dictb_fastcover_optimize |
| 196 | ZDICT_trainFromBuffer_legacy | selectivity {0,1,5,9,12,31}; capacity 4096 and 112640 | [x] | dictb_legacy |
| 197 | ZDICT_finalizeDictionary | content 8192 B; maxDictSize {256,1024,8192,12288}; level {0,1,19}; dictID 1, 0x80000001 | [x] | dictb_finalize |
| 198 | ZDICT_finalizeDictionary (invalid) | maxDictSize=100, 0 samples, identical samples, random samples | [x] | err_dictb_finalize |
| 199 | ZDICT_addEntropyTablesFromBuffer | 8192 B content, capacity 16384, 200 samples | [x] | dictb_finalize |
| 200 | ZDICT_getDictID/getDictHeaderSize/isError/getErrorName | trained, raw, 3-byte, truncated-magic dict | [x] | dictb_queries |
| 201 | COVER_* / divsufsort / divbwt | via optimize entry points; COVER_computeEpochs/sum direct; divsufsort on 4096 B text and zeros | [x] | dictb_cover_internals |
| 202 | FSE_optimalTableLog(_internal)/NCountWriteBound/compressBound | maxTableLog × srcSize × maxSymbolValue × minus | [x] | fse_helpers |
| 203 | HIST_count/_simple/_wksp/countFast(_wksp)/add/isError | maxSymbolValue 1,15,255 × src shapes | [x] | hist_functions |
| 204 | FSE round trip (normalizeCount→writeNCount→buildCTable→compress→readNCount→buildDTable→decompress) | tableLog 5,6,9,11,12 × maxSV 1,15,100,255 × text/random/skewed | [x] | fse_roundtrip |
| 205 | FSE degenerate | buildCTable_rle, useLowProbCount 0/1, tableLog 13 → error, readNCount_bmi2 | [x] | fse_roundtrip, err_fse |
| 206 | HUF 4-stream round trip | optimalTableLog→buildCTable_wksp→writeCTable_wksp→compress4X_usingCTable→readDTableX1/X2→decompress4X | [x] | huf_roundtrip |
| 207 | HUF 1-stream round trip | compress1X_usingCTable→decompress1X_usingDTable/1X1/1X2/1X_DCtx_wksp | [x] | huf_roundtrip |
| 208 | HUF repeat modes | compress4X_repeat/compress1X_repeat with repeat none/check/valid over 3 blocks | [x] | huf_repeat |
| 209 | HUF flags sweep | flags 0,1,2,4,8,16,32,48,63 on the 4X pipeline | [x] | huf_roundtrip |
| 210 | HUF helpers | minTableLog/cardinality/validateCTable/estimateCompressedSize/getNbBits/readCTable(_Header)/selectDecoder/readStats(_wksp)/compressBound/isError | [x] | huf_helpers |
| 211 | ZSTD_XXH32 one-shot | 17 lengths × seeds {0,1,0x9E3779B1,0xFFFFFFFF} | [x] | xxh32 |
| 212 | ZSTD_XXH64 one-shot | 17 lengths × seeds {0,1,0x27D4EB2F165667C5,u64::MAX} | [x] | xxh64 |
| 213 | ZSTD_XXH32 streaming | chunk splits 1,3,7,16,1000 over 4096 B; copyState; canonical | [x] | xxh32_stream |
| 214 | ZSTD_XXH64 streaming | chunk splits over 4096 and 131072 B; copyState; update(NULL,0) | [x] | xxh64_stream |
| 215 | XXH canonicalFromHash/hashFromCanonical/versionNumber | hashes 0,1,MAX and digests from 211/212 | [x] | xxh_canonical |
| 216 | POOL_create/POOL_free | numThreads {0,1,2,4,16} × queueSize {0,1,4}; free(NULL) | [x] | pool_basic |
| 217 | POOL_create_advanced | default and custom customMem; mismatched alloc/free → NULL | [x] | pool_advanced |
| 218 | POOL_add/tryAdd/joinJobs/resize/sizeof | 0,1,5 jobs; resize 0/1/4 | [x] | pool_jobs |
| 219 | getErrorName/getErrorCode/getErrorString/isError/ERR_getErrorString | every ZSTD_ErrorCode + 0,1,(size_t)-1,(size_t)-200 | [x] | err_error_api |
| 220 | FSE_versionNumber/isError/getErrorName, ZSTD_versionNumber/String, g_debuglevel | plain reads | [x] | smoke_version_and_roundtrip, fse_helpers |
| 221 | malformed decode inputs (all decode entry points) | truncations, bit flips, bt_reserved, oversized block, FCS mismatch, windowLog 32, bogus dictID | [x] | err_malformed_frames |
| 222 | compressBegin/Continue + decompressBegin/Continue, windowLog=10 | 4096 B over 3 continue calls (extDict / checkContinuity) | [x] | cfg_extdict_noncontiguous |

---

## Verification summary (Phase D)

| gate | result |
|---|---|
| `nm -D` defined-symbol parity | C 615 / Rust 615, **0 missing, 0 extra** (see `SYMBOLS.md`) |
| undefined non-libc symbols in the Rust `.so` | **0** |
| rows in this table with a passing differential test | **222 / 222** |
| rows in `ERRORS.md` with a passing differential test | **344 / 344** (7 annotated as unreachable through the exported API; nearest reachable boundary compared instead, 1 needs a 4 GB buffer) |
| test functions | **209**, all passing (`tests/`, run with `cargo test --offline --release`) |
| binary/driver executable | none — `c_src/CMakeLists.txt` has no `add_executable` and the crate has no `[[bin]]`, so there is no stdout comparison to make |
| feature combinations | the crate declares **no** `[features]` (single configuration); `cargo check --release --no-default-features` and the full test run were both exercised |

### Divergences found and fixed in the Rust port

| # | file / function | divergence |
|---|-----------------|------------|
| 1 | `src/bits.rs` — `ZSTD_highbit32` | `val == 0` returned `0xFFFFFFFF` (`31 - 32` wrapping); the reference C build observably returns `0` (`bsr` leaves its destination untouched on a zero source). Reached from the public API through `FSE_optimalTableLog*`, `FSE_normalizeCount`, `HUF_minTableLog` with `maxSymbolValue == 0` / `srcSize <= 1`. |
| 2 | `src/compress/fse_compress.rs` — `FSE_minTableLog` | did not reproduce the C `clz == 31` result for `maxSymbolValue == 0`. |
| 3 | `src/compress/fse_compress.rs` — `FSE_optimalTableLog_internal`, `FSE_normalizeCount` | two bespoke UB emulations copied from an *inlined* build produced wrong results for this reference build (e.g. `FSE_optimalTableLog_internal(9, 4096, 0, 0)`: C `9`, Rust `12`; `FSE_normalizeCount(tableLog=9, total=4096, maxSymbolValue=0)`: C `9`, Rust `-GENERIC`). Both now defer to `FSE_minTableLog`. |

Everything else in the 615-symbol surface matched the C library byte-for-byte and
error-code-for-error-code on the first run, including all nine compression strategies,
every `ZSTD_c_*`/`ZSTD_d_*` parameter, streaming, dictionaries, the block and
`decompressContinue` low-level APIs, the sequence APIs, the dictBuilder (cover /
fastcover / legacy / finalize), ZBUFF, the v0.1–v0.7 legacy decoders, FSE/HUF/HIST,
XXH32/64 and POOL.
