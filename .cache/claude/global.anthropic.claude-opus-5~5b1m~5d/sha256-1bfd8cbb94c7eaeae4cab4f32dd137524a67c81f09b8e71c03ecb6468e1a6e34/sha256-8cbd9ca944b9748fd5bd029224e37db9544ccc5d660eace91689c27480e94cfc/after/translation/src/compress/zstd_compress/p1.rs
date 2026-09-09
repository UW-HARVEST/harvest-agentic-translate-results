//! Translation of `compress/zstd_compress.c`, lines 1..2098
//! (helper functions, context memory management, parameter handling,
//!  size estimation, matchState reset).
#![allow(dead_code)]

use core::ffi::{c_int, c_uint, c_void};

use crate::bits::*;
use crate::cmem::*;
use crate::compress::clevels::*;
use crate::compress::zstd_compress_internal::*;
use crate::compress::zstd_cwksp::*;
use crate::error_private::*;
use crate::fse::*;
use crate::huf::*;
use crate::pool::POOL_ctx;
use crate::zstd_h::*;
use crate::zstd_internal::*;

/* ===== zstd.h : `ZSTD_customMem const ZSTD_defaultCMem = { NULL, NULL, NULL };`
 * (a `const` item rather than a `static`: `ZSTD_customMem` holds a raw pointer,
 * so it is not `Sync`; it is never exported from the C .so either, and every use
 * site copies it by value, so this is behaviourally identical.) ===== */
pub const ZSTD_defaultCMem: ZSTD_customMem = ZSTD_customMem {
    customAlloc: None,
    customFree: None,
    opaque: core::ptr::null_mut(),
};

/* ===== zstd.h : `#define ZSTD_CLEVEL_DEFAULT 3` ===== */
pub const ZSTD_CLEVEL_DEFAULT: c_int = 3;

/* ===== zstd_lazy.h : `#define ZSTD_ROW_HASH_TAG_BITS 8` ===== */
pub const ZSTD_ROW_HASH_TAG_BITS: U32 = 8;

/* ===== zstd_ldm.h : `#define ZSTD_LDM_DEFAULT_WINDOW_LOG ZSTD_WINDOWLOG_LIMIT_DEFAULT` ===== */
pub const ZSTD_LDM_DEFAULT_WINDOW_LOG: U32 = ZSTD_WINDOWLOG_LIMIT_DEFAULT as U32;

/* ***************************************************************
*  Tuning parameters
*****************************************************************/
pub const ZSTD_COMPRESS_HEAPMODE: c_int = 0;
pub const ZSTD_HASHLOG3_MAX: U32 = 17;

/* ===== zstd_compress_internal.h lines 615-625 ===== */
/* ZSTD_cParam_withinBounds:
 * @return 1 if value is within cParam bounds,
 * 0 otherwise */
pub unsafe fn ZSTD_cParam_withinBounds(cParam: ZSTD_cParameter, value: c_int) -> c_int {
    let bounds = ZSTD_cParam_getBounds(cParam);
    if crate::zstd_common::ZSTD_isError(bounds.error) != 0 {
        return 0;
    }
    if value < bounds.lowerBound {
        return 0;
    }
    if value > bounds.upperBound {
        return 0;
    }
    1
}

