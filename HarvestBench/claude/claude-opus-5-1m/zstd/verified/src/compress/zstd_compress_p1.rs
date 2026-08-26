/* zstd_compress.c — part 1 (C lines 1..1238)
 *
 * Translated from compress/zstd_compress.c.
 * This file is `include!`d into `crate::compress::zstd_compress`; it must
 * contain items only (no `use`, no `extern "C"` blocks).
 */

/*-*************************************
*  Helper functions
***************************************/
/* ZSTD_compressBound() */
#[unsafe(no_mangle)]
pub extern "C" fn ZSTD_compressBound(srcSize: usize) -> usize {
    let r: usize = ZSTD_COMPRESSBOUND(srcSize);
    if r == 0 {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    r
}

/*-*************************************
*  Context memory management
***************************************/
/* `struct ZSTD_CDict_s` lives in `zstd_compress_internal` as `ZSTD_CDict`. */

/* `ZSTD_cpuid_bmi2(ZSTD_cpuid())` from common/cpu.h */
fn ZSTD_cpuid_bmi2_supported() -> c_int {
    #[cfg(target_arch = "x86_64")]
    {
        return std::arch::is_x86_feature_detected!("bmi2") as c_int;
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        return 0;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_createCCtx() -> *mut ZSTD_CCtx {
    ZSTD_createCCtx_advanced(ZSTD_defaultCMem)
}

unsafe fn ZSTD_initCCtx(cctx: *mut ZSTD_CCtx, memManager: ZSTD_customMem) {
    ZSTD_memset(cctx as *mut c_void, 0, core::mem::size_of::<ZSTD_CCtx>());
    (*cctx).customMem = memManager;
    (*cctx).bmi2 = ZSTD_cpuSupportsBmi2();
    {
        let err: usize = ZSTD_CCtx_reset(cctx, ZSTD_reset_parameters);
        let _ = err;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_createCCtx_advanced(customMem: ZSTD_customMem) -> *mut ZSTD_CCtx {
    if ((customMem.customAlloc.is_none()) as c_int) ^ ((customMem.customFree.is_none()) as c_int)
        != 0
    {
        return core::ptr::null_mut();
    }
    {
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
    let mut ws: ZSTD_cwksp = core::mem::zeroed();
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

    ZSTD_memset(cctx as *mut c_void, 0, core::mem::size_of::<ZSTD_CCtx>());
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
    (*cctx).bmi2 = ZSTD_cpuid_bmi2_supported();
    cctx
}

/**
 * Clears and frees all of the dictionaries in the CCtx.
 */
unsafe fn ZSTD_clearAllDicts(cctx: *mut ZSTD_CCtx) {
    ZSTD_customFree((*cctx).localDict.dictBuffer, (*cctx).customMem);
    let _ = ZSTD_freeCDict((*cctx).localDict.cdict);
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

unsafe fn ZSTD_sizeof_localDict(dict: ZSTD_localDict) -> usize {
    let bufferSize: usize = if !dict.dictBuffer.is_null() {
        dict.dictSize
    } else {
        0
    };
    let cdictSize: usize = ZSTD_sizeof_CDict(dict.cdict);
    bufferSize + cdictSize
}

unsafe fn ZSTD_freeCCtxContent(cctx: *mut ZSTD_CCtx) {
    ZSTD_clearAllDicts(cctx);
    /* ZSTD_MULTITHREAD is not defined: no mtctx to free */
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
        let cctxInWorkspace: c_int =
            ZSTD_cwksp_owns_buffer(&(*cctx).workspace, cctx as *const c_void);
        ZSTD_freeCCtxContent(cctx);
        if cctxInWorkspace == 0 {
            ZSTD_customFree(cctx as *mut c_void, (*cctx).customMem);
        }
    }
    0
}

unsafe fn ZSTD_sizeof_mtctx(cctx: *const ZSTD_CCtx) -> usize {
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
    (if (*cctx).workspace.workspace as *const c_void == cctx as *const c_void {
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
fn ZSTD_rowMatchFinderSupported(strategy: ZSTD_strategy) -> c_int {
    (strategy >= ZSTD_greedy && strategy <= ZSTD_lazy2) as c_int
}

/* Returns true if the strategy and useRowMatchFinder mode indicate that we will
 * use the row based matchfinder for this compression.
 */
fn ZSTD_rowMatchFinderUsed(strategy: ZSTD_strategy, mode: ZSTD_ParamSwitch_e) -> c_int {
    (ZSTD_rowMatchFinderSupported(strategy) != 0 && (mode == ZSTD_ps_enable)) as c_int
}

/* Returns row matchfinder usage given an initial mode and cParams */
unsafe fn ZSTD_resolveRowMatchFinderMode(
    mut mode: ZSTD_ParamSwitch_e,
    cParams: *const ZSTD_compressionParameters,
) -> ZSTD_ParamSwitch_e {
    if mode != ZSTD_ps_auto {
        return mode; /* if requested enabled, but no SIMD, we still will use row matchfinder */
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

/* Returns block splitter usage (generally speaking, when using slower/stronger
 * compression modes) */
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

/* Returns 1 if the arguments indicate that we should allocate a chainTable, 0 otherwise */
fn ZSTD_allocateChainTable(
    strategy: ZSTD_strategy,
    useRowMatchFinder: ZSTD_ParamSwitch_e,
    forDDSDict: U32,
) -> c_int {
    /* We always should allocate a chaintable if we are allocating a matchstate for a
     * DDS dictionary matchstate. We do not allocate a chaintable if we are using
     * ZSTD_fast, or are using the row-based matchfinder.
     */
    (forDDSDict != 0
        || ((strategy != ZSTD_fast) && ZSTD_rowMatchFinderUsed(strategy, useRowMatchFinder) == 0))
        as c_int
}

/* Returns ZSTD_ps_enable if compression parameters are such that we should
 * enable long distance matching (wlog >= 27, strategy >= btopt).
 * Returns ZSTD_ps_disable otherwise.
 */
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

/* Resolves maxBlockSize to the default if no value is present. */
fn ZSTD_resolveMaxBlockSize(maxBlockSize: usize) -> usize {
    if maxBlockSize == 0 {
        ZSTD_BLOCKSIZE_MAX
    } else {
        maxBlockSize
    }
}

fn ZSTD_resolveExternalRepcodeSearch(
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
 * indices are tagged. If so, the tags need to be removed in
 * ZSTD_resetCCtx_byCopyingCDict. */
unsafe fn ZSTD_CDictIndicesAreTagged(cParams: *const ZSTD_compressionParameters) -> c_int {
    ((*cParams).strategy == ZSTD_fast || (*cParams).strategy == ZSTD_dfast) as c_int
}

unsafe fn ZSTD_makeCCtxParamsFromCParams(
    cParams: ZSTD_compressionParameters,
) -> ZSTD_CCtx_params {
    let mut cctxParams: ZSTD_CCtx_params = core::mem::zeroed();
    /* should not matter, as all cParams are presumed properly defined */
    ZSTD_CCtxParams_init(&mut cctxParams, ZSTD_CLEVEL_DEFAULT);
    cctxParams.cParams = cParams;

    /* Adjust advanced params according to cParams */
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

unsafe fn ZSTD_createCCtxParams_advanced(customMem: ZSTD_customMem) -> *mut ZSTD_CCtx_params {
    let params: *mut ZSTD_CCtx_params;
    if ((customMem.customAlloc.is_none()) as c_int) ^ ((customMem.customFree.is_none()) as c_int)
        != 0
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
 * @param compressionLevel If params are derived from a compression level then that
 * compression level, otherwise ZSTD_NO_CLEVEL.
 */
unsafe fn ZSTD_CCtxParams_init_internal(
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
 * @param params Validated zstd parameters.
 */
unsafe fn ZSTD_CCtxParams_setZstdParams(
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
            bounds.lowerBound = ZSTD_minCLevel();
            bounds.upperBound = ZSTD_maxCLevel();
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
unsafe fn ZSTD_cParam_clampBounds(cParam: ZSTD_cParameter, value: *mut c_int) -> usize {
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

/* BOUNDCHECK(cParam, val) */
unsafe fn ZSTD_p1_boundcheck(cParam: ZSTD_cParameter, val: c_int) -> usize {
    if ZSTD_cParam_withinBounds(cParam, val) == 0 {
        return ERROR(ZSTD_error_parameter_outOfBound);
    }
    0
}

fn ZSTD_isUpdateAuthorized(param: ZSTD_cParameter) -> c_int {
    match param {
        ZSTD_c_compressionLevel
        | ZSTD_c_hashLog
        | ZSTD_c_chainLog
        | ZSTD_c_searchLog
        | ZSTD_c_minMatch
        | ZSTD_c_targetLength
        | ZSTD_c_strategy
        | ZSTD_c_blockSplitterLevel => 1,

        /* every other parameter (and the C `default:` case) is not updatable */
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
            {
                let e = ZSTD_p1_boundcheck(ZSTD_c_format, value);
                if e != 0 {
                    return e;
                }
            }
            (*CCtxParams).format = value as ZSTD_format_e;
            (*CCtxParams).format as usize
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
            0 /* return type (size_t) cannot represent negative values */
        }

        ZSTD_c_windowLog => {
            if value != 0 {
                /* 0 => use default */
                let e = ZSTD_p1_boundcheck(ZSTD_c_windowLog, value);
                if e != 0 {
                    return e;
                }
            }
            (*CCtxParams).cParams.windowLog = value as U32;
            (*CCtxParams).cParams.windowLog as usize
        }

        ZSTD_c_hashLog => {
            if value != 0 {
                /* 0 => use default */
                let e = ZSTD_p1_boundcheck(ZSTD_c_hashLog, value);
                if e != 0 {
                    return e;
                }
            }
            (*CCtxParams).cParams.hashLog = value as U32;
            (*CCtxParams).cParams.hashLog as usize
        }

        ZSTD_c_chainLog => {
            if value != 0 {
                /* 0 => use default */
                let e = ZSTD_p1_boundcheck(ZSTD_c_chainLog, value);
                if e != 0 {
                    return e;
                }
            }
            (*CCtxParams).cParams.chainLog = value as U32;
            (*CCtxParams).cParams.chainLog as usize
        }

        ZSTD_c_searchLog => {
            if value != 0 {
                /* 0 => use default */
                let e = ZSTD_p1_boundcheck(ZSTD_c_searchLog, value);
                if e != 0 {
                    return e;
                }
            }
            (*CCtxParams).cParams.searchLog = value as U32;
            value as usize
        }

        ZSTD_c_minMatch => {
            if value != 0 {
                /* 0 => use default */
                let e = ZSTD_p1_boundcheck(ZSTD_c_minMatch, value);
                if e != 0 {
                    return e;
                }
            }
            (*CCtxParams).cParams.minMatch = value as U32;
            (*CCtxParams).cParams.minMatch as usize
        }

        ZSTD_c_targetLength => {
            {
                let e = ZSTD_p1_boundcheck(ZSTD_c_targetLength, value);
                if e != 0 {
                    return e;
                }
            }
            (*CCtxParams).cParams.targetLength = value as U32;
            (*CCtxParams).cParams.targetLength as usize
        }

        ZSTD_c_strategy => {
            if value != 0 {
                /* 0 => use default */
                let e = ZSTD_p1_boundcheck(ZSTD_c_strategy, value);
                if e != 0 {
                    return e;
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
            /* A 32-bits content checksum will be calculated and written at end of frame (default:0) */
            (*CCtxParams).fParams.checksumFlag = (value != 0) as c_int;
            (*CCtxParams).fParams.checksumFlag as usize
        }

        ZSTD_c_dictIDFlag => {
            /* When applicable, dictionary's dictID is provided in frame header (default:1) */
            (*CCtxParams).fParams.noDictIDFlag = (value == 0) as c_int;
            ((*CCtxParams).fParams.noDictIDFlag == 0) as usize
        }

        ZSTD_c_forceMaxWindow => {
            (*CCtxParams).forceWindow = (value != 0) as c_int;
            (*CCtxParams).forceWindow as usize
        }

        ZSTD_c_forceAttachDict => {
            let pref: ZSTD_dictAttachPref_e = value as ZSTD_dictAttachPref_e;
            {
                let e = ZSTD_p1_boundcheck(ZSTD_c_forceAttachDict, pref as c_int);
                if e != 0 {
                    return e;
                }
            }
            (*CCtxParams).attachDictPref = pref;
            (*CCtxParams).attachDictPref as usize
        }

        ZSTD_c_literalCompressionMode => {
            let lcm: ZSTD_ParamSwitch_e = value as ZSTD_ParamSwitch_e;
            {
                let e = ZSTD_p1_boundcheck(ZSTD_c_literalCompressionMode, lcm as c_int);
                if e != 0 {
                    return e;
                }
            }
            (*CCtxParams).literalCompressionMode = lcm;
            (*CCtxParams).literalCompressionMode as usize
        }

        ZSTD_c_nbWorkers => {
            /* ZSTD_MULTITHREAD not defined */
            if value != 0 {
                return ERROR(ZSTD_error_parameter_unsupported);
            }
            0
        }

        ZSTD_c_jobSize => {
            /* ZSTD_MULTITHREAD not defined */
            if value != 0 {
                return ERROR(ZSTD_error_parameter_unsupported);
            }
            0
        }

        ZSTD_c_overlapLog => {
            /* ZSTD_MULTITHREAD not defined */
            if value != 0 {
                return ERROR(ZSTD_error_parameter_unsupported);
            }
            0
        }

        ZSTD_c_rsyncable => {
            /* ZSTD_MULTITHREAD not defined */
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
            {
                let e = ZSTD_p1_boundcheck(ZSTD_c_enableLongDistanceMatching, value);
                if e != 0 {
                    return e;
                }
            }
            (*CCtxParams).ldmParams.enableLdm = value as ZSTD_ParamSwitch_e;
            (*CCtxParams).ldmParams.enableLdm as usize
        }

        ZSTD_c_ldmHashLog => {
            if value != 0 {
                /* 0 ==> auto */
                let e = ZSTD_p1_boundcheck(ZSTD_c_ldmHashLog, value);
                if e != 0 {
                    return e;
                }
            }
            (*CCtxParams).ldmParams.hashLog = value as U32;
            (*CCtxParams).ldmParams.hashLog as usize
        }

        ZSTD_c_ldmMinMatch => {
            if value != 0 {
                /* 0 ==> default */
                let e = ZSTD_p1_boundcheck(ZSTD_c_ldmMinMatch, value);
                if e != 0 {
                    return e;
                }
            }
            (*CCtxParams).ldmParams.minMatchLength = value as U32;
            (*CCtxParams).ldmParams.minMatchLength as usize
        }

        ZSTD_c_ldmBucketSizeLog => {
            if value != 0 {
                /* 0 ==> default */
                let e = ZSTD_p1_boundcheck(ZSTD_c_ldmBucketSizeLog, value);
                if e != 0 {
                    return e;
                }
            }
            (*CCtxParams).ldmParams.bucketSizeLog = value as U32;
            (*CCtxParams).ldmParams.bucketSizeLog as usize
        }

        ZSTD_c_ldmHashRateLog => {
            if value != 0 {
                /* 0 ==> default */
                let e = ZSTD_p1_boundcheck(ZSTD_c_ldmHashRateLog, value);
                if e != 0 {
                    return e;
                }
            }
            (*CCtxParams).ldmParams.hashRateLog = value as U32;
            (*CCtxParams).ldmParams.hashRateLog as usize
        }

        ZSTD_c_targetCBlockSize => {
            if value != 0 {
                /* 0 ==> default */
                value = MAX(value, ZSTD_TARGETCBLOCKSIZE_MIN);
                let e = ZSTD_p1_boundcheck(ZSTD_c_targetCBlockSize, value);
                if e != 0 {
                    return e;
                }
            }
            (*CCtxParams).targetCBlockSize = (value as U32) as usize;
            (*CCtxParams).targetCBlockSize
        }

        ZSTD_c_srcSizeHint => {
            if value != 0 {
                /* 0 ==> default */
                let e = ZSTD_p1_boundcheck(ZSTD_c_srcSizeHint, value);
                if e != 0 {
                    return e;
                }
            }
            (*CCtxParams).srcSizeHint = value;
            (*CCtxParams).srcSizeHint as usize
        }

        ZSTD_c_stableInBuffer => {
            {
                let e = ZSTD_p1_boundcheck(ZSTD_c_stableInBuffer, value);
                if e != 0 {
                    return e;
                }
            }
            (*CCtxParams).inBufferMode = value as ZSTD_bufferMode_e;
            (*CCtxParams).inBufferMode as usize
        }

        ZSTD_c_stableOutBuffer => {
            {
                let e = ZSTD_p1_boundcheck(ZSTD_c_stableOutBuffer, value);
                if e != 0 {
                    return e;
                }
            }
            (*CCtxParams).outBufferMode = value as ZSTD_bufferMode_e;
            (*CCtxParams).outBufferMode as usize
        }

        ZSTD_c_blockDelimiters => {
            {
                let e = ZSTD_p1_boundcheck(ZSTD_c_blockDelimiters, value);
                if e != 0 {
                    return e;
                }
            }
            (*CCtxParams).blockDelimiters = value as ZSTD_SequenceFormat_e;
            (*CCtxParams).blockDelimiters as usize
        }

        ZSTD_c_validateSequences => {
            {
                let e = ZSTD_p1_boundcheck(ZSTD_c_validateSequences, value);
                if e != 0 {
                    return e;
                }
            }
            (*CCtxParams).validateSequences = value;
            (*CCtxParams).validateSequences as usize
        }

        ZSTD_c_splitAfterSequences => {
            {
                let e = ZSTD_p1_boundcheck(ZSTD_c_splitAfterSequences, value);
                if e != 0 {
                    return e;
                }
            }
            (*CCtxParams).postBlockSplitter = value as ZSTD_ParamSwitch_e;
            (*CCtxParams).postBlockSplitter as usize
        }

        ZSTD_c_blockSplitterLevel => {
            {
                let e = ZSTD_p1_boundcheck(ZSTD_c_blockSplitterLevel, value);
                if e != 0 {
                    return e;
                }
            }
            (*CCtxParams).preBlockSplitter_level = value;
            (*CCtxParams).preBlockSplitter_level as usize
        }

        ZSTD_c_useRowMatchFinder => {
            {
                let e = ZSTD_p1_boundcheck(ZSTD_c_useRowMatchFinder, value);
                if e != 0 {
                    return e;
                }
            }
            (*CCtxParams).useRowMatchFinder = value as ZSTD_ParamSwitch_e;
            (*CCtxParams).useRowMatchFinder as usize
        }

        ZSTD_c_deterministicRefPrefix => {
            {
                let e = ZSTD_p1_boundcheck(ZSTD_c_deterministicRefPrefix, value);
                if e != 0 {
                    return e;
                }
            }
            (*CCtxParams).deterministicRefPrefix = (value != 0) as c_int;
            (*CCtxParams).deterministicRefPrefix as usize
        }

        ZSTD_c_prefetchCDictTables => {
            {
                let e = ZSTD_p1_boundcheck(ZSTD_c_prefetchCDictTables, value);
                if e != 0 {
                    return e;
                }
            }
            (*CCtxParams).prefetchCDictTables = value as ZSTD_ParamSwitch_e;
            (*CCtxParams).prefetchCDictTables as usize
        }

        ZSTD_c_enableSeqProducerFallback => {
            {
                let e = ZSTD_p1_boundcheck(ZSTD_c_enableSeqProducerFallback, value);
                if e != 0 {
                    return e;
                }
            }
            (*CCtxParams).enableMatchFinderFallback = value;
            (*CCtxParams).enableMatchFinderFallback as usize
        }

        ZSTD_c_maxBlockSize => {
            if value != 0 {
                /* 0 ==> default */
                let e = ZSTD_p1_boundcheck(ZSTD_c_maxBlockSize, value);
                if e != 0 {
                    return e;
                }
            }
            (*CCtxParams).maxBlockSize = value as usize;
            (*CCtxParams).maxBlockSize
        }

        ZSTD_c_repcodeResolution => {
            {
                let e = ZSTD_p1_boundcheck(ZSTD_c_repcodeResolution, value);
                if e != 0 {
                    return e;
                }
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
    pledgedSrcSize: u64,
) -> usize {
    if (*cctx).streamStage != zcss_init {
        return ERROR(ZSTD_error_stage_wrong);
    }
    (*cctx).pledgedSrcSizePlusOne = pledgedSrcSize.wrapping_add(1);
    0
}
