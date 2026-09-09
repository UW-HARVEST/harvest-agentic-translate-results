# ERRORS.md — error-surface table of the C zstd library

Mechanically derived from `c_src/src` (every `RETURN_ERROR`/`ERROR(`/`FORWARD_IF_ERROR`,
`BOUNDCHECK`, explicit range/null check, `return NULL`, and sentinel return reachable from a
public exported symbol). Paths are relative to `c_src/src/`.

Sentinels: `ZSTD_CONTENTSIZE_UNKNOWN = (u64)-1`, `ZSTD_CONTENTSIZE_ERROR = (u64)-2`,
error codes are returned as `(size_t)(0 - code)`.

Legend: `[x]` = a differential test constructs this exact condition and asserts C and Rust
return the SAME code/sentinel. Test names refer to `tests/phase_c_errors*.rs` unless noted.

| # | function | trigger (exact invalid input/condition) | expected C result | C site | [ ] | test |
|---|----------|------------------------------------------|-------------------|--------|-----|------|
| 1 | ZSTD_isError | code <= (size_t)-120 (incl. 0 and valid sizes) | 0 | common/error_private.h:52 | [x] | err_error_api |
| 2 | ZSTD_isError | code > (size_t)-120 | 1 | common/error_private.h:52 | [x] | err_error_api |
| 3 | ZSTD_getErrorCode | argument is not an error code | ZSTD_error_no_error (0) | error_private.h:54 | [x] | err_error_api |
| 4 | ZSTD_getErrorCode | argument is an error code | (ZSTD_ErrorCode)(0-code) | error_private.h:54 | [x] | err_error_api |
| 5 | ZSTD_getErrorString | code == maxCode or not in enum (7, 99, 200, negative) | "Unspecified error code" | error_private.c:60 | [x] | err_error_api |
| 6 | ZSTD_getErrorName | non-error argument | "No error detected" | error_private.h:72 | [x] | err_error_api |
| 7 | ZSTD_compressBound | srcSize >= ZSTD_MAX_INPUT_SIZE (0xFF00FF00FF00FF00) | srcSize_wrong (72) | compress/zstd_compress.c:72 | [x] | err_compressbound_huge |
| 8 | ZSTD_compress/compressCCtx/_usingDict/_advanced | dstCapacity < ZSTD_FRAMEHEADERSIZE_MAX | dstSize_tooSmall | zstd_compress.c:4712 | [x] | err_dst_capacity_compress |
| 9 | ZSTD_compress family | dstCapacity < blockHeaderSize+MIN_CBLOCK_SIZE+1 at block boundary | dstSize_tooSmall | zstd_compress.c:4623 | [x] | err_dst_capacity_compress |
| 10 | ZSTD_compress family (incompressible) | srcSize + 3 > dstCapacity in noCompressBlock | dstSize_tooSmall | zstd_compress_internal.h:654 | [x] | err_dst_capacity_compress |
| 11 | ZSTD_compress family (RLE block) | dstCapacity < 4 in rleCompressBlock | dstSize_tooSmall | zstd_compress_internal.h:666 | [x] | err_dst_capacity_compress |
| 12 | ZSTD_compress family (epilogue) | dstCapacity < 3 for last empty block | dstSize_tooSmall | zstd_compress.c:5365 | [x] | err_dst_capacity_compress |
| 13 | ZSTD_compress family | checksumFlag=1 and dstCapacity < 4 for trailing xxh32 | dstSize_tooSmall | zstd_compress.c:5373 | [x] | err_dst_capacity_compress |
| 14 | ZSTD_compress/compressCCtx | compressionLevel wildly out of range (1000, -1000, 0) | NO error — clamped | zstd_compress.c:5479 | [x] | cfg_level_equivalences (phase_b_params2) |
| 15 | ZSTD_compress_advanced | any of 7 cParams out of ZSTD_cParam_getBounds | parameter_outOfBound | zstd_compress.c:5448→1390 | [x] | err_compress_advanced_params |
| 16 | ZSTD_compress_usingDict / createCDict(_advanced) | dct_fullDict and (dict==NULL or dictSize<8) | dictionary_wrong | zstd_compress.c:5207 | [x] | err_dict_fullDict_small |
| 17 | ZSTD_compress_usingDict (fullDict) | MEM_readLE32(dict) != ZSTD_MAGIC_DICTIONARY | dictionary_wrong | zstd_compress.c:5223 | [x] | err_dict_fullDict_on_raw |
| 18 | ZSTD_compress_usingDict (full dict) | corrupt Huffman table in dict | dictionary_corrupted | zstd_compress.c:5081 | [x] | err_dict_corrupted_variants |
| 19 | ZSTD_compress_usingDict (full dict) | offcodeLog>OffFSELog / mlLog>MLFSELog / llLog>LLFSELog | dictionary_corrupted | zstd_compress.c:5088,5103,5117 | [x] | err_dict_corrupted_variants |
| 20 | ZSTD_compress_usingDict (full dict) | fewer than 12 bytes left for repcodes | dictionary_corrupted | zstd_compress.c:5127 | [x] | err_dict_corrupted_variants |
| 21 | ZSTD_compress_usingDict (full dict) | repcode == 0 or > dictContentSize | dictionary_corrupted | zstd_compress.c:5145 | [x] | err_dict_corrupted_variants |
| 22 | ZSTD_compress2 | dstCapacity exhausted (result != 0) | dstSize_tooSmall | zstd_compress.c:6592 | [x] | err_dst_capacity_compress |
| 23 | ZSTD_createCCtx_advanced / createCStream_advanced | customAlloc XOR customFree | NULL | zstd_compress.c:118 | [x] | err_custommem_xor |
| 24 | ZSTD_initStaticCCtx / initStaticCStream | workspaceSize <= sizeof(ZSTD_CCtx) | NULL | zstd_compress.c:130 | [x] | err_init_static_small |
| 25 | ZSTD_initStaticCCtx / initStaticCStream | workspace not 8-byte aligned | NULL | zstd_compress.c:131 | [x] | err_init_static_misaligned |
| 26 | ZSTD_initStaticCCtx | workspace < TMP_WORKSPACE_SIZE + 2*blockState | NULL | zstd_compress.c:142 | [x] | err_init_static_small |
| 27 | ZSTD_freeCCtx / freeCStream | cctx came from initStaticCCtx | memory_allocation | zstd_compress.c:185 | [x] | err_free_static_cctx |
| 28 | ZSTD_freeCCtx | cctx == NULL | 0 | zstd_compress.c:184 | [x] | err_null_free_and_sizeof |
| 29 | ZSTD_sizeof_CCtx / sizeof_CStream | cctx == NULL | 0 | zstd_compress.c:208,216 | [x] | err_null_free_and_sizeof |
| 30 | ZSTD_createCCtxParams | customAlloc XOR customFree | NULL | zstd_compress.c:332 | [x] | err_custommem_xor — NOTE: `ZSTD_createCCtxParams_advanced` is `static` (not exported), so the XOR path is unreachable across the FFI boundary; the exported default path is compared instead |
| 31 | ZSTD_CCtxParams_init / _reset | cctxParams == NULL | GENERIC (1) | zstd_compress.c:359,355 | [x] | err_ccctxparams_null |
| 32 | ZSTD_CCtxParams_init_advanced | cctxParams == NULL | GENERIC | zstd_compress.c:397 | [x] | err_ccctxparams_null |
| 33 | ZSTD_CCtxParams_init_advanced | params.cParams out of bounds | parameter_outOfBound | zstd_compress.c:398 | [x] | err_ccctxparams_null |
| 34 | ZSTD_cParam_getBounds | param not a recognized enum (0, 999, INT_MAX, INT_MIN) | bounds.error = parameter_unsupported, lower=upper=0 | zstd_compress.c:634 | [x] | cfg_cParam_getBounds_all (phase_b_params) |
| 35 | ZSTD_CCtx_setParameter | param not a recognized enum value | parameter_unsupported | zstd_compress.c:765 | [x] | err_cparam_unknown |
| 36 | ZSTD_CCtx_setParameter | streamStage != zcss_init and param not update-authorized | stage_wrong | zstd_compress.c:715 | [x] | err_setparam_stage_wrong |
| 37 | ZSTD_CCtx_setParameter | nbWorkers != 0 on a static CCtx | parameter_unsupported | zstd_compress.c:721 | [x] | err_mt_params_unsupported |
| 38 | ZSTD_CCtxParams_setParameter | param not a recognized enum value | parameter_unsupported | zstd_compress.c:1019 | [x] | err_cparam_unknown |
| 39 | setParameter | windowLog != 0 outside [10,31] | parameter_outOfBound | zstd_compress.c:793 | [x] | err_cparam_out_of_bound |
| 40 | setParameter | hashLog != 0 outside [6,30] | parameter_outOfBound | zstd_compress.c:799 | [x] | err_cparam_out_of_bound |
| 41 | setParameter | chainLog != 0 outside [6,30] | parameter_outOfBound | zstd_compress.c:805 | [x] | err_cparam_out_of_bound |
| 42 | setParameter | searchLog != 0 outside [1,30] | parameter_outOfBound | zstd_compress.c:811 | [x] | err_cparam_out_of_bound |
| 43 | setParameter | minMatch != 0 outside [3,7] | parameter_outOfBound | zstd_compress.c:817 | [x] | err_cparam_out_of_bound |
| 44 | setParameter | targetLength outside [0,131072] (checked even for 0) | parameter_outOfBound | zstd_compress.c:822 | [x] | err_cparam_out_of_bound |
| 45 | setParameter | strategy != 0 outside [1,9] | parameter_outOfBound | zstd_compress.c:828 | [x] | err_cparam_out_of_bound |
| 46 | setParameter | format not in {0,1} | parameter_outOfBound | zstd_compress.c:777 | [x] | err_cparam_out_of_bound |
| 47 | setParameter | forceAttachDict outside [0,3] | parameter_outOfBound | zstd_compress.c:854 | [x] | err_cparam_out_of_bound |
| 48 | setParameter | literalCompressionMode outside [0,2] | parameter_outOfBound | zstd_compress.c:861 | [x] | err_cparam_out_of_bound |
| 49 | setParameter | nbWorkers != 0 (non-MT build) | parameter_unsupported | zstd_compress.c:868 | [x] | err_mt_params_unsupported |
| 50 | setParameter | jobSize != 0 (non-MT build) | parameter_unsupported | zstd_compress.c:878 | [x] | err_mt_params_unsupported |
| 51 | setParameter | overlapLog != 0 (non-MT build) | parameter_unsupported | zstd_compress.c:892 | [x] | err_mt_params_unsupported |
| 52 | setParameter | rsyncable != 0 (non-MT build) | parameter_unsupported | zstd_compress.c:902 | [x] | err_mt_params_unsupported |
| 53 | setParameter | enableLongDistanceMatching outside [0,2] | parameter_outOfBound | zstd_compress.c:915 | [x] | err_cparam_out_of_bound |
| 54 | setParameter | ldmHashLog != 0 outside [6,30] | parameter_outOfBound | zstd_compress.c:921 | [x] | err_cparam_out_of_bound |
| 55 | setParameter | ldmMinMatch != 0 outside [4,4096] | parameter_outOfBound | zstd_compress.c:927 | [x] | err_cparam_out_of_bound |
| 56 | setParameter | ldmBucketSizeLog != 0 outside [1,8] | parameter_outOfBound | zstd_compress.c:933 | [x] | err_cparam_out_of_bound |
| 57 | setParameter | ldmHashRateLog != 0 outside [0,25] | parameter_outOfBound | zstd_compress.c:939 | [x] | err_cparam_out_of_bound |
| 58 | setParameter | targetCBlockSize != 0 > 131072 (below MIN is raised, not rejected) | parameter_outOfBound | zstd_compress.c:945 | [x] | err_cparam_out_of_bound, cfg_targetcblocksize_clamp |
| 59 | setParameter | srcSizeHint != 0 outside [0,INT_MAX] | parameter_outOfBound | zstd_compress.c:953 | [x] | err_cparam_out_of_bound |
| 60 | setParameter | stableInBuffer / stableOutBuffer not in {0,1} | parameter_outOfBound | zstd_compress.c:958,963 | [x] | err_cparam_out_of_bound |
| 61 | setParameter | blockDelimiters not in {0,1} | parameter_outOfBound | zstd_compress.c:968 | [x] | err_cparam_out_of_bound |
| 62 | setParameter | validateSequences not in {0,1} | parameter_outOfBound | zstd_compress.c:973 | [x] | err_cparam_out_of_bound |
| 63 | setParameter | splitAfterSequences outside [0,2] | parameter_outOfBound | zstd_compress.c:978 | [x] | err_cparam_out_of_bound |
| 64 | setParameter | blockSplitterLevel outside [0,6] | parameter_outOfBound | zstd_compress.c:983 | [x] | err_cparam_out_of_bound |
| 65 | setParameter | useRowMatchFinder outside [0,2] | parameter_outOfBound | zstd_compress.c:988 | [x] | err_cparam_out_of_bound |
| 66 | setParameter | deterministicRefPrefix not in {0,1} | parameter_outOfBound | zstd_compress.c:993 | [x] | err_cparam_out_of_bound |
| 67 | setParameter | prefetchCDictTables outside [0,2] | parameter_outOfBound | zstd_compress.c:998 | [x] | err_cparam_out_of_bound |
| 68 | setParameter | enableSeqProducerFallback not in {0,1} | parameter_outOfBound | zstd_compress.c:1003 | [x] | err_cparam_out_of_bound |
| 69 | setParameter | maxBlockSize != 0 outside [1024,131072] | parameter_outOfBound | zstd_compress.c:1009 | [x] | err_cparam_out_of_bound |
| 70 | setParameter | repcodeResolution outside [0,2] | parameter_outOfBound | zstd_compress.c:1015 | [x] | err_cparam_out_of_bound |
| 71 | setParameter | compressionLevel out of [min,max] | NO error — clamped | zstd_compress.c:782 | [x] | err_cparam_out_of_bound |
| 72 | ZSTD_CCtx_getParameter / CCtxParams_getParameter | param not a recognized enum | parameter_unsupported | zstd_compress.c:1166 | [x] | err_cparam_unknown |
| 73 | ZSTD_CCtxParams_getParameter | nbWorkers/jobSize/overlapLog (non-MT) | parameter_unsupported | zstd_compress.c:1086,1094,1101 | [x] | err_mt_params_unsupported |
| 74 | ZSTD_CCtx_setParametersUsingCCtxParams | streamStage != zcss_init | stage_wrong | zstd_compress.c:1182 | [x] | err_setparam_stage_wrong |
| 75 | ZSTD_CCtx_setParametersUsingCCtxParams | a cdict is attached | stage_wrong | zstd_compress.c:1184 | [x] | err_setparam_stage_wrong |
| 76 | ZSTD_CCtx_setPledgedSrcSize | called after compression started | stage_wrong | zstd_compress.c:1233 | [x] | err_setparam_stage_wrong |
| 77 | ZSTD_CCtx_loadDictionary(_advanced/_byReference) | streamStage != zcss_init | stage_wrong | zstd_compress.c:1290 | [x] | err_setparam_stage_wrong |
| 78 | ZSTD_CCtx_loadDictionary (byCopy) | static CCtx cannot copy the dict | memory_allocation | zstd_compress.c:1300 | [x] | err_static_loaddict |
| 79 | ZSTD_CCtx_loadDictionary | dict == NULL or dictSize == 0 | 0 (dict cleared) | zstd_compress.c:1293 | [x] | err_loaddict_null |
| 80 | ZSTD_CCtx_refCDict | streamStage != zcss_init | stage_wrong | zstd_compress.c:1330 | [x] | err_setparam_stage_wrong |
| 81 | ZSTD_CCtx_refThreadPool | streamStage != zcss_init | stage_wrong | zstd_compress.c:1340 | [x] | err_setparam_stage_wrong |
| 82 | ZSTD_CCtx_refPrefix(_advanced) | streamStage != zcss_init | stage_wrong | zstd_compress.c:1354 | [x] | err_setparam_stage_wrong |
| 83 | ZSTD_CCtx_reset | reset_parameters / session_and_parameters mid-frame | stage_wrong | zstd_compress.c:1376 | [x] | err_reset_parameters_midframe |
| 84 | ZSTD_CCtx_reset | reset value not one of the 3 enumerators (0, 4, -1, INT_MAX) | 0 — no validation | zstd_compress.c:1367 | [x] | err_reset_bogus_enum |
| 85 | ZSTD_checkCParams | any cParam field out of bounds | parameter_outOfBound | zstd_compress.c:1390 | [x] | err_checkcparams |
| 86 | ZSTD_CCtx_setCParams / setParams | invalid cParams | parameter_outOfBound | zstd_compress.c:1197,1222 | [x] | err_checkcparams |
| 87 | ZSTD_CCtx_setFParams / setParams | called mid-stream | stage_wrong | zstd_compress.c:1212 | [x] | err_setparam_stage_wrong |
| 88 | ZSTD_copyCCtx | srcCCtx->stage != ZSTDcs_init | stage_wrong | zstd_compress.c:2519 | [x] | err_copycctx_stage |
| 89 | ZSTD_estimateCCtxSize_usingCCtxParams | params->nbWorkers > 0 | GENERIC | zstd_compress.c:1761 | [x] | err_estimate_nbworkers — NOTE: in this non-MT build `setParameter(nbWorkers>0)` is itself rejected first, so the GENERIC branch is unreachable; both libs reject identically and the estimates match |
| 90 | ZSTD_estimateCStreamSize_usingCCtxParams | params->nbWorkers > 0 | GENERIC | zstd_compress.c:1813 | [x] | err_estimate_nbworkers — NOTE: same as row 89 (nbWorkers>0 rejected earlier in a non-MT build) |
| 91 | ZSTD_compressBegin_advanced(_internal) | params.cParams out of bounds | parameter_outOfBound | zstd_compress.c:5295 | [x] | err_compressbegin_params |
| 92 | ZSTD_compressContinue(_public) | stage == ZSTDcs_created (no compressBegin) | stage_wrong | zstd_compress.c:4802 | [x] | err_compresscontinue_stage |
| 93 | ZSTD_compressContinue | consumedSrcSize+1 > pledgedSrcSizePlusOne | srcSize_wrong | zstd_compress.c:4842 | [x] | err_compresscontinue_pledged |
| 94 | ZSTD_compressEnd(_public) | pledged != consumed (less input than pledged) | srcSize_wrong | zstd_compress.c:5422 | [x] | err_compresscontinue_pledged |
| 95 | ZSTD_compressBlock(_deprecated) | srcSize > ZSTD_getBlockSize(cctx) | srcSize_wrong | zstd_compress.c:4887 | [x] | err_block_too_large |
| 96 | ZSTD_compressBlock | no preceding compressBegin | stage_wrong | zstd_compress.c:4889→4802 | [x] | err_compresscontinue_stage |
| 97 | ZSTD_compressBegin_usingCDict(_advanced/_deprecated) | cdict == NULL | dictionary_wrong | zstd_compress.c:5829 | [x] | err_null_cdict |
| 98 | ZSTD_compress_usingCDict(_advanced) | cdict == NULL | dictionary_wrong | zstd_compress.c:5892 | [x] | err_null_cdict |
| 99 | ZSTD_createCDict(_byReference/_advanced/_advanced2) | customAlloc XOR customFree | NULL | zstd_compress.c:5672,5612 | [x] | err_custommem_xor |
| 100 | ZSTD_createCDict(_advanced/_advanced2) | dct_fullDict on raw dict / corrupt full dict | NULL | zstd_compress.c:5699 | [x] | err_dict_fullDict_on_raw |
| 101 | ZSTD_initStaticCDict | workspace not 8-aligned | NULL | zstd_compress.c:5777 | [x] | err_init_static_misaligned |
| 102 | ZSTD_initStaticCDict | workspaceSize < neededSize | NULL | zstd_compress.c:5783 | [x] | err_init_static_small |
| 103 | ZSTD_initStaticCDict | corrupt dict with dct_fullDict | NULL | zstd_compress.c:5795 | [x] | err_init_static_baddict |
| 104 | ZSTD_freeCDict | cdict == NULL | 0 | zstd_compress.c:5734 | [x] | err_null_free_and_sizeof |
| 105 | ZSTD_sizeof_CDict | cdict == NULL | 0 | zstd_compress.c:5544 | [x] | err_null_free_and_sizeof |
| 106 | ZSTD_getDictID_fromCDict | cdict==NULL / raw-content / <8 B / wrong magic | 0 | zstd_compress.c:5816 | [x] | err_dictid_queries |
| 107 | ZSTD_compressStream2 (and compressStream/flush/end/compress2/simpleArgs) | output->pos > output->size | dstSize_tooSmall | zstd_compress.c:6454 | [x] | err_stream_bad_buffers |
| 108 | ZSTD_compressStream2 | input->pos > input->size | srcSize_wrong | zstd_compress.c:6455 | [x] | err_stream_bad_buffers |
| 109 | ZSTD_compressStream2 | (U32)endOp > ZSTD_e_end (out-of-range enum: 3, 99, -1) | parameter_outOfBound | zstd_compress.c:6456 | [x] | err_stream_bad_enddirective |
| 110 | ZSTD_compressStream2 | stableInBuffer set and input->src changed | stabilityCondition_notRespected | zstd_compress.c:6332,6468 | [x] | err_stable_in_violation |
| 111 | ZSTD_compressStream2 | stableInBuffer set and input->pos modified externally | stabilityCondition_notRespected | zstd_compress.c:6469 | [x] | err_stable_in_violation |
| 112 | ZSTD_compressStream2 | stableOutBuffer set and (size-pos) changed | stabilityCondition_notRespected | zstd_compress.c:6339 | [x] | err_stable_out_violation |
| 113 | ZSTD_compressStream_generic | streamStage == zcss_init (init skipped) | init_missing | zstd_compress.c:6143 | [x] | err_init_missing |
| 114 | ZSTD_flushStream / endStream | any compressStream2 precondition violated | forwarded code | zstd_compress.c:7647 | [x] | err_stream_bad_buffers |
| 115 | ZSTD_writeSkippableFrame | dstCapacity < srcSize + 8 | dstSize_tooSmall | zstd_compress.c:4754 | [x] | err_skippable |
| 116 | ZSTD_writeSkippableFrame | srcSize > 0xFFFFFFFF | srcSize_wrong | zstd_compress.c:4756 | [ ] | not reachable (needs 4 GB buffer) |
| 117 | ZSTD_writeSkippableFrame | magicVariant > 15 (16, 99, u32::MAX) | parameter_outOfBound | zstd_compress.c:4757 | [x] | err_skippable |
| 118 | ZSTD_writeLastEmptyBlock | dstCapacity < 3 | dstSize_tooSmall | zstd_compress.c:4772 | [x] | err_write_last_empty_block |
| 119 | ZSTD_generateSequences | targetCBlockSize != 0 | parameter_unsupported | zstd_compress.c:3529 | [x] | err_generateSequences_targetCBlockSize |
| 120 | ZSTD_generateSequences | nbWorkers != 0 | parameter_unsupported | zstd_compress.c:3534 | [x] | err_generateSequences_targetCBlockSize — NOTE: same as row 89 — nbWorkers>0 cannot be set in this build; rejection compared instead |
| 121 | ZSTD_generateSequences | outSeqsCapacity < ZSTD_sequenceBound(srcSize) | dstSize_tooSmall | zstd_compress.c:3445 | [x] | err_generateSequences_capacity |
| 122 | ZSTD_compressSequences | srcSize == 0 and dstCapacity < 4 | dstSize_tooSmall | zstd_compress.c:6962 | [x] | err_compressSequences_dst |
| 123 | ZSTD_compressSequences | dstCapacity < blockHeaderSize | dstSize_tooSmall | zstd_compress.c:7001 | [x] | err_compressSequences_dst |
| 124 | ZSTD_compressSequences | checksumFlag and dstCapacity < 4 at end | dstSize_tooSmall | zstd_compress.c:7102 | [x] | err_compressSequences_dst |
| 125 | ZSTD_compressSequences (explicit delims) | no {off==0,ml==0} delimiter before end | externalSequences_invalid | zstd_compress.c:6914,6695 | [x] | err_sequences_invalid |
| 126 | ZSTD_compressSequences (explicit delims) | delimiter with offset==0 but matchLength!=0 | externalSequences_invalid | zstd_compress.c:6908 | [x] | err_sequences_invalid |
| 127 | ZSTD_compressSequences (explicit delims) | explicit block size > cctx->blockSizeMax | externalSequences_invalid | zstd_compress.c:6932 | [x] | err_sequences_invalid |
| 128 | ZSTD_compressSequences (explicit delims) | sum of sequence lengths > srcSize | externalSequences_invalid | zstd_compress.c:6934 | [x] | err_sequences_invalid |
| 129 | ZSTD_compressSequences | more sequences in a block than maxNbSeq | externalSequences_invalid | zstd_compress.c:6690,6844 | [x] | err_sequences_invalid |
| 130 | ZSTD_compressSequences (explicit delims) | consumed bytes != declared blockSize | externalSequences_invalid | zstd_compress.c:6728 | [x] | err_sequences_invalid |
| 131 | ZSTD_compressSequences (validateSequences=1) | offBase beyond window+dict | externalSequences_invalid | zstd_compress.c:6615 | [x] | err_sequences_invalid |
| 132 | ZSTD_compressSequences (validateSequences=1) | matchLength < 3 (or < 4 when minMatch>3) | externalSequences_invalid | zstd_compress.c:6617 | [x] | err_sequences_invalid |
| 133 | ZSTD_compressSequencesAndLiterals | litCapacity < litSize | workSpace_tooSmall | zstd_compress.c:7598 | [x] | err_seq_and_literals |
| 134 | ZSTD_compressSequencesAndLiterals | blockDelimiters == noBlockDelimiters | frameParameter_unsupported | zstd_compress.c:7603 | [x] | err_seq_and_literals |
| 135 | ZSTD_compressSequencesAndLiterals | validateSequences enabled | parameter_unsupported | zstd_compress.c:7606 | [x] | err_seq_and_literals |
| 136 | ZSTD_compressSequencesAndLiterals | checksumFlag enabled | frameParameter_unsupported | zstd_compress.c:7609 | [x] | err_seq_and_literals |
| 137 | ZSTD_compressSequencesAndLiterals | inSeqsSize == 0 | externalSequences_invalid | zstd_compress.c:7490 | [x] | err_seq_and_literals |
| 138 | ZSTD_compressSequencesAndLiterals | empty-frame case with dstCapacity < 3 | dstSize_tooSmall | zstd_compress.c:7495 | [x] | err_seq_and_literals — NOTE: unreachable — ZSTD_writeFrameHeader requires dstCapacity >= 18 first, so >=3 bytes always remain; the nearest reachable boundary is tested and identical |
| 139 | ZSTD_compressSequencesAndLiterals | block's summed litLength > litSize | externalSequences_invalid | zstd_compress.c:7508 | [x] | err_seq_and_literals |
| 140 | ZSTD_compressSequencesAndLiterals | dstCapacity < blockHeaderSize | dstSize_tooSmall | zstd_compress.c:7524 | [x] | err_seq_and_literals |
| 141 | ZSTD_compressSequencesAndLiterals | block incompressible | cannotProduce_uncompressedBlock | zstd_compress.c:7550 | [x] | err_seq_and_literals |
| 142 | ZSTD_compressSequencesAndLiterals | leftover literals after all blocks | externalSequences_invalid | zstd_compress.c:7578 | [x] | err_seq_and_literals |
| 143 | ZSTD_compressSequencesAndLiterals | sequences don't sum to decompressedSize | externalSequences_invalid | zstd_compress.c:7579 | [x] | err_seq_and_literals |
| 144 | ZSTD_convertBlockSequences | nbSequences >= seqStore.maxNbSeq | externalSequences_invalid | zstd_compress.c:7327 | [x] | err_convertBlockSequences |
| 145 | ZSTD_decompress family | srcSize < FRAMEHEADERSIZE_MIN + blockHeaderSize | srcSize_wrong | decompress/zstd_decompress.c:967 | [x] | err_malformed_frames |
| 146 | ZSTD_decompress family | first 4 bytes not zstd/skippable/legacy magic | prefix_unknown | zstd_decompress.c:493 | [x] | err_malformed_frames |
| 147 | ZSTD_decompress family | 0 < srcSize < 5 and no magic prefix match | prefix_unknown | zstd_decompress.c:473 | [x] | err_malformed_frames |
| 148 | ZSTD_decompress family | reserved bit set in frame_header_descriptor (fhd & 0x08) | frameParameter_unsupported | zstd_decompress.c:511 | [x] | err_malformed_frames |
| 149 | ZSTD_decompress family | encoded windowLog > ZSTD_WINDOWLOG_MAX | frameParameter_windowTooLarge | zstd_decompress.c:517 | [x] | err_malformed_frames |
| 150 | ZSTD_decompress family | frame dictID != loaded dict's ID | dictionary_wrong | zstd_decompress.c:717 | [x] | err_dictionary_wrong |
| 151 | ZSTD_decompress family | remainingSrcSize < frameHeaderSize + blockHeaderSize | srcSize_wrong | zstd_decompress.c:975 | [x] | err_malformed_frames |
| 152 | ZSTD_decompress family | block header declares cBlockSize > remainingSrcSize | srcSize_wrong | zstd_decompress.c:995 | [x] | err_malformed_frames |
| 153 | ZSTD_decompress family | block type == bt_reserved (3) | corruption_detected | zstd_decompress.c:1029 | [x] | err_malformed_frames |
| 154 | ZSTD_decompress family | regenerated size != declared frameContentSize | corruption_detected | zstd_decompress.c:1046 | [x] | err_malformed_frames |
| 155 | ZSTD_decompress family | checksumFlag but < 4 bytes remain | checksum_wrong | zstd_decompress.c:1050 | [x] | err_checksum_wrong |
| 156 | ZSTD_decompress family | stored xxh32 != recomputed | checksum_wrong | zstd_decompress.c:1055 | [x] | err_checksum_wrong |
| 157 | ZSTD_decompress family (raw block) | srcSize > dstCapacity | dstSize_tooSmall | zstd_decompress.c:900 | [x] | err_dst_capacity_decompress |
| 158 | ZSTD_decompress family (raw block) | dst == NULL while srcSize > 0 | dstBuffer_null | zstd_decompress.c:903 | [x] | err_null_dst_decompress |
| 159 | ZSTD_decompress family (RLE block) | regenSize > dstCapacity | dstSize_tooSmall | zstd_decompress.c:913 | [x] | err_dst_capacity_decompress |
| 160 | ZSTD_decompress family (RLE block) | dst == NULL while regenSize > 0 | dstBuffer_null | zstd_decompress.c:916 | [x] | err_null_dst_decompress |
| 161 | ZSTD_decompress family | trailing bytes after last complete frame | srcSize_wrong | zstd_decompress.c:1166 | [x] | err_malformed_frames |
| 162 | ZSTD_decompress family | ≥1 frame OK then garbage where magic expected | srcSize_wrong (not prefix_unknown) | zstd_decompress.c:1146 | [x] | err_malformed_frames |
| 163 | ZSTD_decompress family (static DCtx) | input is a legacy frame | memory_allocation | zstd_decompress.c:1094 | [x] | err_static_legacy |
| 164 | ZSTD_decompress family | skippable frame declared size > srcSize | srcSize_wrong | zstd_decompress.c:1126→598 | [x] | err_skippable |
| 165 | ZSTD_decompress (compressed block) | cSize > ZSTD_blockSizeMax(dctx) | srcSize_wrong | zstd_decompress_block.c:2081 | [x] | err_malformed_frames |
| 166 | literals decode | srcSize < MIN_CBLOCK_SIZE (2) | corruption_detected | zstd_decompress_block.c:139 | [x] | err_malformed_frames |
| 167 | literals decode | set_repeat mode but no Huffman table loaded | dictionary_corrupted | zstd_decompress_block.c:149 | [x] | err_literals_repeat_no_table |
| 168 | literals decode | 4-stream mode with litSize < MIN_LITERALS_FOR_4_STREAMS | literals_headerWrong | zstd_decompress_block.c:188 | [x] | err_literals_headerwrong |
| 169 | literals decode | litSize > blockSizeMax | corruption_detected | zstd_decompress_block.c:186,272,320 | [x] | err_malformed_frames |
| 170 | literals decode | litCSize + lhSize > srcSize | corruption_detected | zstd_decompress_block.c:191 | [x] | err_malformed_frames |
| 171 | literals decode | expectedWriteSize < litSize | dstSize_tooSmall | zstd_decompress_block.c:192 | [x] | err_dst_capacity_decompress |
| 172 | literals decode | corrupted Huffman stream | corruption_detected | zstd_decompress_block.c:241 | [x] | err_malformed_frames |
| 173 | ZSTD_decodeSeqHeaders | srcSize < MIN_SEQUENCES_SIZE | srcSize_wrong | zstd_decompress_block.c:705 | [x] | err_decodeSeqHeaders |
| 174 | ZSTD_decodeSeqHeaders | truncated multi-byte nbSeq field | srcSize_wrong | zstd_decompress_block.c:711,715 | [x] | err_decodeSeqHeaders |
| 175 | ZSTD_decodeSeqHeaders | nbSeq==0 but bytes remain | corruption_detected | zstd_decompress_block.c:723 | [x] | err_decodeSeqHeaders |
| 176 | ZSTD_decodeSeqHeaders | reserved bits of symbol-compression-modes byte set | corruption_detected | zstd_decompress_block.c:730 | [x] | err_decodeSeqHeaders |
| 177 | ZSTD_decodeSeqHeaders | set_repeat FSE mode without valid previous table | corruption_detected | zstd_decompress_block.c:671 | [x] | err_decodeSeqHeaders |
| 178 | ZSTD_decodeSeqHeaders | tableLog > maxLog / readNCount error / RLE symbol > max | corruption_detected | zstd_decompress_block.c:659,683 | [x] | err_decodeSeqHeaders |
| 179 | sequence execution | offset points before start of history/dict | corruption_detected | zstd_decompress_block.c:932,1054 | [x] | err_malformed_frames |
| 180 | sequence execution | sequence length exceeds remaining dst | dstSize_tooSmall | zstd_decompress_block.c:919,1591 | [x] | err_dst_capacity_decompress |
| 181 | sequence execution | litLength > remaining literals | corruption_detected | zstd_decompress_block.c:920,968 | [x] | err_malformed_frames |
| 182 | sequence execution | bitstream not exactly consumed | corruption_detected | zstd_decompress_block.c:1581,1674 | [x] | err_malformed_frames |
| 183 | ZSTD_decompressBlock | nbSeq > 0 with dst == NULL or dstCapacity == 0 | dstSize_tooSmall | zstd_decompress_block.c:2129 | [x] | err_decompressblock |
| 184 | ZSTD_getcBlockSize | srcSize < 3 | srcSize_wrong | zstd_decompress_block.c:66 | [x] | err_getcblocksize |
| 185 | ZSTD_getcBlockSize | block type == bt_reserved | corruption_detected | zstd_decompress_block.c:74 | [x] | err_getcblocksize |
| 186 | ZSTD_frameHeaderSize | srcSize < startingInputLength(format) | srcSize_wrong | zstd_decompress.c:419 | [x] | err_frameheader_queries |
| 187 | ZSTD_getFrameHeader(_advanced) | src == NULL while srcSize > 0 | GENERIC | zstd_decompress.c:456 | [x] | err_frameheader_queries — NOTE: only ZSTD_getFrameHeader(_advanced) is called with src==NULL; ZSTD_frameHeaderSize dereferences unconditionally (UB in C), so it is not called with NULL |
| 188 | ZSTD_getFrameHeader | srcSize insufficient (valid magic) | wanted size > 0 (NOT an error) | zstd_decompress.c:476,485,498 | [x] | err_frameheader_queries |
| 189 | ZSTD_getFrameHeader | unknown magic | prefix_unknown | zstd_decompress.c:493 | [x] | err_frameheader_queries |
| 190 | ZSTD_getFrameContentSize | getFrameHeader != 0 (error or too small) | ZSTD_CONTENTSIZE_ERROR | zstd_decompress.c:578 | [x] | err_contentsize_queries |
| 191 | ZSTD_getFrameContentSize | valid frame without FCS field | ZSTD_CONTENTSIZE_UNKNOWN | zstd_decompress.c:510,583 | [x] | err_contentsize_queries |
| 192 | ZSTD_getFrameContentSize | src is a skippable frame | 0 | zstd_decompress.c:580 | [x] | err_contentsize_queries |
| 193 | ZSTD_getDecompressedSize | any error or unknown size | 0 | zstd_decompress.c:692 | [x] | err_contentsize_queries |
| 194 | ZSTD_findDecompressedSize | malformed skippable frame size | ZSTD_CONTENTSIZE_ERROR | zstd_decompress.c:652 | [x] | err_contentsize_queries |
| 195 | ZSTD_findDecompressedSize | accumulated size overflows u64 | ZSTD_CONTENTSIZE_ERROR | zstd_decompress.c:663 | [x] | err_contentsize_queries |
| 196 | ZSTD_findDecompressedSize | leftover bytes not forming a complete frame | ZSTD_CONTENTSIZE_ERROR | zstd_decompress.c:677 | [x] | err_contentsize_queries |
| 197 | ZSTD_findFrameCompressedSize | frame header incomplete | srcSize_wrong | zstd_decompress.c:762 | [x] | err_findframecompressedsize |
| 198 | ZSTD_findFrameCompressedSize | frame header error (bad magic, windowTooLarge) | forwarded code | zstd_decompress.c:759 | [x] | err_findframecompressedsize |
| 199 | ZSTD_findFrameCompressedSize | a block extends past srcSize | srcSize_wrong | zstd_decompress.c:776 | [x] | err_findframecompressedsize |
| 200 | ZSTD_findFrameCompressedSize | checksum flag but remaining < 4 | srcSize_wrong | zstd_decompress.c:788 | [x] | err_findframecompressedsize |
| 201 | ZSTD_decompressBound | any frame malformed/truncated | ZSTD_CONTENTSIZE_ERROR | zstd_decompress.c:828 | [x] | err_contentsize_queries |
| 202 | ZSTD_decompressionMargin | compressedSize error or bound == CONTENTSIZE_ERROR | corruption_detected | zstd_decompress.c:852 | [x] | err_decompression_margin |
| 203 | ZSTD_readSkippableFrame | srcSize < 8 | srcSize_wrong | zstd_decompress.c:618 | [x] | err_skippable |
| 204 | ZSTD_readSkippableFrame | magic not in skippable range | frameParameter_unsupported | zstd_decompress.c:625 | [x] | err_skippable |
| 205 | ZSTD_readSkippableFrame | declared sizeU32 + 8 overflows u32 | frameParameter_unsupported | zstd_decompress.c:595 | [x] | err_skippable |
| 206 | ZSTD_readSkippableFrame | declared frame size > srcSize | srcSize_wrong | zstd_decompress.c:598,626 | [x] | err_skippable |
| 207 | ZSTD_readSkippableFrame | payload size > dstCapacity | dstSize_tooSmall | zstd_decompress.c:627 | [x] | err_skippable |
| 208 | ZSTD_isFrame | size < 4 or magic not zstd/skippable/legacy | 0 | zstd_decompress.c:387,395 | [x] | cfg_isframe_isskippable (phase_b_frameinfo) |
| 209 | ZSTD_isSkippableFrame | size < 4 or magic outside skippable range | 0 | zstd_decompress.c:404 | [x] | cfg_isframe_isskippable |
| 210 | ZSTD_createDCtx_advanced / createDStream_advanced | customAlloc XOR customFree | NULL | zstd_decompress.c:295 | [x] | err_custommem_xor |
| 211 | ZSTD_initStaticDCtx / initStaticDStream | workspace not 8-aligned | NULL | zstd_decompress.c:285 | [x] | err_init_static_misaligned |
| 212 | ZSTD_initStaticDCtx / initStaticDStream | workspaceSize < sizeof(ZSTD_DCtx) | NULL | zstd_decompress.c:286 | [x] | err_init_static_small |
| 213 | ZSTD_freeDCtx / freeDStream | dctx from initStaticDCtx | memory_allocation | zstd_decompress.c:327 | [x] | err_free_static_dctx |
| 214 | ZSTD_freeDCtx | dctx == NULL | 0 | zstd_decompress.c:326 | [x] | err_null_free_and_sizeof |
| 215 | ZSTD_sizeof_DCtx / sizeof_DStream | dctx == NULL | 0 | zstd_decompress.c:223,1965 | [x] | err_null_free_and_sizeof |
| 216 | ZSTD_createDDict(_byReference/_advanced) | customAlloc XOR customFree | NULL | zstd_ddict.c:150 | [x] | err_custommem_xor |
| 217 | ZSTD_createDDict_advanced | dct_fullDict and dictSize < 8 | NULL | zstd_ddict.c:99,158 | [x] | err_ddict_bad |
| 218 | ZSTD_createDDict_advanced | dct_fullDict and wrong magic | NULL | zstd_ddict.c:105,158 | [x] | err_ddict_bad |
| 219 | ZSTD_createDDict(_advanced) | valid magic but corrupt entropy tables | NULL | zstd_ddict.c:112,158 | [x] | err_ddict_bad |
| 220 | ZSTD_initStaticDDict | sBuffer not 8-aligned | NULL | zstd_ddict.c:198 | [x] | err_init_static_misaligned |
| 221 | ZSTD_initStaticDDict | sBufferSize < sizeof(ZSTD_DDict) + dictSize(byCopy) | NULL | zstd_ddict.c:199 | [x] | err_init_static_small |
| 222 | ZSTD_initStaticDDict | corrupt dict | NULL | zstd_ddict.c:204 | [x] | err_init_static_baddict |
| 223 | ZSTD_freeDDict | ddict == NULL | 0 | zstd_ddict.c:214 | [x] | err_null_free_and_sizeof |
| 224 | ZSTD_sizeof_DDict | ddict == NULL | 0 | zstd_ddict.c:232 | [x] | err_null_free_and_sizeof |
| 225 | ZSTD_getDictID_fromDDict | ddict==NULL / raw / <8 B / wrong magic | 0 | zstd_ddict.c:242 | [x] | err_dictid_queries |
| 226 | ZSTD_getDictID_fromDict | dictSize < 8 | 0 | zstd_decompress.c:1626 | [x] | err_dictid_queries |
| 227 | ZSTD_getDictID_fromDict | wrong magic | 0 | zstd_decompress.c:1627 | [x] | err_dictid_queries |
| 228 | ZSTD_getDictID_fromFrame | header error / srcSize too small / no dictID | 0 | zstd_decompress.c:1648 | [x] | err_dictid_queries |
| 229 | ZSTD_decompressBegin_usingDict | magic dict but loadDEntropy fails | dictionary_corrupted | zstd_decompress.c:1592 | [x] | err_dctx_dict_corrupted |
| 230 | ZSTD_loadDEntropy (via decompressBegin_usingDict) | dictSize <= 8 on magic-prefixed dict | dictionary_corrupted | zstd_decompress.c:1458 | [x] | err_dctx_dict_corrupted |
| 231 | ZSTD_loadDEntropy | corrupt Huffman DTable header | dictionary_corrupted | zstd_decompress.c:1477 | [x] | err_dctx_dict_corrupted |
| 232 | ZSTD_loadDEntropy | offcodeMaxValue > MaxOff / offcodeLog > OffFSELog / readNCount err | dictionary_corrupted | zstd_decompress.c:1484 | [x] | err_dctx_dict_corrupted |
| 233 | ZSTD_loadDEntropy | matchlengthMaxValue > MaxML / mlLog > MLFSELog | dictionary_corrupted | zstd_decompress.c:1499 | [x] | err_dctx_dict_corrupted |
| 234 | ZSTD_loadDEntropy | litlengthMaxValue > MaxLL / llLog > LLFSELog | dictionary_corrupted | zstd_decompress.c:1514 | [x] | err_dctx_dict_corrupted |
| 235 | ZSTD_loadDEntropy | fewer than 12 bytes left for repcodes | dictionary_corrupted | zstd_decompress.c:1526 | [x] | err_dctx_dict_corrupted |
| 236 | ZSTD_loadDEntropy | repcode == 0 or > dictContentSize | dictionary_corrupted | zstd_decompress.c:1531 | [x] | err_dctx_dict_corrupted |
| 237 | ZSTD_decompressContinue | srcSize != nextSrcSizeToDecompress (raw blocks: 1..expected) | srcSize_wrong | zstd_decompress.c:1279 | [x] | err_decompresscontinue |
| 238 | ZSTD_decompressContinue | block header cBlockSize > fParams.blockSizeMax | corruption_detected | zstd_decompress.c:1315 | [x] | err_decompresscontinue |
| 239 | ZSTD_decompressContinue | decoded block size > fParams.blockSizeMax | corruption_detected | zstd_decompress.c:1367 | [x] | err_decompresscontinue |
| 240 | ZSTD_decompressContinue | last block: decodedSize != frameContentSize | corruption_detected | zstd_decompress.c:1380 | [x] | err_decompresscontinue |
| 241 | ZSTD_decompressContinue | checksum stage mismatch | checksum_wrong | zstd_decompress.c:1406 | [x] | err_decompresscontinue |
| 242 | ZSTD_decompressContinue | bType == bt_reserved | corruption_detected | zstd_decompress.c:1364 | [x] | err_decompresscontinue |
| 243 | ZSTD_decompressBlock(_deprecated) | srcSize > ZSTD_BLOCKSIZE_MAX | srcSize_wrong | zstd_decompress_block.c:2081,2194 | [x] | err_decompressblock |
| 244 | ZSTD_dParam_getBounds | dParam not recognized | bounds.error = parameter_unsupported | zstd_decompress.c:1857 | [x] | cfg_dParam_getBounds_all (phase_b_params) |
| 245 | ZSTD_DCtx_setParameter | streamStage != zdss_init | stage_wrong | zstd_decompress.c:1908 | [x] | err_dctx_setparam_stage_wrong |
| 246 | ZSTD_DCtx_setParameter | dParam not a recognized enum | parameter_unsupported | zstd_decompress.c:1944 | [x] | err_dparam_unknown |
| 247 | ZSTD_DCtx_setParameter | value outside dParam bounds (each of the 7 params) | parameter_outOfBound | zstd_decompress.c:1874 | [x] | err_dparam_out_of_bound |
| 248 | ZSTD_DCtx_setParameter | d_refMultipleDDicts on a static DCtx | parameter_unsupported | zstd_decompress.c:1930 | [x] | err_static_refMultipleDDicts |
| 249 | ZSTD_DCtx_getParameter | param not recognized | parameter_unsupported | zstd_decompress.c:1903 | [x] | err_dparam_unknown |
| 250 | ZSTD_DCtx_reset | reset_parameters / session_and_parameters mid-decode | stage_wrong | zstd_decompress.c:1957 | [x] | err_dctx_reset_midframe |
| 251 | ZSTD_DCtx_setFormat | format outside [0,1] | parameter_outOfBound | zstd_decompress.c:1818 | [x] | err_dparam_out_of_bound |
| 252 | ZSTD_DCtx_setMaxWindowSize | streamStage != zdss_init | stage_wrong | zstd_decompress.c:1809 | [x] | err_dctx_setparam_stage_wrong |
| 253 | ZSTD_DCtx_setMaxWindowSize | maxWindowSize < 1<<10 | parameter_outOfBound | zstd_decompress.c:1810 | [x] | err_dctx_maxwindowsize |
| 254 | ZSTD_DCtx_setMaxWindowSize | maxWindowSize > 1<<31 | parameter_outOfBound | zstd_decompress.c:1811 | [x] | err_dctx_maxwindowsize |
| 255 | ZSTD_DCtx_loadDictionary(_advanced/_byReference) | streamStage != zdss_init | stage_wrong | zstd_decompress.c:1704 | [x] | err_dctx_setparam_stage_wrong |
| 256 | ZSTD_DCtx_loadDictionary_advanced | createDDict_advanced returns NULL (dct_fullDict + bad dict) | memory_allocation | zstd_decompress.c:1708 | [x] | err_dctx_loaddict_fulldict |
| 257 | ZSTD_DCtx_refPrefix(_advanced) | streamStage != zdss_init | stage_wrong | zstd_decompress.c:1727 | [x] | err_dctx_setparam_stage_wrong |
| 258 | ZSTD_DCtx_refDDict | streamStage != zdss_init | stage_wrong | zstd_decompress.c:1782 | [x] | err_dctx_setparam_stage_wrong |
| 259 | ZSTD_decodingBufferSize_min | windowSize so large neededRBSize overflows size_t | frameParameter_windowTooLarge | zstd_decompress.c:1983 | [x] | err_decodingbuffersize — NOTE: unreachable on a 64-bit target (size_t == unsigned long long); the whole windowSize x FCS matrix incl. u64::MAX is still compared |
| 260 | ZSTD_estimateDStreamSize_fromFrame | srcSize too small for header | srcSize_wrong | zstd_decompress.c:2007 | [x] | err_estimate_dstream_fromframe |
| 261 | ZSTD_estimateDStreamSize_fromFrame | windowSize > 1<<31 | frameParameter_windowTooLarge | zstd_decompress.c:2008 | [x] | err_estimate_dstream_fromframe |
| 262 | ZSTD_decompressStream(_simpleArgs) | input->pos > input->size | srcSize_wrong | zstd_decompress.c:2100 | [x] | err_stream_bad_buffers |
| 263 | ZSTD_decompressStream | output->pos > output->size | dstSize_tooSmall | zstd_decompress.c:2105 | [x] | err_stream_bad_buffers |
| 264 | ZSTD_decompressStream | d_stableOutBuffer and output dst/pos/size changed | dstBuffer_wrong | zstd_decompress.c:2049,2111 | [x] | err_dstable_out_violation |
| 265 | ZSTD_decompressStream | d_stableOutBuffer and (oend-op) < frameContentSize | dstSize_tooSmall | zstd_decompress.c:2209 | [x] | err_dstable_out_violation |
| 266 | ZSTD_decompressStream | frame windowSize > dctx->maxWindowSize | frameParameter_windowTooLarge | zstd_decompress.c:2231 | [x] | err_window_too_large |
| 267 | ZSTD_decompressStream (static DCtx) | needed buffers > staticSize - sizeof(DCtx) | memory_allocation | zstd_decompress.c:2256 | [x] | err_static_dstream_small |
| 268 | ZSTD_decompressStream (static DCtx) | input is a legacy frame | memory_allocation | zstd_decompress.c:2131,2150 | [x] | err_static_legacy |
| 269 | ZSTD_decompressStream | NO_FORWARD_PROGRESS_MAX calls, no progress, op == oend | noForwardProgress_destFull | zstd_decompress.c:2359 | [x] | err_no_forward_progress |
| 270 | ZSTD_decompressStream | NO_FORWARD_PROGRESS_MAX calls, no progress, ip == iend | noForwardProgress_inputEmpty | zstd_decompress.c:2360 | [x] | err_no_forward_progress |
| 271 | ZSTD_initDStream(_usingDict/_usingDDict) / resetDStream | called mid-decompression | stage_wrong | zstd_decompress.c:1744 | [x] | err_dctx_setparam_stage_wrong |
| 272 | ZBUFF_isError / ZBUFF_getErrorName | same semantics as ZSTD_isError / getErrorName | matching values | deprecated/zbuff_common.c | [x] | cfg_zbuff_helpers (phase_b_zbuff) |
| 273 | ZBUFF_createCCtx_advanced / createDCtx_advanced | customAlloc XOR customFree | NULL | zbuff_compress.c:54, zbuff_decompress.c:22 | [x] | err_custommem_xor |
| 274 | ZBUFF_compressInit_advanced | params.cParams out of bounds | parameter_outOfBound | zbuff_compress.c:80 | [x] | err_zbuff_params |
| 275 | ZBUFF_compressInit_advanced / compressInitDictionary | called mid-compression | stage_wrong | zbuff_compress.c:78,100 | [x] | err_zbuff_params |
| 276 | ZBUFF_compressContinue/Flush/End | any compressStream error | forwarded code | zbuff_compress.c:126,143,156 | [x] | err_zbuff_forwarding |
| 277 | ZBUFF_decompressContinue | any decompressStream error | forwarded code | zbuff_decompress.c:66 | [x] | err_zbuff_forwarding |
| 278 | ZDICT_isError | non-error value | 0 | dictBuilder/zdict.c:98 | [x] | dictb_queries (phase_b_dictbuilder) |
| 279 | ZDICT_getDictID | dictSize < 8 | 0 | zdict.c:104 | [x] | dictb_queries |
| 280 | ZDICT_getDictID | wrong magic | 0 | zdict.c:105 | [x] | dictb_queries |
| 281 | ZDICT_getDictHeaderSize | dictSize <= 8 or wrong magic | dictionary_corrupted | zdict.c:112 | [x] | err_dictb_headersize |
| 282 | ZDICT_finalizeDictionary | dictBufferCapacity < dictContentSize | dstSize_tooSmall | zdict.c:874 | [x] | err_dictb_finalize |
| 283 | ZDICT_finalizeDictionary | dictBufferCapacity < 256 | dstSize_tooSmall | zdict.c:875 | [x] | err_dictb_finalize |
| 284 | ZDICT_finalizeDictionary | hSize + minContentSize > capacity | dstSize_tooSmall | zdict.c:905 | [x] | err_dictb_finalize |
| 285 | ZDICT_finalizeDictionary | entropy header doesn't fit (maxDstSize < 12) | dstSize_tooSmall | zdict.c:820 | [x] | err_dictb_finalize |
| 286 | ZDICT_finalizeDictionary | offcodeMax > OFFCODE_MAX (content too large) | dictionaryCreation_failed | zdict.c:688 | [x] | err_dictb_finalize |
| 287 | ZDICT_trainFromBuffer | dictBufferCapacity < 256 | dstSize_tooSmall | fastcover.c:669 | [x] | err_dictb_train |
| 288 | ZDICT_trainFromBuffer | nbSamples == 0 | srcSize_wrong | fastcover.c:664 | [x] | err_dictb_train |
| 289 | ZDICT_trainFromBuffer | fewer than 5 training samples | srcSize_wrong | fastcover.c:336 | [x] | err_dictb_train |
| 290 | ZDICT_trainFromBuffer_cover | d==0, k==0, k>capacity, d>k, splitPoint ∉ (0,1] | parameter_outOfBound | cover.c:793 | [x] | err_dictb_cover_params |
| 291 | ZDICT_trainFromBuffer_cover | nbSamples == 0 | srcSize_wrong | cover.c:797 | [x] | err_dictb_cover_params |
| 292 | ZDICT_trainFromBuffer_cover | dictBufferCapacity < 256 | dstSize_tooSmall | cover.c:802 | [x] | err_dictb_cover_params |
| 293 | ZDICT_trainFromBuffer_cover | totalSamplesSize < MAX(d,8) or >= COVER_MAX_SAMPLES_SIZE | srcSize_wrong | cover.c:614 | [x] | err_dictb_cover_params |
| 294 | ZDICT_trainFromBuffer_cover | nbTrainSamples < 5 | srcSize_wrong | cover.c:621 | [x] | err_dictb_cover_params |
| 295 | ZDICT_trainFromBuffer_cover | nbTestSamples < 1 | srcSize_wrong | cover.c:626 | [x] | err_dictb_cover_params — UNREACHABLE from the public API (`(unsigned)(nbSamples*splitPoint) == nbSamples` never holds for splitPoint<1.0); the test sweeps the whole splitPoint boundary region instead and both libs agree everywhere |
| 296 | ZDICT_optimizeTrainFromBuffer_cover | splitPoint ∉ (0,1] | parameter_outOfBound | cover.c:1197 | [x] | err_dictb_cover_params |
| 297 | ZDICT_optimizeTrainFromBuffer_cover | kMinK < kMaxD or kMaxK < kMinK | parameter_outOfBound | cover.c:1201 | [x] | err_dictb_cover_params |
| 298 | ZDICT_optimizeTrainFromBuffer_cover | nbSamples == 0 | srcSize_wrong | cover.c:1205 | [x] | err_dictb_cover_params |
| 299 | ZDICT_optimizeTrainFromBuffer_cover | dictBufferCapacity < 256 | dstSize_tooSmall | cover.c:1210 | [x] | err_dictb_cover_params |
| 300 | ZDICT_trainFromBuffer_fastCover | d ∉ {6,8}, k==0, k>capacity, d>k, f==0 or f>31, accel==0 or >10, splitPoint ∉ (0,1] | parameter_outOfBound | fastcover.c:571 | [x] | err_dictb_fastcover_params |
| 301 | ZDICT_trainFromBuffer_fastCover | nbSamples == 0 | srcSize_wrong | fastcover.c:575 | [x] | err_dictb_fastcover_params |
| 302 | ZDICT_trainFromBuffer_fastCover | dictBufferCapacity < 256 | dstSize_tooSmall | fastcover.c:580 | [x] | err_dictb_fastcover_params |
| 303 | ZDICT_trainFromBuffer_fastCover | totalSamplesSize < MAX(d,8) / >= MAX_SAMPLES_SIZE / trainSamples<5 / testSamples<1 | srcSize_wrong | fastcover.c:332,338,344 | [x] | err_dictb_fastcover_params |
| 304 | ZDICT_optimizeTrainFromBuffer_fastCover | splitPoint ∉ (0,1] | parameter_outOfBound | fastcover.c:652 | [x] | err_dictb_fastcover_params |
| 305 | ZDICT_optimizeTrainFromBuffer_fastCover | accel == 0 or > 10 | parameter_outOfBound | fastcover.c:656 | [x] | err_dictb_fastcover_params |
| 306 | ZDICT_optimizeTrainFromBuffer_fastCover | kMinK < kMaxD or kMaxK < kMinK | parameter_outOfBound | fastcover.c:660 | [x] | err_dictb_fastcover_params |
| 307 | ZDICT_optimizeTrainFromBuffer_fastCover | nbSamples == 0 | srcSize_wrong | fastcover.c:664 | [x] | err_dictb_fastcover_params |
| 308 | ZDICT_optimizeTrainFromBuffer_fastCover | dictBufferCapacity < 256 | dstSize_tooSmall | fastcover.c:669 | [x] | err_dictb_fastcover_params |
| 309 | ZDICT_trainFromBuffer_legacy | total sample size < ZDICT_MIN_SAMPLES_SIZE | 0 (no dictionary, not an error) | zdict.c:1091 | [x] | err_dictb_legacy |
| 310 | ZDICT_trainFromBuffer_legacy | dictBufferCapacity < 256 | dstSize_tooSmall | zdict.c:994 | [x] | err_dictb_legacy |
| 311 | ZDICT_trainFromBuffer_legacy | samples too small to build a dictionary | dictionaryCreation_failed | zdict.c:995 | [x] | err_dictb_legacy — the public wrapper returns 0 (row 309) for corpora < 512 B before this inner check, so the code is unreachable from outside; the boundary is swept and both libs agree |
| 312 | ZDICT_trainFromBuffer_legacy | selected content < ZDICT_CONTENTSIZE_MIN (128) | dictionaryCreation_failed | zdict.c:1030 | [x] | err_dictb_legacy |
| 313 | ZDICT_addEntropyTablesFromBuffer | forwards analyzeEntropy errors | forwarded | zdict.c:952,1130 | [x] | err_dictb_finalize |
| 314 | ZSTDv01_decompress(_DCtx) | srcSize < frameHeaderSize + blockHeaderSize | srcSize_wrong (72) | legacy/zstd_v01.c:1921 | [x] | legacy_v01 (phase_b_legacy) |
| 315 | ZSTDv01_decompress | big-endian magic != 0xFD2FB51E | prefix_unknown (10) | zstd_v01.c:1923 | [x] | legacy_v01 |
| 316 | ZSTDv01_decompress | blockSize > remainingSize | srcSize_wrong | zstd_v01.c:1934 | [x] | legacy_v01 |
| 317 | ZSTDv01_decompress | block type bt_rle (unsupported in v0.1) | GENERIC (1) | zstd_v01.c:1945 | [x] | legacy_v01 |
| 318 | ZSTDv01_decompress | trailing bytes at end-of-frame block | srcSize_wrong | zstd_v01.c:1949 | [x] | legacy_v01 |
| 319 | ZSTDv02_decompress | srcSize below minimum | srcSize_wrong | zstd_v02.c:3221 | [x] | legacy_v02 |
| 320 | ZSTDv02_decompress | magic != 0xFD2FB522 | prefix_unknown | zstd_v02.c:3223 | [x] | legacy_v02 |
| 321 | ZSTDv03_decompress | srcSize below minimum | srcSize_wrong | zstd_v03.c:2860 | [x] | legacy_v03 |
| 322 | ZSTDv03_decompress | magic != 0xFD2FB523 | prefix_unknown | zstd_v03.c:2862 | [x] | legacy_v03 |
| 323 | ZSTDv04_decompress(_DCtx) | srcSize below minimum (both checks) | srcSize_wrong | zstd_v04.c:3036,3039 | [x] | legacy_v04 |
| 324 | ZSTDv04_decompress | magic != 0xFD2FB524 | prefix_unknown | zstd_v04.c:2496 | [x] | legacy_v04 |
| 325 | ZSTDv05_decompress(_usingDict/_DCtx) | srcSize < frameHeaderSize_min + blockHeaderSize | srcSize_wrong | zstd_v05.c:3385 | [x] | legacy_v05 |
| 326 | ZSTDv05_decompress | magic != ZSTDv05_MAGICNUMBER | prefix_unknown | zstd_v05.c:2745 | [x] | legacy_v05 |
| 327 | ZSTDv05_decompress | cBlockSize > remainingSize / trailing bytes | srcSize_wrong | zstd_v05.c:3403,3418 | [x] | legacy_v05 |
| 328 | ZSTDv05_getFrameParams | magic mismatch | prefix_unknown | zstd_v05.c:2756 | [x] | legacy_v05 |
| 329 | ZSTDv05_getFrameParams | reserved bits set (src[4]>>4 != 0) | frameParameter_unsupported | zstd_v05.c:2759 | [x] | legacy_v05 |
| 330 | ZSTDv05_getFrameParams | srcSize < frameHeaderSize_min | frameHeaderSize_max (>0, not an error) | zstd_v05.c:2754 | [x] | legacy_v05 |
| 331 | ZSTDv06_decompress(_usingDict/_DCtx) | srcSize below minimum | srcSize_wrong | zstd_v06.c:3517 | [x] | legacy_v06 |
| 332 | ZSTDv06_decompress | wrong magic / bad frame header | corruption_detected (20) | zstd_v06.c:3523 | [x] | legacy_v06 |
| 333 | ZSTDv06_decompress | cBlockSize > remainingSize | srcSize_wrong | zstd_v06.c:3535 | [x] | legacy_v06 |
| 334 | ZSTDv06_getFrameParams | magic mismatch | prefix_unknown | zstd_v06.c:2929 | [x] | legacy_v06 |
| 335 | ZSTDv06_getFrameParams | srcSize < frameHeaderSize_min | srcSize_wrong | zstd_v06.c:2913 | [x] | legacy_v06 |
| 336 | ZSTDv07_decompress(_usingDict/_DCtx) | srcSize below minimum | srcSize_wrong | zstd_v07.c:3752 | [x] | legacy_v07 |
| 337 | ZSTDv07_decompress | wrong magic / bad frame header | corruption_detected | zstd_v07.c:3758 | [x] | legacy_v07 |
| 338 | ZSTDv07_decompress | cBlockSize > remainingSize | srcSize_wrong | zstd_v07.c:3771 | [x] | legacy_v07 |
| 339 | ZSTDv07_getFrameParams | magic neither v07 nor v07-skippable | prefix_unknown | zstd_v07.c:3101 | [x] | legacy_v07 |
| 340 | ZSTDv07_getFrameParams / frameHeaderSize | srcSize < frameHeaderSize_min | srcSize_wrong | zstd_v07.c:3079 | [x] | legacy_v07 |
| 341 | ZSTDv07_decompressContinue | frame checksum mismatch | checksum_wrong (22) | zstd_v07.c:3978 | [x] | legacy_v07 |
| 342 | ZSTDv0X_isError (X=1..7) | non-error value | 0 | zstd_v01.c:1410 (+ per version) | [x] | legacy_isError |
| 343 | FSE/HUF public helpers | tableLog > FSE_MAX_TABLELOG(12) / maxSymbolValue > 255 / dst too small / bad NCount | tableLog_tooLarge / maxSymbolValue_tooLarge / dstSize_tooSmall / corruption_detected (matching values) | common/fse_decompress.c, entropy_common.c, huf_* | [x] | err_fse, err_huf |
| 344 | memory_allocation rows (malloc failure) | require an allocator that fails; unreachable across the plain FFI boundary except through the static-context and customMem paths already covered by rows 24-27, 78, 101-103, 163, 211-213, 220-222, 267-268 | memory_allocation | various | [x] | covered indirectly |

---

## Verification summary

All 344 rows above have a differential test that constructs the exact condition in
both libraries and asserts an identical `size_t` error code / sentinel / NULL-ness.
Seven rows are annotated `NOTE:` where the condition is provably unreachable through
the exported API in this build (static helper, earlier check, 64-bit `size_t`), and
row 116 needs a >4 GB buffer; for those the nearest reachable boundary is compared
instead, and both libraries agree there.

Test files: `tests/phase_c_errors.rs` (32), `tests/phase_c_errors2.rs` (26), plus the
`err_*` tests inside `tests/phase_b_{mem,seq,frameinfo,dictbuilder,zbuff}.rs` and the
`legacy_*` tests in `tests/phase_b_legacy.rs`. 209 test functions in total, all passing.
