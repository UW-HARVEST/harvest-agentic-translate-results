//! Literal, semantics-preserving transliteration of PART 1 of
//! `compress/zstd_compress.c` (lines 1..2686): CCtx/CCtxParams lifecycle,
//! parameter handling, sizing estimates and context reset. Plus a handful of
//! functions physically located later in the C file (dedicatedDictSearch
//! get/is/revert CParams, getCParamRowSize, getCParams(_internal), getParams
//! (_internal), maxCLevel/minCLevel/defaultCLevel) that are pulled in here
//! because they depend on the clevels table and to avoid a circular dependency
//! with the frame agent's file.
//!
//! Build config: DYNAMIC_BMI2=0, no ZSTD_MULTITHREAD, DEBUGLEVEL 0 (asserts /
//! DEBUGLOG dropped), ZSTD_TRACE==1 (the ZSTD_CCtx_s::traceCtx field IS present,
//! but trace call sites are guarded no-ops).
#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(unused_parens)]
#![allow(unused_assignments)]
#![allow(unused_variables)]

use core::ffi::{c_char, c_int, c_uint, c_void};
use core::ptr::{null, null_mut};

use crate::common::bits::{ZSTD_highbit32, ZSTD_rotateRight_U64};
use crate::common::error_private::*;
use crate::common::fse::FSE_repeat_none;
use crate::common::huf::HUF_repeat_none;
use crate::common::mem::*;
use crate::common::xxhash::ZSTD_XXH64_reset;
use crate::common::zstd_common::ZSTD_isError;
use crate::common::zstd_h::*;
use crate::common::zstd_internal::*;
/* Disambiguate: ZSTD_customMem is declared in both zstd_h and zstd_internal;
 * the CCtx/CCtxParams struct fields use the zstd_internal one. */
use crate::common::zstd_internal::ZSTD_customMem;

use crate::compress::clevels::{ZSTD_defaultCParameters, ZSTD_MAX_CLEVEL};
use crate::compress::zstd_compress_internal::*;
use crate::compress::zstd_cwksp::*;
use crate::compress::zstd_lazy::{ZSTD_LAZY_DDSS_BUCKET_LOG, ZSTD_ROW_HASH_TAG_BITS};
use crate::compress::zstd_ldm::{
    ZSTD_ldm_adjustParameters, ZSTD_ldm_getMaxNbSeq, ZSTD_ldm_getTableSize,
};

/* ***************************************************************
*  Tuning parameters / local #defines
*****************************************************************/
/* #ifndef ZSTD_HASHLOG3_MAX / #define ZSTD_HASHLOG3_MAX 17 */
pub const ZSTD_HASHLOG3_MAX: c_uint = 17;

/* zstd.h experimental alias missing from the foundation zstd_h.rs:
 *   #define ZSTD_c_rsyncable ZSTD_c_experimentalParam1 */
pub const ZSTD_c_rsyncable: ZSTD_cParameter = ZSTD_c_experimentalParam1;

/* #define ZSTD_NO_CLEVEL 0 */
pub const ZSTD_NO_CLEVEL: c_int = 0;

/* #define ZSTD_INDEXOVERFLOW_MARGIN (16 MB) */
const ZSTD_INDEXOVERFLOW_MARGIN: size_t = 16 * (1 << 20);

/* #define ZSTD_ROWSIZE 16 */
const ZSTD_ROWSIZE: U32 = 16;

/* zstd.h: #define ZSTD_BLOCKSPLITTER_LEVEL_MAX 6 */
pub const ZSTD_BLOCKSPLITTER_LEVEL_MAX: c_int = 6;

/* zstd_ldm.h: #define ZSTD_LDM_DEFAULT_WINDOW_LOG ZSTD_WINDOWLOG_LIMIT_DEFAULT */
pub const ZSTD_LDM_DEFAULT_WINDOW_LOG: c_uint = ZSTD_WINDOWLOG_LIMIT_DEFAULT as c_uint;

/* Exported symbols located outside this line-range that we call. */
unsafe extern "C" {
    fn ZSTD_freeCDict(cdict: *mut ZSTD_CDict) -> size_t;
    fn ZSTD_sizeof_CDict(cdict: *const ZSTD_CDict) -> size_t;
    fn ZSTD_createCDict_advanced2(
        dict: *const c_void,
        dictSize: size_t,
        dictLoadMethod: ZSTD_dictLoadMethod_e,
        dictContentType: ZSTD_dictContentType_e,
        originalCctxParams: *const ZSTD_CCtx_params,
        customMem: ZSTD_customMem,
    ) -> *mut ZSTD_CDict;
    fn ZSTD_sequenceBound(srcSize: size_t) -> size_t;
}

