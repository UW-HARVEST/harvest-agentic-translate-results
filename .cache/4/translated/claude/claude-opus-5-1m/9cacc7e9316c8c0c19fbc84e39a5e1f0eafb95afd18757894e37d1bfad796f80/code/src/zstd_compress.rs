//! Transliteration of compress/zstd_compress.c — PART 1 (C lines 1 - 2692).
//!
//! Build configuration mirrored here:
//!   * `ZSTD_MULTITHREAD` is NOT defined  -> all MT blocks excluded, no `mtctx`.
//!   * `ZSTD_TRACE` evaluates to 0        -> no tracing.
//!   * `DYNAMIC_BMI2` is 0.
//!   * `DEBUGLEVEL` is 0                  -> assert/DEBUGLOG/RAWLOG removed.
#![allow(
    non_snake_case,
    dead_code,
    unused_mut,
    unused_variables,
    non_upper_case_globals,
    non_camel_case_types,
    unused_assignments,
    unused_parens,
    unused_unsafe,
    unreachable_patterns
)]

use core::ffi::{c_char, c_int, c_uint, c_ulonglong, c_void};
use core::ptr::{addr_of, addr_of_mut};

use crate::bits::*;
use crate::error_private::*;
use crate::fse::FSE_repeat_none;
use crate::huf::HUF_repeat_none;
use crate::mem::*;
use crate::xxhash::ZSTD_XXH64_reset;
use crate::zstd_compress_internal::*;
use crate::zstd_cwksp::*;
use crate::zstd_h::*;
use crate::zstd_internal::*;
use crate::zstd_ldm::{
    ZSTD_LDM_DEFAULT_WINDOW_LOG, ZSTD_ldm_adjustParameters, ZSTD_ldm_getMaxNbSeq,
    ZSTD_ldm_getTableSize,
};
use crate::zstdmt_compress::{ZSTDMT_JOBSIZE_MAX, ZSTDMT_NBWORKERS_MAX};

/* ---------------------------------------------------------------------------
 *  Tuning parameters
 * ------------------------------------------------------------------------ */

/// `#define ZSTD_COMPRESS_HEAPMODE 0`
pub const ZSTD_COMPRESS_HEAPMODE: c_int = 0;

/// `#define ZSTD_HASHLOG3_MAX 17`
pub const ZSTD_HASHLOG3_MAX: c_int = 17;

/// `#define ZSTD_NO_CLEVEL 0`
pub const ZSTD_NO_CLEVEL: c_int = 0;

/// `#define ZSTD_ROWSIZE 16`
pub const ZSTD_ROWSIZE: c_int = 16;

/// `#define ZSTD_INDEXOVERFLOW_MARGIN (16 MB)`
pub const ZSTD_INDEXOVERFLOW_MARGIN: U32 = 16 * (1 << 20);

/// `#define ZSTD_ROW_HASH_TAG_BITS 8` (compress/zstd_lazy.h).
/// Kept module-private so it cannot clash with `crate::zstd_lazy`.
const ZSTD_ROW_HASH_TAG_BITS: U32 = 8;

/* ---------------------------------------------------------------------------
 *  Helper functions
 * ------------------------------------------------------------------------ */

/* ZSTD_compressBound()
 * Note that the result from this function is only valid for
 * the one-pass compression functions. */
