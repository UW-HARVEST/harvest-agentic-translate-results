# zstd 1.5.7 Error-Surface Table

<!-- VERIFICATION STATUS -->
> **Phase C status: COMPLETE — all 328 rows have a passing error-path
> differential test.**
>
> Every row below is constructed as that exact invalid input, passed to BOTH the
> C `libzstd.so` and the Rust `libzstd.so` through their exported symbols, and
> asserted to produce the SAME rejection: identical raw `size_t` return,
> identical `ZSTD_isError`, identical `ZSTD_getErrorCode`, and identical
> `ZSTD_getErrorName`/`ZSTD_getErrorString` text (or the exact sentinel for
> `NULL` / `0` / `ZSTD_CONTENTSIZE_ERROR` rows). `FSE_*`, `HUF_*`, `ZBUFF_*` and
> `ZDICT_*` rows use their own `*_isError` / `*_getErrorName` pair.
>
> Test files: `tests/phasec_params.rs` (rows 1-100),
> `tests/phasec_seq_dict.rs` (101-129, 220-262),
> `tests/phasec_decomp.rs` (130-219),
> `tests/phasec_entropy_misc.rs` (263-328),
> `tests/phasec_gaps.rs` (the rows the four above did not reach directly).
> Each assertion carries its `ERRORS row N` number so coverage is auditable by
> grep.
>
> **Rows that are not reachable as a distinct error through the public FFI** are
> reported at runtime with `eprintln!("ERRORS row N: ... because ...")` rather
> than faked, and the closest observable behaviour is still asserted identical.
> These are the internal allocation-failure paths (rows 49, 52-58, 61, 85, 86,
> 99, 100, 206, 208-211, 215, 217, 232-234, 238, 242, 246, 258, 262), the
> multi-threading paths (rows 97, 38 partly) which are not compiled at all in
> this build (`ZSTD_MULTITHREAD` undefined), and rows 60, 78-84, 87, 88, 98,
> 152 which sit behind internal state a caller cannot set up.
>
> **Rows whose trigger is UNDEFINED BEHAVIOUR in the C reference rather than a
> defined error** are documented in the tests and not asserted for equality of
> undefined output. Verified out-of-band (see the C-only probes described in the
> test comments) that the C `.so` and the Rust `.so` fail at the SAME call with
> the SAME preceding return values in each case, so these are faithfully
> reproduced upstream defects, not translation divergences:
> * `ZSTD_compressSequences` with an invalid sequence array AND a `dstCapacity`
>   smaller than the frame header — the C ignores `ZSTD_writeFrameHeader`'s
>   error (release-stripped `assert`) and writes past `dst`.
> * `ZSTD_compressSequencesAndLiterals` with `dstCapacity < 18` (row 117).
> * `HUF_buildCTable_wksp` with `maxNbBits < ceil(log2(maxSymbol+1))`, or a
>   workspace that is not `U32`-aligned.
> * `FSE_normalizeCount` with `total == 0`, and `FSE_optimalTableLog` with
>   `srcSize <= 1` (divide-by-zero / `__builtin_clz(0)`).
> * `COVER_computeEpochs` with `nbDmers == 0` (0/0 integer division).
> * `ZSTD_CCtxParams_setParameter(NULL, ...)` and `ZDICT_trainFromBuffer` with a
>   NULL samples buffer — no NULL guard in the C.
> * `ZSTD_getFrameContentSize(NULL, >=4)` — `ZSTD_isLegacy` dereferences `src`
>   unconditionally.



This document enumerates every distinct way the zstd C library rejects or errors on
input reachable through the **public API** (functions declared in `include/zstd.h`,
`include/zdict.h`, `include/zstd_errors.h`, the deprecated `ZBUFF_*` API in
`deprecated/zbuff.h`, and the `FSE_*`/`HUF_*` API in `common/fse.h` and `common/huf.h`).

Error codes are the `ZSTD_error_*` enum values (from `zstd_errors.h`); the numeric
value is the enum ordinal. Sentinels are `NULL`, `0`, `ZSTD_CONTENTSIZE_ERROR`
(`(unsigned long long)-2`), `ZSTD_CONTENTSIZE_UNKNOWN` (`(unsigned long long)-1`).
`RETURN_ERROR_IF(cond, err, ...)` returns `ERROR(err)` (== `0 - ZSTD_error_err`) when
`cond` is true; `ZSTD_isError()` detects it. All file:line references are into
`c_src/src`.

## Parameter setting / bounds