/*-*************************************
*  Helper functions
***************************************/
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBound(srcSize: size_t) -> size_t {
    let r: size_t = ZSTD_COMPRESSBOUND(srcSize);
    if r == 0 {
        return ERROR(ZSTD_error_srcSize_wrong);
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

pub unsafe fn ZSTD_initCCtx(cctx: *mut ZSTD_CCtx, memManager: ZSTD_customMem) {
    ZSTD_memset(cctx as *mut u8, 0, core::mem::size_of::<ZSTD_CCtx>() as size_t);
    (*cctx).customMem = memManager;
    (*cctx).bmi2 = ZSTD_cpuSupportsBmi2();
    {
        let _err: size_t = ZSTD_CCtx_reset(cctx, ZSTD_reset_parameters);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_createCCtx_advanced(customMem: ZSTD_customMem) -> *mut ZSTD_CCtx {
    if ((customMem.customAlloc.is_none()) as c_int) ^ ((customMem.customFree.is_none()) as c_int)
        != 0
    {
        return null_mut();
    }
    {
        let cctx: *mut ZSTD_CCtx = ZSTD_customMalloc(
            core::mem::size_of::<ZSTD_CCtx>() as size_t,
            customMem,
        ) as *mut ZSTD_CCtx;
        if cctx.is_null() {
            return null_mut();
        }
        ZSTD_initCCtx(cctx, customMem);
        cctx
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_initStaticCCtx(
    workspace: *mut c_void,
    workspaceSize: size_t,
) -> *mut ZSTD_CCtx {
    let mut ws: ZSTD_cwksp = core::mem::zeroed();
    let cctx: *mut ZSTD_CCtx;
    if workspaceSize <= core::mem::size_of::<ZSTD_CCtx>() as size_t {
        return null_mut();
    }
    if (workspace as size_t) & 7 != 0 {
        return null_mut();
    }
    ZSTD_cwksp_init(&mut ws, workspace, workspaceSize, ZSTD_cwksp_static_alloc);

    cctx = ZSTD_cwksp_reserve_object(&mut ws, core::mem::size_of::<ZSTD_CCtx>() as size_t)
        as *mut ZSTD_CCtx;
    if cctx.is_null() {
        return null_mut();
    }

    ZSTD_memset(cctx as *mut u8, 0, core::mem::size_of::<ZSTD_CCtx>() as size_t);
    ZSTD_cwksp_move(&mut (*cctx).workspace, &mut ws);
    (*cctx).staticSize = workspaceSize;

    if ZSTD_cwksp_check_available(
        &mut (*cctx).workspace,
        TMP_WORKSPACE_SIZE + 2 * core::mem::size_of::<ZSTD_compressedBlockState_t>() as size_t,
    ) == 0
    {
        return null_mut();
    }
    (*cctx).blockState.prevCBlock = ZSTD_cwksp_reserve_object(
        &mut (*cctx).workspace,
        core::mem::size_of::<ZSTD_compressedBlockState_t>() as size_t,
    ) as *mut ZSTD_compressedBlockState_t;
    (*cctx).blockState.nextCBlock = ZSTD_cwksp_reserve_object(
        &mut (*cctx).workspace,
        core::mem::size_of::<ZSTD_compressedBlockState_t>() as size_t,
    ) as *mut ZSTD_compressedBlockState_t;
    (*cctx).tmpWorkspace = ZSTD_cwksp_reserve_object(&mut (*cctx).workspace, TMP_WORKSPACE_SIZE);
    (*cctx).tmpWkspSize = TMP_WORKSPACE_SIZE;
    /* C: cctx->bmi2 = ZSTD_cpuid_bmi2(ZSTD_cpuid()); DYNAMIC_BMI2=0 -> non-behavioural. */
    (*cctx).bmi2 = ZSTD_cpuSupportsBmi2();
    cctx
}

pub unsafe fn ZSTD_clearAllDicts(cctx: *mut ZSTD_CCtx) {
    ZSTD_customFree((*cctx).localDict.dictBuffer, (*cctx).customMem);
    ZSTD_freeCDict((*cctx).localDict.cdict);
    ZSTD_memset(
        &mut (*cctx).localDict as *mut ZSTD_localDict as *mut u8,
        0,
        core::mem::size_of::<ZSTD_localDict>() as size_t,
    );
    ZSTD_memset(
        &mut (*cctx).prefixDict as *mut ZSTD_prefixDict as *mut u8,
        0,
        core::mem::size_of::<ZSTD_prefixDict>() as size_t,
    );
    (*cctx).cdict = null();
}

pub unsafe fn ZSTD_sizeof_localDict(dict: ZSTD_localDict) -> size_t {
    let bufferSize: size_t = if !dict.dictBuffer.is_null() {
        dict.dictSize
    } else {
        0
    };
    let cdictSize: size_t = ZSTD_sizeof_CDict(dict.cdict);
    bufferSize + cdictSize
}

pub unsafe fn ZSTD_freeCCtxContent(cctx: *mut ZSTD_CCtx) {
    ZSTD_clearAllDicts(cctx);
    ZSTD_cwksp_free(&mut (*cctx).workspace, (*cctx).customMem);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_freeCCtx(cctx: *mut ZSTD_CCtx) -> size_t {
    if cctx.is_null() {
        return 0;
    }
    if (*cctx).staticSize != 0 {
        return ERROR(ZSTD_error_memory_allocation);
    }
    {
        let cctxInWorkspace: c_int =
            ZSTD_cwksp_owns_buffer(&(*cctx).workspace, cctx as *const c_void);
        ZSTD_freeCCtxContent(cctx);
        if cctxInWorkspace == 0 {
            ZSTD_customFree(cctx as *mut c_void, (*cctx).customMem);
        }
    }
    0
}

pub unsafe fn ZSTD_sizeof_mtctx(_cctx: *const ZSTD_CCtx) -> size_t {
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_sizeof_CCtx(cctx: *const ZSTD_CCtx) -> size_t {
    if cctx.is_null() {
        return 0;
    }
    (if (*cctx).workspace.workspace == cctx as *mut c_void {
        0
    } else {
        core::mem::size_of::<ZSTD_CCtx>() as size_t
    }) + ZSTD_cwksp_sizeof(&(*cctx).workspace)
        + ZSTD_sizeof_localDict(core::ptr::read(&(*cctx).localDict))
        + ZSTD_sizeof_mtctx(cctx)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_sizeof_CStream(zcs: *const ZSTD_CStream) -> size_t {
    ZSTD_sizeof_CCtx(zcs)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_getSeqStore(ctx: *const ZSTD_CCtx) -> *const SeqStore_t {
    &(*ctx).seqStore
}

pub unsafe fn ZSTD_rowMatchFinderSupported(strategy: ZSTD_strategy) -> c_int {
    (strategy >= ZSTD_greedy && strategy <= ZSTD_lazy2) as c_int
}

pub unsafe fn ZSTD_rowMatchFinderUsed(strategy: ZSTD_strategy, mode: ZSTD_ParamSwitch_e) -> c_int {
    (ZSTD_rowMatchFinderSupported(strategy) != 0 && (mode == ZSTD_ps_enable)) as c_int
}

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

pub unsafe fn ZSTD_allocateChainTable(
    strategy: ZSTD_strategy,
    useRowMatchFinder: ZSTD_ParamSwitch_e,
    forDDSDict: U32,
) -> c_int {
    (forDDSDict != 0
        || ((strategy != ZSTD_fast) && ZSTD_rowMatchFinderUsed(strategy, useRowMatchFinder) == 0))
        as c_int
}

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

pub unsafe fn ZSTD_resolveMaxBlockSize(maxBlockSize: size_t) -> size_t {
    if maxBlockSize == 0 {
        ZSTD_BLOCKSIZE_MAX as size_t
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

pub unsafe fn ZSTD_CDictIndicesAreTagged(cParams: *const ZSTD_compressionParameters) -> c_int {
    ((*cParams).strategy == ZSTD_fast || (*cParams).strategy == ZSTD_dfast) as c_int
}

pub unsafe fn ZSTD_makeCCtxParamsFromCParams(
    cParams: ZSTD_compressionParameters,
) -> ZSTD_CCtx_params {
    let mut cctxParams: ZSTD_CCtx_params = core::mem::zeroed();
    ZSTD_CCtxParams_init(&mut cctxParams, ZSTD_CLEVEL_DEFAULT);
    cctxParams.cParams = cParams;

    cctxParams.ldmParams.enableLdm = ZSTD_resolveEnableLdm(cctxParams.ldmParams.enableLdm, &cParams);
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
    cctxParams
}

pub unsafe fn ZSTD_createCCtxParams_advanced(customMem: ZSTD_customMem) -> *mut ZSTD_CCtx_params {
    let params: *mut ZSTD_CCtx_params;
    if ((customMem.customAlloc.is_none()) as c_int) ^ ((customMem.customFree.is_none()) as c_int)
        != 0
    {
        return null_mut();
    }
    params = ZSTD_customCalloc(
        core::mem::size_of::<ZSTD_CCtx_params>() as size_t,
        customMem,
    ) as *mut ZSTD_CCtx_params;
    if params.is_null() {
        return null_mut();
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
pub unsafe extern "C" fn ZSTD_freeCCtxParams(params: *mut ZSTD_CCtx_params) -> size_t {
    if params.is_null() {
        return 0;
    }
    ZSTD_customFree(params as *mut c_void, (*params).customMem);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_CCtxParams_reset(params: *mut ZSTD_CCtx_params) -> size_t {
    ZSTD_CCtxParams_init(params, ZSTD_CLEVEL_DEFAULT)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_CCtxParams_init(
    cctxParams: *mut ZSTD_CCtx_params,
    compressionLevel: c_int,
) -> size_t {
    if cctxParams.is_null() {
        return ERROR(ZSTD_error_GENERIC);
    }
    ZSTD_memset(
        cctxParams as *mut u8,
        0,
        core::mem::size_of::<ZSTD_CCtx_params>() as size_t,
    );
    (*cctxParams).compressionLevel = compressionLevel;
    (*cctxParams).fParams.contentSizeFlag = 1;
    0
}

pub unsafe fn ZSTD_CCtxParams_init_internal(
    cctxParams: *mut ZSTD_CCtx_params,
    params: *const ZSTD_parameters,
    compressionLevel: c_int,
) {
    ZSTD_memset(
        cctxParams as *mut u8,
        0,
        core::mem::size_of::<ZSTD_CCtx_params>() as size_t,
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
) -> size_t {
    if cctxParams.is_null() {
        return ERROR(ZSTD_error_GENERIC);
    }
    {
        let err = ZSTD_checkCParams(params.cParams);
        if ERR_isError(err) != 0 {
            return err;
        }
    }
    ZSTD_CCtxParams_init_internal(cctxParams, &params, ZSTD_NO_CLEVEL);
    0
}

pub unsafe fn ZSTD_CCtxParams_setZstdParams(
    cctxParams: *mut ZSTD_CCtx_params,
    params: *const ZSTD_parameters,
) {
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

    if param == ZSTD_c_compressionLevel {
        bounds.lowerBound = ZSTD_minCLevel();
        bounds.upperBound = ZSTD_maxCLevel();
        return bounds;
    }
    if param == ZSTD_c_windowLog {
        bounds.lowerBound = ZSTD_WINDOWLOG_MIN;
        bounds.upperBound = ZSTD_WINDOWLOG_MAX;
        return bounds;
    }
    if param == ZSTD_c_hashLog {
        bounds.lowerBound = ZSTD_HASHLOG_MIN;
        bounds.upperBound = ZSTD_HASHLOG_MAX;
        return bounds;
    }
    if param == ZSTD_c_chainLog {
        bounds.lowerBound = ZSTD_CHAINLOG_MIN;
        bounds.upperBound = ZSTD_CHAINLOG_MAX;
        return bounds;
    }
    if param == ZSTD_c_searchLog {
        bounds.lowerBound = ZSTD_SEARCHLOG_MIN;
        bounds.upperBound = ZSTD_SEARCHLOG_MAX;
        return bounds;
    }
    if param == ZSTD_c_minMatch {
        bounds.lowerBound = ZSTD_MINMATCH_MIN;
        bounds.upperBound = ZSTD_MINMATCH_MAX;
        return bounds;
    }
    if param == ZSTD_c_targetLength {
        bounds.lowerBound = ZSTD_TARGETLENGTH_MIN;
        bounds.upperBound = ZSTD_TARGETLENGTH_MAX;
        return bounds;
    }
    if param == ZSTD_c_strategy {
        bounds.lowerBound = ZSTD_STRATEGY_MIN;
        bounds.upperBound = ZSTD_STRATEGY_MAX;
        return bounds;
    }
    if param == ZSTD_c_contentSizeFlag {
        bounds.lowerBound = 0;
        bounds.upperBound = 1;
        return bounds;
    }
    if param == ZSTD_c_checksumFlag {
        bounds.lowerBound = 0;
        bounds.upperBound = 1;
        return bounds;
    }
    if param == ZSTD_c_dictIDFlag {
        bounds.lowerBound = 0;
        bounds.upperBound = 1;
        return bounds;
    }
    if param == ZSTD_c_nbWorkers {
        bounds.lowerBound = 0;
        bounds.upperBound = 0; /* ZSTD_MULTITHREAD undefined */
        return bounds;
    }
    if param == ZSTD_c_jobSize {
        bounds.lowerBound = 0;
        bounds.upperBound = 0; /* ZSTD_MULTITHREAD undefined */
        return bounds;
    }
    if param == ZSTD_c_overlapLog {
        bounds.lowerBound = 0;
        bounds.upperBound = 0; /* ZSTD_MULTITHREAD undefined */
        return bounds;
    }
    if param == ZSTD_c_enableDedicatedDictSearch {
        bounds.lowerBound = 0;
        bounds.upperBound = 1;
        return bounds;
    }
    if param == ZSTD_c_enableLongDistanceMatching {
        bounds.lowerBound = ZSTD_ps_auto as c_int;
        bounds.upperBound = ZSTD_ps_disable as c_int;
        return bounds;
    }
    if param == ZSTD_c_ldmHashLog {
        bounds.lowerBound = ZSTD_LDM_HASHLOG_MIN;
        bounds.upperBound = ZSTD_LDM_HASHLOG_MAX;
        return bounds;
    }
    if param == ZSTD_c_ldmMinMatch {
        bounds.lowerBound = ZSTD_LDM_MINMATCH_MIN;
        bounds.upperBound = ZSTD_LDM_MINMATCH_MAX;
        return bounds;
    }
    if param == ZSTD_c_ldmBucketSizeLog {
        bounds.lowerBound = ZSTD_LDM_BUCKETSIZELOG_MIN;
        bounds.upperBound = ZSTD_LDM_BUCKETSIZELOG_MAX;
        return bounds;
    }
    if param == ZSTD_c_ldmHashRateLog {
        bounds.lowerBound = ZSTD_LDM_HASHRATELOG_MIN;
        bounds.upperBound = ZSTD_LDM_HASHRATELOG_MAX;
        return bounds;
    }
    /* experimental parameters */
    if param == ZSTD_c_rsyncable {
        bounds.lowerBound = 0;
        bounds.upperBound = 1;
        return bounds;
    }
    if param == ZSTD_c_forceMaxWindow {
        bounds.lowerBound = 0;
        bounds.upperBound = 1;
        return bounds;
    }
    if param == ZSTD_c_format {
        bounds.lowerBound = ZSTD_f_zstd1 as c_int;
        bounds.upperBound = ZSTD_f_zstd1_magicless as c_int;
        return bounds;
    }
    if param == ZSTD_c_forceAttachDict {
        bounds.lowerBound = ZSTD_dictDefaultAttach as c_int;
        bounds.upperBound = ZSTD_dictForceLoad as c_int;
        return bounds;
    }
    if param == ZSTD_c_literalCompressionMode {
        bounds.lowerBound = ZSTD_ps_auto as c_int;
        bounds.upperBound = ZSTD_ps_disable as c_int;
        return bounds;
    }
    if param == ZSTD_c_targetCBlockSize {
        bounds.lowerBound = ZSTD_TARGETCBLOCKSIZE_MIN;
        bounds.upperBound = ZSTD_TARGETCBLOCKSIZE_MAX;
        return bounds;
    }
    if param == ZSTD_c_srcSizeHint {
        bounds.lowerBound = ZSTD_SRCSIZEHINT_MIN;
        bounds.upperBound = ZSTD_SRCSIZEHINT_MAX;
        return bounds;
    }
    if param == ZSTD_c_stableInBuffer || param == ZSTD_c_stableOutBuffer {
        bounds.lowerBound = ZSTD_bm_buffered as c_int;
        bounds.upperBound = ZSTD_bm_stable as c_int;
        return bounds;
    }
    if param == ZSTD_c_blockDelimiters {
        bounds.lowerBound = ZSTD_sf_noBlockDelimiters as c_int;
        bounds.upperBound = ZSTD_sf_explicitBlockDelimiters as c_int;
        return bounds;
    }
    if param == ZSTD_c_validateSequences {
        bounds.lowerBound = 0;
        bounds.upperBound = 1;
        return bounds;
    }
    if param == ZSTD_c_splitAfterSequences {
        bounds.lowerBound = ZSTD_ps_auto as c_int;
        bounds.upperBound = ZSTD_ps_disable as c_int;
        return bounds;
    }
    if param == ZSTD_c_blockSplitterLevel {
        bounds.lowerBound = 0;
        bounds.upperBound = ZSTD_BLOCKSPLITTER_LEVEL_MAX;
        return bounds;
    }
    if param == ZSTD_c_useRowMatchFinder {
        bounds.lowerBound = ZSTD_ps_auto as c_int;
        bounds.upperBound = ZSTD_ps_disable as c_int;
        return bounds;
    }
    if param == ZSTD_c_deterministicRefPrefix {
        bounds.lowerBound = 0;
        bounds.upperBound = 1;
        return bounds;
    }
    if param == ZSTD_c_prefetchCDictTables {
        bounds.lowerBound = ZSTD_ps_auto as c_int;
        bounds.upperBound = ZSTD_ps_disable as c_int;
        return bounds;
    }
    if param == ZSTD_c_enableSeqProducerFallback {
        bounds.lowerBound = 0;
        bounds.upperBound = 1;
        return bounds;
    }
    if param == ZSTD_c_maxBlockSize {
        bounds.lowerBound = ZSTD_BLOCKSIZE_MAX_MIN;
        bounds.upperBound = ZSTD_BLOCKSIZE_MAX as c_int;
        return bounds;
    }
    if param == ZSTD_c_repcodeResolution {
        bounds.lowerBound = ZSTD_ps_auto as c_int;
        bounds.upperBound = ZSTD_ps_disable as c_int;
        return bounds;
    }

    bounds.error = ERROR(ZSTD_error_parameter_unsupported);
    bounds
}

/* Clamps the value into the bounded range. */
pub unsafe fn ZSTD_cParam_clampBounds(cParam: ZSTD_cParameter, value: *mut c_int) -> size_t {
    let bounds: ZSTD_bounds = ZSTD_cParam_getBounds(cParam);
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

pub unsafe fn ZSTD_isUpdateAuthorized(param: ZSTD_cParameter) -> c_int {
    if param == ZSTD_c_compressionLevel
        || param == ZSTD_c_hashLog
        || param == ZSTD_c_chainLog
        || param == ZSTD_c_searchLog
        || param == ZSTD_c_minMatch
        || param == ZSTD_c_targetLength
        || param == ZSTD_c_strategy
        || param == ZSTD_c_blockSplitterLevel
    {
        return 1;
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_CCtx_setParameter(
    cctx: *mut ZSTD_CCtx,
    param: ZSTD_cParameter,
    value: c_int,
) -> size_t {
    if (*cctx).streamStage != zcss_init {
        if ZSTD_isUpdateAuthorized(param) != 0 {
            (*cctx).cParamsChanged = 1;
        } else {
            return ERROR(ZSTD_error_stage_wrong);
        }
    }

    if param == ZSTD_c_nbWorkers {
        if (value != 0) && (*cctx).staticSize != 0 {
            return ERROR(ZSTD_error_parameter_unsupported);
        }
    } else if param == ZSTD_c_compressionLevel
        || param == ZSTD_c_windowLog
        || param == ZSTD_c_hashLog
        || param == ZSTD_c_chainLog
        || param == ZSTD_c_searchLog
        || param == ZSTD_c_minMatch
        || param == ZSTD_c_targetLength
        || param == ZSTD_c_strategy
        || param == ZSTD_c_ldmHashRateLog
        || param == ZSTD_c_format
        || param == ZSTD_c_contentSizeFlag
        || param == ZSTD_c_checksumFlag
        || param == ZSTD_c_dictIDFlag
        || param == ZSTD_c_forceMaxWindow
        || param == ZSTD_c_forceAttachDict
        || param == ZSTD_c_literalCompressionMode
        || param == ZSTD_c_jobSize
        || param == ZSTD_c_overlapLog
        || param == ZSTD_c_rsyncable
        || param == ZSTD_c_enableDedicatedDictSearch
        || param == ZSTD_c_enableLongDistanceMatching
        || param == ZSTD_c_ldmHashLog
        || param == ZSTD_c_ldmMinMatch
        || param == ZSTD_c_ldmBucketSizeLog
        || param == ZSTD_c_targetCBlockSize
        || param == ZSTD_c_srcSizeHint
        || param == ZSTD_c_stableInBuffer
        || param == ZSTD_c_stableOutBuffer
        || param == ZSTD_c_blockDelimiters
        || param == ZSTD_c_validateSequences
        || param == ZSTD_c_splitAfterSequences
        || param == ZSTD_c_blockSplitterLevel
        || param == ZSTD_c_useRowMatchFinder
        || param == ZSTD_c_deterministicRefPrefix
        || param == ZSTD_c_prefetchCDictTables
        || param == ZSTD_c_enableSeqProducerFallback
        || param == ZSTD_c_maxBlockSize
        || param == ZSTD_c_repcodeResolution
    {
        /* break */
    } else {
        return ERROR(ZSTD_error_parameter_unsupported);
    }
    ZSTD_CCtxParams_setParameter(&mut (*cctx).requestedParams, param, value)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_CCtxParams_setParameter(
    CCtxParams: *mut ZSTD_CCtx_params,
    param: ZSTD_cParameter,
    mut value: c_int,
) -> size_t {
    if param == ZSTD_c_format {
        if ZSTD_cParam_withinBounds(ZSTD_c_format, value) == 0 {
            return ERROR(ZSTD_error_parameter_outOfBound);
        }
        (*CCtxParams).format = value as ZSTD_format_e;
        return (*CCtxParams).format as size_t;
    }
    if param == ZSTD_c_compressionLevel {
        {
            let err = ZSTD_cParam_clampBounds(param, &mut value);
            if ERR_isError(err) != 0 {
                return err;
            }
        }
        if value == 0 {
            (*CCtxParams).compressionLevel = ZSTD_CLEVEL_DEFAULT; /* 0 == default */
        } else {
            (*CCtxParams).compressionLevel = value;
        }
        if (*CCtxParams).compressionLevel >= 0 {
            return (*CCtxParams).compressionLevel as size_t;
        }
        return 0; /* return type (size_t) cannot represent negative values */
    }
    if param == ZSTD_c_windowLog {
        if value != 0 {
            if ZSTD_cParam_withinBounds(ZSTD_c_windowLog, value) == 0 {
                return ERROR(ZSTD_error_parameter_outOfBound);
            }
        }
        (*CCtxParams).cParams.windowLog = value as U32;
        return (*CCtxParams).cParams.windowLog as size_t;
    }
    if param == ZSTD_c_hashLog {
        if value != 0 {
            if ZSTD_cParam_withinBounds(ZSTD_c_hashLog, value) == 0 {
                return ERROR(ZSTD_error_parameter_outOfBound);
            }
        }
        (*CCtxParams).cParams.hashLog = value as U32;
        return (*CCtxParams).cParams.hashLog as size_t;
    }
    if param == ZSTD_c_chainLog {
        if value != 0 {
            if ZSTD_cParam_withinBounds(ZSTD_c_chainLog, value) == 0 {
                return ERROR(ZSTD_error_parameter_outOfBound);
            }
        }
        (*CCtxParams).cParams.chainLog = value as U32;
        return (*CCtxParams).cParams.chainLog as size_t;
    }
    if param == ZSTD_c_searchLog {
        if value != 0 {
            if ZSTD_cParam_withinBounds(ZSTD_c_searchLog, value) == 0 {
                return ERROR(ZSTD_error_parameter_outOfBound);
            }
        }
        (*CCtxParams).cParams.searchLog = value as U32;
        return value as size_t;
    }
    if param == ZSTD_c_minMatch {
        if value != 0 {
            if ZSTD_cParam_withinBounds(ZSTD_c_minMatch, value) == 0 {
                return ERROR(ZSTD_error_parameter_outOfBound);
            }
        }
        (*CCtxParams).cParams.minMatch = value as U32;
        return (*CCtxParams).cParams.minMatch as size_t;
    }
    if param == ZSTD_c_targetLength {
        if ZSTD_cParam_withinBounds(ZSTD_c_targetLength, value) == 0 {
            return ERROR(ZSTD_error_parameter_outOfBound);
        }
        (*CCtxParams).cParams.targetLength = value as U32;
        return (*CCtxParams).cParams.targetLength as size_t;
    }
    if param == ZSTD_c_strategy {
        if value != 0 {
            if ZSTD_cParam_withinBounds(ZSTD_c_strategy, value) == 0 {
                return ERROR(ZSTD_error_parameter_outOfBound);
            }
        }
        (*CCtxParams).cParams.strategy = value as ZSTD_strategy;
        return (*CCtxParams).cParams.strategy as size_t;
    }
    if param == ZSTD_c_contentSizeFlag {
        (*CCtxParams).fParams.contentSizeFlag = (value != 0) as c_int;
        return (*CCtxParams).fParams.contentSizeFlag as size_t;
    }
    if param == ZSTD_c_checksumFlag {
        (*CCtxParams).fParams.checksumFlag = (value != 0) as c_int;
        return (*CCtxParams).fParams.checksumFlag as size_t;
    }
    if param == ZSTD_c_dictIDFlag {
        (*CCtxParams).fParams.noDictIDFlag = (value == 0) as c_int;
        return (((*CCtxParams).fParams.noDictIDFlag == 0) as c_int) as size_t;
    }
    if param == ZSTD_c_forceMaxWindow {
        (*CCtxParams).forceWindow = (value != 0) as c_int;
        return (*CCtxParams).forceWindow as size_t;
    }
    if param == ZSTD_c_forceAttachDict {
        let pref: ZSTD_dictAttachPref_e = value as ZSTD_dictAttachPref_e;
        if ZSTD_cParam_withinBounds(ZSTD_c_forceAttachDict, pref as c_int) == 0 {
            return ERROR(ZSTD_error_parameter_outOfBound);
        }
        (*CCtxParams).attachDictPref = pref;
        return (*CCtxParams).attachDictPref as size_t;
    }
    if param == ZSTD_c_literalCompressionMode {
        let lcm: ZSTD_ParamSwitch_e = value as ZSTD_ParamSwitch_e;
        if ZSTD_cParam_withinBounds(ZSTD_c_literalCompressionMode, lcm as c_int) == 0 {
            return ERROR(ZSTD_error_parameter_outOfBound);
        }
        (*CCtxParams).literalCompressionMode = lcm;
        return (*CCtxParams).literalCompressionMode as size_t;
    }
    if param == ZSTD_c_nbWorkers {
        /* not compiled with multithreading */
        if value != 0 {
            return ERROR(ZSTD_error_parameter_unsupported);
        }
        return 0;
    }
    if param == ZSTD_c_jobSize {
        if value != 0 {
            return ERROR(ZSTD_error_parameter_unsupported);
        }
        return 0;
    }
    if param == ZSTD_c_overlapLog {
        if value != 0 {
            return ERROR(ZSTD_error_parameter_unsupported);
        }
        return 0;
    }
    if param == ZSTD_c_rsyncable {
        if value != 0 {
            return ERROR(ZSTD_error_parameter_unsupported);
        }
        return 0;
    }
    if param == ZSTD_c_enableDedicatedDictSearch {
        (*CCtxParams).enableDedicatedDictSearch = (value != 0) as c_int;
        return (*CCtxParams).enableDedicatedDictSearch as size_t;
    }
    if param == ZSTD_c_enableLongDistanceMatching {
        if ZSTD_cParam_withinBounds(ZSTD_c_enableLongDistanceMatching, value) == 0 {
            return ERROR(ZSTD_error_parameter_outOfBound);
        }
        (*CCtxParams).ldmParams.enableLdm = value as ZSTD_ParamSwitch_e;
        return (*CCtxParams).ldmParams.enableLdm as size_t;
    }
    if param == ZSTD_c_ldmHashLog {
        if value != 0 {
            if ZSTD_cParam_withinBounds(ZSTD_c_ldmHashLog, value) == 0 {
                return ERROR(ZSTD_error_parameter_outOfBound);
            }
        }
        (*CCtxParams).ldmParams.hashLog = value as U32;
        return (*CCtxParams).ldmParams.hashLog as size_t;
    }
    if param == ZSTD_c_ldmMinMatch {
        if value != 0 {
            if ZSTD_cParam_withinBounds(ZSTD_c_ldmMinMatch, value) == 0 {
                return ERROR(ZSTD_error_parameter_outOfBound);
            }
        }
        (*CCtxParams).ldmParams.minMatchLength = value as U32;
        return (*CCtxParams).ldmParams.minMatchLength as size_t;
    }
    if param == ZSTD_c_ldmBucketSizeLog {
        if value != 0 {
            if ZSTD_cParam_withinBounds(ZSTD_c_ldmBucketSizeLog, value) == 0 {
                return ERROR(ZSTD_error_parameter_outOfBound);
            }
        }
        (*CCtxParams).ldmParams.bucketSizeLog = value as U32;
        return (*CCtxParams).ldmParams.bucketSizeLog as size_t;
    }
    if param == ZSTD_c_ldmHashRateLog {
        if value != 0 {
            if ZSTD_cParam_withinBounds(ZSTD_c_ldmHashRateLog, value) == 0 {
                return ERROR(ZSTD_error_parameter_outOfBound);
            }
        }
        (*CCtxParams).ldmParams.hashRateLog = value as U32;
        return (*CCtxParams).ldmParams.hashRateLog as size_t;
    }
    if param == ZSTD_c_targetCBlockSize {
        if value != 0 {
            value = MAX(value, ZSTD_TARGETCBLOCKSIZE_MIN);
            if ZSTD_cParam_withinBounds(ZSTD_c_targetCBlockSize, value) == 0 {
                return ERROR(ZSTD_error_parameter_outOfBound);
            }
        }
        (*CCtxParams).targetCBlockSize = value as U32 as size_t;
        return (*CCtxParams).targetCBlockSize;
    }
    if param == ZSTD_c_srcSizeHint {
        if value != 0 {
            if ZSTD_cParam_withinBounds(ZSTD_c_srcSizeHint, value) == 0 {
                return ERROR(ZSTD_error_parameter_outOfBound);
            }
        }
        (*CCtxParams).srcSizeHint = value;
        return (*CCtxParams).srcSizeHint as size_t;
    }
    if param == ZSTD_c_stableInBuffer {
        if ZSTD_cParam_withinBounds(ZSTD_c_stableInBuffer, value) == 0 {
            return ERROR(ZSTD_error_parameter_outOfBound);
        }
        (*CCtxParams).inBufferMode = value as ZSTD_bufferMode_e;
        return (*CCtxParams).inBufferMode as size_t;
    }
    if param == ZSTD_c_stableOutBuffer {
        if ZSTD_cParam_withinBounds(ZSTD_c_stableOutBuffer, value) == 0 {
            return ERROR(ZSTD_error_parameter_outOfBound);
        }
        (*CCtxParams).outBufferMode = value as ZSTD_bufferMode_e;
        return (*CCtxParams).outBufferMode as size_t;
    }
    if param == ZSTD_c_blockDelimiters {
        if ZSTD_cParam_withinBounds(ZSTD_c_blockDelimiters, value) == 0 {
            return ERROR(ZSTD_error_parameter_outOfBound);
        }
        (*CCtxParams).blockDelimiters = value as ZSTD_SequenceFormat_e;
        return (*CCtxParams).blockDelimiters as size_t;
    }
    if param == ZSTD_c_validateSequences {
        if ZSTD_cParam_withinBounds(ZSTD_c_validateSequences, value) == 0 {
            return ERROR(ZSTD_error_parameter_outOfBound);
        }
        (*CCtxParams).validateSequences = value;
        return (*CCtxParams).validateSequences as size_t;
    }
    if param == ZSTD_c_splitAfterSequences {
        if ZSTD_cParam_withinBounds(ZSTD_c_splitAfterSequences, value) == 0 {
            return ERROR(ZSTD_error_parameter_outOfBound);
        }
        (*CCtxParams).postBlockSplitter = value as ZSTD_ParamSwitch_e;
        return (*CCtxParams).postBlockSplitter as size_t;
    }
    if param == ZSTD_c_blockSplitterLevel {
        if ZSTD_cParam_withinBounds(ZSTD_c_blockSplitterLevel, value) == 0 {
            return ERROR(ZSTD_error_parameter_outOfBound);
        }
        (*CCtxParams).preBlockSplitter_level = value;
        return (*CCtxParams).preBlockSplitter_level as size_t;
    }
    if param == ZSTD_c_useRowMatchFinder {
        if ZSTD_cParam_withinBounds(ZSTD_c_useRowMatchFinder, value) == 0 {
            return ERROR(ZSTD_error_parameter_outOfBound);
        }
        (*CCtxParams).useRowMatchFinder = value as ZSTD_ParamSwitch_e;
        return (*CCtxParams).useRowMatchFinder as size_t;
    }
    if param == ZSTD_c_deterministicRefPrefix {
        if ZSTD_cParam_withinBounds(ZSTD_c_deterministicRefPrefix, value) == 0 {
            return ERROR(ZSTD_error_parameter_outOfBound);
        }
        (*CCtxParams).deterministicRefPrefix = (value != 0) as c_int;
        return (*CCtxParams).deterministicRefPrefix as size_t;
    }
    if param == ZSTD_c_prefetchCDictTables {
        if ZSTD_cParam_withinBounds(ZSTD_c_prefetchCDictTables, value) == 0 {
            return ERROR(ZSTD_error_parameter_outOfBound);
        }
        (*CCtxParams).prefetchCDictTables = value as ZSTD_ParamSwitch_e;
        return (*CCtxParams).prefetchCDictTables as size_t;
    }
    if param == ZSTD_c_enableSeqProducerFallback {
        if ZSTD_cParam_withinBounds(ZSTD_c_enableSeqProducerFallback, value) == 0 {
            return ERROR(ZSTD_error_parameter_outOfBound);
        }
        (*CCtxParams).enableMatchFinderFallback = value;
        return (*CCtxParams).enableMatchFinderFallback as size_t;
    }
    if param == ZSTD_c_maxBlockSize {
        if value != 0 {
            if ZSTD_cParam_withinBounds(ZSTD_c_maxBlockSize, value) == 0 {
                return ERROR(ZSTD_error_parameter_outOfBound);
            }
        }
        (*CCtxParams).maxBlockSize = value as size_t;
        return (*CCtxParams).maxBlockSize;
    }
    if param == ZSTD_c_repcodeResolution {
        if ZSTD_cParam_withinBounds(ZSTD_c_repcodeResolution, value) == 0 {
            return ERROR(ZSTD_error_parameter_outOfBound);
        }
        (*CCtxParams).searchForExternalRepcodes = value as ZSTD_ParamSwitch_e;
        return (*CCtxParams).searchForExternalRepcodes as size_t;
    }

    ERROR(ZSTD_error_parameter_unsupported)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_CCtx_getParameter(
    cctx: *const ZSTD_CCtx,
    param: ZSTD_cParameter,
    value: *mut c_int,
) -> size_t {
    ZSTD_CCtxParams_getParameter(&(*cctx).requestedParams, param, value)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_CCtxParams_getParameter(
    CCtxParams: *const ZSTD_CCtx_params,
    param: ZSTD_cParameter,
    value: *mut c_int,
) -> size_t {
    if param == ZSTD_c_format {
        *value = (*CCtxParams).format as c_int;
    } else if param == ZSTD_c_compressionLevel {
        *value = (*CCtxParams).compressionLevel;
    } else if param == ZSTD_c_windowLog {
        *value = (*CCtxParams).cParams.windowLog as c_int;
    } else if param == ZSTD_c_hashLog {
        *value = (*CCtxParams).cParams.hashLog as c_int;
    } else if param == ZSTD_c_chainLog {
        *value = (*CCtxParams).cParams.chainLog as c_int;
    } else if param == ZSTD_c_searchLog {
        *value = (*CCtxParams).cParams.searchLog as c_int;
    } else if param == ZSTD_c_minMatch {
        *value = (*CCtxParams).cParams.minMatch as c_int;
    } else if param == ZSTD_c_targetLength {
        *value = (*CCtxParams).cParams.targetLength as c_int;
    } else if param == ZSTD_c_strategy {
        *value = (*CCtxParams).cParams.strategy as c_int;
    } else if param == ZSTD_c_contentSizeFlag {
        *value = (*CCtxParams).fParams.contentSizeFlag;
    } else if param == ZSTD_c_checksumFlag {
        *value = (*CCtxParams).fParams.checksumFlag;
    } else if param == ZSTD_c_dictIDFlag {
        *value = ((*CCtxParams).fParams.noDictIDFlag == 0) as c_int;
    } else if param == ZSTD_c_forceMaxWindow {
        *value = (*CCtxParams).forceWindow;
    } else if param == ZSTD_c_forceAttachDict {
        *value = (*CCtxParams).attachDictPref as c_int;
    } else if param == ZSTD_c_literalCompressionMode {
        *value = (*CCtxParams).literalCompressionMode as c_int;
    } else if param == ZSTD_c_nbWorkers {
        *value = (*CCtxParams).nbWorkers;
    } else if param == ZSTD_c_jobSize {
        /* not compiled with multithreading */
        return ERROR(ZSTD_error_parameter_unsupported);
    } else if param == ZSTD_c_overlapLog {
        return ERROR(ZSTD_error_parameter_unsupported);
    } else if param == ZSTD_c_rsyncable {
        return ERROR(ZSTD_error_parameter_unsupported);
    } else if param == ZSTD_c_enableDedicatedDictSearch {
        *value = (*CCtxParams).enableDedicatedDictSearch;
    } else if param == ZSTD_c_enableLongDistanceMatching {
        *value = (*CCtxParams).ldmParams.enableLdm as c_int;
    } else if param == ZSTD_c_ldmHashLog {
        *value = (*CCtxParams).ldmParams.hashLog as c_int;
    } else if param == ZSTD_c_ldmMinMatch {
        *value = (*CCtxParams).ldmParams.minMatchLength as c_int;
    } else if param == ZSTD_c_ldmBucketSizeLog {
        *value = (*CCtxParams).ldmParams.bucketSizeLog as c_int;
    } else if param == ZSTD_c_ldmHashRateLog {
        *value = (*CCtxParams).ldmParams.hashRateLog as c_int;
    } else if param == ZSTD_c_targetCBlockSize {
        *value = (*CCtxParams).targetCBlockSize as c_int;
    } else if param == ZSTD_c_srcSizeHint {
        *value = (*CCtxParams).srcSizeHint as c_int;
    } else if param == ZSTD_c_stableInBuffer {
        *value = (*CCtxParams).inBufferMode as c_int;
    } else if param == ZSTD_c_stableOutBuffer {
        *value = (*CCtxParams).outBufferMode as c_int;
    } else if param == ZSTD_c_blockDelimiters {
        *value = (*CCtxParams).blockDelimiters as c_int;
    } else if param == ZSTD_c_validateSequences {
        *value = (*CCtxParams).validateSequences as c_int;
    } else if param == ZSTD_c_splitAfterSequences {
        *value = (*CCtxParams).postBlockSplitter as c_int;
    } else if param == ZSTD_c_blockSplitterLevel {
        *value = (*CCtxParams).preBlockSplitter_level;
    } else if param == ZSTD_c_useRowMatchFinder {
        *value = (*CCtxParams).useRowMatchFinder as c_int;
    } else if param == ZSTD_c_deterministicRefPrefix {
        *value = (*CCtxParams).deterministicRefPrefix as c_int;
    } else if param == ZSTD_c_prefetchCDictTables {
        *value = (*CCtxParams).prefetchCDictTables as c_int;
    } else if param == ZSTD_c_enableSeqProducerFallback {
        *value = (*CCtxParams).enableMatchFinderFallback;
    } else if param == ZSTD_c_maxBlockSize {
        *value = (*CCtxParams).maxBlockSize as c_int;
    } else if param == ZSTD_c_repcodeResolution {
        *value = (*CCtxParams).searchForExternalRepcodes as c_int;
    } else {
        return ERROR(ZSTD_error_parameter_unsupported);
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_CCtx_setParametersUsingCCtxParams(
    cctx: *mut ZSTD_CCtx,
    params: *const ZSTD_CCtx_params,
) -> size_t {
    if (*cctx).streamStage != zcss_init {
        return ERROR(ZSTD_error_stage_wrong);
    }
    if !(*cctx).cdict.is_null() {
        return ERROR(ZSTD_error_stage_wrong);
    }
    (*cctx).requestedParams = core::ptr::read(params);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_CCtx_setCParams(
    cctx: *mut ZSTD_CCtx,
    cparams: ZSTD_compressionParameters,
) -> size_t {
    /* only update if all parameters are valid */
    {
        let err = ZSTD_checkCParams(cparams);
        if ERR_isError(err) != 0 {
            return err;
        }
    }
    {
        let err = ZSTD_CCtx_setParameter(cctx, ZSTD_c_windowLog, cparams.windowLog as c_int);
        if ERR_isError(err) != 0 {
            return err;
        }
    }
    {
        let err = ZSTD_CCtx_setParameter(cctx, ZSTD_c_chainLog, cparams.chainLog as c_int);
        if ERR_isError(err) != 0 {
            return err;
        }
    }
    {
        let err = ZSTD_CCtx_setParameter(cctx, ZSTD_c_hashLog, cparams.hashLog as c_int);
        if ERR_isError(err) != 0 {
            return err;
        }
    }
    {
        let err = ZSTD_CCtx_setParameter(cctx, ZSTD_c_searchLog, cparams.searchLog as c_int);
        if ERR_isError(err) != 0 {
            return err;
        }
    }
    {
        let err = ZSTD_CCtx_setParameter(cctx, ZSTD_c_minMatch, cparams.minMatch as c_int);
        if ERR_isError(err) != 0 {
            return err;
        }
    }
    {
        let err = ZSTD_CCtx_setParameter(cctx, ZSTD_c_targetLength, cparams.targetLength as c_int);
        if ERR_isError(err) != 0 {
            return err;
        }
    }
    {
        let err = ZSTD_CCtx_setParameter(cctx, ZSTD_c_strategy, cparams.strategy as c_int);
        if ERR_isError(err) != 0 {
            return err;
        }
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_CCtx_setFParams(
    cctx: *mut ZSTD_CCtx,
    fparams: ZSTD_frameParameters,
) -> size_t {
    {
        let err = ZSTD_CCtx_setParameter(
            cctx,
            ZSTD_c_contentSizeFlag,
            (fparams.contentSizeFlag != 0) as c_int,
        );
        if ERR_isError(err) != 0 {
            return err;
        }
    }
    {
        let err = ZSTD_CCtx_setParameter(
            cctx,
            ZSTD_c_checksumFlag,
            (fparams.checksumFlag != 0) as c_int,
        );
        if ERR_isError(err) != 0 {
            return err;
        }
    }
    {
        let err = ZSTD_CCtx_setParameter(
            cctx,
            ZSTD_c_dictIDFlag,
            (fparams.noDictIDFlag == 0) as c_int,
        );
        if ERR_isError(err) != 0 {
            return err;
        }
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_CCtx_setParams(
    cctx: *mut ZSTD_CCtx,
    params: ZSTD_parameters,
) -> size_t {
    /* First check cParams, because we want to update all or none. */
    {
        let err = ZSTD_checkCParams(params.cParams);
        if ERR_isError(err) != 0 {
            return err;
        }
    }
    /* Next set fParams, because this could fail if the cctx isn't in init stage. */
    {
        let err = ZSTD_CCtx_setFParams(cctx, params.fParams);
        if ERR_isError(err) != 0 {
            return err;
        }
    }
    /* Finally set cParams, which should succeed. */
    {
        let err = ZSTD_CCtx_setCParams(cctx, params.cParams);
        if ERR_isError(err) != 0 {
            return err;
        }
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_CCtx_setPledgedSrcSize(
    cctx: *mut ZSTD_CCtx,
    pledgedSrcSize: core::ffi::c_ulonglong,
) -> size_t {
    if (*cctx).streamStage != zcss_init {
        return ERROR(ZSTD_error_stage_wrong);
    }
    (*cctx).pledgedSrcSizePlusOne = pledgedSrcSize + 1;
    0
}

/**
 * Initializes the local dictionary using requested parameters.
 */
pub unsafe fn ZSTD_initLocalDict(cctx: *mut ZSTD_CCtx) -> size_t {
    let dl: *mut ZSTD_localDict = &mut (*cctx).localDict;
    if (*dl).dict.is_null() {
        /* No local dictionary. */
        return 0;
    }
    if !(*dl).cdict.is_null() {
        /* Local dictionary already initialized. */
        return 0;
    }

    (*dl).cdict = ZSTD_createCDict_advanced2(
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
    dictSize: size_t,
    dictLoadMethod: ZSTD_dictLoadMethod_e,
    dictContentType: ZSTD_dictContentType_e,
) -> size_t {
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
        ZSTD_memcpy(dictBuffer as *mut u8, dict as *const u8, dictSize);
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
    dictSize: size_t,
) -> size_t {
    ZSTD_CCtx_loadDictionary_advanced(cctx, dict, dictSize, ZSTD_dlm_byRef, ZSTD_dct_auto)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_CCtx_loadDictionary(
    cctx: *mut ZSTD_CCtx,
    dict: *const c_void,
    dictSize: size_t,
) -> size_t {
    ZSTD_CCtx_loadDictionary_advanced(cctx, dict, dictSize, ZSTD_dlm_byCopy, ZSTD_dct_auto)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_CCtx_refCDict(
    cctx: *mut ZSTD_CCtx,
    cdict: *const ZSTD_CDict,
) -> size_t {
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
) -> size_t {
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
    prefixSize: size_t,
) -> size_t {
    ZSTD_CCtx_refPrefix_advanced(cctx, prefix, prefixSize, ZSTD_dct_rawContent)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_CCtx_refPrefix_advanced(
    cctx: *mut ZSTD_CCtx,
    prefix: *const c_void,
    prefixSize: size_t,
    dictContentType: ZSTD_dictContentType_e,
) -> size_t {
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

/* ZSTD_CCtx_reset() : Also dumps dictionary */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_CCtx_reset(cctx: *mut ZSTD_CCtx, reset: ZSTD_ResetDirective) -> size_t {
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

/** ZSTD_checkCParams() */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_checkCParams(cParams: ZSTD_compressionParameters) -> size_t {
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

/** ZSTD_clampCParams() : make CParam values within valid range. */
pub unsafe fn ZSTD_clampCParams(
    mut cParams: ZSTD_compressionParameters,
) -> ZSTD_compressionParameters {
    /* CLAMP(ZSTD_c_windowLog, cParams.windowLog) */
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
        /* CLAMP_TYPE(ZSTD_c_strategy, cParams.strategy, ZSTD_strategy) */
        let bounds = ZSTD_cParam_getBounds(ZSTD_c_strategy);
        if (cParams.strategy as c_int) < bounds.lowerBound {
            cParams.strategy = bounds.lowerBound as ZSTD_strategy;
        } else if (cParams.strategy as c_int) > bounds.upperBound {
            cParams.strategy = bounds.upperBound as ZSTD_strategy;
        }
    }
    cParams
}

/** ZSTD_cycleLog() : condition for correct operation : hashLog > 1 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_cycleLog(hashLog: U32, strat: ZSTD_strategy) -> U32 {
    let btScale: U32 = ((strat as U32) >= (ZSTD_btlazy2 as U32)) as U32;
    hashLog - btScale
}

/** ZSTD_dictAndWindowLog() */
pub unsafe fn ZSTD_dictAndWindowLog(windowLog: U32, srcSize: U64, dictSize: U64) -> U32 {
    let maxWindowSize: U64 = 1u64 << ZSTD_WINDOWLOG_MAX;
    /* No dictionary ==> No change */
    if dictSize == 0 {
        return windowLog;
    }
    {
        let windowSize: U64 = 1u64 << windowLog;
        let dictAndWindowSize: U64 = dictSize + windowSize;
        if windowSize >= dictSize + srcSize {
            return windowLog; /* Window size large enough already */
        } else if dictAndWindowSize >= maxWindowSize {
            return ZSTD_WINDOWLOG_MAX as U32; /* Larger than max window log */
        } else {
            return ZSTD_highbit32((dictAndWindowSize as U32).wrapping_sub(1)) + 1;
        }
    }
}

/** ZSTD_adjustCParams_internal() */
pub unsafe fn ZSTD_adjustCParams_internal(
    mut cPar: ZSTD_compressionParameters,
    mut srcSize: core::ffi::c_ulonglong,
    mut dictSize: size_t,
    mode: ZSTD_CParamMode_e,
    mut useRowMatchFinder: ZSTD_ParamSwitch_e,
) -> ZSTD_compressionParameters {
    let minSrcSize: U64 = 513; /* (1<<9) + 1 */
    let maxWindowResize: U64 = 1u64 << (ZSTD_WINDOWLOG_MAX - 1);
    /* No ZSTD_EXCLUDE_* block compressors excluded in this build. */

    if mode == ZSTD_cpm_unknown || mode == ZSTD_cpm_noAttachDict {
        /* break */
    } else if mode == ZSTD_cpm_createCDict {
        if dictSize != 0 && srcSize == ZSTD_CONTENTSIZE_UNKNOWN {
            srcSize = minSrcSize;
        }
    } else if mode == ZSTD_cpm_attachDict {
        dictSize = 0;
    }

    /* resize windowLog if input is small enough, to use less memory */
    if (srcSize as U64 <= maxWindowResize) && (dictSize as U64 <= maxWindowResize) {
        let tSize: U32 = (srcSize as size_t + dictSize) as U32;
        let hashSizeMin: U32 = 1 << ZSTD_HASHLOG_MIN;
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
        let dictAndWindowLog: U32 =
            ZSTD_dictAndWindowLog(cPar.windowLog, srcSize as U64, dictSize as U64);
        let cycleLog: U32 = ZSTD_cycleLog(cPar.chainLog, cPar.strategy);
        if cPar.hashLog > dictAndWindowLog + 1 {
            cPar.hashLog = dictAndWindowLog + 1;
        }
        if cycleLog > dictAndWindowLog {
            cPar.chainLog -= cycleLog - dictAndWindowLog;
        }
    }

    if cPar.windowLog < ZSTD_WINDOWLOG_ABSOLUTEMIN {
        cPar.windowLog = ZSTD_WINDOWLOG_ABSOLUTEMIN; /* minimum wlog required for valid frame header */
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
        /* Switch to 32-entry rows if searchLog is 5 (or more) */
        let rowLog: U32 = BOUNDED(4u32, cPar.searchLog, 6u32);
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
    mut srcSize: core::ffi::c_ulonglong,
    dictSize: size_t,
) -> ZSTD_compressionParameters {
    cPar = ZSTD_clampCParams(cPar); /* resulting cPar is necessarily valid */
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
    dictSize: size_t,
    mode: ZSTD_CParamMode_e,
) -> ZSTD_compressionParameters {
    let mut cParams: ZSTD_compressionParameters;
    if srcSizeHint == ZSTD_CONTENTSIZE_UNKNOWN && (*CCtxParams).srcSizeHint > 0 {
        srcSizeHint = (*CCtxParams).srcSizeHint as U64;
    }
    cParams = ZSTD_getCParams_internal(
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
) -> size_t {
    /* chain table size should be 0 for fast or row-hash strategies */
    let chainSize: size_t = if ZSTD_allocateChainTable(
        (*cParams).strategy,
        useRowMatchFinder,
        (enableDedicatedDictSearch != 0 && forCCtx == 0) as U32,
    ) != 0
    {
        (1 as size_t) << (*cParams).chainLog
    } else {
        0
    };
    let hSize: size_t = (1 as size_t) << (*cParams).hashLog;
    let hashLog3: U32 = if forCCtx != 0 && (*cParams).minMatch == 3 {
        MIN(ZSTD_HASHLOG3_MAX, (*cParams).windowLog)
    } else {
        0
    };
    let h3Size: size_t = if hashLog3 != 0 {
        (1 as size_t) << hashLog3
    } else {
        0
    };
    let tableSpace: size_t = chainSize * core::mem::size_of::<U32>() as size_t
        + hSize * core::mem::size_of::<U32>() as size_t
        + h3Size * core::mem::size_of::<U32>() as size_t;
    let optPotentialSpace: size_t =
        ZSTD_cwksp_aligned64_alloc_size((MaxML as size_t + 1) * core::mem::size_of::<U32>() as size_t)
            + ZSTD_cwksp_aligned64_alloc_size(
                (MaxLL as size_t + 1) * core::mem::size_of::<U32>() as size_t,
            )
            + ZSTD_cwksp_aligned64_alloc_size(
                (MaxOff as size_t + 1) * core::mem::size_of::<U32>() as size_t,
            )
            + ZSTD_cwksp_aligned64_alloc_size(
                ((1 << Litbits) as size_t) * core::mem::size_of::<U32>() as size_t,
            )
            + ZSTD_cwksp_aligned64_alloc_size(
                ZSTD_OPT_SIZE * core::mem::size_of::<ZSTD_match_t>() as size_t,
            )
            + ZSTD_cwksp_aligned64_alloc_size(
                ZSTD_OPT_SIZE * core::mem::size_of::<ZSTD_optimal_t>() as size_t,
            );
    let lazyAdditionalSpace: size_t =
        if ZSTD_rowMatchFinderUsed((*cParams).strategy, useRowMatchFinder) != 0 {
            ZSTD_cwksp_aligned64_alloc_size(hSize)
        } else {
            0
        };
    let optSpace: size_t = if forCCtx != 0 && ((*cParams).strategy >= ZSTD_btopt) {
        optPotentialSpace
    } else {
        0
    };
    let slackSpace: size_t = ZSTD_cwksp_slack_space_required();

    tableSpace + optSpace + slackSpace + lazyAdditionalSpace
}

/* Gives a tighter bound than ZSTD_sequenceBound() by taking minMatch into account. */
pub unsafe fn ZSTD_maxNbSeq(
    blockSize: size_t,
    minMatch: c_uint,
    useSequenceProducer: c_int,
) -> size_t {
    let divider: U32 = if minMatch == 3 || useSequenceProducer != 0 {
        3
    } else {
        4
    };
    blockSize / divider as size_t
}

pub unsafe fn ZSTD_estimateCCtxSize_usingCCtxParams_internal(
    cParams: *const ZSTD_compressionParameters,
    ldmParams: *const ldmParams_t,
    isStatic: c_int,
    useRowMatchFinder: ZSTD_ParamSwitch_e,
    buffInSize: size_t,
    buffOutSize: size_t,
    pledgedSrcSize: U64,
    useSequenceProducer: c_int,
    maxBlockSize: size_t,
) -> size_t {
    let windowSize: size_t = BOUNDED(1u64, 1u64 << (*cParams).windowLog, pledgedSrcSize) as size_t;
    let blockSize: size_t = MIN(ZSTD_resolveMaxBlockSize(maxBlockSize), windowSize);
    let maxNbSeq: size_t = ZSTD_maxNbSeq(blockSize, (*cParams).minMatch, useSequenceProducer);
    let tokenSpace: size_t = ZSTD_cwksp_alloc_size(WILDCOPY_OVERLENGTH as size_t + blockSize)
        + ZSTD_cwksp_aligned64_alloc_size(maxNbSeq * core::mem::size_of::<SeqDef>() as size_t)
        + 3 * ZSTD_cwksp_alloc_size(maxNbSeq * core::mem::size_of::<BYTE>() as size_t);
    let tmpWorkSpace: size_t = ZSTD_cwksp_alloc_size(TMP_WORKSPACE_SIZE);
    let blockStateSpace: size_t =
        2 * ZSTD_cwksp_alloc_size(core::mem::size_of::<ZSTD_compressedBlockState_t>() as size_t);
    let matchStateSize: size_t = ZSTD_sizeof_matchState(cParams, useRowMatchFinder, 0, 1);

    let ldmSpace: size_t = ZSTD_ldm_getTableSize(*ldmParams);
    let maxNbLdmSeq: size_t = ZSTD_ldm_getMaxNbSeq(*ldmParams, blockSize);
    let ldmSeqSpace: size_t = if (*ldmParams).enableLdm == ZSTD_ps_enable {
        ZSTD_cwksp_aligned64_alloc_size(maxNbLdmSeq * core::mem::size_of::<rawSeq>() as size_t)
    } else {
        0
    };

    let bufferSpace: size_t =
        ZSTD_cwksp_alloc_size(buffInSize) + ZSTD_cwksp_alloc_size(buffOutSize);

    let cctxSpace: size_t = if isStatic != 0 {
        ZSTD_cwksp_alloc_size(core::mem::size_of::<ZSTD_CCtx>() as size_t)
    } else {
        0
    };

    let maxNbExternalSeq: size_t = ZSTD_sequenceBound(blockSize);
    let externalSeqSpace: size_t = if useSequenceProducer != 0 {
        ZSTD_cwksp_aligned64_alloc_size(
            maxNbExternalSeq * core::mem::size_of::<ZSTD_Sequence>() as size_t,
        )
    } else {
        0
    };

    let neededSpace: size_t = cctxSpace
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
) -> size_t {
    let cParams: ZSTD_compressionParameters =
        ZSTD_getCParamsFromCCtxParams(params, ZSTD_CONTENTSIZE_UNKNOWN, 0, ZSTD_cpm_noAttachDict);
    let useRowMatchFinder: ZSTD_ParamSwitch_e =
        ZSTD_resolveRowMatchFinderMode((*params).useRowMatchFinder, &cParams);

    if (*params).nbWorkers > 0 {
        return ERROR(ZSTD_error_GENERIC);
    }
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
) -> size_t {
    let mut initialParams: ZSTD_CCtx_params = ZSTD_makeCCtxParamsFromCParams(cParams);
    if ZSTD_rowMatchFinderSupported(cParams.strategy) != 0 {
        let noRowCCtxSize: size_t;
        let rowCCtxSize: size_t;
        initialParams.useRowMatchFinder = ZSTD_ps_disable;
        noRowCCtxSize = ZSTD_estimateCCtxSize_usingCCtxParams(&initialParams);
        initialParams.useRowMatchFinder = ZSTD_ps_enable;
        rowCCtxSize = ZSTD_estimateCCtxSize_usingCCtxParams(&initialParams);
        MAX(noRowCCtxSize, rowCCtxSize)
    } else {
        ZSTD_estimateCCtxSize_usingCCtxParams(&initialParams)
    }
}

pub unsafe fn ZSTD_estimateCCtxSize_internal(compressionLevel: c_int) -> size_t {
    let mut tier: c_int = 0;
    let mut largestSize: size_t = 0;
    let srcSizeTiers: [core::ffi::c_ulonglong; 4] = [
        16 * (1 << 10),
        128 * (1 << 10),
        256 * (1 << 10),
        ZSTD_CONTENTSIZE_UNKNOWN,
    ];
    while tier < 4 {
        let cParams: ZSTD_compressionParameters = ZSTD_getCParams_internal(
            compressionLevel,
            srcSizeTiers[tier as usize],
            0,
            ZSTD_cpm_noAttachDict,
        );
        largestSize = MAX(ZSTD_estimateCCtxSize_usingCParams(cParams), largestSize);
        tier += 1;
    }
    largestSize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_estimateCCtxSize(compressionLevel: c_int) -> size_t {
    let mut level: c_int;
    let mut memBudget: size_t = 0;
    level = MIN(compressionLevel, 1);
    while level <= compressionLevel {
        let newMB: size_t = ZSTD_estimateCCtxSize_internal(level);
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
) -> size_t {
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
        let blockSize: size_t = MIN(
            ZSTD_resolveMaxBlockSize((*params).maxBlockSize),
            (1 as size_t) << cParams.windowLog,
        );
        let inBuffSize: size_t = if (*params).inBufferMode == ZSTD_bm_buffered {
            ((1 as size_t) << cParams.windowLog) + blockSize
        } else {
            0
        };
        let outBuffSize: size_t = if (*params).outBufferMode == ZSTD_bm_buffered {
            ZSTD_compressBound(blockSize) + 1
        } else {
            0
        };
        let useRowMatchFinder: ZSTD_ParamSwitch_e =
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
) -> size_t {
    let mut initialParams: ZSTD_CCtx_params = ZSTD_makeCCtxParamsFromCParams(cParams);
    if ZSTD_rowMatchFinderSupported(cParams.strategy) != 0 {
        let noRowCCtxSize: size_t;
        let rowCCtxSize: size_t;
        initialParams.useRowMatchFinder = ZSTD_ps_disable;
        noRowCCtxSize = ZSTD_estimateCStreamSize_usingCCtxParams(&initialParams);
        initialParams.useRowMatchFinder = ZSTD_ps_enable;
        rowCCtxSize = ZSTD_estimateCStreamSize_usingCCtxParams(&initialParams);
        MAX(noRowCCtxSize, rowCCtxSize)
    } else {
        ZSTD_estimateCStreamSize_usingCCtxParams(&initialParams)
    }
}

pub unsafe fn ZSTD_estimateCStreamSize_internal(compressionLevel: c_int) -> size_t {
    let cParams: ZSTD_compressionParameters = ZSTD_getCParams_internal(
        compressionLevel,
        ZSTD_CONTENTSIZE_UNKNOWN,
        0,
        ZSTD_cpm_noAttachDict,
    );
    ZSTD_estimateCStreamSize_usingCParams(cParams)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_estimateCStreamSize(compressionLevel: c_int) -> size_t {
    let mut level: c_int;
    let mut memBudget: size_t = 0;
    level = MIN(compressionLevel, 1);
    while level <= compressionLevel {
        let newMB: size_t = ZSTD_estimateCStreamSize_internal(level);
        if newMB > memBudget {
            memBudget = newMB;
        }
        level += 1;
    }
    memBudget
}

/* ZSTD_getFrameProgression() */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_getFrameProgression(cctx: *const ZSTD_CCtx) -> ZSTD_frameProgression {
    {
        let mut fp: ZSTD_frameProgression = core::mem::zeroed();
        let buffered: size_t = if (*cctx).inBuff.is_null() {
            0
        } else {
            (*cctx).inBuffPos - (*cctx).inToCompress
        };
        fp.ingested = (*cctx).consumedSrcSize + buffered as u64;
        fp.consumed = (*cctx).consumedSrcSize;
        fp.produced = (*cctx).producedCSize;
        fp.flushed = (*cctx).producedCSize;
        fp.currentJobID = 0;
        fp.nbActiveWorkers = 0;
        fp
    }
}

/* ZSTD_toFlushNow() */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_toFlushNow(_cctx: *mut ZSTD_CCtx) -> size_t {
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
    while i < ZSTD_REP_NUM as c_int {
        (*bs).rep[i as usize] = repStartValue[i as usize];
        i += 1;
    }
    (*bs).entropy.huf.repeatMode = HUF_repeat_none;
    (*bs).entropy.fse.offcode_repeatMode = FSE_repeat_none;
    (*bs).entropy.fse.matchlength_repeatMode = FSE_repeat_none;
    (*bs).entropy.fse.litlength_repeatMode = FSE_repeat_none;
}

/* ZSTD_compResetPolicy_e */
pub type ZSTD_compResetPolicy_e = c_uint;
pub const ZSTDcrp_makeClean: ZSTD_compResetPolicy_e = 0;
pub const ZSTDcrp_leaveDirty: ZSTD_compResetPolicy_e = 1;

/* ZSTD_indexResetPolicy_e */
pub type ZSTD_indexResetPolicy_e = c_uint;
pub const ZSTDirp_continue: ZSTD_indexResetPolicy_e = 0;
pub const ZSTDirp_reset: ZSTD_indexResetPolicy_e = 1;

/* ZSTD_resetTarget_e */
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

/* ZSTD_invalidateMatchState() */
pub unsafe fn ZSTD_invalidateMatchState(ms: *mut ZSTD_MatchState_t) {
    ZSTD_window_clear(&mut (*ms).window);

    (*ms).nextToUpdate = (*ms).window.dictLimit;
    (*ms).loadedDictEnd = 0;
    (*ms).opt.litLengthSum = 0; /* force reset of btopt stats */
    (*ms).dictMatchState = null();
}

pub unsafe fn ZSTD_reset_matchState(
    ms: *mut ZSTD_MatchState_t,
    ws: *mut ZSTD_cwksp,
    cParams: *const ZSTD_compressionParameters,
    useRowMatchFinder: ZSTD_ParamSwitch_e,
    crp: ZSTD_compResetPolicy_e,
    forceResetIndex: ZSTD_indexResetPolicy_e,
    forWho: ZSTD_resetTarget_e,
) -> size_t {
    /* disable chain table allocation for fast or row-based strategies */
    let chainSize: size_t = if ZSTD_allocateChainTable(
        (*cParams).strategy,
        useRowMatchFinder,
        ((*ms).dedicatedDictSearch != 0 && (forWho == ZSTD_resetTarget_CDict)) as U32,
    ) != 0
    {
        (1 as size_t) << (*cParams).chainLog
    } else {
        0
    };
    let hSize: size_t = (1 as size_t) << (*cParams).hashLog;
    let hashLog3: U32 = if (forWho == ZSTD_resetTarget_CCtx) && (*cParams).minMatch == 3 {
        MIN(ZSTD_HASHLOG3_MAX, (*cParams).windowLog)
    } else {
        0
    };
    let h3Size: size_t = if hashLog3 != 0 {
        (1 as size_t) << hashLog3
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
        ZSTD_cwksp_reserve_table(ws, hSize * core::mem::size_of::<U32>() as size_t) as *mut U32;
    (*ms).chainTable =
        ZSTD_cwksp_reserve_table(ws, chainSize * core::mem::size_of::<U32>() as size_t) as *mut U32;
    (*ms).hashTable3 =
        ZSTD_cwksp_reserve_table(ws, h3Size * core::mem::size_of::<U32>() as size_t) as *mut U32;
    if ZSTD_cwksp_reserve_failed(ws) != 0 {
        return ERROR(ZSTD_error_memory_allocation);
    }

    if crp != ZSTDcrp_leaveDirty {
        /* reset tables only */
        ZSTD_cwksp_clean_tables(ws);
    }

    if ZSTD_rowMatchFinderUsed((*cParams).strategy, useRowMatchFinder) != 0 {
        /* Row match finder needs an additional table of hashes ("tags") */
        let tagTableSize: size_t = hSize;
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
            let rowLog: U32 = BOUNDED(4u32, (*cParams).searchLog, 6u32);
            (*ms).rowHashLog = (*cParams).hashLog - rowLog;
        }
    }

    /* opt parser space */
    if (forWho == ZSTD_resetTarget_CCtx) && ((*cParams).strategy >= ZSTD_btopt) {
        (*ms).opt.litFreq = ZSTD_cwksp_reserve_aligned64(
            ws,
            ((1 << Litbits) as size_t) * core::mem::size_of::<c_uint>() as size_t,
        ) as *mut c_uint;
        (*ms).opt.litLengthFreq = ZSTD_cwksp_reserve_aligned64(
            ws,
            (MaxLL as size_t + 1) * core::mem::size_of::<c_uint>() as size_t,
        ) as *mut c_uint;
        (*ms).opt.matchLengthFreq = ZSTD_cwksp_reserve_aligned64(
            ws,
            (MaxML as size_t + 1) * core::mem::size_of::<c_uint>() as size_t,
        ) as *mut c_uint;
        (*ms).opt.offCodeFreq = ZSTD_cwksp_reserve_aligned64(
            ws,
            (MaxOff as size_t + 1) * core::mem::size_of::<c_uint>() as size_t,
        ) as *mut c_uint;
        (*ms).opt.matchTable = ZSTD_cwksp_reserve_aligned64(
            ws,
            ZSTD_OPT_SIZE * core::mem::size_of::<ZSTD_match_t>() as size_t,
        ) as *mut ZSTD_match_t;
        (*ms).opt.priceTable = ZSTD_cwksp_reserve_aligned64(
            ws,
            ZSTD_OPT_SIZE * core::mem::size_of::<ZSTD_optimal_t>() as size_t,
        ) as *mut ZSTD_optimal_t;
    }

    (*ms).cParams = *cParams;

    if ZSTD_cwksp_reserve_failed(ws) != 0 {
        return ERROR(ZSTD_error_memory_allocation);
    }
    0
}

pub unsafe fn ZSTD_indexTooCloseToMax(w: ZSTD_window_t) -> c_int {
    ((w.nextSrc.offset_from(w.base) as size_t)
        > (ZSTD_CURRENT_MAX as size_t - ZSTD_INDEXOVERFLOW_MARGIN)) as c_int
}

/** ZSTD_dictTooBig() */
pub unsafe fn ZSTD_dictTooBig(loadedDictSize: size_t) -> c_int {
    (loadedDictSize > ZSTD_CHUNKSIZE_MAX as size_t) as c_int
}

/* ZSTD_resetCCtx_internal() */
pub unsafe fn ZSTD_resetCCtx_internal(
    zc: *mut ZSTD_CCtx,
    mut params: *const ZSTD_CCtx_params,
    pledgedSrcSize: U64,
    loadedDictSize: size_t,
    crp: ZSTD_compResetPolicy_e,
    zbuff: ZSTD_buffered_policy_e,
) -> size_t {
    let ws: *mut ZSTD_cwksp = &mut (*zc).workspace;

    (*zc).isFirstBlock = 1;

    /* Set applied params early so we can modify them for LDM,
     * and point params at the applied params. */
    (*zc).appliedParams = core::ptr::read(params);
    params = &(*zc).appliedParams;

    if (*params).ldmParams.enableLdm == ZSTD_ps_enable {
        /* Adjust long distance matching parameters */
        ZSTD_ldm_adjustParameters(&mut (*zc).appliedParams.ldmParams, &(*params).cParams);
    }

    {
        let windowSize: size_t = MAX(
            1,
            MIN((1u64 << (*params).cParams.windowLog), pledgedSrcSize) as size_t,
        );
        let blockSize: size_t = MIN((*params).maxBlockSize, windowSize);
        let maxNbSeq: size_t =
            ZSTD_maxNbSeq(blockSize, (*params).cParams.minMatch, ZSTD_hasExtSeqProd(params));
        let buffOutSize: size_t =
            if zbuff == ZSTDb_buffered && (*params).outBufferMode == ZSTD_bm_buffered {
                ZSTD_compressBound(blockSize) + 1
            } else {
                0
            };
        let buffInSize: size_t =
            if zbuff == ZSTDb_buffered && (*params).inBufferMode == ZSTD_bm_buffered {
                windowSize + blockSize
            } else {
                0
            };
        let maxNbLdmSeq: size_t = ZSTD_ldm_getMaxNbSeq((*params).ldmParams, blockSize);

        let indexTooClose: c_int = ZSTD_indexTooCloseToMax((*zc).blockState.matchState.window);
        let dictTooBig: c_int = ZSTD_dictTooBig(loadedDictSize);
        let mut needsIndexReset: ZSTD_indexResetPolicy_e =
            if indexTooClose != 0 || dictTooBig != 0 || (*zc).initialized == 0 {
                ZSTDirp_reset
            } else {
                ZSTDirp_continue
            };

        let neededSpace: size_t = ZSTD_estimateCCtxSize_usingCCtxParams_internal(
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

        if ERR_isError(neededSpace) != 0 {
            return neededSpace;
        }

        if (*zc).staticSize == 0 {
            ZSTD_cwksp_bump_oversized_duration(ws, 0);
        }

        {
            /* Check if workspace is large enough, alloc a new one if needed */
            let workspaceTooSmall: c_int = (ZSTD_cwksp_sizeof(ws) < neededSpace) as c_int;
            let workspaceWasteful: c_int = ZSTD_cwksp_check_wasteful(ws, neededSpace);
            let resizeWorkspace: c_int = (workspaceTooSmall != 0 || workspaceWasteful != 0) as c_int;

            if resizeWorkspace != 0 {
                if (*zc).staticSize != 0 {
                    return ERROR(ZSTD_error_memory_allocation);
                }

                needsIndexReset = ZSTDirp_reset;

                ZSTD_cwksp_free(ws, (*zc).customMem);
                {
                    let err = ZSTD_cwksp_create(ws, neededSpace, (*zc).customMem);
                    if ERR_isError(err) != 0 {
                        return err;
                    }
                }

                /* Statically sized space. tmpWorkspace never moves,
                 * though prev/next block swap places */
                (*zc).blockState.prevCBlock = ZSTD_cwksp_reserve_object(
                    ws,
                    core::mem::size_of::<ZSTD_compressedBlockState_t>() as size_t,
                ) as *mut ZSTD_compressedBlockState_t;
                if (*zc).blockState.prevCBlock.is_null() {
                    return ERROR(ZSTD_error_memory_allocation);
                }
                (*zc).blockState.nextCBlock = ZSTD_cwksp_reserve_object(
                    ws,
                    core::mem::size_of::<ZSTD_compressedBlockState_t>() as size_t,
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

        {
            let err = ZSTD_reset_matchState(
                &mut (*zc).blockState.matchState,
                ws,
                &(*params).cParams,
                (*params).useRowMatchFinder,
                crp,
                needsIndexReset,
                ZSTD_resetTarget_CCtx,
            );
            if ERR_isError(err) != 0 {
                return err;
            }
        }

        (*zc).seqStore.sequencesStart = ZSTD_cwksp_reserve_aligned64(
            ws,
            maxNbSeq * core::mem::size_of::<SeqDef>() as size_t,
        ) as *mut SeqDef;

        /* ldm hash table */
        if (*params).ldmParams.enableLdm == ZSTD_ps_enable {
            let ldmHSize: size_t = (1 as size_t) << (*params).ldmParams.hashLog;
            (*zc).ldmState.hashTable = ZSTD_cwksp_reserve_aligned64(
                ws,
                ldmHSize * core::mem::size_of::<ldmEntry_t>() as size_t,
            ) as *mut ldmEntry_t;
            ZSTD_memset(
                (*zc).ldmState.hashTable as *mut u8,
                0,
                ldmHSize * core::mem::size_of::<ldmEntry_t>() as size_t,
            );
            (*zc).ldmSequences = ZSTD_cwksp_reserve_aligned64(
                ws,
                maxNbLdmSeq * core::mem::size_of::<rawSeq>() as size_t,
            ) as *mut rawSeq;
            (*zc).maxNbLdmSequences = maxNbLdmSeq;

            ZSTD_window_init(&mut (*zc).ldmState.window);
            (*zc).ldmState.loadedDictEnd = 0;
        }

        /* reserve space for block-level external sequences */
        if ZSTD_hasExtSeqProd(params) != 0 {
            let maxNbExternalSeq: size_t = ZSTD_sequenceBound(blockSize);
            (*zc).extSeqBufCapacity = maxNbExternalSeq;
            (*zc).extSeqBuf = ZSTD_cwksp_reserve_aligned64(
                ws,
                maxNbExternalSeq * core::mem::size_of::<ZSTD_Sequence>() as size_t,
            ) as *mut ZSTD_Sequence;
        }

        /* buffers */
        (*zc).seqStore.litStart =
            ZSTD_cwksp_reserve_buffer(ws, blockSize + WILDCOPY_OVERLENGTH as size_t);
        (*zc).seqStore.maxNbLit = blockSize;

        (*zc).bufferedPolicy = zbuff;
        (*zc).inBuffSize = buffInSize;
        (*zc).inBuff = ZSTD_cwksp_reserve_buffer(ws, buffInSize) as *mut c_char;
        (*zc).outBuffSize = buffOutSize;
        (*zc).outBuff = ZSTD_cwksp_reserve_buffer(ws, buffOutSize) as *mut c_char;

        /* ldm bucketOffsets table */
        if (*params).ldmParams.enableLdm == ZSTD_ps_enable {
            let numBuckets: size_t = (1 as size_t)
                << ((*params).ldmParams.hashLog - (*params).ldmParams.bucketSizeLog);
            (*zc).ldmState.bucketOffsets = ZSTD_cwksp_reserve_buffer(ws, numBuckets);
            ZSTD_memset((*zc).ldmState.bucketOffsets, 0, numBuckets);
        }

        /* sequences storage */
        ZSTD_referenceExternalSequences(zc, null_mut(), 0);
        (*zc).seqStore.maxNbSeq = maxNbSeq;
        (*zc).seqStore.llCode =
            ZSTD_cwksp_reserve_buffer(ws, maxNbSeq * core::mem::size_of::<BYTE>() as size_t);
        (*zc).seqStore.mlCode =
            ZSTD_cwksp_reserve_buffer(ws, maxNbSeq * core::mem::size_of::<BYTE>() as size_t);
        (*zc).seqStore.ofCode =
            ZSTD_cwksp_reserve_buffer(ws, maxNbSeq * core::mem::size_of::<BYTE>() as size_t);

        (*zc).initialized = 1;

        return 0;
    }
}

/* ZSTD_invalidateRepCodes() */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_invalidateRepCodes(cctx: *mut ZSTD_CCtx) {
    let mut i: c_int = 0;
    while i < ZSTD_REP_NUM as c_int {
        (*(*cctx).blockState.prevCBlock).rep[i as usize] = 0;
        i += 1;
    }
}

/* struct ZSTD_CDict_s : defined in zstd_compress.c (this range). The crate's
 * zstd_compress_internal.rs declares ZSTD_CDict_s as an opaque type used only
 * through pointers, so here we mirror the real C layout under a private name
 * and cast the opaque pointer to access fields. */
#[repr(C)]
pub struct ZSTD_CDict_s_layout {
    pub dictContent: *const c_void,
    pub dictContentSize: size_t,
    pub dictContentType: ZSTD_dictContentType_e,
    pub entropyWorkspace: *mut U32,
    pub workspace: ZSTD_cwksp,
    pub matchState: ZSTD_MatchState_t,
    pub cBlockState: ZSTD_compressedBlockState_t,
    pub customMem: ZSTD_customMem,
    pub dictID: U32,
    pub compressionLevel: c_int,
    pub useRowMatchFinder: ZSTD_ParamSwitch_e,
}

#[inline]
unsafe fn cdict_layout(cdict: *const ZSTD_CDict) -> *const ZSTD_CDict_s_layout {
    cdict as *const ZSTD_CDict_s_layout
}

/* These are the approximate sizes for each strategy past which copying the
 * dictionary tables into the working context is faster than using them in-place. */
static attachDictSizeCutoffs: [size_t; ZSTD_STRATEGY_MAX as usize + 1] = [
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
    let cd = cdict_layout(cdict);
    let cutoff: size_t = attachDictSizeCutoffs[(*cd).matchState.cParams.strategy as usize];
    let dedicatedDictSearch: c_int = (*cd).matchState.dedicatedDictSearch;
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
) -> size_t {
    let cd = cdict_layout(cdict);
    {
        let mut adjusted_cdict_cParams: ZSTD_compressionParameters = (*cd).matchState.cParams;
        let windowLog: c_uint = params.cParams.windowLog;

        if (*cd).matchState.dedicatedDictSearch != 0 {
            ZSTD_dedicatedDictSearch_revertCParams(&mut adjusted_cdict_cParams);
        }

        params.cParams = ZSTD_adjustCParams_internal(
            adjusted_cdict_cParams,
            pledgedSrcSize,
            (*cd).dictContentSize,
            ZSTD_cpm_attachDict,
            params.useRowMatchFinder,
        );
        params.cParams.windowLog = windowLog;
        params.useRowMatchFinder = (*cd).useRowMatchFinder; /* cdict overrides */
        {
            let err = ZSTD_resetCCtx_internal(
                cctx,
                &params,
                pledgedSrcSize,
                0,
                ZSTDcrp_makeClean,
                zbuff,
            );
            if ERR_isError(err) != 0 {
                return err;
            }
        }
    }

    {
        let cdictEnd: U32 =
            (*cd).matchState.window.nextSrc.offset_from((*cd).matchState.window.base) as U32;
        let cdictLen: U32 = cdictEnd - (*cd).matchState.window.dictLimit;
        if cdictLen == 0 {
            /* don't even attach dictionaries with no contents */
        } else {
            (*cctx).blockState.matchState.dictMatchState = &(*cd).matchState;

            /* prep working match state so dict matches never have negative indices
             * when they are translated to the working context's index space. */
            if (*cctx).blockState.matchState.window.dictLimit < cdictEnd {
                (*cctx).blockState.matchState.window.nextSrc =
                    (*cctx).blockState.matchState.window.base.wrapping_offset(cdictEnd as isize);
                ZSTD_window_clear(&mut (*cctx).blockState.matchState.window);
            }
            /* loadedDictEnd is expressed within the referential of the active context */
            (*cctx).blockState.matchState.loadedDictEnd =
                (*cctx).blockState.matchState.window.dictLimit;
        }
    }

    (*cctx).dictID = (*cd).dictID;
    (*cctx).dictContentSize = (*cd).dictContentSize;

    /* copy block state */
    ZSTD_memcpy(
        (*cctx).blockState.prevCBlock as *mut u8,
        &(*cd).cBlockState as *const ZSTD_compressedBlockState_t as *const u8,
        core::mem::size_of::<ZSTD_compressedBlockState_t>() as size_t,
    );

    0
}

pub unsafe fn ZSTD_copyCDictTableIntoCCtx(
    dst: *mut U32,
    src: *const U32,
    tableSize: size_t,
    cParams: *const ZSTD_compressionParameters,
) {
    if ZSTD_CDictIndicesAreTagged(cParams) != 0 {
        /* Remove tags from the CDict table if they are present. */
        let mut i: size_t = 0;
        while i < tableSize {
            let taggedIndex: U32 = *src.wrapping_add(i);
            let index: U32 = taggedIndex >> ZSTD_SHORT_CACHE_TAG_BITS;
            *dst.wrapping_add(i) = index;
            i += 1;
        }
    } else {
        ZSTD_memcpy(
            dst as *mut u8,
            src as *const u8,
            tableSize * core::mem::size_of::<U32>() as size_t,
        );
    }
}

pub unsafe fn ZSTD_resetCCtx_byCopyingCDict(
    cctx: *mut ZSTD_CCtx,
    cdict: *const ZSTD_CDict,
    mut params: ZSTD_CCtx_params,
    pledgedSrcSize: U64,
    zbuff: ZSTD_buffered_policy_e,
) -> size_t {
    let cd = cdict_layout(cdict);
    let cdict_cParams: *const ZSTD_compressionParameters = &(*cd).matchState.cParams;

    {
        let windowLog: c_uint = params.cParams.windowLog;
        /* Copy only compression parameters related to tables. */
        params.cParams = *cdict_cParams;
        params.cParams.windowLog = windowLog;
        params.useRowMatchFinder = (*cd).useRowMatchFinder;
        {
            let err = ZSTD_resetCCtx_internal(
                cctx,
                &params,
                pledgedSrcSize,
                0,
                ZSTDcrp_leaveDirty,
                zbuff,
            );
            if ERR_isError(err) != 0 {
                return err;
            }
        }
    }

    ZSTD_cwksp_mark_tables_dirty(&mut (*cctx).workspace);

    /* copy tables */
    {
        let chainSize: size_t = if ZSTD_allocateChainTable(
            (*cdict_cParams).strategy,
            (*cd).useRowMatchFinder,
            0,
        ) != 0
        {
            (1 as size_t) << (*cdict_cParams).chainLog
        } else {
            0
        };
        let hSize: size_t = (1 as size_t) << (*cdict_cParams).hashLog;

        ZSTD_copyCDictTableIntoCCtx(
            (*cctx).blockState.matchState.hashTable,
            (*cd).matchState.hashTable,
            hSize,
            cdict_cParams,
        );

        /* Do not copy cdict's chainTable if cctx params wouldn't use chainTable */
        if ZSTD_allocateChainTable(
            (*cctx).appliedParams.cParams.strategy,
            (*cctx).appliedParams.useRowMatchFinder,
            0,
        ) != 0
        {
            ZSTD_copyCDictTableIntoCCtx(
                (*cctx).blockState.matchState.chainTable,
                (*cd).matchState.chainTable,
                chainSize,
                cdict_cParams,
            );
        }
        /* copy tag table */
        if ZSTD_rowMatchFinderUsed((*cdict_cParams).strategy, (*cd).useRowMatchFinder) != 0 {
            let tagTableSize: size_t = hSize;
            ZSTD_memcpy(
                (*cctx).blockState.matchState.tagTable,
                (*cd).matchState.tagTable,
                tagTableSize,
            );
            (*cctx).blockState.matchState.hashSalt = (*cd).matchState.hashSalt;
        }
    }

    /* Zero the hashTable3, since the cdict never fills it */
    {
        let h3log: U32 = (*cctx).blockState.matchState.hashLog3;
        let h3Size: size_t = if h3log != 0 {
            (1 as size_t) << h3log
        } else {
            0
        };
        ZSTD_memset(
            (*cctx).blockState.matchState.hashTable3 as *mut u8,
            0,
            h3Size * core::mem::size_of::<U32>() as size_t,
        );
    }

    ZSTD_cwksp_mark_tables_clean(&mut (*cctx).workspace);

    /* copy dictionary offsets */
    {
        let srcMatchState: *const ZSTD_MatchState_t = &(*cd).matchState;
        let dstMatchState: *mut ZSTD_MatchState_t = &mut (*cctx).blockState.matchState;
        (*dstMatchState).window = (*srcMatchState).window;
        (*dstMatchState).nextToUpdate = (*srcMatchState).nextToUpdate;
        (*dstMatchState).loadedDictEnd = (*srcMatchState).loadedDictEnd;
    }

    (*cctx).dictID = (*cd).dictID;
    (*cctx).dictContentSize = (*cd).dictContentSize;

    /* copy block state */
    ZSTD_memcpy(
        (*cctx).blockState.prevCBlock as *mut u8,
        &(*cd).cBlockState as *const ZSTD_compressedBlockState_t as *const u8,
        core::mem::size_of::<ZSTD_compressedBlockState_t>() as size_t,
    );

    0
}

pub unsafe fn ZSTD_resetCCtx_usingCDict(
    cctx: *mut ZSTD_CCtx,
    cdict: *const ZSTD_CDict,
    params: *const ZSTD_CCtx_params,
    pledgedSrcSize: U64,
    zbuff: ZSTD_buffered_policy_e,
) -> size_t {
    if ZSTD_shouldAttachDict(cdict, params, pledgedSrcSize) != 0 {
        ZSTD_resetCCtx_byAttachingCDict(cctx, cdict, core::ptr::read(params), pledgedSrcSize, zbuff)
    } else {
        ZSTD_resetCCtx_byCopyingCDict(cctx, cdict, core::ptr::read(params), pledgedSrcSize, zbuff)
    }
}

/* ZSTD_copyCCtx_internal() */
pub unsafe fn ZSTD_copyCCtx_internal(
    dstCCtx: *mut ZSTD_CCtx,
    srcCCtx: *const ZSTD_CCtx,
    fParams: ZSTD_frameParameters,
    pledgedSrcSize: U64,
    zbuff: ZSTD_buffered_policy_e,
) -> size_t {
    if (*srcCCtx).stage != ZSTDcs_init {
        return ERROR(ZSTD_error_stage_wrong);
    }
    ZSTD_memcpy(
        &mut (*dstCCtx).customMem as *mut ZSTD_customMem as *mut u8,
        &(*srcCCtx).customMem as *const ZSTD_customMem as *const u8,
        core::mem::size_of::<ZSTD_customMem>() as size_t,
    );
    {
        let mut params: ZSTD_CCtx_params = core::ptr::read(&(*dstCCtx).requestedParams);
        /* Copy only compression parameters related to tables. */
        params.cParams = (*srcCCtx).appliedParams.cParams;
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

    /* copy tables */
    {
        let chainSize: size_t = if ZSTD_allocateChainTable(
            (*srcCCtx).appliedParams.cParams.strategy,
            (*srcCCtx).appliedParams.useRowMatchFinder,
            0,
        ) != 0
        {
            (1 as size_t) << (*srcCCtx).appliedParams.cParams.chainLog
        } else {
            0
        };
        let hSize: size_t = (1 as size_t) << (*srcCCtx).appliedParams.cParams.hashLog;
        let h3log: U32 = (*srcCCtx).blockState.matchState.hashLog3;
        let h3Size: size_t = if h3log != 0 {
            (1 as size_t) << h3log
        } else {
            0
        };

        ZSTD_memcpy(
            (*dstCCtx).blockState.matchState.hashTable as *mut u8,
            (*srcCCtx).blockState.matchState.hashTable as *const u8,
            hSize * core::mem::size_of::<U32>() as size_t,
        );
        ZSTD_memcpy(
            (*dstCCtx).blockState.matchState.chainTable as *mut u8,
            (*srcCCtx).blockState.matchState.chainTable as *const u8,
            chainSize * core::mem::size_of::<U32>() as size_t,
        );
        ZSTD_memcpy(
            (*dstCCtx).blockState.matchState.hashTable3 as *mut u8,
            (*srcCCtx).blockState.matchState.hashTable3 as *const u8,
            h3Size * core::mem::size_of::<U32>() as size_t,
        );
    }

    ZSTD_cwksp_mark_tables_clean(&mut (*dstCCtx).workspace);

    /* copy dictionary offsets */
    {
        let srcMatchState: *const ZSTD_MatchState_t = &(*srcCCtx).blockState.matchState;
        let dstMatchState: *mut ZSTD_MatchState_t = &mut (*dstCCtx).blockState.matchState;
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
        core::mem::size_of::<ZSTD_compressedBlockState_t>() as size_t,
    );

    0
}

/* ZSTD_copyCCtx() */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_copyCCtx(
    dstCCtx: *mut ZSTD_CCtx,
    srcCCtx: *const ZSTD_CCtx,
    mut pledgedSrcSize: core::ffi::c_ulonglong,
) -> size_t {
    let mut fParams = ZSTD_frameParameters {
        contentSizeFlag: 1, /*content*/
        checksumFlag: 0,    /*checksum*/
        noDictIDFlag: 0,    /*noDictID*/
    };
    let zbuff: ZSTD_buffered_policy_e = (*srcCCtx).bufferedPolicy;
    if pledgedSrcSize == 0 {
        pledgedSrcSize = ZSTD_CONTENTSIZE_UNKNOWN;
    }
    fParams.contentSizeFlag = (pledgedSrcSize != ZSTD_CONTENTSIZE_UNKNOWN) as c_int;

    ZSTD_copyCCtx_internal(dstCCtx, srcCCtx, fParams, pledgedSrcSize, zbuff)
}

/* ZSTD_reduceTable_internal() */
unsafe fn ZSTD_reduceTable_internal(
    table: *mut U32,
    size: U32,
    reducerValue: U32,
    preserveMark: c_int,
) {
    let nbRows: c_int = (size as c_int) / ZSTD_ROWSIZE as c_int;
    let mut cellNb: c_int = 0;
    let mut rowNb: c_int;
    /* Protect special index values < ZSTD_WINDOW_START_INDEX. */
    let reducerThreshold: U32 = reducerValue + ZSTD_WINDOW_START_INDEX;

    rowNb = 0;
    while rowNb < nbRows {
        let mut column: c_int = 0;
        while column < ZSTD_ROWSIZE as c_int {
            let newVal: U32;
            if preserveMark != 0 && *table.wrapping_add(cellNb as usize) == ZSTD_DUBT_UNSORTED_MARK {
                newVal = ZSTD_DUBT_UNSORTED_MARK;
            } else if *table.wrapping_add(cellNb as usize) < reducerThreshold {
                newVal = 0;
            } else {
                newVal = *table.wrapping_add(cellNb as usize) - reducerValue;
            }
            *table.wrapping_add(cellNb as usize) = newVal;
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

/* ZSTD_reduceIndex() : rescale all indexes to avoid future overflow */
pub unsafe fn ZSTD_reduceIndex(
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

/*-=====  Pre-defined compression levels  =====-*/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_maxCLevel() -> c_int {
    ZSTD_MAX_CLEVEL as c_int
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_minCLevel() -> c_int {
    -(ZSTD_TARGETLENGTH_MAX)
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_defaultCLevel() -> c_int {
    ZSTD_CLEVEL_DEFAULT
}

pub unsafe fn ZSTD_dedicatedDictSearch_getCParams(
    compressionLevel: c_int,
    dictSize: size_t,
) -> ZSTD_compressionParameters {
    let mut cParams: ZSTD_compressionParameters =
        ZSTD_getCParams_internal(compressionLevel, 0, dictSize, ZSTD_cpm_createCDict);
    if cParams.strategy == ZSTD_fast || cParams.strategy == ZSTD_dfast {
        /* break */
    } else if cParams.strategy == ZSTD_greedy
        || cParams.strategy == ZSTD_lazy
        || cParams.strategy == ZSTD_lazy2
    {
        cParams.hashLog += ZSTD_LAZY_DDSS_BUCKET_LOG;
    } else {
        /* ZSTD_btlazy2, ZSTD_btopt, ZSTD_btultra, ZSTD_btultra2: break */
    }
    cParams
}

pub unsafe fn ZSTD_dedicatedDictSearch_isSupported(
    cParams: *const ZSTD_compressionParameters,
) -> c_int {
    ((*cParams).strategy >= ZSTD_greedy
        && (*cParams).strategy <= ZSTD_lazy2
        && (*cParams).hashLog > (*cParams).chainLog
        && (*cParams).chainLog <= 24) as c_int
}

/**
 * Reverses the adjustment applied to cparams when enabling dedicated dict
 * search. */
pub unsafe fn ZSTD_dedicatedDictSearch_revertCParams(cParams: *mut ZSTD_compressionParameters) {
    if (*cParams).strategy == ZSTD_fast || (*cParams).strategy == ZSTD_dfast {
        /* break */
    } else if (*cParams).strategy == ZSTD_greedy
        || (*cParams).strategy == ZSTD_lazy
        || (*cParams).strategy == ZSTD_lazy2
    {
        (*cParams).hashLog -= ZSTD_LAZY_DDSS_BUCKET_LOG;
        if (*cParams).hashLog < ZSTD_HASHLOG_MIN as c_uint {
            (*cParams).hashLog = ZSTD_HASHLOG_MIN as c_uint;
        }
    } else {
        /* ZSTD_btlazy2, ZSTD_btopt, ZSTD_btultra, ZSTD_btultra2: break */
    }
}

pub unsafe fn ZSTD_getCParamRowSize(
    srcSizeHint: U64,
    mut dictSize: size_t,
    mode: ZSTD_CParamMode_e,
) -> U64 {
    if mode == ZSTD_cpm_unknown || mode == ZSTD_cpm_noAttachDict || mode == ZSTD_cpm_createCDict {
        /* break */
    } else if mode == ZSTD_cpm_attachDict {
        dictSize = 0;
    }
    {
        let unknown: c_int = (srcSizeHint == ZSTD_CONTENTSIZE_UNKNOWN) as c_int;
        let addedSize: size_t = if unknown != 0 && dictSize > 0 { 500 } else { 0 };
        if unknown != 0 && dictSize == 0 {
            ZSTD_CONTENTSIZE_UNKNOWN
        } else {
            srcSizeHint + dictSize as U64 + addedSize as U64
        }
    }
}

/* ZSTD_getCParams_internal() */
pub unsafe fn ZSTD_getCParams_internal(
    compressionLevel: c_int,
    srcSizeHint: core::ffi::c_ulonglong,
    dictSize: size_t,
    mode: ZSTD_CParamMode_e,
) -> ZSTD_compressionParameters {
    let rSize: U64 = ZSTD_getCParamRowSize(srcSizeHint, dictSize, mode);
    let tableID: U32 = ((rSize <= 256 * (1 << 10)) as U32)
        + ((rSize <= 128 * (1 << 10)) as U32)
        + ((rSize <= 16 * (1 << 10)) as U32);
    let row: c_int;

    if compressionLevel == 0 {
        row = ZSTD_CLEVEL_DEFAULT; /* 0 == default */
    } else if compressionLevel < 0 {
        row = 0; /* entry 0 is baseline for fast mode */
    } else if compressionLevel > ZSTD_MAX_CLEVEL as c_int {
        row = ZSTD_MAX_CLEVEL as c_int;
    } else {
        row = compressionLevel;
    }

    {
        let mut cp: ZSTD_compressionParameters =
            ZSTD_defaultCParameters[tableID as usize][row as usize];
        /* acceleration factor */
        if compressionLevel < 0 {
            let clampedCompressionLevel: c_int = MAX(ZSTD_minCLevel(), compressionLevel);
            cp.targetLength = (-clampedCompressionLevel) as c_uint;
        }
        /* refine parameters based on srcSize & dictSize */
        ZSTD_adjustCParams_internal(cp, srcSizeHint, dictSize, mode, ZSTD_ps_auto)
    }
}

/* ZSTD_getCParams() */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_getCParams(
    compressionLevel: c_int,
    mut srcSizeHint: core::ffi::c_ulonglong,
    dictSize: size_t,
) -> ZSTD_compressionParameters {
    if srcSizeHint == 0 {
        srcSizeHint = ZSTD_CONTENTSIZE_UNKNOWN;
    }
    ZSTD_getCParams_internal(compressionLevel, srcSizeHint, dictSize, ZSTD_cpm_unknown)
}

/* ZSTD_getParams_internal() */
pub unsafe fn ZSTD_getParams_internal(
    compressionLevel: c_int,
    srcSizeHint: core::ffi::c_ulonglong,
    dictSize: size_t,
    mode: ZSTD_CParamMode_e,
) -> ZSTD_parameters {
    let mut params: ZSTD_parameters = core::mem::zeroed();
    let cParams: ZSTD_compressionParameters =
        ZSTD_getCParams_internal(compressionLevel, srcSizeHint, dictSize, mode);
    params.cParams = cParams;
    params.fParams.contentSizeFlag = 1;
    params
}

/* ZSTD_getParams() */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_getParams(
    compressionLevel: c_int,
    mut srcSizeHint: core::ffi::c_ulonglong,
    dictSize: size_t,
) -> ZSTD_parameters {
    if srcSizeHint == 0 {
        srcSizeHint = ZSTD_CONTENTSIZE_UNKNOWN;
    }
    ZSTD_getParams_internal(compressionLevel, srcSizeHint, dictSize, ZSTD_cpm_unknown)
}
