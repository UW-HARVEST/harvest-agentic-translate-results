//! Faithful translation of compress/zstd_compress.c — main compression entry points,
//! CCtx / CCtx_params / CDict lifecycle, parameter handling, streaming, sequence APIs.
//!
//! Build config: DYNAMIC_BMI2=0, ZSTD_MULTITHREAD undefined (single-thread),
//! ZSTD_TRACE=1 (trace callbacks are no-ops returning 0), ZSTD_LEGACY_SUPPORT=5,
//! no ASM, LE 64-bit. Byte-identical output. No stubs. No bug fixes.
#![allow(
    non_snake_case,
    non_camel_case_types,
    non_upper_case_globals,
    dead_code,
    unused_mut,
    unused_assignments,
    unused_parens,
    unused_variables
)]

use core::ffi::{c_char, c_int, c_uint, c_ulonglong, c_void};

use crate::common::allocations::{
    memcpy, memmove, memset, zstd_custom_calloc, zstd_custom_free, zstd_custom_malloc,
    ZSTD_customMem, ZSTD_defaultCMem,
};
use crate::common::bits::{highbit32 as ZSTD_highbit32, rotate_right_u64 as ZSTD_rotateRight_U64};
use crate::common::error::{code, err_get_error_code, err_is_error, error};
use crate::common::mem::{
    mem_32bits as MEM_32bits, mem_64bits as MEM_64bits, mem_read16 as MEM_read16,
    mem_read32 as MEM_read32,
    mem_read64 as MEM_read64, mem_read_le16 as MEM_readLE16, mem_read_le24 as MEM_readLE24,
    mem_read_le32 as MEM_readLE32, mem_read_le64 as MEM_readLE64, mem_read_st as MEM_readST,
    mem_write32 as MEM_write32, mem_write_le16 as MEM_writeLE16, mem_write_le24 as MEM_writeLE24,
    mem_write_le32 as MEM_writeLE32, mem_write_le64 as MEM_writeLE64, U16, U32, U64,
};
use crate::common::xxhash::{ZSTD_XXH64, ZSTD_XXH64_digest, ZSTD_XXH64_reset, ZSTD_XXH64_update};
use crate::common::zstd_internal::{
    repStartValue, zstd_cpu_supports_bmi2 as ZSTD_cpuSupportsBmi2, DefaultMaxOff, LLFSELog,
    LL_bits, LONGNBSEQ, MLFSELog, ML_bits, MaxLL, MaxML, MaxOff, MaxSeq, MINMATCH, OffFSELog,
    WILDCOPY_OVERLENGTH, ZSTD_REP_NUM, ZSTD_WINDOWLOG_ABSOLUTEMIN, ZSTD_blockHeaderSize,
};

use crate::zstd_h::*;

use crate::common::fse::{FSE_repeat_check, FSE_repeat_none, FSE_repeat_valid};
use crate::common::mem::S64;
use crate::compress::zstd_preSplit::ZSTD_splitBlock;
use crate::common::fse::{FSE_isError, FSE_readNCount};
use crate::common::huf_common::HUF_isError;
use crate::compress::fse_compress::FSE_buildCTable_wksp;
use crate::compress::huf_compress::HUF_readCTable;
use crate::compress::zstd_fast::ZSTD_fillHashTable;
use crate::compress::zstd_double_fast::ZSTD_fillDoubleHashTable;
use crate::compress::zstd_lazy::{
    ZSTD_dedicatedDictSearch_lazy_loadDictionary, ZSTD_insertAndFindFirstIndex, ZSTD_row_update,
};
use crate::compress::zstd_opt::ZSTD_updateTree;
use crate::compress::zstd_ldm::ZSTD_ldm_fillHashTable;
use crate::compress::zstd_compress_superblock::ZSTD_compressSuperBlock;
use crate::common::zstd_internal::{
    set_basic, set_compressed, set_repeat, set_rle, Litbits, MaxLit, ZSTD_BLOCKHEADERSIZE,
};

use crate::compress::zstd_compress_internal::*;
use crate::compress::zstd_cwksp::*;

use crate::compress::zstd_ldm::{
    ZSTD_ldm_adjustParameters, ZSTD_ldm_blockCompress, ZSTD_ldm_generateSequences,
    ZSTD_ldm_getMaxNbSeq, ZSTD_ldm_getTableSize, ZSTD_ldm_skipRawSeqStoreBytes,
    ZSTD_ldm_skipSequences,
};
use crate::common::zstd_internal::{
    bt_compressed, bt_raw, bt_rle, MIN_CBLOCK_SIZE,
};

use crate::common::fse::{FSE_CTable, FSE_repeat};
use crate::common::zstd_internal::{
    LL_defaultNorm, LL_defaultNormLog, ML_defaultNorm, ML_defaultNormLog, OF_defaultNorm,
    OF_defaultNormLog,
};
use crate::compress::hist::{HIST_countFast_wksp, HIST_count_wksp};
use crate::common::huf_common::{HUF_SYMBOLVALUE_MAX, HUF_flags_optimalDepth};
use crate::common::zstd_internal::LitHufLog;
use crate::compress::huf_compress::{
    HUF_buildCTable_wksp, HUF_estimateCompressedSize, HUF_optimalTableLog, HUF_validateCTable,
    HUF_writeCTable_wksp,
};
use crate::compress::zstd_fast::{
    ZSTD_compressBlock_fast, ZSTD_compressBlock_fast_dictMatchState, ZSTD_compressBlock_fast_extDict,
};
use crate::compress::zstd_double_fast::{
    ZSTD_compressBlock_doubleFast, ZSTD_compressBlock_doubleFast_dictMatchState,
    ZSTD_compressBlock_doubleFast_extDict,
};
use crate::compress::zstd_lazy::{
    ZSTD_compressBlock_btlazy2, ZSTD_compressBlock_btlazy2_dictMatchState,
    ZSTD_compressBlock_btlazy2_extDict, ZSTD_compressBlock_greedy,
    ZSTD_compressBlock_greedy_dedicatedDictSearch,
    ZSTD_compressBlock_greedy_dedicatedDictSearch_row, ZSTD_compressBlock_greedy_dictMatchState,
    ZSTD_compressBlock_greedy_dictMatchState_row, ZSTD_compressBlock_greedy_extDict,
    ZSTD_compressBlock_greedy_extDict_row, ZSTD_compressBlock_greedy_row, ZSTD_compressBlock_lazy,
    ZSTD_compressBlock_lazy2, ZSTD_compressBlock_lazy2_dedicatedDictSearch,
    ZSTD_compressBlock_lazy2_dedicatedDictSearch_row, ZSTD_compressBlock_lazy2_dictMatchState,
    ZSTD_compressBlock_lazy2_dictMatchState_row, ZSTD_compressBlock_lazy2_extDict,
    ZSTD_compressBlock_lazy2_extDict_row, ZSTD_compressBlock_lazy2_row,
    ZSTD_compressBlock_lazy_dedicatedDictSearch,
    ZSTD_compressBlock_lazy_dedicatedDictSearch_row, ZSTD_compressBlock_lazy_dictMatchState,
    ZSTD_compressBlock_lazy_dictMatchState_row, ZSTD_compressBlock_lazy_extDict,
    ZSTD_compressBlock_lazy_extDict_row, ZSTD_compressBlock_lazy_row,
};
use crate::compress::zstd_opt::{
    ZSTD_compressBlock_btopt, ZSTD_compressBlock_btopt_dictMatchState,
    ZSTD_compressBlock_btopt_extDict, ZSTD_compressBlock_btultra,
    ZSTD_compressBlock_btultra2, ZSTD_compressBlock_btultra_dictMatchState,
    ZSTD_compressBlock_btultra_extDict,
};
use crate::compress::zstd_compress_literals::ZSTD_compressLiterals;
use crate::compress::zstd_compress_sequences::{
    ZSTD_buildCTable, ZSTD_crossEntropyCost, ZSTD_encodeSequences, ZSTD_fseBitCost,
    ZSTD_selectEncodingType, ZSTD_defaultAllowed, ZSTD_defaultDisallowed, ZSTD_DefaultPolicy_e,
};

// ---------------------------------------------------------------------------
// Local constants (from include/zstd.h and clevels.h; LE 64-bit)
// ---------------------------------------------------------------------------
const ZSTD_CLEVEL_DEFAULT: c_int = 3;
const ZSTD_MAX_CLEVEL: c_int = 22;
const ZSTD_NO_CLEVEL: c_int = 0;

const ZSTD_WINDOWLOG_MAX_64: c_int = 31;
const ZSTD_WINDOWLOG_MAX: c_int = ZSTD_WINDOWLOG_MAX_64;
const ZSTD_WINDOWLOG_MIN: c_int = 10;
const ZSTD_HASHLOG_MAX: c_int = if ZSTD_WINDOWLOG_MAX < 30 { ZSTD_WINDOWLOG_MAX } else { 30 };
const ZSTD_HASHLOG_MIN: c_int = 6;
const ZSTD_CHAINLOG_MAX_64: c_int = 30;
const ZSTD_CHAINLOG_MAX: c_int = ZSTD_CHAINLOG_MAX_64;
const ZSTD_CHAINLOG_MIN: c_int = ZSTD_HASHLOG_MIN;
const ZSTD_SEARCHLOG_MAX: c_int = ZSTD_WINDOWLOG_MAX - 1;
const ZSTD_SEARCHLOG_MIN: c_int = 1;
const ZSTD_MINMATCH_MAX: c_int = 7;
const ZSTD_MINMATCH_MIN: c_int = 3;
const ZSTD_TARGETLENGTH_MAX: c_int = ZSTD_BLOCKSIZE_MAX as c_int;
const ZSTD_TARGETLENGTH_MIN: c_int = 0;
const ZSTD_STRATEGY_MIN: c_int = ZSTD_fast;
const ZSTD_STRATEGY_MAX: c_int = ZSTD_btultra2;
const ZSTD_BLOCKSIZE_MAX_MIN: c_int = 1 << 10;
const ZSTD_OVERLAPLOG_MIN: c_int = 0;
const ZSTD_OVERLAPLOG_MAX: c_int = 9;
const ZSTD_LDM_HASHLOG_MIN: c_int = ZSTD_HASHLOG_MIN;
const ZSTD_LDM_HASHLOG_MAX: c_int = ZSTD_HASHLOG_MAX;
const ZSTD_LDM_MINMATCH_MIN: c_int = 4;
const ZSTD_LDM_MINMATCH_MAX: c_int = 4096;
const ZSTD_LDM_BUCKETSIZELOG_MIN: c_int = 1;
const ZSTD_LDM_BUCKETSIZELOG_MAX: c_int = 8;
const ZSTD_LDM_HASHRATELOG_MIN: c_int = 0;
const ZSTD_LDM_HASHRATELOG_MAX: c_int = ZSTD_WINDOWLOG_MAX - ZSTD_HASHLOG_MIN;
const ZSTD_TARGETCBLOCKSIZE_MIN: c_int = 1340;
const ZSTD_TARGETCBLOCKSIZE_MAX: c_int = ZSTD_BLOCKSIZE_MAX as c_int;
const ZSTD_SRCSIZEHINT_MIN: c_int = 0;
const ZSTD_SRCSIZEHINT_MAX: c_int = c_int::MAX;
const ZSTD_BLOCKSPLITTER_LEVEL_MAX: c_int = 6;

const ZSTD_MAX_INPUT_SIZE: u64 = 0xFF00FF00FF00FF00u64; /* size_t==8 */

const ZSTD_FRAMEHEADERSIZE_MAX: usize = 18;
const ZSTD_SKIPPABLEHEADERSIZE: usize = 8;

// ZSTD_c_* parameter values (public API) — used across this file.
const ZSTD_c_compressionLevel: ZSTD_cParameter = 100;
const ZSTD_c_windowLog: ZSTD_cParameter = 101;
const ZSTD_c_hashLog: ZSTD_cParameter = 102;
const ZSTD_c_chainLog: ZSTD_cParameter = 103;
const ZSTD_c_searchLog: ZSTD_cParameter = 104;
const ZSTD_c_minMatch: ZSTD_cParameter = 105;
const ZSTD_c_targetLength: ZSTD_cParameter = 106;
// ZSTD_c_strategy = 107 defined in internal module.
const ZSTD_c_targetCBlockSize: ZSTD_cParameter = 130;
const ZSTD_c_enableLongDistanceMatching: ZSTD_cParameter = 160;
const ZSTD_c_ldmHashLog: ZSTD_cParameter = 161;
const ZSTD_c_ldmMinMatch: ZSTD_cParameter = 162;
const ZSTD_c_ldmBucketSizeLog: ZSTD_cParameter = 163;
const ZSTD_c_ldmHashRateLog: ZSTD_cParameter = 164;
const ZSTD_c_contentSizeFlag: ZSTD_cParameter = 200;
const ZSTD_c_checksumFlag: ZSTD_cParameter = 201;
const ZSTD_c_dictIDFlag: ZSTD_cParameter = 202;
const ZSTD_c_nbWorkers: ZSTD_cParameter = 400;
const ZSTD_c_jobSize: ZSTD_cParameter = 401;
const ZSTD_c_overlapLog: ZSTD_cParameter = 402;
const ZSTD_c_experimentalParam1: ZSTD_cParameter = 500; /* rsyncable */
const ZSTD_c_experimentalParam2: ZSTD_cParameter = 10; /* format */
const ZSTD_c_experimentalParam3: ZSTD_cParameter = 1000; /* forceMaxWindow */
const ZSTD_c_experimentalParam4: ZSTD_cParameter = 1001; /* forceAttachDict */
const ZSTD_c_experimentalParam5: ZSTD_cParameter = 1002; /* literalCompressionMode */
const ZSTD_c_experimentalParam7: ZSTD_cParameter = 1004; /* srcSizeHint */
const ZSTD_c_experimentalParam8: ZSTD_cParameter = 1005; /* enableDedicatedDictSearch */
const ZSTD_c_experimentalParam9: ZSTD_cParameter = 1006; /* stableInBuffer */
const ZSTD_c_experimentalParam10: ZSTD_cParameter = 1007; /* stableOutBuffer */
const ZSTD_c_experimentalParam11: ZSTD_cParameter = 1008; /* blockDelimiters */
const ZSTD_c_experimentalParam12: ZSTD_cParameter = 1009; /* validateSequences */
const ZSTD_c_experimentalParam13: ZSTD_cParameter = 1010; /* splitAfterSequences */
const ZSTD_c_experimentalParam14: ZSTD_cParameter = 1011; /* useRowMatchFinder */
const ZSTD_c_experimentalParam15: ZSTD_cParameter = 1012; /* deterministicRefPrefix */
const ZSTD_c_experimentalParam16: ZSTD_cParameter = 1013; /* prefetchCDictTables */
const ZSTD_c_experimentalParam17: ZSTD_cParameter = 1014; /* enableSeqProducerFallback */
const ZSTD_c_experimentalParam18: ZSTD_cParameter = 1015; /* maxBlockSize */
const ZSTD_c_experimentalParam19: ZSTD_cParameter = 1016; /* repcodeResolution */
const ZSTD_c_experimentalParam20: ZSTD_cParameter = 1017; /* blockSplitterLevel */
// Aliases used in code below (map experimental params to friendly names).
const ZSTD_c_rsyncable: ZSTD_cParameter = ZSTD_c_experimentalParam1;
const ZSTD_c_format: ZSTD_cParameter = ZSTD_c_experimentalParam2;
const ZSTD_c_forceMaxWindow: ZSTD_cParameter = ZSTD_c_experimentalParam3;
const ZSTD_c_forceAttachDict: ZSTD_cParameter = ZSTD_c_experimentalParam4;
const ZSTD_c_literalCompressionMode: ZSTD_cParameter = ZSTD_c_experimentalParam5;
const ZSTD_c_srcSizeHint: ZSTD_cParameter = ZSTD_c_experimentalParam7;
const ZSTD_c_enableDedicatedDictSearch: ZSTD_cParameter = ZSTD_c_experimentalParam8;
const ZSTD_c_stableInBuffer: ZSTD_cParameter = ZSTD_c_experimentalParam9;
const ZSTD_c_stableOutBuffer: ZSTD_cParameter = ZSTD_c_experimentalParam10;
const ZSTD_c_blockDelimiters: ZSTD_cParameter = ZSTD_c_experimentalParam11;
const ZSTD_c_validateSequences: ZSTD_cParameter = ZSTD_c_experimentalParam12;
const ZSTD_c_splitAfterSequences: ZSTD_cParameter = ZSTD_c_experimentalParam13;
const ZSTD_c_useRowMatchFinder: ZSTD_cParameter = ZSTD_c_experimentalParam14;
const ZSTD_c_deterministicRefPrefix: ZSTD_cParameter = ZSTD_c_experimentalParam15;
const ZSTD_c_prefetchCDictTables: ZSTD_cParameter = ZSTD_c_experimentalParam16;
const ZSTD_c_enableSeqProducerFallback: ZSTD_cParameter = ZSTD_c_experimentalParam17;
const ZSTD_c_maxBlockSize: ZSTD_cParameter = ZSTD_c_experimentalParam18;
const ZSTD_c_repcodeResolution: ZSTD_cParameter = ZSTD_c_experimentalParam19;
const ZSTD_c_searchForExternalRepcodes: ZSTD_cParameter = ZSTD_c_experimentalParam19;
const ZSTD_c_blockSplitterLevel: ZSTD_cParameter = ZSTD_c_experimentalParam20;

const ZSTD_HASHLOG3_MAX: u32 = 17;
const ZSTD_ROW_HASH_TAG_BITS: u32 = 8;
const STREAM_ACCUMULATOR_MIN: u32 = 57; /* LE 64-bit */
const ZSTD_ROWSIZE: usize = 16;
const HUF_OPTIMAL_DEPTH_THRESHOLD: ZSTD_strategy = ZSTD_btultra;
const COMPRESS_LITERALS_SIZE_MIN: usize = 63;
const ZSTD_LAZY_DDSS_BUCKET_LOG: u32 = 2;
const ZSTD_WINDOWLOG_LIMIT_DEFAULT: u32 = 27;
const ZSTD_LDM_DEFAULT_WINDOW_LOG: u32 = ZSTD_WINDOWLOG_LIMIT_DEFAULT;

#[inline]
fn ZSTD_MAX_i32(a: c_int, b: c_int) -> c_int {
    if a > b {
        a
    } else {
        b
    }
}
#[inline]
fn BOUNDED_u32(min: u32, val: u32, max: u32) -> u32 {
    let m = if val < max { val } else { max };
    if min > m {
        min
    } else {
        m
    }
}

// ---------------------------------------------------------------------------
// Error helper macros (mirroring error_private.h behaviour)
// ---------------------------------------------------------------------------
macro_rules! RETURN_ERROR_IF {
    ($cond:expr, $err:ident) => {
        if $cond {
            return error(code::$err);
        }
    };
    ($cond:expr, $err:ident, $($msg:tt)*) => {
        if $cond {
            return error(code::$err);
        }
    };
}

macro_rules! RETURN_ERROR {
    ($err:ident) => {
        return error(code::$err);
    };
    ($err:ident, $($msg:tt)*) => {
        return error(code::$err);
    };
}

macro_rules! BOUNDCHECK {
    ($cparam:expr, $val:expr) => {
        RETURN_ERROR_IF!(
            ZSTD_cParam_withinBounds($cparam, $val) == 0,
            PARAMETER_OUTOFBOUND
        );
    };
}

macro_rules! FORWARD_IF_ERROR {
    ($fnres:expr) => {{
        let __err_code = $fnres;
        if err_is_error(__err_code) != 0 {
            return __err_code;
        }
    }};
    ($fnres:expr, $($msg:tt)*) => {{
        let __err_code = $fnres;
        if err_is_error(__err_code) != 0 {
            return __err_code;
        }
    }};
}

#[inline]
unsafe fn ZSTD_isError(code: usize) -> c_uint {
    err_is_error(code)
}

// cpuid: DYNAMIC_BMI2=0 build; bmi2 field is inert (all paths fold to non-bmi2).
#[derive(Clone, Copy)]
struct ZSTD_cpuid_t {
    f1c: U32,
    f1d: U32,
    f7b: U32,
    f7c: U32,
}
#[inline]
fn ZSTD_cpuid() -> ZSTD_cpuid_t {
    ZSTD_cpuid_t {
        f1c: 0,
        f1d: 0,
        f7b: 0,
        f7c: 0,
    }
}
#[inline]
fn ZSTD_cpuid_bmi2(_cpuid: ZSTD_cpuid_t) -> c_int {
    0
}

// ---------------------------------------------------------------------------
// ZSTD_CDict_s — real definition (the internal module has an opaque stand-in).
// Functions keep the opaque `*mut ZSTD_CDict` ABI type and cast internally.
// ---------------------------------------------------------------------------
#[repr(C)]
pub struct ZSTD_CDict_real {
    pub dictContent: *const c_void,
    pub dictContentSize: usize,
    pub dictContentType: ZSTD_dictContentType_e,
    pub entropyWorkspace: *mut U32, /* entropy workspace of HUF_WORKSPACE_SIZE bytes */
    pub workspace: ZSTD_cwksp,
    pub matchState: ZSTD_MatchState_t,
    pub cBlockState: ZSTD_compressedBlockState_t,
    pub customMem: ZSTD_customMem,
    pub dictID: U32,
    pub compressionLevel: c_int, /* 0 indicates that advanced API was used to select CDict params */
    pub useRowMatchFinder: ZSTD_ParamSwitch_e,
}

type CDictReal = ZSTD_CDict_real;

/// ZSTD_frameProgression (public API, include/zstd.h).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ZSTD_frameProgression {
    pub ingested: c_ulonglong,
    pub consumed: c_ulonglong,
    pub produced: c_ulonglong,
    pub flushed: c_ulonglong,
    pub currentJobID: c_uint,
    pub nbActiveWorkers: c_uint,
}

#[inline]
unsafe fn as_cdict<'a>(cdict: *const ZSTD_CDict) -> *const CDictReal {
    cdict as *const CDictReal
}
#[inline]
unsafe fn as_cdict_mut(cdict: *mut ZSTD_CDict) -> *mut CDictReal {
    cdict as *mut CDictReal
}

// ZSTD_STATIC_ASSERT-style compile time checks are represented as const asserts inline.

/*-*************************************
*  Helper functions
***************************************/
#[inline]
unsafe fn ZSTD_limitCopy(
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let length = core::cmp::min(dstCapacity, srcSize);
    if length > 0 {
        memcpy(dst, src, length);
    }
    length
}

#[inline]
fn ZSTD_FRAMEHEADERSIZE_MIN(format: ZSTD_format_e) -> usize {
    if format == ZSTD_f_zstd1 {
        6
    } else {
        2
    }
}

#[inline]
fn ZSTD_COMPRESSBOUND(srcSize: usize) -> usize {
    if (srcSize as u64) >= ZSTD_MAX_INPUT_SIZE {
        0
    } else {
        srcSize
            + (srcSize >> 8)
            + (if srcSize < (128 << 10) {
                ((128 << 10) - srcSize) >> 11
            } else {
                0
            })
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBound(srcSize: usize) -> usize {
    let r = ZSTD_COMPRESSBOUND(srcSize);
    if r == 0 {
        return error(code::SRCSIZE_WRONG);
    }
    r
}

/*-*************************************
*  Context memory management
***************************************/
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_createCCtx() -> *mut ZSTD_CCtx {
    ZSTD_createCCtx_advanced(ZSTD_defaultCMem)
}

unsafe fn ZSTD_initCCtx(cctx: *mut ZSTD_CCtx, memManager: ZSTD_customMem) {
    debug_assert!(!cctx.is_null());
    memset(cctx as *mut c_void, 0, core::mem::size_of::<ZSTD_CCtx>());
    (*cctx).customMem = memManager;
    (*cctx).bmi2 = ZSTD_cpuSupportsBmi2();
    {
        let err = ZSTD_CCtx_reset(cctx, ZSTD_reset_parameters);
        debug_assert!(ZSTD_isError(err) == 0);
        let _ = err;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_createCCtx_advanced(customMem: ZSTD_customMem) -> *mut ZSTD_CCtx {
    const _: () = assert!(zcss_init == 0);
    const _: () = assert!(ZSTD_CONTENTSIZE_UNKNOWN == (0u64.wrapping_sub(1)));
    if (customMem.customAlloc.is_none()) ^ (customMem.customFree.is_none()) {
        return core::ptr::null_mut();
    }
    {
        let cctx =
            zstd_custom_malloc(core::mem::size_of::<ZSTD_CCtx>(), customMem) as *mut ZSTD_CCtx;
        if cctx.is_null() {
            return core::ptr::null_mut();
        }
        ZSTD_initCCtx(cctx, customMem);
        cctx
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_initStaticCCtx(
    workspace: *mut c_void,
    workspaceSize: usize,
) -> *mut ZSTD_CCtx {
    let mut ws: ZSTD_cwksp = core::mem::zeroed();
    let cctx: *mut ZSTD_CCtx;
    if workspaceSize <= core::mem::size_of::<ZSTD_CCtx>() {
        return core::ptr::null_mut();
    }
    if (workspace as usize) & 7 != 0 {
        return core::ptr::null_mut();
    }
    ZSTD_cwksp_init(&mut ws, workspace, workspaceSize, ZSTD_cwksp_static_alloc);

    cctx = ZSTD_cwksp_reserve_object(&mut ws, core::mem::size_of::<ZSTD_CCtx>()) as *mut ZSTD_CCtx;
    if cctx.is_null() {
        return core::ptr::null_mut();
    }

    memset(cctx as *mut c_void, 0, core::mem::size_of::<ZSTD_CCtx>());
    ZSTD_cwksp_move(&mut (*cctx).workspace, &mut ws);
    (*cctx).staticSize = workspaceSize;

    if ZSTD_cwksp_check_available(
        &mut (*cctx).workspace,
        TMP_WORKSPACE_SIZE + 2 * core::mem::size_of::<ZSTD_compressedBlockState_t>(),
    ) == 0
    {
        return core::ptr::null_mut();
    }
    (*cctx).blockState.prevCBlock = ZSTD_cwksp_reserve_object(
        &mut (*cctx).workspace,
        core::mem::size_of::<ZSTD_compressedBlockState_t>(),
    ) as *mut ZSTD_compressedBlockState_t;
    (*cctx).blockState.nextCBlock = ZSTD_cwksp_reserve_object(
        &mut (*cctx).workspace,
        core::mem::size_of::<ZSTD_compressedBlockState_t>(),
    ) as *mut ZSTD_compressedBlockState_t;
    (*cctx).tmpWorkspace = ZSTD_cwksp_reserve_object(&mut (*cctx).workspace, TMP_WORKSPACE_SIZE);
    (*cctx).tmpWkspSize = TMP_WORKSPACE_SIZE;
    (*cctx).bmi2 = ZSTD_cpuid_bmi2(ZSTD_cpuid());
    cctx
}

/// Clears and frees all of the dictionaries in the CCtx.
unsafe fn ZSTD_clearAllDicts(cctx: *mut ZSTD_CCtx) {
    zstd_custom_free((*cctx).localDict.dictBuffer, (*cctx).customMem);
    ZSTD_freeCDict((*cctx).localDict.cdict);
    memset(
        &mut (*cctx).localDict as *mut _ as *mut c_void,
        0,
        core::mem::size_of::<ZSTD_localDict>(),
    );
    memset(
        &mut (*cctx).prefixDict as *mut _ as *mut c_void,
        0,
        core::mem::size_of::<ZSTD_prefixDict>(),
    );
    (*cctx).cdict = core::ptr::null();
}

unsafe fn ZSTD_sizeof_localDict(dict: *const ZSTD_localDict) -> usize {
    let bufferSize = if !(*dict).dictBuffer.is_null() {
        (*dict).dictSize
    } else {
        0
    };
    let cdictSize = ZSTD_sizeof_CDict((*dict).cdict);
    bufferSize + cdictSize
}

unsafe fn ZSTD_freeCCtxContent(cctx: *mut ZSTD_CCtx) {
    debug_assert!(!cctx.is_null());
    debug_assert!((*cctx).staticSize == 0);
    ZSTD_clearAllDicts(cctx);
    ZSTD_cwksp_free(&mut (*cctx).workspace, (*cctx).customMem);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_freeCCtx(cctx: *mut ZSTD_CCtx) -> usize {
    if cctx.is_null() {
        return 0; /* support free on NULL */
    }
    RETURN_ERROR_IF!((*cctx).staticSize != 0, MEMORY_ALLOCATION);
    {
        let cctxInWorkspace = ZSTD_cwksp_owns_buffer(&(*cctx).workspace, cctx as *const c_void);
        ZSTD_freeCCtxContent(cctx);
        if cctxInWorkspace == 0 {
            zstd_custom_free(cctx as *mut c_void, (*cctx).customMem);
        }
    }
    0
}

unsafe fn ZSTD_sizeof_mtctx(_cctx: *const ZSTD_CCtx) -> usize {
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_sizeof_CCtx(cctx: *const ZSTD_CCtx) -> usize {
    if cctx.is_null() {
        return 0; /* support sizeof on NULL */
    }
    (if (*cctx).workspace.workspace == cctx as *mut c_void {
        0
    } else {
        core::mem::size_of::<ZSTD_CCtx>()
    }) + ZSTD_cwksp_sizeof(&(*cctx).workspace)
        + ZSTD_sizeof_localDict(&(*cctx).localDict)
        + ZSTD_sizeof_mtctx(cctx)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_sizeof_CStream(zcs: *const ZSTD_CStream) -> usize {
    ZSTD_sizeof_CCtx(zcs)
}

/* private API call, for dictBuilder only */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_getSeqStore(ctx: *const ZSTD_CCtx) -> *const SeqStore_t {
    &(*ctx).seqStore
}

/* Returns true if the strategy supports using a row based matchfinder */
fn ZSTD_rowMatchFinderSupported(strategy: ZSTD_strategy) -> c_int {
    (strategy >= ZSTD_greedy && strategy <= ZSTD_lazy2) as c_int
}

fn ZSTD_rowMatchFinderUsed(strategy: ZSTD_strategy, mode: ZSTD_ParamSwitch_e) -> c_int {
    debug_assert!(mode != ZSTD_ps_auto);
    (ZSTD_rowMatchFinderSupported(strategy) != 0 && (mode == ZSTD_ps_enable)) as c_int
}

unsafe fn ZSTD_resolveRowMatchFinderMode(
    mut mode: ZSTD_ParamSwitch_e,
    cParams: *const ZSTD_compressionParameters,
) -> ZSTD_ParamSwitch_e {
    if mode != ZSTD_ps_auto {
        return mode;
    }
    mode = ZSTD_ps_disable;
    if ZSTD_rowMatchFinderSupported((*cParams).strategy) == 0 {
        return mode;
    }
    if (*cParams).windowLog > 14 {
        mode = ZSTD_ps_enable;
    }
    mode
}

unsafe fn ZSTD_resolveBlockSplitterMode(
    mode: ZSTD_ParamSwitch_e,
    cParams: *const ZSTD_compressionParameters,
) -> ZSTD_ParamSwitch_e {
    if mode != ZSTD_ps_auto {
        return mode;
    }
    if (*cParams).strategy >= ZSTD_btopt && (*cParams).windowLog >= 17 {
        ZSTD_ps_enable
    } else {
        ZSTD_ps_disable
    }
}

unsafe fn ZSTD_allocateChainTable(
    strategy: ZSTD_strategy,
    useRowMatchFinder: ZSTD_ParamSwitch_e,
    forDDSDict: U32,
) -> c_int {
    debug_assert!(useRowMatchFinder != ZSTD_ps_auto);
    (forDDSDict != 0
        || ((strategy != ZSTD_fast) && ZSTD_rowMatchFinderUsed(strategy, useRowMatchFinder) == 0))
        as c_int
}

unsafe fn ZSTD_resolveEnableLdm(
    mode: ZSTD_ParamSwitch_e,
    cParams: *const ZSTD_compressionParameters,
) -> ZSTD_ParamSwitch_e {
    if mode != ZSTD_ps_auto {
        return mode;
    }
    if (*cParams).strategy >= ZSTD_btopt && (*cParams).windowLog >= 27 {
        ZSTD_ps_enable
    } else {
        ZSTD_ps_disable
    }
}

fn ZSTD_resolveExternalSequenceValidation(mode: c_int) -> c_int {
    mode
}

fn ZSTD_resolveMaxBlockSize(maxBlockSize: usize) -> usize {
    if maxBlockSize == 0 {
        ZSTD_BLOCKSIZE_MAX
    } else {
        maxBlockSize
    }
}

fn ZSTD_resolveExternalRepcodeSearch(value: ZSTD_ParamSwitch_e, cLevel: c_int) -> ZSTD_ParamSwitch_e {
    if value != ZSTD_ps_auto {
        return value;
    }
    if cLevel < 10 {
        ZSTD_ps_disable
    } else {
        ZSTD_ps_enable
    }
}

unsafe fn ZSTD_CDictIndicesAreTagged(cParams: *const ZSTD_compressionParameters) -> c_int {
    ((*cParams).strategy == ZSTD_fast || (*cParams).strategy == ZSTD_dfast) as c_int
}

unsafe fn ZSTD_makeCCtxParamsFromCParams(
    cParams: ZSTD_compressionParameters,
) -> ZSTD_CCtx_params {
    let mut cctxParams: ZSTD_CCtx_params = core::mem::zeroed();
    ZSTD_CCtxParams_init(&mut cctxParams, ZSTD_CLEVEL_DEFAULT);
    cctxParams.cParams = cParams;

    cctxParams.ldmParams.enableLdm =
        ZSTD_resolveEnableLdm(cctxParams.ldmParams.enableLdm, &cParams);
    if cctxParams.ldmParams.enableLdm == ZSTD_ps_enable {
        ZSTD_ldm_adjustParameters(&mut cctxParams.ldmParams, &cParams);
        debug_assert!(cctxParams.ldmParams.hashLog >= cctxParams.ldmParams.bucketSizeLog);
        debug_assert!(cctxParams.ldmParams.hashRateLog < 32);
    }
    cctxParams.postBlockSplitter =
        ZSTD_resolveBlockSplitterMode(cctxParams.postBlockSplitter, &cParams);
    cctxParams.useRowMatchFinder =
        ZSTD_resolveRowMatchFinderMode(cctxParams.useRowMatchFinder, &cParams);
    cctxParams.validateSequences =
        ZSTD_resolveExternalSequenceValidation(cctxParams.validateSequences);
    cctxParams.maxBlockSize = ZSTD_resolveMaxBlockSize(cctxParams.maxBlockSize);
    cctxParams.searchForExternalRepcodes = ZSTD_resolveExternalRepcodeSearch(
        cctxParams.searchForExternalRepcodes,
        cctxParams.compressionLevel,
    );
    debug_assert!(ZSTD_checkCParams(cParams) == 0);
    cctxParams
}

unsafe fn ZSTD_createCCtxParams_advanced(customMem: ZSTD_customMem) -> *mut ZSTD_CCtx_params {
    let params: *mut ZSTD_CCtx_params;
    if (customMem.customAlloc.is_none()) ^ (customMem.customFree.is_none()) {
        return core::ptr::null_mut();
    }
    params = zstd_custom_calloc(core::mem::size_of::<ZSTD_CCtx_params>(), customMem)
        as *mut ZSTD_CCtx_params;
    if params.is_null() {
        return core::ptr::null_mut();
    }
    ZSTD_CCtxParams_init(params, ZSTD_CLEVEL_DEFAULT);
    (*params).customMem = customMem;
    params
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_createCCtxParams() -> *mut ZSTD_CCtx_params {
    ZSTD_createCCtxParams_advanced(ZSTD_defaultCMem)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_freeCCtxParams(params: *mut ZSTD_CCtx_params) -> usize {
    if params.is_null() {
        return 0;
    }
    zstd_custom_free(params as *mut c_void, (*params).customMem);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_CCtxParams_reset(params: *mut ZSTD_CCtx_params) -> usize {
    ZSTD_CCtxParams_init(params, ZSTD_CLEVEL_DEFAULT)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_CCtxParams_init(
    cctxParams: *mut ZSTD_CCtx_params,
    compressionLevel: c_int,
) -> usize {
    RETURN_ERROR_IF!(cctxParams.is_null(), GENERIC);
    memset(
        cctxParams as *mut c_void,
        0,
        core::mem::size_of::<ZSTD_CCtx_params>(),
    );
    (*cctxParams).compressionLevel = compressionLevel;
    (*cctxParams).fParams.contentSizeFlag = 1;
    0
}

unsafe fn ZSTD_CCtxParams_init_internal(
    cctxParams: *mut ZSTD_CCtx_params,
    params: *const ZSTD_parameters,
    compressionLevel: c_int,
) {
    debug_assert!(ZSTD_checkCParams((*params).cParams) == 0);
    memset(
        cctxParams as *mut c_void,
        0,
        core::mem::size_of::<ZSTD_CCtx_params>(),
    );
    (*cctxParams).cParams = (*params).cParams;
    (*cctxParams).fParams = (*params).fParams;
    (*cctxParams).compressionLevel = compressionLevel;
    (*cctxParams).useRowMatchFinder =
        ZSTD_resolveRowMatchFinderMode((*cctxParams).useRowMatchFinder, &(*params).cParams);
    (*cctxParams).postBlockSplitter =
        ZSTD_resolveBlockSplitterMode((*cctxParams).postBlockSplitter, &(*params).cParams);
    (*cctxParams).ldmParams.enableLdm =
        ZSTD_resolveEnableLdm((*cctxParams).ldmParams.enableLdm, &(*params).cParams);
    (*cctxParams).validateSequences =
        ZSTD_resolveExternalSequenceValidation((*cctxParams).validateSequences);
    (*cctxParams).maxBlockSize = ZSTD_resolveMaxBlockSize((*cctxParams).maxBlockSize);
    (*cctxParams).searchForExternalRepcodes = ZSTD_resolveExternalRepcodeSearch(
        (*cctxParams).searchForExternalRepcodes,
        compressionLevel,
    );
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_CCtxParams_init_advanced(
    cctxParams: *mut ZSTD_CCtx_params,
    params: ZSTD_parameters,
) -> usize {
    RETURN_ERROR_IF!(cctxParams.is_null(), GENERIC);
    FORWARD_IF_ERROR!(ZSTD_checkCParams(params.cParams));
    ZSTD_CCtxParams_init_internal(cctxParams, &params, ZSTD_NO_CLEVEL);
    0
}

unsafe fn ZSTD_CCtxParams_setZstdParams(
    cctxParams: *mut ZSTD_CCtx_params,
    params: *const ZSTD_parameters,
) {
    debug_assert!(ZSTD_checkCParams((*params).cParams) == 0);
    (*cctxParams).cParams = (*params).cParams;
    (*cctxParams).fParams = (*params).fParams;
    (*cctxParams).compressionLevel = ZSTD_NO_CLEVEL;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_cParam_getBounds(param: ZSTD_cParameter) -> ZSTD_bounds {
    let mut bounds = ZSTD_bounds {
        error: 0,
        lowerBound: 0,
        upperBound: 0,
    };
    match param {
        ZSTD_c_compressionLevel => {
            bounds.lowerBound = ZSTD_minCLevel();
            bounds.upperBound = ZSTD_maxCLevel();
        }
        ZSTD_c_windowLog => {
            bounds.lowerBound = ZSTD_WINDOWLOG_MIN;
            bounds.upperBound = ZSTD_WINDOWLOG_MAX;
        }
        ZSTD_c_hashLog => {
            bounds.lowerBound = ZSTD_HASHLOG_MIN;
            bounds.upperBound = ZSTD_HASHLOG_MAX;
        }
        ZSTD_c_chainLog => {
            bounds.lowerBound = ZSTD_CHAINLOG_MIN;
            bounds.upperBound = ZSTD_CHAINLOG_MAX;
        }
        ZSTD_c_searchLog => {
            bounds.lowerBound = ZSTD_SEARCHLOG_MIN;
            bounds.upperBound = ZSTD_SEARCHLOG_MAX;
        }
        ZSTD_c_minMatch => {
            bounds.lowerBound = ZSTD_MINMATCH_MIN;
            bounds.upperBound = ZSTD_MINMATCH_MAX;
        }
        ZSTD_c_targetLength => {
            bounds.lowerBound = ZSTD_TARGETLENGTH_MIN;
            bounds.upperBound = ZSTD_TARGETLENGTH_MAX;
        }
        ZSTD_c_strategy => {
            bounds.lowerBound = ZSTD_STRATEGY_MIN;
            bounds.upperBound = ZSTD_STRATEGY_MAX;
        }
        ZSTD_c_contentSizeFlag => {
            bounds.lowerBound = 0;
            bounds.upperBound = 1;
        }
        ZSTD_c_checksumFlag => {
            bounds.lowerBound = 0;
            bounds.upperBound = 1;
        }
        ZSTD_c_dictIDFlag => {
            bounds.lowerBound = 0;
            bounds.upperBound = 1;
        }
        ZSTD_c_nbWorkers => {
            bounds.lowerBound = 0;
            bounds.upperBound = 0;
        }
        ZSTD_c_jobSize => {
            bounds.lowerBound = 0;
            bounds.upperBound = 0;
        }
        ZSTD_c_overlapLog => {
            bounds.lowerBound = 0;
            bounds.upperBound = 0;
        }
        ZSTD_c_enableDedicatedDictSearch => {
            bounds.lowerBound = 0;
            bounds.upperBound = 1;
        }
        ZSTD_c_enableLongDistanceMatching => {
            bounds.lowerBound = ZSTD_ps_auto as c_int;
            bounds.upperBound = ZSTD_ps_disable as c_int;
        }
        ZSTD_c_ldmHashLog => {
            bounds.lowerBound = ZSTD_LDM_HASHLOG_MIN;
            bounds.upperBound = ZSTD_LDM_HASHLOG_MAX;
        }
        ZSTD_c_ldmMinMatch => {
            bounds.lowerBound = ZSTD_LDM_MINMATCH_MIN;
            bounds.upperBound = ZSTD_LDM_MINMATCH_MAX;
        }
        ZSTD_c_ldmBucketSizeLog => {
            bounds.lowerBound = ZSTD_LDM_BUCKETSIZELOG_MIN;
            bounds.upperBound = ZSTD_LDM_BUCKETSIZELOG_MAX;
        }
        ZSTD_c_ldmHashRateLog => {
            bounds.lowerBound = ZSTD_LDM_HASHRATELOG_MIN;
            bounds.upperBound = ZSTD_LDM_HASHRATELOG_MAX;
        }
        ZSTD_c_rsyncable => {
            bounds.lowerBound = 0;
            bounds.upperBound = 1;
        }
        ZSTD_c_forceMaxWindow => {
            bounds.lowerBound = 0;
            bounds.upperBound = 1;
        }
        ZSTD_c_format => {
            const _: () = assert!(ZSTD_f_zstd1 < ZSTD_f_zstd1_magicless);
            bounds.lowerBound = ZSTD_f_zstd1 as c_int;
            bounds.upperBound = ZSTD_f_zstd1_magicless as c_int;
        }
        ZSTD_c_forceAttachDict => {
            const _: () = assert!(ZSTD_dictDefaultAttach < ZSTD_dictForceLoad);
            bounds.lowerBound = ZSTD_dictDefaultAttach as c_int;
            bounds.upperBound = ZSTD_dictForceLoad as c_int;
        }
        ZSTD_c_literalCompressionMode => {
            const _: () =
                assert!(ZSTD_ps_auto < ZSTD_ps_enable && ZSTD_ps_enable < ZSTD_ps_disable);
            bounds.lowerBound = ZSTD_ps_auto as c_int;
            bounds.upperBound = ZSTD_ps_disable as c_int;
        }
        ZSTD_c_targetCBlockSize => {
            bounds.lowerBound = ZSTD_TARGETCBLOCKSIZE_MIN;
            bounds.upperBound = ZSTD_TARGETCBLOCKSIZE_MAX;
        }
        ZSTD_c_srcSizeHint => {
            bounds.lowerBound = ZSTD_SRCSIZEHINT_MIN;
            bounds.upperBound = ZSTD_SRCSIZEHINT_MAX;
        }
        ZSTD_c_stableInBuffer | ZSTD_c_stableOutBuffer => {
            bounds.lowerBound = ZSTD_bm_buffered as c_int;
            bounds.upperBound = ZSTD_bm_stable as c_int;
        }
        ZSTD_c_blockDelimiters => {
            bounds.lowerBound = ZSTD_sf_noBlockDelimiters as c_int;
            bounds.upperBound = ZSTD_sf_explicitBlockDelimiters as c_int;
        }
        ZSTD_c_validateSequences => {
            bounds.lowerBound = 0;
            bounds.upperBound = 1;
        }
        ZSTD_c_splitAfterSequences => {
            bounds.lowerBound = ZSTD_ps_auto as c_int;
            bounds.upperBound = ZSTD_ps_disable as c_int;
        }
        ZSTD_c_blockSplitterLevel => {
            bounds.lowerBound = 0;
            bounds.upperBound = ZSTD_BLOCKSPLITTER_LEVEL_MAX;
        }
        ZSTD_c_useRowMatchFinder => {
            bounds.lowerBound = ZSTD_ps_auto as c_int;
            bounds.upperBound = ZSTD_ps_disable as c_int;
        }
        ZSTD_c_deterministicRefPrefix => {
            bounds.lowerBound = 0;
            bounds.upperBound = 1;
        }
        ZSTD_c_prefetchCDictTables => {
            bounds.lowerBound = ZSTD_ps_auto as c_int;
            bounds.upperBound = ZSTD_ps_disable as c_int;
        }
        ZSTD_c_enableSeqProducerFallback => {
            bounds.lowerBound = 0;
            bounds.upperBound = 1;
        }
        ZSTD_c_maxBlockSize => {
            bounds.lowerBound = ZSTD_BLOCKSIZE_MAX_MIN;
            bounds.upperBound = ZSTD_BLOCKSIZE_MAX as c_int;
        }
        ZSTD_c_repcodeResolution => {
            bounds.lowerBound = ZSTD_ps_auto as c_int;
            bounds.upperBound = ZSTD_ps_disable as c_int;
        }
        _ => {
            bounds.error = error(code::PARAMETER_UNSUPPORTED);
        }
    }
    bounds
}

unsafe fn ZSTD_cParam_clampBounds(cParam: ZSTD_cParameter, value: *mut c_int) -> usize {
    let bounds = ZSTD_cParam_getBounds(cParam);
    if ZSTD_isError(bounds.error) != 0 {
        return bounds.error;
    }
    if *value < bounds.lowerBound {
        *value = bounds.lowerBound;
    }
    if *value > bounds.upperBound {
        *value = bounds.upperBound;
    }
    0
}

unsafe fn ZSTD_isUpdateAuthorized(param: ZSTD_cParameter) -> c_int {
    match param {
        ZSTD_c_compressionLevel
        | ZSTD_c_hashLog
        | ZSTD_c_chainLog
        | ZSTD_c_searchLog
        | ZSTD_c_minMatch
        | ZSTD_c_targetLength
        | ZSTD_c_strategy
        | ZSTD_c_blockSplitterLevel => 1,
        _ => 0,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_CCtx_setParameter(
    cctx: *mut ZSTD_CCtx,
    param: ZSTD_cParameter,
    value: c_int,
) -> usize {
    if (*cctx).streamStage != zcss_init {
        if ZSTD_isUpdateAuthorized(param) != 0 {
            (*cctx).cParamsChanged = 1;
        } else {
            RETURN_ERROR!(STAGE_WRONG);
        }
    }

    match param {
        ZSTD_c_nbWorkers => {
            RETURN_ERROR_IF!((value != 0) && (*cctx).staticSize != 0, PARAMETER_UNSUPPORTED);
        }
        ZSTD_c_compressionLevel
        | ZSTD_c_windowLog
        | ZSTD_c_hashLog
        | ZSTD_c_chainLog
        | ZSTD_c_searchLog
        | ZSTD_c_minMatch
        | ZSTD_c_targetLength
        | ZSTD_c_strategy
        | ZSTD_c_ldmHashRateLog
        | ZSTD_c_format
        | ZSTD_c_contentSizeFlag
        | ZSTD_c_checksumFlag
        | ZSTD_c_dictIDFlag
        | ZSTD_c_forceMaxWindow
        | ZSTD_c_forceAttachDict
        | ZSTD_c_literalCompressionMode
        | ZSTD_c_jobSize
        | ZSTD_c_overlapLog
        | ZSTD_c_rsyncable
        | ZSTD_c_enableDedicatedDictSearch
        | ZSTD_c_enableLongDistanceMatching
        | ZSTD_c_ldmHashLog
        | ZSTD_c_ldmMinMatch
        | ZSTD_c_ldmBucketSizeLog
        | ZSTD_c_targetCBlockSize
        | ZSTD_c_srcSizeHint
        | ZSTD_c_stableInBuffer
        | ZSTD_c_stableOutBuffer
        | ZSTD_c_blockDelimiters
        | ZSTD_c_validateSequences
        | ZSTD_c_splitAfterSequences
        | ZSTD_c_blockSplitterLevel
        | ZSTD_c_useRowMatchFinder
        | ZSTD_c_deterministicRefPrefix
        | ZSTD_c_prefetchCDictTables
        | ZSTD_c_enableSeqProducerFallback
        | ZSTD_c_maxBlockSize
        | ZSTD_c_repcodeResolution => {}
        _ => {
            RETURN_ERROR!(PARAMETER_UNSUPPORTED);
        }
    }
    ZSTD_CCtxParams_setParameter(&mut (*cctx).requestedParams, param, value)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_CCtxParams_setParameter(
    CCtxParams: *mut ZSTD_CCtx_params,
    param: ZSTD_cParameter,
    mut value: c_int,
) -> usize {
    match param {
        ZSTD_c_format => {
            BOUNDCHECK!(ZSTD_c_format, value);
            (*CCtxParams).format = value as ZSTD_format_e;
            (*CCtxParams).format as usize
        }
        ZSTD_c_compressionLevel => {
            FORWARD_IF_ERROR!(ZSTD_cParam_clampBounds(param, &mut value));
            if value == 0 {
                (*CCtxParams).compressionLevel = ZSTD_CLEVEL_DEFAULT;
            } else {
                (*CCtxParams).compressionLevel = value;
            }
            if (*CCtxParams).compressionLevel >= 0 {
                return (*CCtxParams).compressionLevel as usize;
            }
            0
        }
        ZSTD_c_windowLog => {
            if value != 0 {
                BOUNDCHECK!(ZSTD_c_windowLog, value);
            }
            (*CCtxParams).cParams.windowLog = value as U32;
            (*CCtxParams).cParams.windowLog as usize
        }
        ZSTD_c_hashLog => {
            if value != 0 {
                BOUNDCHECK!(ZSTD_c_hashLog, value);
            }
            (*CCtxParams).cParams.hashLog = value as U32;
            (*CCtxParams).cParams.hashLog as usize
        }
        ZSTD_c_chainLog => {
            if value != 0 {
                BOUNDCHECK!(ZSTD_c_chainLog, value);
            }
            (*CCtxParams).cParams.chainLog = value as U32;
            (*CCtxParams).cParams.chainLog as usize
        }
        ZSTD_c_searchLog => {
            if value != 0 {
                BOUNDCHECK!(ZSTD_c_searchLog, value);
            }
            (*CCtxParams).cParams.searchLog = value as U32;
            value as usize
        }
        ZSTD_c_minMatch => {
            if value != 0 {
                BOUNDCHECK!(ZSTD_c_minMatch, value);
            }
            (*CCtxParams).cParams.minMatch = value as U32;
            (*CCtxParams).cParams.minMatch as usize
        }
        ZSTD_c_targetLength => {
            BOUNDCHECK!(ZSTD_c_targetLength, value);
            (*CCtxParams).cParams.targetLength = value as U32;
            (*CCtxParams).cParams.targetLength as usize
        }
        ZSTD_c_strategy => {
            if value != 0 {
                BOUNDCHECK!(ZSTD_c_strategy, value);
            }
            (*CCtxParams).cParams.strategy = value as ZSTD_strategy;
            (*CCtxParams).cParams.strategy as usize
        }
        ZSTD_c_contentSizeFlag => {
            (*CCtxParams).fParams.contentSizeFlag = (value != 0) as c_int;
            (*CCtxParams).fParams.contentSizeFlag as usize
        }
        ZSTD_c_checksumFlag => {
            (*CCtxParams).fParams.checksumFlag = (value != 0) as c_int;
            (*CCtxParams).fParams.checksumFlag as usize
        }
        ZSTD_c_dictIDFlag => {
            (*CCtxParams).fParams.noDictIDFlag = (value == 0) as c_int;
            (((*CCtxParams).fParams.noDictIDFlag == 0) as c_int) as usize
        }
        ZSTD_c_forceMaxWindow => {
            (*CCtxParams).forceWindow = (value != 0) as c_int;
            (*CCtxParams).forceWindow as usize
        }
        ZSTD_c_forceAttachDict => {
            let pref = value as ZSTD_dictAttachPref_e;
            BOUNDCHECK!(ZSTD_c_forceAttachDict, pref as c_int);
            (*CCtxParams).attachDictPref = pref;
            (*CCtxParams).attachDictPref as usize
        }
        ZSTD_c_literalCompressionMode => {
            let lcm = value as ZSTD_ParamSwitch_e;
            BOUNDCHECK!(ZSTD_c_literalCompressionMode, lcm as c_int);
            (*CCtxParams).literalCompressionMode = lcm;
            (*CCtxParams).literalCompressionMode as usize
        }
        ZSTD_c_nbWorkers => {
            RETURN_ERROR_IF!(value != 0, PARAMETER_UNSUPPORTED);
            0
        }
        ZSTD_c_jobSize => {
            RETURN_ERROR_IF!(value != 0, PARAMETER_UNSUPPORTED);
            0
        }
        ZSTD_c_overlapLog => {
            RETURN_ERROR_IF!(value != 0, PARAMETER_UNSUPPORTED);
            0
        }
        ZSTD_c_rsyncable => {
            RETURN_ERROR_IF!(value != 0, PARAMETER_UNSUPPORTED);
            0
        }
        ZSTD_c_enableDedicatedDictSearch => {
            (*CCtxParams).enableDedicatedDictSearch = (value != 0) as c_int;
            (*CCtxParams).enableDedicatedDictSearch as usize
        }
        ZSTD_c_enableLongDistanceMatching => {
            BOUNDCHECK!(ZSTD_c_enableLongDistanceMatching, value);
            (*CCtxParams).ldmParams.enableLdm = value as ZSTD_ParamSwitch_e;
            (*CCtxParams).ldmParams.enableLdm as usize
        }
        ZSTD_c_ldmHashLog => {
            if value != 0 {
                BOUNDCHECK!(ZSTD_c_ldmHashLog, value);
            }
            (*CCtxParams).ldmParams.hashLog = value as U32;
            (*CCtxParams).ldmParams.hashLog as usize
        }
        ZSTD_c_ldmMinMatch => {
            if value != 0 {
                BOUNDCHECK!(ZSTD_c_ldmMinMatch, value);
            }
            (*CCtxParams).ldmParams.minMatchLength = value as U32;
            (*CCtxParams).ldmParams.minMatchLength as usize
        }
        ZSTD_c_ldmBucketSizeLog => {
            if value != 0 {
                BOUNDCHECK!(ZSTD_c_ldmBucketSizeLog, value);
            }
            (*CCtxParams).ldmParams.bucketSizeLog = value as U32;
            (*CCtxParams).ldmParams.bucketSizeLog as usize
        }
        ZSTD_c_ldmHashRateLog => {
            if value != 0 {
                BOUNDCHECK!(ZSTD_c_ldmHashRateLog, value);
            }
            (*CCtxParams).ldmParams.hashRateLog = value as U32;
            (*CCtxParams).ldmParams.hashRateLog as usize
        }
        ZSTD_c_targetCBlockSize => {
            if value != 0 {
                value = if value > ZSTD_TARGETCBLOCKSIZE_MIN {
                    value
                } else {
                    ZSTD_TARGETCBLOCKSIZE_MIN
                };
                BOUNDCHECK!(ZSTD_c_targetCBlockSize, value);
            }
            (*CCtxParams).targetCBlockSize = value as U32 as usize;
            (*CCtxParams).targetCBlockSize
        }
        ZSTD_c_srcSizeHint => {
            if value != 0 {
                BOUNDCHECK!(ZSTD_c_srcSizeHint, value);
            }
            (*CCtxParams).srcSizeHint = value;
            (*CCtxParams).srcSizeHint as usize
        }
        ZSTD_c_stableInBuffer => {
            BOUNDCHECK!(ZSTD_c_stableInBuffer, value);
            (*CCtxParams).inBufferMode = value as ZSTD_bufferMode_e;
            (*CCtxParams).inBufferMode as usize
        }
        ZSTD_c_stableOutBuffer => {
            BOUNDCHECK!(ZSTD_c_stableOutBuffer, value);
            (*CCtxParams).outBufferMode = value as ZSTD_bufferMode_e;
            (*CCtxParams).outBufferMode as usize
        }
        ZSTD_c_blockDelimiters => {
            BOUNDCHECK!(ZSTD_c_blockDelimiters, value);
            (*CCtxParams).blockDelimiters = value as ZSTD_SequenceFormat_e;
            (*CCtxParams).blockDelimiters as usize
        }
        ZSTD_c_validateSequences => {
            BOUNDCHECK!(ZSTD_c_validateSequences, value);
            (*CCtxParams).validateSequences = value;
            (*CCtxParams).validateSequences as usize
        }
        ZSTD_c_splitAfterSequences => {
            BOUNDCHECK!(ZSTD_c_splitAfterSequences, value);
            (*CCtxParams).postBlockSplitter = value as ZSTD_ParamSwitch_e;
            (*CCtxParams).postBlockSplitter as usize
        }
        ZSTD_c_blockSplitterLevel => {
            BOUNDCHECK!(ZSTD_c_blockSplitterLevel, value);
            (*CCtxParams).preBlockSplitter_level = value;
            (*CCtxParams).preBlockSplitter_level as usize
        }
        ZSTD_c_useRowMatchFinder => {
            BOUNDCHECK!(ZSTD_c_useRowMatchFinder, value);
            (*CCtxParams).useRowMatchFinder = value as ZSTD_ParamSwitch_e;
            (*CCtxParams).useRowMatchFinder as usize
        }
        ZSTD_c_deterministicRefPrefix => {
            BOUNDCHECK!(ZSTD_c_deterministicRefPrefix, value);
            (*CCtxParams).deterministicRefPrefix = (value != 0) as c_int;
            (*CCtxParams).deterministicRefPrefix as usize
        }
        ZSTD_c_prefetchCDictTables => {
            BOUNDCHECK!(ZSTD_c_prefetchCDictTables, value);
            (*CCtxParams).prefetchCDictTables = value as ZSTD_ParamSwitch_e;
            (*CCtxParams).prefetchCDictTables as usize
        }
        ZSTD_c_enableSeqProducerFallback => {
            BOUNDCHECK!(ZSTD_c_enableSeqProducerFallback, value);
            (*CCtxParams).enableMatchFinderFallback = value;
            (*CCtxParams).enableMatchFinderFallback as usize
        }
        ZSTD_c_maxBlockSize => {
            if value != 0 {
                BOUNDCHECK!(ZSTD_c_maxBlockSize, value);
            }
            debug_assert!(value >= 0);
            (*CCtxParams).maxBlockSize = value as usize;
            (*CCtxParams).maxBlockSize
        }
        ZSTD_c_repcodeResolution => {
            BOUNDCHECK!(ZSTD_c_repcodeResolution, value);
            (*CCtxParams).searchForExternalRepcodes = value as ZSTD_ParamSwitch_e;
            (*CCtxParams).searchForExternalRepcodes as usize
        }
        _ => {
            RETURN_ERROR!(PARAMETER_UNSUPPORTED);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_CCtx_getParameter(
    cctx: *const ZSTD_CCtx,
    param: ZSTD_cParameter,
    value: *mut c_int,
) -> usize {
    ZSTD_CCtxParams_getParameter(&(*cctx).requestedParams, param, value)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_CCtxParams_getParameter(
    CCtxParams: *const ZSTD_CCtx_params,
    param: ZSTD_cParameter,
    value: *mut c_int,
) -> usize {
    match param {
        ZSTD_c_format => *value = (*CCtxParams).format as c_int,
        ZSTD_c_compressionLevel => *value = (*CCtxParams).compressionLevel,
        ZSTD_c_windowLog => *value = (*CCtxParams).cParams.windowLog as c_int,
        ZSTD_c_hashLog => *value = (*CCtxParams).cParams.hashLog as c_int,
        ZSTD_c_chainLog => *value = (*CCtxParams).cParams.chainLog as c_int,
        ZSTD_c_searchLog => *value = (*CCtxParams).cParams.searchLog as c_int,
        ZSTD_c_minMatch => *value = (*CCtxParams).cParams.minMatch as c_int,
        ZSTD_c_targetLength => *value = (*CCtxParams).cParams.targetLength as c_int,
        ZSTD_c_strategy => *value = (*CCtxParams).cParams.strategy as c_int,
        ZSTD_c_contentSizeFlag => *value = (*CCtxParams).fParams.contentSizeFlag,
        ZSTD_c_checksumFlag => *value = (*CCtxParams).fParams.checksumFlag,
        ZSTD_c_dictIDFlag => *value = ((*CCtxParams).fParams.noDictIDFlag == 0) as c_int,
        ZSTD_c_forceMaxWindow => *value = (*CCtxParams).forceWindow,
        ZSTD_c_forceAttachDict => *value = (*CCtxParams).attachDictPref as c_int,
        ZSTD_c_literalCompressionMode => *value = (*CCtxParams).literalCompressionMode as c_int,
        ZSTD_c_nbWorkers => {
            debug_assert!((*CCtxParams).nbWorkers == 0);
            *value = (*CCtxParams).nbWorkers;
        }
        ZSTD_c_jobSize => {
            RETURN_ERROR!(PARAMETER_UNSUPPORTED);
        }
        ZSTD_c_overlapLog => {
            RETURN_ERROR!(PARAMETER_UNSUPPORTED);
        }
        ZSTD_c_rsyncable => {
            RETURN_ERROR!(PARAMETER_UNSUPPORTED);
        }
        ZSTD_c_enableDedicatedDictSearch => *value = (*CCtxParams).enableDedicatedDictSearch,
        ZSTD_c_enableLongDistanceMatching => *value = (*CCtxParams).ldmParams.enableLdm as c_int,
        ZSTD_c_ldmHashLog => *value = (*CCtxParams).ldmParams.hashLog as c_int,
        ZSTD_c_ldmMinMatch => *value = (*CCtxParams).ldmParams.minMatchLength as c_int,
        ZSTD_c_ldmBucketSizeLog => *value = (*CCtxParams).ldmParams.bucketSizeLog as c_int,
        ZSTD_c_ldmHashRateLog => *value = (*CCtxParams).ldmParams.hashRateLog as c_int,
        ZSTD_c_targetCBlockSize => *value = (*CCtxParams).targetCBlockSize as c_int,
        ZSTD_c_srcSizeHint => *value = (*CCtxParams).srcSizeHint,
        ZSTD_c_stableInBuffer => *value = (*CCtxParams).inBufferMode as c_int,
        ZSTD_c_stableOutBuffer => *value = (*CCtxParams).outBufferMode as c_int,
        ZSTD_c_blockDelimiters => *value = (*CCtxParams).blockDelimiters as c_int,
        ZSTD_c_validateSequences => *value = (*CCtxParams).validateSequences,
        ZSTD_c_splitAfterSequences => *value = (*CCtxParams).postBlockSplitter as c_int,
        ZSTD_c_blockSplitterLevel => *value = (*CCtxParams).preBlockSplitter_level,
        ZSTD_c_useRowMatchFinder => *value = (*CCtxParams).useRowMatchFinder as c_int,
        ZSTD_c_deterministicRefPrefix => *value = (*CCtxParams).deterministicRefPrefix,
        ZSTD_c_prefetchCDictTables => *value = (*CCtxParams).prefetchCDictTables as c_int,
        ZSTD_c_enableSeqProducerFallback => *value = (*CCtxParams).enableMatchFinderFallback,
        ZSTD_c_maxBlockSize => *value = (*CCtxParams).maxBlockSize as c_int,
        ZSTD_c_repcodeResolution => *value = (*CCtxParams).searchForExternalRepcodes as c_int,
        _ => {
            RETURN_ERROR!(PARAMETER_UNSUPPORTED);
        }
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_CCtx_setParametersUsingCCtxParams(
    cctx: *mut ZSTD_CCtx,
    params: *const ZSTD_CCtx_params,
) -> usize {
    RETURN_ERROR_IF!((*cctx).streamStage != zcss_init, STAGE_WRONG);
    RETURN_ERROR_IF!(!(*cctx).cdict.is_null(), STAGE_WRONG);
    core::ptr::copy_nonoverlapping(params, &mut (*cctx).requestedParams, 1);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_CCtx_setCParams(
    cctx: *mut ZSTD_CCtx,
    cparams: ZSTD_compressionParameters,
) -> usize {
    FORWARD_IF_ERROR!(ZSTD_checkCParams(cparams));
    FORWARD_IF_ERROR!(ZSTD_CCtx_setParameter(cctx, ZSTD_c_windowLog, cparams.windowLog as c_int));
    FORWARD_IF_ERROR!(ZSTD_CCtx_setParameter(cctx, ZSTD_c_chainLog, cparams.chainLog as c_int));
    FORWARD_IF_ERROR!(ZSTD_CCtx_setParameter(cctx, ZSTD_c_hashLog, cparams.hashLog as c_int));
    FORWARD_IF_ERROR!(ZSTD_CCtx_setParameter(cctx, ZSTD_c_searchLog, cparams.searchLog as c_int));
    FORWARD_IF_ERROR!(ZSTD_CCtx_setParameter(cctx, ZSTD_c_minMatch, cparams.minMatch as c_int));
    FORWARD_IF_ERROR!(ZSTD_CCtx_setParameter(
        cctx,
        ZSTD_c_targetLength,
        cparams.targetLength as c_int
    ));
    FORWARD_IF_ERROR!(ZSTD_CCtx_setParameter(cctx, ZSTD_c_strategy, cparams.strategy as c_int));
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_CCtx_setFParams(
    cctx: *mut ZSTD_CCtx,
    fparams: ZSTD_frameParameters,
) -> usize {
    FORWARD_IF_ERROR!(ZSTD_CCtx_setParameter(
        cctx,
        ZSTD_c_contentSizeFlag,
        (fparams.contentSizeFlag != 0) as c_int
    ));
    FORWARD_IF_ERROR!(ZSTD_CCtx_setParameter(
        cctx,
        ZSTD_c_checksumFlag,
        (fparams.checksumFlag != 0) as c_int
    ));
    FORWARD_IF_ERROR!(ZSTD_CCtx_setParameter(
        cctx,
        ZSTD_c_dictIDFlag,
        (fparams.noDictIDFlag == 0) as c_int
    ));
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_CCtx_setParams(
    cctx: *mut ZSTD_CCtx,
    params: ZSTD_parameters,
) -> usize {
    FORWARD_IF_ERROR!(ZSTD_checkCParams(params.cParams));
    FORWARD_IF_ERROR!(ZSTD_CCtx_setFParams(cctx, params.fParams));
    FORWARD_IF_ERROR!(ZSTD_CCtx_setCParams(cctx, params.cParams));
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_CCtx_setPledgedSrcSize(
    cctx: *mut ZSTD_CCtx,
    pledgedSrcSize: c_ulonglong,
) -> usize {
    RETURN_ERROR_IF!((*cctx).streamStage != zcss_init, STAGE_WRONG);
    (*cctx).pledgedSrcSizePlusOne = pledgedSrcSize + 1;
    0
}

unsafe fn ZSTD_initLocalDict(cctx: *mut ZSTD_CCtx) -> usize {
    let dl: *mut ZSTD_localDict = &mut (*cctx).localDict;
    if (*dl).dict.is_null() {
        debug_assert!((*dl).dictBuffer.is_null());
        debug_assert!((*dl).cdict.is_null());
        debug_assert!((*dl).dictSize == 0);
        return 0;
    }
    if !(*dl).cdict.is_null() {
        debug_assert!((*cctx).cdict == (*dl).cdict as *const ZSTD_CDict);
        return 0;
    }
    debug_assert!((*dl).dictSize > 0);
    debug_assert!((*cctx).cdict.is_null());
    debug_assert!((*cctx).prefixDict.dict.is_null());

    (*dl).cdict = ZSTD_createCDict_advanced2(
        (*dl).dict,
        (*dl).dictSize,
        ZSTD_dlm_byRef,
        (*dl).dictContentType,
        &(*cctx).requestedParams,
        (*cctx).customMem,
    );
    RETURN_ERROR_IF!((*dl).cdict.is_null(), MEMORY_ALLOCATION);
    (*cctx).cdict = (*dl).cdict;
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_CCtx_loadDictionary_advanced(
    cctx: *mut ZSTD_CCtx,
    dict: *const c_void,
    dictSize: usize,
    dictLoadMethod: ZSTD_dictLoadMethod_e,
    dictContentType: ZSTD_dictContentType_e,
) -> usize {
    RETURN_ERROR_IF!((*cctx).streamStage != zcss_init, STAGE_WRONG);
    ZSTD_clearAllDicts(cctx);
    if dict.is_null() || dictSize == 0 {
        return 0;
    }
    if dictLoadMethod == ZSTD_dlm_byRef {
        (*cctx).localDict.dict = dict;
    } else {
        let dictBuffer: *mut c_void;
        RETURN_ERROR_IF!((*cctx).staticSize != 0, MEMORY_ALLOCATION);
        dictBuffer = zstd_custom_malloc(dictSize, (*cctx).customMem);
        RETURN_ERROR_IF!(dictBuffer.is_null(), MEMORY_ALLOCATION);
        memcpy(dictBuffer, dict, dictSize);
        (*cctx).localDict.dictBuffer = dictBuffer;
        (*cctx).localDict.dict = dictBuffer;
    }
    (*cctx).localDict.dictSize = dictSize;
    (*cctx).localDict.dictContentType = dictContentType;
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_CCtx_loadDictionary_byReference(
    cctx: *mut ZSTD_CCtx,
    dict: *const c_void,
    dictSize: usize,
) -> usize {
    ZSTD_CCtx_loadDictionary_advanced(cctx, dict, dictSize, ZSTD_dlm_byRef, ZSTD_dct_auto)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_CCtx_loadDictionary(
    cctx: *mut ZSTD_CCtx,
    dict: *const c_void,
    dictSize: usize,
) -> usize {
    ZSTD_CCtx_loadDictionary_advanced(cctx, dict, dictSize, ZSTD_dlm_byCopy, ZSTD_dct_auto)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_CCtx_refCDict(
    cctx: *mut ZSTD_CCtx,
    cdict: *const ZSTD_CDict,
) -> usize {
    RETURN_ERROR_IF!((*cctx).streamStage != zcss_init, STAGE_WRONG);
    ZSTD_clearAllDicts(cctx);
    (*cctx).cdict = cdict;
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_CCtx_refThreadPool(
    cctx: *mut ZSTD_CCtx,
    pool: *mut ZSTD_threadPool,
) -> usize {
    RETURN_ERROR_IF!((*cctx).streamStage != zcss_init, STAGE_WRONG);
    (*cctx).pool = pool;
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_CCtx_refPrefix(
    cctx: *mut ZSTD_CCtx,
    prefix: *const c_void,
    prefixSize: usize,
) -> usize {
    ZSTD_CCtx_refPrefix_advanced(cctx, prefix, prefixSize, ZSTD_dct_rawContent)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_CCtx_refPrefix_advanced(
    cctx: *mut ZSTD_CCtx,
    prefix: *const c_void,
    prefixSize: usize,
    dictContentType: ZSTD_dictContentType_e,
) -> usize {
    RETURN_ERROR_IF!((*cctx).streamStage != zcss_init, STAGE_WRONG);
    ZSTD_clearAllDicts(cctx);
    if !prefix.is_null() && prefixSize > 0 {
        (*cctx).prefixDict.dict = prefix;
        (*cctx).prefixDict.dictSize = prefixSize;
        (*cctx).prefixDict.dictContentType = dictContentType;
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_CCtx_reset(cctx: *mut ZSTD_CCtx, reset: ZSTD_ResetDirective) -> usize {
    if (reset == ZSTD_reset_session_only) || (reset == ZSTD_reset_session_and_parameters) {
        (*cctx).streamStage = zcss_init;
        (*cctx).pledgedSrcSizePlusOne = 0;
    }
    if (reset == ZSTD_reset_parameters) || (reset == ZSTD_reset_session_and_parameters) {
        RETURN_ERROR_IF!((*cctx).streamStage != zcss_init, STAGE_WRONG);
        ZSTD_clearAllDicts(cctx);
        return ZSTD_CCtxParams_reset(&mut (*cctx).requestedParams);
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_checkCParams(cParams: ZSTD_compressionParameters) -> usize {
    BOUNDCHECK!(ZSTD_c_windowLog, cParams.windowLog as c_int);
    BOUNDCHECK!(ZSTD_c_chainLog, cParams.chainLog as c_int);
    BOUNDCHECK!(ZSTD_c_hashLog, cParams.hashLog as c_int);
    BOUNDCHECK!(ZSTD_c_searchLog, cParams.searchLog as c_int);
    BOUNDCHECK!(ZSTD_c_minMatch, cParams.minMatch as c_int);
    BOUNDCHECK!(ZSTD_c_targetLength, cParams.targetLength as c_int);
    BOUNDCHECK!(ZSTD_c_strategy, cParams.strategy as c_int);
    0
}

unsafe fn ZSTD_clampCParams(mut cParams: ZSTD_compressionParameters) -> ZSTD_compressionParameters {
    macro_rules! CLAMP_U32 {
        ($cparam:expr, $val:expr) => {{
            let bounds = ZSTD_cParam_getBounds($cparam);
            if ($val as c_int) < bounds.lowerBound {
                $val = bounds.lowerBound as u32;
            } else if ($val as c_int) > bounds.upperBound {
                $val = bounds.upperBound as u32;
            }
        }};
    }
    CLAMP_U32!(ZSTD_c_windowLog, cParams.windowLog);
    CLAMP_U32!(ZSTD_c_chainLog, cParams.chainLog);
    CLAMP_U32!(ZSTD_c_hashLog, cParams.hashLog);
    CLAMP_U32!(ZSTD_c_searchLog, cParams.searchLog);
    CLAMP_U32!(ZSTD_c_minMatch, cParams.minMatch);
    CLAMP_U32!(ZSTD_c_targetLength, cParams.targetLength);
    {
        let bounds = ZSTD_cParam_getBounds(ZSTD_c_strategy);
        if (cParams.strategy as c_int) < bounds.lowerBound {
            cParams.strategy = bounds.lowerBound as ZSTD_strategy;
        } else if (cParams.strategy as c_int) > bounds.upperBound {
            cParams.strategy = bounds.upperBound as ZSTD_strategy;
        }
    }
    cParams
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_cycleLog(hashLog: U32, strat: ZSTD_strategy) -> U32 {
    let btScale: U32 = ((strat as U32) >= (ZSTD_btlazy2 as U32)) as U32;
    hashLog - btScale
}

unsafe fn ZSTD_dictAndWindowLog(windowLog: U32, srcSize: U64, dictSize: U64) -> U32 {
    let maxWindowSize: U64 = 1u64 << ZSTD_WINDOWLOG_MAX;
    if dictSize == 0 {
        return windowLog;
    }
    debug_assert!(windowLog <= ZSTD_WINDOWLOG_MAX as u32);
    debug_assert!(srcSize != ZSTD_CONTENTSIZE_UNKNOWN);
    {
        let windowSize: U64 = 1u64 << windowLog;
        let dictAndWindowSize: U64 = dictSize + windowSize;
        if windowSize >= dictSize + srcSize {
            windowLog
        } else if dictAndWindowSize >= maxWindowSize {
            ZSTD_WINDOWLOG_MAX as U32
        } else {
            ZSTD_highbit32((dictAndWindowSize as U32) - 1) + 1
        }
    }
}

unsafe fn ZSTD_adjustCParams_internal(
    mut cPar: ZSTD_compressionParameters,
    mut srcSize: c_ulonglong,
    mut dictSize: usize,
    mode: ZSTD_CParamMode_e,
    mut useRowMatchFinder: ZSTD_ParamSwitch_e,
) -> ZSTD_compressionParameters {
    let minSrcSize: U64 = 513;
    let maxWindowResize: U64 = 1u64 << (ZSTD_WINDOWLOG_MAX - 1);
    debug_assert!(ZSTD_checkCParams(cPar) == 0);

    match mode {
        ZSTD_cpm_unknown | ZSTD_cpm_noAttachDict => {}
        ZSTD_cpm_createCDict => {
            if dictSize != 0 && srcSize == ZSTD_CONTENTSIZE_UNKNOWN {
                srcSize = minSrcSize;
            }
        }
        ZSTD_cpm_attachDict => {
            dictSize = 0;
        }
        _ => {
            debug_assert!(false);
        }
    }

    if (srcSize as U64 <= maxWindowResize) && (dictSize as U64 <= maxWindowResize) {
        let tSize: U32 = (srcSize + dictSize as u64) as U32;
        const hashSizeMin: U32 = 1 << ZSTD_HASHLOG_MIN;
        let srcLog: U32 = if tSize < hashSizeMin {
            ZSTD_HASHLOG_MIN as U32
        } else {
            ZSTD_highbit32(tSize - 1) + 1
        };
        if cPar.windowLog > srcLog {
            cPar.windowLog = srcLog;
        }
    }
    if srcSize != ZSTD_CONTENTSIZE_UNKNOWN {
        let dictAndWindowLog = ZSTD_dictAndWindowLog(cPar.windowLog, srcSize as U64, dictSize as U64);
        let cycleLog = ZSTD_cycleLog(cPar.chainLog, cPar.strategy);
        if cPar.hashLog > dictAndWindowLog + 1 {
            cPar.hashLog = dictAndWindowLog + 1;
        }
        if cycleLog > dictAndWindowLog {
            cPar.chainLog -= cycleLog - dictAndWindowLog;
        }
    }

    if cPar.windowLog < ZSTD_WINDOWLOG_ABSOLUTEMIN {
        cPar.windowLog = ZSTD_WINDOWLOG_ABSOLUTEMIN;
    }

    if mode == ZSTD_cpm_createCDict && ZSTD_CDictIndicesAreTagged(&cPar) != 0 {
        let maxShortCacheHashLog: U32 = 32 - ZSTD_SHORT_CACHE_TAG_BITS;
        if cPar.hashLog > maxShortCacheHashLog {
            cPar.hashLog = maxShortCacheHashLog;
        }
        if cPar.chainLog > maxShortCacheHashLog {
            cPar.chainLog = maxShortCacheHashLog;
        }
    }

    if useRowMatchFinder == ZSTD_ps_auto {
        useRowMatchFinder = ZSTD_ps_enable;
    }

    if ZSTD_rowMatchFinderUsed(cPar.strategy, useRowMatchFinder) != 0 {
        let rowLog: U32 = BOUNDED_u32(4, cPar.searchLog, 6);
        let maxRowHashLog: U32 = 32 - ZSTD_ROW_HASH_TAG_BITS;
        let maxHashLog: U32 = maxRowHashLog + rowLog;
        debug_assert!(cPar.hashLog >= rowLog);
        if cPar.hashLog > maxHashLog {
            cPar.hashLog = maxHashLog;
        }
    }

    cPar
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_adjustCParams(
    mut cPar: ZSTD_compressionParameters,
    mut srcSize: c_ulonglong,
    dictSize: usize,
) -> ZSTD_compressionParameters {
    cPar = ZSTD_clampCParams(cPar);
    if srcSize == 0 {
        srcSize = ZSTD_CONTENTSIZE_UNKNOWN;
    }
    ZSTD_adjustCParams_internal(cPar, srcSize, dictSize, ZSTD_cpm_unknown, ZSTD_ps_auto)
}

unsafe fn ZSTD_overrideCParams(
    cParams: *mut ZSTD_compressionParameters,
    overrides: *const ZSTD_compressionParameters,
) {
    if (*overrides).windowLog != 0 {
        (*cParams).windowLog = (*overrides).windowLog;
    }
    if (*overrides).hashLog != 0 {
        (*cParams).hashLog = (*overrides).hashLog;
    }
    if (*overrides).chainLog != 0 {
        (*cParams).chainLog = (*overrides).chainLog;
    }
    if (*overrides).searchLog != 0 {
        (*cParams).searchLog = (*overrides).searchLog;
    }
    if (*overrides).minMatch != 0 {
        (*cParams).minMatch = (*overrides).minMatch;
    }
    if (*overrides).targetLength != 0 {
        (*cParams).targetLength = (*overrides).targetLength;
    }
    if (*overrides).strategy != 0 {
        (*cParams).strategy = (*overrides).strategy;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_getCParamsFromCCtxParams(
    CCtxParams: *const ZSTD_CCtx_params,
    mut srcSizeHint: U64,
    dictSize: usize,
    mode: ZSTD_CParamMode_e,
) -> ZSTD_compressionParameters {
    let mut cParams: ZSTD_compressionParameters;
    if srcSizeHint == ZSTD_CONTENTSIZE_UNKNOWN && (*CCtxParams).srcSizeHint > 0 {
        debug_assert!((*CCtxParams).srcSizeHint >= 0);
        srcSizeHint = (*CCtxParams).srcSizeHint as U64;
    }
    cParams = ZSTD_getCParams_internal((*CCtxParams).compressionLevel, srcSizeHint, dictSize, mode);
    if (*CCtxParams).ldmParams.enableLdm == ZSTD_ps_enable {
        cParams.windowLog = ZSTD_LDM_DEFAULT_WINDOW_LOG;
    }
    ZSTD_overrideCParams(&mut cParams, &(*CCtxParams).cParams);
    debug_assert!(ZSTD_checkCParams(cParams) == 0);
    ZSTD_adjustCParams_internal(
        cParams,
        srcSizeHint,
        dictSize,
        mode,
        (*CCtxParams).useRowMatchFinder,
    )
}

unsafe fn ZSTD_sizeof_matchState(
    cParams: *const ZSTD_compressionParameters,
    useRowMatchFinder: ZSTD_ParamSwitch_e,
    enableDedicatedDictSearch: c_int,
    forCCtx: U32,
) -> usize {
    let chainSize: usize = if ZSTD_allocateChainTable(
        (*cParams).strategy,
        useRowMatchFinder,
        (enableDedicatedDictSearch != 0 && forCCtx == 0) as U32,
    ) != 0
    {
        1usize << (*cParams).chainLog
    } else {
        0
    };
    let hSize: usize = 1usize << (*cParams).hashLog;
    let hashLog3: U32 = if forCCtx != 0 && (*cParams).minMatch == 3 {
        core::cmp::min(ZSTD_HASHLOG3_MAX, (*cParams).windowLog)
    } else {
        0
    };
    let h3Size: usize = if hashLog3 != 0 { 1usize << hashLog3 } else { 0 };
    let tableSpace: usize = chainSize * core::mem::size_of::<U32>()
        + hSize * core::mem::size_of::<U32>()
        + h3Size * core::mem::size_of::<U32>();
    let optPotentialSpace: usize =
        ZSTD_cwksp_aligned64_alloc_size((MaxML as usize + 1) * core::mem::size_of::<U32>())
            + ZSTD_cwksp_aligned64_alloc_size((MaxLL as usize + 1) * core::mem::size_of::<U32>())
            + ZSTD_cwksp_aligned64_alloc_size((MaxOff as usize + 1) * core::mem::size_of::<U32>())
            + ZSTD_cwksp_aligned64_alloc_size((1usize << Litbits) * core::mem::size_of::<U32>())
            + ZSTD_cwksp_aligned64_alloc_size(ZSTD_OPT_SIZE * core::mem::size_of::<ZSTD_match_t>())
            + ZSTD_cwksp_aligned64_alloc_size(
                ZSTD_OPT_SIZE * core::mem::size_of::<ZSTD_optimal_t>(),
            );
    let lazyAdditionalSpace: usize =
        if ZSTD_rowMatchFinderUsed((*cParams).strategy, useRowMatchFinder) != 0 {
            ZSTD_cwksp_aligned64_alloc_size(hSize)
        } else {
            0
        };
    let optSpace: usize = if forCCtx != 0 && ((*cParams).strategy >= ZSTD_btopt) {
        optPotentialSpace
    } else {
        0
    };
    let slackSpace: usize = ZSTD_cwksp_slack_space_required();

    debug_assert!(useRowMatchFinder != ZSTD_ps_auto);
    tableSpace + optSpace + slackSpace + lazyAdditionalSpace
}

unsafe fn ZSTD_maxNbSeq(blockSize: usize, minMatch: c_uint, useSequenceProducer: c_int) -> usize {
    let divider: U32 = if minMatch == 3 || useSequenceProducer != 0 {
        3
    } else {
        4
    };
    blockSize / divider as usize
}

unsafe fn ZSTD_estimateCCtxSize_usingCCtxParams_internal(
    cParams: *const ZSTD_compressionParameters,
    ldmParams: *const ldmParams_t,
    isStatic: c_int,
    useRowMatchFinder: ZSTD_ParamSwitch_e,
    buffInSize: usize,
    buffOutSize: usize,
    pledgedSrcSize: U64,
    useSequenceProducer: c_int,
    maxBlockSize: usize,
) -> usize {
    let windowSize: usize = {
        let val = 1u64 << (*cParams).windowLog;
        let v = if val < pledgedSrcSize { val } else { pledgedSrcSize };
        (if 1u64 > v { 1u64 } else { v }) as usize
    };
    let blockSize: usize = core::cmp::min(ZSTD_resolveMaxBlockSize(maxBlockSize), windowSize);
    let maxNbSeq: usize = ZSTD_maxNbSeq(blockSize, (*cParams).minMatch, useSequenceProducer);
    let tokenSpace: usize = ZSTD_cwksp_alloc_size(WILDCOPY_OVERLENGTH + blockSize)
        + ZSTD_cwksp_aligned64_alloc_size(maxNbSeq * core::mem::size_of::<SeqDef>())
        + 3 * ZSTD_cwksp_alloc_size(maxNbSeq * core::mem::size_of::<u8>());
    let tmpWorkSpace: usize = ZSTD_cwksp_alloc_size(TMP_WORKSPACE_SIZE);
    let blockStateSpace: usize =
        2 * ZSTD_cwksp_alloc_size(core::mem::size_of::<ZSTD_compressedBlockState_t>());
    let matchStateSize: usize = ZSTD_sizeof_matchState(cParams, useRowMatchFinder, 0, 1);

    let ldmSpace: usize = ZSTD_ldm_getTableSize(*ldmParams);
    let maxNbLdmSeq: usize = ZSTD_ldm_getMaxNbSeq(*ldmParams, blockSize);
    let ldmSeqSpace: usize = if (*ldmParams).enableLdm == ZSTD_ps_enable {
        ZSTD_cwksp_aligned64_alloc_size(maxNbLdmSeq * core::mem::size_of::<rawSeq>())
    } else {
        0
    };

    let bufferSpace: usize =
        ZSTD_cwksp_alloc_size(buffInSize) + ZSTD_cwksp_alloc_size(buffOutSize);

    let cctxSpace: usize = if isStatic != 0 {
        ZSTD_cwksp_alloc_size(core::mem::size_of::<ZSTD_CCtx>())
    } else {
        0
    };

    let maxNbExternalSeq: usize = ZSTD_sequenceBound(blockSize);
    let externalSeqSpace: usize = if useSequenceProducer != 0 {
        ZSTD_cwksp_aligned64_alloc_size(maxNbExternalSeq * core::mem::size_of::<ZSTD_Sequence>())
    } else {
        0
    };

    let neededSpace: usize = cctxSpace
        + tmpWorkSpace
        + blockStateSpace
        + ldmSpace
        + ldmSeqSpace
        + matchStateSize
        + tokenSpace
        + bufferSpace
        + externalSeqSpace;

    neededSpace
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_estimateCCtxSize_usingCCtxParams(
    params: *const ZSTD_CCtx_params,
) -> usize {
    let cParams =
        ZSTD_getCParamsFromCCtxParams(params, ZSTD_CONTENTSIZE_UNKNOWN, 0, ZSTD_cpm_noAttachDict);
    let useRowMatchFinder =
        ZSTD_resolveRowMatchFinderMode((*params).useRowMatchFinder, &cParams);
    RETURN_ERROR_IF!((*params).nbWorkers > 0, GENERIC);
    ZSTD_estimateCCtxSize_usingCCtxParams_internal(
        &cParams,
        &(*params).ldmParams,
        1,
        useRowMatchFinder,
        0,
        0,
        ZSTD_CONTENTSIZE_UNKNOWN,
        ZSTD_hasExtSeqProd(params),
        (*params).maxBlockSize,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_estimateCCtxSize_usingCParams(
    cParams: ZSTD_compressionParameters,
) -> usize {
    let mut initialParams = ZSTD_makeCCtxParamsFromCParams(cParams);
    if ZSTD_rowMatchFinderSupported(cParams.strategy) != 0 {
        let noRowCCtxSize: usize;
        let rowCCtxSize: usize;
        initialParams.useRowMatchFinder = ZSTD_ps_disable;
        noRowCCtxSize = ZSTD_estimateCCtxSize_usingCCtxParams(&initialParams);
        initialParams.useRowMatchFinder = ZSTD_ps_enable;
        rowCCtxSize = ZSTD_estimateCCtxSize_usingCCtxParams(&initialParams);
        core::cmp::max(noRowCCtxSize, rowCCtxSize)
    } else {
        ZSTD_estimateCCtxSize_usingCCtxParams(&initialParams)
    }
}

unsafe fn ZSTD_estimateCCtxSize_internal(compressionLevel: c_int) -> usize {
    let mut tier = 0;
    let mut largestSize: usize = 0;
    let srcSizeTiers: [c_ulonglong; 4] = [
        16 * 1024,
        128 * 1024,
        256 * 1024,
        ZSTD_CONTENTSIZE_UNKNOWN,
    ];
    while tier < 4 {
        let cParams =
            ZSTD_getCParams_internal(compressionLevel, srcSizeTiers[tier], 0, ZSTD_cpm_noAttachDict);
        largestSize = core::cmp::max(ZSTD_estimateCCtxSize_usingCParams(cParams), largestSize);
        tier += 1;
    }
    largestSize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_estimateCCtxSize(compressionLevel: c_int) -> usize {
    let mut level;
    let mut memBudget: usize = 0;
    level = core::cmp::min(compressionLevel, 1);
    while level <= compressionLevel {
        let newMB = ZSTD_estimateCCtxSize_internal(level);
        if newMB > memBudget {
            memBudget = newMB;
        }
        level += 1;
    }
    memBudget
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_estimateCStreamSize_usingCCtxParams(
    params: *const ZSTD_CCtx_params,
) -> usize {
    RETURN_ERROR_IF!((*params).nbWorkers > 0, GENERIC);
    {
        let cParams = ZSTD_getCParamsFromCCtxParams(
            params,
            ZSTD_CONTENTSIZE_UNKNOWN,
            0,
            ZSTD_cpm_noAttachDict,
        );
        let blockSize: usize = core::cmp::min(
            ZSTD_resolveMaxBlockSize((*params).maxBlockSize),
            1usize << cParams.windowLog,
        );
        let inBuffSize: usize = if (*params).inBufferMode == ZSTD_bm_buffered {
            (1usize << cParams.windowLog) + blockSize
        } else {
            0
        };
        let outBuffSize: usize = if (*params).outBufferMode == ZSTD_bm_buffered {
            ZSTD_compressBound(blockSize) + 1
        } else {
            0
        };
        let useRowMatchFinder =
            ZSTD_resolveRowMatchFinderMode((*params).useRowMatchFinder, &(*params).cParams);
        ZSTD_estimateCCtxSize_usingCCtxParams_internal(
            &cParams,
            &(*params).ldmParams,
            1,
            useRowMatchFinder,
            inBuffSize,
            outBuffSize,
            ZSTD_CONTENTSIZE_UNKNOWN,
            ZSTD_hasExtSeqProd(params),
            (*params).maxBlockSize,
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_estimateCStreamSize_usingCParams(
    cParams: ZSTD_compressionParameters,
) -> usize {
    let mut initialParams = ZSTD_makeCCtxParamsFromCParams(cParams);
    if ZSTD_rowMatchFinderSupported(cParams.strategy) != 0 {
        let noRowCCtxSize: usize;
        let rowCCtxSize: usize;
        initialParams.useRowMatchFinder = ZSTD_ps_disable;
        noRowCCtxSize = ZSTD_estimateCStreamSize_usingCCtxParams(&initialParams);
        initialParams.useRowMatchFinder = ZSTD_ps_enable;
        rowCCtxSize = ZSTD_estimateCStreamSize_usingCCtxParams(&initialParams);
        core::cmp::max(noRowCCtxSize, rowCCtxSize)
    } else {
        ZSTD_estimateCStreamSize_usingCCtxParams(&initialParams)
    }
}

unsafe fn ZSTD_estimateCStreamSize_internal(compressionLevel: c_int) -> usize {
    let cParams =
        ZSTD_getCParams_internal(compressionLevel, ZSTD_CONTENTSIZE_UNKNOWN, 0, ZSTD_cpm_noAttachDict);
    ZSTD_estimateCStreamSize_usingCParams(cParams)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_estimateCStreamSize(compressionLevel: c_int) -> usize {
    let mut level;
    let mut memBudget: usize = 0;
    level = core::cmp::min(compressionLevel, 1);
    while level <= compressionLevel {
        let newMB = ZSTD_estimateCStreamSize_internal(level);
        if newMB > memBudget {
            memBudget = newMB;
        }
        level += 1;
    }
    memBudget
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_getFrameProgression(cctx: *const ZSTD_CCtx) -> ZSTD_frameProgression {
    {
        let mut fp: ZSTD_frameProgression = core::mem::zeroed();
        let buffered: usize = if (*cctx).inBuff.is_null() {
            0
        } else {
            (*cctx).inBuffPos - (*cctx).inToCompress
        };
        if buffered != 0 {
            debug_assert!((*cctx).inBuffPos >= (*cctx).inToCompress);
        }
        debug_assert!(buffered <= ZSTD_BLOCKSIZE_MAX);
        fp.ingested = (*cctx).consumedSrcSize + buffered as u64;
        fp.consumed = (*cctx).consumedSrcSize;
        fp.produced = (*cctx).producedCSize;
        fp.flushed = (*cctx).producedCSize;
        fp.currentJobID = 0;
        fp.nbActiveWorkers = 0;
        fp
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_toFlushNow(cctx: *mut ZSTD_CCtx) -> usize {
    let _ = cctx;
    0
}

unsafe fn ZSTD_assertEqualCParams(
    cParams1: ZSTD_compressionParameters,
    cParams2: ZSTD_compressionParameters,
) {
    let _ = cParams1;
    let _ = cParams2;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_reset_compressedBlockState(bs: *mut ZSTD_compressedBlockState_t) {
    let mut i = 0;
    while i < ZSTD_REP_NUM {
        (*bs).rep[i] = repStartValue[i];
        i += 1;
    }
    (*bs).entropy.huf.repeatMode = HUF_repeat_none;
    (*bs).entropy.fse.offcode_repeatMode = FSE_repeat_none;
    (*bs).entropy.fse.matchlength_repeatMode = FSE_repeat_none;
    (*bs).entropy.fse.litlength_repeatMode = FSE_repeat_none;
}

unsafe fn ZSTD_invalidateMatchState(ms: *mut ZSTD_MatchState_t) {
    ZSTD_window_clear(&mut (*ms).window);
    (*ms).nextToUpdate = (*ms).window.dictLimit;
    (*ms).loadedDictEnd = 0;
    (*ms).opt.litLengthSum = 0;
    (*ms).dictMatchState = core::ptr::null();
}

// ZSTD_compResetPolicy_e
type ZSTD_compResetPolicy_e = u32;
const ZSTDcrp_makeClean: ZSTD_compResetPolicy_e = 0;
const ZSTDcrp_leaveDirty: ZSTD_compResetPolicy_e = 1;

// ZSTD_indexResetPolicy_e
type ZSTD_indexResetPolicy_e = u32;
const ZSTDirp_continue: ZSTD_indexResetPolicy_e = 0;
const ZSTDirp_reset: ZSTD_indexResetPolicy_e = 1;

// ZSTD_resetTarget_e
type ZSTD_resetTarget_e = u32;
const ZSTD_resetTarget_CDict: ZSTD_resetTarget_e = 0;
const ZSTD_resetTarget_CCtx: ZSTD_resetTarget_e = 1;

unsafe fn ZSTD_bitmix(mut val: U64, len: U64) -> U64 {
    val ^= ZSTD_rotateRight_U64(val, 49) ^ ZSTD_rotateRight_U64(val, 24);
    val = val.wrapping_mul(0x9FB21C651E98DF25u64);
    val ^= (val >> 35).wrapping_add(len);
    val = val.wrapping_mul(0x9FB21C651E98DF25u64);
    val ^ (val >> 28)
}

unsafe fn ZSTD_advanceHashSalt(ms: *mut ZSTD_MatchState_t) {
    (*ms).hashSalt = ZSTD_bitmix((*ms).hashSalt, 8) ^ ZSTD_bitmix((*ms).hashSaltEntropy as U64, 4);
}

unsafe fn ZSTD_reset_matchState(
    ms: *mut ZSTD_MatchState_t,
    ws: *mut ZSTD_cwksp,
    cParams: *const ZSTD_compressionParameters,
    useRowMatchFinder: ZSTD_ParamSwitch_e,
    crp: ZSTD_compResetPolicy_e,
    forceResetIndex: ZSTD_indexResetPolicy_e,
    forWho: ZSTD_resetTarget_e,
) -> usize {
    let chainSize: usize = if ZSTD_allocateChainTable(
        (*cParams).strategy,
        useRowMatchFinder,
        ((*ms).dedicatedDictSearch != 0 && (forWho == ZSTD_resetTarget_CDict)) as U32,
    ) != 0
    {
        1usize << (*cParams).chainLog
    } else {
        0
    };
    let hSize: usize = 1usize << (*cParams).hashLog;
    let hashLog3: U32 = if (forWho == ZSTD_resetTarget_CCtx) && (*cParams).minMatch == 3 {
        core::cmp::min(ZSTD_HASHLOG3_MAX, (*cParams).windowLog)
    } else {
        0
    };
    let h3Size: usize = if hashLog3 != 0 { 1usize << hashLog3 } else { 0 };

    debug_assert!(useRowMatchFinder != ZSTD_ps_auto);
    if forceResetIndex == ZSTDirp_reset {
        ZSTD_window_init(&mut (*ms).window);
        ZSTD_cwksp_mark_tables_dirty(ws);
    }

    (*ms).hashLog3 = hashLog3;
    (*ms).lazySkipping = 0;

    ZSTD_invalidateMatchState(ms);

    debug_assert!(ZSTD_cwksp_reserve_failed(ws) == 0);

    ZSTD_cwksp_clear_tables(ws);

    (*ms).hashTable = ZSTD_cwksp_reserve_table(ws, hSize * core::mem::size_of::<U32>()) as *mut U32;
    (*ms).chainTable =
        ZSTD_cwksp_reserve_table(ws, chainSize * core::mem::size_of::<U32>()) as *mut U32;
    (*ms).hashTable3 =
        ZSTD_cwksp_reserve_table(ws, h3Size * core::mem::size_of::<U32>()) as *mut U32;
    RETURN_ERROR_IF!(ZSTD_cwksp_reserve_failed(ws) != 0, MEMORY_ALLOCATION);

    if crp != ZSTDcrp_leaveDirty {
        ZSTD_cwksp_clean_tables(ws);
    }

    if ZSTD_rowMatchFinderUsed((*cParams).strategy, useRowMatchFinder) != 0 {
        let tagTableSize: usize = hSize;
        if forWho == ZSTD_resetTarget_CCtx {
            (*ms).tagTable = ZSTD_cwksp_reserve_aligned_init_once(ws, tagTableSize) as *mut u8;
            ZSTD_advanceHashSalt(ms);
        } else {
            (*ms).tagTable = ZSTD_cwksp_reserve_aligned64(ws, tagTableSize) as *mut u8;
            memset((*ms).tagTable as *mut c_void, 0, tagTableSize);
            (*ms).hashSalt = 0;
        }
        {
            let rowLog: U32 = BOUNDED_u32(4, (*cParams).searchLog, 6);
            debug_assert!((*cParams).hashLog >= rowLog);
            (*ms).rowHashLog = (*cParams).hashLog - rowLog;
        }
    }

    if (forWho == ZSTD_resetTarget_CCtx) && ((*cParams).strategy >= ZSTD_btopt) {
        (*ms).opt.litFreq = ZSTD_cwksp_reserve_aligned64(
            ws,
            (1usize << Litbits) * core::mem::size_of::<c_uint>(),
        ) as *mut u32;
        (*ms).opt.litLengthFreq = ZSTD_cwksp_reserve_aligned64(
            ws,
            (MaxLL as usize + 1) * core::mem::size_of::<c_uint>(),
        ) as *mut u32;
        (*ms).opt.matchLengthFreq = ZSTD_cwksp_reserve_aligned64(
            ws,
            (MaxML as usize + 1) * core::mem::size_of::<c_uint>(),
        ) as *mut u32;
        (*ms).opt.offCodeFreq = ZSTD_cwksp_reserve_aligned64(
            ws,
            (MaxOff as usize + 1) * core::mem::size_of::<c_uint>(),
        ) as *mut u32;
        (*ms).opt.matchTable = ZSTD_cwksp_reserve_aligned64(
            ws,
            ZSTD_OPT_SIZE * core::mem::size_of::<ZSTD_match_t>(),
        ) as *mut ZSTD_match_t;
        (*ms).opt.priceTable = ZSTD_cwksp_reserve_aligned64(
            ws,
            ZSTD_OPT_SIZE * core::mem::size_of::<ZSTD_optimal_t>(),
        ) as *mut ZSTD_optimal_t;
    }

    (*ms).cParams = *cParams;

    RETURN_ERROR_IF!(ZSTD_cwksp_reserve_failed(ws) != 0, MEMORY_ALLOCATION);
    0
}

unsafe fn ZSTD_indexTooCloseToMax(w: ZSTD_window_t) -> c_int {
    (((w.nextSrc as usize) - (w.base as usize)) > (ZSTD_CURRENT_MAX() as usize - (16 * 1024 * 1024)))
        as c_int
}

unsafe fn ZSTD_dictTooBig(loadedDictSize: usize) -> c_int {
    (loadedDictSize > ZSTD_CHUNKSIZE_MAX() as usize) as c_int
}

unsafe fn ZSTD_resetCCtx_internal(
    zc: *mut ZSTD_CCtx,
    mut params: *const ZSTD_CCtx_params,
    pledgedSrcSize: U64,
    loadedDictSize: usize,
    crp: ZSTD_compResetPolicy_e,
    zbuff: ZSTD_buffered_policy_e,
) -> usize {
    let ws: *mut ZSTD_cwksp = &mut (*zc).workspace;
    debug_assert!(ZSTD_isError(ZSTD_checkCParams((*params).cParams)) == 0);

    (*zc).isFirstBlock = 1;

    core::ptr::copy_nonoverlapping(params, &mut (*zc).appliedParams, 1);
    params = &(*zc).appliedParams;

    debug_assert!((*params).useRowMatchFinder != ZSTD_ps_auto);
    debug_assert!((*params).postBlockSplitter != ZSTD_ps_auto);
    debug_assert!((*params).ldmParams.enableLdm != ZSTD_ps_auto);
    debug_assert!((*params).maxBlockSize != 0);
    if (*params).ldmParams.enableLdm == ZSTD_ps_enable {
        ZSTD_ldm_adjustParameters(&mut (*zc).appliedParams.ldmParams, &(*params).cParams);
        debug_assert!((*params).ldmParams.hashLog >= (*params).ldmParams.bucketSizeLog);
        debug_assert!((*params).ldmParams.hashRateLog < 32);
    }

    {
        let windowSize: usize = {
            let val = 1u64 << (*params).cParams.windowLog;
            let v = if val < pledgedSrcSize { val } else { pledgedSrcSize } as usize;
            if 1 > v { 1 } else { v }
        };
        let blockSize: usize = core::cmp::min((*params).maxBlockSize, windowSize);
        let maxNbSeq: usize =
            ZSTD_maxNbSeq(blockSize, (*params).cParams.minMatch, ZSTD_hasExtSeqProd(params));
        let buffOutSize: usize =
            if zbuff == ZSTDb_buffered && (*params).outBufferMode == ZSTD_bm_buffered {
                ZSTD_compressBound(blockSize) + 1
            } else {
                0
            };
        let buffInSize: usize =
            if zbuff == ZSTDb_buffered && (*params).inBufferMode == ZSTD_bm_buffered {
                windowSize + blockSize
            } else {
                0
            };
        let maxNbLdmSeq: usize = ZSTD_ldm_getMaxNbSeq((*params).ldmParams, blockSize);

        let indexTooClose = ZSTD_indexTooCloseToMax((*zc).blockState.matchState.window);
        let dictTooBig = ZSTD_dictTooBig(loadedDictSize);
        let mut needsIndexReset: ZSTD_indexResetPolicy_e =
            if indexTooClose != 0 || dictTooBig != 0 || (*zc).initialized == 0 {
                ZSTDirp_reset
            } else {
                ZSTDirp_continue
            };

        let neededSpace: usize = ZSTD_estimateCCtxSize_usingCCtxParams_internal(
            &(*params).cParams,
            &(*params).ldmParams,
            ((*zc).staticSize != 0) as c_int,
            (*params).useRowMatchFinder,
            buffInSize,
            buffOutSize,
            pledgedSrcSize,
            ZSTD_hasExtSeqProd(params),
            (*params).maxBlockSize,
        );

        FORWARD_IF_ERROR!(neededSpace);

        if (*zc).staticSize == 0 {
            ZSTD_cwksp_bump_oversized_duration(ws, 0);
        }

        {
            let workspaceTooSmall = (ZSTD_cwksp_sizeof(ws) < neededSpace) as c_int;
            let workspaceWasteful = ZSTD_cwksp_check_wasteful(ws, neededSpace);
            let resizeWorkspace = workspaceTooSmall != 0 || workspaceWasteful != 0;

            if resizeWorkspace {
                RETURN_ERROR_IF!((*zc).staticSize != 0, MEMORY_ALLOCATION);

                needsIndexReset = ZSTDirp_reset;

                ZSTD_cwksp_free(ws, (*zc).customMem);
                FORWARD_IF_ERROR!(ZSTD_cwksp_create(ws, neededSpace, (*zc).customMem));

                debug_assert!(
                    ZSTD_cwksp_check_available(
                        ws,
                        2 * core::mem::size_of::<ZSTD_compressedBlockState_t>()
                    ) != 0
                );
                (*zc).blockState.prevCBlock = ZSTD_cwksp_reserve_object(
                    ws,
                    core::mem::size_of::<ZSTD_compressedBlockState_t>(),
                )
                    as *mut ZSTD_compressedBlockState_t;
                RETURN_ERROR_IF!((*zc).blockState.prevCBlock.is_null(), MEMORY_ALLOCATION);
                (*zc).blockState.nextCBlock = ZSTD_cwksp_reserve_object(
                    ws,
                    core::mem::size_of::<ZSTD_compressedBlockState_t>(),
                )
                    as *mut ZSTD_compressedBlockState_t;
                RETURN_ERROR_IF!((*zc).blockState.nextCBlock.is_null(), MEMORY_ALLOCATION);
                (*zc).tmpWorkspace = ZSTD_cwksp_reserve_object(ws, TMP_WORKSPACE_SIZE);
                RETURN_ERROR_IF!((*zc).tmpWorkspace.is_null(), MEMORY_ALLOCATION);
                (*zc).tmpWkspSize = TMP_WORKSPACE_SIZE;
            }
        }

        ZSTD_cwksp_clear(ws);

        (*zc).blockState.matchState.cParams = (*params).cParams;
        (*zc).blockState.matchState.prefetchCDictTables =
            ((*params).prefetchCDictTables == ZSTD_ps_enable) as c_int;
        (*zc).pledgedSrcSizePlusOne = pledgedSrcSize + 1;
        (*zc).consumedSrcSize = 0;
        (*zc).producedCSize = 0;
        if pledgedSrcSize == ZSTD_CONTENTSIZE_UNKNOWN {
            (*zc).appliedParams.fParams.contentSizeFlag = 0;
        }
        (*zc).blockSizeMax = blockSize;

        ZSTD_XXH64_reset(&mut (*zc).xxhState, 0);
        (*zc).stage = ZSTDcs_init;
        (*zc).dictID = 0;
        (*zc).dictContentSize = 0;

        ZSTD_reset_compressedBlockState((*zc).blockState.prevCBlock);

        FORWARD_IF_ERROR!(ZSTD_reset_matchState(
            &mut (*zc).blockState.matchState,
            ws,
            &(*params).cParams,
            (*params).useRowMatchFinder,
            crp,
            needsIndexReset,
            ZSTD_resetTarget_CCtx,
        ));

        (*zc).seqStore.sequencesStart = ZSTD_cwksp_reserve_aligned64(
            ws,
            maxNbSeq * core::mem::size_of::<SeqDef>(),
        ) as *mut SeqDef;

        if (*params).ldmParams.enableLdm == ZSTD_ps_enable {
            let ldmHSize: usize = 1usize << (*params).ldmParams.hashLog;
            (*zc).ldmState.hashTable = ZSTD_cwksp_reserve_aligned64(
                ws,
                ldmHSize * core::mem::size_of::<ldmEntry_t>(),
            ) as *mut ldmEntry_t;
            memset(
                (*zc).ldmState.hashTable as *mut c_void,
                0,
                ldmHSize * core::mem::size_of::<ldmEntry_t>(),
            );
            (*zc).ldmSequences = ZSTD_cwksp_reserve_aligned64(
                ws,
                maxNbLdmSeq * core::mem::size_of::<rawSeq>(),
            ) as *mut rawSeq;
            (*zc).maxNbLdmSequences = maxNbLdmSeq;

            ZSTD_window_init(&mut (*zc).ldmState.window);
            (*zc).ldmState.loadedDictEnd = 0;
        }

        if ZSTD_hasExtSeqProd(params) != 0 {
            let maxNbExternalSeq: usize = ZSTD_sequenceBound(blockSize);
            (*zc).extSeqBufCapacity = maxNbExternalSeq;
            (*zc).extSeqBuf = ZSTD_cwksp_reserve_aligned64(
                ws,
                maxNbExternalSeq * core::mem::size_of::<ZSTD_Sequence>(),
            ) as *mut ZSTD_Sequence;
        }

        (*zc).seqStore.litStart = ZSTD_cwksp_reserve_buffer(ws, blockSize + WILDCOPY_OVERLENGTH);
        (*zc).seqStore.maxNbLit = blockSize;

        (*zc).bufferedPolicy = zbuff;
        (*zc).inBuffSize = buffInSize;
        (*zc).inBuff = ZSTD_cwksp_reserve_buffer(ws, buffInSize) as *mut c_char;
        (*zc).outBuffSize = buffOutSize;
        (*zc).outBuff = ZSTD_cwksp_reserve_buffer(ws, buffOutSize) as *mut c_char;

        if (*params).ldmParams.enableLdm == ZSTD_ps_enable {
            let numBuckets: usize =
                1usize << ((*params).ldmParams.hashLog - (*params).ldmParams.bucketSizeLog);
            (*zc).ldmState.bucketOffsets = ZSTD_cwksp_reserve_buffer(ws, numBuckets);
            memset((*zc).ldmState.bucketOffsets as *mut c_void, 0, numBuckets);
        }

        ZSTD_referenceExternalSequences(zc, core::ptr::null_mut(), 0);
        (*zc).seqStore.maxNbSeq = maxNbSeq;
        (*zc).seqStore.llCode = ZSTD_cwksp_reserve_buffer(ws, maxNbSeq * core::mem::size_of::<u8>());
        (*zc).seqStore.mlCode = ZSTD_cwksp_reserve_buffer(ws, maxNbSeq * core::mem::size_of::<u8>());
        (*zc).seqStore.ofCode = ZSTD_cwksp_reserve_buffer(ws, maxNbSeq * core::mem::size_of::<u8>());

        (*zc).initialized = 1;

        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_invalidateRepCodes(cctx: *mut ZSTD_CCtx) {
    let mut i = 0;
    while i < ZSTD_REP_NUM {
        (*(*cctx).blockState.prevCBlock).rep[i] = 0;
        i += 1;
    }
    debug_assert!(ZSTD_window_hasExtDict((*cctx).blockState.matchState.window) == 0);
}

static attachDictSizeCutoffs: [usize; (ZSTD_STRATEGY_MAX + 1) as usize] = [
    8 * 1024,
    8 * 1024,
    16 * 1024,
    32 * 1024,
    32 * 1024,
    32 * 1024,
    32 * 1024,
    32 * 1024,
    8 * 1024,
    8 * 1024,
];

unsafe fn ZSTD_shouldAttachDict(
    cdict: *const ZSTD_CDict,
    params: *const ZSTD_CCtx_params,
    pledgedSrcSize: U64,
) -> c_int {
    let cdict = as_cdict(cdict);
    let cutoff = attachDictSizeCutoffs[(*cdict).matchState.cParams.strategy as usize];
    let dedicatedDictSearch = (*cdict).matchState.dedicatedDictSearch;
    (dedicatedDictSearch != 0
        || ((pledgedSrcSize <= cutoff as u64
            || pledgedSrcSize == ZSTD_CONTENTSIZE_UNKNOWN
            || (*params).attachDictPref == ZSTD_dictForceAttach)
            && (*params).attachDictPref != ZSTD_dictForceCopy
            && (*params).forceWindow == 0)) as c_int
}

unsafe fn ZSTD_resetCCtx_byAttachingCDict(
    cctx: *mut ZSTD_CCtx,
    cdict: *const ZSTD_CDict,
    mut params: ZSTD_CCtx_params,
    pledgedSrcSize: U64,
    zbuff: ZSTD_buffered_policy_e,
) -> usize {
    let cdict = as_cdict(cdict);
    {
        let mut adjusted_cdict_cParams = (*cdict).matchState.cParams;
        let windowLog = params.cParams.windowLog;
        debug_assert!(windowLog != 0);

        if (*cdict).matchState.dedicatedDictSearch != 0 {
            ZSTD_dedicatedDictSearch_revertCParams(&mut adjusted_cdict_cParams);
        }

        params.cParams = ZSTD_adjustCParams_internal(
            adjusted_cdict_cParams,
            pledgedSrcSize,
            (*cdict).dictContentSize,
            ZSTD_cpm_attachDict,
            params.useRowMatchFinder,
        );
        params.cParams.windowLog = windowLog;
        params.useRowMatchFinder = (*cdict).useRowMatchFinder;
        FORWARD_IF_ERROR!(ZSTD_resetCCtx_internal(
            cctx,
            &params,
            pledgedSrcSize,
            0,
            ZSTDcrp_makeClean,
            zbuff,
        ));
        debug_assert!((*cctx).appliedParams.cParams.strategy == adjusted_cdict_cParams.strategy);
    }

    {
        let cdictEnd: U32 = ((*cdict).matchState.window.nextSrc as usize
            - (*cdict).matchState.window.base as usize) as U32;
        let cdictLen: U32 = cdictEnd - (*cdict).matchState.window.dictLimit;
        if cdictLen == 0 {
            /* don't even attach dictionaries with no contents */
        } else {
            (*cctx).blockState.matchState.dictMatchState = &(*cdict).matchState;

            if (*cctx).blockState.matchState.window.dictLimit < cdictEnd {
                (*cctx).blockState.matchState.window.nextSrc =
                    (*cctx).blockState.matchState.window.base.add(cdictEnd as usize);
                ZSTD_window_clear(&mut (*cctx).blockState.matchState.window);
            }
            (*cctx).blockState.matchState.loadedDictEnd =
                (*cctx).blockState.matchState.window.dictLimit;
        }
    }

    (*cctx).dictID = (*cdict).dictID;
    (*cctx).dictContentSize = (*cdict).dictContentSize;

    memcpy(
        (*cctx).blockState.prevCBlock as *mut c_void,
        &(*cdict).cBlockState as *const _ as *const c_void,
        core::mem::size_of::<ZSTD_compressedBlockState_t>(),
    );

    0
}

unsafe fn ZSTD_copyCDictTableIntoCCtx(
    dst: *mut U32,
    src: *const U32,
    tableSize: usize,
    cParams: *const ZSTD_compressionParameters,
) {
    if ZSTD_CDictIndicesAreTagged(cParams) != 0 {
        let mut i = 0;
        while i < tableSize {
            let taggedIndex = *src.add(i);
            let index = taggedIndex >> ZSTD_SHORT_CACHE_TAG_BITS;
            *dst.add(i) = index;
            i += 1;
        }
    } else {
        memcpy(
            dst as *mut c_void,
            src as *const c_void,
            tableSize * core::mem::size_of::<U32>(),
        );
    }
}

unsafe fn ZSTD_resetCCtx_byCopyingCDict(
    cctx: *mut ZSTD_CCtx,
    cdict: *const ZSTD_CDict,
    mut params: ZSTD_CCtx_params,
    pledgedSrcSize: U64,
    zbuff: ZSTD_buffered_policy_e,
) -> usize {
    let cdict = as_cdict(cdict);
    let cdict_cParams: *const ZSTD_compressionParameters = &(*cdict).matchState.cParams;

    debug_assert!((*cdict).matchState.dedicatedDictSearch == 0);

    {
        let windowLog = params.cParams.windowLog;
        debug_assert!(windowLog != 0);
        params.cParams = *cdict_cParams;
        params.cParams.windowLog = windowLog;
        params.useRowMatchFinder = (*cdict).useRowMatchFinder;
        FORWARD_IF_ERROR!(ZSTD_resetCCtx_internal(
            cctx,
            &params,
            pledgedSrcSize,
            0,
            ZSTDcrp_leaveDirty,
            zbuff,
        ));
    }

    ZSTD_cwksp_mark_tables_dirty(&mut (*cctx).workspace);
    debug_assert!(params.useRowMatchFinder != ZSTD_ps_auto);

    {
        let chainSize: usize = if ZSTD_allocateChainTable(
            (*cdict_cParams).strategy,
            (*cdict).useRowMatchFinder,
            0,
        ) != 0
        {
            1usize << (*cdict_cParams).chainLog
        } else {
            0
        };
        let hSize: usize = 1usize << (*cdict_cParams).hashLog;

        ZSTD_copyCDictTableIntoCCtx(
            (*cctx).blockState.matchState.hashTable,
            (*cdict).matchState.hashTable,
            hSize,
            cdict_cParams,
        );

        if ZSTD_allocateChainTable(
            (*cctx).appliedParams.cParams.strategy,
            (*cctx).appliedParams.useRowMatchFinder,
            0,
        ) != 0
        {
            ZSTD_copyCDictTableIntoCCtx(
                (*cctx).blockState.matchState.chainTable,
                (*cdict).matchState.chainTable,
                chainSize,
                cdict_cParams,
            );
        }
        if ZSTD_rowMatchFinderUsed((*cdict_cParams).strategy, (*cdict).useRowMatchFinder) != 0 {
            let tagTableSize: usize = hSize;
            memcpy(
                (*cctx).blockState.matchState.tagTable as *mut c_void,
                (*cdict).matchState.tagTable as *const c_void,
                tagTableSize,
            );
            (*cctx).blockState.matchState.hashSalt = (*cdict).matchState.hashSalt;
        }
    }

    debug_assert!((*cctx).blockState.matchState.hashLog3 <= 31);
    {
        let h3log: U32 = (*cctx).blockState.matchState.hashLog3;
        let h3Size: usize = if h3log != 0 { 1usize << h3log } else { 0 };
        debug_assert!((*cdict).matchState.hashLog3 == 0);
        memset(
            (*cctx).blockState.matchState.hashTable3 as *mut c_void,
            0,
            h3Size * core::mem::size_of::<U32>(),
        );
    }

    ZSTD_cwksp_mark_tables_clean(&mut (*cctx).workspace);

    {
        let srcMatchState: *const ZSTD_MatchState_t = &(*cdict).matchState;
        let dstMatchState: *mut ZSTD_MatchState_t = &mut (*cctx).blockState.matchState;
        (*dstMatchState).window = (*srcMatchState).window;
        (*dstMatchState).nextToUpdate = (*srcMatchState).nextToUpdate;
        (*dstMatchState).loadedDictEnd = (*srcMatchState).loadedDictEnd;
    }

    (*cctx).dictID = (*cdict).dictID;
    (*cctx).dictContentSize = (*cdict).dictContentSize;

    memcpy(
        (*cctx).blockState.prevCBlock as *mut c_void,
        &(*cdict).cBlockState as *const _ as *const c_void,
        core::mem::size_of::<ZSTD_compressedBlockState_t>(),
    );

    0
}

unsafe fn ZSTD_resetCCtx_usingCDict(
    cctx: *mut ZSTD_CCtx,
    cdict: *const ZSTD_CDict,
    params: *const ZSTD_CCtx_params,
    pledgedSrcSize: U64,
    zbuff: ZSTD_buffered_policy_e,
) -> usize {
    if ZSTD_shouldAttachDict(cdict, params, pledgedSrcSize) != 0 {
        ZSTD_resetCCtx_byAttachingCDict(
            cctx,
            cdict,
            core::ptr::read(params),
            pledgedSrcSize,
            zbuff,
        )
    } else {
        ZSTD_resetCCtx_byCopyingCDict(
            cctx,
            cdict,
            core::ptr::read(params),
            pledgedSrcSize,
            zbuff,
        )
    }
}

unsafe fn ZSTD_copyCCtx_internal(
    dstCCtx: *mut ZSTD_CCtx,
    srcCCtx: *const ZSTD_CCtx,
    fParams: ZSTD_frameParameters,
    pledgedSrcSize: U64,
    zbuff: ZSTD_buffered_policy_e,
) -> usize {
    RETURN_ERROR_IF!((*srcCCtx).stage != ZSTDcs_init, STAGE_WRONG);
    memcpy(
        &mut (*dstCCtx).customMem as *mut _ as *mut c_void,
        &(*srcCCtx).customMem as *const _ as *const c_void,
        core::mem::size_of::<ZSTD_customMem>(),
    );
    {
        let mut params: ZSTD_CCtx_params = core::ptr::read(&(*dstCCtx).requestedParams);
        params.cParams = (*srcCCtx).appliedParams.cParams;
        debug_assert!((*srcCCtx).appliedParams.useRowMatchFinder != ZSTD_ps_auto);
        debug_assert!((*srcCCtx).appliedParams.postBlockSplitter != ZSTD_ps_auto);
        debug_assert!((*srcCCtx).appliedParams.ldmParams.enableLdm != ZSTD_ps_auto);
        params.useRowMatchFinder = (*srcCCtx).appliedParams.useRowMatchFinder;
        params.postBlockSplitter = (*srcCCtx).appliedParams.postBlockSplitter;
        params.ldmParams = (*srcCCtx).appliedParams.ldmParams;
        params.fParams = fParams;
        params.maxBlockSize = (*srcCCtx).appliedParams.maxBlockSize;
        ZSTD_resetCCtx_internal(
            dstCCtx,
            &params,
            pledgedSrcSize,
            0,
            ZSTDcrp_leaveDirty,
            zbuff,
        );
    }

    ZSTD_cwksp_mark_tables_dirty(&mut (*dstCCtx).workspace);

    {
        let chainSize: usize = if ZSTD_allocateChainTable(
            (*srcCCtx).appliedParams.cParams.strategy,
            (*srcCCtx).appliedParams.useRowMatchFinder,
            0,
        ) != 0
        {
            1usize << (*srcCCtx).appliedParams.cParams.chainLog
        } else {
            0
        };
        let hSize: usize = 1usize << (*srcCCtx).appliedParams.cParams.hashLog;
        let h3log: U32 = (*srcCCtx).blockState.matchState.hashLog3;
        let h3Size: usize = if h3log != 0 { 1usize << h3log } else { 0 };

        memcpy(
            (*dstCCtx).blockState.matchState.hashTable as *mut c_void,
            (*srcCCtx).blockState.matchState.hashTable as *const c_void,
            hSize * core::mem::size_of::<U32>(),
        );
        memcpy(
            (*dstCCtx).blockState.matchState.chainTable as *mut c_void,
            (*srcCCtx).blockState.matchState.chainTable as *const c_void,
            chainSize * core::mem::size_of::<U32>(),
        );
        memcpy(
            (*dstCCtx).blockState.matchState.hashTable3 as *mut c_void,
            (*srcCCtx).blockState.matchState.hashTable3 as *const c_void,
            h3Size * core::mem::size_of::<U32>(),
        );
    }

    ZSTD_cwksp_mark_tables_clean(&mut (*dstCCtx).workspace);

    {
        let srcMatchState: *const ZSTD_MatchState_t = &(*srcCCtx).blockState.matchState;
        let dstMatchState: *mut ZSTD_MatchState_t = &mut (*dstCCtx).blockState.matchState;
        (*dstMatchState).window = (*srcMatchState).window;
        (*dstMatchState).nextToUpdate = (*srcMatchState).nextToUpdate;
        (*dstMatchState).loadedDictEnd = (*srcMatchState).loadedDictEnd;
    }
    (*dstCCtx).dictID = (*srcCCtx).dictID;
    (*dstCCtx).dictContentSize = (*srcCCtx).dictContentSize;

    memcpy(
        (*dstCCtx).blockState.prevCBlock as *mut c_void,
        (*srcCCtx).blockState.prevCBlock as *const c_void,
        core::mem::size_of::<ZSTD_compressedBlockState_t>(),
    );

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_copyCCtx(
    dstCCtx: *mut ZSTD_CCtx,
    srcCCtx: *const ZSTD_CCtx,
    mut pledgedSrcSize: c_ulonglong,
) -> usize {
    let mut fParams = ZSTD_frameParameters {
        contentSizeFlag: 1,
        checksumFlag: 0,
        noDictIDFlag: 0,
    };
    let zbuff = (*srcCCtx).bufferedPolicy;
    if pledgedSrcSize == 0 {
        pledgedSrcSize = ZSTD_CONTENTSIZE_UNKNOWN;
    }
    fParams.contentSizeFlag = (pledgedSrcSize != ZSTD_CONTENTSIZE_UNKNOWN) as c_int;

    ZSTD_copyCCtx_internal(dstCCtx, srcCCtx, fParams, pledgedSrcSize, zbuff)
}

#[inline(always)]
unsafe fn ZSTD_reduceTable_internal(
    table: *mut U32,
    size: U32,
    reducerValue: U32,
    preserveMark: c_int,
) {
    let nbRows = size as i32 / ZSTD_ROWSIZE as i32;
    let mut cellNb = 0;
    let mut rowNb;
    let reducerThreshold: U32 = reducerValue + ZSTD_WINDOW_START_INDEX;
    debug_assert!((size & (ZSTD_ROWSIZE as U32 - 1)) == 0);
    debug_assert!(size < (1u32 << 31));

    rowNb = 0;
    while rowNb < nbRows {
        let mut column = 0;
        while column < ZSTD_ROWSIZE as i32 {
            let newVal: U32;
            if preserveMark != 0 && *table.add(cellNb) == ZSTD_DUBT_UNSORTED_MARK {
                newVal = ZSTD_DUBT_UNSORTED_MARK;
            } else if *table.add(cellNb) < reducerThreshold {
                newVal = 0;
            } else {
                newVal = *table.add(cellNb) - reducerValue;
            }
            *table.add(cellNb) = newVal;
            cellNb += 1;
            column += 1;
        }
        rowNb += 1;
    }
}

unsafe fn ZSTD_reduceTable(table: *mut U32, size: U32, reducerValue: U32) {
    ZSTD_reduceTable_internal(table, size, reducerValue, 0);
}

unsafe fn ZSTD_reduceTable_btlazy2(table: *mut U32, size: U32, reducerValue: U32) {
    ZSTD_reduceTable_internal(table, size, reducerValue, 1);
}

unsafe fn ZSTD_reduceIndex(
    ms: *mut ZSTD_MatchState_t,
    params: *const ZSTD_CCtx_params,
    reducerValue: U32,
) {
    {
        let hSize: U32 = 1u32 << (*params).cParams.hashLog;
        ZSTD_reduceTable((*ms).hashTable, hSize, reducerValue);
    }

    if ZSTD_allocateChainTable(
        (*params).cParams.strategy,
        (*params).useRowMatchFinder,
        (*ms).dedicatedDictSearch as U32,
    ) != 0
    {
        let chainSize: U32 = 1u32 << (*params).cParams.chainLog;
        if (*params).cParams.strategy == ZSTD_btlazy2 {
            ZSTD_reduceTable_btlazy2((*ms).chainTable, chainSize, reducerValue);
        } else {
            ZSTD_reduceTable((*ms).chainTable, chainSize, reducerValue);
        }
    }

    if (*ms).hashLog3 != 0 {
        let h3Size: U32 = 1u32 << (*ms).hashLog3;
        ZSTD_reduceTable((*ms).hashTable3, h3Size, reducerValue);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_seqToCodes(seqStorePtr: *const SeqStore_t) -> c_int {
    let sequences: *const SeqDef = (*seqStorePtr).sequencesStart;
    let llCodeTable: *mut u8 = (*seqStorePtr).llCode;
    let ofCodeTable: *mut u8 = (*seqStorePtr).ofCode;
    let mlCodeTable: *mut u8 = (*seqStorePtr).mlCode;
    let nbSeq: U32 =
        ((*seqStorePtr).sequences as usize - (*seqStorePtr).sequencesStart as usize) as U32
            / core::mem::size_of::<SeqDef>() as U32;
    let mut u;
    let mut longOffsets = 0;
    debug_assert!(nbSeq as usize <= (*seqStorePtr).maxNbSeq);
    u = 0;
    while u < nbSeq {
        let llv: U32 = (*sequences.add(u as usize)).litLength as U32;
        let ofCode: U32 = ZSTD_highbit32((*sequences.add(u as usize)).offBase);
        let mlv: U32 = (*sequences.add(u as usize)).mlBase as U32;
        *llCodeTable.add(u as usize) = ZSTD_LLcode(llv) as u8;
        *ofCodeTable.add(u as usize) = ofCode as u8;
        *mlCodeTable.add(u as usize) = ZSTD_MLcode(mlv) as u8;
        debug_assert!(!(MEM_64bits() != 0 && ofCode >= STREAM_ACCUMULATOR_MIN));
        if MEM_32bits() != 0 && ofCode >= STREAM_ACCUMULATOR_MIN {
            longOffsets = 1;
        }
        u += 1;
    }
    if (*seqStorePtr).longLengthType == ZSTD_llt_literalLength {
        *llCodeTable.add((*seqStorePtr).longLengthPos as usize) = MaxLL as u8;
    }
    if (*seqStorePtr).longLengthType == ZSTD_llt_matchLength {
        *mlCodeTable.add((*seqStorePtr).longLengthPos as usize) = MaxML as u8;
    }
    longOffsets
}

unsafe fn ZSTD_useTargetCBlockSize(cctxParams: *const ZSTD_CCtx_params) -> c_int {
    ((*cctxParams).targetCBlockSize != 0) as c_int
}

unsafe fn ZSTD_blockSplitterEnabled(cctxParams: *mut ZSTD_CCtx_params) -> c_int {
    debug_assert!((*cctxParams).postBlockSplitter != ZSTD_ps_auto);
    ((*cctxParams).postBlockSplitter == ZSTD_ps_enable) as c_int
}

#[repr(C)]
#[derive(Clone, Copy)]
struct ZSTD_symbolEncodingTypeStats_t {
    LLtype: U32,
    Offtype: U32,
    MLtype: U32,
    size: usize,
    lastCountSize: usize,
    longOffsets: c_int,
}

unsafe fn ZSTD_buildSequencesStatistics(
    seqStorePtr: *const SeqStore_t,
    nbSeq: usize,
    prevEntropy: *const ZSTD_fseCTables_t,
    nextEntropy: *mut ZSTD_fseCTables_t,
    dst: *mut u8,
    dstEnd: *const u8,
    strategy: ZSTD_strategy,
    countWorkspace: *mut c_uint,
    entropyWorkspace: *mut c_void,
    entropyWkspSize: usize,
) -> ZSTD_symbolEncodingTypeStats_t {
    let ostart: *mut u8 = dst;
    let oend: *const u8 = dstEnd;
    let mut op: *mut u8 = ostart;
    let CTable_LitLength: *mut FSE_CTable = (*nextEntropy).litlengthCTable.as_mut_ptr();
    let CTable_OffsetBits: *mut FSE_CTable = (*nextEntropy).offcodeCTable.as_mut_ptr();
    let CTable_MatchLength: *mut FSE_CTable = (*nextEntropy).matchlengthCTable.as_mut_ptr();
    let ofCodeTable: *const u8 = (*seqStorePtr).ofCode;
    let llCodeTable: *const u8 = (*seqStorePtr).llCode;
    let mlCodeTable: *const u8 = (*seqStorePtr).mlCode;
    let mut stats: ZSTD_symbolEncodingTypeStats_t = core::mem::zeroed();

    stats.lastCountSize = 0;
    stats.longOffsets = ZSTD_seqToCodes(seqStorePtr);
    debug_assert!(op as *const u8 <= oend);
    debug_assert!(nbSeq != 0);
    {
        let mut max: c_uint = MaxLL;
        let mostFrequent = HIST_countFast_wksp(
            countWorkspace,
            &mut max,
            llCodeTable as *const c_void,
            nbSeq,
            entropyWorkspace,
            entropyWkspSize,
        );
        (*nextEntropy).litlength_repeatMode = (*prevEntropy).litlength_repeatMode;
        stats.LLtype = ZSTD_selectEncodingType(
            &mut (*nextEntropy).litlength_repeatMode,
            countWorkspace,
            max,
            mostFrequent,
            nbSeq,
            LLFSELog,
            (*prevEntropy).litlengthCTable.as_ptr(),
            LL_defaultNorm.as_ptr(),
            LL_defaultNormLog,
            ZSTD_defaultAllowed,
            strategy,
        );
        {
            let countSize = ZSTD_buildCTable(
                op as *mut c_void,
                (oend as usize - op as usize),
                CTable_LitLength,
                LLFSELog,
                stats.LLtype as SymbolEncodingType_e,
                countWorkspace,
                max,
                llCodeTable,
                nbSeq,
                LL_defaultNorm.as_ptr(),
                LL_defaultNormLog,
                MaxLL,
                (*prevEntropy).litlengthCTable.as_ptr(),
                core::mem::size_of_val(&(*prevEntropy).litlengthCTable),
                entropyWorkspace,
                entropyWkspSize,
            );
            if ZSTD_isError(countSize) != 0 {
                stats.size = countSize;
                return stats;
            }
            if stats.LLtype == set_compressed {
                stats.lastCountSize = countSize;
            }
            op = op.add(countSize);
            debug_assert!(op as *const u8 <= oend);
        }
    }
    {
        let mut max: c_uint = MaxOff;
        let mostFrequent = HIST_countFast_wksp(
            countWorkspace,
            &mut max,
            ofCodeTable as *const c_void,
            nbSeq,
            entropyWorkspace,
            entropyWkspSize,
        );
        let defaultPolicy: ZSTD_DefaultPolicy_e = if max <= DefaultMaxOff {
            ZSTD_defaultAllowed
        } else {
            ZSTD_defaultDisallowed
        };
        (*nextEntropy).offcode_repeatMode = (*prevEntropy).offcode_repeatMode;
        stats.Offtype = ZSTD_selectEncodingType(
            &mut (*nextEntropy).offcode_repeatMode,
            countWorkspace,
            max,
            mostFrequent,
            nbSeq,
            OffFSELog,
            (*prevEntropy).offcodeCTable.as_ptr(),
            OF_defaultNorm.as_ptr(),
            OF_defaultNormLog,
            defaultPolicy,
            strategy,
        );
        {
            let countSize = ZSTD_buildCTable(
                op as *mut c_void,
                (oend as usize - op as usize),
                CTable_OffsetBits,
                OffFSELog,
                stats.Offtype as SymbolEncodingType_e,
                countWorkspace,
                max,
                ofCodeTable,
                nbSeq,
                OF_defaultNorm.as_ptr(),
                OF_defaultNormLog,
                DefaultMaxOff,
                (*prevEntropy).offcodeCTable.as_ptr(),
                core::mem::size_of_val(&(*prevEntropy).offcodeCTable),
                entropyWorkspace,
                entropyWkspSize,
            );
            if ZSTD_isError(countSize) != 0 {
                stats.size = countSize;
                return stats;
            }
            if stats.Offtype == set_compressed {
                stats.lastCountSize = countSize;
            }
            op = op.add(countSize);
            debug_assert!(op as *const u8 <= oend);
        }
    }
    {
        let mut max: c_uint = MaxML;
        let mostFrequent = HIST_countFast_wksp(
            countWorkspace,
            &mut max,
            mlCodeTable as *const c_void,
            nbSeq,
            entropyWorkspace,
            entropyWkspSize,
        );
        (*nextEntropy).matchlength_repeatMode = (*prevEntropy).matchlength_repeatMode;
        stats.MLtype = ZSTD_selectEncodingType(
            &mut (*nextEntropy).matchlength_repeatMode,
            countWorkspace,
            max,
            mostFrequent,
            nbSeq,
            MLFSELog,
            (*prevEntropy).matchlengthCTable.as_ptr(),
            ML_defaultNorm.as_ptr(),
            ML_defaultNormLog,
            ZSTD_defaultAllowed,
            strategy,
        );
        {
            let countSize = ZSTD_buildCTable(
                op as *mut c_void,
                (oend as usize - op as usize),
                CTable_MatchLength,
                MLFSELog,
                stats.MLtype as SymbolEncodingType_e,
                countWorkspace,
                max,
                mlCodeTable,
                nbSeq,
                ML_defaultNorm.as_ptr(),
                ML_defaultNormLog,
                MaxML,
                (*prevEntropy).matchlengthCTable.as_ptr(),
                core::mem::size_of_val(&(*prevEntropy).matchlengthCTable),
                entropyWorkspace,
                entropyWkspSize,
            );
            if ZSTD_isError(countSize) != 0 {
                stats.size = countSize;
                return stats;
            }
            if stats.MLtype == set_compressed {
                stats.lastCountSize = countSize;
            }
            op = op.add(countSize);
            debug_assert!(op as *const u8 <= oend);
        }
    }
    stats.size = op as usize - ostart as usize;
    stats
}

const SUSPECT_UNCOMPRESSIBLE_LITERAL_RATIO: usize = 20;

unsafe fn ZSTD_entropyCompressSeqStore_internal(
    dst: *mut c_void,
    dstCapacity: usize,
    literals: *const c_void,
    litSize: usize,
    seqStorePtr: *const SeqStore_t,
    prevEntropy: *const ZSTD_entropyCTables_t,
    nextEntropy: *mut ZSTD_entropyCTables_t,
    cctxParams: *const ZSTD_CCtx_params,
    mut entropyWorkspace: *mut c_void,
    mut entropyWkspSize: usize,
    bmi2: c_int,
) -> usize {
    let strategy = (*cctxParams).cParams.strategy;
    let count: *mut c_uint = entropyWorkspace as *mut c_uint;
    let CTable_LitLength: *const FSE_CTable = (*nextEntropy).fse.litlengthCTable.as_ptr();
    let CTable_OffsetBits: *const FSE_CTable = (*nextEntropy).fse.offcodeCTable.as_ptr();
    let CTable_MatchLength: *const FSE_CTable = (*nextEntropy).fse.matchlengthCTable.as_ptr();
    let sequences: *const SeqDef = (*seqStorePtr).sequencesStart;
    let nbSeq: usize = ((*seqStorePtr).sequences as usize - (*seqStorePtr).sequencesStart as usize)
        / core::mem::size_of::<SeqDef>();
    let ofCodeTable: *const u8 = (*seqStorePtr).ofCode;
    let llCodeTable: *const u8 = (*seqStorePtr).llCode;
    let mlCodeTable: *const u8 = (*seqStorePtr).mlCode;
    let ostart: *mut u8 = dst as *mut u8;
    let oend: *const u8 = ostart.add(dstCapacity);
    let mut op: *mut u8 = ostart;
    let lastCountSize: usize;
    let mut longOffsets = 0;

    entropyWorkspace = count.add(MaxSeq as usize + 1) as *mut c_void;
    entropyWkspSize -= (MaxSeq as usize + 1) * core::mem::size_of::<c_uint>();

    debug_assert!(entropyWkspSize >= HUF_WORKSPACE_SIZE);

    /* Compress literals */
    {
        let numSequences: usize =
            ((*seqStorePtr).sequences as usize - (*seqStorePtr).sequencesStart as usize)
                / core::mem::size_of::<SeqDef>();
        let suspectUncompressible: c_int = (numSequences == 0
            || (litSize / numSequences >= SUSPECT_UNCOMPRESSIBLE_LITERAL_RATIO))
            as c_int;

        let cSize = ZSTD_compressLiterals(
            op as *mut c_void,
            dstCapacity,
            literals,
            litSize,
            entropyWorkspace,
            entropyWkspSize,
            &(*prevEntropy).huf,
            &mut (*nextEntropy).huf,
            (*cctxParams).cParams.strategy,
            ZSTD_literalsCompressionIsDisabled(cctxParams),
            suspectUncompressible,
            bmi2,
        );
        FORWARD_IF_ERROR!(cSize);
        debug_assert!(cSize <= dstCapacity);
        op = op.add(cSize);
    }

    /* Sequences Header */
    RETURN_ERROR_IF!((oend as usize - op as usize) < 3 + 1, DSTSIZE_TOOSMALL);
    if nbSeq < 128 {
        *op = nbSeq as u8;
        op = op.add(1);
    } else if (nbSeq as u32) < LONGNBSEQ {
        *op.add(0) = ((nbSeq >> 8) + 0x80) as u8;
        *op.add(1) = nbSeq as u8;
        op = op.add(2);
    } else {
        *op.add(0) = 0xFF;
        MEM_writeLE16(op.add(1) as *mut c_void, (nbSeq as u32 - LONGNBSEQ) as U16);
        op = op.add(3);
    }
    debug_assert!(op as *const u8 <= oend);
    if nbSeq == 0 {
        memcpy(
            &mut (*nextEntropy).fse as *mut _ as *mut c_void,
            &(*prevEntropy).fse as *const _ as *const c_void,
            core::mem::size_of::<ZSTD_fseCTables_t>(),
        );
        return op as usize - ostart as usize;
    }
    {
        let seqHead: *mut u8 = op;
        op = op.add(1);
        let stats = ZSTD_buildSequencesStatistics(
            seqStorePtr,
            nbSeq,
            &(*prevEntropy).fse,
            &mut (*nextEntropy).fse,
            op,
            oend,
            strategy,
            count,
            entropyWorkspace,
            entropyWkspSize,
        );
        FORWARD_IF_ERROR!(stats.size);
        *seqHead =
            ((stats.LLtype << 6) + (stats.Offtype << 4) + (stats.MLtype << 2)) as u8;
        lastCountSize = stats.lastCountSize;
        op = op.add(stats.size);
        longOffsets = stats.longOffsets;
    }

    {
        let bitstreamSize = ZSTD_encodeSequences(
            op as *mut c_void,
            (oend as usize - op as usize),
            CTable_MatchLength,
            mlCodeTable,
            CTable_OffsetBits,
            ofCodeTable,
            CTable_LitLength,
            llCodeTable,
            sequences,
            nbSeq,
            longOffsets,
            bmi2,
        );
        FORWARD_IF_ERROR!(bitstreamSize);
        op = op.add(bitstreamSize);
        debug_assert!(op as *const u8 <= oend);
        if lastCountSize != 0 && (lastCountSize + bitstreamSize) < 4 {
            debug_assert!(lastCountSize + bitstreamSize == 3);
            return 0;
        }
    }

    op as usize - ostart as usize
}

unsafe fn ZSTD_entropyCompressSeqStore_wExtLitBuffer(
    dst: *mut c_void,
    dstCapacity: usize,
    literals: *const c_void,
    litSize: usize,
    blockSize: usize,
    seqStorePtr: *const SeqStore_t,
    prevEntropy: *const ZSTD_entropyCTables_t,
    nextEntropy: *mut ZSTD_entropyCTables_t,
    cctxParams: *const ZSTD_CCtx_params,
    entropyWorkspace: *mut c_void,
    entropyWkspSize: usize,
    bmi2: c_int,
) -> usize {
    let cSize = ZSTD_entropyCompressSeqStore_internal(
        dst,
        dstCapacity,
        literals,
        litSize,
        seqStorePtr,
        prevEntropy,
        nextEntropy,
        cctxParams,
        entropyWorkspace,
        entropyWkspSize,
        bmi2,
    );
    if cSize == 0 {
        return 0;
    }
    if (cSize == error(code::DSTSIZE_TOOSMALL)) & (blockSize <= dstCapacity) {
        return 0;
    }
    FORWARD_IF_ERROR!(cSize);

    {
        let maxCSize = blockSize - ZSTD_minGain(blockSize, (*cctxParams).cParams.strategy);
        if cSize >= maxCSize {
            return 0;
        }
    }
    debug_assert!(cSize < ZSTD_BLOCKSIZE_MAX);
    cSize
}

unsafe fn ZSTD_entropyCompressSeqStore(
    seqStorePtr: *const SeqStore_t,
    prevEntropy: *const ZSTD_entropyCTables_t,
    nextEntropy: *mut ZSTD_entropyCTables_t,
    cctxParams: *const ZSTD_CCtx_params,
    dst: *mut c_void,
    dstCapacity: usize,
    srcSize: usize,
    entropyWorkspace: *mut c_void,
    entropyWkspSize: usize,
    bmi2: c_int,
) -> usize {
    ZSTD_entropyCompressSeqStore_wExtLitBuffer(
        dst,
        dstCapacity,
        (*seqStorePtr).litStart as *const c_void,
        ((*seqStorePtr).lit as usize - (*seqStorePtr).litStart as usize),
        srcSize,
        seqStorePtr,
        prevEntropy,
        nextEntropy,
        cctxParams,
        entropyWorkspace,
        entropyWkspSize,
        bmi2,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_selectBlockCompressor(
    strat: ZSTD_strategy,
    useRowMatchFinder: ZSTD_ParamSwitch_e,
    dictMode: ZSTD_dictMode_e,
) -> ZSTD_BlockCompressor_f {
    static blockCompressor: [[ZSTD_BlockCompressor_f; (ZSTD_STRATEGY_MAX + 1) as usize]; 4] = [
        [
            Some(ZSTD_compressBlock_fast),
            Some(ZSTD_compressBlock_fast),
            Some(ZSTD_compressBlock_doubleFast),
            Some(ZSTD_compressBlock_greedy),
            Some(ZSTD_compressBlock_lazy),
            Some(ZSTD_compressBlock_lazy2),
            Some(ZSTD_compressBlock_btlazy2),
            Some(ZSTD_compressBlock_btopt),
            Some(ZSTD_compressBlock_btultra),
            Some(ZSTD_compressBlock_btultra2),
        ],
        [
            Some(ZSTD_compressBlock_fast_extDict),
            Some(ZSTD_compressBlock_fast_extDict),
            Some(ZSTD_compressBlock_doubleFast_extDict),
            Some(ZSTD_compressBlock_greedy_extDict),
            Some(ZSTD_compressBlock_lazy_extDict),
            Some(ZSTD_compressBlock_lazy2_extDict),
            Some(ZSTD_compressBlock_btlazy2_extDict),
            Some(ZSTD_compressBlock_btopt_extDict),
            Some(ZSTD_compressBlock_btultra_extDict),
            Some(ZSTD_compressBlock_btultra_extDict),
        ],
        [
            Some(ZSTD_compressBlock_fast_dictMatchState),
            Some(ZSTD_compressBlock_fast_dictMatchState),
            Some(ZSTD_compressBlock_doubleFast_dictMatchState),
            Some(ZSTD_compressBlock_greedy_dictMatchState),
            Some(ZSTD_compressBlock_lazy_dictMatchState),
            Some(ZSTD_compressBlock_lazy2_dictMatchState),
            Some(ZSTD_compressBlock_btlazy2_dictMatchState),
            Some(ZSTD_compressBlock_btopt_dictMatchState),
            Some(ZSTD_compressBlock_btultra_dictMatchState),
            Some(ZSTD_compressBlock_btultra_dictMatchState),
        ],
        [
            None,
            None,
            None,
            Some(ZSTD_compressBlock_greedy_dedicatedDictSearch),
            Some(ZSTD_compressBlock_lazy_dedicatedDictSearch),
            Some(ZSTD_compressBlock_lazy2_dedicatedDictSearch),
            None,
            None,
            None,
            None,
        ],
    ];
    let selectedCompressor: ZSTD_BlockCompressor_f;

    if ZSTD_rowMatchFinderUsed(strat, useRowMatchFinder) != 0 {
        static rowBasedBlockCompressors: [[ZSTD_BlockCompressor_f; 3]; 4] = [
            [
                Some(ZSTD_compressBlock_greedy_row),
                Some(ZSTD_compressBlock_lazy_row),
                Some(ZSTD_compressBlock_lazy2_row),
            ],
            [
                Some(ZSTD_compressBlock_greedy_extDict_row),
                Some(ZSTD_compressBlock_lazy_extDict_row),
                Some(ZSTD_compressBlock_lazy2_extDict_row),
            ],
            [
                Some(ZSTD_compressBlock_greedy_dictMatchState_row),
                Some(ZSTD_compressBlock_lazy_dictMatchState_row),
                Some(ZSTD_compressBlock_lazy2_dictMatchState_row),
            ],
            [
                Some(ZSTD_compressBlock_greedy_dedicatedDictSearch_row),
                Some(ZSTD_compressBlock_lazy_dedicatedDictSearch_row),
                Some(ZSTD_compressBlock_lazy2_dedicatedDictSearch_row),
            ],
        ];
        debug_assert!(useRowMatchFinder != ZSTD_ps_auto);
        selectedCompressor =
            rowBasedBlockCompressors[dictMode as usize][(strat - ZSTD_greedy) as usize];
    } else {
        selectedCompressor = blockCompressor[dictMode as usize][strat as usize];
    }
    debug_assert!(selectedCompressor.is_some());
    selectedCompressor
}

unsafe fn ZSTD_storeLastLiterals(
    seqStorePtr: *mut SeqStore_t,
    anchor: *const u8,
    lastLLSize: usize,
) {
    memcpy(
        (*seqStorePtr).lit as *mut c_void,
        anchor as *const c_void,
        lastLLSize,
    );
    (*seqStorePtr).lit = (*seqStorePtr).lit.add(lastLLSize);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_resetSeqStore(ssPtr: *mut SeqStore_t) {
    (*ssPtr).lit = (*ssPtr).litStart;
    (*ssPtr).sequences = (*ssPtr).sequencesStart;
    (*ssPtr).longLengthType = ZSTD_llt_none;
}

unsafe fn ZSTD_postProcessSequenceProducerResult(
    outSeqs: *mut ZSTD_Sequence,
    nbExternalSeqs: usize,
    outSeqsCapacity: usize,
    srcSize: usize,
) -> usize {
    RETURN_ERROR_IF!(nbExternalSeqs > outSeqsCapacity, SEQUENCEPRODUCER_FAILED);
    RETURN_ERROR_IF!(nbExternalSeqs == 0 && srcSize > 0, SEQUENCEPRODUCER_FAILED);

    if srcSize == 0 {
        memset(
            &mut *outSeqs.add(0) as *mut _ as *mut c_void,
            0,
            core::mem::size_of::<ZSTD_Sequence>(),
        );
        return 1;
    }

    {
        let lastSeq = *outSeqs.add(nbExternalSeqs - 1);

        if lastSeq.offset == 0 && lastSeq.matchLength == 0 {
            return nbExternalSeqs;
        }

        RETURN_ERROR_IF!(nbExternalSeqs == outSeqsCapacity, SEQUENCEPRODUCER_FAILED);

        memset(
            &mut *outSeqs.add(nbExternalSeqs) as *mut _ as *mut c_void,
            0,
            core::mem::size_of::<ZSTD_Sequence>(),
        );
        nbExternalSeqs + 1
    }
}

unsafe fn ZSTD_fastSequenceLengthSum(seqBuf: *const ZSTD_Sequence, seqBufSize: usize) -> usize {
    let mut matchLenSum: usize = 0;
    let mut litLenSum: usize = 0;
    let mut i = 0;
    while i < seqBufSize {
        litLenSum += (*seqBuf.add(i)).litLength as usize;
        matchLenSum += (*seqBuf.add(i)).matchLength as usize;
        i += 1;
    }
    litLenSum + matchLenSum
}

unsafe fn ZSTD_validateSeqStore(
    _seqStore: *const SeqStore_t,
    _cParams: *const ZSTD_compressionParameters,
) {
    /* DEBUGLEVEL 0: no-op */
}

// ZSTD_BuildSeqStore_e
type ZSTD_BuildSeqStore_e = u32;
const ZSTDbss_compress: ZSTD_BuildSeqStore_e = 0;
const ZSTDbss_noCompress: ZSTD_BuildSeqStore_e = 1;

unsafe fn ZSTD_buildSeqStore(zc: *mut ZSTD_CCtx, src: *const c_void, srcSize: usize) -> usize {
    let ms: *mut ZSTD_MatchState_t = &mut (*zc).blockState.matchState;
    debug_assert!(srcSize <= ZSTD_BLOCKSIZE_MAX);
    ZSTD_assertEqualCParams((*zc).appliedParams.cParams, (*ms).cParams);
    if srcSize < MIN_CBLOCK_SIZE + ZSTD_blockHeaderSize + 1 + 1 {
        if (*zc).appliedParams.cParams.strategy >= ZSTD_btopt {
            ZSTD_ldm_skipRawSeqStoreBytes(&mut (*zc).externSeqStore, srcSize);
        } else {
            ZSTD_ldm_skipSequences(
                &mut (*zc).externSeqStore,
                srcSize,
                (*zc).appliedParams.cParams.minMatch,
            );
        }
        return ZSTDbss_noCompress as usize;
    }
    ZSTD_resetSeqStore(&mut (*zc).seqStore);
    (*ms).opt.symbolCosts = &(*(*zc).blockState.prevCBlock).entropy;
    (*ms).opt.literalCompressionMode = (*zc).appliedParams.literalCompressionMode;
    debug_assert!(
        (*ms).dictMatchState.is_null() || (*ms).loadedDictEnd == (*ms).window.dictLimit
    );

    {
        let base: *const u8 = (*ms).window.base;
        let istart: *const u8 = src as *const u8;
        let curr: U32 = (istart as usize - base as usize) as U32;
        if curr > (*ms).nextToUpdate + 384 {
            (*ms).nextToUpdate =
                curr - core::cmp::min(192, curr - (*ms).nextToUpdate - 384);
        }
    }

    {
        let dictMode: ZSTD_dictMode_e = ZSTD_matchState_dictMode(ms);
        let mut lastLLSize: usize;
        {
            let mut i = 0;
            while i < ZSTD_REP_NUM {
                (*(*zc).blockState.nextCBlock).rep[i] = (*(*zc).blockState.prevCBlock).rep[i];
                i += 1;
            }
        }
        if (*zc).externSeqStore.pos < (*zc).externSeqStore.size {
            debug_assert!((*zc).appliedParams.ldmParams.enableLdm == ZSTD_ps_disable);
            RETURN_ERROR_IF!(
                ZSTD_hasExtSeqProd(&(*zc).appliedParams) != 0,
                PARAMETER_COMBINATION_UNSUPPORTED
            );
            lastLLSize = ZSTD_ldm_blockCompress(
                &mut (*zc).externSeqStore,
                ms,
                &mut (*zc).seqStore,
                (*(*zc).blockState.nextCBlock).rep.as_mut_ptr(),
                (*zc).appliedParams.useRowMatchFinder,
                src,
                srcSize,
            );
            debug_assert!((*zc).externSeqStore.pos <= (*zc).externSeqStore.size);
        } else if (*zc).appliedParams.ldmParams.enableLdm == ZSTD_ps_enable {
            let mut ldmSeqStore: RawSeqStore_t = kNullRawSeqStore;
            RETURN_ERROR_IF!(
                ZSTD_hasExtSeqProd(&(*zc).appliedParams) != 0,
                PARAMETER_COMBINATION_UNSUPPORTED
            );
            ldmSeqStore.seq = (*zc).ldmSequences;
            ldmSeqStore.capacity = (*zc).maxNbLdmSequences;
            FORWARD_IF_ERROR!(ZSTD_ldm_generateSequences(
                &mut (*zc).ldmState,
                &mut ldmSeqStore,
                &(*zc).appliedParams.ldmParams,
                src,
                srcSize,
            ));
            lastLLSize = ZSTD_ldm_blockCompress(
                &mut ldmSeqStore,
                ms,
                &mut (*zc).seqStore,
                (*(*zc).blockState.nextCBlock).rep.as_mut_ptr(),
                (*zc).appliedParams.useRowMatchFinder,
                src,
                srcSize,
            );
            debug_assert!(ldmSeqStore.pos == ldmSeqStore.size);
        } else if ZSTD_hasExtSeqProd(&(*zc).appliedParams) != 0 {
            debug_assert!((*zc).extSeqBufCapacity >= ZSTD_sequenceBound(srcSize));
            debug_assert!((*zc).appliedParams.extSeqProdFunc.is_some());

            {
                let windowSize: U32 = 1u32 << (*zc).appliedParams.cParams.windowLog;

                let nbExternalSeqs = ((*zc).appliedParams.extSeqProdFunc.unwrap())(
                    (*zc).appliedParams.extSeqProdState,
                    (*zc).extSeqBuf,
                    (*zc).extSeqBufCapacity,
                    src,
                    srcSize,
                    core::ptr::null(),
                    0,
                    (*zc).appliedParams.compressionLevel,
                    windowSize as usize,
                );

                let nbPostProcessedSeqs = ZSTD_postProcessSequenceProducerResult(
                    (*zc).extSeqBuf,
                    nbExternalSeqs,
                    (*zc).extSeqBufCapacity,
                    srcSize,
                );

                if ZSTD_isError(nbPostProcessedSeqs) == 0 {
                    let mut seqPos = ZSTD_SequencePosition {
                        idx: 0,
                        posInSequence: 0,
                        posInSrc: 0,
                    };
                    let seqLenSum =
                        ZSTD_fastSequenceLengthSum((*zc).extSeqBuf, nbPostProcessedSeqs);
                    RETURN_ERROR_IF!(seqLenSum > srcSize, EXTERNALSEQUENCES_INVALID);
                    FORWARD_IF_ERROR!(ZSTD_transferSequences_wBlockDelim(
                        zc,
                        &mut seqPos,
                        (*zc).extSeqBuf,
                        nbPostProcessedSeqs,
                        src,
                        srcSize,
                        (*zc).appliedParams.searchForExternalRepcodes,
                    ));
                    (*ms).ldmSeqStore = core::ptr::null();
                    return ZSTDbss_compress as usize;
                }

                if (*zc).appliedParams.enableMatchFinderFallback == 0 {
                    return nbPostProcessedSeqs;
                }

                {
                    let blockCompressor = ZSTD_selectBlockCompressor(
                        (*zc).appliedParams.cParams.strategy,
                        (*zc).appliedParams.useRowMatchFinder,
                        dictMode,
                    );
                    (*ms).ldmSeqStore = core::ptr::null();
                    lastLLSize = (blockCompressor.unwrap())(
                        ms,
                        &mut (*zc).seqStore,
                        (*(*zc).blockState.nextCBlock).rep.as_mut_ptr(),
                        src,
                        srcSize,
                    );
                }
            }
        } else {
            let blockCompressor = ZSTD_selectBlockCompressor(
                (*zc).appliedParams.cParams.strategy,
                (*zc).appliedParams.useRowMatchFinder,
                dictMode,
            );
            (*ms).ldmSeqStore = core::ptr::null();
            lastLLSize = (blockCompressor.unwrap())(
                ms,
                &mut (*zc).seqStore,
                (*(*zc).blockState.nextCBlock).rep.as_mut_ptr(),
                src,
                srcSize,
            );
        }
        {
            let lastLiterals: *const u8 = (src as *const u8).add(srcSize).sub(lastLLSize);
            ZSTD_storeLastLiterals(&mut (*zc).seqStore, lastLiterals, lastLLSize);
        }
    }
    ZSTD_validateSeqStore(&(*zc).seqStore, &(*zc).appliedParams.cParams);
    ZSTDbss_compress as usize
}

unsafe fn ZSTD_copyBlockSequences(
    seqCollector: *mut SeqCollector,
    seqStore: *const SeqStore_t,
    prevRepcodes: *const U32,
) -> usize {
    let inSeqs: *const SeqDef = (*seqStore).sequencesStart;
    let nbInSequences: usize =
        ((*seqStore).sequences as usize - inSeqs as usize) / core::mem::size_of::<SeqDef>();
    let nbInLiterals: usize = (*seqStore).lit as usize - (*seqStore).litStart as usize;

    let outSeqs: *mut ZSTD_Sequence = if (*seqCollector).seqIndex == 0 {
        (*seqCollector).seqStart
    } else {
        (*seqCollector).seqStart.add((*seqCollector).seqIndex)
    };
    let nbOutSequences: usize = nbInSequences + 1;
    let mut nbOutLiterals: usize = 0;
    let mut repcodes: Repcodes_t = core::mem::zeroed();
    let mut i;

    debug_assert!((*seqCollector).seqIndex <= (*seqCollector).maxSequences);
    RETURN_ERROR_IF!(
        nbOutSequences > ((*seqCollector).maxSequences - (*seqCollector).seqIndex),
        DSTSIZE_TOOSMALL
    );

    memcpy(
        &mut repcodes as *mut _ as *mut c_void,
        prevRepcodes as *const c_void,
        core::mem::size_of::<Repcodes_t>(),
    );
    i = 0;
    while i < nbInSequences {
        let rawOffset: U32;
        (*outSeqs.add(i)).litLength = (*inSeqs.add(i)).litLength as u32;
        (*outSeqs.add(i)).matchLength = (*inSeqs.add(i)).mlBase as u32 + MINMATCH;
        (*outSeqs.add(i)).rep = 0;

        if i == (*seqStore).longLengthPos as usize {
            if (*seqStore).longLengthType == ZSTD_llt_literalLength {
                (*outSeqs.add(i)).litLength += 0x10000;
            } else if (*seqStore).longLengthType == ZSTD_llt_matchLength {
                (*outSeqs.add(i)).matchLength += 0x10000;
            }
        }

        if OFFBASE_IS_REPCODE((*inSeqs.add(i)).offBase) {
            let repcode = OFFBASE_TO_REPCODE((*inSeqs.add(i)).offBase);
            debug_assert!(repcode > 0);
            (*outSeqs.add(i)).rep = repcode;
            if (*outSeqs.add(i)).litLength != 0 {
                rawOffset = repcodes.rep[(repcode - 1) as usize];
            } else {
                if repcode == 3 {
                    debug_assert!(repcodes.rep[0] > 1);
                    rawOffset = repcodes.rep[0] - 1;
                } else {
                    rawOffset = repcodes.rep[repcode as usize];
                }
            }
        } else {
            rawOffset = OFFBASE_TO_OFFSET((*inSeqs.add(i)).offBase);
        }
        (*outSeqs.add(i)).offset = rawOffset;

        ZSTD_updateRep(
            repcodes.rep.as_mut_ptr(),
            (*inSeqs.add(i)).offBase,
            ((*inSeqs.add(i)).litLength == 0) as U32,
        );

        nbOutLiterals += (*outSeqs.add(i)).litLength as usize;
        i += 1;
    }
    debug_assert!(nbInLiterals >= nbOutLiterals);
    {
        let lastLLSize = nbInLiterals - nbOutLiterals;
        (*outSeqs.add(nbInSequences)).litLength = lastLLSize as U32;
        (*outSeqs.add(nbInSequences)).matchLength = 0;
        (*outSeqs.add(nbInSequences)).offset = 0;
        debug_assert!(nbOutSequences == nbInSequences + 1);
    }
    (*seqCollector).seqIndex += nbOutSequences;
    debug_assert!((*seqCollector).seqIndex <= (*seqCollector).maxSequences);

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_sequenceBound(srcSize: usize) -> usize {
    let maxNbSeq = (srcSize / ZSTD_MINMATCH_MIN as usize) + 1;
    let maxNbDelims = (srcSize / ZSTD_BLOCKSIZE_MAX_MIN as usize) + 1;
    maxNbSeq + maxNbDelims
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_generateSequences(
    zc: *mut ZSTD_CCtx,
    outSeqs: *mut ZSTD_Sequence,
    outSeqsSize: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let dstCapacity = ZSTD_compressBound(srcSize);
    let dst: *mut c_void;
    let mut seqCollector: SeqCollector = core::mem::zeroed();
    {
        let mut targetCBlockSize: c_int = 0;
        FORWARD_IF_ERROR!(ZSTD_CCtx_getParameter(
            zc,
            ZSTD_c_targetCBlockSize,
            &mut targetCBlockSize
        ));
        RETURN_ERROR_IF!(targetCBlockSize != 0, PARAMETER_UNSUPPORTED);
    }
    {
        let mut nbWorkers: c_int = 0;
        FORWARD_IF_ERROR!(ZSTD_CCtx_getParameter(zc, ZSTD_c_nbWorkers, &mut nbWorkers));
        RETURN_ERROR_IF!(nbWorkers != 0, PARAMETER_UNSUPPORTED);
    }

    dst = zstd_custom_malloc(dstCapacity, ZSTD_defaultCMem);
    RETURN_ERROR_IF!(dst.is_null(), MEMORY_ALLOCATION);

    seqCollector.collectSequences = 1;
    seqCollector.seqStart = outSeqs;
    seqCollector.seqIndex = 0;
    seqCollector.maxSequences = outSeqsSize;
    (*zc).seqCollector = seqCollector;

    {
        let ret = ZSTD_compress2(zc, dst, dstCapacity, src, srcSize);
        zstd_custom_free(dst, ZSTD_defaultCMem);
        FORWARD_IF_ERROR!(ret);
    }
    (*zc).seqCollector.seqIndex
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_mergeBlockDelimiters(
    sequences: *mut ZSTD_Sequence,
    seqsSize: usize,
) -> usize {
    let mut r#in = 0;
    let mut out = 0;
    while r#in < seqsSize {
        if (*sequences.add(r#in)).offset == 0 && (*sequences.add(r#in)).matchLength == 0 {
            if r#in != seqsSize - 1 {
                (*sequences.add(r#in + 1)).litLength += (*sequences.add(r#in)).litLength;
            }
        } else {
            *sequences.add(out) = *sequences.add(r#in);
            out += 1;
        }
        r#in += 1;
    }
    out
}

unsafe fn ZSTD_isRLE(src: *const u8, length: usize) -> c_int {
    let ip = src;
    let value = *ip.add(0);
    let valueST: usize = (value as U64).wrapping_mul(0x0101010101010101u64) as usize;
    let unrollSize: usize = core::mem::size_of::<usize>() * 4;
    let unrollMask: usize = unrollSize - 1;
    let prefixLength: usize = length & unrollMask;
    let mut i;
    if length == 1 {
        return 1;
    }
    if prefixLength != 0
        && ZSTD_count(ip.add(1), ip, ip.add(prefixLength)) != prefixLength - 1
    {
        return 0;
    }
    i = prefixLength;
    while i != length {
        let mut u = 0;
        while u < unrollSize {
            if MEM_readST(ip.add(i + u) as *const c_void) != valueST {
                return 0;
            }
            u += core::mem::size_of::<usize>();
        }
        i += unrollSize;
    }
    1
}

unsafe fn ZSTD_maybeRLE(seqStore: *const SeqStore_t) -> c_int {
    let nbSeqs: usize =
        ((*seqStore).sequences as usize - (*seqStore).sequencesStart as usize)
            / core::mem::size_of::<SeqDef>();
    let nbLits: usize = (*seqStore).lit as usize - (*seqStore).litStart as usize;
    (nbSeqs < 4 && nbLits < 10) as c_int
}

unsafe fn ZSTD_blockState_confirmRepcodesAndEntropyTables(bs: *mut ZSTD_blockState_t) {
    let tmp = (*bs).prevCBlock;
    (*bs).prevCBlock = (*bs).nextCBlock;
    (*bs).nextCBlock = tmp;
}

unsafe fn writeBlockHeader(op: *mut c_void, cSize: usize, blockSize: usize, lastBlock: U32) {
    let cBlockHeader: U32 = if cSize == 1 {
        lastBlock + ((bt_rle) << 1) + ((blockSize << 3) as U32)
    } else {
        lastBlock + ((bt_compressed) << 1) + ((cSize << 3) as U32)
    };
    MEM_writeLE24(op, cBlockHeader);
}

unsafe fn ZSTD_buildBlockEntropyStats_literals(
    src: *mut c_void,
    srcSize: usize,
    prevHuf: *const ZSTD_hufCTables_t,
    nextHuf: *mut ZSTD_hufCTables_t,
    hufMetadata: *mut ZSTD_hufCTablesMetadata_t,
    literalsCompressionIsDisabled: c_int,
    workspace: *mut c_void,
    wkspSize: usize,
    hufFlags: c_int,
) -> usize {
    let wkspStart: *mut u8 = workspace as *mut u8;
    let wkspEnd: *mut u8 = wkspStart.add(wkspSize);
    let countWkspStart: *mut u8 = wkspStart;
    let countWksp: *mut c_uint = workspace as *mut c_uint;
    let countWkspSize: usize = (HUF_SYMBOLVALUE_MAX as usize + 1) * core::mem::size_of::<c_uint>();
    let nodeWksp: *mut u8 = countWkspStart.add(countWkspSize);
    let nodeWkspSize: usize = wkspEnd as usize - nodeWksp as usize;
    let mut maxSymbolValue: c_uint = HUF_SYMBOLVALUE_MAX;
    let mut huffLog: c_uint = LitHufLog;
    let mut repeat: HUF_repeat = (*prevHuf).repeatMode;

    memcpy(
        nextHuf as *mut c_void,
        prevHuf as *const c_void,
        core::mem::size_of::<ZSTD_hufCTables_t>(),
    );

    if literalsCompressionIsDisabled != 0 {
        (*hufMetadata).hType = set_basic;
        return 0;
    }

    {
        let minLitSize: usize = if (*prevHuf).repeatMode == HUF_repeat_valid {
            6
        } else {
            COMPRESS_LITERALS_SIZE_MIN
        };
        if srcSize <= minLitSize {
            (*hufMetadata).hType = set_basic;
            return 0;
        }
    }

    {
        let largest = HIST_count_wksp(
            countWksp,
            &mut maxSymbolValue,
            src as *const u8 as *const c_void,
            srcSize,
            workspace,
            wkspSize,
        );
        FORWARD_IF_ERROR!(largest);
        if largest == srcSize {
            (*hufMetadata).hType = set_rle;
            return 0;
        }
        if largest <= (srcSize >> 7) + 4 {
            (*hufMetadata).hType = set_basic;
            return 0;
        }
    }

    if repeat == HUF_repeat_check
        && HUF_validateCTable(
            (*prevHuf).CTable.as_ptr() as *const HUF_CElt,
            countWksp,
            maxSymbolValue,
        ) == 0
    {
        repeat = HUF_repeat_none;
    }

    memset(
        (*nextHuf).CTable.as_mut_ptr() as *mut c_void,
        0,
        core::mem::size_of_val(&(*nextHuf).CTable),
    );
    huffLog = HUF_optimalTableLog(
        huffLog,
        srcSize,
        maxSymbolValue,
        nodeWksp as *mut c_void,
        nodeWkspSize,
        (*nextHuf).CTable.as_mut_ptr() as *mut HUF_CElt,
        countWksp,
        hufFlags,
    );
    debug_assert!(huffLog <= LitHufLog);
    {
        let maxBits = HUF_buildCTable_wksp(
            (*nextHuf).CTable.as_mut_ptr() as *mut HUF_CElt,
            countWksp,
            maxSymbolValue,
            huffLog,
            nodeWksp as *mut c_void,
            nodeWkspSize,
        );
        FORWARD_IF_ERROR!(maxBits);
        huffLog = maxBits as U32;
    }
    {
        let newCSize = HUF_estimateCompressedSize(
            (*nextHuf).CTable.as_ptr() as *const HUF_CElt,
            countWksp,
            maxSymbolValue,
        );
        let hSize = HUF_writeCTable_wksp(
            (*hufMetadata).hufDesBuffer.as_mut_ptr() as *mut c_void,
            core::mem::size_of_val(&(*hufMetadata).hufDesBuffer),
            (*nextHuf).CTable.as_ptr() as *const HUF_CElt,
            maxSymbolValue,
            huffLog,
            nodeWksp as *mut c_void,
            nodeWkspSize,
        );
        if repeat != HUF_repeat_none {
            let oldCSize = HUF_estimateCompressedSize(
                (*prevHuf).CTable.as_ptr() as *const HUF_CElt,
                countWksp,
                maxSymbolValue,
            );
            if oldCSize < srcSize && (oldCSize <= hSize + newCSize || hSize + 12 >= srcSize) {
                memcpy(
                    nextHuf as *mut c_void,
                    prevHuf as *const c_void,
                    core::mem::size_of::<ZSTD_hufCTables_t>(),
                );
                (*hufMetadata).hType = set_repeat;
                return 0;
            }
        }
        if newCSize + hSize >= srcSize {
            memcpy(
                nextHuf as *mut c_void,
                prevHuf as *const c_void,
                core::mem::size_of::<ZSTD_hufCTables_t>(),
            );
            (*hufMetadata).hType = set_basic;
            return 0;
        }
        (*hufMetadata).hType = set_compressed;
        (*nextHuf).repeatMode = HUF_repeat_check;
        return hSize;
    }
}

unsafe fn ZSTD_buildDummySequencesStatistics(
    nextEntropy: *mut ZSTD_fseCTables_t,
) -> ZSTD_symbolEncodingTypeStats_t {
    let stats = ZSTD_symbolEncodingTypeStats_t {
        LLtype: set_basic,
        Offtype: set_basic,
        MLtype: set_basic,
        size: 0,
        lastCountSize: 0,
        longOffsets: 0,
    };
    (*nextEntropy).litlength_repeatMode = FSE_repeat_none;
    (*nextEntropy).offcode_repeatMode = FSE_repeat_none;
    (*nextEntropy).matchlength_repeatMode = FSE_repeat_none;
    stats
}

unsafe fn ZSTD_buildBlockEntropyStats_sequences(
    seqStorePtr: *const SeqStore_t,
    prevEntropy: *const ZSTD_fseCTables_t,
    nextEntropy: *mut ZSTD_fseCTables_t,
    cctxParams: *const ZSTD_CCtx_params,
    fseMetadata: *mut ZSTD_fseCTablesMetadata_t,
    workspace: *mut c_void,
    wkspSize: usize,
) -> usize {
    let strategy = (*cctxParams).cParams.strategy;
    let nbSeq: usize =
        ((*seqStorePtr).sequences as usize - (*seqStorePtr).sequencesStart as usize)
            / core::mem::size_of::<SeqDef>();
    let ostart: *mut u8 = (*fseMetadata).fseTablesBuffer.as_mut_ptr();
    let oend: *const u8 = ostart.add(core::mem::size_of_val(&(*fseMetadata).fseTablesBuffer));
    let op: *mut u8 = ostart;
    let countWorkspace: *mut c_uint = workspace as *mut c_uint;
    let entropyWorkspace: *mut c_uint = countWorkspace.add(MaxSeq as usize + 1);
    let entropyWorkspaceSize: usize =
        wkspSize - (MaxSeq as usize + 1) * core::mem::size_of::<c_uint>();
    let stats: ZSTD_symbolEncodingTypeStats_t;

    stats = if nbSeq != 0 {
        ZSTD_buildSequencesStatistics(
            seqStorePtr,
            nbSeq,
            prevEntropy,
            nextEntropy,
            op,
            oend,
            strategy,
            countWorkspace,
            entropyWorkspace as *mut c_void,
            entropyWorkspaceSize,
        )
    } else {
        ZSTD_buildDummySequencesStatistics(nextEntropy)
    };
    FORWARD_IF_ERROR!(stats.size);
    (*fseMetadata).llType = stats.LLtype as SymbolEncodingType_e;
    (*fseMetadata).ofType = stats.Offtype as SymbolEncodingType_e;
    (*fseMetadata).mlType = stats.MLtype as SymbolEncodingType_e;
    (*fseMetadata).lastCountSize = stats.lastCountSize;
    stats.size
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_buildBlockEntropyStats(
    seqStorePtr: *const SeqStore_t,
    prevEntropy: *const ZSTD_entropyCTables_t,
    nextEntropy: *mut ZSTD_entropyCTables_t,
    cctxParams: *const ZSTD_CCtx_params,
    entropyMetadata: *mut ZSTD_entropyCTablesMetadata_t,
    workspace: *mut c_void,
    wkspSize: usize,
) -> usize {
    let litSize: usize = (*seqStorePtr).lit as usize - (*seqStorePtr).litStart as usize;
    let huf_useOptDepth: c_int =
        ((*cctxParams).cParams.strategy >= HUF_OPTIMAL_DEPTH_THRESHOLD) as c_int;
    let hufFlags: c_int = if huf_useOptDepth != 0 {
        HUF_flags_optimalDepth
    } else {
        0
    };

    (*entropyMetadata).hufMetadata.hufDesSize = ZSTD_buildBlockEntropyStats_literals(
        (*seqStorePtr).litStart as *mut c_void,
        litSize,
        &(*prevEntropy).huf,
        &mut (*nextEntropy).huf,
        &mut (*entropyMetadata).hufMetadata,
        ZSTD_literalsCompressionIsDisabled(cctxParams),
        workspace,
        wkspSize,
        hufFlags,
    );

    FORWARD_IF_ERROR!((*entropyMetadata).hufMetadata.hufDesSize);
    (*entropyMetadata).fseMetadata.fseTablesSize = ZSTD_buildBlockEntropyStats_sequences(
        seqStorePtr,
        &(*prevEntropy).fse,
        &mut (*nextEntropy).fse,
        cctxParams,
        &mut (*entropyMetadata).fseMetadata,
        workspace,
        wkspSize,
    );
    FORWARD_IF_ERROR!((*entropyMetadata).fseMetadata.fseTablesSize);
    0
}

unsafe fn ZSTD_estimateBlockSize_literal(
    literals: *const u8,
    litSize: usize,
    huf: *const ZSTD_hufCTables_t,
    hufMetadata: *const ZSTD_hufCTablesMetadata_t,
    workspace: *mut c_void,
    wkspSize: usize,
    writeEntropy: c_int,
) -> usize {
    let countWksp: *mut c_uint = workspace as *mut c_uint;
    let mut maxSymbolValue: c_uint = HUF_SYMBOLVALUE_MAX;
    let literalSectionHeaderSize: usize =
        3 + (litSize >= 1024) as usize + (litSize >= 16 * 1024) as usize;
    let singleStream: U32 = (litSize < 256) as U32;

    if (*hufMetadata).hType == set_basic {
        return litSize;
    } else if (*hufMetadata).hType == set_rle {
        return 1;
    } else if (*hufMetadata).hType == set_compressed || (*hufMetadata).hType == set_repeat {
        let largest = HIST_count_wksp(
            countWksp,
            &mut maxSymbolValue,
            literals as *const c_void,
            litSize,
            workspace,
            wkspSize,
        );
        if ZSTD_isError(largest) != 0 {
            return litSize;
        }
        {
            let mut cLitSizeEstimate = HUF_estimateCompressedSize(
                (*huf).CTable.as_ptr() as *const HUF_CElt,
                countWksp,
                maxSymbolValue,
            );
            if writeEntropy != 0 {
                cLitSizeEstimate += (*hufMetadata).hufDesSize;
            }
            if singleStream == 0 {
                cLitSizeEstimate += 6;
            }
            return cLitSizeEstimate + literalSectionHeaderSize;
        }
    }
    debug_assert!(false);
    0
}

unsafe fn ZSTD_estimateBlockSize_symbolType(
    r#type: SymbolEncodingType_e,
    codeTable: *const u8,
    nbSeq: usize,
    maxCode: c_uint,
    fseCTable: *const FSE_CTable,
    additionalBits: *const u8,
    defaultNorm: *const i16,
    defaultNormLog: U32,
    defaultMax: U32,
    workspace: *mut c_void,
    wkspSize: usize,
) -> usize {
    let countWksp: *mut c_uint = workspace as *mut c_uint;
    let mut ctp: *const u8 = codeTable;
    let ctStart: *const u8 = ctp;
    let ctEnd: *const u8 = ctStart.add(nbSeq);
    let mut cSymbolTypeSizeEstimateInBits: usize = 0;
    let mut max: c_uint = maxCode;

    HIST_countFast_wksp(
        countWksp,
        &mut max,
        codeTable as *const c_void,
        nbSeq,
        workspace,
        wkspSize,
    );
    if r#type == set_basic {
        debug_assert!(max <= defaultMax);
        cSymbolTypeSizeEstimateInBits =
            ZSTD_crossEntropyCost(defaultNorm, defaultNormLog, countWksp, max);
    } else if r#type == set_rle {
        cSymbolTypeSizeEstimateInBits = 0;
    } else if r#type == set_compressed || r#type == set_repeat {
        cSymbolTypeSizeEstimateInBits = ZSTD_fseBitCost(fseCTable, countWksp, max);
    }
    if ZSTD_isError(cSymbolTypeSizeEstimateInBits) != 0 {
        return nbSeq * 10;
    }
    while ctp < ctEnd {
        if !additionalBits.is_null() {
            cSymbolTypeSizeEstimateInBits += *additionalBits.add(*ctp as usize) as usize;
        } else {
            cSymbolTypeSizeEstimateInBits += *ctp as usize;
        }
        ctp = ctp.add(1);
    }
    cSymbolTypeSizeEstimateInBits >> 3
}

unsafe fn ZSTD_estimateBlockSize_sequences(
    ofCodeTable: *const u8,
    llCodeTable: *const u8,
    mlCodeTable: *const u8,
    nbSeq: usize,
    fseTables: *const ZSTD_fseCTables_t,
    fseMetadata: *const ZSTD_fseCTablesMetadata_t,
    workspace: *mut c_void,
    wkspSize: usize,
    writeEntropy: c_int,
) -> usize {
    let sequencesSectionHeaderSize: usize =
        1 + 1 + (nbSeq >= 128) as usize + (nbSeq >= LONGNBSEQ as usize) as usize;
    let mut cSeqSizeEstimate: usize = 0;
    cSeqSizeEstimate += ZSTD_estimateBlockSize_symbolType(
        (*fseMetadata).ofType,
        ofCodeTable,
        nbSeq,
        MaxOff,
        (*fseTables).offcodeCTable.as_ptr(),
        core::ptr::null(),
        OF_defaultNorm.as_ptr(),
        OF_defaultNormLog,
        DefaultMaxOff,
        workspace,
        wkspSize,
    );
    cSeqSizeEstimate += ZSTD_estimateBlockSize_symbolType(
        (*fseMetadata).llType,
        llCodeTable,
        nbSeq,
        MaxLL,
        (*fseTables).litlengthCTable.as_ptr(),
        LL_bits.as_ptr(),
        LL_defaultNorm.as_ptr(),
        LL_defaultNormLog,
        MaxLL,
        workspace,
        wkspSize,
    );
    cSeqSizeEstimate += ZSTD_estimateBlockSize_symbolType(
        (*fseMetadata).mlType,
        mlCodeTable,
        nbSeq,
        MaxML,
        (*fseTables).matchlengthCTable.as_ptr(),
        ML_bits.as_ptr(),
        ML_defaultNorm.as_ptr(),
        ML_defaultNormLog,
        MaxML,
        workspace,
        wkspSize,
    );
    if writeEntropy != 0 {
        cSeqSizeEstimate += (*fseMetadata).fseTablesSize;
    }
    cSeqSizeEstimate + sequencesSectionHeaderSize
}

unsafe fn ZSTD_estimateBlockSize(
    literals: *const u8,
    litSize: usize,
    ofCodeTable: *const u8,
    llCodeTable: *const u8,
    mlCodeTable: *const u8,
    nbSeq: usize,
    entropy: *const ZSTD_entropyCTables_t,
    entropyMetadata: *const ZSTD_entropyCTablesMetadata_t,
    workspace: *mut c_void,
    wkspSize: usize,
    writeLitEntropy: c_int,
    writeSeqEntropy: c_int,
) -> usize {
    let literalsSize = ZSTD_estimateBlockSize_literal(
        literals,
        litSize,
        &(*entropy).huf,
        &(*entropyMetadata).hufMetadata,
        workspace,
        wkspSize,
        writeLitEntropy,
    );
    let seqSize = ZSTD_estimateBlockSize_sequences(
        ofCodeTable,
        llCodeTable,
        mlCodeTable,
        nbSeq,
        &(*entropy).fse,
        &(*entropyMetadata).fseMetadata,
        workspace,
        wkspSize,
        writeSeqEntropy,
    );
    seqSize + literalsSize + ZSTD_blockHeaderSize
}

unsafe fn ZSTD_buildEntropyStatisticsAndEstimateSubBlockSize(
    seqStore: *mut SeqStore_t,
    zc: *mut ZSTD_CCtx,
) -> usize {
    let entropyMetadata: *mut ZSTD_entropyCTablesMetadata_t =
        &mut (*zc).blockSplitCtx.entropyMetadata;
    FORWARD_IF_ERROR!(ZSTD_buildBlockEntropyStats(
        seqStore,
        &(*(*zc).blockState.prevCBlock).entropy,
        &mut (*(*zc).blockState.nextCBlock).entropy,
        &(*zc).appliedParams,
        entropyMetadata,
        (*zc).tmpWorkspace,
        (*zc).tmpWkspSize,
    ));
    ZSTD_estimateBlockSize(
        (*seqStore).litStart,
        ((*seqStore).lit as usize - (*seqStore).litStart as usize),
        (*seqStore).ofCode,
        (*seqStore).llCode,
        (*seqStore).mlCode,
        ((*seqStore).sequences as usize - (*seqStore).sequencesStart as usize)
            / core::mem::size_of::<SeqDef>(),
        &(*(*zc).blockState.nextCBlock).entropy,
        entropyMetadata,
        (*zc).tmpWorkspace,
        (*zc).tmpWkspSize,
        ((*entropyMetadata).hufMetadata.hType == set_compressed) as c_int,
        1,
    )
}

unsafe fn ZSTD_countSeqStoreLiteralsBytes(seqStore: *const SeqStore_t) -> usize {
    let mut literalsBytes: usize = 0;
    let nbSeqs: usize =
        ((*seqStore).sequences as usize - (*seqStore).sequencesStart as usize)
            / core::mem::size_of::<SeqDef>();
    let mut i = 0;
    while i < nbSeqs {
        let seq = *(*seqStore).sequencesStart.add(i);
        literalsBytes += seq.litLength as usize;
        if i == (*seqStore).longLengthPos as usize
            && (*seqStore).longLengthType == ZSTD_llt_literalLength
        {
            literalsBytes += 0x10000;
        }
        i += 1;
    }
    literalsBytes
}

unsafe fn ZSTD_countSeqStoreMatchBytes(seqStore: *const SeqStore_t) -> usize {
    let mut matchBytes: usize = 0;
    let nbSeqs: usize =
        ((*seqStore).sequences as usize - (*seqStore).sequencesStart as usize)
            / core::mem::size_of::<SeqDef>();
    let mut i = 0;
    while i < nbSeqs {
        let seq = *(*seqStore).sequencesStart.add(i);
        matchBytes += seq.mlBase as usize + MINMATCH as usize;
        if i == (*seqStore).longLengthPos as usize
            && (*seqStore).longLengthType == ZSTD_llt_matchLength
        {
            matchBytes += 0x10000;
        }
        i += 1;
    }
    matchBytes
}

unsafe fn ZSTD_deriveSeqStoreChunk(
    resultSeqStore: *mut SeqStore_t,
    originalSeqStore: *const SeqStore_t,
    startIdx: usize,
    endIdx: usize,
) {
    *resultSeqStore = *originalSeqStore;
    if startIdx > 0 {
        (*resultSeqStore).sequences = (*originalSeqStore).sequencesStart.add(startIdx);
        (*resultSeqStore).litStart = (*resultSeqStore)
            .litStart
            .add(ZSTD_countSeqStoreLiteralsBytes(resultSeqStore));
    }

    if (*originalSeqStore).longLengthType != ZSTD_llt_none {
        if ((*originalSeqStore).longLengthPos as usize) < startIdx
            || (*originalSeqStore).longLengthPos as usize > endIdx
        {
            (*resultSeqStore).longLengthType = ZSTD_llt_none;
        } else {
            (*resultSeqStore).longLengthPos -= startIdx as U32;
        }
    }
    (*resultSeqStore).sequencesStart = (*originalSeqStore).sequencesStart.add(startIdx);
    (*resultSeqStore).sequences = (*originalSeqStore).sequencesStart.add(endIdx);
    if endIdx
        == ((*originalSeqStore).sequences as usize - (*originalSeqStore).sequencesStart as usize)
            / core::mem::size_of::<SeqDef>()
    {
        debug_assert!((*resultSeqStore).lit == (*originalSeqStore).lit);
    } else {
        let literalsBytes = ZSTD_countSeqStoreLiteralsBytes(resultSeqStore);
        (*resultSeqStore).lit = (*resultSeqStore).litStart.add(literalsBytes);
    }
    (*resultSeqStore).llCode = (*resultSeqStore).llCode.add(startIdx);
    (*resultSeqStore).mlCode = (*resultSeqStore).mlCode.add(startIdx);
    (*resultSeqStore).ofCode = (*resultSeqStore).ofCode.add(startIdx);
}

unsafe fn ZSTD_resolveRepcodeToRawOffset(rep: *const U32, offBase: U32, ll0: U32) -> U32 {
    let adjustedRepCode: U32 = OFFBASE_TO_REPCODE(offBase) - 1 + ll0;
    debug_assert!(OFFBASE_IS_REPCODE(offBase));
    if adjustedRepCode == ZSTD_REP_NUM as U32 {
        debug_assert!(ll0 != 0);
        return *rep.add(0) - 1;
    }
    *rep.add(adjustedRepCode as usize)
}

unsafe fn ZSTD_seqStore_resolveOffCodes(
    dRepcodes: *mut Repcodes_t,
    cRepcodes: *mut Repcodes_t,
    seqStore: *const SeqStore_t,
    nbSeq: U32,
) {
    let mut idx: U32 = 0;
    let longLitLenIdx: U32 = if (*seqStore).longLengthType == ZSTD_llt_literalLength {
        (*seqStore).longLengthPos
    } else {
        nbSeq
    };
    while idx < nbSeq {
        let seq: *mut SeqDef = (*seqStore).sequencesStart.add(idx as usize);
        let ll0: U32 = (((*seq).litLength == 0) && (idx != longLitLenIdx)) as U32;
        let offBase: U32 = (*seq).offBase;
        debug_assert!(offBase > 0);
        if OFFBASE_IS_REPCODE(offBase) {
            let dRawOffset =
                ZSTD_resolveRepcodeToRawOffset((*dRepcodes).rep.as_ptr(), offBase, ll0);
            let cRawOffset =
                ZSTD_resolveRepcodeToRawOffset((*cRepcodes).rep.as_ptr(), offBase, ll0);
            if dRawOffset != cRawOffset {
                (*seq).offBase = OFFSET_TO_OFFBASE(cRawOffset);
            }
        }
        ZSTD_updateRep((*dRepcodes).rep.as_mut_ptr(), (*seq).offBase, ll0);
        ZSTD_updateRep((*cRepcodes).rep.as_mut_ptr(), offBase, ll0);
        idx += 1;
    }
}

unsafe fn ZSTD_compressSeqStore_singleBlock(
    zc: *mut ZSTD_CCtx,
    seqStore: *const SeqStore_t,
    dRep: *mut Repcodes_t,
    cRep: *mut Repcodes_t,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
    lastBlock: U32,
    isPartition: U32,
) -> usize {
    let rleMaxLength: U32 = 25;
    let op: *mut u8 = dst as *mut u8;
    let ip: *const u8 = src as *const u8;
    let mut cSize: usize;
    let mut cSeqsSize: usize;

    let dRepOriginal: Repcodes_t = *dRep;
    if isPartition != 0 {
        ZSTD_seqStore_resolveOffCodes(
            dRep,
            cRep,
            seqStore,
            (((*seqStore).sequences as usize - (*seqStore).sequencesStart as usize)
                / core::mem::size_of::<SeqDef>()) as U32,
        );
    }

    RETURN_ERROR_IF!(dstCapacity < ZSTD_blockHeaderSize, DSTSIZE_TOOSMALL);
    cSeqsSize = ZSTD_entropyCompressSeqStore(
        seqStore,
        &(*(*zc).blockState.prevCBlock).entropy,
        &mut (*(*zc).blockState.nextCBlock).entropy,
        &(*zc).appliedParams,
        op.add(ZSTD_blockHeaderSize) as *mut c_void,
        dstCapacity - ZSTD_blockHeaderSize,
        srcSize,
        (*zc).tmpWorkspace,
        (*zc).tmpWkspSize,
        (*zc).bmi2,
    );
    FORWARD_IF_ERROR!(cSeqsSize);

    if (*zc).isFirstBlock == 0
        && cSeqsSize < rleMaxLength as usize
        && ZSTD_isRLE(src as *const u8, srcSize) != 0
    {
        cSeqsSize = 1;
    }

    if (*zc).seqCollector.collectSequences != 0 {
        FORWARD_IF_ERROR!(ZSTD_copyBlockSequences(
            &mut (*zc).seqCollector,
            seqStore,
            dRepOriginal.rep.as_ptr()
        ));
        ZSTD_blockState_confirmRepcodesAndEntropyTables(&mut (*zc).blockState);
        return 0;
    }

    if cSeqsSize == 0 {
        cSize = ZSTD_noCompressBlock(op as *mut c_void, dstCapacity, ip as *const c_void, srcSize, lastBlock);
        FORWARD_IF_ERROR!(cSize);
        *dRep = dRepOriginal;
    } else if cSeqsSize == 1 {
        cSize = ZSTD_rleCompressBlock(op as *mut c_void, dstCapacity, *ip, srcSize, lastBlock);
        FORWARD_IF_ERROR!(cSize);
        *dRep = dRepOriginal;
    } else {
        ZSTD_blockState_confirmRepcodesAndEntropyTables(&mut (*zc).blockState);
        writeBlockHeader(op as *mut c_void, cSeqsSize, srcSize, lastBlock);
        cSize = ZSTD_blockHeaderSize + cSeqsSize;
    }

    if (*(*zc).blockState.prevCBlock).entropy.fse.offcode_repeatMode == FSE_repeat_valid {
        (*(*zc).blockState.prevCBlock).entropy.fse.offcode_repeatMode = FSE_repeat_check;
    }

    cSize
}

#[repr(C)]
struct seqStoreSplits {
    splitLocations: *mut U32,
    idx: usize,
}

const MIN_SEQUENCES_BLOCK_SPLITTING: usize = 300;

unsafe fn ZSTD_deriveBlockSplitsHelper(
    splits: *mut seqStoreSplits,
    startIdx: usize,
    endIdx: usize,
    zc: *mut ZSTD_CCtx,
    origSeqStore: *const SeqStore_t,
) {
    let fullSeqStoreChunk: *mut SeqStore_t = &mut (*zc).blockSplitCtx.fullSeqStoreChunk;
    let firstHalfSeqStore: *mut SeqStore_t = &mut (*zc).blockSplitCtx.firstHalfSeqStore;
    let secondHalfSeqStore: *mut SeqStore_t = &mut (*zc).blockSplitCtx.secondHalfSeqStore;
    let estimatedOriginalSize: usize;
    let estimatedFirstHalfSize: usize;
    let estimatedSecondHalfSize: usize;
    let midIdx: usize = (startIdx + endIdx) / 2;

    debug_assert!(endIdx >= startIdx);
    if endIdx - startIdx < MIN_SEQUENCES_BLOCK_SPLITTING
        || (*splits).idx >= ZSTD_MAX_NB_BLOCK_SPLITS
    {
        return;
    }
    ZSTD_deriveSeqStoreChunk(fullSeqStoreChunk, origSeqStore, startIdx, endIdx);
    ZSTD_deriveSeqStoreChunk(firstHalfSeqStore, origSeqStore, startIdx, midIdx);
    ZSTD_deriveSeqStoreChunk(secondHalfSeqStore, origSeqStore, midIdx, endIdx);
    estimatedOriginalSize = ZSTD_buildEntropyStatisticsAndEstimateSubBlockSize(fullSeqStoreChunk, zc);
    estimatedFirstHalfSize =
        ZSTD_buildEntropyStatisticsAndEstimateSubBlockSize(firstHalfSeqStore, zc);
    estimatedSecondHalfSize =
        ZSTD_buildEntropyStatisticsAndEstimateSubBlockSize(secondHalfSeqStore, zc);
    if ZSTD_isError(estimatedOriginalSize) != 0
        || ZSTD_isError(estimatedFirstHalfSize) != 0
        || ZSTD_isError(estimatedSecondHalfSize) != 0
    {
        return;
    }
    if estimatedFirstHalfSize + estimatedSecondHalfSize < estimatedOriginalSize {
        ZSTD_deriveBlockSplitsHelper(splits, startIdx, midIdx, zc, origSeqStore);
        *(*splits).splitLocations.add((*splits).idx) = midIdx as U32;
        (*splits).idx += 1;
        ZSTD_deriveBlockSplitsHelper(splits, midIdx, endIdx, zc, origSeqStore);
    }
}

unsafe fn ZSTD_deriveBlockSplits(zc: *mut ZSTD_CCtx, partitions: *mut U32, nbSeq: U32) -> usize {
    let mut splits: seqStoreSplits = seqStoreSplits {
        splitLocations: partitions,
        idx: 0,
    };
    if nbSeq <= 4 {
        return 0;
    }
    ZSTD_deriveBlockSplitsHelper(&mut splits, 0, nbSeq as usize, zc, &(*zc).seqStore);
    *splits.splitLocations.add(splits.idx) = nbSeq;
    splits.idx
}

unsafe fn ZSTD_compressBlock_splitBlock_internal(
    zc: *mut ZSTD_CCtx,
    dst: *mut c_void,
    mut dstCapacity: usize,
    src: *const c_void,
    blockSize: usize,
    lastBlock: U32,
    nbSeq: U32,
) -> usize {
    let mut cSize: usize = 0;
    let mut ip: *const u8 = src as *const u8;
    let mut op: *mut u8 = dst as *mut u8;
    let mut i: usize = 0;
    let mut srcBytesTotal: usize = 0;
    let partitions: *mut U32 = (*zc).blockSplitCtx.partitions.as_mut_ptr();
    let nextSeqStore: *mut SeqStore_t = &mut (*zc).blockSplitCtx.nextSeqStore;
    let currSeqStore: *mut SeqStore_t = &mut (*zc).blockSplitCtx.currSeqStore;
    let numSplits: usize = ZSTD_deriveBlockSplits(zc, partitions, nbSeq);

    let mut dRep: Repcodes_t = core::mem::zeroed();
    let mut cRep: Repcodes_t = core::mem::zeroed();
    memcpy(
        dRep.rep.as_mut_ptr() as *mut c_void,
        (*(*zc).blockState.prevCBlock).rep.as_ptr() as *const c_void,
        core::mem::size_of::<Repcodes_t>(),
    );
    memcpy(
        cRep.rep.as_mut_ptr() as *mut c_void,
        (*(*zc).blockState.prevCBlock).rep.as_ptr() as *const c_void,
        core::mem::size_of::<Repcodes_t>(),
    );
    memset(
        nextSeqStore as *mut c_void,
        0,
        core::mem::size_of::<SeqStore_t>(),
    );

    if numSplits == 0 {
        let cSizeSingleBlock = ZSTD_compressSeqStore_singleBlock(
            zc,
            &(*zc).seqStore,
            &mut dRep,
            &mut cRep,
            op as *mut c_void,
            dstCapacity,
            ip as *const c_void,
            blockSize,
            lastBlock,
            0,
        );
        FORWARD_IF_ERROR!(cSizeSingleBlock);
        debug_assert!((*zc).blockSizeMax <= ZSTD_BLOCKSIZE_MAX);
        debug_assert!(cSizeSingleBlock <= (*zc).blockSizeMax + ZSTD_blockHeaderSize);
        return cSizeSingleBlock;
    }

    ZSTD_deriveSeqStoreChunk(currSeqStore, &(*zc).seqStore, 0, *partitions.add(0) as usize);
    i = 0;
    while i <= numSplits {
        let cSizeChunk: usize;
        let lastPartition: U32 = (i == numSplits) as U32;
        let mut lastBlockEntireSrc: U32 = 0;

        let mut srcBytes: usize = ZSTD_countSeqStoreLiteralsBytes(currSeqStore)
            + ZSTD_countSeqStoreMatchBytes(currSeqStore);
        srcBytesTotal += srcBytes;
        if lastPartition != 0 {
            srcBytes += blockSize - srcBytesTotal;
            lastBlockEntireSrc = lastBlock;
        } else {
            ZSTD_deriveSeqStoreChunk(
                nextSeqStore,
                &(*zc).seqStore,
                *partitions.add(i) as usize,
                *partitions.add(i + 1) as usize,
            );
        }

        cSizeChunk = ZSTD_compressSeqStore_singleBlock(
            zc,
            currSeqStore,
            &mut dRep,
            &mut cRep,
            op as *mut c_void,
            dstCapacity,
            ip as *const c_void,
            srcBytes,
            lastBlockEntireSrc,
            1,
        );
        FORWARD_IF_ERROR!(cSizeChunk);

        ip = ip.add(srcBytes);
        op = op.add(cSizeChunk);
        dstCapacity -= cSizeChunk;
        cSize += cSizeChunk;
        *currSeqStore = *nextSeqStore;
        debug_assert!(cSizeChunk <= (*zc).blockSizeMax + ZSTD_blockHeaderSize);
        i += 1;
    }
    memcpy(
        (*(*zc).blockState.prevCBlock).rep.as_mut_ptr() as *mut c_void,
        dRep.rep.as_ptr() as *const c_void,
        core::mem::size_of::<Repcodes_t>(),
    );
    cSize
}

unsafe fn ZSTD_compressBlock_splitBlock(
    zc: *mut ZSTD_CCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
    lastBlock: U32,
) -> usize {
    let nbSeq: U32;
    let cSize: usize;
    debug_assert!((*zc).appliedParams.postBlockSplitter == ZSTD_ps_enable);

    {
        let bss = ZSTD_buildSeqStore(zc, src, srcSize);
        FORWARD_IF_ERROR!(bss);
        if bss == ZSTDbss_noCompress as usize {
            if (*(*zc).blockState.prevCBlock).entropy.fse.offcode_repeatMode == FSE_repeat_valid {
                (*(*zc).blockState.prevCBlock).entropy.fse.offcode_repeatMode = FSE_repeat_check;
            }
            RETURN_ERROR_IF!((*zc).seqCollector.collectSequences != 0, SEQUENCEPRODUCER_FAILED);
            let cSize0 = ZSTD_noCompressBlock(dst, dstCapacity, src, srcSize, lastBlock);
            FORWARD_IF_ERROR!(cSize0);
            return cSize0;
        }
        nbSeq = (((*zc).seqStore.sequences as usize - (*zc).seqStore.sequencesStart as usize)
            / core::mem::size_of::<SeqDef>()) as U32;
    }

    cSize = ZSTD_compressBlock_splitBlock_internal(zc, dst, dstCapacity, src, srcSize, lastBlock, nbSeq);
    FORWARD_IF_ERROR!(cSize);
    cSize
}

unsafe fn ZSTD_compressBlock_internal(
    zc: *mut ZSTD_CCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
    frame: U32,
) -> usize {
    let rleMaxLength: U32 = 25;
    let mut cSize: usize;
    let ip: *const u8 = src as *const u8;
    let op: *mut u8 = dst as *mut u8;

    'out: {
        {
            let bss = ZSTD_buildSeqStore(zc, src, srcSize);
            FORWARD_IF_ERROR!(bss);
            if bss == ZSTDbss_noCompress as usize {
                RETURN_ERROR_IF!((*zc).seqCollector.collectSequences != 0, SEQUENCEPRODUCER_FAILED);
                cSize = 0;
                break 'out;
            }
        }

        if (*zc).seqCollector.collectSequences != 0 {
            FORWARD_IF_ERROR!(ZSTD_copyBlockSequences(
                &mut (*zc).seqCollector,
                ZSTD_getSeqStore(zc),
                (*(*zc).blockState.prevCBlock).rep.as_ptr()
            ));
            ZSTD_blockState_confirmRepcodesAndEntropyTables(&mut (*zc).blockState);
            return 0;
        }

        cSize = ZSTD_entropyCompressSeqStore(
            &(*zc).seqStore,
            &(*(*zc).blockState.prevCBlock).entropy,
            &mut (*(*zc).blockState.nextCBlock).entropy,
            &(*zc).appliedParams,
            dst,
            dstCapacity,
            srcSize,
            (*zc).tmpWorkspace,
            (*zc).tmpWkspSize,
            (*zc).bmi2,
        );

        if frame != 0
            && (*zc).isFirstBlock == 0
            && cSize < rleMaxLength as usize
            && ZSTD_isRLE(ip, srcSize) != 0
        {
            cSize = 1;
            *op.add(0) = *ip.add(0);
        }
    }

    // out:
    if ZSTD_isError(cSize) == 0 && cSize > 1 {
        ZSTD_blockState_confirmRepcodesAndEntropyTables(&mut (*zc).blockState);
    }
    if (*(*zc).blockState.prevCBlock).entropy.fse.offcode_repeatMode == FSE_repeat_valid {
        (*(*zc).blockState.prevCBlock).entropy.fse.offcode_repeatMode = FSE_repeat_check;
    }

    cSize
}

unsafe fn ZSTD_compressBlock_targetCBlockSize_body(
    zc: *mut ZSTD_CCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
    bss: usize,
    lastBlock: U32,
) -> usize {
    if bss == ZSTDbss_compress as usize {
        if (*zc).isFirstBlock == 0
            && ZSTD_maybeRLE(&(*zc).seqStore) != 0
            && ZSTD_isRLE(src as *const u8, srcSize) != 0
        {
            return ZSTD_rleCompressBlock(dst, dstCapacity, *(src as *const u8), srcSize, lastBlock);
        }
        {
            let cSize = ZSTD_compressSuperBlock(zc, dst, dstCapacity, src, srcSize, lastBlock);
            if cSize != error(code::DSTSIZE_TOOSMALL) {
                let maxCSize =
                    srcSize - ZSTD_minGain(srcSize, (*zc).appliedParams.cParams.strategy);
                FORWARD_IF_ERROR!(cSize);
                if cSize != 0 && cSize < maxCSize + ZSTD_blockHeaderSize {
                    ZSTD_blockState_confirmRepcodesAndEntropyTables(&mut (*zc).blockState);
                    return cSize;
                }
            }
        }
    }

    ZSTD_noCompressBlock(dst, dstCapacity, src, srcSize, lastBlock)
}

unsafe fn ZSTD_compressBlock_targetCBlockSize(
    zc: *mut ZSTD_CCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
    lastBlock: U32,
) -> usize {
    let mut cSize: usize = 0;
    let bss = ZSTD_buildSeqStore(zc, src, srcSize);
    FORWARD_IF_ERROR!(bss);

    cSize = ZSTD_compressBlock_targetCBlockSize_body(zc, dst, dstCapacity, src, srcSize, bss, lastBlock);
    FORWARD_IF_ERROR!(cSize);

    if (*(*zc).blockState.prevCBlock).entropy.fse.offcode_repeatMode == FSE_repeat_valid {
        (*(*zc).blockState.prevCBlock).entropy.fse.offcode_repeatMode = FSE_repeat_check;
    }

    cSize
}

unsafe fn ZSTD_overflowCorrectIfNeeded(
    ms: *mut ZSTD_MatchState_t,
    ws: *mut ZSTD_cwksp,
    params: *const ZSTD_CCtx_params,
    ip: *const c_void,
    iend: *const c_void,
) {
    let cycleLog: U32 = ZSTD_cycleLog((*params).cParams.chainLog, (*params).cParams.strategy);
    let maxDist: U32 = 1u32 << (*params).cParams.windowLog;
    if ZSTD_window_needOverflowCorrection(
        (*ms).window,
        cycleLog,
        maxDist,
        (*ms).loadedDictEnd,
        ip,
        iend,
    ) != 0
    {
        let correction = ZSTD_window_correctOverflow(&mut (*ms).window, cycleLog, maxDist, ip);
        ZSTD_cwksp_mark_tables_dirty(ws);
        ZSTD_reduceIndex(ms, params, correction);
        ZSTD_cwksp_mark_tables_clean(ws);
        if (*ms).nextToUpdate < correction {
            (*ms).nextToUpdate = 0;
        } else {
            (*ms).nextToUpdate -= correction;
        }
        (*ms).loadedDictEnd = 0;
        (*ms).dictMatchState = core::ptr::null();
    }
}

unsafe fn ZSTD_optimalBlockSize(
    cctx: *mut ZSTD_CCtx,
    src: *const c_void,
    srcSize: usize,
    blockSizeMax: usize,
    mut splitLevel: c_int,
    strat: ZSTD_strategy,
    savings: S64,
) -> usize {
    static splitLevels: [c_int; 10] = [0, 0, 1, 2, 2, 3, 3, 4, 4, 4];
    if srcSize < 128 * 1024 || blockSizeMax < 128 * 1024 {
        return core::cmp::min(srcSize, blockSizeMax);
    }
    if savings < 3 {
        return 128 * 1024;
    }
    if splitLevel == 1 {
        return 128 * 1024;
    }
    if splitLevel == 0 {
        debug_assert!(ZSTD_fast <= strat && strat <= ZSTD_btultra2);
        splitLevel = splitLevels[strat as usize];
    } else {
        debug_assert!(2 <= splitLevel && splitLevel <= 6);
        splitLevel -= 2;
    }
    ZSTD_splitBlock(
        src,
        blockSizeMax,
        splitLevel,
        (*cctx).tmpWorkspace,
        (*cctx).tmpWkspSize,
    )
}

unsafe fn ZSTD_compress_frameChunk(
    cctx: *mut ZSTD_CCtx,
    dst: *mut c_void,
    mut dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
    lastFrameChunk: U32,
) -> usize {
    let blockSizeMax: usize = (*cctx).blockSizeMax;
    let mut remaining: usize = srcSize;
    let mut ip: *const u8 = src as *const u8;
    let ostart: *mut u8 = dst as *mut u8;
    let mut op: *mut u8 = ostart;
    let maxDist: U32 = 1u32 << (*cctx).appliedParams.cParams.windowLog;
    let mut savings: S64 = (*cctx).consumedSrcSize as S64 - (*cctx).producedCSize as S64;

    if (*cctx).appliedParams.fParams.checksumFlag != 0 && srcSize != 0 {
        ZSTD_XXH64_update(&mut (*cctx).xxhState, src, srcSize);
    }

    while remaining != 0 {
        let ms: *mut ZSTD_MatchState_t = &mut (*cctx).blockState.matchState;
        let blockSize: usize = ZSTD_optimalBlockSize(
            cctx,
            ip as *const c_void,
            remaining,
            blockSizeMax,
            (*cctx).appliedParams.preBlockSplitter_level,
            (*cctx).appliedParams.cParams.strategy,
            savings,
        );
        let lastBlock: U32 = lastFrameChunk & (blockSize == remaining) as U32;
        debug_assert!(blockSize <= remaining);

        RETURN_ERROR_IF!(
            dstCapacity < ZSTD_blockHeaderSize + MIN_CBLOCK_SIZE + 1,
            DSTSIZE_TOOSMALL
        );

        ZSTD_overflowCorrectIfNeeded(
            ms,
            &mut (*cctx).workspace,
            &(*cctx).appliedParams,
            ip as *const c_void,
            ip.add(blockSize) as *const c_void,
        );
        ZSTD_checkDictValidity(
            &(*ms).window,
            ip.add(blockSize) as *const c_void,
            maxDist,
            &mut (*ms).loadedDictEnd,
            &mut (*ms).dictMatchState,
        );
        ZSTD_window_enforceMaxDist(
            &mut (*ms).window,
            ip as *const c_void,
            maxDist,
            &mut (*ms).loadedDictEnd,
            &mut (*ms).dictMatchState,
        );

        if (*ms).nextToUpdate < (*ms).window.lowLimit {
            (*ms).nextToUpdate = (*ms).window.lowLimit;
        }

        {
            let mut cSize: usize;
            if ZSTD_useTargetCBlockSize(&(*cctx).appliedParams) != 0 {
                cSize = ZSTD_compressBlock_targetCBlockSize(
                    cctx,
                    op as *mut c_void,
                    dstCapacity,
                    ip as *const c_void,
                    blockSize,
                    lastBlock,
                );
                FORWARD_IF_ERROR!(cSize);
                debug_assert!(cSize > 0);
                debug_assert!(cSize <= blockSize + ZSTD_blockHeaderSize);
            } else if ZSTD_blockSplitterEnabled(&mut (*cctx).appliedParams) != 0 {
                cSize = ZSTD_compressBlock_splitBlock(
                    cctx,
                    op as *mut c_void,
                    dstCapacity,
                    ip as *const c_void,
                    blockSize,
                    lastBlock,
                );
                FORWARD_IF_ERROR!(cSize);
            } else {
                cSize = ZSTD_compressBlock_internal(
                    cctx,
                    op.add(ZSTD_blockHeaderSize) as *mut c_void,
                    dstCapacity - ZSTD_blockHeaderSize,
                    ip as *const c_void,
                    blockSize,
                    1,
                );
                FORWARD_IF_ERROR!(cSize);

                if cSize == 0 {
                    cSize = ZSTD_noCompressBlock(
                        op as *mut c_void,
                        dstCapacity,
                        ip as *const c_void,
                        blockSize,
                        lastBlock,
                    );
                    FORWARD_IF_ERROR!(cSize);
                } else {
                    let cBlockHeader: U32 = if cSize == 1 {
                        lastBlock + ((bt_rle) << 1) + ((blockSize << 3) as U32)
                    } else {
                        lastBlock + ((bt_compressed) << 1) + ((cSize << 3) as U32)
                    };
                    MEM_writeLE24(op as *mut c_void, cBlockHeader);
                    cSize += ZSTD_blockHeaderSize;
                }
            }

            savings += blockSize as S64 - cSize as S64;

            ip = ip.add(blockSize);
            debug_assert!(remaining >= blockSize);
            remaining -= blockSize;
            op = op.add(cSize);
            debug_assert!(dstCapacity >= cSize);
            dstCapacity -= cSize;
            (*cctx).isFirstBlock = 0;
        }
    }

    if lastFrameChunk != 0 && (op as usize > ostart as usize) {
        (*cctx).stage = ZSTDcs_ending;
    }
    op as usize - ostart as usize
}

unsafe fn ZSTD_writeFrameHeader(
    dst: *mut c_void,
    dstCapacity: usize,
    params: *const ZSTD_CCtx_params,
    pledgedSrcSize: U64,
    dictID: U32,
) -> usize {
    let op: *mut u8 = dst as *mut u8;
    let dictIDSizeCodeLength: U32 =
        (dictID > 0) as U32 + (dictID >= 256) as U32 + (dictID >= 65536) as U32;
    let dictIDSizeCode: U32 = if (*params).fParams.noDictIDFlag != 0 {
        0
    } else {
        dictIDSizeCodeLength
    };
    let checksumFlag: U32 = ((*params).fParams.checksumFlag > 0) as U32;
    let windowSize: U32 = 1u32 << (*params).cParams.windowLog;
    let singleSegment: U32 =
        ((*params).fParams.contentSizeFlag != 0 && (windowSize as U64 >= pledgedSrcSize)) as U32;
    let windowLogByte: u8 =
        (((*params).cParams.windowLog - ZSTD_WINDOWLOG_ABSOLUTEMIN) << 3) as u8;
    let fcsCode: U32 = if (*params).fParams.contentSizeFlag != 0 {
        (pledgedSrcSize >= 256) as U32
            + (pledgedSrcSize >= 65536 + 256) as U32
            + (pledgedSrcSize >= 0xFFFFFFFFu64) as U32
    } else {
        0
    };
    let frameHeaderDescriptionByte: u8 =
        (dictIDSizeCode + (checksumFlag << 2) + (singleSegment << 5) + (fcsCode << 6)) as u8;
    let mut pos: usize = 0;

    RETURN_ERROR_IF!(dstCapacity < ZSTD_FRAMEHEADERSIZE_MAX, DSTSIZE_TOOSMALL);
    if (*params).format == ZSTD_f_zstd1 {
        MEM_writeLE32(dst, ZSTD_MAGICNUMBER);
        pos = 4;
    }
    *op.add(pos) = frameHeaderDescriptionByte;
    pos += 1;
    if singleSegment == 0 {
        *op.add(pos) = windowLogByte;
        pos += 1;
    }
    match dictIDSizeCode {
        0 => {}
        1 => {
            *op.add(pos) = dictID as u8;
            pos += 1;
        }
        2 => {
            MEM_writeLE16(op.add(pos) as *mut c_void, dictID as U16);
            pos += 2;
        }
        3 => {
            MEM_writeLE32(op.add(pos) as *mut c_void, dictID);
            pos += 4;
        }
        _ => {
            debug_assert!(false);
        }
    }
    match fcsCode {
        0 => {
            if singleSegment != 0 {
                *op.add(pos) = pledgedSrcSize as u8;
                pos += 1;
            }
        }
        1 => {
            MEM_writeLE16(op.add(pos) as *mut c_void, (pledgedSrcSize - 256) as U16);
            pos += 2;
        }
        2 => {
            MEM_writeLE32(op.add(pos) as *mut c_void, pledgedSrcSize as U32);
            pos += 4;
        }
        3 => {
            MEM_writeLE64(op.add(pos) as *mut c_void, pledgedSrcSize as U64);
            pos += 8;
        }
        _ => {
            debug_assert!(false);
        }
    }
    pos
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_writeSkippableFrame(
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
    magicVariant: c_uint,
) -> usize {
    let op: *mut u8 = dst as *mut u8;
    RETURN_ERROR_IF!(dstCapacity < srcSize + ZSTD_SKIPPABLEHEADERSIZE, DSTSIZE_TOOSMALL);
    RETURN_ERROR_IF!(srcSize > 0xFFFFFFFF, SRCSIZE_WRONG);
    RETURN_ERROR_IF!(magicVariant > 15, PARAMETER_OUTOFBOUND);

    MEM_writeLE32(op as *mut c_void, ZSTD_MAGIC_SKIPPABLE_START + magicVariant);
    MEM_writeLE32(op.add(4) as *mut c_void, srcSize as U32);
    memcpy(op.add(8) as *mut c_void, src, srcSize);
    srcSize + ZSTD_SKIPPABLEHEADERSIZE
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_writeLastEmptyBlock(dst: *mut c_void, dstCapacity: usize) -> usize {
    RETURN_ERROR_IF!(dstCapacity < ZSTD_blockHeaderSize, DSTSIZE_TOOSMALL);
    {
        let cBlockHeader24: U32 = 1 + ((bt_raw) << 1);
        MEM_writeLE24(dst, cBlockHeader24);
        ZSTD_blockHeaderSize
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_referenceExternalSequences(
    cctx: *mut ZSTD_CCtx,
    seq: *mut rawSeq,
    nbSeq: usize,
) {
    debug_assert!((*cctx).stage == ZSTDcs_init);
    (*cctx).externSeqStore.seq = seq;
    (*cctx).externSeqStore.size = nbSeq;
    (*cctx).externSeqStore.capacity = nbSeq;
    (*cctx).externSeqStore.pos = 0;
    (*cctx).externSeqStore.posInSequence = 0;
}

unsafe fn ZSTD_compressContinue_internal(
    cctx: *mut ZSTD_CCtx,
    mut dst: *mut c_void,
    mut dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
    frame: U32,
    lastFrameChunk: U32,
) -> usize {
    let ms: *mut ZSTD_MatchState_t = &mut (*cctx).blockState.matchState;
    let mut fhSize: usize = 0;

    RETURN_ERROR_IF!((*cctx).stage == ZSTDcs_created, STAGE_WRONG);

    if frame != 0 && ((*cctx).stage == ZSTDcs_init) {
        fhSize = ZSTD_writeFrameHeader(
            dst,
            dstCapacity,
            &(*cctx).appliedParams,
            (*cctx).pledgedSrcSizePlusOne - 1,
            (*cctx).dictID,
        );
        FORWARD_IF_ERROR!(fhSize);
        debug_assert!(fhSize <= dstCapacity);
        dstCapacity -= fhSize;
        dst = (dst as *mut c_char).add(fhSize) as *mut c_void;
        (*cctx).stage = ZSTDcs_ongoing;
    }

    if srcSize == 0 {
        return fhSize;
    }

    if ZSTD_window_update(&mut (*ms).window, src, srcSize, (*ms).forceNonContiguous) == 0 {
        (*ms).forceNonContiguous = 0;
        (*ms).nextToUpdate = (*ms).window.dictLimit;
    }
    if (*cctx).appliedParams.ldmParams.enableLdm == ZSTD_ps_enable {
        ZSTD_window_update(&mut (*cctx).ldmState.window, src, srcSize, 0);
    }

    if frame == 0 {
        ZSTD_overflowCorrectIfNeeded(
            ms,
            &mut (*cctx).workspace,
            &(*cctx).appliedParams,
            src,
            (src as *const u8).add(srcSize) as *const c_void,
        );
    }

    {
        let cSize = if frame != 0 {
            ZSTD_compress_frameChunk(cctx, dst, dstCapacity, src, srcSize, lastFrameChunk)
        } else {
            ZSTD_compressBlock_internal(cctx, dst, dstCapacity, src, srcSize, 0)
        };
        FORWARD_IF_ERROR!(cSize);
        (*cctx).consumedSrcSize += srcSize as u64;
        (*cctx).producedCSize += (cSize + fhSize) as u64;
        if (*cctx).pledgedSrcSizePlusOne != 0 {
            RETURN_ERROR_IF!(
                (*cctx).consumedSrcSize + 1 > (*cctx).pledgedSrcSizePlusOne,
                SRCSIZE_WRONG
            );
        }
        cSize + fhSize
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressContinue_public(
    cctx: *mut ZSTD_CCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    ZSTD_compressContinue_internal(cctx, dst, dstCapacity, src, srcSize, 1, 0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressContinue(
    cctx: *mut ZSTD_CCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    ZSTD_compressContinue_public(cctx, dst, dstCapacity, src, srcSize)
}

unsafe fn ZSTD_getBlockSize_deprecated(cctx: *const ZSTD_CCtx) -> usize {
    let cParams = (*cctx).appliedParams.cParams;
    debug_assert!(ZSTD_checkCParams(cParams) == 0);
    core::cmp::min((*cctx).appliedParams.maxBlockSize, 1usize << cParams.windowLog)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_getBlockSize(cctx: *const ZSTD_CCtx) -> usize {
    ZSTD_getBlockSize_deprecated(cctx)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_deprecated(
    cctx: *mut ZSTD_CCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    {
        let blockSizeMax = ZSTD_getBlockSize_deprecated(cctx);
        RETURN_ERROR_IF!(srcSize > blockSizeMax, SRCSIZE_WRONG);
    }
    ZSTD_compressContinue_internal(cctx, dst, dstCapacity, src, srcSize, 0, 0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock(
    cctx: *mut ZSTD_CCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    ZSTD_compressBlock_deprecated(cctx, dst, dstCapacity, src, srcSize)
}

unsafe fn ZSTD_loadDictionaryContent(
    ms: *mut ZSTD_MatchState_t,
    ls: *mut ldmState_t,
    ws: *mut ZSTD_cwksp,
    params: *const ZSTD_CCtx_params,
    mut src: *const c_void,
    mut srcSize: usize,
    dtlm: ZSTD_dictTableLoadMethod_e,
    tfp: ZSTD_tableFillPurpose_e,
) -> usize {
    let mut ip: *const u8 = src as *const u8;
    let iend: *const u8 = ip.add(srcSize);
    let loadLdmDict: c_int =
        ((*params).ldmParams.enableLdm == ZSTD_ps_enable && !ls.is_null()) as c_int;

    ZSTD_assertEqualCParams((*params).cParams, (*ms).cParams);

    {
        let mut maxDictSize: U32 = ZSTD_CURRENT_MAX() - ZSTD_WINDOW_START_INDEX;

        let CDictTaggedIndices = ZSTD_CDictIndicesAreTagged(&(*params).cParams);
        if CDictTaggedIndices != 0 && tfp == ZSTD_tfp_forCDict {
            let shortCacheMaxDictSize: U32 =
                (1u32 << (32 - ZSTD_SHORT_CACHE_TAG_BITS)) - ZSTD_WINDOW_START_INDEX;
            maxDictSize = core::cmp::min(maxDictSize, shortCacheMaxDictSize);
            debug_assert!(loadLdmDict == 0);
        }

        if srcSize > maxDictSize as usize {
            ip = iend.sub(maxDictSize as usize);
            src = ip as *const c_void;
            srcSize = maxDictSize as usize;
        }
    }

    if srcSize > ZSTD_CHUNKSIZE_MAX() as usize {
        debug_assert!(ZSTD_window_isEmpty((*ms).window) != 0);
        if loadLdmDict != 0 {
            debug_assert!(ZSTD_window_isEmpty((*ls).window) != 0);
        }
    }
    ZSTD_window_update(&mut (*ms).window, src, srcSize, 0);

    if loadLdmDict != 0 {
        ZSTD_window_update(&mut (*ls).window, src, srcSize, 0);
        (*ls).loadedDictEnd = if (*params).forceWindow != 0 {
            0
        } else {
            (iend as usize - (*ls).window.base as usize) as U32
        };
        ZSTD_ldm_fillHashTable(ls, ip, iend, &(*params).ldmParams);
    }

    {
        let maxDictSize: U32 = 1u32
            << core::cmp::min(
                core::cmp::max(
                    (*params).cParams.hashLog + 3,
                    (*params).cParams.chainLog + 1,
                ),
                31,
            );
        if srcSize > maxDictSize as usize {
            ip = iend.sub(maxDictSize as usize);
            src = ip as *const c_void;
            srcSize = maxDictSize as usize;
        }
    }

    (*ms).nextToUpdate = (ip as usize - (*ms).window.base as usize) as U32;
    (*ms).loadedDictEnd = if (*params).forceWindow != 0 {
        0
    } else {
        (iend as usize - (*ms).window.base as usize) as U32
    };
    (*ms).forceNonContiguous = (*params).deterministicRefPrefix;

    if srcSize <= HASH_READ_SIZE {
        return 0;
    }

    ZSTD_overflowCorrectIfNeeded(ms, ws, params, ip as *const c_void, iend as *const c_void);

    match (*params).cParams.strategy {
        ZSTD_fast => {
            ZSTD_fillHashTable(ms, iend as *const c_void, dtlm, tfp);
        }
        ZSTD_dfast => {
            ZSTD_fillDoubleHashTable(ms, iend as *const c_void, dtlm, tfp);
        }
        ZSTD_greedy | ZSTD_lazy | ZSTD_lazy2 => {
            debug_assert!(srcSize >= HASH_READ_SIZE);
            if (*ms).dedicatedDictSearch != 0 {
                debug_assert!(!(*ms).chainTable.is_null());
                ZSTD_dedicatedDictSearch_lazy_loadDictionary(ms, iend.sub(HASH_READ_SIZE));
            } else {
                debug_assert!((*params).useRowMatchFinder != ZSTD_ps_auto);
                if (*params).useRowMatchFinder == ZSTD_ps_enable {
                    let tagTableSize: usize = 1usize << (*params).cParams.hashLog;
                    memset((*ms).tagTable as *mut c_void, 0, tagTableSize);
                    ZSTD_row_update(ms, iend.sub(HASH_READ_SIZE));
                } else {
                    ZSTD_insertAndFindFirstIndex(ms, iend.sub(HASH_READ_SIZE));
                }
            }
        }
        ZSTD_btlazy2 | ZSTD_btopt | ZSTD_btultra | ZSTD_btultra2 => {
            debug_assert!(srcSize >= HASH_READ_SIZE);
            ZSTD_updateTree(ms, iend.sub(HASH_READ_SIZE), iend);
        }
        _ => {
            debug_assert!(false);
        }
    }

    (*ms).nextToUpdate = (iend as usize - (*ms).window.base as usize) as U32;
    0
}

unsafe fn ZSTD_dictNCountRepeat(
    normalizedCounter: *mut i16,
    dictMaxSymbolValue: c_uint,
    maxSymbolValue: c_uint,
) -> FSE_repeat {
    let mut s;
    if dictMaxSymbolValue < maxSymbolValue {
        return FSE_repeat_check;
    }
    s = 0;
    while s <= maxSymbolValue {
        if *normalizedCounter.add(s as usize) == 0 {
            return FSE_repeat_check;
        }
        s += 1;
    }
    FSE_repeat_valid
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_loadCEntropy(
    bs: *mut ZSTD_compressedBlockState_t,
    workspace: *mut c_void,
    dict: *const c_void,
    dictSize: usize,
) -> usize {
    let mut offcodeNCount = [0i16; MaxOff as usize + 1];
    let mut offcodeMaxValue: c_uint = MaxOff;
    let mut dictPtr: *const u8 = dict as *const u8;
    let dictEnd: *const u8 = dictPtr.add(dictSize);
    dictPtr = dictPtr.add(8);
    (*bs).entropy.huf.repeatMode = HUF_repeat_check;

    {
        let mut maxSymbolValue: c_uint = 255;
        let mut hasZeroWeights: c_uint = 1;
        let hufHeaderSize = HUF_readCTable(
            (*bs).entropy.huf.CTable.as_mut_ptr() as *mut HUF_CElt,
            &mut maxSymbolValue,
            dictPtr as *const c_void,
            (dictEnd as usize - dictPtr as usize),
            &mut hasZeroWeights,
        );

        if hasZeroWeights == 0 && maxSymbolValue == 255 {
            (*bs).entropy.huf.repeatMode = HUF_repeat_valid;
        }

        RETURN_ERROR_IF!(HUF_isError(hufHeaderSize) != 0, DICTIONARY_CORRUPTED);
        dictPtr = dictPtr.add(hufHeaderSize);
    }

    {
        let mut offcodeLog: c_uint = 0;
        let offcodeHeaderSize = FSE_readNCount(
            offcodeNCount.as_mut_ptr(),
            &mut offcodeMaxValue,
            &mut offcodeLog,
            dictPtr as *const c_void,
            (dictEnd as usize - dictPtr as usize),
        );
        RETURN_ERROR_IF!(FSE_isError(offcodeHeaderSize) != 0, DICTIONARY_CORRUPTED);
        RETURN_ERROR_IF!(offcodeLog > OffFSELog, DICTIONARY_CORRUPTED);
        RETURN_ERROR_IF!(
            FSE_isError(FSE_buildCTable_wksp(
                (*bs).entropy.fse.offcodeCTable.as_mut_ptr(),
                offcodeNCount.as_ptr(),
                MaxOff,
                offcodeLog,
                workspace,
                HUF_WORKSPACE_SIZE,
            )) != 0,
            DICTIONARY_CORRUPTED
        );
        dictPtr = dictPtr.add(offcodeHeaderSize);
    }

    {
        let mut matchlengthNCount = [0i16; MaxML as usize + 1];
        let mut matchlengthMaxValue: c_uint = MaxML;
        let mut matchlengthLog: c_uint = 0;
        let matchlengthHeaderSize = FSE_readNCount(
            matchlengthNCount.as_mut_ptr(),
            &mut matchlengthMaxValue,
            &mut matchlengthLog,
            dictPtr as *const c_void,
            (dictEnd as usize - dictPtr as usize),
        );
        RETURN_ERROR_IF!(FSE_isError(matchlengthHeaderSize) != 0, DICTIONARY_CORRUPTED);
        RETURN_ERROR_IF!(matchlengthLog > MLFSELog, DICTIONARY_CORRUPTED);
        RETURN_ERROR_IF!(
            FSE_isError(FSE_buildCTable_wksp(
                (*bs).entropy.fse.matchlengthCTable.as_mut_ptr(),
                matchlengthNCount.as_ptr(),
                matchlengthMaxValue,
                matchlengthLog,
                workspace,
                HUF_WORKSPACE_SIZE,
            )) != 0,
            DICTIONARY_CORRUPTED
        );
        (*bs).entropy.fse.matchlength_repeatMode =
            ZSTD_dictNCountRepeat(matchlengthNCount.as_mut_ptr(), matchlengthMaxValue, MaxML);
        dictPtr = dictPtr.add(matchlengthHeaderSize);
    }

    {
        let mut litlengthNCount = [0i16; MaxLL as usize + 1];
        let mut litlengthMaxValue: c_uint = MaxLL;
        let mut litlengthLog: c_uint = 0;
        let litlengthHeaderSize = FSE_readNCount(
            litlengthNCount.as_mut_ptr(),
            &mut litlengthMaxValue,
            &mut litlengthLog,
            dictPtr as *const c_void,
            (dictEnd as usize - dictPtr as usize),
        );
        RETURN_ERROR_IF!(FSE_isError(litlengthHeaderSize) != 0, DICTIONARY_CORRUPTED);
        RETURN_ERROR_IF!(litlengthLog > LLFSELog, DICTIONARY_CORRUPTED);
        RETURN_ERROR_IF!(
            FSE_isError(FSE_buildCTable_wksp(
                (*bs).entropy.fse.litlengthCTable.as_mut_ptr(),
                litlengthNCount.as_ptr(),
                litlengthMaxValue,
                litlengthLog,
                workspace,
                HUF_WORKSPACE_SIZE,
            )) != 0,
            DICTIONARY_CORRUPTED
        );
        (*bs).entropy.fse.litlength_repeatMode =
            ZSTD_dictNCountRepeat(litlengthNCount.as_mut_ptr(), litlengthMaxValue, MaxLL);
        dictPtr = dictPtr.add(litlengthHeaderSize);
    }

    RETURN_ERROR_IF!(dictPtr.add(12) > dictEnd, DICTIONARY_CORRUPTED);
    (*bs).rep[0] = MEM_readLE32(dictPtr.add(0) as *const c_void);
    (*bs).rep[1] = MEM_readLE32(dictPtr.add(4) as *const c_void);
    (*bs).rep[2] = MEM_readLE32(dictPtr.add(8) as *const c_void);
    dictPtr = dictPtr.add(12);

    {
        let dictContentSize: usize = dictEnd as usize - dictPtr as usize;
        let mut offcodeMax: U32 = MaxOff;
        if dictContentSize <= (u32::MAX as usize) - (128 * 1024) {
            let maxOffset: U32 = dictContentSize as U32 + (128 * 1024);
            offcodeMax = ZSTD_highbit32(maxOffset);
        }
        (*bs).entropy.fse.offcode_repeatMode = ZSTD_dictNCountRepeat(
            offcodeNCount.as_mut_ptr(),
            offcodeMaxValue,
            core::cmp::min(offcodeMax, MaxOff),
        );

        {
            let mut u = 0;
            while u < 3 {
                RETURN_ERROR_IF!((*bs).rep[u] == 0, DICTIONARY_CORRUPTED);
                RETURN_ERROR_IF!((*bs).rep[u] as usize > dictContentSize, DICTIONARY_CORRUPTED);
                u += 1;
            }
        }
    }

    dictPtr as usize - dict as usize
}

unsafe fn ZSTD_loadZstdDictionary(
    bs: *mut ZSTD_compressedBlockState_t,
    ms: *mut ZSTD_MatchState_t,
    ws: *mut ZSTD_cwksp,
    params: *const ZSTD_CCtx_params,
    dict: *const c_void,
    dictSize: usize,
    dtlm: ZSTD_dictTableLoadMethod_e,
    tfp: ZSTD_tableFillPurpose_e,
    workspace: *mut c_void,
) -> usize {
    let mut dictPtr: *const u8 = dict as *const u8;
    let dictEnd: *const u8 = dictPtr.add(dictSize);
    let dictID: usize;
    let eSize: usize;
    debug_assert!(dictSize >= 8);
    debug_assert!(MEM_readLE32(dictPtr as *const c_void) == ZSTD_MAGIC_DICTIONARY);

    dictID = if (*params).fParams.noDictIDFlag != 0 {
        0
    } else {
        MEM_readLE32(dictPtr.add(4) as *const c_void) as usize
    };
    eSize = ZSTD_loadCEntropy(bs, workspace, dict, dictSize);
    FORWARD_IF_ERROR!(eSize);
    dictPtr = dictPtr.add(eSize);

    {
        let dictContentSize: usize = dictEnd as usize - dictPtr as usize;
        FORWARD_IF_ERROR!(ZSTD_loadDictionaryContent(
            ms,
            core::ptr::null_mut(),
            ws,
            params,
            dictPtr as *const c_void,
            dictContentSize,
            dtlm,
            tfp,
        ));
    }
    dictID
}

unsafe fn ZSTD_compress_insertDictionary(
    bs: *mut ZSTD_compressedBlockState_t,
    ms: *mut ZSTD_MatchState_t,
    ls: *mut ldmState_t,
    ws: *mut ZSTD_cwksp,
    params: *const ZSTD_CCtx_params,
    dict: *const c_void,
    dictSize: usize,
    dictContentType: ZSTD_dictContentType_e,
    dtlm: ZSTD_dictTableLoadMethod_e,
    tfp: ZSTD_tableFillPurpose_e,
    workspace: *mut c_void,
) -> usize {
    if dict.is_null() || dictSize < 8 {
        RETURN_ERROR_IF!(dictContentType == ZSTD_dct_fullDict, DICTIONARY_WRONG);
        return 0;
    }

    ZSTD_reset_compressedBlockState(bs);

    if dictContentType == ZSTD_dct_rawContent {
        return ZSTD_loadDictionaryContent(ms, ls, ws, params, dict, dictSize, dtlm, tfp);
    }

    if MEM_readLE32(dict) != ZSTD_MAGIC_DICTIONARY {
        if dictContentType == ZSTD_dct_auto {
            return ZSTD_loadDictionaryContent(ms, ls, ws, params, dict, dictSize, dtlm, tfp);
        }
        RETURN_ERROR_IF!(dictContentType == ZSTD_dct_fullDict, DICTIONARY_WRONG);
        debug_assert!(false);
    }

    ZSTD_loadZstdDictionary(bs, ms, ws, params, dict, dictSize, dtlm, tfp, workspace)
}

const ZSTD_USE_CDICT_PARAMS_SRCSIZE_CUTOFF: U64 = (128 * 1024);
const ZSTD_USE_CDICT_PARAMS_DICTSIZE_MULTIPLIER: U64 = 6;

unsafe fn ZSTD_compressBegin_internal(
    cctx: *mut ZSTD_CCtx,
    dict: *const c_void,
    dictSize: usize,
    dictContentType: ZSTD_dictContentType_e,
    dtlm: ZSTD_dictTableLoadMethod_e,
    cdict: *const ZSTD_CDict,
    params: *const ZSTD_CCtx_params,
    pledgedSrcSize: U64,
    zbuff: ZSTD_buffered_policy_e,
) -> usize {
    let cdictReal = if cdict.is_null() {
        core::ptr::null()
    } else {
        as_cdict(cdict)
    };
    let dictContentSize: usize = if !cdict.is_null() {
        (*cdictReal).dictContentSize
    } else {
        dictSize
    };
    (*cctx).traceCtx = 0; /* ZSTD_TRACE=1 but trace hooks are no-ops */
    debug_assert!(ZSTD_isError(ZSTD_checkCParams((*params).cParams)) == 0);
    if !cdict.is_null()
        && (*cdictReal).dictContentSize > 0
        && (pledgedSrcSize < ZSTD_USE_CDICT_PARAMS_SRCSIZE_CUTOFF
            || pledgedSrcSize
                < (*cdictReal).dictContentSize as U64 * ZSTD_USE_CDICT_PARAMS_DICTSIZE_MULTIPLIER
            || pledgedSrcSize == ZSTD_CONTENTSIZE_UNKNOWN
            || (*cdictReal).compressionLevel == 0)
        && ((*params).attachDictPref != ZSTD_dictForceLoad)
    {
        return ZSTD_resetCCtx_usingCDict(cctx, cdict, params, pledgedSrcSize, zbuff);
    }

    FORWARD_IF_ERROR!(ZSTD_resetCCtx_internal(
        cctx,
        params,
        pledgedSrcSize,
        dictContentSize,
        ZSTDcrp_makeClean,
        zbuff,
    ));
    {
        let dictID: usize = if !cdict.is_null() {
            ZSTD_compress_insertDictionary(
                (*cctx).blockState.prevCBlock,
                &mut (*cctx).blockState.matchState,
                &mut (*cctx).ldmState,
                &mut (*cctx).workspace,
                &(*cctx).appliedParams,
                (*cdictReal).dictContent,
                (*cdictReal).dictContentSize,
                (*cdictReal).dictContentType,
                dtlm,
                ZSTD_tfp_forCCtx,
                (*cctx).tmpWorkspace,
            )
        } else {
            ZSTD_compress_insertDictionary(
                (*cctx).blockState.prevCBlock,
                &mut (*cctx).blockState.matchState,
                &mut (*cctx).ldmState,
                &mut (*cctx).workspace,
                &(*cctx).appliedParams,
                dict,
                dictSize,
                dictContentType,
                dtlm,
                ZSTD_tfp_forCCtx,
                (*cctx).tmpWorkspace,
            )
        };
        FORWARD_IF_ERROR!(dictID);
        (*cctx).dictID = dictID as U32;
        (*cctx).dictContentSize = dictContentSize;
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBegin_advanced_internal(
    cctx: *mut ZSTD_CCtx,
    dict: *const c_void,
    dictSize: usize,
    dictContentType: ZSTD_dictContentType_e,
    dtlm: ZSTD_dictTableLoadMethod_e,
    cdict: *const ZSTD_CDict,
    params: *const ZSTD_CCtx_params,
    pledgedSrcSize: c_ulonglong,
) -> usize {
    FORWARD_IF_ERROR!(ZSTD_checkCParams((*params).cParams));
    ZSTD_compressBegin_internal(
        cctx,
        dict,
        dictSize,
        dictContentType,
        dtlm,
        cdict,
        params,
        pledgedSrcSize,
        ZSTDb_not_buffered,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBegin_advanced(
    cctx: *mut ZSTD_CCtx,
    dict: *const c_void,
    dictSize: usize,
    params: ZSTD_parameters,
    pledgedSrcSize: c_ulonglong,
) -> usize {
    let mut cctxParams: ZSTD_CCtx_params = core::mem::zeroed();
    ZSTD_CCtxParams_init_internal(&mut cctxParams, &params, ZSTD_NO_CLEVEL);
    ZSTD_compressBegin_advanced_internal(
        cctx,
        dict,
        dictSize,
        ZSTD_dct_auto,
        ZSTD_dtlm_fast,
        core::ptr::null(),
        &cctxParams,
        pledgedSrcSize,
    )
}

unsafe fn ZSTD_compressBegin_usingDict_deprecated(
    cctx: *mut ZSTD_CCtx,
    dict: *const c_void,
    dictSize: usize,
    compressionLevel: c_int,
) -> usize {
    let mut cctxParams: ZSTD_CCtx_params = core::mem::zeroed();
    {
        let params = ZSTD_getParams_internal(
            compressionLevel,
            ZSTD_CONTENTSIZE_UNKNOWN,
            dictSize,
            ZSTD_cpm_noAttachDict,
        );
        ZSTD_CCtxParams_init_internal(
            &mut cctxParams,
            &params,
            if compressionLevel == 0 {
                ZSTD_CLEVEL_DEFAULT
            } else {
                compressionLevel
            },
        );
    }
    ZSTD_compressBegin_internal(
        cctx,
        dict,
        dictSize,
        ZSTD_dct_auto,
        ZSTD_dtlm_fast,
        core::ptr::null(),
        &cctxParams,
        ZSTD_CONTENTSIZE_UNKNOWN,
        ZSTDb_not_buffered,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBegin_usingDict(
    cctx: *mut ZSTD_CCtx,
    dict: *const c_void,
    dictSize: usize,
    compressionLevel: c_int,
) -> usize {
    ZSTD_compressBegin_usingDict_deprecated(cctx, dict, dictSize, compressionLevel)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBegin(cctx: *mut ZSTD_CCtx, compressionLevel: c_int) -> usize {
    ZSTD_compressBegin_usingDict_deprecated(cctx, core::ptr::null(), 0, compressionLevel)
}

unsafe fn ZSTD_writeEpilogue(cctx: *mut ZSTD_CCtx, dst: *mut c_void, mut dstCapacity: usize) -> usize {
    let ostart: *mut u8 = dst as *mut u8;
    let mut op: *mut u8 = ostart;

    RETURN_ERROR_IF!((*cctx).stage == ZSTDcs_created, STAGE_WRONG);

    if (*cctx).stage == ZSTDcs_init {
        let fhSize = ZSTD_writeFrameHeader(dst, dstCapacity, &(*cctx).appliedParams, 0, 0);
        FORWARD_IF_ERROR!(fhSize);
        dstCapacity -= fhSize;
        op = op.add(fhSize);
        (*cctx).stage = ZSTDcs_ongoing;
    }

    if (*cctx).stage != ZSTDcs_ending {
        let cBlockHeader24: U32 = 1 + ((bt_raw) << 1) + 0;
        RETURN_ERROR_IF!(dstCapacity < 3, DSTSIZE_TOOSMALL);
        MEM_writeLE24(op as *mut c_void, cBlockHeader24);
        op = op.add(ZSTD_blockHeaderSize);
        dstCapacity -= ZSTD_blockHeaderSize;
    }

    if (*cctx).appliedParams.fParams.checksumFlag != 0 {
        let checksum: U32 = ZSTD_XXH64_digest(&(*cctx).xxhState) as U32;
        RETURN_ERROR_IF!(dstCapacity < 4, DSTSIZE_TOOSMALL);
        MEM_writeLE32(op as *mut c_void, checksum);
        op = op.add(4);
    }

    (*cctx).stage = ZSTDcs_created;
    op as usize - ostart as usize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_CCtx_trace(cctx: *mut ZSTD_CCtx, extraCSize: usize) {
    /* ZSTD_TRACE=1 but trace hooks (ZSTD_trace_compress_end) are undefined/no-op. */
    let _ = extraCSize;
    (*cctx).traceCtx = 0;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressEnd_public(
    cctx: *mut ZSTD_CCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let endResult: usize;
    let cSize = ZSTD_compressContinue_internal(cctx, dst, dstCapacity, src, srcSize, 1, 1);
    FORWARD_IF_ERROR!(cSize);
    endResult = ZSTD_writeEpilogue(
        cctx,
        (dst as *mut c_char).add(cSize) as *mut c_void,
        dstCapacity - cSize,
    );
    FORWARD_IF_ERROR!(endResult);
    if (*cctx).pledgedSrcSizePlusOne != 0 {
        RETURN_ERROR_IF!(
            (*cctx).pledgedSrcSizePlusOne != (*cctx).consumedSrcSize + 1,
            SRCSIZE_WRONG
        );
    }
    ZSTD_CCtx_trace(cctx, endResult);
    cSize + endResult
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressEnd(
    cctx: *mut ZSTD_CCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    ZSTD_compressEnd_public(cctx, dst, dstCapacity, src, srcSize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compress_advanced(
    cctx: *mut ZSTD_CCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
    dict: *const c_void,
    dictSize: usize,
    params: ZSTD_parameters,
) -> usize {
    FORWARD_IF_ERROR!(ZSTD_checkCParams(params.cParams));
    ZSTD_CCtxParams_init_internal(&mut (*cctx).simpleApiParams, &params, ZSTD_NO_CLEVEL);
    ZSTD_compress_advanced_internal(
        cctx,
        dst,
        dstCapacity,
        src,
        srcSize,
        dict,
        dictSize,
        &(*cctx).simpleApiParams,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compress_advanced_internal(
    cctx: *mut ZSTD_CCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
    dict: *const c_void,
    dictSize: usize,
    params: *const ZSTD_CCtx_params,
) -> usize {
    FORWARD_IF_ERROR!(ZSTD_compressBegin_internal(
        cctx,
        dict,
        dictSize,
        ZSTD_dct_auto,
        ZSTD_dtlm_fast,
        core::ptr::null(),
        params,
        srcSize as U64,
        ZSTDb_not_buffered,
    ));
    ZSTD_compressEnd_public(cctx, dst, dstCapacity, src, srcSize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compress_usingDict(
    cctx: *mut ZSTD_CCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
    dict: *const c_void,
    dictSize: usize,
    compressionLevel: c_int,
) -> usize {
    {
        let params = ZSTD_getParams_internal(
            compressionLevel,
            srcSize as U64,
            if !dict.is_null() { dictSize } else { 0 },
            ZSTD_cpm_noAttachDict,
        );
        debug_assert!(params.fParams.contentSizeFlag == 1);
        ZSTD_CCtxParams_init_internal(
            &mut (*cctx).simpleApiParams,
            &params,
            if compressionLevel == 0 {
                ZSTD_CLEVEL_DEFAULT
            } else {
                compressionLevel
            },
        );
    }
    ZSTD_compress_advanced_internal(
        cctx,
        dst,
        dstCapacity,
        src,
        srcSize,
        dict,
        dictSize,
        &(*cctx).simpleApiParams,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressCCtx(
    cctx: *mut ZSTD_CCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
    compressionLevel: c_int,
) -> usize {
    debug_assert!(!cctx.is_null());
    ZSTD_compress_usingDict(
        cctx,
        dst,
        dstCapacity,
        src,
        srcSize,
        core::ptr::null(),
        0,
        compressionLevel,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compress(
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
    compressionLevel: c_int,
) -> usize {
    let result: usize;
    /* ZSTD_COMPRESS_HEAPMODE == 0 : stack context */
    let mut ctxBody: ZSTD_CCtx = core::mem::zeroed();
    ZSTD_initCCtx(&mut ctxBody, ZSTD_defaultCMem);
    result = ZSTD_compressCCtx(&mut ctxBody, dst, dstCapacity, src, srcSize, compressionLevel);
    ZSTD_freeCCtxContent(&mut ctxBody);
    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_estimateCDictSize_advanced(
    dictSize: usize,
    cParams: ZSTD_compressionParameters,
    dictLoadMethod: ZSTD_dictLoadMethod_e,
) -> usize {
    ZSTD_cwksp_alloc_size(core::mem::size_of::<CDictReal>())
        + ZSTD_cwksp_alloc_size(HUF_WORKSPACE_SIZE)
        + ZSTD_sizeof_matchState(
            &cParams,
            ZSTD_resolveRowMatchFinderMode(ZSTD_ps_auto, &cParams),
            1,
            0,
        )
        + (if dictLoadMethod == ZSTD_dlm_byRef {
            0
        } else {
            ZSTD_cwksp_alloc_size(ZSTD_cwksp_align(dictSize, core::mem::size_of::<*mut c_void>()))
        })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_estimateCDictSize(dictSize: usize, compressionLevel: c_int) -> usize {
    let cParams = ZSTD_getCParams_internal(
        compressionLevel,
        ZSTD_CONTENTSIZE_UNKNOWN,
        dictSize,
        ZSTD_cpm_createCDict,
    );
    ZSTD_estimateCDictSize_advanced(dictSize, cParams, ZSTD_dlm_byCopy)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_sizeof_CDict(cdict: *const ZSTD_CDict) -> usize {
    if cdict.is_null() {
        return 0;
    }
    let cdict = as_cdict(cdict);
    (if (*cdict).workspace.workspace == cdict as *mut c_void {
        0
    } else {
        core::mem::size_of::<CDictReal>()
    }) + ZSTD_cwksp_sizeof(&(*cdict).workspace)
}

unsafe fn ZSTD_initCDict_internal(
    cdict: *mut CDictReal,
    dictBuffer: *const c_void,
    dictSize: usize,
    dictLoadMethod: ZSTD_dictLoadMethod_e,
    dictContentType: ZSTD_dictContentType_e,
    mut params: ZSTD_CCtx_params,
) -> usize {
    debug_assert!(ZSTD_checkCParams(params.cParams) == 0);
    (*cdict).matchState.cParams = params.cParams;
    (*cdict).matchState.dedicatedDictSearch = params.enableDedicatedDictSearch;
    if (dictLoadMethod == ZSTD_dlm_byRef) || dictBuffer.is_null() || dictSize == 0 {
        (*cdict).dictContent = dictBuffer;
    } else {
        let internalBuffer = ZSTD_cwksp_reserve_object(
            &mut (*cdict).workspace,
            ZSTD_cwksp_align(dictSize, core::mem::size_of::<*mut c_void>()),
        );
        RETURN_ERROR_IF!(internalBuffer.is_null(), MEMORY_ALLOCATION);
        (*cdict).dictContent = internalBuffer;
        memcpy(internalBuffer, dictBuffer, dictSize);
    }
    (*cdict).dictContentSize = dictSize;
    (*cdict).dictContentType = dictContentType;

    (*cdict).entropyWorkspace =
        ZSTD_cwksp_reserve_object(&mut (*cdict).workspace, HUF_WORKSPACE_SIZE) as *mut U32;

    ZSTD_reset_compressedBlockState(&mut (*cdict).cBlockState);
    FORWARD_IF_ERROR!(ZSTD_reset_matchState(
        &mut (*cdict).matchState,
        &mut (*cdict).workspace,
        &params.cParams,
        params.useRowMatchFinder,
        ZSTDcrp_makeClean,
        ZSTDirp_reset,
        ZSTD_resetTarget_CDict,
    ));
    {
        params.compressionLevel = ZSTD_CLEVEL_DEFAULT;
        params.fParams.contentSizeFlag = 1;
        {
            let dictID = ZSTD_compress_insertDictionary(
                &mut (*cdict).cBlockState,
                &mut (*cdict).matchState,
                core::ptr::null_mut(),
                &mut (*cdict).workspace,
                &params,
                (*cdict).dictContent,
                (*cdict).dictContentSize,
                dictContentType,
                ZSTD_dtlm_full,
                ZSTD_tfp_forCDict,
                (*cdict).entropyWorkspace as *mut c_void,
            );
            FORWARD_IF_ERROR!(dictID);
            (*cdict).dictID = dictID as U32;
        }
    }

    0
}

unsafe fn ZSTD_createCDict_advanced_internal(
    dictSize: usize,
    dictLoadMethod: ZSTD_dictLoadMethod_e,
    cParams: ZSTD_compressionParameters,
    useRowMatchFinder: ZSTD_ParamSwitch_e,
    enableDedicatedDictSearch: c_int,
    customMem: ZSTD_customMem,
) -> *mut ZSTD_CDict {
    if (customMem.customAlloc.is_none()) ^ (customMem.customFree.is_none()) {
        return core::ptr::null_mut();
    }

    {
        let workspaceSize: usize = ZSTD_cwksp_alloc_size(core::mem::size_of::<CDictReal>())
            + ZSTD_cwksp_alloc_size(HUF_WORKSPACE_SIZE)
            + ZSTD_sizeof_matchState(&cParams, useRowMatchFinder, enableDedicatedDictSearch, 0)
            + (if dictLoadMethod == ZSTD_dlm_byRef {
                0
            } else {
                ZSTD_cwksp_alloc_size(ZSTD_cwksp_align(
                    dictSize,
                    core::mem::size_of::<*mut c_void>(),
                ))
            });
        let workspace = zstd_custom_malloc(workspaceSize, customMem);
        let mut ws: ZSTD_cwksp = core::mem::zeroed();
        let cdict: *mut CDictReal;

        if workspace.is_null() {
            zstd_custom_free(workspace, customMem);
            return core::ptr::null_mut();
        }

        ZSTD_cwksp_init(&mut ws, workspace, workspaceSize, ZSTD_cwksp_dynamic_alloc);

        cdict = ZSTD_cwksp_reserve_object(&mut ws, core::mem::size_of::<CDictReal>())
            as *mut CDictReal;
        debug_assert!(!cdict.is_null());
        ZSTD_cwksp_move(&mut (*cdict).workspace, &mut ws);
        (*cdict).customMem = customMem;
        (*cdict).compressionLevel = ZSTD_NO_CLEVEL;
        (*cdict).useRowMatchFinder = useRowMatchFinder;
        cdict as *mut ZSTD_CDict
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_createCDict_advanced(
    dictBuffer: *const c_void,
    dictSize: usize,
    dictLoadMethod: ZSTD_dictLoadMethod_e,
    dictContentType: ZSTD_dictContentType_e,
    cParams: ZSTD_compressionParameters,
    customMem: ZSTD_customMem,
) -> *mut ZSTD_CDict {
    let mut cctxParams: ZSTD_CCtx_params = core::mem::zeroed();
    ZSTD_CCtxParams_init(&mut cctxParams, 0);
    cctxParams.cParams = cParams;
    cctxParams.customMem = customMem;
    ZSTD_createCDict_advanced2(
        dictBuffer,
        dictSize,
        dictLoadMethod,
        dictContentType,
        &cctxParams,
        customMem,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_createCDict_advanced2(
    dict: *const c_void,
    dictSize: usize,
    dictLoadMethod: ZSTD_dictLoadMethod_e,
    dictContentType: ZSTD_dictContentType_e,
    originalCctxParams: *const ZSTD_CCtx_params,
    customMem: ZSTD_customMem,
) -> *mut ZSTD_CDict {
    let mut cctxParams: ZSTD_CCtx_params = core::ptr::read(originalCctxParams);
    let mut cParams: ZSTD_compressionParameters;
    let cdict: *mut ZSTD_CDict;

    if (customMem.customAlloc.is_none()) ^ (customMem.customFree.is_none()) {
        return core::ptr::null_mut();
    }

    if cctxParams.enableDedicatedDictSearch != 0 {
        cParams = ZSTD_dedicatedDictSearch_getCParams(cctxParams.compressionLevel, dictSize);
        ZSTD_overrideCParams(&mut cParams, &cctxParams.cParams);
    } else {
        cParams = ZSTD_getCParamsFromCCtxParams(
            &cctxParams,
            ZSTD_CONTENTSIZE_UNKNOWN,
            dictSize,
            ZSTD_cpm_createCDict,
        );
    }

    if ZSTD_dedicatedDictSearch_isSupported(&cParams) == 0 {
        cctxParams.enableDedicatedDictSearch = 0;
        cParams = ZSTD_getCParamsFromCCtxParams(
            &cctxParams,
            ZSTD_CONTENTSIZE_UNKNOWN,
            dictSize,
            ZSTD_cpm_createCDict,
        );
    }

    cctxParams.cParams = cParams;
    cctxParams.useRowMatchFinder =
        ZSTD_resolveRowMatchFinderMode(cctxParams.useRowMatchFinder, &cParams);

    cdict = ZSTD_createCDict_advanced_internal(
        dictSize,
        dictLoadMethod,
        cctxParams.cParams,
        cctxParams.useRowMatchFinder,
        cctxParams.enableDedicatedDictSearch,
        customMem,
    );

    if cdict.is_null()
        || ZSTD_isError(ZSTD_initCDict_internal(
            as_cdict_mut(cdict),
            dict,
            dictSize,
            dictLoadMethod,
            dictContentType,
            cctxParams,
        )) != 0
    {
        ZSTD_freeCDict(cdict);
        return core::ptr::null_mut();
    }

    cdict
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_createCDict(
    dict: *const c_void,
    dictSize: usize,
    compressionLevel: c_int,
) -> *mut ZSTD_CDict {
    let cParams = ZSTD_getCParams_internal(
        compressionLevel,
        ZSTD_CONTENTSIZE_UNKNOWN,
        dictSize,
        ZSTD_cpm_createCDict,
    );
    let cdict = ZSTD_createCDict_advanced(
        dict,
        dictSize,
        ZSTD_dlm_byCopy,
        ZSTD_dct_auto,
        cParams,
        ZSTD_defaultCMem,
    );
    if !cdict.is_null() {
        (*as_cdict_mut(cdict)).compressionLevel = if compressionLevel == 0 {
            ZSTD_CLEVEL_DEFAULT
        } else {
            compressionLevel
        };
    }
    cdict
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_createCDict_byReference(
    dict: *const c_void,
    dictSize: usize,
    compressionLevel: c_int,
) -> *mut ZSTD_CDict {
    let cParams = ZSTD_getCParams_internal(
        compressionLevel,
        ZSTD_CONTENTSIZE_UNKNOWN,
        dictSize,
        ZSTD_cpm_createCDict,
    );
    let cdict = ZSTD_createCDict_advanced(
        dict,
        dictSize,
        ZSTD_dlm_byRef,
        ZSTD_dct_auto,
        cParams,
        ZSTD_defaultCMem,
    );
    if !cdict.is_null() {
        (*as_cdict_mut(cdict)).compressionLevel = if compressionLevel == 0 {
            ZSTD_CLEVEL_DEFAULT
        } else {
            compressionLevel
        };
    }
    cdict
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_freeCDict(cdict: *mut ZSTD_CDict) -> usize {
    if cdict.is_null() {
        return 0;
    }
    let cdict = as_cdict_mut(cdict);
    {
        let cMem = (*cdict).customMem;
        let cdictInWorkspace =
            ZSTD_cwksp_owns_buffer(&(*cdict).workspace, cdict as *const c_void);
        ZSTD_cwksp_free(&mut (*cdict).workspace, cMem);
        if cdictInWorkspace == 0 {
            zstd_custom_free(cdict as *mut c_void, cMem);
        }
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_initStaticCDict(
    workspace: *mut c_void,
    workspaceSize: usize,
    dict: *const c_void,
    dictSize: usize,
    dictLoadMethod: ZSTD_dictLoadMethod_e,
    dictContentType: ZSTD_dictContentType_e,
    cParams: ZSTD_compressionParameters,
) -> *const ZSTD_CDict {
    let useRowMatchFinder = ZSTD_resolveRowMatchFinderMode(ZSTD_ps_auto, &cParams);
    let matchStateSize = ZSTD_sizeof_matchState(&cParams, useRowMatchFinder, 1, 0);
    let neededSize = ZSTD_cwksp_alloc_size(core::mem::size_of::<CDictReal>())
        + (if dictLoadMethod == ZSTD_dlm_byRef {
            0
        } else {
            ZSTD_cwksp_alloc_size(ZSTD_cwksp_align(dictSize, core::mem::size_of::<*mut c_void>()))
        })
        + ZSTD_cwksp_alloc_size(HUF_WORKSPACE_SIZE)
        + matchStateSize;
    let cdict: *mut CDictReal;
    let mut params: ZSTD_CCtx_params = core::mem::zeroed();

    if (workspace as usize) & 7 != 0 {
        return core::ptr::null();
    }

    {
        let mut ws: ZSTD_cwksp = core::mem::zeroed();
        ZSTD_cwksp_init(&mut ws, workspace, workspaceSize, ZSTD_cwksp_static_alloc);
        cdict = ZSTD_cwksp_reserve_object(&mut ws, core::mem::size_of::<CDictReal>())
            as *mut CDictReal;
        if cdict.is_null() {
            return core::ptr::null();
        }
        ZSTD_cwksp_move(&mut (*cdict).workspace, &mut ws);
    }

    if workspaceSize < neededSize {
        return core::ptr::null();
    }

    ZSTD_CCtxParams_init(&mut params, 0);
    params.cParams = cParams;
    params.useRowMatchFinder = useRowMatchFinder;
    (*cdict).useRowMatchFinder = useRowMatchFinder;
    (*cdict).compressionLevel = ZSTD_NO_CLEVEL;

    if ZSTD_isError(ZSTD_initCDict_internal(
        cdict,
        dict,
        dictSize,
        dictLoadMethod,
        dictContentType,
        params,
    )) != 0
    {
        return core::ptr::null();
    }

    cdict as *const ZSTD_CDict
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_getCParamsFromCDict(
    cdict: *const ZSTD_CDict,
) -> ZSTD_compressionParameters {
    let cdict = as_cdict(cdict);
    (*cdict).matchState.cParams
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_getDictID_fromCDict(cdict: *const ZSTD_CDict) -> c_uint {
    if cdict.is_null() {
        return 0;
    }
    (*as_cdict(cdict)).dictID
}

unsafe fn ZSTD_compressBegin_usingCDict_internal(
    cctx: *mut ZSTD_CCtx,
    cdict: *const ZSTD_CDict,
    fParams: ZSTD_frameParameters,
    pledgedSrcSize: c_ulonglong,
) -> usize {
    let mut cctxParams: ZSTD_CCtx_params = core::mem::zeroed();
    RETURN_ERROR_IF!(cdict.is_null(), DICTIONARY_WRONG);
    let cdictReal = as_cdict(cdict);
    {
        let mut params: ZSTD_parameters = core::mem::zeroed();
        params.fParams = fParams;
        params.cParams = if pledgedSrcSize < ZSTD_USE_CDICT_PARAMS_SRCSIZE_CUTOFF
            || pledgedSrcSize
                < (*cdictReal).dictContentSize as U64 * ZSTD_USE_CDICT_PARAMS_DICTSIZE_MULTIPLIER
            || pledgedSrcSize == ZSTD_CONTENTSIZE_UNKNOWN
            || (*cdictReal).compressionLevel == 0
        {
            ZSTD_getCParamsFromCDict(cdict)
        } else {
            ZSTD_getCParams(
                (*cdictReal).compressionLevel,
                pledgedSrcSize,
                (*cdictReal).dictContentSize,
            )
        };
        ZSTD_CCtxParams_init_internal(&mut cctxParams, &params, (*cdictReal).compressionLevel);
    }
    if pledgedSrcSize != ZSTD_CONTENTSIZE_UNKNOWN {
        let limitedSrcSize: U32 = core::cmp::min(pledgedSrcSize, 1u64 << 19) as U32;
        let limitedSrcLog: U32 = if limitedSrcSize > 1 {
            ZSTD_highbit32(limitedSrcSize - 1) + 1
        } else {
            1
        };
        cctxParams.cParams.windowLog =
            core::cmp::max(cctxParams.cParams.windowLog, limitedSrcLog);
    }
    ZSTD_compressBegin_internal(
        cctx,
        core::ptr::null(),
        0,
        ZSTD_dct_auto,
        ZSTD_dtlm_fast,
        cdict,
        &cctxParams,
        pledgedSrcSize,
        ZSTDb_not_buffered,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBegin_usingCDict_advanced(
    cctx: *mut ZSTD_CCtx,
    cdict: *const ZSTD_CDict,
    fParams: ZSTD_frameParameters,
    pledgedSrcSize: c_ulonglong,
) -> usize {
    ZSTD_compressBegin_usingCDict_internal(cctx, cdict, fParams, pledgedSrcSize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBegin_usingCDict_deprecated(
    cctx: *mut ZSTD_CCtx,
    cdict: *const ZSTD_CDict,
) -> usize {
    let fParams = ZSTD_frameParameters {
        contentSizeFlag: 0,
        checksumFlag: 0,
        noDictIDFlag: 0,
    };
    ZSTD_compressBegin_usingCDict_internal(cctx, cdict, fParams, ZSTD_CONTENTSIZE_UNKNOWN)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBegin_usingCDict(
    cctx: *mut ZSTD_CCtx,
    cdict: *const ZSTD_CDict,
) -> usize {
    ZSTD_compressBegin_usingCDict_deprecated(cctx, cdict)
}

unsafe fn ZSTD_compress_usingCDict_internal(
    cctx: *mut ZSTD_CCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
    cdict: *const ZSTD_CDict,
    fParams: ZSTD_frameParameters,
) -> usize {
    FORWARD_IF_ERROR!(ZSTD_compressBegin_usingCDict_internal(
        cctx,
        cdict,
        fParams,
        srcSize as U64
    ));
    ZSTD_compressEnd_public(cctx, dst, dstCapacity, src, srcSize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compress_usingCDict_advanced(
    cctx: *mut ZSTD_CCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
    cdict: *const ZSTD_CDict,
    fParams: ZSTD_frameParameters,
) -> usize {
    ZSTD_compress_usingCDict_internal(cctx, dst, dstCapacity, src, srcSize, cdict, fParams)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compress_usingCDict(
    cctx: *mut ZSTD_CCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
    cdict: *const ZSTD_CDict,
) -> usize {
    let fParams = ZSTD_frameParameters {
        contentSizeFlag: 1,
        checksumFlag: 0,
        noDictIDFlag: 0,
    };
    ZSTD_compress_usingCDict_internal(cctx, dst, dstCapacity, src, srcSize, cdict, fParams)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_createCStream() -> *mut ZSTD_CStream {
    ZSTD_createCStream_advanced(ZSTD_defaultCMem)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_initStaticCStream(
    workspace: *mut c_void,
    workspaceSize: usize,
) -> *mut ZSTD_CStream {
    ZSTD_initStaticCCtx(workspace, workspaceSize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_createCStream_advanced(customMem: ZSTD_customMem) -> *mut ZSTD_CStream {
    ZSTD_createCCtx_advanced(customMem)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_freeCStream(zcs: *mut ZSTD_CStream) -> usize {
    ZSTD_freeCCtx(zcs)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_CStreamInSize() -> usize {
    ZSTD_BLOCKSIZE_MAX
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_CStreamOutSize() -> usize {
    ZSTD_compressBound(ZSTD_BLOCKSIZE_MAX) + ZSTD_blockHeaderSize + 4
}

unsafe fn ZSTD_getCParamMode(
    cdict: *const ZSTD_CDict,
    params: *const ZSTD_CCtx_params,
    pledgedSrcSize: U64,
) -> ZSTD_CParamMode_e {
    if !cdict.is_null() && ZSTD_shouldAttachDict(cdict, params, pledgedSrcSize) != 0 {
        ZSTD_cpm_attachDict
    } else {
        ZSTD_cpm_noAttachDict
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_resetCStream(zcs: *mut ZSTD_CStream, pss: c_ulonglong) -> usize {
    let pledgedSrcSize: U64 = if pss == 0 {
        ZSTD_CONTENTSIZE_UNKNOWN
    } else {
        pss
    };
    FORWARD_IF_ERROR!(ZSTD_CCtx_reset(zcs, ZSTD_reset_session_only));
    FORWARD_IF_ERROR!(ZSTD_CCtx_setPledgedSrcSize(zcs, pledgedSrcSize));
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_initCStream_internal(
    zcs: *mut ZSTD_CStream,
    dict: *const c_void,
    dictSize: usize,
    cdict: *const ZSTD_CDict,
    params: *const ZSTD_CCtx_params,
    pledgedSrcSize: c_ulonglong,
) -> usize {
    FORWARD_IF_ERROR!(ZSTD_CCtx_reset(zcs, ZSTD_reset_session_only));
    FORWARD_IF_ERROR!(ZSTD_CCtx_setPledgedSrcSize(zcs, pledgedSrcSize));
    debug_assert!(ZSTD_isError(ZSTD_checkCParams((*params).cParams)) == 0);
    core::ptr::copy_nonoverlapping(params, &mut (*zcs).requestedParams, 1);
    if !dict.is_null() {
        FORWARD_IF_ERROR!(ZSTD_CCtx_loadDictionary(zcs, dict, dictSize));
    } else {
        FORWARD_IF_ERROR!(ZSTD_CCtx_refCDict(zcs, cdict));
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_initCStream_usingCDict_advanced(
    zcs: *mut ZSTD_CStream,
    cdict: *const ZSTD_CDict,
    fParams: ZSTD_frameParameters,
    pledgedSrcSize: c_ulonglong,
) -> usize {
    FORWARD_IF_ERROR!(ZSTD_CCtx_reset(zcs, ZSTD_reset_session_only));
    FORWARD_IF_ERROR!(ZSTD_CCtx_setPledgedSrcSize(zcs, pledgedSrcSize));
    (*zcs).requestedParams.fParams = fParams;
    FORWARD_IF_ERROR!(ZSTD_CCtx_refCDict(zcs, cdict));
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_initCStream_usingCDict(
    zcs: *mut ZSTD_CStream,
    cdict: *const ZSTD_CDict,
) -> usize {
    FORWARD_IF_ERROR!(ZSTD_CCtx_reset(zcs, ZSTD_reset_session_only));
    FORWARD_IF_ERROR!(ZSTD_CCtx_refCDict(zcs, cdict));
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_initCStream_advanced(
    zcs: *mut ZSTD_CStream,
    dict: *const c_void,
    dictSize: usize,
    params: ZSTD_parameters,
    pss: c_ulonglong,
) -> usize {
    let pledgedSrcSize: U64 = if pss == 0 && params.fParams.contentSizeFlag == 0 {
        ZSTD_CONTENTSIZE_UNKNOWN
    } else {
        pss
    };
    FORWARD_IF_ERROR!(ZSTD_CCtx_reset(zcs, ZSTD_reset_session_only));
    FORWARD_IF_ERROR!(ZSTD_CCtx_setPledgedSrcSize(zcs, pledgedSrcSize));
    FORWARD_IF_ERROR!(ZSTD_checkCParams(params.cParams));
    ZSTD_CCtxParams_setZstdParams(&mut (*zcs).requestedParams, &params);
    FORWARD_IF_ERROR!(ZSTD_CCtx_loadDictionary(zcs, dict, dictSize));
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_initCStream_usingDict(
    zcs: *mut ZSTD_CStream,
    dict: *const c_void,
    dictSize: usize,
    compressionLevel: c_int,
) -> usize {
    FORWARD_IF_ERROR!(ZSTD_CCtx_reset(zcs, ZSTD_reset_session_only));
    FORWARD_IF_ERROR!(ZSTD_CCtx_setParameter(zcs, ZSTD_c_compressionLevel, compressionLevel));
    FORWARD_IF_ERROR!(ZSTD_CCtx_loadDictionary(zcs, dict, dictSize));
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_initCStream_srcSize(
    zcs: *mut ZSTD_CStream,
    compressionLevel: c_int,
    pss: c_ulonglong,
) -> usize {
    let pledgedSrcSize: U64 = if pss == 0 {
        ZSTD_CONTENTSIZE_UNKNOWN
    } else {
        pss
    };
    FORWARD_IF_ERROR!(ZSTD_CCtx_reset(zcs, ZSTD_reset_session_only));
    FORWARD_IF_ERROR!(ZSTD_CCtx_refCDict(zcs, core::ptr::null()));
    FORWARD_IF_ERROR!(ZSTD_CCtx_setParameter(zcs, ZSTD_c_compressionLevel, compressionLevel));
    FORWARD_IF_ERROR!(ZSTD_CCtx_setPledgedSrcSize(zcs, pledgedSrcSize));
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_initCStream(zcs: *mut ZSTD_CStream, compressionLevel: c_int) -> usize {
    FORWARD_IF_ERROR!(ZSTD_CCtx_reset(zcs, ZSTD_reset_session_only));
    FORWARD_IF_ERROR!(ZSTD_CCtx_refCDict(zcs, core::ptr::null()));
    FORWARD_IF_ERROR!(ZSTD_CCtx_setParameter(zcs, ZSTD_c_compressionLevel, compressionLevel));
    0
}

unsafe fn ZSTD_nextInputSizeHint(cctx: *const ZSTD_CCtx) -> usize {
    if (*cctx).appliedParams.inBufferMode == ZSTD_bm_stable {
        return (*cctx).blockSizeMax - (*cctx).stableIn_notConsumed;
    }
    {
        let mut hintInSize = (*cctx).inBuffTarget - (*cctx).inBuffPos;
        if hintInSize == 0 {
            hintInSize = (*cctx).blockSizeMax;
        }
        hintInSize
    }
}

unsafe fn ZSTD_compressStream_generic(
    zcs: *mut ZSTD_CStream,
    output: *mut ZSTD_outBuffer,
    input: *mut ZSTD_inBuffer,
    flushMode: ZSTD_EndDirective,
) -> usize {
    let istart: *const c_char = (*input).src as *const c_char;
    let iend: *const c_char = if !istart.is_null() {
        istart.add((*input).size)
    } else {
        istart
    };
    let mut ip: *const c_char = if !istart.is_null() {
        istart.add((*input).pos)
    } else {
        istart
    };
    let ostart: *mut c_char = (*output).dst as *mut c_char;
    let oend: *mut c_char = if !ostart.is_null() {
        ostart.add((*output).size)
    } else {
        ostart
    };
    let mut op: *mut c_char = if !ostart.is_null() {
        ostart.add((*output).pos)
    } else {
        ostart
    };
    let mut someMoreWork: U32 = 1;

    if (*zcs).appliedParams.inBufferMode == ZSTD_bm_stable {
        debug_assert!((*input).pos >= (*zcs).stableIn_notConsumed);
        (*input).pos -= (*zcs).stableIn_notConsumed;
        if !ip.is_null() {
            ip = ip.sub((*zcs).stableIn_notConsumed);
        }
        (*zcs).stableIn_notConsumed = 0;
    }

    while someMoreWork != 0 {
        match (*zcs).streamStage {
            zcss_init => {
                RETURN_ERROR!(INIT_MISSING);
            }
            zcss_load => {
                'load_block: {
                    if (flushMode == ZSTD_e_end)
                        && (((oend as usize - op as usize)
                            >= ZSTD_compressBound(iend as usize - ip as usize))
                            || (*zcs).appliedParams.outBufferMode == ZSTD_bm_stable)
                        && ((*zcs).inBuffPos == 0)
                    {
                        let cSize = ZSTD_compressEnd_public(
                            zcs,
                            op as *mut c_void,
                            oend as usize - op as usize,
                            ip as *const c_void,
                            iend as usize - ip as usize,
                        );
                        FORWARD_IF_ERROR!(cSize);
                        ip = iend;
                        op = op.add(cSize);
                        (*zcs).frameEnded = 1;
                        ZSTD_CCtx_reset(zcs, ZSTD_reset_session_only);
                        someMoreWork = 0;
                        break 'load_block;
                    }
                    if (*zcs).appliedParams.inBufferMode == ZSTD_bm_buffered {
                        let toLoad = (*zcs).inBuffTarget - (*zcs).inBuffPos;
                        let loaded = ZSTD_limitCopy(
                            (*zcs).inBuff.add((*zcs).inBuffPos) as *mut c_void,
                            toLoad,
                            ip as *const c_void,
                            iend as usize - ip as usize,
                        );
                        (*zcs).inBuffPos += loaded;
                        if !ip.is_null() {
                            ip = ip.add(loaded);
                        }
                        if (flushMode == ZSTD_e_continue) && ((*zcs).inBuffPos < (*zcs).inBuffTarget)
                        {
                            someMoreWork = 0;
                            break 'load_block;
                        }
                        if (flushMode == ZSTD_e_flush) && ((*zcs).inBuffPos == (*zcs).inToCompress) {
                            someMoreWork = 0;
                            break 'load_block;
                        }
                    } else {
                        if (flushMode == ZSTD_e_continue)
                            && ((iend as usize - ip as usize) < (*zcs).blockSizeMax)
                        {
                            (*zcs).stableIn_notConsumed = iend as usize - ip as usize;
                            ip = iend;
                            someMoreWork = 0;
                            break 'load_block;
                        }
                        if (flushMode == ZSTD_e_flush) && (ip == iend) {
                            someMoreWork = 0;
                            break 'load_block;
                        }
                    }
                    {
                        let inputBuffered =
                            ((*zcs).appliedParams.inBufferMode == ZSTD_bm_buffered) as c_int;
                        let cDst: *mut c_void;
                        let cSize: usize;
                        let mut oSize: usize = oend as usize - op as usize;
                        let iSize: usize = if inputBuffered != 0 {
                            (*zcs).inBuffPos - (*zcs).inToCompress
                        } else {
                            core::cmp::min(iend as usize - ip as usize, (*zcs).blockSizeMax)
                        };
                        if oSize >= ZSTD_compressBound(iSize)
                            || (*zcs).appliedParams.outBufferMode == ZSTD_bm_stable
                        {
                            cDst = op as *mut c_void;
                        } else {
                            cDst = (*zcs).outBuff as *mut c_void;
                            oSize = (*zcs).outBuffSize;
                        }
                        if inputBuffered != 0 {
                            let lastBlock =
                                ((flushMode == ZSTD_e_end) && (ip == iend)) as c_uint;
                            cSize = if lastBlock != 0 {
                                ZSTD_compressEnd_public(
                                    zcs,
                                    cDst,
                                    oSize,
                                    (*zcs).inBuff.add((*zcs).inToCompress) as *const c_void,
                                    iSize,
                                )
                            } else {
                                ZSTD_compressContinue_public(
                                    zcs,
                                    cDst,
                                    oSize,
                                    (*zcs).inBuff.add((*zcs).inToCompress) as *const c_void,
                                    iSize,
                                )
                            };
                            FORWARD_IF_ERROR!(cSize);
                            (*zcs).frameEnded = lastBlock;
                            (*zcs).inBuffTarget = (*zcs).inBuffPos + (*zcs).blockSizeMax;
                            if (*zcs).inBuffTarget > (*zcs).inBuffSize {
                                (*zcs).inBuffPos = 0;
                                (*zcs).inBuffTarget = (*zcs).blockSizeMax;
                            }
                            if lastBlock == 0 {
                                debug_assert!((*zcs).inBuffTarget <= (*zcs).inBuffSize);
                            }
                            (*zcs).inToCompress = (*zcs).inBuffPos;
                        } else {
                            let lastBlock = ((flushMode == ZSTD_e_end)
                                && (ip.add(iSize) == iend))
                                as c_uint;
                            cSize = if lastBlock != 0 {
                                ZSTD_compressEnd_public(zcs, cDst, oSize, ip as *const c_void, iSize)
                            } else {
                                ZSTD_compressContinue_public(
                                    zcs,
                                    cDst,
                                    oSize,
                                    ip as *const c_void,
                                    iSize,
                                )
                            };
                            if !ip.is_null() {
                                ip = ip.add(iSize);
                            }
                            FORWARD_IF_ERROR!(cSize);
                            (*zcs).frameEnded = lastBlock;
                            if lastBlock != 0 {
                                debug_assert!(ip == iend);
                            }
                        }
                        if cDst == op as *mut c_void {
                            op = op.add(cSize);
                            if (*zcs).frameEnded != 0 {
                                someMoreWork = 0;
                                ZSTD_CCtx_reset(zcs, ZSTD_reset_session_only);
                            }
                            break 'load_block;
                        }
                        (*zcs).outBuffContentSize = cSize;
                        (*zcs).outBuffFlushedSize = 0;
                        (*zcs).streamStage = zcss_flush;
                        // fallthrough to zcss_flush handled below
                        ZSTD_compressStream_generic_flush(
                            zcs,
                            &mut op,
                            oend,
                            &mut someMoreWork,
                        );
                    }
                }
            }
            zcss_flush => {
                ZSTD_compressStream_generic_flush(zcs, &mut op, oend, &mut someMoreWork);
            }
            _ => {
                debug_assert!(false);
            }
        }
    }

    (*input).pos = ip as usize - istart as usize;
    (*output).pos = op as usize - ostart as usize;
    if (*zcs).frameEnded != 0 {
        return 0;
    }
    ZSTD_nextInputSizeHint(zcs)
}

/* Helper implementing the zcss_flush stage (and the fall-through from zcss_load). */
unsafe fn ZSTD_compressStream_generic_flush(
    zcs: *mut ZSTD_CStream,
    op: *mut *mut c_char,
    oend: *mut c_char,
    someMoreWork: *mut U32,
) {
    let toFlush = (*zcs).outBuffContentSize - (*zcs).outBuffFlushedSize;
    let flushed = ZSTD_limitCopy(
        *op as *mut c_void,
        oend as usize - *op as usize,
        (*zcs).outBuff.add((*zcs).outBuffFlushedSize) as *const c_void,
        toFlush,
    );
    if flushed != 0 {
        *op = (*op).add(flushed);
    }
    (*zcs).outBuffFlushedSize += flushed;
    if toFlush != flushed {
        debug_assert!(*op == oend);
        *someMoreWork = 0;
        return;
    }
    (*zcs).outBuffContentSize = 0;
    (*zcs).outBuffFlushedSize = 0;
    if (*zcs).frameEnded != 0 {
        *someMoreWork = 0;
        ZSTD_CCtx_reset(zcs, ZSTD_reset_session_only);
        return;
    }
    (*zcs).streamStage = zcss_load;
}

unsafe fn ZSTD_nextInputSizeHint_MTorST(cctx: *const ZSTD_CCtx) -> usize {
    ZSTD_nextInputSizeHint(cctx)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressStream(
    zcs: *mut ZSTD_CStream,
    output: *mut ZSTD_outBuffer,
    input: *mut ZSTD_inBuffer,
) -> usize {
    FORWARD_IF_ERROR!(ZSTD_compressStream2(zcs, output, input, ZSTD_e_continue));
    ZSTD_nextInputSizeHint_MTorST(zcs)
}

unsafe fn ZSTD_setBufferExpectations(
    cctx: *mut ZSTD_CCtx,
    output: *const ZSTD_outBuffer,
    input: *const ZSTD_inBuffer,
) {
    if (*cctx).appliedParams.inBufferMode == ZSTD_bm_stable {
        (*cctx).expectedInBuffer = *input;
    }
    if (*cctx).appliedParams.outBufferMode == ZSTD_bm_stable {
        (*cctx).expectedOutBufferSize = (*output).size - (*output).pos;
    }
}

unsafe fn ZSTD_checkBufferStability(
    cctx: *const ZSTD_CCtx,
    output: *const ZSTD_outBuffer,
    input: *const ZSTD_inBuffer,
    endOp: ZSTD_EndDirective,
) -> usize {
    if (*cctx).appliedParams.inBufferMode == ZSTD_bm_stable {
        let expect = (*cctx).expectedInBuffer;
        if expect.src != (*input).src || expect.pos != (*input).pos {
            RETURN_ERROR!(STABILITYCONDITION_NOTRESPECTED);
        }
    }
    let _ = endOp;
    if (*cctx).appliedParams.outBufferMode == ZSTD_bm_stable {
        let outBufferSize = (*output).size - (*output).pos;
        if (*cctx).expectedOutBufferSize != outBufferSize {
            RETURN_ERROR!(STABILITYCONDITION_NOTRESPECTED);
        }
    }
    0
}

unsafe fn ZSTD_CCtx_init_compressStream2(
    cctx: *mut ZSTD_CCtx,
    endOp: ZSTD_EndDirective,
    inSize: usize,
) -> usize {
    let mut params: ZSTD_CCtx_params = core::ptr::read(&(*cctx).requestedParams);
    let prefixDict: ZSTD_prefixDict = core::ptr::read(&(*cctx).prefixDict);
    FORWARD_IF_ERROR!(ZSTD_initLocalDict(cctx));
    memset(
        &mut (*cctx).prefixDict as *mut _ as *mut c_void,
        0,
        core::mem::size_of::<ZSTD_prefixDict>(),
    );
    if !(*cctx).cdict.is_null() && (*cctx).localDict.cdict.is_null() {
        params.compressionLevel = (*as_cdict((*cctx).cdict)).compressionLevel;
    }
    if endOp == ZSTD_e_end {
        (*cctx).pledgedSrcSizePlusOne = inSize as u64 + 1;
    }

    {
        let dictSize: usize = if !prefixDict.dict.is_null() {
            prefixDict.dictSize
        } else if !(*cctx).cdict.is_null() {
            (*as_cdict((*cctx).cdict)).dictContentSize
        } else {
            0
        };
        let mode = ZSTD_getCParamMode((*cctx).cdict, &params, (*cctx).pledgedSrcSizePlusOne - 1);
        params.cParams = ZSTD_getCParamsFromCCtxParams(
            &params,
            (*cctx).pledgedSrcSizePlusOne - 1,
            dictSize,
            mode,
        );
    }

    params.postBlockSplitter =
        ZSTD_resolveBlockSplitterMode(params.postBlockSplitter, &params.cParams);
    params.ldmParams.enableLdm =
        ZSTD_resolveEnableLdm(params.ldmParams.enableLdm, &params.cParams);
    params.useRowMatchFinder =
        ZSTD_resolveRowMatchFinderMode(params.useRowMatchFinder, &params.cParams);
    params.validateSequences = ZSTD_resolveExternalSequenceValidation(params.validateSequences);
    params.maxBlockSize = ZSTD_resolveMaxBlockSize(params.maxBlockSize);
    params.searchForExternalRepcodes = ZSTD_resolveExternalRepcodeSearch(
        params.searchForExternalRepcodes,
        params.compressionLevel,
    );

    {
        let pledgedSrcSize: U64 = (*cctx).pledgedSrcSizePlusOne - 1;
        debug_assert!(ZSTD_isError(ZSTD_checkCParams(params.cParams)) == 0);
        FORWARD_IF_ERROR!(ZSTD_compressBegin_internal(
            cctx,
            prefixDict.dict,
            prefixDict.dictSize,
            prefixDict.dictContentType,
            ZSTD_dtlm_fast,
            (*cctx).cdict,
            &params,
            pledgedSrcSize,
            ZSTDb_buffered,
        ));
        debug_assert!((*cctx).appliedParams.nbWorkers == 0);
        (*cctx).inToCompress = 0;
        (*cctx).inBuffPos = 0;
        if (*cctx).appliedParams.inBufferMode == ZSTD_bm_buffered {
            (*cctx).inBuffTarget =
                (*cctx).blockSizeMax + ((*cctx).blockSizeMax as u64 == pledgedSrcSize) as usize;
        } else {
            (*cctx).inBuffTarget = 0;
        }
        (*cctx).outBuffContentSize = 0;
        (*cctx).outBuffFlushedSize = 0;
        (*cctx).streamStage = zcss_load;
        (*cctx).frameEnded = 0;
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressStream2(
    cctx: *mut ZSTD_CCtx,
    output: *mut ZSTD_outBuffer,
    input: *mut ZSTD_inBuffer,
    endOp: ZSTD_EndDirective,
) -> usize {
    RETURN_ERROR_IF!((*output).pos > (*output).size, DSTSIZE_TOOSMALL);
    RETURN_ERROR_IF!((*input).pos > (*input).size, SRCSIZE_WRONG);
    RETURN_ERROR_IF!(endOp > ZSTD_e_end, PARAMETER_OUTOFBOUND);

    if (*cctx).streamStage == zcss_init {
        let inputSize = (*input).size - (*input).pos;
        let totalInputSize = inputSize + (*cctx).stableIn_notConsumed;
        if ((*cctx).requestedParams.inBufferMode == ZSTD_bm_stable)
            && (endOp == ZSTD_e_continue)
            && (totalInputSize < ZSTD_BLOCKSIZE_MAX)
        {
            if (*cctx).stableIn_notConsumed != 0 {
                RETURN_ERROR_IF!(
                    (*input).src != (*cctx).expectedInBuffer.src,
                    STABILITYCONDITION_NOTRESPECTED
                );
                RETURN_ERROR_IF!(
                    (*input).pos != (*cctx).expectedInBuffer.size,
                    STABILITYCONDITION_NOTRESPECTED
                );
            }
            (*input).pos = (*input).size;
            (*cctx).expectedInBuffer = *input;
            (*cctx).stableIn_notConsumed += inputSize;
            return ZSTD_FRAMEHEADERSIZE_MIN((*cctx).requestedParams.format);
        }
        FORWARD_IF_ERROR!(ZSTD_CCtx_init_compressStream2(cctx, endOp, totalInputSize));
        ZSTD_setBufferExpectations(cctx, output, input);
    }

    FORWARD_IF_ERROR!(ZSTD_checkBufferStability(cctx, output, input, endOp));
    FORWARD_IF_ERROR!(ZSTD_compressStream_generic(cctx, output, input, endOp));
    ZSTD_setBufferExpectations(cctx, output, input);
    (*cctx).outBuffContentSize - (*cctx).outBuffFlushedSize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressStream2_simpleArgs(
    cctx: *mut ZSTD_CCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    dstPos: *mut usize,
    src: *const c_void,
    srcSize: usize,
    srcPos: *mut usize,
    endOp: ZSTD_EndDirective,
) -> usize {
    let mut output: ZSTD_outBuffer = ZSTD_outBuffer {
        dst,
        size: dstCapacity,
        pos: *dstPos,
    };
    let mut input: ZSTD_inBuffer = ZSTD_inBuffer {
        src,
        size: srcSize,
        pos: *srcPos,
    };
    {
        let cErr = ZSTD_compressStream2(cctx, &mut output, &mut input, endOp);
        *dstPos = output.pos;
        *srcPos = input.pos;
        cErr
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compress2(
    cctx: *mut ZSTD_CCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let originalInBufferMode = (*cctx).requestedParams.inBufferMode;
    let originalOutBufferMode = (*cctx).requestedParams.outBufferMode;
    ZSTD_CCtx_reset(cctx, ZSTD_reset_session_only);
    (*cctx).requestedParams.inBufferMode = ZSTD_bm_stable;
    (*cctx).requestedParams.outBufferMode = ZSTD_bm_stable;
    {
        let mut oPos: usize = 0;
        let mut iPos: usize = 0;
        let result = ZSTD_compressStream2_simpleArgs(
            cctx,
            dst,
            dstCapacity,
            &mut oPos,
            src,
            srcSize,
            &mut iPos,
            ZSTD_e_end,
        );
        (*cctx).requestedParams.inBufferMode = originalInBufferMode;
        (*cctx).requestedParams.outBufferMode = originalOutBufferMode;

        FORWARD_IF_ERROR!(result);
        if result != 0 {
            RETURN_ERROR!(DSTSIZE_TOOSMALL);
        }
        oPos
    }
}

unsafe fn ZSTD_validateSequence(
    offBase: U32,
    matchLength: U32,
    minMatch: U32,
    posInSrc: usize,
    windowLog: U32,
    dictSize: usize,
    useSequenceProducer: c_int,
) -> usize {
    let windowSize: U32 = 1u32 << windowLog;
    let offsetBound: usize = if posInSrc > windowSize as usize {
        windowSize as usize
    } else {
        posInSrc + dictSize
    };
    let matchLenLowerBound: usize = if minMatch == 3 || useSequenceProducer != 0 {
        3
    } else {
        4
    };
    RETURN_ERROR_IF!(offBase > OFFSET_TO_OFFBASE(offsetBound as U32), EXTERNALSEQUENCES_INVALID);
    RETURN_ERROR_IF!(
        (matchLength as usize) < matchLenLowerBound,
        EXTERNALSEQUENCES_INVALID
    );
    0
}

unsafe fn ZSTD_finalizeOffBase(rawOffset: U32, rep: *const U32, ll0: U32) -> U32 {
    let mut offBase = OFFSET_TO_OFFBASE(rawOffset);

    if ll0 == 0 && rawOffset == *rep.add(0) {
        offBase = REPCODE1_TO_OFFBASE;
    } else if rawOffset == *rep.add(1) {
        offBase = REPCODE_TO_OFFBASE(2 - ll0);
    } else if rawOffset == *rep.add(2) {
        offBase = REPCODE_TO_OFFBASE(3 - ll0);
    } else if ll0 != 0 && rawOffset == *rep.add(0) - 1 {
        offBase = REPCODE3_TO_OFFBASE;
    }
    offBase
}

unsafe extern "C" fn ZSTD_transferSequences_wBlockDelim(
    cctx: *mut ZSTD_CCtx,
    seqPos: *mut ZSTD_SequencePosition,
    inSeqs: *const ZSTD_Sequence,
    inSeqsSize: usize,
    src: *const c_void,
    blockSize: usize,
    externalRepSearch: ZSTD_ParamSwitch_e,
) -> usize {
    let mut idx: U32 = (*seqPos).idx;
    let startIdx: U32 = idx;
    let mut ip: *const u8 = src as *const u8;
    let iend: *const u8 = ip.add(blockSize);
    let mut updatedRepcodes: Repcodes_t = core::mem::zeroed();
    let dictSize: U32;

    if !(*cctx).cdict.is_null() {
        dictSize = (*as_cdict((*cctx).cdict)).dictContentSize as U32;
    } else if !(*cctx).prefixDict.dict.is_null() {
        dictSize = (*cctx).prefixDict.dictSize as U32;
    } else {
        dictSize = 0;
    }
    memcpy(
        updatedRepcodes.rep.as_mut_ptr() as *mut c_void,
        (*(*cctx).blockState.prevCBlock).rep.as_ptr() as *const c_void,
        core::mem::size_of::<Repcodes_t>(),
    );
    while idx < inSeqsSize as U32
        && ((*inSeqs.add(idx as usize)).matchLength != 0
            || (*inSeqs.add(idx as usize)).offset != 0)
    {
        let litLength: U32 = (*inSeqs.add(idx as usize)).litLength;
        let matchLength: U32 = (*inSeqs.add(idx as usize)).matchLength;
        let offBase: U32;

        if externalRepSearch == ZSTD_ps_disable {
            offBase = OFFSET_TO_OFFBASE((*inSeqs.add(idx as usize)).offset);
        } else {
            let ll0: U32 = (litLength == 0) as U32;
            offBase = ZSTD_finalizeOffBase(
                (*inSeqs.add(idx as usize)).offset,
                updatedRepcodes.rep.as_ptr(),
                ll0,
            );
            ZSTD_updateRep(updatedRepcodes.rep.as_mut_ptr(), offBase, ll0);
        }

        if (*cctx).appliedParams.validateSequences != 0 {
            (*seqPos).posInSrc += (litLength + matchLength) as usize;
            FORWARD_IF_ERROR!(ZSTD_validateSequence(
                offBase,
                matchLength,
                (*cctx).appliedParams.cParams.minMatch,
                (*seqPos).posInSrc,
                (*cctx).appliedParams.cParams.windowLog,
                dictSize as usize,
                ZSTD_hasExtSeqProd(&(*cctx).appliedParams),
            ));
        }
        RETURN_ERROR_IF!(
            (idx - (*seqPos).idx) as usize >= (*cctx).seqStore.maxNbSeq,
            EXTERNALSEQUENCES_INVALID
        );
        ZSTD_storeSeq(
            &mut (*cctx).seqStore,
            litLength as usize,
            ip,
            iend,
            offBase,
            matchLength as usize,
        );
        ip = ip.add((matchLength + litLength) as usize);
        idx += 1;
    }
    RETURN_ERROR_IF!(idx == inSeqsSize as U32, EXTERNALSEQUENCES_INVALID);

    debug_assert!(externalRepSearch != ZSTD_ps_auto);
    debug_assert!(idx >= startIdx);
    if externalRepSearch == ZSTD_ps_disable && idx != startIdx {
        let rep: *mut U32 = updatedRepcodes.rep.as_mut_ptr();
        let lastSeqIdx: U32 = idx - 1;

        if lastSeqIdx >= startIdx + 2 {
            *rep.add(2) = (*inSeqs.add((lastSeqIdx - 2) as usize)).offset;
            *rep.add(1) = (*inSeqs.add((lastSeqIdx - 1) as usize)).offset;
            *rep.add(0) = (*inSeqs.add(lastSeqIdx as usize)).offset;
        } else if lastSeqIdx == startIdx + 1 {
            *rep.add(2) = *rep.add(0);
            *rep.add(1) = (*inSeqs.add((lastSeqIdx - 1) as usize)).offset;
            *rep.add(0) = (*inSeqs.add(lastSeqIdx as usize)).offset;
        } else {
            debug_assert!(lastSeqIdx == startIdx);
            *rep.add(2) = *rep.add(1);
            *rep.add(1) = *rep.add(0);
            *rep.add(0) = (*inSeqs.add(lastSeqIdx as usize)).offset;
        }
    }

    memcpy(
        (*(*cctx).blockState.nextCBlock).rep.as_mut_ptr() as *mut c_void,
        updatedRepcodes.rep.as_ptr() as *const c_void,
        core::mem::size_of::<Repcodes_t>(),
    );

    if (*inSeqs.add(idx as usize)).litLength != 0 {
        ZSTD_storeLastLiterals(
            &mut (*cctx).seqStore,
            ip,
            (*inSeqs.add(idx as usize)).litLength as usize,
        );
        ip = ip.add((*inSeqs.add(idx as usize)).litLength as usize);
        (*seqPos).posInSrc += (*inSeqs.add(idx as usize)).litLength as usize;
    }
    RETURN_ERROR_IF!(ip != iend, EXTERNALSEQUENCES_INVALID);
    (*seqPos).idx = idx + 1;
    blockSize
}

unsafe extern "C" fn ZSTD_transferSequences_noDelim(
    cctx: *mut ZSTD_CCtx,
    seqPos: *mut ZSTD_SequencePosition,
    inSeqs: *const ZSTD_Sequence,
    inSeqsSize: usize,
    src: *const c_void,
    blockSize: usize,
    externalRepSearch: ZSTD_ParamSwitch_e,
) -> usize {
    let mut idx: U32 = (*seqPos).idx;
    let mut startPosInSequence: U32 = (*seqPos).posInSequence;
    let mut endPosInSequence: U32 = (*seqPos).posInSequence + blockSize as U32;
    let dictSize: usize;
    let istart: *const u8 = src as *const u8;
    let mut ip: *const u8 = istart;
    let mut iend: *const u8 = istart.add(blockSize);
    let mut updatedRepcodes: Repcodes_t = core::mem::zeroed();
    let mut bytesAdjustment: U32 = 0;
    let mut finalMatchSplit: U32 = 0;

    let _ = externalRepSearch;

    if !(*cctx).cdict.is_null() {
        dictSize = (*as_cdict((*cctx).cdict)).dictContentSize;
    } else if !(*cctx).prefixDict.dict.is_null() {
        dictSize = (*cctx).prefixDict.dictSize;
    } else {
        dictSize = 0;
    }
    memcpy(
        updatedRepcodes.rep.as_mut_ptr() as *mut c_void,
        (*(*cctx).blockState.prevCBlock).rep.as_ptr() as *const c_void,
        core::mem::size_of::<Repcodes_t>(),
    );
    while endPosInSequence != 0 && idx < inSeqsSize as U32 && finalMatchSplit == 0 {
        let currSeq = *inSeqs.add(idx as usize);
        let mut litLength: U32 = currSeq.litLength;
        let mut matchLength: U32 = currSeq.matchLength;
        let rawOffset: U32 = currSeq.offset;
        let offBase: U32;

        if endPosInSequence >= currSeq.litLength + currSeq.matchLength {
            if startPosInSequence >= litLength {
                startPosInSequence -= litLength;
                litLength = 0;
                matchLength -= startPosInSequence;
            } else {
                litLength -= startPosInSequence;
            }
            endPosInSequence -= currSeq.litLength + currSeq.matchLength;
            startPosInSequence = 0;
        } else {
            if endPosInSequence > litLength {
                let firstHalfMatchLength: U32;
                litLength = if startPosInSequence >= litLength {
                    0
                } else {
                    litLength - startPosInSequence
                };
                let mut firstHalfMatchLength_v = endPosInSequence - startPosInSequence - litLength;
                if matchLength as usize > blockSize
                    && firstHalfMatchLength_v >= (*cctx).appliedParams.cParams.minMatch
                {
                    let secondHalfMatchLength =
                        currSeq.matchLength + currSeq.litLength - endPosInSequence;
                    if secondHalfMatchLength < (*cctx).appliedParams.cParams.minMatch {
                        endPosInSequence -=
                            (*cctx).appliedParams.cParams.minMatch - secondHalfMatchLength;
                        bytesAdjustment =
                            (*cctx).appliedParams.cParams.minMatch - secondHalfMatchLength;
                        firstHalfMatchLength_v -= bytesAdjustment;
                    }
                    firstHalfMatchLength = firstHalfMatchLength_v;
                    matchLength = firstHalfMatchLength;
                    finalMatchSplit = 1;
                } else {
                    bytesAdjustment = endPosInSequence - currSeq.litLength;
                    endPosInSequence = currSeq.litLength;
                    break;
                }
            } else {
                break;
            }
        }
        {
            let ll0: U32 = (litLength == 0) as U32;
            offBase = ZSTD_finalizeOffBase(rawOffset, updatedRepcodes.rep.as_ptr(), ll0);
            ZSTD_updateRep(updatedRepcodes.rep.as_mut_ptr(), offBase, ll0);
        }

        if (*cctx).appliedParams.validateSequences != 0 {
            (*seqPos).posInSrc += (litLength + matchLength) as usize;
            FORWARD_IF_ERROR!(ZSTD_validateSequence(
                offBase,
                matchLength,
                (*cctx).appliedParams.cParams.minMatch,
                (*seqPos).posInSrc,
                (*cctx).appliedParams.cParams.windowLog,
                dictSize,
                ZSTD_hasExtSeqProd(&(*cctx).appliedParams),
            ));
        }
        RETURN_ERROR_IF!(
            (idx - (*seqPos).idx) as usize >= (*cctx).seqStore.maxNbSeq,
            EXTERNALSEQUENCES_INVALID
        );
        ZSTD_storeSeq(
            &mut (*cctx).seqStore,
            litLength as usize,
            ip,
            iend,
            offBase,
            matchLength as usize,
        );
        ip = ip.add((matchLength + litLength) as usize);
        if finalMatchSplit == 0 {
            idx += 1;
        }
    }
    debug_assert!(
        idx == inSeqsSize as U32
            || endPosInSequence
                <= (*inSeqs.add(idx as usize)).litLength + (*inSeqs.add(idx as usize)).matchLength
    );
    (*seqPos).idx = idx;
    (*seqPos).posInSequence = endPosInSequence;
    memcpy(
        (*(*cctx).blockState.nextCBlock).rep.as_mut_ptr() as *mut c_void,
        updatedRepcodes.rep.as_ptr() as *const c_void,
        core::mem::size_of::<Repcodes_t>(),
    );

    iend = iend.sub(bytesAdjustment as usize);
    if ip != iend {
        let lastLLSize: U32 = (iend as usize - ip as usize) as U32;
        debug_assert!(ip <= iend);
        ZSTD_storeLastLiterals(&mut (*cctx).seqStore, ip, lastLLSize as usize);
        (*seqPos).posInSrc += lastLLSize as usize;
    }

    iend as usize - istart as usize
}

type ZSTD_SequenceCopier_f = unsafe extern "C" fn(
    *mut ZSTD_CCtx,
    *mut ZSTD_SequencePosition,
    *const ZSTD_Sequence,
    usize,
    *const c_void,
    usize,
    ZSTD_ParamSwitch_e,
) -> usize;

unsafe fn ZSTD_selectSequenceCopier(mode: ZSTD_SequenceFormat_e) -> ZSTD_SequenceCopier_f {
    if mode == ZSTD_sf_explicitBlockDelimiters {
        return ZSTD_transferSequences_wBlockDelim;
    }
    debug_assert!(mode == ZSTD_sf_noBlockDelimiters);
    ZSTD_transferSequences_noDelim
}

unsafe fn blockSize_explicitDelimiter(
    inSeqs: *const ZSTD_Sequence,
    inSeqsSize: usize,
    seqPos: ZSTD_SequencePosition,
) -> usize {
    let mut end: c_int = 0;
    let mut blockSize: usize = 0;
    let mut spos: usize = seqPos.idx as usize;
    debug_assert!(spos <= inSeqsSize);
    while spos < inSeqsSize {
        end = ((*inSeqs.add(spos)).offset == 0) as c_int;
        blockSize += ((*inSeqs.add(spos)).litLength + (*inSeqs.add(spos)).matchLength) as usize;
        if end != 0 {
            if (*inSeqs.add(spos)).matchLength != 0 {
                RETURN_ERROR!(EXTERNALSEQUENCES_INVALID);
            }
            break;
        }
        spos += 1;
    }
    if end == 0 {
        RETURN_ERROR!(EXTERNALSEQUENCES_INVALID);
    }
    blockSize
}

unsafe fn determine_blockSize(
    mode: ZSTD_SequenceFormat_e,
    blockSize: usize,
    remaining: usize,
    inSeqs: *const ZSTD_Sequence,
    inSeqsSize: usize,
    seqPos: ZSTD_SequencePosition,
) -> usize {
    if mode == ZSTD_sf_noBlockDelimiters {
        return core::cmp::min(remaining, blockSize);
    }
    debug_assert!(mode == ZSTD_sf_explicitBlockDelimiters);
    {
        let explicitBlockSize = blockSize_explicitDelimiter(inSeqs, inSeqsSize, seqPos);
        FORWARD_IF_ERROR!(explicitBlockSize);
        if explicitBlockSize > blockSize {
            RETURN_ERROR!(EXTERNALSEQUENCES_INVALID);
        }
        if explicitBlockSize > remaining {
            RETURN_ERROR!(EXTERNALSEQUENCES_INVALID);
        }
        explicitBlockSize
    }
}

unsafe fn ZSTD_compressSequences_internal(
    cctx: *mut ZSTD_CCtx,
    dst: *mut c_void,
    mut dstCapacity: usize,
    inSeqs: *const ZSTD_Sequence,
    inSeqsSize: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let mut cSize: usize = 0;
    let mut remaining: usize = srcSize;
    let mut seqPos = ZSTD_SequencePosition {
        idx: 0,
        posInSequence: 0,
        posInSrc: 0,
    };

    let mut ip: *const u8 = src as *const u8;
    let mut op: *mut u8 = dst as *mut u8;
    let sequenceCopier = ZSTD_selectSequenceCopier((*cctx).appliedParams.blockDelimiters);

    if remaining == 0 {
        let cBlockHeader24: U32 = 1 + ((bt_raw) << 1);
        RETURN_ERROR_IF!(dstCapacity < 4, DSTSIZE_TOOSMALL);
        MEM_writeLE32(op as *mut c_void, cBlockHeader24);
        op = op.add(ZSTD_blockHeaderSize);
        dstCapacity -= ZSTD_blockHeaderSize;
        cSize += ZSTD_blockHeaderSize;
    }

    while remaining != 0 {
        let mut compressedSeqsSize: usize;
        let cBlockSize: usize;
        let mut blockSize = determine_blockSize(
            (*cctx).appliedParams.blockDelimiters,
            (*cctx).blockSizeMax,
            remaining,
            inSeqs,
            inSeqsSize,
            seqPos,
        );
        let lastBlock: U32 = (blockSize == remaining) as U32;
        FORWARD_IF_ERROR!(blockSize);
        ZSTD_resetSeqStore(&mut (*cctx).seqStore);

        blockSize = sequenceCopier(
            cctx,
            &mut seqPos,
            inSeqs,
            inSeqsSize,
            ip as *const c_void,
            blockSize,
            (*cctx).appliedParams.searchForExternalRepcodes,
        );
        FORWARD_IF_ERROR!(blockSize);

        if blockSize < MIN_CBLOCK_SIZE + ZSTD_blockHeaderSize + 1 + 1 {
            cBlockSize = ZSTD_noCompressBlock(
                op as *mut c_void,
                dstCapacity,
                ip as *const c_void,
                blockSize,
                lastBlock,
            );
            FORWARD_IF_ERROR!(cBlockSize);
            cSize += cBlockSize;
            ip = ip.add(blockSize);
            op = op.add(cBlockSize);
            remaining -= blockSize;
            dstCapacity -= cBlockSize;
            continue;
        }

        RETURN_ERROR_IF!(dstCapacity < ZSTD_blockHeaderSize, DSTSIZE_TOOSMALL);
        compressedSeqsSize = ZSTD_entropyCompressSeqStore(
            &(*cctx).seqStore,
            &(*(*cctx).blockState.prevCBlock).entropy,
            &mut (*(*cctx).blockState.nextCBlock).entropy,
            &(*cctx).appliedParams,
            op.add(ZSTD_blockHeaderSize) as *mut c_void,
            dstCapacity - ZSTD_blockHeaderSize,
            blockSize,
            (*cctx).tmpWorkspace,
            (*cctx).tmpWkspSize,
            (*cctx).bmi2,
        );
        FORWARD_IF_ERROR!(compressedSeqsSize);

        if (*cctx).isFirstBlock == 0
            && ZSTD_maybeRLE(&(*cctx).seqStore) != 0
            && ZSTD_isRLE(ip, blockSize) != 0
        {
            compressedSeqsSize = 1;
        }

        if compressedSeqsSize == 0 {
            cBlockSize = ZSTD_noCompressBlock(
                op as *mut c_void,
                dstCapacity,
                ip as *const c_void,
                blockSize,
                lastBlock,
            );
            FORWARD_IF_ERROR!(cBlockSize);
        } else if compressedSeqsSize == 1 {
            cBlockSize = ZSTD_rleCompressBlock(op as *mut c_void, dstCapacity, *ip, blockSize, lastBlock);
            FORWARD_IF_ERROR!(cBlockSize);
        } else {
            let cBlockHeader: U32;
            ZSTD_blockState_confirmRepcodesAndEntropyTables(&mut (*cctx).blockState);
            if (*(*cctx).blockState.prevCBlock).entropy.fse.offcode_repeatMode == FSE_repeat_valid {
                (*(*cctx).blockState.prevCBlock).entropy.fse.offcode_repeatMode = FSE_repeat_check;
            }
            cBlockHeader =
                lastBlock + ((bt_compressed) << 1) + ((compressedSeqsSize << 3) as U32);
            MEM_writeLE24(op as *mut c_void, cBlockHeader);
            cBlockSize = ZSTD_blockHeaderSize + compressedSeqsSize;
        }

        cSize += cBlockSize;

        if lastBlock != 0 {
            break;
        } else {
            ip = ip.add(blockSize);
            op = op.add(cBlockSize);
            remaining -= blockSize;
            dstCapacity -= cBlockSize;
            (*cctx).isFirstBlock = 0;
        }
    }

    cSize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressSequences(
    cctx: *mut ZSTD_CCtx,
    dst: *mut c_void,
    mut dstCapacity: usize,
    inSeqs: *const ZSTD_Sequence,
    inSeqsSize: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let mut op: *mut u8 = dst as *mut u8;
    let mut cSize: usize = 0;

    FORWARD_IF_ERROR!(ZSTD_CCtx_init_compressStream2(cctx, ZSTD_e_end, srcSize));

    {
        let frameHeaderSize = ZSTD_writeFrameHeader(
            op as *mut c_void,
            dstCapacity,
            &(*cctx).appliedParams,
            srcSize as U64,
            (*cctx).dictID,
        );
        op = op.add(frameHeaderSize);
        debug_assert!(frameHeaderSize <= dstCapacity);
        dstCapacity -= frameHeaderSize;
        cSize += frameHeaderSize;
    }
    if (*cctx).appliedParams.fParams.checksumFlag != 0 && srcSize != 0 {
        ZSTD_XXH64_update(&mut (*cctx).xxhState, src, srcSize);
    }

    {
        let cBlocksSize = ZSTD_compressSequences_internal(
            cctx,
            op as *mut c_void,
            dstCapacity,
            inSeqs,
            inSeqsSize,
            src,
            srcSize,
        );
        FORWARD_IF_ERROR!(cBlocksSize);
        cSize += cBlocksSize;
        debug_assert!(cBlocksSize <= dstCapacity);
        dstCapacity -= cBlocksSize;
    }

    if (*cctx).appliedParams.fParams.checksumFlag != 0 {
        let checksum: U32 = ZSTD_XXH64_digest(&(*cctx).xxhState) as U32;
        RETURN_ERROR_IF!(dstCapacity < 4, DSTSIZE_TOOSMALL);
        MEM_writeLE32((dst as *mut c_char).add(cSize) as *mut c_void, checksum);
        cSize += 4;
    }

    cSize
}

unsafe fn convertSequences_noRepcodes(
    dstSeqs: *mut SeqDef,
    inSeqs: *const ZSTD_Sequence,
    nbSequences: usize,
) -> usize {
    let mut longLen: usize = 0;
    let mut n = 0;
    while n < nbSequences {
        (*dstSeqs.add(n)).offBase = OFFSET_TO_OFFBASE((*inSeqs.add(n)).offset);
        (*dstSeqs.add(n)).litLength = (*inSeqs.add(n)).litLength as U16;
        (*dstSeqs.add(n)).mlBase = ((*inSeqs.add(n)).matchLength - MINMATCH) as U16;
        if (*inSeqs.add(n)).matchLength > 65535 + MINMATCH {
            longLen = n + 1;
        }
        if (*inSeqs.add(n)).litLength > 65535 {
            longLen = n + nbSequences + 1;
        }
        n += 1;
    }
    longLen
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_convertBlockSequences(
    cctx: *mut ZSTD_CCtx,
    inSeqs: *const ZSTD_Sequence,
    nbSequences: usize,
    repcodeResolution: c_int,
) -> usize {
    let mut updatedRepcodes: Repcodes_t = core::mem::zeroed();
    let mut seqNb: usize = 0;

    RETURN_ERROR_IF!(
        nbSequences >= (*cctx).seqStore.maxNbSeq,
        EXTERNALSEQUENCES_INVALID
    );

    memcpy(
        updatedRepcodes.rep.as_mut_ptr() as *mut c_void,
        (*(*cctx).blockState.prevCBlock).rep.as_ptr() as *const c_void,
        core::mem::size_of::<Repcodes_t>(),
    );

    debug_assert!(nbSequences >= 1);

    if repcodeResolution == 0 {
        let longl = convertSequences_noRepcodes(
            (*cctx).seqStore.sequencesStart,
            inSeqs,
            nbSequences - 1,
        );
        (*cctx).seqStore.sequences = (*cctx).seqStore.sequencesStart.add(nbSequences - 1);
        if longl != 0 {
            debug_assert!((*cctx).seqStore.longLengthType == ZSTD_llt_none);
            if longl <= nbSequences - 1 {
                (*cctx).seqStore.longLengthType = ZSTD_llt_matchLength;
                (*cctx).seqStore.longLengthPos = (longl - 1) as U32;
            } else {
                debug_assert!(longl <= 2 * (nbSequences - 1));
                (*cctx).seqStore.longLengthType = ZSTD_llt_literalLength;
                (*cctx).seqStore.longLengthPos = (longl - (nbSequences - 1) - 1) as U32;
            }
        }
    } else {
        seqNb = 0;
        while seqNb < nbSequences - 1 {
            let litLength: U32 = (*inSeqs.add(seqNb)).litLength;
            let matchLength: U32 = (*inSeqs.add(seqNb)).matchLength;
            let ll0: U32 = (litLength == 0) as U32;
            let offBase: U32 =
                ZSTD_finalizeOffBase((*inSeqs.add(seqNb)).offset, updatedRepcodes.rep.as_ptr(), ll0);

            ZSTD_storeSeqOnly(
                &mut (*cctx).seqStore,
                litLength as usize,
                offBase,
                matchLength as usize,
            );
            ZSTD_updateRep(updatedRepcodes.rep.as_mut_ptr(), offBase, ll0);
            seqNb += 1;
        }
    }

    if repcodeResolution == 0 && nbSequences > 1 {
        let rep: *mut U32 = updatedRepcodes.rep.as_mut_ptr();

        if nbSequences >= 4 {
            let lastSeqIdx: U32 = nbSequences as U32 - 2;
            *rep.add(2) = (*inSeqs.add((lastSeqIdx - 2) as usize)).offset;
            *rep.add(1) = (*inSeqs.add((lastSeqIdx - 1) as usize)).offset;
            *rep.add(0) = (*inSeqs.add(lastSeqIdx as usize)).offset;
        } else if nbSequences == 3 {
            *rep.add(2) = *rep.add(0);
            *rep.add(1) = (*inSeqs.add(0)).offset;
            *rep.add(0) = (*inSeqs.add(1)).offset;
        } else {
            debug_assert!(nbSequences == 2);
            *rep.add(2) = *rep.add(1);
            *rep.add(1) = *rep.add(0);
            *rep.add(0) = (*inSeqs.add(0)).offset;
        }
    }

    memcpy(
        (*(*cctx).blockState.nextCBlock).rep.as_mut_ptr() as *mut c_void,
        updatedRepcodes.rep.as_ptr() as *const c_void,
        core::mem::size_of::<Repcodes_t>(),
    );

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_get1BlockSummary(
    seqs: *const ZSTD_Sequence,
    nbSeqs: usize,
) -> BlockSummary {
    let mut totalMatchSize: usize = 0;
    let mut litSize: usize = 0;
    let mut n = 0;
    debug_assert!(!seqs.is_null());
    while n < nbSeqs {
        totalMatchSize += (*seqs.add(n)).matchLength as usize;
        litSize += (*seqs.add(n)).litLength as usize;
        if (*seqs.add(n)).matchLength == 0 {
            debug_assert!((*seqs.add(n)).offset == 0);
            break;
        }
        n += 1;
    }
    if n == nbSeqs {
        let mut bs: BlockSummary = core::mem::zeroed();
        bs.nbSequences = error(code::EXTERNALSEQUENCES_INVALID);
        return bs;
    }
    {
        let mut bs: BlockSummary = core::mem::zeroed();
        bs.nbSequences = n + 1;
        bs.blockSize = litSize + totalMatchSize;
        bs.litSize = litSize;
        bs
    }
}

unsafe fn ZSTD_compressSequencesAndLiterals_internal(
    cctx: *mut ZSTD_CCtx,
    dst: *mut c_void,
    mut dstCapacity: usize,
    mut inSeqs: *const ZSTD_Sequence,
    mut nbSequences: usize,
    mut literals: *const c_void,
    mut litSize: usize,
    srcSize: usize,
) -> usize {
    let mut remaining: usize = srcSize;
    let mut cSize: usize = 0;
    let mut op: *mut u8 = dst as *mut u8;
    let repcodeResolution: c_int =
        ((*cctx).appliedParams.searchForExternalRepcodes == ZSTD_ps_enable) as c_int;
    debug_assert!((*cctx).appliedParams.searchForExternalRepcodes != ZSTD_ps_auto);

    RETURN_ERROR_IF!(nbSequences == 0, EXTERNALSEQUENCES_INVALID);

    if (nbSequences == 1) && ((*inSeqs.add(0)).litLength == 0) {
        let cBlockHeader24: U32 = 1 + ((bt_raw) << 1);
        RETURN_ERROR_IF!(dstCapacity < 3, DSTSIZE_TOOSMALL);
        MEM_writeLE24(op as *mut c_void, cBlockHeader24);
        op = op.add(ZSTD_blockHeaderSize);
        dstCapacity -= ZSTD_blockHeaderSize;
        cSize += ZSTD_blockHeaderSize;
    }

    while nbSequences != 0 {
        let compressedSeqsSize;
        let cBlockSize: usize;
        let conversionStatus: usize;
        let block = ZSTD_get1BlockSummary(inSeqs, nbSequences);
        let lastBlock: U32 = (block.nbSequences == nbSequences) as U32;
        FORWARD_IF_ERROR!(block.nbSequences);
        debug_assert!(block.nbSequences <= nbSequences);
        RETURN_ERROR_IF!(block.litSize > litSize, EXTERNALSEQUENCES_INVALID);
        ZSTD_resetSeqStore(&mut (*cctx).seqStore);

        conversionStatus = ZSTD_convertBlockSequences(cctx, inSeqs, block.nbSequences, repcodeResolution);
        FORWARD_IF_ERROR!(conversionStatus);
        inSeqs = inSeqs.add(block.nbSequences);
        nbSequences -= block.nbSequences;
        remaining -= block.blockSize;

        RETURN_ERROR_IF!(dstCapacity < ZSTD_blockHeaderSize, DSTSIZE_TOOSMALL);

        let mut compressedSeqsSize_v = ZSTD_entropyCompressSeqStore_internal(
            op.add(ZSTD_blockHeaderSize) as *mut c_void,
            dstCapacity - ZSTD_blockHeaderSize,
            literals,
            block.litSize,
            &(*cctx).seqStore,
            &(*(*cctx).blockState.prevCBlock).entropy,
            &mut (*(*cctx).blockState.nextCBlock).entropy,
            &(*cctx).appliedParams,
            (*cctx).tmpWorkspace,
            (*cctx).tmpWkspSize,
            (*cctx).bmi2,
        );
        FORWARD_IF_ERROR!(compressedSeqsSize_v);
        if compressedSeqsSize_v > (*cctx).blockSizeMax {
            compressedSeqsSize_v = 0;
        }
        compressedSeqsSize = compressedSeqsSize_v;
        litSize -= block.litSize;
        literals = (literals as *const c_char).add(block.litSize) as *const c_void;

        if compressedSeqsSize == 0 {
            RETURN_ERROR!(CANNOTPRODUCE_UNCOMPRESSEDBLOCK);
        } else {
            let cBlockHeader: U32;
            debug_assert!(compressedSeqsSize > 1);
            ZSTD_blockState_confirmRepcodesAndEntropyTables(&mut (*cctx).blockState);
            if (*(*cctx).blockState.prevCBlock).entropy.fse.offcode_repeatMode == FSE_repeat_valid {
                (*(*cctx).blockState.prevCBlock).entropy.fse.offcode_repeatMode = FSE_repeat_check;
            }
            cBlockHeader =
                lastBlock + ((bt_compressed) << 1) + ((compressedSeqsSize << 3) as U32);
            MEM_writeLE24(op as *mut c_void, cBlockHeader);
            cBlockSize = ZSTD_blockHeaderSize + compressedSeqsSize;
        }

        cSize += cBlockSize;
        op = op.add(cBlockSize);
        dstCapacity -= cBlockSize;
        (*cctx).isFirstBlock = 0;

        if lastBlock != 0 {
            debug_assert!(nbSequences == 0);
            break;
        }
    }

    RETURN_ERROR_IF!(litSize != 0, EXTERNALSEQUENCES_INVALID);
    RETURN_ERROR_IF!(remaining != 0, EXTERNALSEQUENCES_INVALID);
    cSize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressSequencesAndLiterals(
    cctx: *mut ZSTD_CCtx,
    dst: *mut c_void,
    mut dstCapacity: usize,
    inSeqs: *const ZSTD_Sequence,
    inSeqsSize: usize,
    literals: *const c_void,
    litSize: usize,
    litCapacity: usize,
    decompressedSize: usize,
) -> usize {
    let mut op: *mut u8 = dst as *mut u8;
    let mut cSize: usize = 0;

    if litCapacity < litSize {
        RETURN_ERROR!(WORKSPACE_TOOSMALL);
    }
    FORWARD_IF_ERROR!(ZSTD_CCtx_init_compressStream2(cctx, ZSTD_e_end, decompressedSize));

    if (*cctx).appliedParams.blockDelimiters == ZSTD_sf_noBlockDelimiters {
        RETURN_ERROR!(FRAMEPARAMETER_UNSUPPORTED);
    }
    if (*cctx).appliedParams.validateSequences != 0 {
        RETURN_ERROR!(PARAMETER_UNSUPPORTED);
    }
    if (*cctx).appliedParams.fParams.checksumFlag != 0 {
        RETURN_ERROR!(FRAMEPARAMETER_UNSUPPORTED);
    }

    {
        let frameHeaderSize = ZSTD_writeFrameHeader(
            op as *mut c_void,
            dstCapacity,
            &(*cctx).appliedParams,
            decompressedSize as U64,
            (*cctx).dictID,
        );
        op = op.add(frameHeaderSize);
        debug_assert!(frameHeaderSize <= dstCapacity);
        dstCapacity -= frameHeaderSize;
        cSize += frameHeaderSize;
    }

    {
        let cBlocksSize = ZSTD_compressSequencesAndLiterals_internal(
            cctx,
            op as *mut c_void,
            dstCapacity,
            inSeqs,
            inSeqsSize,
            literals,
            litSize,
            decompressedSize,
        );
        FORWARD_IF_ERROR!(cBlocksSize);
        cSize += cBlocksSize;
        debug_assert!(cBlocksSize <= dstCapacity);
        dstCapacity -= cBlocksSize;
    }

    cSize
}

unsafe fn inBuffer_forEndFlush(zcs: *const ZSTD_CStream) -> ZSTD_inBuffer {
    let nullInput = ZSTD_inBuffer {
        src: core::ptr::null(),
        size: 0,
        pos: 0,
    };
    let stableInput = ((*zcs).appliedParams.inBufferMode == ZSTD_bm_stable) as c_int;
    if stableInput != 0 {
        (*zcs).expectedInBuffer
    } else {
        nullInput
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_flushStream(
    zcs: *mut ZSTD_CStream,
    output: *mut ZSTD_outBuffer,
) -> usize {
    let mut input = inBuffer_forEndFlush(zcs);
    input.size = input.pos;
    ZSTD_compressStream2(zcs, output, &mut input, ZSTD_e_flush)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_endStream(
    zcs: *mut ZSTD_CStream,
    output: *mut ZSTD_outBuffer,
) -> usize {
    let mut input = inBuffer_forEndFlush(zcs);
    let remainingToFlush = ZSTD_compressStream2(zcs, output, &mut input, ZSTD_e_end);
    FORWARD_IF_ERROR!(remainingToFlush);
    if (*zcs).appliedParams.nbWorkers > 0 {
        return remainingToFlush;
    }
    {
        let lastBlockSize: usize = if (*zcs).frameEnded != 0 {
            0
        } else {
            ZSTD_BLOCKHEADERSIZE
        };
        let checksumSize: usize = if (*zcs).frameEnded != 0 {
            0
        } else {
            ((*zcs).appliedParams.fParams.checksumFlag * 4) as usize
        };
        let toFlush = remainingToFlush + lastBlockSize + checksumSize;
        toFlush
    }
}

/*-=====  Pre-defined compression levels  =====-*/
static ZSTD_defaultCParameters: [[ZSTD_compressionParameters; (ZSTD_MAX_CLEVEL + 1) as usize]; 4] = {
    macro_rules! cp {
        ($w:expr,$c:expr,$h:expr,$s:expr,$l:expr,$tl:expr,$strat:expr) => {
            ZSTD_compressionParameters {
                windowLog: $w,
                chainLog: $c,
                hashLog: $h,
                searchLog: $s,
                minMatch: $l,
                targetLength: $tl,
                strategy: $strat,
            }
        };
    }
    [
        [
            cp!(19, 12, 13, 1, 6, 1, ZSTD_fast),
            cp!(19, 13, 14, 1, 7, 0, ZSTD_fast),
            cp!(20, 15, 16, 1, 6, 0, ZSTD_fast),
            cp!(21, 16, 17, 1, 5, 0, ZSTD_dfast),
            cp!(21, 18, 18, 1, 5, 0, ZSTD_dfast),
            cp!(21, 18, 19, 3, 5, 2, ZSTD_greedy),
            cp!(21, 18, 19, 3, 5, 4, ZSTD_lazy),
            cp!(21, 19, 20, 4, 5, 8, ZSTD_lazy),
            cp!(21, 19, 20, 4, 5, 16, ZSTD_lazy2),
            cp!(22, 20, 21, 4, 5, 16, ZSTD_lazy2),
            cp!(22, 21, 22, 5, 5, 16, ZSTD_lazy2),
            cp!(22, 21, 22, 6, 5, 16, ZSTD_lazy2),
            cp!(22, 22, 23, 6, 5, 32, ZSTD_lazy2),
            cp!(22, 22, 22, 4, 5, 32, ZSTD_btlazy2),
            cp!(22, 22, 23, 5, 5, 32, ZSTD_btlazy2),
            cp!(22, 23, 23, 6, 5, 32, ZSTD_btlazy2),
            cp!(22, 22, 22, 5, 5, 48, ZSTD_btopt),
            cp!(23, 23, 22, 5, 4, 64, ZSTD_btopt),
            cp!(23, 23, 22, 6, 3, 64, ZSTD_btultra),
            cp!(23, 24, 22, 7, 3, 256, ZSTD_btultra2),
            cp!(25, 25, 23, 7, 3, 256, ZSTD_btultra2),
            cp!(26, 26, 24, 7, 3, 512, ZSTD_btultra2),
            cp!(27, 27, 25, 9, 3, 999, ZSTD_btultra2),
        ],
        [
            cp!(18, 12, 13, 1, 5, 1, ZSTD_fast),
            cp!(18, 13, 14, 1, 6, 0, ZSTD_fast),
            cp!(18, 14, 14, 1, 5, 0, ZSTD_dfast),
            cp!(18, 16, 16, 1, 4, 0, ZSTD_dfast),
            cp!(18, 16, 17, 3, 5, 2, ZSTD_greedy),
            cp!(18, 17, 18, 5, 5, 2, ZSTD_greedy),
            cp!(18, 18, 19, 3, 5, 4, ZSTD_lazy),
            cp!(18, 18, 19, 4, 4, 4, ZSTD_lazy),
            cp!(18, 18, 19, 4, 4, 8, ZSTD_lazy2),
            cp!(18, 18, 19, 5, 4, 8, ZSTD_lazy2),
            cp!(18, 18, 19, 6, 4, 8, ZSTD_lazy2),
            cp!(18, 18, 19, 5, 4, 12, ZSTD_btlazy2),
            cp!(18, 19, 19, 7, 4, 12, ZSTD_btlazy2),
            cp!(18, 18, 19, 4, 4, 16, ZSTD_btopt),
            cp!(18, 18, 19, 4, 3, 32, ZSTD_btopt),
            cp!(18, 18, 19, 6, 3, 128, ZSTD_btopt),
            cp!(18, 19, 19, 6, 3, 128, ZSTD_btultra),
            cp!(18, 19, 19, 8, 3, 256, ZSTD_btultra),
            cp!(18, 19, 19, 6, 3, 128, ZSTD_btultra2),
            cp!(18, 19, 19, 8, 3, 256, ZSTD_btultra2),
            cp!(18, 19, 19, 10, 3, 512, ZSTD_btultra2),
            cp!(18, 19, 19, 12, 3, 512, ZSTD_btultra2),
            cp!(18, 19, 19, 13, 3, 999, ZSTD_btultra2),
        ],
        [
            cp!(17, 12, 12, 1, 5, 1, ZSTD_fast),
            cp!(17, 12, 13, 1, 6, 0, ZSTD_fast),
            cp!(17, 13, 15, 1, 5, 0, ZSTD_fast),
            cp!(17, 15, 16, 2, 5, 0, ZSTD_dfast),
            cp!(17, 17, 17, 2, 4, 0, ZSTD_dfast),
            cp!(17, 16, 17, 3, 4, 2, ZSTD_greedy),
            cp!(17, 16, 17, 3, 4, 4, ZSTD_lazy),
            cp!(17, 16, 17, 3, 4, 8, ZSTD_lazy2),
            cp!(17, 16, 17, 4, 4, 8, ZSTD_lazy2),
            cp!(17, 16, 17, 5, 4, 8, ZSTD_lazy2),
            cp!(17, 16, 17, 6, 4, 8, ZSTD_lazy2),
            cp!(17, 17, 17, 5, 4, 8, ZSTD_btlazy2),
            cp!(17, 18, 17, 7, 4, 12, ZSTD_btlazy2),
            cp!(17, 18, 17, 3, 4, 12, ZSTD_btopt),
            cp!(17, 18, 17, 4, 3, 32, ZSTD_btopt),
            cp!(17, 18, 17, 6, 3, 256, ZSTD_btopt),
            cp!(17, 18, 17, 6, 3, 128, ZSTD_btultra),
            cp!(17, 18, 17, 8, 3, 256, ZSTD_btultra),
            cp!(17, 18, 17, 10, 3, 512, ZSTD_btultra),
            cp!(17, 18, 17, 5, 3, 256, ZSTD_btultra2),
            cp!(17, 18, 17, 7, 3, 512, ZSTD_btultra2),
            cp!(17, 18, 17, 9, 3, 512, ZSTD_btultra2),
            cp!(17, 18, 17, 11, 3, 999, ZSTD_btultra2),
        ],
        [
            cp!(14, 12, 13, 1, 5, 1, ZSTD_fast),
            cp!(14, 14, 15, 1, 5, 0, ZSTD_fast),
            cp!(14, 14, 15, 1, 4, 0, ZSTD_fast),
            cp!(14, 14, 15, 2, 4, 0, ZSTD_dfast),
            cp!(14, 14, 14, 4, 4, 2, ZSTD_greedy),
            cp!(14, 14, 14, 3, 4, 4, ZSTD_lazy),
            cp!(14, 14, 14, 4, 4, 8, ZSTD_lazy2),
            cp!(14, 14, 14, 6, 4, 8, ZSTD_lazy2),
            cp!(14, 14, 14, 8, 4, 8, ZSTD_lazy2),
            cp!(14, 15, 14, 5, 4, 8, ZSTD_btlazy2),
            cp!(14, 15, 14, 9, 4, 8, ZSTD_btlazy2),
            cp!(14, 15, 14, 3, 4, 12, ZSTD_btopt),
            cp!(14, 15, 14, 4, 3, 24, ZSTD_btopt),
            cp!(14, 15, 14, 5, 3, 32, ZSTD_btultra),
            cp!(14, 15, 15, 6, 3, 64, ZSTD_btultra),
            cp!(14, 15, 15, 7, 3, 256, ZSTD_btultra),
            cp!(14, 15, 15, 5, 3, 48, ZSTD_btultra2),
            cp!(14, 15, 15, 6, 3, 128, ZSTD_btultra2),
            cp!(14, 15, 15, 7, 3, 256, ZSTD_btultra2),
            cp!(14, 15, 15, 8, 3, 256, ZSTD_btultra2),
            cp!(14, 15, 15, 8, 3, 512, ZSTD_btultra2),
            cp!(14, 15, 15, 9, 3, 512, ZSTD_btultra2),
            cp!(14, 15, 15, 10, 3, 999, ZSTD_btultra2),
        ],
    ]
};

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_maxCLevel() -> c_int {
    ZSTD_MAX_CLEVEL
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_minCLevel() -> c_int {
    -ZSTD_TARGETLENGTH_MAX
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_defaultCLevel() -> c_int {
    ZSTD_CLEVEL_DEFAULT
}

unsafe fn ZSTD_dedicatedDictSearch_getCParams(
    compressionLevel: c_int,
    dictSize: usize,
) -> ZSTD_compressionParameters {
    let mut cParams =
        ZSTD_getCParams_internal(compressionLevel, 0, dictSize, ZSTD_cpm_createCDict);
    match cParams.strategy {
        ZSTD_fast | ZSTD_dfast => {}
        ZSTD_greedy | ZSTD_lazy | ZSTD_lazy2 => {
            cParams.hashLog += ZSTD_LAZY_DDSS_BUCKET_LOG;
        }
        ZSTD_btlazy2 | ZSTD_btopt | ZSTD_btultra | ZSTD_btultra2 => {}
        _ => {}
    }
    cParams
}

unsafe fn ZSTD_dedicatedDictSearch_isSupported(
    cParams: *const ZSTD_compressionParameters,
) -> c_int {
    ((*cParams).strategy >= ZSTD_greedy
        && (*cParams).strategy <= ZSTD_lazy2
        && (*cParams).hashLog > (*cParams).chainLog
        && (*cParams).chainLog <= 24) as c_int
}

unsafe fn ZSTD_dedicatedDictSearch_revertCParams(cParams: *mut ZSTD_compressionParameters) {
    match (*cParams).strategy {
        ZSTD_fast | ZSTD_dfast => {}
        ZSTD_greedy | ZSTD_lazy | ZSTD_lazy2 => {
            (*cParams).hashLog -= ZSTD_LAZY_DDSS_BUCKET_LOG;
            if (*cParams).hashLog < ZSTD_HASHLOG_MIN as u32 {
                (*cParams).hashLog = ZSTD_HASHLOG_MIN as u32;
            }
        }
        ZSTD_btlazy2 | ZSTD_btopt | ZSTD_btultra | ZSTD_btultra2 => {}
        _ => {}
    }
}

unsafe fn ZSTD_getCParamRowSize(
    srcSizeHint: U64,
    mut dictSize: usize,
    mode: ZSTD_CParamMode_e,
) -> U64 {
    match mode {
        ZSTD_cpm_unknown | ZSTD_cpm_noAttachDict | ZSTD_cpm_createCDict => {}
        ZSTD_cpm_attachDict => {
            dictSize = 0;
        }
        _ => {
            debug_assert!(false);
        }
    }
    {
        let unknown = (srcSizeHint == ZSTD_CONTENTSIZE_UNKNOWN) as c_int;
        let addedSize: usize = if unknown != 0 && dictSize > 0 { 500 } else { 0 };
        if unknown != 0 && dictSize == 0 {
            ZSTD_CONTENTSIZE_UNKNOWN
        } else {
            srcSizeHint + dictSize as u64 + addedSize as u64
        }
    }
}

unsafe fn ZSTD_getCParams_internal(
    compressionLevel: c_int,
    srcSizeHint: c_ulonglong,
    dictSize: usize,
    mode: ZSTD_CParamMode_e,
) -> ZSTD_compressionParameters {
    let rSize: U64 = ZSTD_getCParamRowSize(srcSizeHint, dictSize, mode);
    let tableID: U32 = (rSize <= 256 * 1024) as U32
        + (rSize <= 128 * 1024) as U32
        + (rSize <= 16 * 1024) as U32;
    let row: c_int;

    if compressionLevel == 0 {
        row = ZSTD_CLEVEL_DEFAULT;
    } else if compressionLevel < 0 {
        row = 0;
    } else if compressionLevel > ZSTD_MAX_CLEVEL {
        row = ZSTD_MAX_CLEVEL;
    } else {
        row = compressionLevel;
    }

    {
        let mut cp = ZSTD_defaultCParameters[tableID as usize][row as usize];
        if compressionLevel < 0 {
            let clampedCompressionLevel = ZSTD_MAX_i32(ZSTD_minCLevel(), compressionLevel);
            cp.targetLength = (-clampedCompressionLevel) as u32;
        }
        ZSTD_adjustCParams_internal(cp, srcSizeHint, dictSize, mode, ZSTD_ps_auto)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_getCParams(
    compressionLevel: c_int,
    mut srcSizeHint: c_ulonglong,
    dictSize: usize,
) -> ZSTD_compressionParameters {
    if srcSizeHint == 0 {
        srcSizeHint = ZSTD_CONTENTSIZE_UNKNOWN;
    }
    ZSTD_getCParams_internal(compressionLevel, srcSizeHint, dictSize, ZSTD_cpm_unknown)
}

unsafe fn ZSTD_getParams_internal(
    compressionLevel: c_int,
    srcSizeHint: c_ulonglong,
    dictSize: usize,
    mode: ZSTD_CParamMode_e,
) -> ZSTD_parameters {
    let mut params: ZSTD_parameters = core::mem::zeroed();
    let cParams = ZSTD_getCParams_internal(compressionLevel, srcSizeHint, dictSize, mode);
    params.cParams = cParams;
    params.fParams.contentSizeFlag = 1;
    params
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_getParams(
    compressionLevel: c_int,
    mut srcSizeHint: c_ulonglong,
    dictSize: usize,
) -> ZSTD_parameters {
    if srcSizeHint == 0 {
        srcSizeHint = ZSTD_CONTENTSIZE_UNKNOWN;
    }
    ZSTD_getParams_internal(compressionLevel, srcSizeHint, dictSize, ZSTD_cpm_unknown)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_registerSequenceProducer(
    zc: *mut ZSTD_CCtx,
    extSeqProdState: *mut c_void,
    extSeqProdFunc: ZSTD_sequenceProducer_F,
) {
    ZSTD_CCtxParams_registerSequenceProducer(
        &mut (*zc).requestedParams,
        extSeqProdState,
        extSeqProdFunc,
    );
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_CCtxParams_registerSequenceProducer(
    params: *mut ZSTD_CCtx_params,
    extSeqProdState: *mut c_void,
    extSeqProdFunc: ZSTD_sequenceProducer_F,
) {
    if extSeqProdFunc.is_some() {
        (*params).extSeqProdFunc = extSeqProdFunc;
        (*params).extSeqProdState = extSeqProdState;
    } else {
        (*params).extSeqProdFunc = None;
        (*params).extSeqProdState = core::ptr::null_mut();
    }
}