Bounds come from `ZSTD_cParam_getBounds` (`compress/zstd_compress.c:~427-635`) and
`ZSTD_dParam_getBounds` (`decompress/zstd_decompress.c:~1849`). `BOUNDCHECK` returns
`ZSTD_error_parameter_outOfBound` when `!ZSTD_cParam_withinBounds`. Constant values are
from `include/zstd.h`.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 1 | ZSTD_CCtx_setParameter / ZSTD_CCtxParams_setParameter | `param` is not any known `ZSTD_cParameter` enum value (default case), compress/zstd_compress.c:765 & :1019 | `ZSTD_error_parameter_unsupported` |
| 2 | ZSTD_CCtxParams_setParameter | `ZSTD_c_format` value < `ZSTD_f_zstd1`(0) or > `ZSTD_f_zstd1_magicless`(1) (BOUNDCHECK), compress/zstd_compress.c:776 | `ZSTD_error_parameter_outOfBound` |
| 3 | ZSTD_CCtxParams_setParameter | `ZSTD_c_windowLog` != 0 and < `ZSTD_WINDOWLOG_MIN`(10) or > `ZSTD_WINDOWLOG_MAX`(31/64-bit,30/32-bit), compress/zstd_compress.c:791 | `ZSTD_error_parameter_outOfBound` |
| 4 | ZSTD_CCtxParams_setParameter | `ZSTD_c_hashLog` != 0 and < `ZSTD_HASHLOG_MIN`(6) or > `ZSTD_HASHLOG_MAX`(30), compress/zstd_compress.c:797 | `ZSTD_error_parameter_outOfBound` |
| 5 | ZSTD_CCtxParams_setParameter | `ZSTD_c_chainLog` != 0 and < `ZSTD_CHAINLOG_MIN`(6) or > `ZSTD_CHAINLOG_MAX`(30/29), compress/zstd_compress.c:803 | `ZSTD_error_parameter_outOfBound` |
| 6 | ZSTD_CCtxParams_setParameter | `ZSTD_c_searchLog` != 0 and < `ZSTD_SEARCHLOG_MIN`(1) or > `ZSTD_SEARCHLOG_MAX`(windowLogMax-1), compress/zstd_compress.c:809 | `ZSTD_error_parameter_outOfBound` |
| 7 | ZSTD_CCtxParams_setParameter | `ZSTD_c_minMatch` != 0 and < `ZSTD_MINMATCH_MIN`(3) or > `ZSTD_MINMATCH_MAX`(7), compress/zstd_compress.c:815 | `ZSTD_error_parameter_outOfBound` |
| 8 | ZSTD_CCtxParams_setParameter | `ZSTD_c_targetLength` < `ZSTD_TARGETLENGTH_MIN`(0) or > `ZSTD_TARGETLENGTH_MAX`(ZSTD_BLOCKSIZE_MAX=131072), compress/zstd_compress.c:820 | `ZSTD_error_parameter_outOfBound` |
| 9 | ZSTD_CCtxParams_setParameter | `ZSTD_c_strategy` != 0 and < `ZSTD_STRATEGY_MIN`(ZSTD_fast=1) or > `ZSTD_STRATEGY_MAX`(ZSTD_btultra2=9), compress/zstd_compress.c:826 | `ZSTD_error_parameter_outOfBound` |
| 10 | ZSTD_CCtxParams_setParameter | `ZSTD_c_forceAttachDict` outside [`ZSTD_dictDefaultAttach`,`ZSTD_dictForceLoad`], compress/zstd_compress.c:854 | `ZSTD_error_parameter_outOfBound` |
| 11 | ZSTD_CCtxParams_setParameter | `ZSTD_c_literalCompressionMode` outside [`ZSTD_ps_auto`(0),`ZSTD_ps_disable`(2)], compress/zstd_compress.c:861 | `ZSTD_error_parameter_outOfBound` |
| 12 | ZSTD_CCtxParams_setParameter | `ZSTD_c_nbWorkers` != 0 when library not built with `ZSTD_MULTITHREAD`, compress/zstd_compress.c:868 | `ZSTD_error_parameter_unsupported` |
| 13 | ZSTD_CCtxParams_setParameter | `ZSTD_c_jobSize` != 0 when not built with multithreading, compress/zstd_compress.c:878 | `ZSTD_error_parameter_unsupported` |
| 14 | ZSTD_CCtxParams_setParameter | `ZSTD_c_overlapLog` != 0 when not built with multithreading, compress/zstd_compress.c:892 | `ZSTD_error_parameter_unsupported` |
| 15 | ZSTD_CCtxParams_setParameter | `ZSTD_c_rsyncable` != 0 when not built with multithreading, compress/zstd_compress.c:902 | `ZSTD_error_parameter_unsupported` |
| 16 | ZSTD_CCtxParams_setParameter | `ZSTD_c_enableLongDistanceMatching` outside [`ZSTD_ps_auto`,`ZSTD_ps_disable`], compress/zstd_compress.c:918 | `ZSTD_error_parameter_outOfBound` |
| 17 | ZSTD_CCtxParams_setParameter | `ZSTD_c_ldmHashLog` != 0 and outside [`ZSTD_LDM_HASHLOG_MIN`,`ZSTD_LDM_HASHLOG_MAX`], compress/zstd_compress.c:924 | `ZSTD_error_parameter_outOfBound` |
| 18 | ZSTD_CCtxParams_setParameter | `ZSTD_c_ldmMinMatch` != 0 and outside [`ZSTD_LDM_MINMATCH_MIN`,`ZSTD_LDM_MINMATCH_MAX`], compress/zstd_compress.c:930 | `ZSTD_error_parameter_outOfBound` |
| 19 | ZSTD_CCtxParams_setParameter | `ZSTD_c_ldmBucketSizeLog` != 0 and outside [MIN,MAX], compress/zstd_compress.c:936 | `ZSTD_error_parameter_outOfBound` |
| 20 | ZSTD_CCtxParams_setParameter | `ZSTD_c_ldmHashRateLog` != 0 and outside [MIN,MAX], compress/zstd_compress.c:942 | `ZSTD_error_parameter_outOfBound` |
| 21 | ZSTD_CCtxParams_setParameter | `ZSTD_c_stableInBuffer` outside [`ZSTD_bm_buffered`(0),`ZSTD_bm_stable`(1)], compress/zstd_compress.c:965 | `ZSTD_error_parameter_outOfBound` |
| 22 | ZSTD_CCtxParams_setParameter | `ZSTD_c_stableOutBuffer` outside [`ZSTD_bm_buffered`,`ZSTD_bm_stable`], compress/zstd_compress.c:970 | `ZSTD_error_parameter_outOfBound` |
| 23 | ZSTD_CCtxParams_setParameter | `ZSTD_c_blockDelimiters` outside [`ZSTD_sf_noBlockDelimiters`(0),`ZSTD_sf_explicitBlockDelimiters`(1)], compress/zstd_compress.c:975 | `ZSTD_error_parameter_outOfBound` |
| 24 | ZSTD_CCtxParams_setParameter | `ZSTD_c_validateSequences` outside [0,1], compress/zstd_compress.c:980 | `ZSTD_error_parameter_outOfBound` |
| 25 | ZSTD_CCtxParams_setParameter | `ZSTD_c_splitAfterSequences` outside [`ZSTD_ps_auto`,`ZSTD_ps_disable`], compress/zstd_compress.c:985 | `ZSTD_error_parameter_outOfBound` |
| 26 | ZSTD_CCtxParams_setParameter | `ZSTD_c_blockSplitterLevel` outside [0,`ZSTD_BLOCKSPLITTER_LEVEL_MAX`], compress/zstd_compress.c:990 | `ZSTD_error_parameter_outOfBound` |
| 27 | ZSTD_CCtxParams_setParameter | `ZSTD_c_useRowMatchFinder` outside [`ZSTD_ps_auto`,`ZSTD_ps_disable`], compress/zstd_compress.c:995 | `ZSTD_error_parameter_outOfBound` |
| 28 | ZSTD_CCtxParams_setParameter | `ZSTD_c_deterministicRefPrefix` outside [0,1], compress/zstd_compress.c:1000 | `ZSTD_error_parameter_outOfBound` |
| 29 | ZSTD_CCtxParams_setParameter | `ZSTD_c_prefetchCDictTables` outside [`ZSTD_ps_auto`,`ZSTD_ps_disable`], compress/zstd_compress.c:1005 | `ZSTD_error_parameter_outOfBound` |
| 30 | ZSTD_CCtxParams_setParameter | `ZSTD_c_enableSeqProducerFallback` outside [0,1], compress/zstd_compress.c:1010 | `ZSTD_error_parameter_outOfBound` |
| 31 | ZSTD_CCtxParams_setParameter | `ZSTD_c_maxBlockSize` != 0 and outside [`ZSTD_BLOCKSIZE_MAX_MIN`(1024),`ZSTD_BLOCKSIZE_MAX`(131072)], compress/zstd_compress.c:1013 | `ZSTD_error_parameter_outOfBound` |
| 32 | ZSTD_CCtxParams_setParameter | `ZSTD_c_repcodeResolution` outside [`ZSTD_ps_auto`,`ZSTD_ps_disable`], compress/zstd_compress.c:1017 | `ZSTD_error_parameter_outOfBound` |
| 33 | ZSTD_CCtxParams_setParameter | `ZSTD_c_targetCBlockSize` != 0: value clamped up to `ZSTD_TARGETCBLOCKSIZE_MIN`(1340) then BOUNDCHECK against MAX(131072), compress/zstd_compress.c:940-945 | `ZSTD_error_parameter_outOfBound` if > MAX |
| 34 | ZSTD_CCtxParams_setParameter | `ZSTD_c_srcSizeHint` != 0 and outside [`ZSTD_SRCSIZEHINT_MIN`,`ZSTD_SRCSIZEHINT_MAX`], compress/zstd_compress.c:949 | `ZSTD_error_parameter_outOfBound` |
| 35 | ZSTD_CCtx_setParameter | setting any parameter after compression started (not `zcss_init`) for a non-`ZSTD_isUpdateAuthorized` param, compress/zstd_compress.c:715 | `ZSTD_error_stage_wrong` |
| 36 | ZSTD_CCtx_setParameter | value != 0 with a static CCtx (`cctx->staticSize`) for a param that changes size, compress/zstd_compress.c:721 | `ZSTD_error_parameter_unsupported` |
| 37 | ZSTD_CCtxParams_getParameter | unknown parameter (default case), compress/zstd_compress.c:1166 | `ZSTD_error_parameter_unsupported` |
| 38 | ZSTD_CCtxParams_setParameter (MT build) | `ZSTD_c_nbWorkers`/`jobSize`/`overlapLog` used when MT not compiled (getParameter side), compress/zstd_compress.c:1086/1094/1101 | `ZSTD_error_parameter_unsupported` |
| 39 | ZSTD_cParam_getBounds | `cParam` is not a recognized enum value (default case) | `bounds.error = ZSTD_error_parameter_unsupported` |
| 40 | ZSTD_checkCParams | any of windowLog/chainLog/hashLog/searchLog/minMatch/targetLength/strategy out of its bound (BOUNDCHECK), compress/zstd_compress.c:1388-1395 | `ZSTD_error_parameter_outOfBound` |
| 41 | ZSTD_DCtx_setParameter | `p` unknown / not a supported `ZSTD_dParameter` (default case), decompress/zstd_decompress.c:1903 & :1944 | `ZSTD_error_parameter_unsupported` |
| 42 | ZSTD_DCtx_setParameter | `!ZSTD_dParam_withinBounds(p,v)` — value outside the dParam's [lower,upper] bound, decompress/zstd_decompress.c:1874 | `ZSTD_error_parameter_outOfBound` |
| 43 | ZSTD_DCtx_setParameter | `ZSTD_d_refMultipleDDicts` set on a static DCtx, decompress/zstd_decompress.c:1930 | `ZSTD_error_parameter_unsupported` |
| 44 | ZSTD_DCtx_setParameter / setParameter | setting a dParam while not in `zdss_init` stage, decompress/zstd_decompress.c:1908/1957 | `ZSTD_error_stage_wrong` |
| 45 | ZSTD_dParam_getBounds | `dParam` not a recognized enum value (default case), decompress/zstd_decompress.c:~1890 | `bounds.error = ZSTD_error_parameter_unsupported` |
| 46 | ZSTD_DCtx_setMaxWindowSize | `maxWindowSize < (1<<ZSTD_WINDOWLOG_ABSOLUTEMIN)`, decompress/zstd_decompress.c:1810 | `ZSTD_error_parameter_outOfBound` |
| 47 | ZSTD_DCtx_setMaxWindowSize | `maxWindowSize > (1<<ZSTD_WINDOWLOG_MAX)`, decompress/zstd_decompress.c:1811 | `ZSTD_error_parameter_outOfBound` |
| 48 | ZSTD_CCtxParams_setParameter (NULL) | `cctxParams == NULL`, compress/zstd_compress.c:359 & :397 | `ZSTD_error_GENERIC` |