/*-*************************************
*  Helper functions
***************************************/
/* ZSTD_compressBound() */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBound(srcSize: usize) -> usize {
    let r = ZSTD_COMPRESSBOUND(srcSize);
    if r == 0 {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    r
}

/*-*************************************
*  Context memory management
***************************************/
/* `struct ZSTD_CDict_s` is defined in `compress/zstd_compress_internal.rs`. */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_createCCtx() -> *mut ZSTD_CCtx {
    ZSTD_createCCtx_advanced(ZSTD_defaultCMem)
}

pub unsafe fn ZSTD_initCCtx(cctx: *mut ZSTD_CCtx, memManager: ZSTD_customMem) {
    ZSTD_memset(
        cctx as *mut c_void,
        0,
        core::mem::size_of::<ZSTD_CCtx>(),
    );
    (*cctx).customMem = memManager;
    (*cctx).bmi2 = ZSTD_cpuSupportsBmi2();
    {
        let _err = ZSTD_CCtx_reset(cctx, ZSTD_reset_parameters);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_createCCtx_advanced(customMem: ZSTD_customMem) -> *mut ZSTD_CCtx {
    if ((customMem.customAlloc.is_none() as c_int) ^ (customMem.customFree.is_none() as c_int)) != 0
    {
        return core::ptr::null_mut();
    }
    {
        let cctx =
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
    let mut ws = ZSTD_cwksp::default();
    let cctx: *mut ZSTD_CCtx;
    if workspaceSize <= core::mem::size_of::<ZSTD_CCtx>() {
        return core::ptr::null_mut(); /* minimum size */
    }
    if (workspace as usize) & 7 != 0 {
        return core::ptr::null_mut(); /* must be 8-aligned */
    }
    ZSTD_cwksp_init(&mut ws, workspace, workspaceSize, ZSTD_cwksp_static_alloc);

    cctx = ZSTD_cwksp_reserve_object(&mut ws, core::mem::size_of::<ZSTD_CCtx>()) as *mut ZSTD_CCtx;
    if cctx.is_null() {
        return core::ptr::null_mut();
    }

    ZSTD_memset(
        cctx as *mut c_void,
        0,
        core::mem::size_of::<ZSTD_CCtx>(),
    );
    ZSTD_cwksp_move(&mut (*cctx).workspace, &mut ws);
    (*cctx).staticSize = workspaceSize;

    /* statically sized space. tmpWorkspace never moves (but prev/next block swap places) */
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
    /* C: `cctx->bmi2 = ZSTD_cpuid_bmi2(ZSTD_cpuid());` — with DYNAMIC_BMI2==0 this
     * field never selects a code path, and `ZSTD_cpuSupportsBmi2()` is the
     * equivalent (always-0) helper provided by this port. */
    (*cctx).bmi2 = ZSTD_cpuSupportsBmi2();
    cctx
}

/**
 * Clears and frees all of the dictionaries in the CCtx.
 */
pub unsafe fn ZSTD_clearAllDicts(cctx: *mut ZSTD_CCtx) {
    ZSTD_customFree((*cctx).localDict.dictBuffer, (*cctx).customMem);
    crate::compress::zstd_compress::ZSTD_freeCDict((*cctx).localDict.cdict);
    ZSTD_memset(
        &mut (*cctx).localDict as *mut ZSTD_localDict as *mut c_void,
        0,
        core::mem::size_of::<ZSTD_localDict>(),
    );
    ZSTD_memset(
        &mut (*cctx).prefixDict as *mut ZSTD_prefixDict as *mut c_void,
        0,
        core::mem::size_of::<ZSTD_prefixDict>(),
    );
    (*cctx).cdict = core::ptr::null();
}

pub unsafe fn ZSTD_sizeof_localDict(dict: ZSTD_localDict) -> usize {
    let bufferSize = if !dict.dictBuffer.is_null() {
        dict.dictSize
    } else {
        0
    };
    let cdictSize = crate::compress::zstd_compress::ZSTD_sizeof_CDict(dict.cdict);
    bufferSize + cdictSize
}

pub unsafe fn ZSTD_freeCCtxContent(cctx: *mut ZSTD_CCtx) {
    ZSTD_clearAllDicts(cctx);
    ZSTD_cwksp_free(&mut (*cctx).workspace, (*cctx).customMem);
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
        let cctxInWorkspace =
            ZSTD_cwksp_owns_buffer(&(*cctx).workspace, cctx as *const c_void);
        let customMem = (*cctx).customMem;
        ZSTD_freeCCtxContent(cctx);
        if cctxInWorkspace == 0 {
            ZSTD_customFree(cctx as *mut c_void, customMem);
        }
    }
    0
}

pub unsafe fn ZSTD_sizeof_mtctx(_cctx: *const ZSTD_CCtx) -> usize {
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
    }) + ZSTD_cwksp_sizeof(&(*cctx).workspace)
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
    &(*ctx).seqStore
}

/* Returns true if the strategy supports using a row based matchfinder */
pub unsafe fn ZSTD_rowMatchFinderSupported(strategy: ZSTD_strategy) -> c_int {
    (strategy >= ZSTD_greedy && strategy <= ZSTD_lazy2) as c_int
}

/* Returns true if the strategy and useRowMatchFinder mode indicate that we will
 * use the row based matchfinder for this compression.
 */
pub unsafe fn ZSTD_rowMatchFinderUsed(
    strategy: ZSTD_strategy,
    mode: ZSTD_ParamSwitch_e,
) -> c_int {
    (ZSTD_rowMatchFinderSupported(strategy) != 0 && (mode == ZSTD_ps_enable)) as c_int
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
pub unsafe fn ZSTD_allocateChainTable(
    strategy: ZSTD_strategy,
    useRowMatchFinder: ZSTD_ParamSwitch_e,
    forDDSDict: U32,
) -> c_int {
    (forDDSDict != 0
        || ((strategy != ZSTD_fast) && ZSTD_rowMatchFinderUsed(strategy, useRowMatchFinder) == 0))
        as c_int
}

/* Returns ZSTD_ps_enable if compression parameters are such that we should
 * enable long distance matching (wlog >= 27, strategy >= btopt).
 * Returns ZSTD_ps_disable otherwise.
 */
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

pub unsafe fn ZSTD_resolveExternalSequenceValidation(mode: c_int) -> c_int {
    mode
}

/* Resolves maxBlockSize to the default if no value is present. */
pub unsafe fn ZSTD_resolveMaxBlockSize(maxBlockSize: usize) -> usize {
    if maxBlockSize == 0 {
        ZSTD_BLOCKSIZE_MAX
    } else {
        maxBlockSize
    }
}

pub unsafe fn ZSTD_resolveExternalRepcodeSearch(
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

/* Returns 1 if compression parameters are such that CDict hashtable and chaintable
 * indices are tagged. */
pub unsafe fn ZSTD_CDictIndicesAreTagged(cParams: *const ZSTD_compressionParameters) -> c_int {
    ((*cParams).strategy == ZSTD_fast || (*cParams).strategy == ZSTD_dfast) as c_int
}

pub unsafe fn ZSTD_makeCCtxParamsFromCParams(
    cParams: ZSTD_compressionParameters,
) -> ZSTD_CCtx_params {
    let mut cctxParams = ZSTD_CCtx_params::default();
    /* should not matter, as all cParams are presumed properly defined */
    ZSTD_CCtxParams_init(&mut cctxParams, ZSTD_CLEVEL_DEFAULT);
    cctxParams.cParams = cParams;

    /* Adjust advanced params according to cParams */
    cctxParams.ldmParams.enableLdm =
        ZSTD_resolveEnableLdm(cctxParams.ldmParams.enableLdm, &cParams);
    if cctxParams.ldmParams.enableLdm == ZSTD_ps_enable {
        crate::compress::zstd_ldm::ZSTD_ldm_adjustParameters(
            &mut cctxParams.ldmParams,
            &cParams,
        );
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
    ZSTD_customFree(params as *mut c_void, (*params).customMem);
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
        cctxParams as *mut c_void,
        0,
        core::mem::size_of::<ZSTD_CCtx_params>(),
    );
    (*cctxParams).compressionLevel = compressionLevel;
    (*cctxParams).fParams.contentSizeFlag = 1;
    0
}

pub const ZSTD_NO_CLEVEL: c_int = 0;

/**
 * Initializes `cctxParams` from `params` and `compressionLevel`.
 */
pub unsafe fn ZSTD_CCtxParams_init_internal(
    cctxParams: *mut ZSTD_CCtx_params,
    params: *const ZSTD_parameters,
    compressionLevel: c_int,
) {
    ZSTD_memset(
        cctxParams as *mut c_void,
        0,
        core::mem::size_of::<ZSTD_CCtx_params>(),
    );
    (*cctxParams).cParams = (*params).cParams;
    (*cctxParams).fParams = (*params).fParams;
    /* Should not matter, as all cParams are presumed properly defined.
     * But, set it for tracing anyway.
     */
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
    if cctxParams.is_null() {
        return ERROR(ZSTD_error_GENERIC);
    }
    {
        let e = ZSTD_checkCParams(params.cParams);
        if ERR_isError(e) != 0 {
            return e;
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
     * But, set it for tracing anyway.
     */
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
            bounds.lowerBound = crate::compress::zstd_compress::ZSTD_minCLevel();
            bounds.upperBound = crate::compress::zstd_compress::ZSTD_maxCLevel();
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
            bounds.upperBound = 0;
            return bounds;
        }

        ZSTD_c_jobSize => {
            bounds.lowerBound = 0;
            bounds.upperBound = 0;
            return bounds;
        }

        ZSTD_c_overlapLog => {
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
            bounds.lowerBound = ZSTD_f_zstd1 as c_int;
            bounds.upperBound = ZSTD_f_zstd1_magicless as c_int;
            return bounds;
        }

        ZSTD_c_forceAttachDict => {
            bounds.lowerBound = ZSTD_dictDefaultAttach as c_int;
            bounds.upperBound = ZSTD_dictForceLoad as c_int;
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
            bounds.lowerBound = ZSTD_BLOCKSIZE_MAX_MIN as c_int;
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
    let bounds = ZSTD_cParam_getBounds(cParam);
    if crate::zstd_common::ZSTD_isError(bounds.error) != 0 {
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

pub unsafe fn ZSTD_isUpdateAuthorized(param: ZSTD_cParameter) -> c_int {
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
            if (value != 0) && (*cctx).staticSize != 0 {
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
            if ZSTD_cParam_withinBounds(ZSTD_c_format, value) == 0 {
                return ERROR(ZSTD_error_parameter_outOfBound);
            }
            (*CCtxParams).format = value as ZSTD_format_e;
            (*CCtxParams).format as usize
        }

        ZSTD_c_compressionLevel => {
            {
                let e = ZSTD_cParam_clampBounds(param, &mut value);
                if ERR_isError(e) != 0 {
                    return e;
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
            0 /* return type (size_t) cannot represent negative values */
        }

        ZSTD_c_windowLog => {
            if value != 0 {
                /* 0 => use default */
                if ZSTD_cParam_withinBounds(ZSTD_c_windowLog, value) == 0 {
                    return ERROR(ZSTD_error_parameter_outOfBound);
                }
            }
            (*CCtxParams).cParams.windowLog = value as U32;
            (*CCtxParams).cParams.windowLog as usize
        }

        ZSTD_c_hashLog => {
            if value != 0 {
                if ZSTD_cParam_withinBounds(ZSTD_c_hashLog, value) == 0 {
                    return ERROR(ZSTD_error_parameter_outOfBound);
                }
            }
            (*CCtxParams).cParams.hashLog = value as U32;
            (*CCtxParams).cParams.hashLog as usize
        }

        ZSTD_c_chainLog => {
            if value != 0 {
                if ZSTD_cParam_withinBounds(ZSTD_c_chainLog, value) == 0 {
                    return ERROR(ZSTD_error_parameter_outOfBound);
                }
            }
            (*CCtxParams).cParams.chainLog = value as U32;
            (*CCtxParams).cParams.chainLog as usize
        }

        ZSTD_c_searchLog => {
            if value != 0 {
                if ZSTD_cParam_withinBounds(ZSTD_c_searchLog, value) == 0 {
                    return ERROR(ZSTD_error_parameter_outOfBound);
                }
            }
            (*CCtxParams).cParams.searchLog = value as U32;
            value as usize
        }

        ZSTD_c_minMatch => {
            if value != 0 {
                if ZSTD_cParam_withinBounds(ZSTD_c_minMatch, value) == 0 {
                    return ERROR(ZSTD_error_parameter_outOfBound);
                }
            }
            (*CCtxParams).cParams.minMatch = value as U32;
            (*CCtxParams).cParams.minMatch as usize
        }

        ZSTD_c_targetLength => {
            if ZSTD_cParam_withinBounds(ZSTD_c_targetLength, value) == 0 {
                return ERROR(ZSTD_error_parameter_outOfBound);
            }
            (*CCtxParams).cParams.targetLength = value as U32;
            (*CCtxParams).cParams.targetLength as usize
        }

        ZSTD_c_strategy => {
            if value != 0 {
                if ZSTD_cParam_withinBounds(ZSTD_c_strategy, value) == 0 {
                    return ERROR(ZSTD_error_parameter_outOfBound);
                }
            }
            (*CCtxParams).cParams.strategy = value as ZSTD_strategy;
            (*CCtxParams).cParams.strategy as usize
        }

        ZSTD_c_contentSizeFlag => {
            /* Content size written in frame header _when known_ (default:1) */
            (*CCtxParams).fParams.contentSizeFlag = (value != 0) as c_int;
            (*CCtxParams).fParams.contentSizeFlag as usize
        }

        ZSTD_c_checksumFlag => {
            (*CCtxParams).fParams.checksumFlag = (value != 0) as c_int;
            (*CCtxParams).fParams.checksumFlag as usize
        }

        ZSTD_c_dictIDFlag => {
            (*CCtxParams).fParams.noDictIDFlag = (value == 0) as c_int;
            ((*CCtxParams).fParams.noDictIDFlag == 0) as usize
        }

        ZSTD_c_forceMaxWindow => {
            (*CCtxParams).forceWindow = (value != 0) as c_int;
            (*CCtxParams).forceWindow as usize
        }

        ZSTD_c_forceAttachDict => {
            let pref = value as ZSTD_dictAttachPref_e;
            if ZSTD_cParam_withinBounds(ZSTD_c_forceAttachDict, pref as c_int) == 0 {
                return ERROR(ZSTD_error_parameter_outOfBound);
            }
            (*CCtxParams).attachDictPref = pref;
            (*CCtxParams).attachDictPref as usize
        }

        ZSTD_c_literalCompressionMode => {
            let lcm = value as ZSTD_ParamSwitch_e;
            if ZSTD_cParam_withinBounds(ZSTD_c_literalCompressionMode, lcm as c_int) == 0 {
                return ERROR(ZSTD_error_parameter_outOfBound);
            }
            (*CCtxParams).literalCompressionMode = lcm;
            (*CCtxParams).literalCompressionMode as usize
        }

        ZSTD_c_nbWorkers => {
            if value != 0 {
                return ERROR(ZSTD_error_parameter_unsupported);
            }
            0
        }

        ZSTD_c_jobSize => {
            if value != 0 {
                return ERROR(ZSTD_error_parameter_unsupported);
            }
            0
        }

        ZSTD_c_overlapLog => {
            if value != 0 {
                return ERROR(ZSTD_error_parameter_unsupported);
            }
            0
        }

        ZSTD_c_rsyncable => {
            if value != 0 {
                return ERROR(ZSTD_error_parameter_unsupported);
            }
            0
        }

        ZSTD_c_enableDedicatedDictSearch => {
            (*CCtxParams).enableDedicatedDictSearch = (value != 0) as c_int;
            (*CCtxParams).enableDedicatedDictSearch as usize
        }

        ZSTD_c_enableLongDistanceMatching => {
            if ZSTD_cParam_withinBounds(ZSTD_c_enableLongDistanceMatching, value) == 0 {
                return ERROR(ZSTD_error_parameter_outOfBound);
            }
            (*CCtxParams).ldmParams.enableLdm = value as ZSTD_ParamSwitch_e;
            (*CCtxParams).ldmParams.enableLdm as usize
        }

        ZSTD_c_ldmHashLog => {
            if value != 0 {
                /* 0 ==> auto */
                if ZSTD_cParam_withinBounds(ZSTD_c_ldmHashLog, value) == 0 {
                    return ERROR(ZSTD_error_parameter_outOfBound);
                }
            }
            (*CCtxParams).ldmParams.hashLog = value as U32;
            (*CCtxParams).ldmParams.hashLog as usize
        }

        ZSTD_c_ldmMinMatch => {
            if value != 0 {
                if ZSTD_cParam_withinBounds(ZSTD_c_ldmMinMatch, value) == 0 {
                    return ERROR(ZSTD_error_parameter_outOfBound);
                }
            }
            (*CCtxParams).ldmParams.minMatchLength = value as U32;
            (*CCtxParams).ldmParams.minMatchLength as usize
        }

        ZSTD_c_ldmBucketSizeLog => {
            if value != 0 {
                if ZSTD_cParam_withinBounds(ZSTD_c_ldmBucketSizeLog, value) == 0 {
                    return ERROR(ZSTD_error_parameter_outOfBound);
                }
            }
            (*CCtxParams).ldmParams.bucketSizeLog = value as U32;
            (*CCtxParams).ldmParams.bucketSizeLog as usize
        }

        ZSTD_c_ldmHashRateLog => {
            if value != 0 {
                if ZSTD_cParam_withinBounds(ZSTD_c_ldmHashRateLog, value) == 0 {
                    return ERROR(ZSTD_error_parameter_outOfBound);
                }
            }
            (*CCtxParams).ldmParams.hashRateLog = value as U32;
            (*CCtxParams).ldmParams.hashRateLog as usize
        }

        ZSTD_c_targetCBlockSize => {
            if value != 0 {
                /* 0 ==> default */
                value = if value > ZSTD_TARGETCBLOCKSIZE_MIN {
                    value
                } else {
                    ZSTD_TARGETCBLOCKSIZE_MIN
                };
                if ZSTD_cParam_withinBounds(ZSTD_c_targetCBlockSize, value) == 0 {
                    return ERROR(ZSTD_error_parameter_outOfBound);
                }
            }
            (*CCtxParams).targetCBlockSize = (value as U32) as usize;
            (*CCtxParams).targetCBlockSize
        }

        ZSTD_c_srcSizeHint => {
            if value != 0 {
                /* 0 ==> default */
                if ZSTD_cParam_withinBounds(ZSTD_c_srcSizeHint, value) == 0 {
                    return ERROR(ZSTD_error_parameter_outOfBound);
                }
            }
            (*CCtxParams).srcSizeHint = value;
            (*CCtxParams).srcSizeHint as usize
        }

        ZSTD_c_stableInBuffer => {
            if ZSTD_cParam_withinBounds(ZSTD_c_stableInBuffer, value) == 0 {
                return ERROR(ZSTD_error_parameter_outOfBound);
            }
            (*CCtxParams).inBufferMode = value as ZSTD_bufferMode_e;
            (*CCtxParams).inBufferMode as usize
        }

        ZSTD_c_stableOutBuffer => {
            if ZSTD_cParam_withinBounds(ZSTD_c_stableOutBuffer, value) == 0 {
                return ERROR(ZSTD_error_parameter_outOfBound);
            }
            (*CCtxParams).outBufferMode = value as ZSTD_bufferMode_e;
            (*CCtxParams).outBufferMode as usize
        }

        ZSTD_c_blockDelimiters => {
            if ZSTD_cParam_withinBounds(ZSTD_c_blockDelimiters, value) == 0 {
                return ERROR(ZSTD_error_parameter_outOfBound);
            }
            (*CCtxParams).blockDelimiters = value as ZSTD_SequenceFormat_e;
            (*CCtxParams).blockDelimiters as usize
        }

        ZSTD_c_validateSequences => {
            if ZSTD_cParam_withinBounds(ZSTD_c_validateSequences, value) == 0 {
                return ERROR(ZSTD_error_parameter_outOfBound);
            }
            (*CCtxParams).validateSequences = value;
            (*CCtxParams).validateSequences as usize
        }

        ZSTD_c_splitAfterSequences => {
            if ZSTD_cParam_withinBounds(ZSTD_c_splitAfterSequences, value) == 0 {
                return ERROR(ZSTD_error_parameter_outOfBound);
            }
            (*CCtxParams).postBlockSplitter = value as ZSTD_ParamSwitch_e;
            (*CCtxParams).postBlockSplitter as usize
        }

        ZSTD_c_blockSplitterLevel => {
            if ZSTD_cParam_withinBounds(ZSTD_c_blockSplitterLevel, value) == 0 {
                return ERROR(ZSTD_error_parameter_outOfBound);
            }
            (*CCtxParams).preBlockSplitter_level = value;
            (*CCtxParams).preBlockSplitter_level as usize
        }

        ZSTD_c_useRowMatchFinder => {
            if ZSTD_cParam_withinBounds(ZSTD_c_useRowMatchFinder, value) == 0 {
                return ERROR(ZSTD_error_parameter_outOfBound);
            }
            (*CCtxParams).useRowMatchFinder = value as ZSTD_ParamSwitch_e;
            (*CCtxParams).useRowMatchFinder as usize
        }

        ZSTD_c_deterministicRefPrefix => {
            if ZSTD_cParam_withinBounds(ZSTD_c_deterministicRefPrefix, value) == 0 {
                return ERROR(ZSTD_error_parameter_outOfBound);
            }
            (*CCtxParams).deterministicRefPrefix = (value != 0) as c_int;
            (*CCtxParams).deterministicRefPrefix as usize
        }

        ZSTD_c_prefetchCDictTables => {
            if ZSTD_cParam_withinBounds(ZSTD_c_prefetchCDictTables, value) == 0 {
                return ERROR(ZSTD_error_parameter_outOfBound);
            }
            (*CCtxParams).prefetchCDictTables = value as ZSTD_ParamSwitch_e;
            (*CCtxParams).prefetchCDictTables as usize
        }

        ZSTD_c_enableSeqProducerFallback => {
            if ZSTD_cParam_withinBounds(ZSTD_c_enableSeqProducerFallback, value) == 0 {
                return ERROR(ZSTD_error_parameter_outOfBound);
            }
            (*CCtxParams).enableMatchFinderFallback = value;
            (*CCtxParams).enableMatchFinderFallback as usize
        }

        ZSTD_c_maxBlockSize => {
            if value != 0 {
                /* 0 ==> default */
                if ZSTD_cParam_withinBounds(ZSTD_c_maxBlockSize, value) == 0 {
                    return ERROR(ZSTD_error_parameter_outOfBound);
                }
            }
            (*CCtxParams).maxBlockSize = value as usize;
            (*CCtxParams).maxBlockSize
        }

        ZSTD_c_repcodeResolution => {
            if ZSTD_cParam_withinBounds(ZSTD_c_repcodeResolution, value) == 0 {
                return ERROR(ZSTD_error_parameter_outOfBound);
            }
            (*CCtxParams).searchForExternalRepcodes = value as ZSTD_ParamSwitch_e;
            (*CCtxParams).searchForExternalRepcodes as usize
        }

        _ => ERROR(ZSTD_error_parameter_unsupported),
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
            *value = (*CCtxParams).nbWorkers;
        }
        ZSTD_c_jobSize => {
            return ERROR(ZSTD_error_parameter_unsupported);
        }
        ZSTD_c_overlapLog => {
            return ERROR(ZSTD_error_parameter_unsupported);
        }
        ZSTD_c_rsyncable => {
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
 *  no action is performed, parameters are merely stored.
 */
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
        let e = ZSTD_checkCParams(cparams);
        if ERR_isError(e) != 0 {
            return e;
        }
    }
    {
        let e = ZSTD_CCtx_setParameter(cctx, ZSTD_c_windowLog, cparams.windowLog as c_int);
        if ERR_isError(e) != 0 {
            return e;
        }
    }
    {
        let e = ZSTD_CCtx_setParameter(cctx, ZSTD_c_chainLog, cparams.chainLog as c_int);
        if ERR_isError(e) != 0 {
            return e;
        }
    }
    {
        let e = ZSTD_CCtx_setParameter(cctx, ZSTD_c_hashLog, cparams.hashLog as c_int);
        if ERR_isError(e) != 0 {
            return e;
        }
    }
    {
        let e = ZSTD_CCtx_setParameter(cctx, ZSTD_c_searchLog, cparams.searchLog as c_int);
        if ERR_isError(e) != 0 {
            return e;
        }
    }
    {
        let e = ZSTD_CCtx_setParameter(cctx, ZSTD_c_minMatch, cparams.minMatch as c_int);
        if ERR_isError(e) != 0 {
            return e;
        }
    }
    {
        let e = ZSTD_CCtx_setParameter(cctx, ZSTD_c_targetLength, cparams.targetLength as c_int);
        if ERR_isError(e) != 0 {
            return e;
        }
    }
    {
        let e = ZSTD_CCtx_setParameter(cctx, ZSTD_c_strategy, cparams.strategy as c_int);
        if ERR_isError(e) != 0 {
            return e;
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
        let e = ZSTD_CCtx_setParameter(
            cctx,
            ZSTD_c_contentSizeFlag,
            (fparams.contentSizeFlag != 0) as c_int,
        );
        if ERR_isError(e) != 0 {
            return e;
        }
    }
    {
        let e = ZSTD_CCtx_setParameter(
            cctx,
            ZSTD_c_checksumFlag,
            (fparams.checksumFlag != 0) as c_int,
        );
        if ERR_isError(e) != 0 {
            return e;
        }
    }
    {
        let e = ZSTD_CCtx_setParameter(
            cctx,
            ZSTD_c_dictIDFlag,
            (fparams.noDictIDFlag == 0) as c_int,
        );
        if ERR_isError(e) != 0 {
            return e;
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
        let e = ZSTD_checkCParams(params.cParams);
        if ERR_isError(e) != 0 {
            return e;
        }
    }
    /* Next set fParams, because this could fail if the cctx isn't in init stage. */
    {
        let e = ZSTD_CCtx_setFParams(cctx, params.fParams);
        if ERR_isError(e) != 0 {
            return e;
        }
    }
    /* Finally set cParams, which should succeed. */
    {
        let e = ZSTD_CCtx_setCParams(cctx, params.cParams);
        if ERR_isError(e) != 0 {
            return e;
        }
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_CCtx_setPledgedSrcSize(
    cctx: *mut ZSTD_CCtx,
    pledgedSrcSize: u64,
) -> usize {
    if (*cctx).streamStage != zcss_init {
        return ERROR(ZSTD_error_stage_wrong);
    }
    (*cctx).pledgedSrcSizePlusOne = pledgedSrcSize.wrapping_add(1);
    0
}

/* Forward declarations (defined later in zstd_compress.c):
 *   ZSTD_dedicatedDictSearch_getCParams
 *   ZSTD_dedicatedDictSearch_isSupported
 *   ZSTD_dedicatedDictSearch_revertCParams
 */

/**
 * Initializes the local dictionary using requested parameters.
 */
pub unsafe fn ZSTD_initLocalDict(cctx: *mut ZSTD_CCtx) -> usize {
    let dl: *mut ZSTD_localDict = &mut (*cctx).localDict;
    if (*dl).dict.is_null() {
        /* No local dictionary. */
        return 0;
    }
    if !(*dl).cdict.is_null() {
        /* Local dictionary already initialized. */
        return 0;
    }

    (*dl).cdict = crate::compress::zstd_compress::ZSTD_createCDict_advanced2(
        (*dl).dict,
        (*dl).dictSize,
        ZSTD_dlm_byRef,
        (*dl).dictContentType,
        &(*cctx).requestedParams,
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
        dictBuffer = ZSTD_customMalloc(dictSize, (*cctx).customMem);
        if dictBuffer.is_null() {
            return ERROR(ZSTD_error_memory_allocation);
        }
        ZSTD_memcpy(dictBuffer, dict, dictSize);
        (*cctx).localDict.dictBuffer = dictBuffer; /* owned ptr to free */
        (*cctx).localDict.dict = dictBuffer; /* read-only reference */
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
    pool: *mut POOL_ctx,
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
        return ZSTD_CCtxParams_reset(&mut (*cctx).requestedParams);
    }
    0
}

/** ZSTD_checkCParams() :
    control CParam values remain within authorized range.
    @return : 0, or an error code if one value is beyond authorized range */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_checkCParams(cParams: ZSTD_compressionParameters) -> usize {
    if ZSTD_cParam_withinBounds(ZSTD_c_windowLog, cParams.windowLog as c_int) == 0 {
        return ERROR(ZSTD_error_parameter_outOfBound);
    }
    if ZSTD_cParam_withinBounds(ZSTD_c_chainLog, cParams.chainLog as c_int) == 0 {
        return ERROR(ZSTD_error_parameter_outOfBound);
    }
    if ZSTD_cParam_withinBounds(ZSTD_c_hashLog, cParams.hashLog as c_int) == 0 {
        return ERROR(ZSTD_error_parameter_outOfBound);
    }
    if ZSTD_cParam_withinBounds(ZSTD_c_searchLog, cParams.searchLog as c_int) == 0 {
        return ERROR(ZSTD_error_parameter_outOfBound);
    }
    if ZSTD_cParam_withinBounds(ZSTD_c_minMatch, cParams.minMatch as c_int) == 0 {
        return ERROR(ZSTD_error_parameter_outOfBound);
    }
    if ZSTD_cParam_withinBounds(ZSTD_c_targetLength, cParams.targetLength as c_int) == 0 {
        return ERROR(ZSTD_error_parameter_outOfBound);
    }
    if ZSTD_cParam_withinBounds(ZSTD_c_strategy, cParams.strategy as c_int) == 0 {
        return ERROR(ZSTD_error_parameter_outOfBound);
    }
    0
}

/** ZSTD_clampCParams() :
 *  make CParam values within valid range.
 *  @return : valid CParams */
pub unsafe fn ZSTD_clampCParams(
    mut cParams: ZSTD_compressionParameters,
) -> ZSTD_compressionParameters {
    {
        let bounds = ZSTD_cParam_getBounds(ZSTD_c_windowLog);
        if (cParams.windowLog as c_int) < bounds.lowerBound {
            cParams.windowLog = bounds.lowerBound as c_uint;
        } else if (cParams.windowLog as c_int) > bounds.upperBound {
            cParams.windowLog = bounds.upperBound as c_uint;
        }
    }
    {
        let bounds = ZSTD_cParam_getBounds(ZSTD_c_chainLog);
        if (cParams.chainLog as c_int) < bounds.lowerBound {
            cParams.chainLog = bounds.lowerBound as c_uint;
        } else if (cParams.chainLog as c_int) > bounds.upperBound {
            cParams.chainLog = bounds.upperBound as c_uint;
        }
    }
    {
        let bounds = ZSTD_cParam_getBounds(ZSTD_c_hashLog);
        if (cParams.hashLog as c_int) < bounds.lowerBound {
            cParams.hashLog = bounds.lowerBound as c_uint;
        } else if (cParams.hashLog as c_int) > bounds.upperBound {
            cParams.hashLog = bounds.upperBound as c_uint;
        }
    }
    {
        let bounds = ZSTD_cParam_getBounds(ZSTD_c_searchLog);
        if (cParams.searchLog as c_int) < bounds.lowerBound {
            cParams.searchLog = bounds.lowerBound as c_uint;
        } else if (cParams.searchLog as c_int) > bounds.upperBound {
            cParams.searchLog = bounds.upperBound as c_uint;
        }
    }
    {
        let bounds = ZSTD_cParam_getBounds(ZSTD_c_minMatch);
        if (cParams.minMatch as c_int) < bounds.lowerBound {
            cParams.minMatch = bounds.lowerBound as c_uint;
        } else if (cParams.minMatch as c_int) > bounds.upperBound {
            cParams.minMatch = bounds.upperBound as c_uint;
        }
    }
    {
        let bounds = ZSTD_cParam_getBounds(ZSTD_c_targetLength);
        if (cParams.targetLength as c_int) < bounds.lowerBound {
            cParams.targetLength = bounds.lowerBound as c_uint;
        } else if (cParams.targetLength as c_int) > bounds.upperBound {
            cParams.targetLength = bounds.upperBound as c_uint;
        }
    }
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

/** ZSTD_cycleLog() :
 *  condition for correct operation : hashLog > 1 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_cycleLog(hashLog: U32, strat: ZSTD_strategy) -> U32 {
    let btScale: U32 = ((strat as U32) >= (ZSTD_btlazy2 as U32)) as U32;
    hashLog.wrapping_sub(btScale)
}

/** ZSTD_dictAndWindowLog() :
 * Returns an adjusted window log that is large enough to fit the source and the dictionary.
 */
pub unsafe fn ZSTD_dictAndWindowLog(windowLog: U32, srcSize: U64, dictSize: U64) -> U32 {
    let maxWindowSize: U64 = 1u64 << ZSTD_WINDOWLOG_MAX;
    /* No dictionary ==> No change */
    if dictSize == 0 {
        return windowLog;
    }
    {
        let windowSize: U64 = 1u64 << windowLog;
        let dictAndWindowSize: U64 = dictSize.wrapping_add(windowSize);
        if windowSize >= dictSize.wrapping_add(srcSize) {
            windowLog /* Window size large enough already */
        } else if dictAndWindowSize >= maxWindowSize {
            ZSTD_WINDOWLOG_MAX as U32 /* Larger than max window log */
        } else {
            ZSTD_highbit32((dictAndWindowSize as U32).wrapping_sub(1)) + 1
        }
    }
}

/** ZSTD_adjustCParams_internal() :
 *  optimize `cPar` for a specified input (`srcSize` and `dictSize`).
 */
pub unsafe fn ZSTD_adjustCParams_internal(
    mut cPar: ZSTD_compressionParameters,
    mut srcSize: u64,
    mut dictSize: usize,
    mode: ZSTD_CParamMode_e,
    mut useRowMatchFinder: ZSTD_ParamSwitch_e,
) -> ZSTD_compressionParameters {
    let minSrcSize: U64 = 513; /* (1<<9) + 1 */
    let maxWindowResize: U64 = 1u64 << (ZSTD_WINDOWLOG_MAX - 1);

    match mode {
        ZSTD_cpm_unknown | ZSTD_cpm_noAttachDict => {
            /* If we don't know the source size, don't make any
             * assumptions about it. We will already have selected
             * smaller parameters if a dictionary is in use.
             */
        }
        ZSTD_cpm_createCDict => {
            /* Assume a small source size when creating a dictionary
             * with an unknown source size.
             */
            if dictSize != 0 && srcSize == ZSTD_CONTENTSIZE_UNKNOWN {
                srcSize = minSrcSize;
            }
        }
        ZSTD_cpm_attachDict => {
            /* Dictionary has its own dedicated parameters which have
             * already been selected. We are selecting parameters
             * for only the source.
             */
            dictSize = 0;
        }
        _ => {}
    }

    /* resize windowLog if input is small enough, to use less memory */
    if (srcSize <= maxWindowResize) && ((dictSize as U64) <= maxWindowResize) {
        let tSize: U32 = srcSize.wrapping_add(dictSize as u64) as U32;
        let hashSizeMin: U32 = 1u32 << ZSTD_HASHLOG_MIN;
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
        if cPar.hashLog > dictAndWindowLog + 1 {
            cPar.hashLog = dictAndWindowLog + 1;
        }
        if cycleLog > dictAndWindowLog {
            cPar.chainLog = cPar.chainLog.wrapping_sub(cycleLog - dictAndWindowLog);
        }
    }

    if cPar.windowLog < ZSTD_WINDOWLOG_ABSOLUTEMIN {
        cPar.windowLog = ZSTD_WINDOWLOG_ABSOLUTEMIN; /* minimum wlog required for valid frame header */
    }

    /* We can't use more than 32 bits of hash in total, so that means that we require:
     * (hashLog + 8) <= 32 && (chainLog + 8) <= 32
     */
    if mode == ZSTD_cpm_createCDict && ZSTD_CDictIndicesAreTagged(&cPar) != 0 {
        let maxShortCacheHashLog: U32 = 32 - ZSTD_SHORT_CACHE_TAG_BITS;
        if cPar.hashLog > maxShortCacheHashLog {
            cPar.hashLog = maxShortCacheHashLog;
        }
        if cPar.chainLog > maxShortCacheHashLog {
            cPar.chainLog = maxShortCacheHashLog;
        }
    }

    /* At this point, we aren't 100% sure if we are using the row match finder.
     * Unless it is explicitly disabled, conservatively assume that it is enabled.
     */
    if useRowMatchFinder == ZSTD_ps_auto {
        useRowMatchFinder = ZSTD_ps_enable;
    }

    /* We can't hash more than 32-bits in total. So that means that we require:
     * (hashLog - rowLog + 8) <= 32
     */
    if ZSTD_rowMatchFinderUsed(cPar.strategy, useRowMatchFinder) != 0 {
        /* Switch to 32-entry rows if searchLog is 5 (or more) */
        let rowLog: U32 = {
            let m = if cPar.searchLog < 6 { cPar.searchLog } else { 6 };
            if 4 > m { 4 } else { m }
        };
        let maxRowHashLog: U32 = 32 - ZSTD_ROW_HASH_TAG_BITS;
        let maxHashLog: U32 = maxRowHashLog + rowLog;
        if cPar.hashLog > maxHashLog {
            cPar.hashLog = maxHashLog;
        }
    }

    cPar
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_adjustCParams(
    mut cPar: ZSTD_compressionParameters,
    mut srcSize: u64,
    dictSize: usize,
) -> ZSTD_compressionParameters {
    cPar = ZSTD_clampCParams(cPar); /* resulting cPar is necessarily valid */
    if srcSize == 0 {
        srcSize = ZSTD_CONTENTSIZE_UNKNOWN;
    }
    ZSTD_adjustCParams_internal(cPar, srcSize, dictSize, ZSTD_cpm_unknown, ZSTD_ps_auto)
}

/* Forward declarations (defined later in zstd_compress.c):
 *   ZSTD_getCParams_internal
 *   ZSTD_getParams_internal
 */

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
    cParams = crate::compress::zstd_compress::ZSTD_getCParams_internal(
        (*CCtxParams).compressionLevel,
        srcSizeHint,
        dictSize,
        mode,
    );
    if (*CCtxParams).ldmParams.enableLdm == ZSTD_ps_enable {
        cParams.windowLog = ZSTD_LDM_DEFAULT_WINDOW_LOG;
    }
    ZSTD_overrideCParams(&mut cParams, &(*CCtxParams).cParams);
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
        1usize << (*cParams).chainLog
    } else {
        0
    };
    let hSize: usize = 1usize << (*cParams).hashLog;
    let hashLog3: U32 = if forCCtx != 0 && (*cParams).minMatch == 3 {
        if ZSTD_HASHLOG3_MAX < (*cParams).windowLog {
            ZSTD_HASHLOG3_MAX
        } else {
            (*cParams).windowLog
        }
    } else {
        0
    };
    let h3Size: usize = if hashLog3 != 0 {
        1usize << hashLog3
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
    )
        + ZSTD_cwksp_aligned64_alloc_size((MaxLL as usize + 1) * core::mem::size_of::<U32>())
        + ZSTD_cwksp_aligned64_alloc_size((MaxOff as usize + 1) * core::mem::size_of::<U32>())
        + ZSTD_cwksp_aligned64_alloc_size((1usize << Litbits) * core::mem::size_of::<U32>())
        + ZSTD_cwksp_aligned64_alloc_size(
            ZSTD_OPT_SIZE * core::mem::size_of::<ZSTD_match_t>(),
        )
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

    tableSpace + optSpace + slackSpace + lazyAdditionalSpace
}

/* Helper function for calculating memory requirements.
 * Gives a tighter bound than ZSTD_sequenceBound() by taking minMatch into account. */
pub unsafe fn ZSTD_maxNbSeq(
    blockSize: usize,
    minMatch: c_uint,
    useSequenceProducer: c_int,
) -> usize {
    let divider: U32 = if minMatch == 3 || useSequenceProducer != 0 {
        3
    } else {
        4
    };
    blockSize / divider as usize
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
    let windowSize: usize = {
        let inner = {
            let a = 1u64 << (*cParams).windowLog;
            if a < pledgedSrcSize { a } else { pledgedSrcSize }
        };
        (if 1u64 > inner { 1u64 } else { inner }) as usize
    };
    let blockSize: usize = {
        let a = ZSTD_resolveMaxBlockSize(maxBlockSize);
        if a < windowSize { a } else { windowSize }
    };
    let maxNbSeq: usize = ZSTD_maxNbSeq(blockSize, (*cParams).minMatch, useSequenceProducer);
    let tokenSpace: usize = ZSTD_cwksp_alloc_size(WILDCOPY_OVERLENGTH + blockSize)
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

    let ldmSpace: usize = crate::compress::zstd_ldm::ZSTD_ldm_getTableSize(*ldmParams);
    let maxNbLdmSeq: usize =
        crate::compress::zstd_ldm::ZSTD_ldm_getMaxNbSeq(*ldmParams, blockSize);
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

    let maxNbExternalSeq: usize = crate::compress::zstd_compress::ZSTD_sequenceBound(blockSize);
    let externalSeqSpace: usize = if useSequenceProducer != 0 {
        ZSTD_cwksp_aligned64_alloc_size(
            maxNbExternalSeq * core::mem::size_of::<ZSTD_Sequence>(),
        )
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
    let cParams = ZSTD_getCParamsFromCCtxParams(
        params,
        ZSTD_CONTENTSIZE_UNKNOWN,
        0,
        ZSTD_cpm_noAttachDict,
    );
    let useRowMatchFinder =
        ZSTD_resolveRowMatchFinderMode((*params).useRowMatchFinder, &cParams);

    if (*params).nbWorkers > 0 {
        return ERROR(ZSTD_error_GENERIC);
    }
    /* estimateCCtxSize is for one-shot compression. So no buffers should
     * be needed. However, we still allocate two 0-sized buffers, which can
     * take space under ASAN. */
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
        /* Pick bigger of not using and using row-based matchfinder */
        let noRowCCtxSize: usize;
        let rowCCtxSize: usize;
        initialParams.useRowMatchFinder = ZSTD_ps_disable;
        noRowCCtxSize = ZSTD_estimateCCtxSize_usingCCtxParams(&initialParams);
        initialParams.useRowMatchFinder = ZSTD_ps_enable;
        rowCCtxSize = ZSTD_estimateCCtxSize_usingCCtxParams(&initialParams);
        if noRowCCtxSize > rowCCtxSize {
            noRowCCtxSize
        } else {
            rowCCtxSize
        }
    } else {
        ZSTD_estimateCCtxSize_usingCCtxParams(&initialParams)
    }
}

pub static srcSizeTiers: [u64; 4] = [
    16 * (1 << 10),
    128 * (1 << 10),
    256 * (1 << 10),
    ZSTD_CONTENTSIZE_UNKNOWN,
];

pub unsafe fn ZSTD_estimateCCtxSize_internal(compressionLevel: c_int) -> usize {
    let mut tier: c_int = 0;
    let mut largestSize: usize = 0;
    while tier < 4 {
        /* Choose the set of cParams for a given level across all srcSizes that
         * give the largest cctxSize */
        let cParams = crate::compress::zstd_compress::ZSTD_getCParams_internal(
            compressionLevel,
            srcSizeTiers[tier as usize],
            0,
            ZSTD_cpm_noAttachDict,
        );
        let cand = ZSTD_estimateCCtxSize_usingCParams(cParams);
        largestSize = if cand > largestSize { cand } else { largestSize };
        tier += 1;
    }
    largestSize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_estimateCCtxSize(compressionLevel: c_int) -> usize {
    let mut level: c_int;
    let mut memBudget: usize = 0;
    level = if compressionLevel < 1 {
        compressionLevel
    } else {
        1
    };
    while level <= compressionLevel {
        /* Ensure monotonically increasing memory usage as compression level increases */
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
    if (*params).nbWorkers > 0 {
        return ERROR(ZSTD_error_GENERIC);
    }
    {
        let cParams = ZSTD_getCParamsFromCCtxParams(
            params,
            ZSTD_CONTENTSIZE_UNKNOWN,
            0,
            ZSTD_cpm_noAttachDict,
        );
        let blockSize: usize = {
            let a = ZSTD_resolveMaxBlockSize((*params).maxBlockSize);
            let b = 1usize << cParams.windowLog;
            if a < b { a } else { b }
        };
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
        /* Pick bigger of not using and using row-based matchfinder */
        let noRowCCtxSize: usize;
        let rowCCtxSize: usize;
        initialParams.useRowMatchFinder = ZSTD_ps_disable;
        noRowCCtxSize = ZSTD_estimateCStreamSize_usingCCtxParams(&initialParams);
        initialParams.useRowMatchFinder = ZSTD_ps_enable;
        rowCCtxSize = ZSTD_estimateCStreamSize_usingCCtxParams(&initialParams);
        if noRowCCtxSize > rowCCtxSize {
            noRowCCtxSize
        } else {
            rowCCtxSize
        }
    } else {
        ZSTD_estimateCStreamSize_usingCCtxParams(&initialParams)
    }
}

pub unsafe fn ZSTD_estimateCStreamSize_internal(compressionLevel: c_int) -> usize {
    let cParams = crate::compress::zstd_compress::ZSTD_getCParams_internal(
        compressionLevel,
        ZSTD_CONTENTSIZE_UNKNOWN,
        0,
        ZSTD_cpm_noAttachDict,
    );
    ZSTD_estimateCStreamSize_usingCParams(cParams)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_estimateCStreamSize(compressionLevel: c_int) -> usize {
    let mut level: c_int;
    let mut memBudget: usize = 0;
    level = if compressionLevel < 1 {
        compressionLevel
    } else {
        1
    };
    while level <= compressionLevel {
        let newMB = ZSTD_estimateCStreamSize_internal(level);
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
        let mut fp = ZSTD_frameProgression::default();
        let buffered: usize = if (*cctx).inBuff.is_null() {
            0
        } else {
            (*cctx).inBuffPos.wrapping_sub((*cctx).inToCompress)
        };
        fp.ingested = (*cctx).consumedSrcSize.wrapping_add(buffered as u64);
        fp.consumed = (*cctx).consumedSrcSize;
        fp.produced = (*cctx).producedCSize;
        fp.flushed = (*cctx).producedCSize; /* simplified */
        fp.currentJobID = 0;
        fp.nbActiveWorkers = 0;
        fp
    }
}

/* ZSTD_toFlushNow()
 *  Only useful for multithreading scenarios currently (nbWorkers >= 1).
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_toFlushNow(_cctx: *mut ZSTD_CCtx) -> usize {
    0
}

pub unsafe fn ZSTD_assertEqualCParams(
    _cParams1: ZSTD_compressionParameters,
    _cParams2: ZSTD_compressionParameters,
) {
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_reset_compressedBlockState(bs: *mut ZSTD_compressedBlockState_t) {
    let mut i: c_int = 0;
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
    ZSTD_window_clear(&mut (*ms).window);

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
pub type ZSTD_compResetPolicy_e = c_uint;
pub const ZSTDcrp_makeClean: ZSTD_compResetPolicy_e = 0;
pub const ZSTDcrp_leaveDirty: ZSTD_compResetPolicy_e = 1;

/**
 * Controls, for this matchState reset, whether indexing can continue where it
 * left off (ZSTDirp_continue), or whether it needs to be restarted from zero
 * (ZSTDirp_reset).
 */
pub type ZSTD_indexResetPolicy_e = c_uint;
pub const ZSTDirp_continue: ZSTD_indexResetPolicy_e = 0;
pub const ZSTDirp_reset: ZSTD_indexResetPolicy_e = 1;

pub type ZSTD_resetTarget_e = c_uint;
pub const ZSTD_resetTarget_CDict: ZSTD_resetTarget_e = 0;
pub const ZSTD_resetTarget_CCtx: ZSTD_resetTarget_e = 1;

/* Mixes bits in a 64 bits in a value, based on XXH3_rrmxmx */
pub unsafe fn ZSTD_bitmix(mut val: U64, len: U64) -> U64 {
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
        1usize << (*cParams).chainLog
    } else {
        0
    };
    let hSize: usize = 1usize << (*cParams).hashLog;
    let hashLog3: U32 = if (forWho == ZSTD_resetTarget_CCtx) && (*cParams).minMatch == 3 {
        if ZSTD_HASHLOG3_MAX < (*cParams).windowLog {
            ZSTD_HASHLOG3_MAX
        } else {
            (*cParams).windowLog
        }
    } else {
        0
    };
    let h3Size: usize = if hashLog3 != 0 {
        1usize << hashLog3
    } else {
        0
    };

    if forceResetIndex == ZSTDirp_reset {
        ZSTD_window_init(&mut (*ms).window);
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
            (*ms).tagTable =
                ZSTD_cwksp_reserve_aligned_init_once(ws, tagTableSize) as *mut BYTE;
            ZSTD_advanceHashSalt(ms);
        } else {
            /* When we are not salting we want to always memset the memory */
            (*ms).tagTable = ZSTD_cwksp_reserve_aligned64(ws, tagTableSize) as *mut BYTE;
            ZSTD_memset((*ms).tagTable as *mut c_void, 0, tagTableSize);
            (*ms).hashSalt = 0;
        }
        {
            /* Switch to 32-entry rows if searchLog is 5 (or more) */
            let rowLog: U32 = {
                let m = if (*cParams).searchLog < 6 {
                    (*cParams).searchLog
                } else {
                    6
                };
                if 4 > m { 4 } else { m }
            };
            (*ms).rowHashLog = (*cParams).hashLog - rowLog;
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
pub const ZSTD_INDEXOVERFLOW_MARGIN: U32 = 16 * (1 << 20);

pub unsafe fn ZSTD_indexTooCloseToMax(w: ZSTD_window_t) -> c_int {
    (((w.nextSrc as usize).wrapping_sub(w.base as usize))
        > (ZSTD_CURRENT_MAX().wrapping_sub(ZSTD_INDEXOVERFLOW_MARGIN)) as usize) as c_int
}

/** ZSTD_dictTooBig():
 * When dictionaries are larger than ZSTD_CHUNKSIZE_MAX they can't be loaded in
 * one go generically.
 */
pub unsafe fn ZSTD_dictTooBig(loadedDictSize: usize) -> c_int {
    (loadedDictSize > ZSTD_CHUNKSIZE_MAX() as usize) as c_int
}