#[unsafe(no_mangle)]
pub extern "C" fn ZSTD_compressBound(srcSize: usize) -> usize {
    let r: usize = ZSTD_COMPRESSBOUND(srcSize);
    if r == 0 {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    r
}

/* ---------------------------------------------------------------------------
 *  Context memory management
 * ------------------------------------------------------------------------ */

/* `struct ZSTD_CDict_s` is declared in crate::zstd_compress_internal so that
 * every module agrees on its layout. */

#[unsafe(no_mangle)]
pub extern "C" fn ZSTD_createCCtx() -> *mut ZSTD_CCtx {
    ZSTD_createCCtx_advanced(ZSTD_defaultCMem)
}

pub unsafe fn ZSTD_initCCtx(cctx: *mut ZSTD_CCtx, memManager: ZSTD_customMem) {
    ZSTD_memset(cctx as *mut u8, 0, core::mem::size_of::<ZSTD_CCtx>());
    (*cctx).customMem = memManager;
    (*cctx).bmi2 = ZSTD_cpuSupportsBmi2();
    {
        let err: usize = ZSTD_CCtx_reset(cctx, ZSTD_reset_parameters);
        let _ = err;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ZSTD_createCCtx_advanced(customMem: ZSTD_customMem) -> *mut ZSTD_CCtx {
    if ((customMem.customAlloc.is_none() as c_int) ^ (customMem.customFree.is_none() as c_int)) != 0
    {
        return core::ptr::null_mut();
    }
    unsafe {
        let cctx: *mut ZSTD_CCtx =
            ZSTD_customMalloc(core::mem::size_of::<ZSTD_CCtx>(), customMem) as *mut ZSTD_CCtx;
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
    let mut ws: ZSTD_cwksp = ZSTD_cwksp::default();
    let cctx: *mut ZSTD_CCtx;
    if workspaceSize <= core::mem::size_of::<ZSTD_CCtx>() {
        return core::ptr::null_mut();
    }
    if ((workspace as usize) & 7) != 0 {
        return core::ptr::null_mut();
    }
    ZSTD_cwksp_init(
        &mut ws,
        workspace,
        workspaceSize,
        ZSTD_cwksp_static_alloc,
    );

    cctx = ZSTD_cwksp_reserve_object(&mut ws, core::mem::size_of::<ZSTD_CCtx>()) as *mut ZSTD_CCtx;
    if cctx.is_null() {
        return core::ptr::null_mut();
    }

    ZSTD_memset(cctx as *mut u8, 0, core::mem::size_of::<ZSTD_CCtx>());
    ZSTD_cwksp_move(addr_of_mut!((*cctx).workspace), &mut ws);
    (*cctx).staticSize = workspaceSize;

    /* statically sized space. tmpWorkspace never moves (but prev/next block swap places) */
    if ZSTD_cwksp_check_available(
        addr_of_mut!((*cctx).workspace),
        TMP_WORKSPACE_SIZE + 2 * core::mem::size_of::<ZSTD_compressedBlockState_t>(),
    ) == 0
    {
        return core::ptr::null_mut();
    }
    (*cctx).blockState.prevCBlock = ZSTD_cwksp_reserve_object(
        addr_of_mut!((*cctx).workspace),
        core::mem::size_of::<ZSTD_compressedBlockState_t>(),
    ) as *mut ZSTD_compressedBlockState_t;
    (*cctx).blockState.nextCBlock = ZSTD_cwksp_reserve_object(
        addr_of_mut!((*cctx).workspace),
        core::mem::size_of::<ZSTD_compressedBlockState_t>(),
    ) as *mut ZSTD_compressedBlockState_t;
    (*cctx).tmpWorkspace =
        ZSTD_cwksp_reserve_object(addr_of_mut!((*cctx).workspace), TMP_WORKSPACE_SIZE);
    (*cctx).tmpWkspSize = TMP_WORKSPACE_SIZE;
    /* C: cctx->bmi2 = ZSTD_cpuid_bmi2(ZSTD_cpuid());
     * DYNAMIC_BMI2 == 0 makes this value unobservable. */
    (*cctx).bmi2 = ZSTD_cpuSupportsBmi2();
    cctx
}

/**
 * Clears and frees all of the dictionaries in the CCtx.
 */
pub unsafe fn ZSTD_clearAllDicts(cctx: *mut ZSTD_CCtx) {
    ZSTD_customFree(
        (*cctx).localDict.dictBuffer as *mut u8,
        (*cctx).customMem,
    );
    crate::zstd_compress_p3::ZSTD_freeCDict((*cctx).localDict.cdict);
    ZSTD_memset(
        addr_of_mut!((*cctx).localDict) as *mut u8,
        0,
        core::mem::size_of::<ZSTD_localDict>(),
    );
    ZSTD_memset(
        addr_of_mut!((*cctx).prefixDict) as *mut u8,
        0,
        core::mem::size_of::<ZSTD_prefixDict>(),
    );
    (*cctx).cdict = core::ptr::null();
}

pub unsafe fn ZSTD_sizeof_localDict(dict: ZSTD_localDict) -> usize {
    let bufferSize: usize = if !dict.dictBuffer.is_null() {
        dict.dictSize
    } else {
        0
    };
    let cdictSize: usize = crate::zstd_compress_p3::ZSTD_sizeof_CDict(dict.cdict);
    bufferSize + cdictSize
}

pub unsafe fn ZSTD_freeCCtxContent(cctx: *mut ZSTD_CCtx) {
    ZSTD_clearAllDicts(cctx);
    ZSTD_cwksp_free(addr_of_mut!((*cctx).workspace), (*cctx).customMem);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_freeCCtx(cctx: *mut ZSTD_CCtx) -> usize {
    if cctx.is_null() {
        return 0; /* support free on NULL */
    }
    if (*cctx).staticSize != 0 {
        return ERROR(ZSTD_error_memory_allocation);
    }
    {
        let cctxInWorkspace: c_int =
            ZSTD_cwksp_owns_buffer(addr_of!((*cctx).workspace), cctx as *const c_void);
        ZSTD_freeCCtxContent(cctx);
        if cctxInWorkspace == 0 {
            ZSTD_customFree(cctx as *mut u8, (*cctx).customMem);
        }
    }
    0
}

pub unsafe fn ZSTD_sizeof_mtctx(cctx: *const ZSTD_CCtx) -> usize {
    /* ZSTD_MULTITHREAD is not defined */
    let _ = cctx;
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_sizeof_CCtx(cctx: *const ZSTD_CCtx) -> usize {
    if cctx.is_null() {
        return 0; /* support sizeof on NULL */
    }
    /* cctx may be in the workspace */
    (if (*cctx).workspace.workspace == cctx as *mut c_void {
        0
    } else {
        core::mem::size_of::<ZSTD_CCtx>()
    }) + ZSTD_cwksp_sizeof(addr_of!((*cctx).workspace))
        + ZSTD_sizeof_localDict((*cctx).localDict)
        + ZSTD_sizeof_mtctx(cctx)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_sizeof_CStream(zcs: *const ZSTD_CStream) -> usize {
    ZSTD_sizeof_CCtx(zcs) /* same object */
}

/* private API call, for dictBuilder only */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_getSeqStore(ctx: *const ZSTD_CCtx) -> *const SeqStore_t {
    addr_of!((*ctx).seqStore)
}

/* Returns true if the strategy supports using a row based matchfinder */
pub fn ZSTD_rowMatchFinderSupported(strategy: ZSTD_strategy) -> c_int {
    (strategy >= ZSTD_greedy && strategy <= ZSTD_lazy2) as c_int
}

/* Returns true if the strategy and useRowMatchFinder mode indicate that we will
 * use the row based matchfinder for this compression. */
pub fn ZSTD_rowMatchFinderUsed(strategy: ZSTD_strategy, mode: ZSTD_ParamSwitch_e) -> c_int {
    ((ZSTD_rowMatchFinderSupported(strategy) != 0) && (mode == ZSTD_ps_enable)) as c_int
}

/* Returns row matchfinder usage given an initial mode and cParams */
pub unsafe fn ZSTD_resolveRowMatchFinderMode(
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

/* Returns block splitter usage */
pub unsafe fn ZSTD_resolveBlockSplitterMode(
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

/* Returns 1 if the arguments indicate that we should allocate a chainTable, 0 otherwise */
pub fn ZSTD_allocateChainTable(
    strategy: ZSTD_strategy,
    useRowMatchFinder: ZSTD_ParamSwitch_e,
    forDDSDict: U32,
) -> c_int {
    (forDDSDict != 0
        || ((strategy != ZSTD_fast) && (ZSTD_rowMatchFinderUsed(strategy, useRowMatchFinder) == 0)))
        as c_int
}

/* Returns ZSTD_ps_enable if compression parameters are such that we should
 * enable long distance matching (wlog >= 27, strategy >= btopt). */
pub unsafe fn ZSTD_resolveEnableLdm(
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

pub fn ZSTD_resolveExternalSequenceValidation(mode: c_int) -> c_int {
    mode
}

/* Resolves maxBlockSize to the default if no value is present. */
pub fn ZSTD_resolveMaxBlockSize(maxBlockSize: usize) -> usize {
    if maxBlockSize == 0 {
        ZSTD_BLOCKSIZE_MAX as usize
    } else {
        maxBlockSize
    }
}

pub fn ZSTD_resolveExternalRepcodeSearch(
    value: ZSTD_ParamSwitch_e,
    cLevel: c_int,
) -> ZSTD_ParamSwitch_e {
    if value != ZSTD_ps_auto {
        return value;
    }
    if cLevel < 10 {
        ZSTD_ps_disable
    } else {
        ZSTD_ps_enable
    }
}

/* Returns 1 if compression parameters are such that CDict hashtable and
 * chaintable indices are tagged. */
pub unsafe fn ZSTD_CDictIndicesAreTagged(cParams: *const ZSTD_compressionParameters) -> c_int {
    ((*cParams).strategy == ZSTD_fast || (*cParams).strategy == ZSTD_dfast) as c_int
}

pub fn ZSTD_makeCCtxParamsFromCParams(
    cParams: ZSTD_compressionParameters,
) -> ZSTD_CCtx_params {
    let mut cctxParams: ZSTD_CCtx_params = ZSTD_CCtx_params::default();
    unsafe {
        /* should not matter, as all cParams are presumed properly defined */
        ZSTD_CCtxParams_init(&mut cctxParams, ZSTD_CLEVEL_DEFAULT);
        cctxParams.cParams = cParams;

        /* Adjust advanced params according to cParams */
        cctxParams.ldmParams.enableLdm =
            ZSTD_resolveEnableLdm(cctxParams.ldmParams.enableLdm, &cParams);
        if cctxParams.ldmParams.enableLdm == ZSTD_ps_enable {
            ZSTD_ldm_adjustParameters(&mut cctxParams.ldmParams, &cParams);
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
    }
    cctxParams
}

pub unsafe fn ZSTD_createCCtxParams_advanced(
    customMem: ZSTD_customMem,
) -> *mut ZSTD_CCtx_params {
    let params: *mut ZSTD_CCtx_params;
    if ((customMem.customAlloc.is_none() as c_int) ^ (customMem.customFree.is_none() as c_int)) != 0
    {
        return core::ptr::null_mut();
    }
    params = ZSTD_customCalloc(core::mem::size_of::<ZSTD_CCtx_params>(), customMem)
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
    ZSTD_customFree(params as *mut u8, (*params).customMem);
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
    if cctxParams.is_null() {
        return ERROR(ZSTD_error_GENERIC);
    }
    ZSTD_memset(
        cctxParams as *mut u8,
        0,
        core::mem::size_of::<ZSTD_CCtx_params>(),
    );
    (*cctxParams).compressionLevel = compressionLevel;
    (*cctxParams).fParams.contentSizeFlag = 1;
    0
}

/**
 * Initializes `cctxParams` from `params` and `compressionLevel`.
 */
pub unsafe fn ZSTD_CCtxParams_init_internal(
    cctxParams: *mut ZSTD_CCtx_params,
    params: *const ZSTD_parameters,
    compressionLevel: c_int,
) {
    ZSTD_memset(
        cctxParams as *mut u8,
        0,
        core::mem::size_of::<ZSTD_CCtx_params>(),
    );
    (*cctxParams).cParams = (*params).cParams;
    (*cctxParams).fParams = (*params).fParams;
    /* Should not matter, as all cParams are presumed properly defined.
     * But, set it for tracing anyway. */
    (*cctxParams).compressionLevel = compressionLevel;
    (*cctxParams).useRowMatchFinder =
        ZSTD_resolveRowMatchFinderMode((*cctxParams).useRowMatchFinder, addr_of!((*params).cParams));
    (*cctxParams).postBlockSplitter =
        ZSTD_resolveBlockSplitterMode((*cctxParams).postBlockSplitter, addr_of!((*params).cParams));
    (*cctxParams).ldmParams.enableLdm =
        ZSTD_resolveEnableLdm((*cctxParams).ldmParams.enableLdm, addr_of!((*params).cParams));
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
    if cctxParams.is_null() {
        return ERROR(ZSTD_error_GENERIC);
    }
    {
        let err_code = ZSTD_checkCParams(params.cParams);
        if ERR_isError(err_code) != 0 {
            return err_code;
        }
    }
    ZSTD_CCtxParams_init_internal(cctxParams, &params, ZSTD_NO_CLEVEL);
    0
}

/**
 * Sets cctxParams' cParams and fParams from params, but otherwise leaves them alone.
 */
pub unsafe fn ZSTD_CCtxParams_setZstdParams(
    cctxParams: *mut ZSTD_CCtx_params,
    params: *const ZSTD_parameters,
) {
    (*cctxParams).cParams = (*params).cParams;
    (*cctxParams).fParams = (*params).fParams;
    /* Should not matter, as all cParams are presumed properly defined.
     * But, set it for tracing anyway. */
    (*cctxParams).compressionLevel = ZSTD_NO_CLEVEL;
}

#[unsafe(no_mangle)]
pub extern "C" fn ZSTD_cParam_getBounds(param: ZSTD_cParameter) -> ZSTD_bounds {
    let mut bounds: ZSTD_bounds = ZSTD_bounds {
        error: 0,
        lowerBound: 0,
        upperBound: 0,
    };

    match param {
        ZSTD_c_compressionLevel => {
            bounds.lowerBound = unsafe { crate::zstd_compress_p4::ZSTD_minCLevel() };
            bounds.upperBound = unsafe { crate::zstd_compress_p4::ZSTD_maxCLevel() };
            return bounds;
        }

        ZSTD_c_windowLog => {
            bounds.lowerBound = ZSTD_WINDOWLOG_MIN;
            bounds.upperBound = ZSTD_WINDOWLOG_MAX;
            return bounds;
        }

        ZSTD_c_hashLog => {
            bounds.lowerBound = ZSTD_HASHLOG_MIN;
            bounds.upperBound = ZSTD_HASHLOG_MAX;
            return bounds;
        }

        ZSTD_c_chainLog => {
            bounds.lowerBound = ZSTD_CHAINLOG_MIN;
            bounds.upperBound = ZSTD_CHAINLOG_MAX;
            return bounds;
        }

        ZSTD_c_searchLog => {
            bounds.lowerBound = ZSTD_SEARCHLOG_MIN;
            bounds.upperBound = ZSTD_SEARCHLOG_MAX;
            return bounds;
        }

        ZSTD_c_minMatch => {
            bounds.lowerBound = ZSTD_MINMATCH_MIN;
            bounds.upperBound = ZSTD_MINMATCH_MAX;
            return bounds;
        }

        ZSTD_c_targetLength => {
            bounds.lowerBound = ZSTD_TARGETLENGTH_MIN;
            bounds.upperBound = ZSTD_TARGETLENGTH_MAX;
            return bounds;
        }

        ZSTD_c_strategy => {
            bounds.lowerBound = ZSTD_STRATEGY_MIN;
            bounds.upperBound = ZSTD_STRATEGY_MAX;
            return bounds;
        }

        ZSTD_c_contentSizeFlag => {
            bounds.lowerBound = 0;
            bounds.upperBound = 1;
            return bounds;
        }

        ZSTD_c_checksumFlag => {
            bounds.lowerBound = 0;
            bounds.upperBound = 1;
            return bounds;
        }

        ZSTD_c_dictIDFlag => {
            bounds.lowerBound = 0;
            bounds.upperBound = 1;
            return bounds;
        }

        ZSTD_c_nbWorkers => {
            bounds.lowerBound = 0;
            /* ZSTD_MULTITHREAD not defined */
            bounds.upperBound = 0;
            return bounds;
        }

        ZSTD_c_jobSize => {
            bounds.lowerBound = 0;
            /* ZSTD_MULTITHREAD not defined */
            bounds.upperBound = 0;
            return bounds;
        }

        ZSTD_c_overlapLog => {
            /* ZSTD_MULTITHREAD not defined */
            bounds.lowerBound = 0;
            bounds.upperBound = 0;
            return bounds;
        }

        ZSTD_c_enableDedicatedDictSearch => {
            bounds.lowerBound = 0;
            bounds.upperBound = 1;
            return bounds;
        }

        ZSTD_c_enableLongDistanceMatching => {
            bounds.lowerBound = ZSTD_ps_auto as c_int;
            bounds.upperBound = ZSTD_ps_disable as c_int;
            return bounds;
        }

        ZSTD_c_ldmHashLog => {
            bounds.lowerBound = ZSTD_LDM_HASHLOG_MIN;
            bounds.upperBound = ZSTD_LDM_HASHLOG_MAX;
            return bounds;
        }

        ZSTD_c_ldmMinMatch => {
            bounds.lowerBound = ZSTD_LDM_MINMATCH_MIN;
            bounds.upperBound = ZSTD_LDM_MINMATCH_MAX;
            return bounds;
        }

        ZSTD_c_ldmBucketSizeLog => {
            bounds.lowerBound = ZSTD_LDM_BUCKETSIZELOG_MIN;
            bounds.upperBound = ZSTD_LDM_BUCKETSIZELOG_MAX;
            return bounds;
        }

        ZSTD_c_ldmHashRateLog => {
            bounds.lowerBound = ZSTD_LDM_HASHRATELOG_MIN;
            bounds.upperBound = ZSTD_LDM_HASHRATELOG_MAX;
            return bounds;
        }

        /* experimental parameters */
        ZSTD_c_rsyncable => {
            bounds.lowerBound = 0;
            bounds.upperBound = 1;
            return bounds;
        }

        ZSTD_c_forceMaxWindow => {
            bounds.lowerBound = 0;
            bounds.upperBound = 1;
            return bounds;
        }

        ZSTD_c_format => {
            bounds.lowerBound = ZSTD_f_zstd1;
            bounds.upperBound = ZSTD_f_zstd1_magicless;
            return bounds;
        }

        ZSTD_c_forceAttachDict => {
            bounds.lowerBound = ZSTD_dictDefaultAttach;
            bounds.upperBound = ZSTD_dictForceLoad;
            return bounds;
        }

        ZSTD_c_literalCompressionMode => {
            bounds.lowerBound = ZSTD_ps_auto as c_int;
            bounds.upperBound = ZSTD_ps_disable as c_int;
            return bounds;
        }

        ZSTD_c_targetCBlockSize => {
            bounds.lowerBound = ZSTD_TARGETCBLOCKSIZE_MIN;
            bounds.upperBound = ZSTD_TARGETCBLOCKSIZE_MAX;
            return bounds;
        }

        ZSTD_c_srcSizeHint => {
            bounds.lowerBound = ZSTD_SRCSIZEHINT_MIN;
            bounds.upperBound = ZSTD_SRCSIZEHINT_MAX;
            return bounds;
        }

        ZSTD_c_stableInBuffer | ZSTD_c_stableOutBuffer => {
            bounds.lowerBound = ZSTD_bm_buffered as c_int;
            bounds.upperBound = ZSTD_bm_stable as c_int;
            return bounds;
        }

        ZSTD_c_blockDelimiters => {
            bounds.lowerBound = ZSTD_sf_noBlockDelimiters as c_int;
            bounds.upperBound = ZSTD_sf_explicitBlockDelimiters as c_int;
            return bounds;
        }

        ZSTD_c_validateSequences => {
            bounds.lowerBound = 0;
            bounds.upperBound = 1;
            return bounds;
        }

        ZSTD_c_splitAfterSequences => {
            bounds.lowerBound = ZSTD_ps_auto as c_int;
            bounds.upperBound = ZSTD_ps_disable as c_int;
            return bounds;
        }

        ZSTD_c_blockSplitterLevel => {
            bounds.lowerBound = 0;
            bounds.upperBound = ZSTD_BLOCKSPLITTER_LEVEL_MAX;
            return bounds;
        }

        ZSTD_c_useRowMatchFinder => {
            bounds.lowerBound = ZSTD_ps_auto as c_int;
            bounds.upperBound = ZSTD_ps_disable as c_int;
            return bounds;
        }

        ZSTD_c_deterministicRefPrefix => {
            bounds.lowerBound = 0;
            bounds.upperBound = 1;
            return bounds;
        }

        ZSTD_c_prefetchCDictTables => {
            bounds.lowerBound = ZSTD_ps_auto as c_int;
            bounds.upperBound = ZSTD_ps_disable as c_int;
            return bounds;
        }

        ZSTD_c_enableSeqProducerFallback => {
            bounds.lowerBound = 0;
            bounds.upperBound = 1;
            return bounds;
        }

        ZSTD_c_maxBlockSize => {
            bounds.lowerBound = ZSTD_BLOCKSIZE_MAX_MIN;
            bounds.upperBound = ZSTD_BLOCKSIZE_MAX as c_int;
            return bounds;
        }

        ZSTD_c_repcodeResolution => {
            bounds.lowerBound = ZSTD_ps_auto as c_int;
            bounds.upperBound = ZSTD_ps_disable as c_int;
            return bounds;
        }

        _ => {
            bounds.error = ERROR(ZSTD_error_parameter_unsupported);
            return bounds;
        }
    }
}

/* ZSTD_cParam_clampBounds:
 * Clamps the value into the bounded range.
 */
pub unsafe fn ZSTD_cParam_clampBounds(cParam: ZSTD_cParameter, value: *mut c_int) -> usize {
    let bounds: ZSTD_bounds = ZSTD_cParam_getBounds(cParam);
    if ERR_isError(bounds.error) != 0 {
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

/* `BOUNDCHECK(cParam, val)` */
macro_rules! BOUNDCHECK {
    ($cParam:expr, $val:expr) => {
        if ZSTD_cParam_withinBounds($cParam, $val) == 0 {
            return ERROR(ZSTD_error_parameter_outOfBound);
        }
    };
}

pub fn ZSTD_isUpdateAuthorized(param: ZSTD_cParameter) -> c_int {
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
            return ERROR(ZSTD_error_stage_wrong);
        }
    }

    match param {
        ZSTD_c_nbWorkers => {
            if (value != 0) && ((*cctx).staticSize != 0) {
                return ERROR(ZSTD_error_parameter_unsupported);
            }
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

        _ => return ERROR(ZSTD_error_parameter_unsupported),
    }
    ZSTD_CCtxParams_setParameter(addr_of_mut!((*cctx).requestedParams), param, value)
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
            return (*CCtxParams).format as usize;
        }

        ZSTD_c_compressionLevel => {
            {
                let err_code = ZSTD_cParam_clampBounds(param, &mut value);
                if ERR_isError(err_code) != 0 {
                    return err_code;
                }
            }
            if value == 0 {
                (*CCtxParams).compressionLevel = ZSTD_CLEVEL_DEFAULT; /* 0 == default */
            } else {
                (*CCtxParams).compressionLevel = value;
            }
            if (*CCtxParams).compressionLevel >= 0 {
                return (*CCtxParams).compressionLevel as usize;
            }
            return 0; /* return type (size_t) cannot represent negative values */
        }

        ZSTD_c_windowLog => {
            if value != 0 {
                /* 0 => use default */
                BOUNDCHECK!(ZSTD_c_windowLog, value);
            }
            (*CCtxParams).cParams.windowLog = value as U32;
            return (*CCtxParams).cParams.windowLog as usize;
        }

        ZSTD_c_hashLog => {
            if value != 0 {
                /* 0 => use default */
                BOUNDCHECK!(ZSTD_c_hashLog, value);
            }
            (*CCtxParams).cParams.hashLog = value as U32;
            return (*CCtxParams).cParams.hashLog as usize;
        }

        ZSTD_c_chainLog => {
            if value != 0 {
                /* 0 => use default */
                BOUNDCHECK!(ZSTD_c_chainLog, value);
            }
            (*CCtxParams).cParams.chainLog = value as U32;
            return (*CCtxParams).cParams.chainLog as usize;
        }

        ZSTD_c_searchLog => {
            if value != 0 {
                /* 0 => use default */
                BOUNDCHECK!(ZSTD_c_searchLog, value);
            }
            (*CCtxParams).cParams.searchLog = value as U32;
            return value as usize;
        }

        ZSTD_c_minMatch => {
            if value != 0 {
                /* 0 => use default */
                BOUNDCHECK!(ZSTD_c_minMatch, value);
            }
            (*CCtxParams).cParams.minMatch = value as U32;
            return (*CCtxParams).cParams.minMatch as usize;
        }

        ZSTD_c_targetLength => {
            BOUNDCHECK!(ZSTD_c_targetLength, value);
            (*CCtxParams).cParams.targetLength = value as U32;
            return (*CCtxParams).cParams.targetLength as usize;
        }

        ZSTD_c_strategy => {
            if value != 0 {
                /* 0 => use default */
                BOUNDCHECK!(ZSTD_c_strategy, value);
            }
            (*CCtxParams).cParams.strategy = value as ZSTD_strategy;
            return (*CCtxParams).cParams.strategy as usize;
        }

        ZSTD_c_contentSizeFlag => {
            /* Content size written in frame header _when known_ (default:1) */
            (*CCtxParams).fParams.contentSizeFlag = (value != 0) as c_int;
            return (*CCtxParams).fParams.contentSizeFlag as usize;
        }

        ZSTD_c_checksumFlag => {
            /* A 32-bits content checksum will be calculated and written at end of frame (default:0) */
            (*CCtxParams).fParams.checksumFlag = (value != 0) as c_int;
            return (*CCtxParams).fParams.checksumFlag as usize;
        }

        ZSTD_c_dictIDFlag => {
            (*CCtxParams).fParams.noDictIDFlag = (value == 0) as c_int;
            return ((*CCtxParams).fParams.noDictIDFlag == 0) as usize;
        }

        ZSTD_c_forceMaxWindow => {
            (*CCtxParams).forceWindow = (value != 0) as c_int;
            return (*CCtxParams).forceWindow as usize;
        }

        ZSTD_c_forceAttachDict => {
            let pref: ZSTD_dictAttachPref_e = value as ZSTD_dictAttachPref_e;
            BOUNDCHECK!(ZSTD_c_forceAttachDict, pref as c_int);
            (*CCtxParams).attachDictPref = pref;
            return (*CCtxParams).attachDictPref as usize;
        }

        ZSTD_c_literalCompressionMode => {
            let lcm: ZSTD_ParamSwitch_e = value as ZSTD_ParamSwitch_e;
            BOUNDCHECK!(ZSTD_c_literalCompressionMode, lcm as c_int);
            (*CCtxParams).literalCompressionMode = lcm;
            return (*CCtxParams).literalCompressionMode as usize;
        }

        ZSTD_c_nbWorkers => {
            /* ZSTD_MULTITHREAD not defined */
            if value != 0 {
                return ERROR(ZSTD_error_parameter_unsupported);
            }
            return 0;
        }

        ZSTD_c_jobSize => {
            /* ZSTD_MULTITHREAD not defined */
            if value != 0 {
                return ERROR(ZSTD_error_parameter_unsupported);
            }
            return 0;
        }

        ZSTD_c_overlapLog => {
            /* ZSTD_MULTITHREAD not defined */
            if value != 0 {
                return ERROR(ZSTD_error_parameter_unsupported);
            }
            return 0;
        }

        ZSTD_c_rsyncable => {
            /* ZSTD_MULTITHREAD not defined */
            if value != 0 {
                return ERROR(ZSTD_error_parameter_unsupported);
            }
            return 0;
        }

        ZSTD_c_enableDedicatedDictSearch => {
            (*CCtxParams).enableDedicatedDictSearch = (value != 0) as c_int;
            return (*CCtxParams).enableDedicatedDictSearch as usize;
        }

        ZSTD_c_enableLongDistanceMatching => {
            BOUNDCHECK!(ZSTD_c_enableLongDistanceMatching, value);
            (*CCtxParams).ldmParams.enableLdm = value as ZSTD_ParamSwitch_e;
            return (*CCtxParams).ldmParams.enableLdm as usize;
        }

        ZSTD_c_ldmHashLog => {
            if value != 0 {
                /* 0 ==> auto */
                BOUNDCHECK!(ZSTD_c_ldmHashLog, value);
            }
            (*CCtxParams).ldmParams.hashLog = value as U32;
            return (*CCtxParams).ldmParams.hashLog as usize;
        }

        ZSTD_c_ldmMinMatch => {
            if value != 0 {
                /* 0 ==> default */
                BOUNDCHECK!(ZSTD_c_ldmMinMatch, value);
            }
            (*CCtxParams).ldmParams.minMatchLength = value as U32;
            return (*CCtxParams).ldmParams.minMatchLength as usize;
        }

        ZSTD_c_ldmBucketSizeLog => {
            if value != 0 {
                /* 0 ==> default */
                BOUNDCHECK!(ZSTD_c_ldmBucketSizeLog, value);
            }
            (*CCtxParams).ldmParams.bucketSizeLog = value as U32;
            return (*CCtxParams).ldmParams.bucketSizeLog as usize;
        }

        ZSTD_c_ldmHashRateLog => {
            if value != 0 {
                /* 0 ==> default */
                BOUNDCHECK!(ZSTD_c_ldmHashRateLog, value);
            }
            (*CCtxParams).ldmParams.hashRateLog = value as U32;
            return (*CCtxParams).ldmParams.hashRateLog as usize;
        }

        ZSTD_c_targetCBlockSize => {
            if value != 0 {
                /* 0 ==> default */
                value = MAX(value, ZSTD_TARGETCBLOCKSIZE_MIN);
                BOUNDCHECK!(ZSTD_c_targetCBlockSize, value);
            }
            (*CCtxParams).targetCBlockSize = (value as U32) as usize;
            return (*CCtxParams).targetCBlockSize;
        }

        ZSTD_c_srcSizeHint => {
            if value != 0 {
                /* 0 ==> default */
                BOUNDCHECK!(ZSTD_c_srcSizeHint, value);
            }
            (*CCtxParams).srcSizeHint = value;
            return (*CCtxParams).srcSizeHint as usize;
        }

        ZSTD_c_stableInBuffer => {
            BOUNDCHECK!(ZSTD_c_stableInBuffer, value);
            (*CCtxParams).inBufferMode = value as ZSTD_bufferMode_e;
            return (*CCtxParams).inBufferMode as usize;
        }

        ZSTD_c_stableOutBuffer => {
            BOUNDCHECK!(ZSTD_c_stableOutBuffer, value);
            (*CCtxParams).outBufferMode = value as ZSTD_bufferMode_e;
            return (*CCtxParams).outBufferMode as usize;
        }

        ZSTD_c_blockDelimiters => {
            BOUNDCHECK!(ZSTD_c_blockDelimiters, value);
            (*CCtxParams).blockDelimiters = value as ZSTD_SequenceFormat_e;
            return (*CCtxParams).blockDelimiters as usize;
        }

        ZSTD_c_validateSequences => {
            BOUNDCHECK!(ZSTD_c_validateSequences, value);
            (*CCtxParams).validateSequences = value;
            return (*CCtxParams).validateSequences as usize;
        }

        ZSTD_c_splitAfterSequences => {
            BOUNDCHECK!(ZSTD_c_splitAfterSequences, value);
            (*CCtxParams).postBlockSplitter = value as ZSTD_ParamSwitch_e;
            return (*CCtxParams).postBlockSplitter as usize;
        }

        ZSTD_c_blockSplitterLevel => {
            BOUNDCHECK!(ZSTD_c_blockSplitterLevel, value);
            (*CCtxParams).preBlockSplitter_level = value;
            return (*CCtxParams).preBlockSplitter_level as usize;
        }

        ZSTD_c_useRowMatchFinder => {
            BOUNDCHECK!(ZSTD_c_useRowMatchFinder, value);
            (*CCtxParams).useRowMatchFinder = value as ZSTD_ParamSwitch_e;
            return (*CCtxParams).useRowMatchFinder as usize;
        }

        ZSTD_c_deterministicRefPrefix => {
            BOUNDCHECK!(ZSTD_c_deterministicRefPrefix, value);
            (*CCtxParams).deterministicRefPrefix = (value != 0) as c_int;
            return (*CCtxParams).deterministicRefPrefix as usize;
        }

        ZSTD_c_prefetchCDictTables => {
            BOUNDCHECK!(ZSTD_c_prefetchCDictTables, value);
            (*CCtxParams).prefetchCDictTables = value as ZSTD_ParamSwitch_e;
            return (*CCtxParams).prefetchCDictTables as usize;
        }

        ZSTD_c_enableSeqProducerFallback => {
            BOUNDCHECK!(ZSTD_c_enableSeqProducerFallback, value);
            (*CCtxParams).enableMatchFinderFallback = value;
            return (*CCtxParams).enableMatchFinderFallback as usize;
        }

        ZSTD_c_maxBlockSize => {
            if value != 0 {
                /* 0 ==> default */
                BOUNDCHECK!(ZSTD_c_maxBlockSize, value);
            }
            (*CCtxParams).maxBlockSize = value as usize;
            return (*CCtxParams).maxBlockSize;
        }

        ZSTD_c_repcodeResolution => {
            BOUNDCHECK!(ZSTD_c_repcodeResolution, value);
            (*CCtxParams).searchForExternalRepcodes = value as ZSTD_ParamSwitch_e;
            return (*CCtxParams).searchForExternalRepcodes as usize;
        }

        _ => return ERROR(ZSTD_error_parameter_unsupported),
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_CCtx_getParameter(
    cctx: *const ZSTD_CCtx,
    param: ZSTD_cParameter,
    value: *mut c_int,
) -> usize {
    ZSTD_CCtxParams_getParameter(addr_of!((*cctx).requestedParams), param, value)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_CCtxParams_getParameter(
    CCtxParams: *const ZSTD_CCtx_params,
    param: ZSTD_cParameter,
    value: *mut c_int,
) -> usize {
    match param {
        ZSTD_c_format => {
            *value = (*CCtxParams).format as c_int;
        }
        ZSTD_c_compressionLevel => {
            *value = (*CCtxParams).compressionLevel;
        }
        ZSTD_c_windowLog => {
            *value = (*CCtxParams).cParams.windowLog as c_int;
        }
        ZSTD_c_hashLog => {
            *value = (*CCtxParams).cParams.hashLog as c_int;
        }
        ZSTD_c_chainLog => {
            *value = (*CCtxParams).cParams.chainLog as c_int;
        }
        ZSTD_c_searchLog => {
            *value = (*CCtxParams).cParams.searchLog as c_int;
        }
        ZSTD_c_minMatch => {
            *value = (*CCtxParams).cParams.minMatch as c_int;
        }
        ZSTD_c_targetLength => {
            *value = (*CCtxParams).cParams.targetLength as c_int;
        }
        ZSTD_c_strategy => {
            *value = (*CCtxParams).cParams.strategy as c_int;
        }
        ZSTD_c_contentSizeFlag => {
            *value = (*CCtxParams).fParams.contentSizeFlag;
        }
        ZSTD_c_checksumFlag => {
            *value = (*CCtxParams).fParams.checksumFlag;
        }
        ZSTD_c_dictIDFlag => {
            *value = ((*CCtxParams).fParams.noDictIDFlag == 0) as c_int;
        }
        ZSTD_c_forceMaxWindow => {
            *value = (*CCtxParams).forceWindow;
        }
        ZSTD_c_forceAttachDict => {
            *value = (*CCtxParams).attachDictPref as c_int;
        }
        ZSTD_c_literalCompressionMode => {
            *value = (*CCtxParams).literalCompressionMode as c_int;
        }
        ZSTD_c_nbWorkers => {
            /* ZSTD_MULTITHREAD not defined: nbWorkers is always 0 */
            *value = (*CCtxParams).nbWorkers;
        }
        ZSTD_c_jobSize => {
            /* ZSTD_MULTITHREAD not defined */
            return ERROR(ZSTD_error_parameter_unsupported);
        }
        ZSTD_c_overlapLog => {
            /* ZSTD_MULTITHREAD not defined */
            return ERROR(ZSTD_error_parameter_unsupported);
        }
        ZSTD_c_rsyncable => {
            /* ZSTD_MULTITHREAD not defined */
            return ERROR(ZSTD_error_parameter_unsupported);
        }
        ZSTD_c_enableDedicatedDictSearch => {
            *value = (*CCtxParams).enableDedicatedDictSearch;
        }
        ZSTD_c_enableLongDistanceMatching => {
            *value = (*CCtxParams).ldmParams.enableLdm as c_int;
        }
        ZSTD_c_ldmHashLog => {
            *value = (*CCtxParams).ldmParams.hashLog as c_int;
        }
        ZSTD_c_ldmMinMatch => {
            *value = (*CCtxParams).ldmParams.minMatchLength as c_int;
        }
        ZSTD_c_ldmBucketSizeLog => {
            *value = (*CCtxParams).ldmParams.bucketSizeLog as c_int;
        }
        ZSTD_c_ldmHashRateLog => {
            *value = (*CCtxParams).ldmParams.hashRateLog as c_int;
        }
        ZSTD_c_targetCBlockSize => {
            *value = (*CCtxParams).targetCBlockSize as c_int;
        }
        ZSTD_c_srcSizeHint => {
            *value = (*CCtxParams).srcSizeHint as c_int;
        }
        ZSTD_c_stableInBuffer => {
            *value = (*CCtxParams).inBufferMode as c_int;
        }
        ZSTD_c_stableOutBuffer => {
            *value = (*CCtxParams).outBufferMode as c_int;
        }
        ZSTD_c_blockDelimiters => {
            *value = (*CCtxParams).blockDelimiters as c_int;
        }
        ZSTD_c_validateSequences => {
            *value = (*CCtxParams).validateSequences as c_int;
        }
        ZSTD_c_splitAfterSequences => {
            *value = (*CCtxParams).postBlockSplitter as c_int;
        }
        ZSTD_c_blockSplitterLevel => {
            *value = (*CCtxParams).preBlockSplitter_level;
        }
        ZSTD_c_useRowMatchFinder => {
            *value = (*CCtxParams).useRowMatchFinder as c_int;
        }
        ZSTD_c_deterministicRefPrefix => {
            *value = (*CCtxParams).deterministicRefPrefix as c_int;
        }
        ZSTD_c_prefetchCDictTables => {
            *value = (*CCtxParams).prefetchCDictTables as c_int;
        }
        ZSTD_c_enableSeqProducerFallback => {
            *value = (*CCtxParams).enableMatchFinderFallback;
        }
        ZSTD_c_maxBlockSize => {
            *value = (*CCtxParams).maxBlockSize as c_int;
        }
        ZSTD_c_repcodeResolution => {
            *value = (*CCtxParams).searchForExternalRepcodes as c_int;
        }
        _ => return ERROR(ZSTD_error_parameter_unsupported),
    }
    0
}

/** ZSTD_CCtx_setParametersUsingCCtxParams() :
 *  just applies `params` into `cctx`
 *  no action is performed, parameters are merely stored. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_CCtx_setParametersUsingCCtxParams(
    cctx: *mut ZSTD_CCtx,
    params: *const ZSTD_CCtx_params,
) -> usize {
    if (*cctx).streamStage != zcss_init {
        return ERROR(ZSTD_error_stage_wrong);
    }
    if !(*cctx).cdict.is_null() {
        return ERROR(ZSTD_error_stage_wrong);
    }

    (*cctx).requestedParams = *params;
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_CCtx_setCParams(
    cctx: *mut ZSTD_CCtx,
    cparams: ZSTD_compressionParameters,
) -> usize {
    /* only update if all parameters are valid */
    {
        let err_code = ZSTD_checkCParams(cparams);
        if ERR_isError(err_code) != 0 {
            return err_code;
        }
    }
    {
        let err_code = ZSTD_CCtx_setParameter(cctx, ZSTD_c_windowLog, cparams.windowLog as c_int);
        if ERR_isError(err_code) != 0 {
            return err_code;
        }
    }
    {
        let err_code = ZSTD_CCtx_setParameter(cctx, ZSTD_c_chainLog, cparams.chainLog as c_int);
        if ERR_isError(err_code) != 0 {
            return err_code;
        }
    }
    {
        let err_code = ZSTD_CCtx_setParameter(cctx, ZSTD_c_hashLog, cparams.hashLog as c_int);
        if ERR_isError(err_code) != 0 {
            return err_code;
        }
    }
    {
        let err_code = ZSTD_CCtx_setParameter(cctx, ZSTD_c_searchLog, cparams.searchLog as c_int);
        if ERR_isError(err_code) != 0 {
            return err_code;
        }
    }
    {
        let err_code = ZSTD_CCtx_setParameter(cctx, ZSTD_c_minMatch, cparams.minMatch as c_int);
        if ERR_isError(err_code) != 0 {
            return err_code;
        }
    }
    {
        let err_code =
            ZSTD_CCtx_setParameter(cctx, ZSTD_c_targetLength, cparams.targetLength as c_int);
        if ERR_isError(err_code) != 0 {
            return err_code;
        }
    }
    {
        let err_code = ZSTD_CCtx_setParameter(cctx, ZSTD_c_strategy, cparams.strategy as c_int);
        if ERR_isError(err_code) != 0 {
            return err_code;
        }
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_CCtx_setFParams(
    cctx: *mut ZSTD_CCtx,
    fparams: ZSTD_frameParameters,
) -> usize {
    {
        let err_code = ZSTD_CCtx_setParameter(
            cctx,
            ZSTD_c_contentSizeFlag,
            (fparams.contentSizeFlag != 0) as c_int,
        );
        if ERR_isError(err_code) != 0 {
            return err_code;
        }
    }
    {
        let err_code = ZSTD_CCtx_setParameter(
            cctx,
            ZSTD_c_checksumFlag,
            (fparams.checksumFlag != 0) as c_int,
        );
        if ERR_isError(err_code) != 0 {
            return err_code;
        }
    }
    {
        let err_code = ZSTD_CCtx_setParameter(
            cctx,
            ZSTD_c_dictIDFlag,
            (fparams.noDictIDFlag == 0) as c_int,
        );
        if ERR_isError(err_code) != 0 {
            return err_code;
        }
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_CCtx_setParams(
    cctx: *mut ZSTD_CCtx,
    params: ZSTD_parameters,
) -> usize {
    /* First check cParams, because we want to update all or none. */
    {
        let err_code = ZSTD_checkCParams(params.cParams);
        if ERR_isError(err_code) != 0 {
            return err_code;
        }
    }
    /* Next set fParams, because this could fail if the cctx isn't in init stage. */
    {
        let err_code = ZSTD_CCtx_setFParams(cctx, params.fParams);
        if ERR_isError(err_code) != 0 {
            return err_code;
        }
    }
    /* Finally set cParams, which should succeed. */
    {
        let err_code = ZSTD_CCtx_setCParams(cctx, params.cParams);
        if ERR_isError(err_code) != 0 {
            return err_code;
        }
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_CCtx_setPledgedSrcSize(
    cctx: *mut ZSTD_CCtx,
    pledgedSrcSize: c_ulonglong,
) -> usize {
    if (*cctx).streamStage != zcss_init {
        return ERROR(ZSTD_error_stage_wrong);
    }
    (*cctx).pledgedSrcSizePlusOne = pledgedSrcSize.wrapping_add(1);
    0
}

/**
 * Initializes the local dictionary using requested parameters.
 * NOTE: Initialization does not employ the pledged src size,
 * because the dictionary may be used for multiple compressions.
 */
pub unsafe fn ZSTD_initLocalDict(cctx: *mut ZSTD_CCtx) -> usize {
    let dl: *mut ZSTD_localDict = addr_of_mut!((*cctx).localDict);
    if (*dl).dict.is_null() {
        /* No local dictionary. */
        return 0;
    }
    if !(*dl).cdict.is_null() {
        /* Local dictionary already initialized. */
        return 0;
    }

    (*dl).cdict = crate::zstd_compress_p3::ZSTD_createCDict_advanced2(
        (*dl).dict,
        (*dl).dictSize,
        ZSTD_dlm_byRef,
        (*dl).dictContentType,
        addr_of!((*cctx).requestedParams),
        (*cctx).customMem,
    );
    if (*dl).cdict.is_null() {
        return ERROR(ZSTD_error_memory_allocation);
    }
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
    if (*cctx).streamStage != zcss_init {
        return ERROR(ZSTD_error_stage_wrong);
    }
    ZSTD_clearAllDicts(cctx); /* erase any previously set dictionary */
    if dict.is_null() || dictSize == 0 {
        /* no dictionary */
        return 0;
    }
    if dictLoadMethod == ZSTD_dlm_byRef {
        (*cctx).localDict.dict = dict;
    } else {
        /* copy dictionary content inside CCtx to own its lifetime */
        let dictBuffer: *mut c_void;
        if (*cctx).staticSize != 0 {
            return ERROR(ZSTD_error_memory_allocation);
        }
        dictBuffer = ZSTD_customMalloc(dictSize, (*cctx).customMem) as *mut c_void;
        if dictBuffer.is_null() {
            return ERROR(ZSTD_error_memory_allocation);
        }
        ZSTD_memcpy(dictBuffer as *mut u8, dict as *const u8, dictSize);
        (*cctx).localDict.dictBuffer = dictBuffer; /* owned ptr to free */
        (*cctx).localDict.dict = dictBuffer as *const c_void; /* read-only reference */
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
    if (*cctx).streamStage != zcss_init {
        return ERROR(ZSTD_error_stage_wrong);
    }
    /* Free the existing local cdict (if any) to save memory. */
    ZSTD_clearAllDicts(cctx);
    (*cctx).cdict = cdict;
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_CCtx_refThreadPool(
    cctx: *mut ZSTD_CCtx,
    pool: *mut ZSTD_threadPool,
) -> usize {
    if (*cctx).streamStage != zcss_init {
        return ERROR(ZSTD_error_stage_wrong);
    }
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
    if (*cctx).streamStage != zcss_init {
        return ERROR(ZSTD_error_stage_wrong);
    }
    ZSTD_clearAllDicts(cctx);
    if !prefix.is_null() && prefixSize > 0 {
        (*cctx).prefixDict.dict = prefix;
        (*cctx).prefixDict.dictSize = prefixSize;
        (*cctx).prefixDict.dictContentType = dictContentType;
    }
    0
}

/* ZSTD_CCtx_reset() :
 *  Also dumps dictionary */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_CCtx_reset(
    cctx: *mut ZSTD_CCtx,
    reset: ZSTD_ResetDirective,
) -> usize {
    if (reset == ZSTD_reset_session_only) || (reset == ZSTD_reset_session_and_parameters) {
        (*cctx).streamStage = zcss_init;
        (*cctx).pledgedSrcSizePlusOne = 0;
    }
    if (reset == ZSTD_reset_parameters) || (reset == ZSTD_reset_session_and_parameters) {
        if (*cctx).streamStage != zcss_init {
            return ERROR(ZSTD_error_stage_wrong);
        }
        ZSTD_clearAllDicts(cctx);
        return ZSTD_CCtxParams_reset(addr_of_mut!((*cctx).requestedParams));
    }
    0
}

/** ZSTD_checkCParams() :
    control CParam values remain within authorized range.
    @return : 0, or an error code if one value is beyond authorized range */
#[unsafe(no_mangle)]
pub extern "C" fn ZSTD_checkCParams(cParams: ZSTD_compressionParameters) -> usize {
    BOUNDCHECK!(ZSTD_c_windowLog, cParams.windowLog as c_int);
    BOUNDCHECK!(ZSTD_c_chainLog, cParams.chainLog as c_int);
    BOUNDCHECK!(ZSTD_c_hashLog, cParams.hashLog as c_int);
    BOUNDCHECK!(ZSTD_c_searchLog, cParams.searchLog as c_int);
    BOUNDCHECK!(ZSTD_c_minMatch, cParams.minMatch as c_int);
    BOUNDCHECK!(ZSTD_c_targetLength, cParams.targetLength as c_int);
    BOUNDCHECK!(ZSTD_c_strategy, cParams.strategy as c_int);
    0
}

/** ZSTD_clampCParams() :
 *  make CParam values within valid range.
 *  @return : valid CParams */
pub fn ZSTD_clampCParams(mut cParams: ZSTD_compressionParameters) -> ZSTD_compressionParameters {
    /* CLAMP(cParam, val) */
    {
        let bounds: ZSTD_bounds = ZSTD_cParam_getBounds(ZSTD_c_windowLog);
        if (cParams.windowLog as c_int) < bounds.lowerBound {
            cParams.windowLog = bounds.lowerBound as c_uint;
        } else if (cParams.windowLog as c_int) > bounds.upperBound {
            cParams.windowLog = bounds.upperBound as c_uint;
        }
    }
    {
        let bounds: ZSTD_bounds = ZSTD_cParam_getBounds(ZSTD_c_chainLog);
        if (cParams.chainLog as c_int) < bounds.lowerBound {
            cParams.chainLog = bounds.lowerBound as c_uint;
        } else if (cParams.chainLog as c_int) > bounds.upperBound {
            cParams.chainLog = bounds.upperBound as c_uint;
        }
    }
    {
        let bounds: ZSTD_bounds = ZSTD_cParam_getBounds(ZSTD_c_hashLog);
        if (cParams.hashLog as c_int) < bounds.lowerBound {
            cParams.hashLog = bounds.lowerBound as c_uint;
        } else if (cParams.hashLog as c_int) > bounds.upperBound {
            cParams.hashLog = bounds.upperBound as c_uint;
        }
    }
    {
        let bounds: ZSTD_bounds = ZSTD_cParam_getBounds(ZSTD_c_searchLog);
        if (cParams.searchLog as c_int) < bounds.lowerBound {
            cParams.searchLog = bounds.lowerBound as c_uint;
        } else if (cParams.searchLog as c_int) > bounds.upperBound {
            cParams.searchLog = bounds.upperBound as c_uint;
        }
    }
    {
        let bounds: ZSTD_bounds = ZSTD_cParam_getBounds(ZSTD_c_minMatch);
        if (cParams.minMatch as c_int) < bounds.lowerBound {
            cParams.minMatch = bounds.lowerBound as c_uint;
        } else if (cParams.minMatch as c_int) > bounds.upperBound {
            cParams.minMatch = bounds.upperBound as c_uint;
        }
    }
    {
        let bounds: ZSTD_bounds = ZSTD_cParam_getBounds(ZSTD_c_targetLength);
        if (cParams.targetLength as c_int) < bounds.lowerBound {
            cParams.targetLength = bounds.lowerBound as c_uint;
        } else if (cParams.targetLength as c_int) > bounds.upperBound {
            cParams.targetLength = bounds.upperBound as c_uint;
        }
    }
    /* CLAMP_TYPE(ZSTD_c_strategy, cParams.strategy, ZSTD_strategy) */
    {
        let bounds: ZSTD_bounds = ZSTD_cParam_getBounds(ZSTD_c_strategy);
        if (cParams.strategy as c_int) < bounds.lowerBound {
            cParams.strategy = bounds.lowerBound as ZSTD_strategy;
        } else if (cParams.strategy as c_int) > bounds.upperBound {
            cParams.strategy = bounds.upperBound as ZSTD_strategy;
        }
    }
    cParams
}

/** ZSTD_cycleLog() :
 *  condition for correct operation : hashLog > 1 */
#[unsafe(no_mangle)]
pub extern "C" fn ZSTD_cycleLog(hashLog: U32, strat: ZSTD_strategy) -> U32 {
    let btScale: U32 = ((strat as U32) >= (ZSTD_btlazy2 as U32)) as U32;
    hashLog.wrapping_sub(btScale)
}

/** ZSTD_dictAndWindowLog() :
 * Returns an adjusted window log that is large enough to fit the source and the dictionary.
 * NOTE: srcSize must not be ZSTD_CONTENTSIZE_UNKNOWN.
 */
pub fn ZSTD_dictAndWindowLog(windowLog: U32, srcSize: U64, dictSize: U64) -> U32 {
    let maxWindowSize: U64 = 1u64 << ZSTD_WINDOWLOG_MAX;
    /* No dictionary ==> No change */
    if dictSize == 0 {
        return windowLog;
    }
    {
        let windowSize: U64 = 1u64 << windowLog;
        let dictAndWindowSize: U64 = dictSize.wrapping_add(windowSize);
        /* If the window size is already large enough to fit both the source and
         * the dictionary then just use the window size. */
        if windowSize >= dictSize.wrapping_add(srcSize) {
            return windowLog; /* Window size large enough already */
        } else if dictAndWindowSize >= maxWindowSize {
            return ZSTD_WINDOWLOG_MAX as U32; /* Larger than max window log */
        } else {
            return ZSTD_highbit32((dictAndWindowSize as U32).wrapping_sub(1)) + 1;
        }
    }
}

/** ZSTD_adjustCParams_internal() :
 *  optimize `cPar` for a specified input (`srcSize` and `dictSize`). */
pub fn ZSTD_adjustCParams_internal(
    mut cPar: ZSTD_compressionParameters,
    mut srcSize: c_ulonglong,
    mut dictSize: usize,
    mode: ZSTD_CParamMode_e,
    mut useRowMatchFinder: ZSTD_ParamSwitch_e,
) -> ZSTD_compressionParameters {
    let minSrcSize: U64 = 513; /* (1<<9) + 1 */
    let maxWindowResize: U64 = 1u64 << (ZSTD_WINDOWLOG_MAX - 1);

    /* No ZSTD_EXCLUDE_*_BLOCK_COMPRESSOR macros are defined. */

    match mode {
        ZSTD_cpm_unknown | ZSTD_cpm_noAttachDict => {
            /* If we don't know the source size, don't make any
             * assumptions about it. */
        }
        ZSTD_cpm_createCDict => {
            /* Assume a small source size when creating a dictionary
             * with an unknown source size. */
            if dictSize != 0 && srcSize == ZSTD_CONTENTSIZE_UNKNOWN {
                srcSize = minSrcSize;
            }
        }
        ZSTD_cpm_attachDict => {
            /* Dictionary has its own dedicated parameters which have
             * already been selected. */
            dictSize = 0;
        }
        _ => {}
    }

    /* resize windowLog if input is small enough, to use less memory */
    if (srcSize <= maxWindowResize) && ((dictSize as U64) <= maxWindowResize) {
        let tSize: U32 = srcSize.wrapping_add(dictSize as U64) as U32;
        const hashSizeMin: U32 = 1u32 << ZSTD_HASHLOG_MIN;
        let srcLog: U32 = if tSize < hashSizeMin {
            ZSTD_HASHLOG_MIN as U32
        } else {
            ZSTD_highbit32(tSize.wrapping_sub(1)) + 1
        };
        if cPar.windowLog > srcLog {
            cPar.windowLog = srcLog;
        }
    }
    if srcSize != ZSTD_CONTENTSIZE_UNKNOWN {
        let dictAndWindowLog: U32 =
            ZSTD_dictAndWindowLog(cPar.windowLog, srcSize as U64, dictSize as U64);
        let cycleLog: U32 = ZSTD_cycleLog(cPar.chainLog, cPar.strategy);
        if cPar.hashLog > dictAndWindowLog.wrapping_add(1) {
            cPar.hashLog = dictAndWindowLog.wrapping_add(1);
        }
        if cycleLog > dictAndWindowLog {
            cPar.chainLog = cPar
                .chainLog
                .wrapping_sub(cycleLog.wrapping_sub(dictAndWindowLog));
        }
    }

    if cPar.windowLog < ZSTD_WINDOWLOG_ABSOLUTEMIN {
        cPar.windowLog = ZSTD_WINDOWLOG_ABSOLUTEMIN; /* minimum wlog required for valid frame header */
    }

    /* We can't use more than 32 bits of hash in total, so that means that we require:
     * (hashLog + 8) <= 32 && (chainLog + 8) <= 32
     */
    if mode == ZSTD_cpm_createCDict && unsafe { ZSTD_CDictIndicesAreTagged(&cPar) } != 0 {
        let maxShortCacheHashLog: U32 = 32 - ZSTD_SHORT_CACHE_TAG_BITS;
        if cPar.hashLog > maxShortCacheHashLog {
            cPar.hashLog = maxShortCacheHashLog;
        }
        if cPar.chainLog > maxShortCacheHashLog {
            cPar.chainLog = maxShortCacheHashLog;
        }
    }

    /* At this point, we aren't 100% sure if we are using the row match finder.
     * Unless it is explicitly disabled, conservatively assume that it is enabled. */
    if useRowMatchFinder == ZSTD_ps_auto {
        useRowMatchFinder = ZSTD_ps_enable;
    }

    /* We can't hash more than 32-bits in total. So that means that we require:
     * (hashLog - rowLog + 8) <= 32
     */
    if ZSTD_rowMatchFinderUsed(cPar.strategy, useRowMatchFinder) != 0 {
        /* Switch to 32-entry rows if searchLog is 5 (or more) */
        let rowLog: U32 = MAX(4u32, MIN(cPar.searchLog, 6u32));
        let maxRowHashLog: U32 = 32 - ZSTD_ROW_HASH_TAG_BITS;
        let maxHashLog: U32 = maxRowHashLog.wrapping_add(rowLog);
        if cPar.hashLog > maxHashLog {
            cPar.hashLog = maxHashLog;
        }
    }

    cPar
}

#[unsafe(no_mangle)]
pub extern "C" fn ZSTD_adjustCParams(
    mut cPar: ZSTD_compressionParameters,
    mut srcSize: c_ulonglong,
    dictSize: usize,
) -> ZSTD_compressionParameters {
    cPar = ZSTD_clampCParams(cPar); /* resulting cPar is necessarily valid (all parameters within range) */
    if srcSize == 0 {
        srcSize = ZSTD_CONTENTSIZE_UNKNOWN;
    }
    ZSTD_adjustCParams_internal(cPar, srcSize, dictSize, ZSTD_cpm_unknown, ZSTD_ps_auto)
}

pub unsafe fn ZSTD_overrideCParams(
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
        srcSizeHint = (*CCtxParams).srcSizeHint as U64;
    }
    cParams = crate::zstd_compress_p4::ZSTD_getCParams_internal(
        (*CCtxParams).compressionLevel,
        srcSizeHint,
        dictSize,
        mode,
    );
    if (*CCtxParams).ldmParams.enableLdm == ZSTD_ps_enable {
        cParams.windowLog = ZSTD_LDM_DEFAULT_WINDOW_LOG as c_uint;
    }
    ZSTD_overrideCParams(&mut cParams, addr_of!((*CCtxParams).cParams));
    /* srcSizeHint == 0 means 0 */
    ZSTD_adjustCParams_internal(
        cParams,
        srcSizeHint,
        dictSize,
        mode,
        (*CCtxParams).useRowMatchFinder,
    )
}

pub unsafe fn ZSTD_sizeof_matchState(
    cParams: *const ZSTD_compressionParameters,
    useRowMatchFinder: ZSTD_ParamSwitch_e,
    enableDedicatedDictSearch: c_int,
    forCCtx: U32,
) -> usize {
    /* chain table size should be 0 for fast or row-hash strategies */
    let chainSize: usize = if ZSTD_allocateChainTable(
        (*cParams).strategy,
        useRowMatchFinder,
        ((enableDedicatedDictSearch != 0) && (forCCtx == 0)) as U32,
    ) != 0
    {
        (1usize) << (*cParams).chainLog
    } else {
        0
    };
    let hSize: usize = (1usize) << (*cParams).hashLog;
    let hashLog3: U32 = if (forCCtx != 0) && (*cParams).minMatch == 3 {
        MIN(ZSTD_HASHLOG3_MAX as U32, (*cParams).windowLog)
    } else {
        0
    };
    let h3Size: usize = if hashLog3 != 0 {
        (1usize) << hashLog3
    } else {
        0
    };
    /* We don't use ZSTD_cwksp_alloc_size() here because the tables aren't
     * surrounded by redzones in ASAN. */
    let tableSpace: usize = chainSize * core::mem::size_of::<U32>()
        + hSize * core::mem::size_of::<U32>()
        + h3Size * core::mem::size_of::<U32>();
    let optPotentialSpace: usize = ZSTD_cwksp_aligned64_alloc_size(
        (MaxML as usize + 1) * core::mem::size_of::<U32>(),
    ) + ZSTD_cwksp_aligned64_alloc_size(
        (MaxLL as usize + 1) * core::mem::size_of::<U32>(),
    ) + ZSTD_cwksp_aligned64_alloc_size(
        (MaxOff as usize + 1) * core::mem::size_of::<U32>(),
    ) + ZSTD_cwksp_aligned64_alloc_size(
        (1usize << Litbits) * core::mem::size_of::<U32>(),
    ) + ZSTD_cwksp_aligned64_alloc_size(
        ZSTD_OPT_SIZE * core::mem::size_of::<ZSTD_match_t>(),
    ) + ZSTD_cwksp_aligned64_alloc_size(
        ZSTD_OPT_SIZE * core::mem::size_of::<ZSTD_optimal_t>(),
    );
    let lazyAdditionalSpace: usize =
        if ZSTD_rowMatchFinderUsed((*cParams).strategy, useRowMatchFinder) != 0 {
            ZSTD_cwksp_aligned64_alloc_size(hSize)
        } else {
            0
        };
    let optSpace: usize = if (forCCtx != 0) && ((*cParams).strategy >= ZSTD_btopt) {
        optPotentialSpace
    } else {
        0
    };
    let slackSpace: usize = ZSTD_cwksp_slack_space_required();

    tableSpace + optSpace + slackSpace + lazyAdditionalSpace
}

/* Helper function for calculating memory requirements.
 * Gives a tighter bound than ZSTD_sequenceBound() by taking minMatch into account. */
pub fn ZSTD_maxNbSeq(blockSize: usize, minMatch: c_uint, useSequenceProducer: c_int) -> usize {
    let divider: U32 = if minMatch == 3 || useSequenceProducer != 0 {
        3
    } else {
        4
    };
    blockSize / (divider as usize)
}

pub unsafe fn ZSTD_estimateCCtxSize_usingCCtxParams_internal(
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
    let windowSize: usize =
        MAX(1u64, MIN(1u64 << (*cParams).windowLog, pledgedSrcSize)) as usize;
    let blockSize: usize = MIN(ZSTD_resolveMaxBlockSize(maxBlockSize), windowSize);
    let maxNbSeq: usize = ZSTD_maxNbSeq(blockSize, (*cParams).minMatch, useSequenceProducer);
    let tokenSpace: usize =
        ZSTD_cwksp_alloc_size((WILDCOPY_OVERLENGTH as usize).wrapping_add(blockSize))
            + ZSTD_cwksp_aligned64_alloc_size(maxNbSeq * core::mem::size_of::<SeqDef>())
            + 3 * ZSTD_cwksp_alloc_size(maxNbSeq * core::mem::size_of::<BYTE>());
    let tmpWorkSpace: usize = ZSTD_cwksp_alloc_size(TMP_WORKSPACE_SIZE);
    let blockStateSpace: usize =
        2 * ZSTD_cwksp_alloc_size(core::mem::size_of::<ZSTD_compressedBlockState_t>());
    let matchStateSize: usize = ZSTD_sizeof_matchState(
        cParams,
        useRowMatchFinder,
        /* enableDedicatedDictSearch */ 0,
        /* forCCtx */ 1,
    );

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

    let maxNbExternalSeq: usize = crate::zstd_compress_p2::ZSTD_sequenceBound(blockSize);
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
    let cParams: ZSTD_compressionParameters = ZSTD_getCParamsFromCCtxParams(
        params,
        ZSTD_CONTENTSIZE_UNKNOWN,
        0,
        ZSTD_cpm_noAttachDict,
    );
    let useRowMatchFinder: ZSTD_ParamSwitch_e =
        ZSTD_resolveRowMatchFinderMode((*params).useRowMatchFinder, &cParams);

    if (*params).nbWorkers > 0 {
        return ERROR(ZSTD_error_GENERIC);
    }
    /* estimateCCtxSize is for one-shot compression. So no buffers should
     * be needed. However, we still allocate two 0-sized buffers, which can
     * take space under ASAN. */
    ZSTD_estimateCCtxSize_usingCCtxParams_internal(
        &cParams,
        addr_of!((*params).ldmParams),
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
pub extern "C" fn ZSTD_estimateCCtxSize_usingCParams(
    cParams: ZSTD_compressionParameters,
) -> usize {
    let mut initialParams: ZSTD_CCtx_params = ZSTD_makeCCtxParamsFromCParams(cParams);
    unsafe {
        if ZSTD_rowMatchFinderSupported(cParams.strategy) != 0 {
            /* Pick bigger of not using and using row-based matchfinder for greedy and lazy strategies */
            let noRowCCtxSize: usize;
            let rowCCtxSize: usize;
            initialParams.useRowMatchFinder = ZSTD_ps_disable;
            noRowCCtxSize = ZSTD_estimateCCtxSize_usingCCtxParams(&initialParams);
            initialParams.useRowMatchFinder = ZSTD_ps_enable;
            rowCCtxSize = ZSTD_estimateCCtxSize_usingCCtxParams(&initialParams);
            MAX(noRowCCtxSize, rowCCtxSize)
        } else {
            ZSTD_estimateCCtxSize_usingCCtxParams(&initialParams)
        }
    }
}

pub static srcSizeTiers: [c_ulonglong; 4] = [
    16 * (1 << 10),
    128 * (1 << 10),
    256 * (1 << 10),
    ZSTD_CONTENTSIZE_UNKNOWN,
];

pub fn ZSTD_estimateCCtxSize_internal(compressionLevel: c_int) -> usize {
    let mut tier: c_int = 0;
    let mut largestSize: usize = 0;
    while tier < 4 {
        /* Choose the set of cParams for a given level across all srcSizes that give the largest cctxSize */
        let cParams: ZSTD_compressionParameters = unsafe {
            crate::zstd_compress_p4::ZSTD_getCParams_internal(
                compressionLevel,
                srcSizeTiers[tier as usize],
                0,
                ZSTD_cpm_noAttachDict,
            )
        };
        largestSize = MAX(ZSTD_estimateCCtxSize_usingCParams(cParams), largestSize);
        tier += 1;
    }
    largestSize
}

#[unsafe(no_mangle)]
pub extern "C" fn ZSTD_estimateCCtxSize(compressionLevel: c_int) -> usize {
    let mut level: c_int;
    let mut memBudget: usize = 0;
    level = MIN(compressionLevel, 1);
    while level <= compressionLevel {
        /* Ensure monotonically increasing memory usage as compression level increases */
        let newMB: usize = ZSTD_estimateCCtxSize_internal(level);
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
    if (*params).nbWorkers > 0 {
        return ERROR(ZSTD_error_GENERIC);
    }
    {
        let cParams: ZSTD_compressionParameters = ZSTD_getCParamsFromCCtxParams(
            params,
            ZSTD_CONTENTSIZE_UNKNOWN,
            0,
            ZSTD_cpm_noAttachDict,
        );
        let blockSize: usize = MIN(
            ZSTD_resolveMaxBlockSize((*params).maxBlockSize),
            (1usize) << cParams.windowLog,
        );
        let inBuffSize: usize = if (*params).inBufferMode == ZSTD_bm_buffered {
            ((1usize) << cParams.windowLog) + blockSize
        } else {
            0
        };
        let outBuffSize: usize = if (*params).outBufferMode == ZSTD_bm_buffered {
            ZSTD_compressBound(blockSize) + 1
        } else {
            0
        };
        let useRowMatchFinder: ZSTD_ParamSwitch_e = ZSTD_resolveRowMatchFinderMode(
            (*params).useRowMatchFinder,
            addr_of!((*params).cParams),
        );

        ZSTD_estimateCCtxSize_usingCCtxParams_internal(
            &cParams,
            addr_of!((*params).ldmParams),
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
pub extern "C" fn ZSTD_estimateCStreamSize_usingCParams(
    cParams: ZSTD_compressionParameters,
) -> usize {
    let mut initialParams: ZSTD_CCtx_params = ZSTD_makeCCtxParamsFromCParams(cParams);
    unsafe {
        if ZSTD_rowMatchFinderSupported(cParams.strategy) != 0 {
            /* Pick bigger of not using and using row-based matchfinder for greedy and lazy strategies */
            let noRowCCtxSize: usize;
            let rowCCtxSize: usize;
            initialParams.useRowMatchFinder = ZSTD_ps_disable;
            noRowCCtxSize = ZSTD_estimateCStreamSize_usingCCtxParams(&initialParams);
            initialParams.useRowMatchFinder = ZSTD_ps_enable;
            rowCCtxSize = ZSTD_estimateCStreamSize_usingCCtxParams(&initialParams);
            MAX(noRowCCtxSize, rowCCtxSize)
        } else {
            ZSTD_estimateCStreamSize_usingCCtxParams(&initialParams)
        }
    }
}

pub fn ZSTD_estimateCStreamSize_internal(compressionLevel: c_int) -> usize {
    let cParams: ZSTD_compressionParameters = unsafe {
        crate::zstd_compress_p4::ZSTD_getCParams_internal(
            compressionLevel,
            ZSTD_CONTENTSIZE_UNKNOWN,
            0,
            ZSTD_cpm_noAttachDict,
        )
    };
    ZSTD_estimateCStreamSize_usingCParams(cParams)
}

#[unsafe(no_mangle)]
pub extern "C" fn ZSTD_estimateCStreamSize(compressionLevel: c_int) -> usize {
    let mut level: c_int;
    let mut memBudget: usize = 0;
    level = MIN(compressionLevel, 1);
    while level <= compressionLevel {
        let newMB: usize = ZSTD_estimateCStreamSize_internal(level);
        if newMB > memBudget {
            memBudget = newMB;
        }
        level += 1;
    }
    memBudget
}

/* ZSTD_getFrameProgression():
 * tells how much data has been consumed (input) and produced (output) for current frame.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_getFrameProgression(
    cctx: *const ZSTD_CCtx,
) -> ZSTD_frameProgression {
    {
        let mut fp: ZSTD_frameProgression = ZSTD_frameProgression::default();
        let buffered: usize = if (*cctx).inBuff.is_null() {
            0
        } else {
            (*cctx).inBuffPos.wrapping_sub((*cctx).inToCompress)
        };
        fp.ingested = (*cctx).consumedSrcSize.wrapping_add(buffered as c_ulonglong);
        fp.consumed = (*cctx).consumedSrcSize;
        fp.produced = (*cctx).producedCSize;
        fp.flushed = (*cctx).producedCSize; /* simplified; some data might still be left within streaming output buffer */
        fp.currentJobID = 0;
        fp.nbActiveWorkers = 0;
        fp
    }
}

/* ZSTD_toFlushNow()
 *  Only useful for multithreading scenarios currently (nbWorkers >= 1).
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_toFlushNow(cctx: *mut ZSTD_CCtx) -> usize {
    let _ = cctx;
    0 /* over-simplification */
}

pub fn ZSTD_assertEqualCParams(
    cParams1: ZSTD_compressionParameters,
    cParams2: ZSTD_compressionParameters,
) {
    /* body is asserts only (DEBUGLEVEL == 0) */
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_reset_compressedBlockState(bs: *mut ZSTD_compressedBlockState_t) {
    let mut i: c_int;
    i = 0;
    while (i as usize) < ZSTD_REP_NUM {
        (*bs).rep[i as usize] = repStartValue[i as usize];
        i += 1;
    }
    (*bs).entropy.huf.repeatMode = HUF_repeat_none;
    (*bs).entropy.fse.offcode_repeatMode = FSE_repeat_none;
    (*bs).entropy.fse.matchlength_repeatMode = FSE_repeat_none;
    (*bs).entropy.fse.litlength_repeatMode = FSE_repeat_none;
}

/* ZSTD_invalidateMatchState()
 *  Invalidate all the matches in the match finder tables.
 *  Requires nextSrc and base to be set (can be NULL).
 */
pub unsafe fn ZSTD_invalidateMatchState(ms: *mut ZSTD_MatchState_t) {
    ZSTD_window_clear(addr_of_mut!((*ms).window));

    (*ms).nextToUpdate = (*ms).window.dictLimit;
    (*ms).loadedDictEnd = 0;
    (*ms).opt.litLengthSum = 0; /* force reset of btopt stats */
    (*ms).dictMatchState = core::ptr::null();
}

/**
 * Controls, for this matchState reset, whether the tables need to be cleared /
 * prepared for the coming compression (ZSTDcrp_makeClean), or whether the
 * tables can be left unclean (ZSTDcrp_leaveDirty).
 */
pub type ZSTD_compResetPolicy_e = c_int;
pub const ZSTDcrp_makeClean: ZSTD_compResetPolicy_e = 0;
pub const ZSTDcrp_leaveDirty: ZSTD_compResetPolicy_e = 1;
/* convenience alias */
pub type ZSTD_ResetPolicy_e = ZSTD_compResetPolicy_e;

/**
 * Controls, for this matchState reset, whether indexing can continue where it
 * left off (ZSTDirp_continue), or whether it needs to be restarted from zero
 * (ZSTDirp_reset).
 */
pub type ZSTD_indexResetPolicy_e = c_int;
pub const ZSTDirp_continue: ZSTD_indexResetPolicy_e = 0;
pub const ZSTDirp_reset: ZSTD_indexResetPolicy_e = 1;

pub type ZSTD_resetTarget_e = c_int;
pub const ZSTD_resetTarget_CDict: ZSTD_resetTarget_e = 0;
pub const ZSTD_resetTarget_CCtx: ZSTD_resetTarget_e = 1;

/* Mixes bits in a 64 bits in a value, based on XXH3_rrmxmx */
pub fn ZSTD_bitmix(mut val: U64, len: U64) -> U64 {
    val ^= ZSTD_rotateRight_U64(val, 49) ^ ZSTD_rotateRight_U64(val, 24);
    val = val.wrapping_mul(0x9FB21C651E98DF25u64);
    val ^= (val >> 35).wrapping_add(len);
    val = val.wrapping_mul(0x9FB21C651E98DF25u64);
    val ^ (val >> 28)
}

/* Mixes in the hashSalt and hashSaltEntropy to create a new hashSalt */
pub unsafe fn ZSTD_advanceHashSalt(ms: *mut ZSTD_MatchState_t) {
    (*ms).hashSalt =
        ZSTD_bitmix((*ms).hashSalt, 8) ^ ZSTD_bitmix((*ms).hashSaltEntropy as U64, 4);
}

pub unsafe fn ZSTD_reset_matchState(
    ms: *mut ZSTD_MatchState_t,
    ws: *mut ZSTD_cwksp,
    cParams: *const ZSTD_compressionParameters,
    useRowMatchFinder: ZSTD_ParamSwitch_e,
    crp: ZSTD_compResetPolicy_e,
    forceResetIndex: ZSTD_indexResetPolicy_e,
    forWho: ZSTD_resetTarget_e,
) -> usize {
    /* disable chain table allocation for fast or row-based strategies */
    let chainSize: usize = if ZSTD_allocateChainTable(
        (*cParams).strategy,
        useRowMatchFinder,
        (((*ms).dedicatedDictSearch != 0) && (forWho == ZSTD_resetTarget_CDict)) as U32,
    ) != 0
    {
        (1usize) << (*cParams).chainLog
    } else {
        0
    };
    let hSize: usize = (1usize) << (*cParams).hashLog;
    let hashLog3: U32 = if (forWho == ZSTD_resetTarget_CCtx) && (*cParams).minMatch == 3 {
        MIN(ZSTD_HASHLOG3_MAX as U32, (*cParams).windowLog)
    } else {
        0
    };
    let h3Size: usize = if hashLog3 != 0 {
        (1usize) << hashLog3
    } else {
        0
    };

    if forceResetIndex == ZSTDirp_reset {
        ZSTD_window_init(addr_of_mut!((*ms).window));
        ZSTD_cwksp_mark_tables_dirty(ws);
    }

    (*ms).hashLog3 = hashLog3;
    (*ms).lazySkipping = 0;

    ZSTD_invalidateMatchState(ms);

    ZSTD_cwksp_clear_tables(ws);

    /* table Space */
    (*ms).hashTable =
        ZSTD_cwksp_reserve_table(ws, hSize * core::mem::size_of::<U32>()) as *mut U32;
    (*ms).chainTable =
        ZSTD_cwksp_reserve_table(ws, chainSize * core::mem::size_of::<U32>()) as *mut U32;
    (*ms).hashTable3 =
        ZSTD_cwksp_reserve_table(ws, h3Size * core::mem::size_of::<U32>()) as *mut U32;
    if ZSTD_cwksp_reserve_failed(ws) != 0 {
        return ERROR(ZSTD_error_memory_allocation);
    }

    if crp != ZSTDcrp_leaveDirty {
        /* reset tables only */
        ZSTD_cwksp_clean_tables(ws);
    }

    if ZSTD_rowMatchFinderUsed((*cParams).strategy, useRowMatchFinder) != 0 {
        /* Row match finder needs an additional table of hashes ("tags") */
        let tagTableSize: usize = hSize;
        /* We want to generate a new salt in case we reset a Cctx, but we always want to use
         * 0 when we reset a Cdict */
        if forWho == ZSTD_resetTarget_CCtx {
            (*ms).tagTable = ZSTD_cwksp_reserve_aligned_init_once(ws, tagTableSize) as *mut BYTE;
            ZSTD_advanceHashSalt(ms);
        } else {
            /* When we are not salting we want to always memset the memory */
            (*ms).tagTable = ZSTD_cwksp_reserve_aligned64(ws, tagTableSize) as *mut BYTE;
            ZSTD_memset((*ms).tagTable, 0, tagTableSize);
            (*ms).hashSalt = 0;
        }
        {
            /* Switch to 32-entry rows if searchLog is 5 (or more) */
            let rowLog: U32 = MAX(4u32, MIN((*cParams).searchLog, 6u32));
            (*ms).rowHashLog = (*cParams).hashLog.wrapping_sub(rowLog);
        }
    }

    /* opt parser space */
    if (forWho == ZSTD_resetTarget_CCtx) && ((*cParams).strategy >= ZSTD_btopt) {
        (*ms).opt.litFreq = ZSTD_cwksp_reserve_aligned64(
            ws,
            (1usize << Litbits) * core::mem::size_of::<c_uint>(),
        ) as *mut c_uint;
        (*ms).opt.litLengthFreq = ZSTD_cwksp_reserve_aligned64(
            ws,
            (MaxLL as usize + 1) * core::mem::size_of::<c_uint>(),
        ) as *mut c_uint;
        (*ms).opt.matchLengthFreq = ZSTD_cwksp_reserve_aligned64(
            ws,
            (MaxML as usize + 1) * core::mem::size_of::<c_uint>(),
        ) as *mut c_uint;
        (*ms).opt.offCodeFreq = ZSTD_cwksp_reserve_aligned64(
            ws,
            (MaxOff as usize + 1) * core::mem::size_of::<c_uint>(),
        ) as *mut c_uint;
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

    if ZSTD_cwksp_reserve_failed(ws) != 0 {
        return ERROR(ZSTD_error_memory_allocation);
    }
    0
}

/* ZSTD_indexTooCloseToMax() :
 * minor optimization : prefer memset() rather than reduceIndex()
 * which is measurably slow in some circumstances (reported for Visual Studio).
 */
pub fn ZSTD_indexTooCloseToMax(w: ZSTD_window_t) -> c_int {
    (((w.nextSrc as usize).wrapping_sub(w.base as usize))
        > (ZSTD_CURRENT_MAX.wrapping_sub(ZSTD_INDEXOVERFLOW_MARGIN)) as usize)
        as c_int
}

/** ZSTD_dictTooBig():
 * When dictionaries are larger than ZSTD_CHUNKSIZE_MAX they can't be loaded in
 * one go generically.
 */
pub fn ZSTD_dictTooBig(loadedDictSize: usize) -> c_int {
    (loadedDictSize > ZSTD_CHUNKSIZE_MAX as usize) as c_int
}

/* ZSTD_resetCCtx_internal() :
 * @param loadedDictSize The size of the dictionary to be loaded
 * into the context, if any.
 * note : `params` are assumed fully validated at this stage.
 */
pub unsafe fn ZSTD_resetCCtx_internal(
    zc: *mut ZSTD_CCtx,
    mut params: *const ZSTD_CCtx_params,
    pledgedSrcSize: U64,
    loadedDictSize: usize,
    crp: ZSTD_compResetPolicy_e,
    zbuff: ZSTD_buffered_policy_e,
) -> usize {
    let ws: *mut ZSTD_cwksp = addr_of_mut!((*zc).workspace);

    (*zc).isFirstBlock = 1;

    /* Set applied params early so we can modify them for LDM,
     * and point params at the applied params. */
    (*zc).appliedParams = *params;
    params = addr_of!((*zc).appliedParams);

    if (*params).ldmParams.enableLdm == ZSTD_ps_enable {
        /* Adjust long distance matching parameters */
        ZSTD_ldm_adjustParameters(
            addr_of_mut!((*zc).appliedParams.ldmParams),
            addr_of!((*params).cParams),
        );
    }

    {
        let windowSize: usize = MAX(
            1usize,
            MIN(
                (1u64) << (*params).cParams.windowLog,
                pledgedSrcSize,
            ) as usize,
        );
        let blockSize: usize = MIN((*params).maxBlockSize, windowSize);
        let maxNbSeq: usize = ZSTD_maxNbSeq(
            blockSize,
            (*params).cParams.minMatch,
            ZSTD_hasExtSeqProd(params),
        );
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

        let indexTooClose: c_int = ZSTD_indexTooCloseToMax((*zc).blockState.matchState.window);
        let dictTooBig: c_int = ZSTD_dictTooBig(loadedDictSize);
        let mut needsIndexReset: ZSTD_indexResetPolicy_e =
            if indexTooClose != 0 || dictTooBig != 0 || (*zc).initialized == 0 {
                ZSTDirp_reset
            } else {
                ZSTDirp_continue
            };

        let neededSpace: usize = ZSTD_estimateCCtxSize_usingCCtxParams_internal(
            addr_of!((*params).cParams),
            addr_of!((*params).ldmParams),
            ((*zc).staticSize != 0) as c_int,
            (*params).useRowMatchFinder,
            buffInSize,
            buffOutSize,
            pledgedSrcSize,
            ZSTD_hasExtSeqProd(params),
            (*params).maxBlockSize,
        );

        {
            let err_code = neededSpace;
            if ERR_isError(err_code) != 0 {
                return err_code;
            }
        }

        if (*zc).staticSize == 0 {
            ZSTD_cwksp_bump_oversized_duration(ws, 0);
        }

        {
            /* Check if workspace is large enough, alloc a new one if needed */
            let workspaceTooSmall: c_int = (ZSTD_cwksp_sizeof(ws) < neededSpace) as c_int;
            let workspaceWasteful: c_int = ZSTD_cwksp_check_wasteful(ws, neededSpace);
            let resizeWorkspace: c_int =
                (workspaceTooSmall != 0 || workspaceWasteful != 0) as c_int;

            if resizeWorkspace != 0 {
                if (*zc).staticSize != 0 {
                    return ERROR(ZSTD_error_memory_allocation);
                }

                needsIndexReset = ZSTDirp_reset;

                ZSTD_cwksp_free(ws, (*zc).customMem);
                {
                    let err_code = ZSTD_cwksp_create(ws, neededSpace, (*zc).customMem);
                    if ERR_isError(err_code) != 0 {
                        return err_code;
                    }
                }

                /* Statically sized space.
                 * tmpWorkspace never moves,
                 * though prev/next block swap places */
                (*zc).blockState.prevCBlock = ZSTD_cwksp_reserve_object(
                    ws,
                    core::mem::size_of::<ZSTD_compressedBlockState_t>(),
                ) as *mut ZSTD_compressedBlockState_t;
                if (*zc).blockState.prevCBlock.is_null() {
                    return ERROR(ZSTD_error_memory_allocation);
                }
                (*zc).blockState.nextCBlock = ZSTD_cwksp_reserve_object(
                    ws,
                    core::mem::size_of::<ZSTD_compressedBlockState_t>(),
                ) as *mut ZSTD_compressedBlockState_t;
                if (*zc).blockState.nextCBlock.is_null() {
                    return ERROR(ZSTD_error_memory_allocation);
                }
                (*zc).tmpWorkspace = ZSTD_cwksp_reserve_object(ws, TMP_WORKSPACE_SIZE);
                if (*zc).tmpWorkspace.is_null() {
                    return ERROR(ZSTD_error_memory_allocation);
                }
                (*zc).tmpWkspSize = TMP_WORKSPACE_SIZE;
            }
        }

        ZSTD_cwksp_clear(ws);

        /* init params */
        (*zc).blockState.matchState.cParams = (*params).cParams;
        (*zc).blockState.matchState.prefetchCDictTables =
            ((*params).prefetchCDictTables == ZSTD_ps_enable) as c_int;
        (*zc).pledgedSrcSizePlusOne = pledgedSrcSize.wrapping_add(1);
        (*zc).consumedSrcSize = 0;
        (*zc).producedCSize = 0;
        if pledgedSrcSize == ZSTD_CONTENTSIZE_UNKNOWN {
            (*zc).appliedParams.fParams.contentSizeFlag = 0;
        }
        (*zc).blockSizeMax = blockSize;

        ZSTD_XXH64_reset(addr_of_mut!((*zc).xxhState), 0);
        (*zc).stage = ZSTDcs_init;
        (*zc).dictID = 0;
        (*zc).dictContentSize = 0;

        ZSTD_reset_compressedBlockState((*zc).blockState.prevCBlock);

        {
            let err_code = ZSTD_reset_matchState(
                addr_of_mut!((*zc).blockState.matchState),
                ws,
                addr_of!((*params).cParams),
                (*params).useRowMatchFinder,
                crp,
                needsIndexReset,
                ZSTD_resetTarget_CCtx,
            );
            if ERR_isError(err_code) != 0 {
                return err_code;
            }
        }

        (*zc).seqStore.sequencesStart = ZSTD_cwksp_reserve_aligned64(
            ws,
            maxNbSeq * core::mem::size_of::<SeqDef>(),
        ) as *mut SeqDef;

        /* ldm hash table */
        if (*params).ldmParams.enableLdm == ZSTD_ps_enable {
            /* TODO: avoid memset? */
            let ldmHSize: usize = (1usize) << (*params).ldmParams.hashLog;
            (*zc).ldmState.hashTable = ZSTD_cwksp_reserve_aligned64(
                ws,
                ldmHSize * core::mem::size_of::<ldmEntry_t>(),
            ) as *mut ldmEntry_t;
            ZSTD_memset(
                (*zc).ldmState.hashTable as *mut u8,
                0,
                ldmHSize * core::mem::size_of::<ldmEntry_t>(),
            );
            (*zc).ldmSequences = ZSTD_cwksp_reserve_aligned64(
                ws,
                maxNbLdmSeq * core::mem::size_of::<rawSeq>(),
            ) as *mut rawSeq;
            (*zc).maxNbLdmSequences = maxNbLdmSeq;

            ZSTD_window_init(addr_of_mut!((*zc).ldmState.window));
            (*zc).ldmState.loadedDictEnd = 0;
        }

        /* reserve space for block-level external sequences */
        if ZSTD_hasExtSeqProd(params) != 0 {
            let maxNbExternalSeq: usize = crate::zstd_compress_p2::ZSTD_sequenceBound(blockSize);
            (*zc).extSeqBufCapacity = maxNbExternalSeq;
            (*zc).extSeqBuf = ZSTD_cwksp_reserve_aligned64(
                ws,
                maxNbExternalSeq * core::mem::size_of::<ZSTD_Sequence>(),
            ) as *mut ZSTD_Sequence;
        }

        /* buffers */

        /* ZSTD_wildcopy() is used to copy into the literals buffer,
         * so we have to oversize the buffer by WILDCOPY_OVERLENGTH bytes. */
        (*zc).seqStore.litStart =
            ZSTD_cwksp_reserve_buffer(ws, blockSize + WILDCOPY_OVERLENGTH as usize);
        (*zc).seqStore.maxNbLit = blockSize;

        (*zc).bufferedPolicy = zbuff;
        (*zc).inBuffSize = buffInSize;
        (*zc).inBuff = ZSTD_cwksp_reserve_buffer(ws, buffInSize) as *mut c_char;
        (*zc).outBuffSize = buffOutSize;
        (*zc).outBuff = ZSTD_cwksp_reserve_buffer(ws, buffOutSize) as *mut c_char;

        /* ldm bucketOffsets table */
        if (*params).ldmParams.enableLdm == ZSTD_ps_enable {
            /* TODO: avoid memset? */
            let numBuckets: usize = (1usize)
                << ((*params)
                    .ldmParams
                    .hashLog
                    .wrapping_sub((*params).ldmParams.bucketSizeLog));
            (*zc).ldmState.bucketOffsets = ZSTD_cwksp_reserve_buffer(ws, numBuckets);
            ZSTD_memset((*zc).ldmState.bucketOffsets, 0, numBuckets);
        }

        /* sequences storage */
        crate::zstd_compress_p3::ZSTD_referenceExternalSequences(
            zc,
            core::ptr::null_mut(),
            0,
        );
        (*zc).seqStore.maxNbSeq = maxNbSeq;
        (*zc).seqStore.llCode =
            ZSTD_cwksp_reserve_buffer(ws, maxNbSeq * core::mem::size_of::<BYTE>());
        (*zc).seqStore.mlCode =
            ZSTD_cwksp_reserve_buffer(ws, maxNbSeq * core::mem::size_of::<BYTE>());
        (*zc).seqStore.ofCode =
            ZSTD_cwksp_reserve_buffer(ws, maxNbSeq * core::mem::size_of::<BYTE>());

        (*zc).initialized = 1;

        return 0;
    }
}

/* ZSTD_invalidateRepCodes() :
 * ensures next compression will not use repcodes from previous block.
 * Note : only works with regular variant;
 *        do not use with extDict variant ! */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_invalidateRepCodes(cctx: *mut ZSTD_CCtx) {
    let mut i: c_int = 0;
    while (i as usize) < ZSTD_REP_NUM {
        (*(*cctx).blockState.prevCBlock).rep[i as usize] = 0;
        i += 1;
    }
}

/* These are the approximate sizes for each strategy past which copying the
 * dictionary tables into the working context is faster than using them
 * in-place.
 */
pub static attachDictSizeCutoffs: [usize; (ZSTD_STRATEGY_MAX + 1) as usize] = [
    8 * (1 << 10),  /* unused */
    8 * (1 << 10),  /* ZSTD_fast */
    16 * (1 << 10), /* ZSTD_dfast */
    32 * (1 << 10), /* ZSTD_greedy */
    32 * (1 << 10), /* ZSTD_lazy */
    32 * (1 << 10), /* ZSTD_lazy2 */
    32 * (1 << 10), /* ZSTD_btlazy2 */
    32 * (1 << 10), /* ZSTD_btopt */
    8 * (1 << 10),  /* ZSTD_btultra */
    8 * (1 << 10),  /* ZSTD_btultra2 */
];

pub unsafe fn ZSTD_shouldAttachDict(
    cdict: *const ZSTD_CDict,
    params: *const ZSTD_CCtx_params,
    pledgedSrcSize: U64,
) -> c_int {
    let cutoff: usize = attachDictSizeCutoffs[(*cdict).matchState.cParams.strategy as usize];
    let dedicatedDictSearch: c_int = (*cdict).matchState.dedicatedDictSearch;
    (dedicatedDictSearch != 0
        || ((pledgedSrcSize <= cutoff as U64
            || pledgedSrcSize == ZSTD_CONTENTSIZE_UNKNOWN
            || (*params).attachDictPref == ZSTD_dictForceAttach)
            && (*params).attachDictPref != ZSTD_dictForceCopy
            && (*params).forceWindow == 0)) as c_int
}

pub unsafe fn ZSTD_resetCCtx_byAttachingCDict(
    cctx: *mut ZSTD_CCtx,
    cdict: *const ZSTD_CDict,
    mut params: ZSTD_CCtx_params,
    pledgedSrcSize: U64,
    zbuff: ZSTD_buffered_policy_e,
) -> usize {
    {
        let mut adjusted_cdict_cParams: ZSTD_compressionParameters = (*cdict).matchState.cParams;
        let windowLog: c_uint = params.cParams.windowLog;
        /* Resize working context table params for input only, since the dict
         * has its own tables. */
        /* pledgedSrcSize == 0 means 0! */

        if (*cdict).matchState.dedicatedDictSearch != 0 {
            crate::zstd_compress_p4::ZSTD_dedicatedDictSearch_revertCParams(
                &mut adjusted_cdict_cParams,
            );
        }

        params.cParams = ZSTD_adjustCParams_internal(
            adjusted_cdict_cParams,
            pledgedSrcSize,
            (*cdict).dictContentSize,
            ZSTD_cpm_attachDict,
            params.useRowMatchFinder,
        );
        params.cParams.windowLog = windowLog;
        params.useRowMatchFinder = (*cdict).useRowMatchFinder; /* cdict overrides */
        {
            let err_code = ZSTD_resetCCtx_internal(
                cctx,
                &params,
                pledgedSrcSize,
                /* loadedDictSize */ 0,
                ZSTDcrp_makeClean,
                zbuff,
            );
            if ERR_isError(err_code) != 0 {
                return err_code;
            }
        }
    }

    {
        let cdictEnd: U32 = ((*cdict).matchState.window.nextSrc as usize)
            .wrapping_sub((*cdict).matchState.window.base as usize) as U32;
        let cdictLen: U32 = cdictEnd.wrapping_sub((*cdict).matchState.window.dictLimit);
        if cdictLen == 0 {
            /* don't even attach dictionaries with no contents */
        } else {
            (*cctx).blockState.matchState.dictMatchState = addr_of!((*cdict).matchState);

            /* prep working match state so dict matches never have negative indices
             * when they are translated to the working context's index space. */
            if (*cctx).blockState.matchState.window.dictLimit < cdictEnd {
                (*cctx).blockState.matchState.window.nextSrc = (*cctx)
                    .blockState
                    .matchState
                    .window
                    .base
                    .wrapping_add(cdictEnd as usize);
                ZSTD_window_clear(addr_of_mut!((*cctx).blockState.matchState.window));
            }
            /* loadedDictEnd is expressed within the referential of the active context */
            (*cctx).blockState.matchState.loadedDictEnd =
                (*cctx).blockState.matchState.window.dictLimit;
        }
    }

    (*cctx).dictID = (*cdict).dictID;
    (*cctx).dictContentSize = (*cdict).dictContentSize;

    /* copy block state */
    ZSTD_memcpy(
        (*cctx).blockState.prevCBlock as *mut u8,
        addr_of!((*cdict).cBlockState) as *const u8,
        core::mem::size_of::<ZSTD_compressedBlockState_t>(),
    );

    0
}

pub unsafe fn ZSTD_copyCDictTableIntoCCtx(
    dst: *mut U32,
    src: *const U32,
    tableSize: usize,
    cParams: *const ZSTD_compressionParameters,
) {
    if ZSTD_CDictIndicesAreTagged(cParams) != 0 {
        /* Remove tags from the CDict table if they are present.
         * See docs on "short cache" in zstd_compress_internal.h for context. */
        let mut i: usize = 0;
        while i < tableSize {
            let taggedIndex: U32 = *src.add(i);
            let index: U32 = taggedIndex >> ZSTD_SHORT_CACHE_TAG_BITS;
            *dst.add(i) = index;
            i += 1;
        }
    } else {
        ZSTD_memcpy(
            dst as *mut u8,
            src as *const u8,
            tableSize * core::mem::size_of::<U32>(),
        );
    }
}

pub unsafe fn ZSTD_resetCCtx_byCopyingCDict(
    cctx: *mut ZSTD_CCtx,
    cdict: *const ZSTD_CDict,
    mut params: ZSTD_CCtx_params,
    pledgedSrcSize: U64,
    zbuff: ZSTD_buffered_policy_e,
) -> usize {
    let cdict_cParams: *const ZSTD_compressionParameters = addr_of!((*cdict).matchState.cParams);

    {
        let windowLog: c_uint = params.cParams.windowLog;
        /* Copy only compression parameters related to tables. */
        params.cParams = *cdict_cParams;
        params.cParams.windowLog = windowLog;
        params.useRowMatchFinder = (*cdict).useRowMatchFinder;
        {
            let err_code = ZSTD_resetCCtx_internal(
                cctx,
                &params,
                pledgedSrcSize,
                /* loadedDictSize */ 0,
                ZSTDcrp_leaveDirty,
                zbuff,
            );
            if ERR_isError(err_code) != 0 {
                return err_code;
            }
        }
    }

    ZSTD_cwksp_mark_tables_dirty(addr_of_mut!((*cctx).workspace));

    /* copy tables */
    {
        let chainSize: usize = if ZSTD_allocateChainTable(
            (*cdict_cParams).strategy,
            (*cdict).useRowMatchFinder,
            0, /* DDS guaranteed disabled */
        ) != 0
        {
            (1usize) << (*cdict_cParams).chainLog
        } else {
            0
        };
        let hSize: usize = (1usize) << (*cdict_cParams).hashLog;

        ZSTD_copyCDictTableIntoCCtx(
            (*cctx).blockState.matchState.hashTable,
            (*cdict).matchState.hashTable,
            hSize,
            cdict_cParams,
        );

        /* Do not copy cdict's chainTable if cctx has parameters such that it would not use chainTable */
        if ZSTD_allocateChainTable(
            (*cctx).appliedParams.cParams.strategy,
            (*cctx).appliedParams.useRowMatchFinder,
            0, /* forDDSDict */
        ) != 0
        {
            ZSTD_copyCDictTableIntoCCtx(
                (*cctx).blockState.matchState.chainTable,
                (*cdict).matchState.chainTable,
                chainSize,
                cdict_cParams,
            );
        }
        /* copy tag table */
        if ZSTD_rowMatchFinderUsed((*cdict_cParams).strategy, (*cdict).useRowMatchFinder) != 0 {
            let tagTableSize: usize = hSize;
            ZSTD_memcpy(
                (*cctx).blockState.matchState.tagTable,
                (*cdict).matchState.tagTable,
                tagTableSize,
            );
            (*cctx).blockState.matchState.hashSalt = (*cdict).matchState.hashSalt;
        }
    }

    /* Zero the hashTable3, since the cdict never fills it */
    {
        let h3log: U32 = (*cctx).blockState.matchState.hashLog3;
        let h3Size: usize = if h3log != 0 { (1usize) << h3log } else { 0 };
        ZSTD_memset(
            (*cctx).blockState.matchState.hashTable3 as *mut u8,
            0,
            h3Size * core::mem::size_of::<U32>(),
        );
    }

    ZSTD_cwksp_mark_tables_clean(addr_of_mut!((*cctx).workspace));

    /* copy dictionary offsets */
    {
        let srcMatchState: *const ZSTD_MatchState_t = addr_of!((*cdict).matchState);
        let dstMatchState: *mut ZSTD_MatchState_t = addr_of_mut!((*cctx).blockState.matchState);
        (*dstMatchState).window = (*srcMatchState).window;
        (*dstMatchState).nextToUpdate = (*srcMatchState).nextToUpdate;
        (*dstMatchState).loadedDictEnd = (*srcMatchState).loadedDictEnd;
    }

    (*cctx).dictID = (*cdict).dictID;
    (*cctx).dictContentSize = (*cdict).dictContentSize;

    /* copy block state */
    ZSTD_memcpy(
        (*cctx).blockState.prevCBlock as *mut u8,
        addr_of!((*cdict).cBlockState) as *const u8,
        core::mem::size_of::<ZSTD_compressedBlockState_t>(),
    );

    0
}

/* We have a choice between copying the dictionary context into the working
 * context, or referencing the dictionary context from the working context
 * in-place. We decide here which strategy to use. */
pub unsafe fn ZSTD_resetCCtx_usingCDict(
    cctx: *mut ZSTD_CCtx,
    cdict: *const ZSTD_CDict,
    params: *const ZSTD_CCtx_params,
    pledgedSrcSize: U64,
    zbuff: ZSTD_buffered_policy_e,
) -> usize {
    if ZSTD_shouldAttachDict(cdict, params, pledgedSrcSize) != 0 {
        ZSTD_resetCCtx_byAttachingCDict(cctx, cdict, *params, pledgedSrcSize, zbuff)
    } else {
        ZSTD_resetCCtx_byCopyingCDict(cctx, cdict, *params, pledgedSrcSize, zbuff)
    }
}

/* ZSTD_copyCCtx_internal() :
 *  Duplicate an existing context `srcCCtx` into another one `dstCCtx`.
 *  Only works during stage ZSTDcs_init.
 * @return : 0, or an error code */
pub unsafe fn ZSTD_copyCCtx_internal(
    dstCCtx: *mut ZSTD_CCtx,
    srcCCtx: *const ZSTD_CCtx,
    fParams: ZSTD_frameParameters,
    pledgedSrcSize: U64,
    zbuff: ZSTD_buffered_policy_e,
) -> usize {
    if (*srcCCtx).stage != ZSTDcs_init {
        return ERROR(ZSTD_error_stage_wrong);
    }
    ZSTD_memcpy(
        addr_of_mut!((*dstCCtx).customMem) as *mut u8,
        addr_of!((*srcCCtx).customMem) as *const u8,
        core::mem::size_of::<ZSTD_customMem>(),
    );
    {
        let mut params: ZSTD_CCtx_params = (*dstCCtx).requestedParams;
        /* Copy only compression parameters related to tables. */
        params.cParams = (*srcCCtx).appliedParams.cParams;
        params.useRowMatchFinder = (*srcCCtx).appliedParams.useRowMatchFinder;
        params.postBlockSplitter = (*srcCCtx).appliedParams.postBlockSplitter;
        params.ldmParams = (*srcCCtx).appliedParams.ldmParams;
        params.fParams = fParams;
        params.maxBlockSize = (*srcCCtx).appliedParams.maxBlockSize;
        let _ = ZSTD_resetCCtx_internal(
            dstCCtx,
            &params,
            pledgedSrcSize,
            /* loadedDictSize */ 0,
            ZSTDcrp_leaveDirty,
            zbuff,
        );
    }

    ZSTD_cwksp_mark_tables_dirty(addr_of_mut!((*dstCCtx).workspace));

    /* copy tables */
    {
        let chainSize: usize = if ZSTD_allocateChainTable(
            (*srcCCtx).appliedParams.cParams.strategy,
            (*srcCCtx).appliedParams.useRowMatchFinder,
            0, /* forDDSDict */
        ) != 0
        {
            (1usize) << (*srcCCtx).appliedParams.cParams.chainLog
        } else {
            0
        };
        let hSize: usize = (1usize) << (*srcCCtx).appliedParams.cParams.hashLog;
        let h3log: U32 = (*srcCCtx).blockState.matchState.hashLog3;
        let h3Size: usize = if h3log != 0 { (1usize) << h3log } else { 0 };

        ZSTD_memcpy(
            (*dstCCtx).blockState.matchState.hashTable as *mut u8,
            (*srcCCtx).blockState.matchState.hashTable as *const u8,
            hSize * core::mem::size_of::<U32>(),
        );
        ZSTD_memcpy(
            (*dstCCtx).blockState.matchState.chainTable as *mut u8,
            (*srcCCtx).blockState.matchState.chainTable as *const u8,
            chainSize * core::mem::size_of::<U32>(),
        );
        ZSTD_memcpy(
            (*dstCCtx).blockState.matchState.hashTable3 as *mut u8,
            (*srcCCtx).blockState.matchState.hashTable3 as *const u8,
            h3Size * core::mem::size_of::<U32>(),
        );
    }

    ZSTD_cwksp_mark_tables_clean(addr_of_mut!((*dstCCtx).workspace));

    /* copy dictionary offsets */
    {
        let srcMatchState: *const ZSTD_MatchState_t = addr_of!((*srcCCtx).blockState.matchState);
        let dstMatchState: *mut ZSTD_MatchState_t = addr_of_mut!((*dstCCtx).blockState.matchState);
        (*dstMatchState).window = (*srcMatchState).window;
        (*dstMatchState).nextToUpdate = (*srcMatchState).nextToUpdate;
        (*dstMatchState).loadedDictEnd = (*srcMatchState).loadedDictEnd;
    }
    (*dstCCtx).dictID = (*srcCCtx).dictID;
    (*dstCCtx).dictContentSize = (*srcCCtx).dictContentSize;

    /* copy block state */
    ZSTD_memcpy(
        (*dstCCtx).blockState.prevCBlock as *mut u8,
        (*srcCCtx).blockState.prevCBlock as *const u8,
        core::mem::size_of::<ZSTD_compressedBlockState_t>(),
    );

    0
}

/* ZSTD_copyCCtx() :
 *  Duplicate an existing context `srcCCtx` into another one `dstCCtx`.
 *  pledgedSrcSize==0 means "unknown".
 *  @return : 0, or an error code */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_copyCCtx(
    dstCCtx: *mut ZSTD_CCtx,
    srcCCtx: *const ZSTD_CCtx,
    mut pledgedSrcSize: c_ulonglong,
) -> usize {
    let mut fParams: ZSTD_frameParameters = ZSTD_frameParameters {
        contentSizeFlag: 1, /* content */
        checksumFlag: 0,    /* checksum */
        noDictIDFlag: 0,    /* noDictID */
    };
    let zbuff: ZSTD_buffered_policy_e = (*srcCCtx).bufferedPolicy;
    if pledgedSrcSize == 0 {
        pledgedSrcSize = ZSTD_CONTENTSIZE_UNKNOWN;
    }
    fParams.contentSizeFlag = (pledgedSrcSize != ZSTD_CONTENTSIZE_UNKNOWN) as c_int;

    ZSTD_copyCCtx_internal(dstCCtx, srcCCtx, fParams, pledgedSrcSize, zbuff)
}

/* ZSTD_reduceTable() :
 *  reduce table indexes by `reducerValue`, or squash to zero.
 *  PreserveMark preserves "unsorted mark" for btlazy2 strategy.
 *  Presume table size is a multiple of ZSTD_ROWSIZE
 *  to help auto-vectorization */
#[inline(always)]
pub unsafe fn ZSTD_reduceTable_internal(
    table: *mut U32,
    size: U32,
    reducerValue: U32,
    preserveMark: c_int,
) {
    let nbRows: c_int = (size as c_int) / ZSTD_ROWSIZE;
    let mut cellNb: c_int = 0;
    let mut rowNb: c_int;
    /* Protect special index values < ZSTD_WINDOW_START_INDEX. */
    let reducerThreshold: U32 = reducerValue.wrapping_add(ZSTD_WINDOW_START_INDEX);

    rowNb = 0;
    while rowNb < nbRows {
        let mut column: c_int = 0;
        while column < ZSTD_ROWSIZE {
            let newVal: U32;
            if preserveMark != 0 && *table.offset(cellNb as isize) == ZSTD_DUBT_UNSORTED_MARK {
                /* This write is pointless, but is required(?) for the compiler
                 * to auto-vectorize the loop. */
                newVal = ZSTD_DUBT_UNSORTED_MARK;
            } else if *table.offset(cellNb as isize) < reducerThreshold {
                newVal = 0;
            } else {
                newVal = (*table.offset(cellNb as isize)).wrapping_sub(reducerValue);
            }
            *table.offset(cellNb as isize) = newVal;
            cellNb += 1;
            column += 1;
        }
        rowNb += 1;
    }
}

pub unsafe fn ZSTD_reduceTable(table: *mut U32, size: U32, reducerValue: U32) {
    ZSTD_reduceTable_internal(table, size, reducerValue, 0);
}

pub unsafe fn ZSTD_reduceTable_btlazy2(table: *mut U32, size: U32, reducerValue: U32) {
    ZSTD_reduceTable_internal(table, size, reducerValue, 1);
}

/* ZSTD_reduceIndex() :
 *   rescale all indexes to avoid future overflow (indexes are U32) */
pub unsafe fn ZSTD_reduceIndex(
    ms: *mut ZSTD_MatchState_t,
    params: *const ZSTD_CCtx_params,
    reducerValue: U32,
) {
    {
        let hSize: U32 = (1u32) << (*params).cParams.hashLog;
        ZSTD_reduceTable((*ms).hashTable, hSize, reducerValue);
    }

    if ZSTD_allocateChainTable(
        (*params).cParams.strategy,
        (*params).useRowMatchFinder,
        (*ms).dedicatedDictSearch as U32,
    ) != 0
    {
        let chainSize: U32 = (1u32) << (*params).cParams.chainLog;
        if (*params).cParams.strategy == ZSTD_btlazy2 {
            ZSTD_reduceTable_btlazy2((*ms).chainTable, chainSize, reducerValue);
        } else {
            ZSTD_reduceTable((*ms).chainTable, chainSize, reducerValue);
        }
    }

    if (*ms).hashLog3 != 0 {
        let h3Size: U32 = (1u32) << (*ms).hashLog3;
        ZSTD_reduceTable((*ms).hashTable3, h3Size, reducerValue);
    }
}

/* ---------------------------------------------------------------------------
 *  Compatibility re-exports.
 *
 *  zstd_compress.c has been split across four Rust modules
 *  (`zstd_compress`, `zstd_compress_p2`, `zstd_compress_p3`,
 *  `zstd_compress_p4`).  Several foundation modules that were translated
 *  before the split still reach for symbols through
 *  `crate::zstd_compress::<NAME>`.  Re-export the ones that ended up in the
 *  other parts so that those paths keep resolving.
 * ------------------------------------------------------------------------ */
pub use crate::zstd_compress_p2::ZSTD_resetSeqStore;
pub use crate::zstd_compress_p3::{
    ZSTD_CCtx_trace, ZSTD_compressBegin_advanced_internal, ZSTD_compressContinue_public,
    ZSTD_compressEnd_public, ZSTD_compress_usingCDict, ZSTD_createCDict,
    ZSTD_createCDict_advanced, ZSTD_freeCDict, ZSTD_referenceExternalSequences,
    ZSTD_sizeof_CDict, ZSTD_writeLastEmptyBlock,
};
