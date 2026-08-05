# Compress cluster contract

Shared foundation modules ALREADY EXIST and compile:
- `crate::compress::zstd_cwksp::*` — ZSTD_cwksp arena allocator (ZSTD_cwksp struct,
  ZSTD_cwksp_init/create/free/move/clear, reserve_object/table/aligned64/aligned_init_once/buffer,
  align, alloc_size, aligned64_alloc_size, available_space, sizeof, used, check_available/too_large/wasteful,
  mark_tables_dirty/clean, clean_tables, clear_tables, bump_oversized_duration, estimated_space_within_bounds,
  slack_space_required, bytes_to_align_ptr, initialAllocStart, owns_buffer, reserve_failed, phase enum consts
  ZSTD_cwksp_alloc_objects/aligned_init_once/aligned/buffers, ZSTD_cwksp_dynamic_alloc/static_alloc).
- `crate::compress::zstd_compress_internal::*` — ALL shared compress types + inline helpers.
  Includes ZSTD_CCtx_s (full struct, typedef ZSTD_CCtx), ZSTD_CCtx_params_s (typedef ZSTD_CCtx_params),
  ZSTD_MatchState_t, ZSTD_blockState_t, ZSTD_compressedBlockState_t, ZSTD_window_t, SeqDef, SeqStore_t,
  ZSTD_entropyCTables_t (huf/fse CTables), ldmState_t/ldmParams_t, optState_t, ZSTD_optimal_t, ZSTD_match_t,
  rawSeq, RawSeqStore_t, kNullRawSeqStore, SeqCollector, ZSTD_blockSplitCtx, ZSTD_hufCTablesMetadata_t,
  ZSTD_fseCTablesMetadata_t, ZSTD_entropyCTablesMetadata_t, ZSTD_localDict, ZSTD_prefixDict, ZSTD_SequenceLength.
  Enums as u32 consts: ZSTD_dictMode_e (ZSTD_noDict/extDict/dictMatchState/dedicatedDictSearch),
  ZSTD_CParamMode_e (ZSTD_cpm_*), ZSTD_dictTableLoadMethod_e (ZSTD_dtlm_fast/full), ZSTD_tableFillPurpose_e
  (ZSTD_tfp_forCCtx/forCDict), ZSTD_buffered_policy_e (ZSTDb_not_buffered/buffered),
  ZSTD_compressionStage_e (ZSTDcs_*), ZSTD_cStreamStage (zcss_*), ZSTD_longLengthType_e (ZSTD_llt_*),
  ZSTD_ParamSwitch_e (ZSTD_ps_auto=0/enable=1/disable=2), ZSTD_dictAttachPref_e, ZSTD_SequenceFormat_e.
  Inline helpers (all pub, crate-internal): ZSTD_getSequenceLength, ZSTD_storeSeq, ZSTD_updateRep,
  ZSTD_noCompressBlock/ZSTD_rleCompressBlock (check names), ZSTD_minGain, ZSTD_literalsCompressionIsDisabled,
  ZSTD_window_* (hasExtDict, isExtDict, needOverflowCorrection, correctOverflow, enforceMaxDist,
  updateSlidingWindow/ updateSlidingWindowSpeed, ZSTD_window_init, ZSTD_window_update, ZSTD_getLowestMatchIndex,
  ZSTD_getLowestPrefixIndex), ZSTD_matchState_dictMode, ZSTD_cParam_withinBounds, ZSTD_selectAddr, etc.
  Constants: kSearchStrength, HASH_READ_SIZE, ZSTD_DUBT_UNSORTED_MARK, ZSTD_WINDOW_START_INDEX,
  ZSTD_ROW_HASH_CACHE_SIZE, LDM_BATCH_SIZE, ENTROPY_WORKSPACE_SIZE, TMP_WORKSPACE_SIZE, ZSTD_OPT_SIZE, etc.

Common layer: crate::common::{error, mem, bits, bitstream, fse, huf_common, allocations, pool, xxhash,
zstd_internal, zstd_common}. Public types: crate::zstd_h::*.

Entropy encoders are exported C symbols you can CALL:
- crate::compress::hist (HIST_count_wksp, HIST_countFast_wksp, HIST_count, HIST_add, ...)
- crate::compress::fse_compress (FSE_compress_usingCTable, FSE_optimalTableLog, FSE_normalizeCount,
  FSE_writeNCount, FSE_buildCTable_wksp, FSE_NCountWriteBound, FSE_optimalTableLog_internal, ...)
- crate::compress::huf_compress (HUF_compress1X_repeat, HUF_compress4X_repeat, HUF_compress4X_usingCTable,
  HUF_optimalTableLog, HUF_writeCTable_wksp, HUF_estimateCompressedSize, HUF_readCTableHeader,
  HUF_getNbBitsFromCTable, HUF_buildCTable_wksp, ...)

Cross-file compress functions: exported as #[unsafe(no_mangle)] where global in C. To call a sibling
compress module's function, either `use crate::compress::<mod>::FN` or declare `extern "C"`.
Prefer crate paths for functions with complex private-type signatures within the same cluster —
but if the signature uses a type private to the sibling, use extern "C" with matching ABI types
from the shared internal module.

Build config: DYNAMIC_BMI2=0, ZSTD_MULTITHREAD NOT defined (single-thread; zstdmt is a thin wrapper),
ZSTD_TRACE=1 (trace hooks weak/undefined -> treat as no-ops, traceCtx is u64 field), no ASM,
ZSTD_LEGACY_SUPPORT=5, no sanitizers. LE 64-bit. Byte-identical output. No stubs. No bug fixes.
Preserve exact error-check order, wrapping arithmetic, and static-branch folding.
Use INCREMENTAL writing (Write header + `// __APPEND_HERE__`, Edit repeatedly <350 lines) to avoid timeouts.
Do NOT edit other files or c_src/. Do NOT add mod declarations (coordinator wires them).