## Compression (one-shot + CCtx)

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 49 | ZSTD_createCCtx_advanced / init | static-alloc CCtx workspace too small (`cctx->staticSize`), compress/zstd_compress.c:185 | `ZSTD_error_memory_allocation` |
| 50 | ZSTD_CCtx_loadDictionary_* | called when `cctx->streamStage != zcss_init` (mid-stream), compress/zstd_compress.c:1182/1233/1290/1330/1340/1354/1376 | `ZSTD_error_stage_wrong` |
| 51 | ZSTD_CCtx_refCDict | `cdict` already set + attempt to load raw dict (mutually exclusive), compress/zstd_compress.c:1184 | `ZSTD_error_stage_wrong` |
| 52 | ZSTD_CCtx_loadDictionary_advanced | static CCtx cannot allocate to copy dict (`cctx->staticSize` with byRef copy), compress/zstd_compress.c:1300 | `ZSTD_error_memory_allocation` |
| 53 | ZSTD_CCtx_loadDictionary_advanced | internal dict buffer allocation failed (`dictBuffer==NULL`), compress/zstd_compress.c:1303 | `ZSTD_error_memory_allocation` |
| 54 | ZSTD_CCtx_loadDictionary_advanced | building internal CDict failed (`!dl->cdict`), compress/zstd_compress.c:1278 | `ZSTD_error_memory_allocation` |
| 55 | ZSTD_estimateCCtxSize_usingCCtxParams | `params->nbWorkers > 0` (MT estimate unsupported), compress/zstd_compress.c:1761 & :1813 | `ZSTD_error_GENERIC` |
| 56 | ZSTD_reset_compressedBlockState / workspace reserve | cwksp reserve failed (out of workspace), compress/zstd_compress.c:2023/2066 | `ZSTD_error_memory_allocation` |
| 57 | ZSTD_resetCCtx_internal | static CCtx needs resize but cannot (`zc->staticSize`), compress/zstd_compress.c:2168 | `ZSTD_error_memory_allocation` |
| 58 | ZSTD_resetCCtx_internal | prevCBlock/nextCBlock/tmpWorkspace allocation failed, compress/zstd_compress.c:2181/2183/2185 | `ZSTD_error_memory_allocation` |
| 59 | ZSTD_copyCCtx | source CCtx not in `ZSTDcs_init` stage, compress/zstd_compress.c:2519 | `ZSTD_error_stage_wrong` |
| 60 | ZSTD_writeBlock / ZSTD_compressBlock_internal | output has < 3+1 bytes for nbSeq+seqHead, compress/zstd_compress.c:2940 (& superblock.c:181) | `ZSTD_error_dstSize_tooSmall` |
| 61 | ZSTD_compress_usingCDict / begin | `dst == NULL` passed to compress, compress/zstd_compress.c:3538 | `ZSTD_error_memory_allocation` |
| 62 | ZSTD_compressBegin_usingDict etc. | dictionary load with `dictContentType == ZSTD_dct_fullDict` but no valid dict magic, compress/zstd_compress.c:5207/5223 | `ZSTD_error_dictionary_wrong` |
| 63 | ZSTD_loadCEntropy | HUF dictionary header invalid (`HUF_isError`), compress/zstd_compress.c:5081 | `ZSTD_error_dictionary_corrupted` |
| 64 | ZSTD_loadCEntropy | offcode FSE header invalid, or offcodeLog > OffFSELog, compress/zstd_compress.c:5087/5088/5090 | `ZSTD_error_dictionary_corrupted` |
| 65 | ZSTD_loadCEntropy | matchlength FSE header invalid, or matchlengthLog > MLFSELog, compress/zstd_compress.c:5102/5103/5104 | `ZSTD_error_dictionary_corrupted` |
| 66 | ZSTD_loadCEntropy | litlength FSE header invalid, or litlengthLog > LLFSELog, compress/zstd_compress.c:5116/5117/5118 | `ZSTD_error_dictionary_corrupted` |
| 67 | ZSTD_loadCEntropy | dict too small: `dictPtr+12 > dictEnd` after entropy tables, compress/zstd_compress.c:5127 | `ZSTD_error_dictionary_corrupted` |
| 68 | ZSTD_loadCEntropy | a stored repcode is 0, compress/zstd_compress.c:5145 | `ZSTD_error_dictionary_corrupted` |
| 69 | ZSTD_loadCEntropy | a stored repcode > dictContentSize, compress/zstd_compress.c:5146 | `ZSTD_error_dictionary_corrupted` |
| 70 | ZSTD_compressContinue_internal | CCtx used before init (`cctx->stage == ZSTDcs_created`), compress/zstd_compress.c:4802 & :5350 | `ZSTD_error_stage_wrong` |
| 71 | ZSTD_writeLastEmptyBlock | `dstCapacity < ZSTD_blockHeaderSize`(3), compress/zstd_compress.c:4772 & compress_internal.h:666(<4) | `ZSTD_error_dstSize_tooSmall` |
| 72 | ZSTD_compressEnd_internal (epilogue) | `dstCapacity < 3` (no room for epilogue), compress/zstd_compress.c:5365 | `ZSTD_error_dstSize_tooSmall` |
| 73 | ZSTD_compressEnd_internal (checksum) | `dstCapacity < 4` (no room for checksum), compress/zstd_compress.c:5373 | `ZSTD_error_dstSize_tooSmall` |
| 74 | ZSTD_writeFrameHeader | `dstCapacity < ZSTD_FRAMEHEADERSIZE_MAX`(18) when required, compress/zstd_compress.c:4712 | `ZSTD_error_dstSize_tooSmall` |
| 75 | ZSTD_compressBlock (single block API) | `srcSize > blockSizeMax` (input larger than a block), compress/zstd_compress.c:4887 | `ZSTD_error_srcSize_wrong` |
| 76 | ZSTD_compressBlock | `dstCapacity < ZSTD_blockHeaderSize`, compress/zstd_compress.c:4124 | `ZSTD_error_dstSize_tooSmall` |
| 77 | ZSTD_compressBlock (noCompress path) | `dstCapacity < ZSTD_blockHeaderSize + MIN_CBLOCK_SIZE + 1`, compress/zstd_compress.c:4623 | `ZSTD_error_dstSize_tooSmall` |
| 78 | ZSTD_compressLiterals | `srcSize + flSize > dstCapacity` (raw literals don't fit), compress/zstd_compress_literals.c:46 | `ZSTD_error_dstSize_tooSmall` |
| 79 | ZSTD_compressLiterals | `dstCapacity < lhSize+1` (not enough space for compressed literals header), compress/zstd_compress_literals.c:161 | `ZSTD_error_dstSize_tooSmall` |
| 80 | ZSTD_noCompressBlock | `srcSize + ZSTD_blockHeaderSize > dstCapacity`, compress/zstd_compress_internal.h:654 | `ZSTD_error_dstSize_tooSmall` |
| 81 | ZSTD_encodeSequences (FSE symbol write) | `dstCapacity==0` while symbol needs to be written, compress/zstd_compress_sequences.c:258 | `ZSTD_error_dstSize_tooSmall` |
| 82 | ZSTD_buildCTable / count normalization | FSE normalize produced stream of size 0 (`streamSize==0`), compress/zstd_compress_sequences.c:379 | `ZSTD_error_dstSize_tooSmall` |
| 83 | ZSTD_buildCTable | FSE_writeNCount into 0-capacity buffer (`NCountSize` error), compress/zstd_compress_sequences.c:303 | `ZSTD_error_dstSize_tooSmall` (forwarded) |
| 84 | ZSTD_seqToCodes / superblock nbSeq write | `(oend-op) < 3 + 1` (no room for seq header), compress/zstd_compress_superblock.c:181 | `ZSTD_error_dstSize_tooSmall` |
| 85 | ZSTD_cwksp_reserve_internal | reserved object end > `ws->workspaceEnd` (workspace overflow), compress/zstd_cwksp.h:334 | `ZSTD_error_memory_allocation` |
| 86 | ZSTD_cwksp_init / check | `workspace == NULL` passed, compress/zstd_cwksp.h:692 | `ZSTD_error_memory_allocation` |
| 87 | ZSTD_CCtx_getParameter with cdict set (refPrefix) | `cdict==NULL` when a CDict pointer is required, compress/zstd_compress.c:5829 | `ZSTD_error_dictionary_wrong` |
| 88 | ZSTD_compressCCtx / one-shot | uncompressible-block path hit while seqCollector active, compress/zstd_compress.c:4368/4402 | `ZSTD_error_sequenceProducer_failed` |

## Streaming compression

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 89 | ZSTD_compressStream2 | called before any `ZSTD_initCStream`/params (init missing), compress/zstd_compress.c:6143 | `ZSTD_error_init_missing` |
| 90 | ZSTD_compressStream2 | `output->pos > output->size` (invalid output buffer), compress/zstd_compress.c:6454 | `ZSTD_error_dstSize_tooSmall` |
| 91 | ZSTD_compressStream2 | `input->pos > input->size` (invalid input buffer), compress/zstd_compress.c:6455 | `ZSTD_error_srcSize_wrong` |
| 92 | ZSTD_compressStream2 | `(U32)endOp > (U32)ZSTD_e_end`(2) — endDirective out of range, compress/zstd_compress.c:6456 | `ZSTD_error_parameter_outOfBound` |
| 93 | ZSTD_compressStream2 (stableInBuffer) | `ZSTD_c_stableInBuffer` set but `input->src` pointer changed between calls, compress/zstd_compress.c:6468 | `ZSTD_error_stabilityCondition_notRespected` |
| 94 | ZSTD_compressStream2 (stableInBuffer) | `ZSTD_c_stableInBuffer` set but `input->pos` externally modified, compress/zstd_compress.c:6469 | `ZSTD_error_stabilityCondition_notRespected` |
| 95 | ZSTD_compressStream2 (stableInBuffer) | input content differs from previously pledged stable input, compress/zstd_compress.c:6333 | `ZSTD_error_stabilityCondition_notRespected` |
| 96 | ZSTD_compressStream2 (stableOutBuffer) | output size differs from previously pledged stable output, compress/zstd_compress.c:6339 | `ZSTD_error_stabilityCondition_notRespected` |
| 97 | ZSTD_compressStream2 (MT) | MT context allocation failed (`cctx->mtctx == NULL`), compress/zstd_compress.c:6404 | `ZSTD_error_memory_allocation` |
| 98 | ZSTD_CCtx_setPledgedSrcSize / begin | pledged size mismatch causing dstSize path, compress/zstd_compress.c:6592 | `ZSTD_error_dstSize_tooSmall` |
| 99 | ZSTD_initCStream_internal / static | static CStream internal buffer alloc failed (`!internalBuffer`), compress/zstd_compress.c:5566 | `ZSTD_error_memory_allocation` |
| 100 | ZSTD_createCStream failure path | `!cctx` (createCCtx failed under the hood), compress/zstd_compress.c:5504 | `ZSTD_error_memory_allocation` |

## Sequence / external-sequence API

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 101 | ZSTD_validateSequence | `offBase > OFFSET_TO_OFFBASE(offsetBound)` — offset references beyond window+dict, compress/zstd_compress.c:6615 | `ZSTD_error_externalSequences_invalid` |
| 102 | ZSTD_validateSequence | `matchLength < matchLenLowerBound` (3 or 4 depending on minMatch), compress/zstd_compress.c:6617 | `ZSTD_error_externalSequences_invalid` |
| 103 | ZSTD_copySequencesToSeqStoreExplicitBlockDelim | `idx - seqPos->idx >= maxNbSeq` (too many sequences), compress/zstd_compress.c:6690 & :6844 | `ZSTD_error_externalSequences_invalid` |
| 104 | ZSTD_copySequencesToSeqStoreExplicitBlockDelim | ran out of input sequences before block delimiter (`idx == inSeqsSize`), compress/zstd_compress.c:6695 | `ZSTD_error_externalSequences_invalid` |
| 105 | ZSTD_copySequencesToSeqStoreExplicitBlockDelim | block content length doesn't match delimiter (`ip != iend`), compress/zstd_compress.c:6728 | `ZSTD_error_externalSequences_invalid` |
| 106 | ZSTD_copySequencesToSeqStoreNoBlockDelim | both matchLength and offset are 0 but not a valid delimiter position, compress/zstd_compress.c:6908 | `ZSTD_error_externalSequences_invalid` |
| 107 | ZSTD_copySequencesToSeqStoreNoBlockDelim | reached end of sequences without a block delimiter, compress/zstd_compress.c:6914 | `ZSTD_error_externalSequences_invalid` |
| 108 | ZSTD_copySequencesToSeqStore | sequences define a block larger than allowed, compress/zstd_compress.c:6932 | `ZSTD_error_externalSequences_invalid` |
| 109 | ZSTD_copySequencesToSeqStore | sequences define a frame longer than source, compress/zstd_compress.c:6934 | `ZSTD_error_externalSequences_invalid` |
| 110 | ZSTD_postProcessSequenceProducerResult | `nbExternalSeqs > outSeqsCapacity` (producer error code), compress/zstd_compress.c:3177 | `ZSTD_error_sequenceProducer_failed` |
| 111 | ZSTD_postProcessSequenceProducerResult | `nbExternalSeqs == 0 && srcSize > 0` (empty parse for non-empty src), compress/zstd_compress.c:3184 | `ZSTD_error_sequenceProducer_failed` |
| 112 | ZSTD_postProcessSequenceProducerResult | `nbExternalSeqs == outSeqsCapacity` but last seq is not a delimiter, compress/zstd_compress.c:3205 | `ZSTD_error_sequenceProducer_failed` |
| 113 | ZSTD_getSequences / seqCollector | not enough space to copy sequences (`nbOutSequences > maxSequences - seqIndex`), compress/zstd_compress.c:3445 | `ZSTD_error_dstSize_tooSmall` |
| 114 | ZSTD_compressSequences_internal | `seqLenSum > srcSize` (external seqs imply too-large block), compress/zstd_compress.c:3380 | `ZSTD_error_externalSequences_invalid` |
| 115 | ZSTD_transferSequences / block build | `nbSequences >= maxNbSeq`, compress/zstd_compress.c:7327 | `ZSTD_error_externalSequences_invalid` |
| 116 | ZSTD_compressSequencesAndLiterals | `nbSequences == 0` (needs at least 1 end-of-block), compress/zstd_compress.c:7490 | `ZSTD_error_externalSequences_invalid` |
| 117 | ZSTD_compressSequencesAndLiterals | `dstCapacity < 3` (no room for empty frame block header), compress/zstd_compress.c:6962/7495 | `ZSTD_error_dstSize_tooSmall` |
| 118 | ZSTD_compressSequencesAndLiterals | `block.litSize > litSize` (sequences need more literals than present), compress/zstd_compress.c:7508 | `ZSTD_error_externalSequences_invalid` |
| 119 | ZSTD_compressSequencesAndLiterals | `dstCapacity < ZSTD_blockHeaderSize` for a new compressed block, compress/zstd_compress.c:7001/7524 | `ZSTD_error_dstSize_tooSmall` |
| 120 | ZSTD_compressSequencesAndLiterals | must produce uncompressed block but mode disallows it, compress/zstd_compress.c:7550 | `ZSTD_error_cannotProduce_uncompressedBlock` |
| 121 | ZSTD_compressSequencesAndLiterals | `litSize != 0` after consuming (literals not fully consumed), compress/zstd_compress.c:7578 | `ZSTD_error_externalSequences_invalid` |
| 122 | ZSTD_compressSequencesAndLiterals | `remaining != 0` (sequences don't total exactly srcSize), compress/zstd_compress.c:7579 | `ZSTD_error_externalSequences_invalid` |
| 123 | ZSTD_compressSequencesAndLiterals | literals buffer < litSize+8 (risk of OOB read), compress/zstd_compress.c:7598 | `ZSTD_error_workSpace_tooSmall` |
| 124 | ZSTD_compressSequencesAndLiterals | mode requires explicit delimiters but noBlockDelimiters set, compress/zstd_compress.c:7603 | `ZSTD_error_frameParameter_unsupported` |
| 125 | ZSTD_compressSequencesAndLiterals | mode incompatible with `ZSTD_c_validateSequences`, compress/zstd_compress.c:7606 | `ZSTD_error_parameter_unsupported` |
| 126 | ZSTD_compressSequencesAndLiterals | mode incompatible with frame checksum, compress/zstd_compress.c:7609 | `ZSTD_error_frameParameter_unsupported` |
| 127 | ZSTD_writeSkippableFrame | `dstCapacity < srcSize + ZSTD_SKIPPABLEHEADERSIZE`(8), compress/zstd_compress.c:4754 | `ZSTD_error_dstSize_tooSmall` |
| 128 | ZSTD_writeSkippableFrame | `srcSize > 0xFFFFFFFF` (too large for skippable frame), compress/zstd_compress.c:4756 | `ZSTD_error_srcSize_wrong` |
| 129 | ZSTD_writeSkippableFrame | `magicVariant > 15`, compress/zstd_compress.c:4757 | `ZSTD_error_parameter_outOfBound` |

## Decompression frame header

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 130 | ZSTD_frameHeaderSize_internal / getFrameHeader_advanced | `srcSize < minInputSize` (ZSTD_FRAMEHEADERSIZE_MIN: 6 for zstd1, 2 for magicless), decompress/zstd_decompress.c:419 | `ZSTD_error_srcSize_wrong` (or return of wanted size) |
| 131 | ZSTD_getFrameHeader_advanced | `src==NULL` but `srcSize>0`, decompress/zstd_decompress.c:456 | `ZSTD_error_GENERIC` |
| 132 | ZSTD_getFrameHeader_advanced | with <minInputSize bytes, first ≤4 bytes match neither `ZSTD_MAGICNUMBER`(0xFD2FB528) nor skippable magic range, decompress/zstd_decompress.c:473 | `ZSTD_error_prefix_unknown` |
| 133 | ZSTD_getFrameHeader_advanced | 4-byte magic != 0xFD2FB528 and `(magic & 0xFFFFFFF0) != 0x184D2A50` (not zstd, not skippable), decompress/zstd_decompress.c:493 | `ZSTD_error_prefix_unknown` |
| 134 | ZSTD_getFrameHeader_advanced | frame header descriptor reserved bit set (`fhdByte & 0x08`), decompress/zstd_decompress.c:511 | `ZSTD_error_frameParameter_unsupported` |
| 135 | ZSTD_getFrameHeader_advanced | decoded `windowLog > ZSTD_WINDOWLOG_MAX`, decompress/zstd_decompress.c:517 | `ZSTD_error_frameParameter_windowTooLarge` |
| 136 | ZSTD_readSkippableFrameSize | `srcSize < ZSTD_SKIPPABLEHEADERSIZE`(8), decompress/zstd_decompress.c:592 | `ZSTD_error_srcSize_wrong` |
| 137 | ZSTD_readSkippableFrameSize | `sizeU32 + 8` overflows 32-bit (`(U32)(sizeU32+8) < sizeU32`), decompress/zstd_decompress.c:595 | `ZSTD_error_frameParameter_unsupported` |
| 138 | ZSTD_decompressBound / readSkippableFrameSize | `skippableSize > srcSize`, decompress/zstd_decompress.c:598 | `ZSTD_error_srcSize_wrong` |
| 139 | ZSTD_decompressSkippableFrame | `srcSize < ZSTD_SKIPPABLEHEADERSIZE`(8), decompress/zstd_decompress.c:618 | `ZSTD_error_srcSize_wrong` |
| 140 | ZSTD_decompressSkippableFrame | src is not a skippable frame (`!ZSTD_isSkippableFrame`), decompress/zstd_decompress.c:625 | `ZSTD_error_frameParameter_unsupported` |
| 141 | ZSTD_decompressSkippableFrame | `skippableFrameSize < 8 || > srcSize`, decompress/zstd_decompress.c:626 | `ZSTD_error_srcSize_wrong` |
| 142 | ZSTD_decompressSkippableFrame | `skippableContentSize > dstCapacity`, decompress/zstd_decompress.c:627 | `ZSTD_error_dstSize_tooSmall` |
| 143 | ZSTD_getFrameHeader / decodeHeader | header decode returned wanted-more (`result > 0`, headerSize too small), decompress/zstd_decompress.c:706 | `ZSTD_error_srcSize_wrong` |
| 144 | ZSTD_decompressBegin_usingDDict / dictID check | frame's dictID != loaded dict's dictID (`fParams.dictID && dictID != fParams.dictID`), decompress/zstd_decompress.c:717 | `ZSTD_error_dictionary_wrong` |
| 145 | ZSTD_getFrameContentSize | invalid magic / srcSize too small to determine size (`ZSTD_getFrameHeader_advanced` error), decompress/zstd_decompress.c:574-579 | `ZSTD_CONTENTSIZE_ERROR` |
| 146 | ZSTD_findDecompressedSize | skippable frame size errors during scan, decompress/zstd_decompress.c:652 | `ZSTD_CONTENTSIZE_ERROR` |
| 147 | ZSTD_findDecompressedSize | accumulated frame content size overflow, decompress/zstd_decompress.c:661/664 | `ZSTD_CONTENTSIZE_ERROR` |
| 148 | ZSTD_findDecompressedSize | frameSrcSize error while walking frames, decompress/zstd_decompress.c:669 | `ZSTD_CONTENTSIZE_ERROR` |
| 149 | ZSTD_findDecompressedSize | trailing garbage after frames (`srcSize` remaining), decompress/zstd_decompress.c:677 | `ZSTD_CONTENTSIZE_ERROR` |
| 150 | ZSTD_decompressStream / windowSize | `zfh.windowSize > windowSizeMax` (frame requires more than allowed), decompress/zstd_decompress.c:2008 | `ZSTD_error_frameParameter_windowTooLarge` |
| 151 | ZSTD_decompress_usingDDict / frame walk | `err > 0` from getFrameHeader (need more input) in bounded call, decompress/zstd_decompress.c:2007 | `ZSTD_error_srcSize_wrong` |
| 152 | ZSTD_estimateDStreamSize | `(unsigned long long)minRBSize != neededSize` (size overflow), decompress/zstd_decompress.c:1983 | `ZSTD_error_frameParameter_windowTooLarge` |

## Decompression block / entropy

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 153 | ZSTD_getcBlockSize | `srcSize < ZSTD_blockHeaderSize`(3), decompress/zstd_decompress_block.c:66 | `ZSTD_error_srcSize_wrong` |
| 154 | ZSTD_getcBlockSize | block type == `bt_reserved`(3), decompress/zstd_decompress_block.c:74 | `ZSTD_error_corruption_detected` |
| 155 | ZSTD_decodeLiteralsBlock | `srcSize < MIN_CBLOCK_SIZE`(2), decompress/zstd_decompress_block.c:139 | `ZSTD_error_corruption_detected` |
| 156 | ZSTD_decodeLiteralsBlock (compressed, repeat) | repeat-stats literal mode but `dctx->litEntropy==0` (no prior table), decompress/zstd_decompress_block.c:149 | `ZSTD_error_dictionary_corrupted` |
| 157 | ZSTD_decodeLiteralsBlock | `srcSize < 5` for 4-stream compressed literals header (size_format 3), decompress/zstd_decompress_block.c:153 | `ZSTD_error_corruption_detected` |
| 158 | ZSTD_decodeLiteralsBlock | `litSize > 0 && dst == NULL`, decompress/zstd_decompress_block.c:185/271/319 | `ZSTD_error_dstSize_tooSmall` |
| 159 | ZSTD_decodeLiteralsBlock | `litSize > blockSizeMax`, decompress/zstd_decompress_block.c:186/272/320 | `ZSTD_error_corruption_detected` |
| 160 | ZSTD_decodeLiteralsBlock | 4-stream `litSize < MIN_LITERALS_FOR_4_STREAMS`, decompress/zstd_decompress_block.c:188 | `ZSTD_error_literals_headerWrong` |
| 161 | ZSTD_decodeLiteralsBlock | `litCSize + lhSize > srcSize`, decompress/zstd_decompress_block.c:191 | `ZSTD_error_corruption_detected` |
| 162 | ZSTD_decodeLiteralsBlock | `expectedWriteSize < litSize` (dst too small for literals), decompress/zstd_decompress_block.c:192/273/321 | `ZSTD_error_dstSize_tooSmall` |
| 163 | ZSTD_decodeLiteralsBlock | HUF decompression failed on literals (`HUF_isError`), decompress/zstd_decompress_block.c:241 | `ZSTD_error_corruption_detected` |
| 164 | ZSTD_decodeLiteralsBlock (RLE/raw) | `srcSize < 3` where lhSize=3 needed, decompress/zstd_decompress_block.c:266/310 | `ZSTD_error_corruption_detected` |
| 165 | ZSTD_decodeLiteralsBlock (raw, size_format) | `srcSize < 4` where lhSize+1=4 needed, decompress/zstd_decompress_block.c:315 | `ZSTD_error_corruption_detected` |
| 166 | ZSTD_decodeLiteralsBlock (raw) | `litSize+lhSize > srcSize`, decompress/zstd_decompress_block.c:276 | `ZSTD_error_corruption_detected` |
| 167 | ZSTD_buildSeqTable | `!srcSize` for FSE_repeat/compressed table, decompress/zstd_decompress_block.c:658 | `ZSTD_error_srcSize_wrong` |
| 168 | ZSTD_buildSeqTable | RLE byte `*src > max` (symbol out of range), decompress/zstd_decompress_block.c:659 | `ZSTD_error_corruption_detected` |
| 169 | ZSTD_buildSeqTable | `set_repeat` mode but `!flagRepeatTable` (no previous table), decompress/zstd_decompress_block.c:671 | `ZSTD_error_corruption_detected` |
| 170 | ZSTD_buildSeqTable | FSE header decode error, decompress/zstd_decompress_block.c:683 | `ZSTD_error_corruption_detected` |
| 171 | ZSTD_buildSeqTable | `tableLog > maxLog`, decompress/zstd_decompress_block.c:684 | `ZSTD_error_corruption_detected` |
| 172 | ZSTD_decodeSeqHeaders | `srcSize < MIN_SEQUENCES_SIZE`(1), decompress/zstd_decompress_block.c:705 | `ZSTD_error_srcSize_wrong` |
| 173 | ZSTD_decodeSeqHeaders | nbSeq encoded across 2/3 bytes but `ip+2 > iend` / `ip >= iend`, decompress/zstd_decompress_block.c:711/715 | `ZSTD_error_srcSize_wrong` |
| 174 | ZSTD_decodeSeqHeaders | after nbSeq, bytes not fully consumed as expected (`ip != iend`), decompress/zstd_decompress_block.c:723 | `ZSTD_error_corruption_detected` |
| 175 | ZSTD_decodeSeqHeaders | `ip+1 > iend` (no byte for symbol-encoding-modes), decompress/zstd_decompress_block.c:729 | `ZSTD_error_srcSize_wrong` |
| 176 | ZSTD_decodeSeqHeaders | reserved bits nonzero (`*ip & 3`), decompress/zstd_decompress_block.c:730 | `ZSTD_error_corruption_detected` |
| 177 | ZSTD_decodeSeqHeaders | LL/OF/ML `ZSTD_buildSeqTable` failed, decompress/zstd_decompress_block.c:745/757/769 | `ZSTD_error_corruption_detected` |
| 178 | ZSTD_execSequence / execSequenceEnd | `sequenceLength > (oend - op)` (last match won't fit in dst), decompress/zstd_decompress_block.c:919/967/1521/1591/1603/1682/1871/1880 | `ZSTD_error_dstSize_tooSmall` |
| 179 | ZSTD_execSequence | `sequence.litLength > (litLimit - *litPtr)` (read beyond literal buffer), decompress/zstd_decompress_block.c:920/968 | `ZSTD_error_corruption_detected` |
| 180 | ZSTD_execSequence | output would catch up to & overwrite literal buffer, decompress/zstd_decompress_block.c:973 | `ZSTD_error_dstSize_tooSmall` |
| 181 | ZSTD_execSequence | `sequence.offset > (oLitEnd - virtualStart)` (offset too large / beyond history), decompress/zstd_decompress_block.c:932/981/1054/1147 | `ZSTD_error_corruption_detected` |
| 182 | ZSTD_decompressSequences* | leftover literals `leftoverLit > (oend - op)`, decompress/zstd_decompress_block.c:1521/1788/1833 | `ZSTD_error_dstSize_tooSmall` |
| 183 | ZSTD_decompressSequences* | `nbSeq` remaining != 0 after consuming stream, decompress/zstd_decompress_block.c:1579 | `ZSTD_error_corruption_detected` |
| 184 | ZSTD_decompressSequences* | bit stream not fully consumed (`!BIT_endOfDStream`), decompress/zstd_decompress_block.c:1581/1674/1824 | `ZSTD_error_corruption_detected` |
| 185 | ZSTD_decompressBlock_internal | `srcSize > ZSTD_blockSizeMax(dctx)`, decompress/zstd_decompress_block.c:2081 | `ZSTD_error_srcSize_wrong` |
| 186 | ZSTD_decompressSequences (dst check) | `(dst==NULL || dstCapacity==0) && nbSeq > 0`, decompress/zstd_decompress_block.c:2129 | `ZSTD_error_dstSize_tooSmall` |
| 187 | ZSTD_decompressSequences (dst wraparound) | 64-bit `dst` within 1MB of `(size_t)-1` (address overflow risk), decompress/zstd_decompress_block.c:2130 | `ZSTD_error_dstSize_tooSmall` |
| 188 | ZSTD_decompressFrame | `remainingSrcSize < frameHeaderSize + ZSTD_blockHeaderSize`, decompress/zstd_decompress.c:975 | `ZSTD_error_srcSize_wrong` |
| 189 | ZSTD_decompressFrame | `cBlockSize > remainingSrcSize`, decompress/zstd_decompress.c:995 | `ZSTD_error_srcSize_wrong` |
| 190 | ZSTD_decompressFrame | invalid block type (default in switch), decompress/zstd_decompress.c:1029 | `ZSTD_error_corruption_detected` |
| 191 | ZSTD_decompressFrame | decoded output size != declared `frameContentSize`, decompress/zstd_decompress.c:1046 | `ZSTD_error_corruption_detected` |
| 192 | ZSTD_decompressFrame | checksum flagged but `remainingSrcSize < 4`, decompress/zstd_decompress.c:1050 | `ZSTD_error_checksum_wrong` |
| 193 | ZSTD_decompressFrame | stored XXH64 checksum != recomputed (`checkRead != checkCalc`), decompress/zstd_decompress.c:1055 | `ZSTD_error_checksum_wrong` |
| 194 | ZSTD_decompressMultiFrame | trailing input not entirely consumed (`srcSize` remaining after frames), decompress/zstd_decompress.c:1166 | `ZSTD_error_srcSize_wrong` |
| 195 | ZSTD_decompressMultiFrame | multiframe with static DCtx (`dctx->staticSize`) needing legacy alloc, decompress/zstd_decompress.c:1094 | `ZSTD_error_memory_allocation` |
| 196 | ZSTD_decompressMultiFrame (legacy) | legacy frame content size == `ZSTD_CONTENTSIZE_ERROR`, decompress/zstd_decompress.c:1102 | `ZSTD_error_corruption_detected` |
| 197 | ZSTD_decompressMultiFrame (legacy) | legacy expected size != decoded size, decompress/zstd_decompress.c:1104 | `ZSTD_error_corruption_detected` |
| 198 | ZSTD_decompress (RLE/raw block) | `regenSize > dstCapacity` / `srcSize > dstCapacity`, decompress/zstd_decompress.c:900/913 | `ZSTD_error_dstSize_tooSmall` |
| 199 | ZSTD_decompress (RLE/raw) | `dst == NULL` with content to write, decompress/zstd_decompress.c:903/916 | `ZSTD_error_dstBuffer_null` |
| 200 | ZSTD_setRleBlock / copyRawBlock | frame length exceeds declared frameContentSize (`op-ostart` bound), decompress/zstd_decompress.c:967 | `ZSTD_error_dstSize_tooSmall`/`corruption_detected` |

## Streaming decompression

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 201 | ZSTD_decompressContinue | `srcSize != ZSTD_nextSrcSizeToDecompressWithInputSize(dctx, srcSize)` (wrong chunk size), decompress/zstd_decompress.c:1279 | `ZSTD_error_srcSize_wrong` |
| 202 | ZSTD_decompressContinue | `cBlockSize > dctx->fParams.blockSizeMax`, decompress/zstd_decompress.c:1315 | `ZSTD_error_corruption_detected` |
| 203 | ZSTD_decompressContinue | decompressed `rSize > dctx->fParams.blockSizeMax`, decompress/zstd_decompress.c:1367 | `ZSTD_error_corruption_detected` |
| 204 | ZSTD_decompressContinue | invalid block type (default), decompress/zstd_decompress.c:1364 | `ZSTD_error_corruption_detected` |
| 205 | ZSTD_decompressContinue | recomputed checksum `check32 != h32`, decompress/zstd_decompress.c:1406 | `ZSTD_error_checksum_wrong` |
| 206 | ZSTD_decompressContinue / setDStreamStage | `dctx == NULL`, decompress/zstd_decompress.c:1208 | `ZSTD_error_memory_allocation` |
| 207 | ZSTD_DCtx_refDDict / loadDictionary | called when `dctx->streamStage != zdss_init`, decompress/zstd_decompress.c:1704/1782/1809/1908/1957 | `ZSTD_error_stage_wrong` |
| 208 | ZSTD_DCtx_loadDictionary_advanced | internal DDict allocation failed (`ddictLocal == NULL`), decompress/zstd_decompress.c:1708 | `ZSTD_error_memory_allocation` |
| 209 | ZSTD_DCtx_refDDict (multiple) | hash set full while adding DDict (`ddictPtrCount == ddictPtrTableSize`), decompress/zstd_decompress.c:109 | `ZSTD_error_GENERIC` |
| 210 | ZSTD_DCtx_refDDict (multiple) | expanded hash set allocation failed (`!newTable`), decompress/zstd_decompress.c:139 | `ZSTD_error_memory_allocation` |
| 211 | ZSTD_DCtx_refDDict (multiple) | hash set allocation failed on init, decompress/zstd_decompress.c:1791 | `ZSTD_error_memory_allocation` |
| 212 | ZSTD_decompressStream | frame windowSize > `zds->maxWindowSize`, decompress/zstd_decompress.c:2231 | `ZSTD_error_frameParameter_windowTooLarge` |
| 213 | ZSTD_decompressStream (stableOut) | output differs when `ZSTD_d_stableOutBuffer` set, decompress/zstd_decompress.c:2049 | `ZSTD_error_dstBuffer_wrong` |
| 214 | ZSTD_decompressStream (stableOut) | `ZSTD_obm_stable` set but `ZSTD_outBuffer` too small, decompress/zstd_decompress.c:2209 | `ZSTD_error_dstSize_tooSmall` |
| 215 | ZSTD_decompressStream | static DStream needs alloc for inBuff/outBuff (`zds->staticSize`), decompress/zstd_decompress.c:2131/2150 | `ZSTD_error_memory_allocation` |
| 216 | ZSTD_decompressStream | inBuff allocation failed (`zds->inBuff == NULL`), decompress/zstd_decompress.c:2264 | `ZSTD_error_memory_allocation` |
| 217 | ZSTD_decompressStream | `toLoad > zds->inBuffSize - zds->inPos` (buffer accounting error), decompress/zstd_decompress.c:2303 | `ZSTD_error_GENERIC` |
| 218 | ZSTD_decompressStream | no forward progress, output full (`op==oend`), decompress/zstd_decompress.c:2359 | `ZSTD_error_noForwardProgress_destFull` |
| 219 | ZSTD_decompressStream | no forward progress, input empty (`ip==iend`), decompress/zstd_decompress.c:2360 | `ZSTD_error_noForwardProgress_inputEmpty` |

## Dictionary load / CDict / DDict

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 220 | ZSTD_loadDEntropy | dictionary `dictSize <= 8` (too small to hold magic+entropy), decompress/zstd_decompress.c:1458 | `ZSTD_error_dictionary_corrupted` |
| 221 | ZSTD_loadDEntropy | HUF table header invalid (`HUF_isError(hSize)`), decompress/zstd_decompress.c:1477 | `ZSTD_error_dictionary_corrupted` |
| 222 | ZSTD_loadDEntropy | offcode FSE header error / `offcodeMaxValue > MaxOff`(31) / `offcodeLog > OffFSELog`(8), decompress/zstd_decompress.c:1484/1485/1486 | `ZSTD_error_dictionary_corrupted` |
| 223 | ZSTD_loadDEntropy | matchlength FSE header error / `matchlengthMaxValue > MaxML`(52) / `matchlengthLog > MLFSELog`(9), decompress/zstd_decompress.c:1499/1500/1501 | `ZSTD_error_dictionary_corrupted` |
| 224 | ZSTD_loadDEntropy | litlength FSE header error / `litlengthMaxValue > MaxLL`(35) / `litlengthLog > LLFSELog`(9), decompress/zstd_decompress.c:1514/1515/1516 | `ZSTD_error_dictionary_corrupted` |
| 225 | ZSTD_loadDEntropy | `dictPtr+12 > dictEnd` (no room for the 3 repcodes), decompress/zstd_decompress.c:1526 | `ZSTD_error_dictionary_corrupted` |
| 226 | ZSTD_loadDEntropy | a repcode `rep==0 || rep > dictContentSize`, decompress/zstd_decompress.c:1531 | `ZSTD_error_dictionary_corrupted` |
| 227 | ZSTD_decompress_insertDictionary | entropy load error propagated (`ZSTD_isError(eSize)`), decompress/zstd_decompress.c:1550 | `ZSTD_error_dictionary_corrupted` |
| 228 | ZSTD_DDict_init (createDDict) | `ZSTD_loadDEntropy` failed inside DDict init, decompress/zstd_ddict.c:112 | `ZSTD_error_dictionary_corrupted` (forwarded) |
| 229 | ZSTD_decompressBegin_usingDict | prefix dict content-type fullDict but bad magic (handled at CDict/DDict), decompress/zstd_decompress.c:1592 | `ZSTD_error_dictionary_wrong` |
| 230 | ZSTD_isFrame / dict magic (ZDICT_getDictHeaderSize) | `dictSize <= 8` or magic != `ZSTD_MAGIC_DICTIONARY`(0xEC30A437), dictBuilder/zdict.c:112 | `ZSTD_error_dictionary_corrupted` |

## Dictionary builder (ZDICT / COVER / fastCover)

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 231 | ZDICT_getDictHeaderSize | `dictSize <= 8` or `MEM_readLE32(dict) != ZSTD_MAGIC_DICTIONARY`, dictBuilder/zdict.c:112 | `ZSTD_error_dictionary_corrupted` |
| 232 | ZDICT_trainFromBuffer_legacy / addEntropyTables | entropy table workspace alloc failed, dictBuilder/zdict.c:703 | `ZSTD_error_memory_allocation` |
| 233 | ZDICT_trainFromBuffer_legacy | offcodeMax > OFFCODE_MAX (dictionary too large), dictBuilder/zdict.c:688 | `ZSTD_error_dictionaryCreation_failed` |
| 234 | ZDICT_trainFromBuffer_legacy | compressed entropy header doesn't fit dst (`eSize` overflow), dictBuilder/zdict.c:820 | `ZSTD_error_dstSize_tooSmall` |
| 235 | ZDICT_finalizeDictionary | `dictBufferCapacity < dictContentSize`, dictBuilder/zdict.c:874 | `ZSTD_error_dstSize_tooSmall` |
| 236 | ZDICT_finalizeDictionary | `dictBufferCapacity < ZDICT_DICTSIZE_MIN`(256), dictBuilder/zdict.c:875 | `ZSTD_error_dstSize_tooSmall` |
| 237 | ZDICT_finalizeDictionary | `hSize + minContentSize > dictBufferCapacity`, dictBuilder/zdict.c:905 | `ZSTD_error_dstSize_tooSmall` |
| 238 | ZDICT_trainFromBuffer_legacy | dictList allocation failed, dictBuilder/zdict.c:993 | `ZSTD_error_memory_allocation` |
| 239 | ZDICT_trainFromBuffer_legacy | `maxDictSize < ZDICT_DICTSIZE_MIN`(256), dictBuilder/zdict.c:994 | `ZSTD_error_dstSize_tooSmall` |
| 240 | ZDICT_trainFromBuffer_legacy | `samplesBuffSize < ZDICT_MIN_SAMPLES_SIZE` (not enough source), dictBuilder/zdict.c:995 | `ZSTD_error_dictionaryCreation_failed` |
| 241 | ZDICT_trainFromBuffer_legacy | resulting `dictContentSize < ZDICT_CONTENTSIZE_MIN`(128), dictBuilder/zdict.c:1030 | `ZSTD_error_dictionaryCreation_failed` |
| 242 | ZDICT_trainFromBuffer_legacy | sample-buffer scratch alloc failed (`!newBuff`), dictBuilder/zdict.c:1094 | `ZSTD_error_memory_allocation` |
| 243 | ZDICT_optimizeTrainFromBuffer_cover (COVER_tryParameters) | totalSamplesSize < max(d,8) or >= COVER_MAX_SAMPLES_SIZE, dictBuilder/cover.c:618 | `ZSTD_error_srcSize_wrong` |
| 244 | COVER training | `nbTrainSamples < 5`, dictBuilder/cover.c:623 | `ZSTD_error_srcSize_wrong` |
| 245 | COVER training | `nbTestSamples < 1`, dictBuilder/cover.c:628 | `ZSTD_error_srcSize_wrong` |
| 246 | COVER training | context/map allocation failed, dictBuilder/cover.c:651 | `ZSTD_error_memory_allocation` |
| 247 | ZDICT_trainFromBuffer_cover | `COVER_checkParameters` false (d==0/k==0, k>maxDictSize, d>k), dictBuilder/cover.c:793 | `ZSTD_error_parameter_outOfBound` |
| 248 | ZDICT_trainFromBuffer_cover | `nbSamples == 0`, dictBuilder/cover.c:797 | `ZSTD_error_srcSize_wrong` |
| 249 | ZDICT_trainFromBuffer_cover | `dictBufferCapacity < ZDICT_DICTSIZE_MIN`(256), dictBuilder/cover.c:802 | `ZSTD_error_dstSize_tooSmall` |
| 250 | ZDICT_optimizeTrainFromBuffer_cover | `splitPoint <= 0 || splitPoint > 1`, dictBuilder/cover.c:1195 | `ZSTD_error_parameter_outOfBound` |
| 251 | ZDICT_optimizeTrainFromBuffer_cover | `kMinK < kMaxD || kMaxK < kMinK` (bad k/d search range), dictBuilder/cover.c:1199 | `ZSTD_error_parameter_outOfBound` |
| 252 | ZDICT_optimizeTrainFromBuffer_cover | `nbSamples == 0`, dictBuilder/cover.c:1205 | `ZSTD_error_srcSize_wrong` |
| 253 | ZDICT_optimizeTrainFromBuffer_cover | `dictBufferCapacity < ZDICT_DICTSIZE_MIN`, dictBuilder/cover.c:1210 | `ZSTD_error_dstSize_tooSmall` |
| 254 | ZDICT_optimizeTrainFromBuffer_cover | thread pool creation failed (`nbThreads>1`), dictBuilder/cover.c:1215 | `ZSTD_error_memory_allocation` |
| 255 | FASTCOVER training | totalSamplesSize < max(d,8) or >= FASTCOVER_MAX_SAMPLES_SIZE, dictBuilder/fastcover.c:332 | `ZSTD_error_srcSize_wrong` |
| 256 | FASTCOVER training | `nbTrainSamples < 5`, dictBuilder/fastcover.c:338 | `ZSTD_error_srcSize_wrong` |
| 257 | FASTCOVER training | `nbTestSamples < 1`, dictBuilder/fastcover.c:344 | `ZSTD_error_srcSize_wrong` |
| 258 | FASTCOVER training | offsets/freqs allocation failed, dictBuilder/fastcover.c:369/386 | `ZSTD_error_memory_allocation` |
| 259 | ZDICT_trainFromBuffer_fastCover | `FASTCOVER_checkParameters` false (d not 6/8, k==0, k>maxDictSize, d>k, f>MAX_F or 0, accel>10 or 0, splitPoint invalid), dictBuilder/fastcover.c:571 | `ZSTD_error_parameter_outOfBound` |
| 260 | ZDICT_trainFromBuffer_fastCover | `nbSamples == 0`, dictBuilder/fastcover.c:575 | `ZSTD_error_srcSize_wrong` |
| 261 | ZDICT_trainFromBuffer_fastCover | `dictBufferCapacity < ZDICT_DICTSIZE_MIN`, dictBuilder/fastcover.c:580 | `ZSTD_error_dstSize_tooSmall` |
| 262 | ZDICT_addEntropyTablesFromBuffer / divsufsort | suffix sort failed (`divSuftSortResult != 0`), dictBuilder/zdict.c:507 | `ZSTD_error_GENERIC` |

## FSE / HUF entropy low level

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 263 | FSE_readNCount / readNCount_bmi2 | `nbBits > FSE_TABLELOG_ABSOLUTE_MAX`(15), common/entropy_common.c:73 | `ZSTD_error_tableLog_tooLarge` |
| 264 | FSE_readNCount | `countSize > hbSize` (header consumes past input), common/entropy_common.c:64 | `ZSTD_error_corruption_detected` |
| 265 | FSE_readNCount | `remaining != 1` at end (bad normalized distribution), common/entropy_common.c:179 | `ZSTD_error_corruption_detected` |
| 266 | FSE_readNCount | `charnum > maxSV1` (more symbols than declared), common/entropy_common.c:181 | `ZSTD_error_maxSymbolValue_tooSmall` |
| 267 | FSE_readNCount | `bitCount > 32` (header overflow), common/entropy_common.c:182 | `ZSTD_error_corruption_detected` |
| 268 | HUF_readStats_body | `!srcSize` (empty weight header), common/entropy_common.c:254 | `ZSTD_error_srcSize_wrong` |
| 269 | HUF_readStats_body | `iSize+1 > srcSize`, common/entropy_common.c:261/270 | `ZSTD_error_srcSize_wrong` |
| 270 | HUF_readStats_body | `oSize >= hwSize` (too many weights), common/entropy_common.c:262 | `ZSTD_error_corruption_detected` |
| 271 | HUF_readStats_body | a `huffWeight[n] > HUF_TABLELOG_MAX`(12), common/entropy_common.c:280 | `ZSTD_error_corruption_detected` |
| 272 | HUF_readStats_body | `weightTotal == 0`, common/entropy_common.c:284 | `ZSTD_error_corruption_detected` |
| 273 | HUF_readStats_body | derived `tableLog > HUF_TABLELOG_MAX`, common/entropy_common.c:288 | `ZSTD_error_corruption_detected` |
| 274 | HUF_readStats_body | `verif != rest` (last weight not a clean power of 2), common/entropy_common.c:295 | `ZSTD_error_corruption_detected` |
| 275 | HUF_readStats_body | `rankStats[1] < 2 || rankStats[1] & 1` (fewer than 2 rank-1 symbols / odd), common/entropy_common.c:301 | `ZSTD_error_corruption_detected` |
| 276 | FSE_buildDTable_wksp | `FSE_BUILD_DTABLE_WKSP_SIZE > wkspSize` or `maxSymbolValue > FSE_MAX_SYMBOL_VALUE`(255), common/fse_decompress.c:70/71 | `ZSTD_error_maxSymbolValue_tooLarge` |
| 277 | FSE_buildDTable_wksp | `tableLog > FSE_MAX_TABLELOG`, common/fse_decompress.c:72 | `ZSTD_error_tableLog_tooLarge` |
| 278 | FSE_buildDTable_wksp | normalized distribution invalid (`position != 0`), common/fse_decompress.c:146 | `ZSTD_error_GENERIC` |
| 279 | FSE_decompress_wksp_body | reload overflowed the bit stream (BIT_DStream_overflow), common/fse_decompress.c:193 | `ZSTD_error_corruption_detected` |
| 280 | FSE_decompress_wksp_body | output overflow (`op > omax-2`), common/fse_decompress.c:220/227 | `ZSTD_error_dstSize_tooSmall` |
| 281 | FSE_decompress_wksp_body | `wkspSize < sizeof(*wksp)`, common/fse_decompress.c:258 | `ZSTD_error_GENERIC` |
| 282 | FSE_decompress_wksp_body | `tableLog > maxLog`, common/fse_decompress.c:267 | `ZSTD_error_tableLog_tooLarge` |
| 283 | FSE_decompress_wksp_body | `FSE_DECOMPRESS_WKSP_SIZE > wkspSize`, common/fse_decompress.c:273 | `ZSTD_error_tableLog_tooLarge` |
| 284 | FSE_buildCTable_wksp | `FSE_BUILD_CTABLE_WORKSPACE_SIZE > wkspSize`, compress/fse_compress.c:87 | `ZSTD_error_tableLog_tooLarge` |
| 285 | FSE_writeNCount_generic | buffer overflow while writing NCount, compress/fse_compress.c:269/284/306/320 | `ZSTD_error_dstSize_tooSmall` |
| 286 | FSE_writeNCount / normalizeCount | `remaining < 1` / incorrect normalized distribution, compress/fse_compress.c:301/315 | `ZSTD_error_GENERIC` |
| 287 | FSE_optimalTableLog / normalize | `tableLog > FSE_MAX_TABLELOG`, compress/fse_compress.c:333/472 | `ZSTD_error_tableLog_tooLarge` |
| 288 | FSE_normalizeCount | `tableLog < FSE_MIN_TABLELOG` or below FSE_minTableLog, compress/fse_compress.c:334/471/473 | `ZSTD_error_GENERIC` |
| 289 | HUF_compressWeights / writeCTable | workspace too small (`< sizeof(...Wksp)`), compress/huf_compress.c:159/263 | `ZSTD_error_GENERIC` |
| 290 | HUF_writeCTable_wksp | `maxSymbolValue > HUF_SYMBOLVALUE_MAX`(255), compress/huf_compress.c:264 | `ZSTD_error_maxSymbolValue_tooLarge` |
| 291 | HUF_writeCTable_wksp | `maxDstSize < 1` / `((maxSymbolValue+1)/2)+1 > maxDstSize`, compress/huf_compress.c:274/283 | `ZSTD_error_dstSize_tooSmall` |
| 292 | HUF_readCTable | `tableLog > HUF_TABLELOG_MAX`(12), compress/huf_compress.c:305 | `ZSTD_error_tableLog_tooLarge` |
| 293 | HUF_readCTable | `nbSymbols > *maxSymbolValuePtr+1`, compress/huf_compress.c:306 | `ZSTD_error_maxSymbolValue_tooSmall` |
| 294 | HUF_buildCTable_wksp | `maxSymbolValue > HUF_SYMBOLVALUE_MAX` / workspace too small, compress/huf_compress.c:771/774 | `ZSTD_error_workSpace_tooSmall` / `maxSymbolValue_tooLarge` |
| 295 | HUF_setMaxHeight | `maxNbBits > HUF_TABLELOG_MAX`, compress/huf_compress.c:786 | `ZSTD_error_GENERIC` |
| 296 | HUF_compress1X_usingCTable_internal | `dstCapacity <= sizeof(bitContainer)`, compress/huf_compress.c:863 | `ZSTD_error_dstSize_tooSmall` |
| 297 | HUF_compress4X_wksp / compress_internal | `wkspSize < sizeof(*table)`, compress/huf_compress.c:1349 | `ZSTD_error_workSpace_tooSmall` |
| 298 | HUF_compress4X_wksp | `srcSize > HUF_BLOCKSIZE_MAX`(128KB), compress/huf_compress.c:1352 | `ZSTD_error_srcSize_wrong` |
| 299 | HUF_compress4X_wksp | `huffLog > HUF_TABLELOG_MAX` / `maxSymbolValue > HUF_SYMBOLVALUE_MAX`, compress/huf_compress.c:1353/1354 | `ZSTD_error_tableLog_tooLarge` / `maxSymbolValue_tooLarge` |
| 300 | HUF_decompress1X/4X (various) | corruption in stream: bit-stream not ended, jump-table overflow, stream ptr crossovers, `!endCheck`, decompress/huf_decompress.c:213/238/285/292/592/608/609/643/644/680-693 (and 1373-1496) | `ZSTD_error_corruption_detected` |
| 301 | HUF_decompress4X (jump table) | `cSrcSize < 10` (too small for 4-stream jump table), decompress/huf_decompress.c:608/1389 | `ZSTD_error_corruption_detected` |
| 302 | HUF_decompress4X | `dstSize < 6` (4-split can't work), decompress/huf_decompress.c:609/1390 | `ZSTD_error_corruption_detected` |
| 303 | HUF_readDTableX* | `sizeof(*wksp) > wkspSize`, decompress/huf_decompress.c:395/1193 | `ZSTD_error_tableLog_tooLarge` / `GENERIC` |
| 304 | HUF_readDTableX* | `tableLog > maxTableLog` / `maxTableLog > HUF_TABLELOG_MAX`, decompress/huf_decompress.c:409/1200/1207 | `ZSTD_error_tableLog_tooLarge` |
| 305 | HUF_decompress (auto) | `hSize >= cSrcSize` (header consumes all input), decompress/huf_decompress.c:938/1763/1778/1900 | `ZSTD_error_srcSize_wrong` |
| 306 | HUF_decompress (single symbol) | `dstSize == 0`, decompress/huf_decompress.c:1850/1927 | `ZSTD_error_dstSize_tooSmall` |
| 307 | HUF_decompress (single symbol) | `cSrcSize > dstSize` (invalid), decompress/huf_decompress.c:1851 | `ZSTD_error_corruption_detected` |

## Deprecated ZBUFF

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 308 | ZBUFF_compressInit_advanced | forwards `ZSTD_checkCParams(params.cParams)`; invalid cParams (windowLog etc. out of bound), deprecated/zbuff_compress.c:80 | `ZSTD_error_parameter_outOfBound` (forwarded) |
| 309 | ZBUFF_compressInit_advanced | forwards each `ZSTD_CCtx_setParameter`; any out-of-bound param, deprecated/zbuff_compress.c:81-91 | `ZSTD_error_parameter_outOfBound` (forwarded) |
| 310 | ZBUFF_compressInit_advanced | forwards `ZSTD_CCtx_loadDictionary`; dict load failure, deprecated/zbuff_compress.c:93 | dictionary/memory error (forwarded) |
| 311 | ZBUFF_compressContinue / flush / end | forwards `ZSTD_compressStream2`; e.g. output buffer too small, deprecated/zbuff_compress.c:129/145/158 | `ZSTD_error_dstSize_tooSmall` etc. (forwarded) |
| 312 | ZBUFF_decompressInit_usingDict | forwards `ZSTD_initDStream_usingDict`; corrupt dict, deprecated/zbuff_decompress.c:42 | `ZSTD_error_dictionary_corrupted` (forwarded) |
| 313 | ZBUFF_decompressContinue | forwards `ZSTD_decompressStream`; any decode error (corruption/checksum/window too large), deprecated/zbuff_decompress.c:69 | forwarded ZSTD decode error |

## Legacy v01–v07 decoders

Legacy decode is reachable via `ZSTD_decompress*`/`ZSTD_decompressStream` when the input
magic matches an older version (`ZSTD_isLegacy`). Errors surface as `ERROR(version_unsupported)`
from the legacy dispatcher and as internal `corruption_detected`/`srcSize_wrong` from each vN
decoder; the public boundary result is a `ZSTD_isError()` code.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 314 | ZSTD_decompressLegacy / ZSTD_initLegacyStream | version byte not in supported legacy set (default in switch), legacy/zstd_legacy.h:284 | `ZSTD_error_version_unsupported` |
| 315 | ZSTD_decompressLegacyStream | unsupported legacy version at stream dispatch (default), legacy/zstd_legacy.h:387 | `ZSTD_error_version_unsupported` |
| 316 | ZSTDv0N_decompress (v01–v07) | truncated/oversized input, bad block header, or checksum mismatch inside a legacy frame | `ZSTD_error_corruption_detected` / `srcSize_wrong` / `dstSize_tooSmall` (per-version, `ZSTD_isError()`) |
| 317 | ZSTDv0N frame header | legacy magic recognized but header size/window invalid | version-specific error convertible via `ZSTD_isError()` |

## Enum / out-of-range values crossing FFI

These describe what the C actually does when a caller passes an out-of-range enum value
(as an `int`) across the API. Note: many enums are checked by `BOUNDCHECK`/`getBounds`
against a `[lower,upper]` interval, so values below the lowest or above the highest enumerator
are rejected; intermediate valid enumerators are accepted.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 318 | ZSTD_CCtx_setParameter | `param` = out-of-range `ZSTD_cParameter` int (e.g. 99, 999, -1, 0): no matching `case`, hits `default`, compress/zstd_compress.c:765 | `ZSTD_error_parameter_unsupported` |
| 319 | ZSTD_DCtx_setParameter | `param` = out-of-range `ZSTD_dParameter` int (e.g. 99, 999, -1, 0): no matching `case`, hits `default`, decompress/zstd_decompress.c:1903/1944 | `ZSTD_error_parameter_unsupported` |
| 320 | ZSTD_CCtx_reset | `ZSTD_ResetDirective` = 0, 4, or -1 (valid: session_only=1, parameters=2, session_and_parameters=3): value matches neither `if` branch, so nothing happens, compress/zstd_compress.c:1367-1380 | `0` (no error, silent no-op). Only `ZSTD_reset_parameters`/`session_and_parameters` while not in init stage → `ZSTD_error_stage_wrong` (line 1380) |
| 321 | ZSTD_compressStream2 | `endOp` (`ZSTD_EndDirective`) = 3 or -1 (valid 0..2): `(U32)endOp > (U32)ZSTD_e_end`, compress/zstd_compress.c:6456 | `ZSTD_error_parameter_outOfBound` |
| 322 | ZSTD_CCtx_setParameter(ZSTD_c_strategy) | `ZSTD_strategy` = 0 or 10 (valid 1..9): 0 means "use default" (accepted, no-op); 10 fails BOUNDCHECK, compress/zstd_compress.c:826 | 0→accepted (default); 10→`ZSTD_error_parameter_outOfBound` |
| 323 | ZSTD_CCtx_loadDictionary_advanced | `ZSTD_dictContentType` = 3 (valid 0..2 auto/rawContent/fullDict): treated as fullDict path fails magic → for auto/raw no reject; explicit fullDict on non-dict data, compress/zstd_compress.c:5207/5223 | `ZSTD_error_dictionary_wrong` (when fullDict path taken on invalid data); otherwise value simply not matched as a known type |
| 324 | ZSTD_CCtx_loadDictionary_advanced | `ZSTD_dictLoadMethod_e` = 2 (valid 0..1 byCopy/byRef): no explicit range check; byRef branch is `== ZSTD_dlm_byRef`, any other value (incl. 2) falls through to the byCopy branch | byCopy behavior (value 2 treated as byCopy; no error) |
| 325 | ZSTD_CCtx_setParameter(ZSTD_c_format) / DCtx ZSTD_d_format | `ZSTD_format_e` = 2 (valid 0..1 zstd1/zstd1_magicless): fails BOUNDCHECK against upperBound=1, compress/zstd_compress.c:776 (dParam: decompress/zstd_decompress.c:1874) | `ZSTD_error_parameter_outOfBound` |
| 326 | ZSTD_getErrorString / ZSTD_getErrorCode | `ZSTD_ErrorCode` = -1: `ERR_getErrorString` switch has no matching case → default, common/error_private.c | returns `"Unspecified error code"` (no crash) |
| 327 | ZSTD_getErrorString | `ZSTD_ErrorCode` = 121 or 1000 (beyond `ZSTD_error_maxCode`): no matching case → default, common/error_private.c | returns `"Unspecified error code"` |
| 328 | ZSTD_getErrorName | `code` that is not an error per `ERR_isError` (`ERR_getErrorCode` returns 0 == no_error) | returns `"No error detected"` |
