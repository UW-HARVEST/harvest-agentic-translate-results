/* ==========================================================================
 * Part 2 of the translation of `compress/zstd_compress.c`
 * (C lines 1239 .. 2600)
 *
 * This file is textually `include!`d by `zstd_compress.rs`; it must contain
 * items only -- no `use`, no `extern "C"` blocks, no inner attributes.
 * ========================================================================== */

/* from compress/zstd_lazy.h : nb bits to use for the tag */
const ZSTD_ROW_HASH_TAG_BITS: U32 = 8;

/**
 * Initializes the local dictionary using requested parameters.
 * NOTE: Initialization does not employ the pledged src size,
 * because the dictionary may be used for multiple compressions.
 */
unsafe fn ZSTD_initLocalDict(cctx: *mut ZSTD_CCtx) -> usize {
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
pub unsafe extern "C" fn ZSTD_CCtx_reset(cctx: *mut ZSTD_CCtx, reset: ZSTD_ResetDirective) -> usize {
    if reset == ZSTD_reset_session_only || reset == ZSTD_reset_session_and_parameters {
        (*cctx).streamStage = zcss_init;
        (*cctx).pledgedSrcSizePlusOne = 0;
    }
    if reset == ZSTD_reset_parameters || reset == ZSTD_reset_session_and_parameters {
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
unsafe fn ZSTD_clampCParams(
    mut cParams: ZSTD_compressionParameters,
) -> ZSTD_compressionParameters {
    {
        let bounds = ZSTD_cParam_getBounds(ZSTD_c_windowLog);
        if (cParams.windowLog as c_int) < bounds.lowerBound {
            cParams.windowLog = bounds.lowerBound as c_uint;
        } else if cParams.windowLog as c_int > bounds.upperBound {
            cParams.windowLog = bounds.upperBound as c_uint;
        }
    }
    {
        let bounds = ZSTD_cParam_getBounds(ZSTD_c_chainLog);
        if (cParams.chainLog as c_int) < bounds.lowerBound {
            cParams.chainLog = bounds.lowerBound as c_uint;
        } else if cParams.chainLog as c_int > bounds.upperBound {
            cParams.chainLog = bounds.upperBound as c_uint;
        }
    }
    {
        let bounds = ZSTD_cParam_getBounds(ZSTD_c_hashLog);
        if (cParams.hashLog as c_int) < bounds.lowerBound {
            cParams.hashLog = bounds.lowerBound as c_uint;
        } else if cParams.hashLog as c_int > bounds.upperBound {
            cParams.hashLog = bounds.upperBound as c_uint;
        }
    }
    {
        let bounds = ZSTD_cParam_getBounds(ZSTD_c_searchLog);
        if (cParams.searchLog as c_int) < bounds.lowerBound {
            cParams.searchLog = bounds.lowerBound as c_uint;
        } else if cParams.searchLog as c_int > bounds.upperBound {
            cParams.searchLog = bounds.upperBound as c_uint;
        }
    }
    {
        let bounds = ZSTD_cParam_getBounds(ZSTD_c_minMatch);
        if (cParams.minMatch as c_int) < bounds.lowerBound {
            cParams.minMatch = bounds.lowerBound as c_uint;
        } else if cParams.minMatch as c_int > bounds.upperBound {
            cParams.minMatch = bounds.upperBound as c_uint;
        }
    }
    {
        let bounds = ZSTD_cParam_getBounds(ZSTD_c_targetLength);
        if (cParams.targetLength as c_int) < bounds.lowerBound {
            cParams.targetLength = bounds.lowerBound as c_uint;
        } else if cParams.targetLength as c_int > bounds.upperBound {
            cParams.targetLength = bounds.upperBound as c_uint;
        }
    }
    {
        let bounds = ZSTD_cParam_getBounds(ZSTD_c_strategy);
        if (cParams.strategy as c_int) < bounds.lowerBound {
            cParams.strategy = bounds.lowerBound as ZSTD_strategy;
        } else if cParams.strategy as c_int > bounds.upperBound {
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
 * Returns an adjusted window log that is large enough to fit the source and the
 * dictionary.
 * NOTE: srcSize must not be ZSTD_CONTENTSIZE_UNKNOWN.
 */
unsafe fn ZSTD_dictAndWindowLog(windowLog: U32, srcSize: U64, dictSize: U64) -> U32 {
    let maxWindowSize: U64 = 1u64 << ZSTD_WINDOWLOG_MAX;
    /* No dictionary ==> No change */
    if dictSize == 0 {
        return windowLog;
    }
    {
        let windowSize: U64 = 1u64 << windowLog;
        let dictAndWindowSize: U64 = dictSize.wrapping_add(windowSize);
        /* If the window size is already large enough to fit both the source and
         * the dictionary then just use the window size. Otherwise adjust so that
         * it fits the dictionary and the window.
         */
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
 *  mostly downsize to reduce memory consumption and initialization latency.
 * `srcSize` can be ZSTD_CONTENTSIZE_UNKNOWN when not known.
 *  note : `srcSize==0` means 0!
 *  condition : cPar is presumed validated (can be checked using ZSTD_checkCParams()). */
unsafe fn ZSTD_adjustCParams_internal(
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
    if srcSize <= maxWindowResize && (dictSize as U64) <= maxWindowResize {
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
        /* minimum wlog required for valid frame header */
        cPar.windowLog = ZSTD_WINDOWLOG_ABSOLUTEMIN;
    }

    /* We can't use more than 32 bits of hash in total, so that means that we
     * require: (hashLog + 8) <= 32 && (chainLog + 8) <= 32
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
        let rowLog: U32 = BOUNDED(4, cPar.searchLog, 6);
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
    dictSize: usize,
) -> ZSTD_compressionParameters {
    /* resulting cPar is necessarily valid (all parameters within range) */
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
        srcSizeHint = (*CCtxParams).srcSizeHint as U64;
    }
    cParams = ZSTD_getCParams_internal(
        (*CCtxParams).compressionLevel,
        srcSizeHint,
        dictSize,
        mode,
    );
    if (*CCtxParams).ldmParams.enableLdm == ZSTD_ps_enable {
        cParams.windowLog = ZSTD_WINDOWLOG_LIMIT_DEFAULT as c_uint;
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

unsafe fn ZSTD_sizeof_matchState(
    cParams: *const ZSTD_compressionParameters,
    useRowMatchFinder: ZSTD_ParamSwitch_e,
    enableDedicatedDictSearch: c_int,
    forCCtx: U32,
) -> usize {
    /* chain table size should be 0 for fast or row-hash strategies */
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
        MIN(ZSTD_HASHLOG3_MAX, (*cParams).windowLog)
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
    ) + ZSTD_cwksp_aligned64_alloc_size((MaxLL as usize + 1) * core::mem::size_of::<U32>())
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
    let optSpace: usize = if forCCtx != 0 && (*cParams).strategy >= ZSTD_btopt {
        optPotentialSpace
    } else {
        0
    };
    let slackSpace: usize = ZSTD_cwksp_slack_space_required();

    tableSpace + optSpace + slackSpace + lazyAdditionalSpace
}

/* Helper function for calculating memory requirements.
 * Gives a tighter bound than ZSTD_sequenceBound() by taking minMatch into account. */
unsafe fn ZSTD_maxNbSeq(
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
    let windowSize: usize =
        BOUNDED(1u64, 1u64 << (*cParams).windowLog, pledgedSrcSize) as usize;
    let blockSize: usize = MIN(ZSTD_resolveMaxBlockSize(maxBlockSize), windowSize);
    let maxNbSeq: usize = ZSTD_maxNbSeq(blockSize, (*cParams).minMatch, useSequenceProducer);
    let tokenSpace: usize = ZSTD_cwksp_alloc_size(WILDCOPY_OVERLENGTH as usize + blockSize)
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

    let maxNbExternalSeq: usize = ZSTD_sequenceBound(blockSize);
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
    let mut initialParams: ZSTD_CCtx_params = ZSTD_makeCCtxParamsFromCParams(cParams);
    if ZSTD_rowMatchFinderSupported(cParams.strategy) != 0 {
        /* Pick bigger of not using and using row-based matchfinder for greedy
         * and lazy strategies */
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

unsafe fn ZSTD_estimateCCtxSize_internal(compressionLevel: c_int) -> usize {
    let mut tier: c_int = 0;
    let mut largestSize: usize = 0;
    static srcSizeTiers: [core::ffi::c_ulonglong; 4] = [
        16 * (1 << 10),
        128 * (1 << 10),
        256 * (1 << 10),
        ZSTD_CONTENTSIZE_UNKNOWN,
    ];
    while tier < 4 {
        /* Choose the set of cParams for a given level across all srcSizes that
         * give the largest cctxSize */
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
pub unsafe extern "C" fn ZSTD_estimateCCtxSize(compressionLevel: c_int) -> usize {
    let mut level: c_int;
    let mut memBudget: usize = 0;
    level = MIN(compressionLevel, 1);
    while level <= compressionLevel {
        /* Ensure monotonically increasing memory usage as compression level
         * increases */
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
) -> usize {
    let mut initialParams: ZSTD_CCtx_params = ZSTD_makeCCtxParamsFromCParams(cParams);
    if ZSTD_rowMatchFinderSupported(cParams.strategy) != 0 {
        /* Pick bigger of not using and using row-based matchfinder for greedy
         * and lazy strategies */
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

unsafe fn ZSTD_estimateCStreamSize_internal(compressionLevel: c_int) -> usize {
    let cParams: ZSTD_compressionParameters = ZSTD_getCParams_internal(
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
 * tells how much data has been consumed (input) and produced (output) for
 * current frame.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_getFrameProgression(
    cctx: *const ZSTD_CCtx,
) -> ZSTD_frameProgression {
    {
        let mut fp: ZSTD_frameProgression = ZSTD_frameProgression {
            ingested: 0,
            consumed: 0,
            produced: 0,
            flushed: 0,
            currentJobID: 0,
            nbActiveWorkers: 0,
        };
        let buffered: usize = if (*cctx).inBuff.is_null() {
            0
        } else {
            (*cctx).inBuffPos.wrapping_sub((*cctx).inToCompress)
        };
        fp.ingested = (*cctx).consumedSrcSize.wrapping_add(buffered as u64);
        fp.consumed = (*cctx).consumedSrcSize;
        fp.produced = (*cctx).producedCSize;
        /* simplified; some data might still be left within streaming output
         * buffer */
        fp.flushed = (*cctx).producedCSize;
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
    /* over-simplification; could also check if context is currently running in
     * streaming mode, and in which case, report how many bytes are left to be
     * flushed within output buffer */
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
pub unsafe extern "C" fn ZSTD_reset_compressedBlockState(
    bs: *mut ZSTD_compressedBlockState_t,
) {
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
unsafe fn ZSTD_invalidateMatchState(ms: *mut ZSTD_MatchState_t) {
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
type ZSTD_compResetPolicy_e = c_int;
const ZSTDcrp_makeClean: ZSTD_compResetPolicy_e = 0;
const ZSTDcrp_leaveDirty: ZSTD_compResetPolicy_e = 1;

/**
 * Controls, for this matchState reset, whether indexing can continue where it
 * left off (ZSTDirp_continue), or whether it needs to be restarted from zero
 * (ZSTDirp_reset).
 */
type ZSTD_indexResetPolicy_e = c_int;
const ZSTDirp_continue: ZSTD_indexResetPolicy_e = 0;
const ZSTDirp_reset: ZSTD_indexResetPolicy_e = 1;

type ZSTD_resetTarget_e = c_int;
const ZSTD_resetTarget_CDict: ZSTD_resetTarget_e = 0;
const ZSTD_resetTarget_CCtx: ZSTD_resetTarget_e = 1;

/* Mixes bits in a 64 bits in a value, based on XXH3_rrmxmx */
unsafe fn ZSTD_bitmix(mut val: U64, len: U64) -> U64 {
    val ^= ZSTD_rotateRight_U64(val, 49) ^ ZSTD_rotateRight_U64(val, 24);
    val = val.wrapping_mul(0x9FB21C651E98DF25u64);
    val ^= (val >> 35).wrapping_add(len);
    val = val.wrapping_mul(0x9FB21C651E98DF25u64);
    val ^ (val >> 28)
}

/* Mixes in the hashSalt and hashSaltEntropy to create a new hashSalt */
unsafe fn ZSTD_advanceHashSalt(ms: *mut ZSTD_MatchState_t) {
    (*ms).hashSalt =
        ZSTD_bitmix((*ms).hashSalt, 8) ^ ZSTD_bitmix((*ms).hashSaltEntropy as U64, 4);
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
    /* disable chain table allocation for fast or row-based strategies */
    let chainSize: usize = if ZSTD_allocateChainTable(
        (*cParams).strategy,
        useRowMatchFinder,
        ((*ms).dedicatedDictSearch != 0 && forWho == ZSTD_resetTarget_CDict) as U32,
    ) != 0
    {
        1usize << (*cParams).chainLog
    } else {
        0
    };
    let hSize: usize = 1usize << (*cParams).hashLog;
    let hashLog3: U32 = if forWho == ZSTD_resetTarget_CCtx && (*cParams).minMatch == 3 {
        MIN(ZSTD_HASHLOG3_MAX, (*cParams).windowLog)
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
        /* We want to generate a new salt in case we reset a Cctx, but we always
         * want to use 0 when we reset a Cdict */
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
            let rowLog: U32 = BOUNDED(4, (*cParams).searchLog, 6);
            (*ms).rowHashLog = (*cParams).hashLog - rowLog;
        }
    }

    /* opt parser space */
    if forWho == ZSTD_resetTarget_CCtx && (*cParams).strategy >= ZSTD_btopt {
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
const ZSTD_INDEXOVERFLOW_MARGIN: U32 = 16 * (1 << 20);

unsafe fn ZSTD_indexTooCloseToMax(w: ZSTD_window_t) -> c_int {
    (w.nextSrc.offset_from(w.base) as usize
        > (ZSTD_CURRENT_MAX - ZSTD_INDEXOVERFLOW_MARGIN) as usize) as c_int
}

/** ZSTD_dictTooBig():
 * When dictionaries are larger than ZSTD_CHUNKSIZE_MAX they can't be loaded in
 * one go generically. So we ensure that in that case we reset the tables to
 * zero, so that we can load as much of the dictionary as possible.
 */
unsafe fn ZSTD_dictTooBig(loadedDictSize: usize) -> c_int {
    (loadedDictSize > ZSTD_CHUNKSIZE_MAX as usize) as c_int
}

/* ZSTD_resetCCtx_internal() :
 * @param loadedDictSize The size of the dictionary to be loaded
 * into the context, if any. If no dictionary is used, or the
 * dictionary is being attached / copied, then pass 0.
 * note : `params` are assumed fully validated at this stage.
 */
unsafe fn ZSTD_resetCCtx_internal(
    zc: *mut ZSTD_CCtx,
    mut params: *const ZSTD_CCtx_params,
    pledgedSrcSize: U64,
    loadedDictSize: usize,
    crp: ZSTD_compResetPolicy_e,
    zbuff: ZSTD_buffered_policy_e,
) -> usize {
    let ws: *mut ZSTD_cwksp = &mut (*zc).workspace;

    (*zc).isFirstBlock = 1;

    /* Set applied params early so we can modify them for LDM,
     * and point params at the applied params.
     */
    (*zc).appliedParams = *params;
    params = &(*zc).appliedParams;

    if (*params).ldmParams.enableLdm == ZSTD_ps_enable {
        /* Adjust long distance matching parameters */
        ZSTD_ldm_adjustParameters(&mut (*zc).appliedParams.ldmParams, &(*params).cParams);
    }

    {
        let windowSize: usize = MAX(
            1usize,
            MIN(1u64 << (*params).cParams.windowLog, pledgedSrcSize) as usize,
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

        let indexTooClose: c_int =
            ZSTD_indexTooCloseToMax((*zc).blockState.matchState.window);
        let dictTooBig: c_int = ZSTD_dictTooBig(loadedDictSize);
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

        ZSTD_XXH64_reset(&mut (*zc).xxhState, 0);
        (*zc).stage = ZSTDcs_init;
        (*zc).dictID = 0;
        (*zc).dictContentSize = 0;

        ZSTD_reset_compressedBlockState((*zc).blockState.prevCBlock);

        {
            let err_code = ZSTD_reset_matchState(
                &mut (*zc).blockState.matchState,
                ws,
                &(*params).cParams,
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
            let ldmHSize: usize = 1usize << (*params).ldmParams.hashLog;
            (*zc).ldmState.hashTable = ZSTD_cwksp_reserve_aligned64(
                ws,
                ldmHSize * core::mem::size_of::<ldmEntry_t>(),
            ) as *mut ldmEntry_t;
            ZSTD_memset(
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

        /* reserve space for block-level external sequences */
        if ZSTD_hasExtSeqProd(params) != 0 {
            let maxNbExternalSeq: usize = ZSTD_sequenceBound(blockSize);
            (*zc).extSeqBufCapacity = maxNbExternalSeq;
            (*zc).extSeqBuf = ZSTD_cwksp_reserve_aligned64(
                ws,
                maxNbExternalSeq * core::mem::size_of::<ZSTD_Sequence>(),
            ) as *mut ZSTD_Sequence;
        }

        /* buffers */

        /* ZSTD_wildcopy() is used to copy into the literals buffer,
         * so we have to oversize the buffer by WILDCOPY_OVERLENGTH bytes.
         */
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
            let numBuckets: usize = 1usize
                << ((*params).ldmParams.hashLog - (*params).ldmParams.bucketSizeLog);
            (*zc).ldmState.bucketOffsets = ZSTD_cwksp_reserve_buffer(ws, numBuckets);
            ZSTD_memset(
                (*zc).ldmState.bucketOffsets as *mut c_void,
                0,
                numBuckets,
            );
        }

        /* sequences storage */
        ZSTD_referenceExternalSequences(zc, core::ptr::null_mut(), 0);
        (*zc).seqStore.maxNbSeq = maxNbSeq;
        (*zc).seqStore.llCode =
            ZSTD_cwksp_reserve_buffer(ws, maxNbSeq * core::mem::size_of::<BYTE>());
        (*zc).seqStore.mlCode =
            ZSTD_cwksp_reserve_buffer(ws, maxNbSeq * core::mem::size_of::<BYTE>());
        (*zc).seqStore.ofCode =
            ZSTD_cwksp_reserve_buffer(ws, maxNbSeq * core::mem::size_of::<BYTE>());

        (*zc).initialized = 1;

        0
    }
}

/* ZSTD_invalidateRepCodes() :
 * ensures next compression will not use repcodes from previous block.
 * Note : only works with regular variant;
 *        do not use with extDict variant ! */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_invalidateRepCodes(cctx: *mut ZSTD_CCtx) {
    let mut i: c_int;
    i = 0;
    while (i as usize) < ZSTD_REP_NUM {
        (*(*cctx).blockState.prevCBlock).rep[i as usize] = 0;
        i += 1;
    }
}

/* These are the approximate sizes for each strategy past which copying the
 * dictionary tables into the working context is faster than using them
 * in-place.
 */
static attachDictSizeCutoffs: [usize; ZSTD_STRATEGY_MAX as usize + 1] = [
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

unsafe fn ZSTD_shouldAttachDict(
    cdict: *const ZSTD_CDict,
    params: *const ZSTD_CCtx_params,
    pledgedSrcSize: U64,
) -> c_int {
    let cutoff: usize =
        attachDictSizeCutoffs[(*cdict).matchState.cParams.strategy as usize];
    let dedicatedDictSearch: c_int = (*cdict).matchState.dedicatedDictSearch;
    (dedicatedDictSearch != 0
        || ((pledgedSrcSize <= cutoff as U64
            || pledgedSrcSize == ZSTD_CONTENTSIZE_UNKNOWN
            || (*params).attachDictPref == ZSTD_dictForceAttach)
            && (*params).attachDictPref != ZSTD_dictForceCopy
            && (*params).forceWindow == 0)) as c_int
    /* dictMatchState isn't correctly handled in _enforceMaxDist */
}

unsafe fn ZSTD_resetCCtx_byAttachingCDict(
    cctx: *mut ZSTD_CCtx,
    cdict: *const ZSTD_CDict,
    mut params: ZSTD_CCtx_params,
    pledgedSrcSize: U64,
    zbuff: ZSTD_buffered_policy_e,
) -> usize {
    {
        let mut adjusted_cdict_cParams: ZSTD_compressionParameters =
            (*cdict).matchState.cParams;
        let windowLog: c_uint = params.cParams.windowLog;
        /* Resize working context table params for input only, since the dict
         * has its own tables. */
        /* pledgedSrcSize == 0 means 0! */

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
        let cdictEnd: U32 = (*cdict)
            .matchState
            .window
            .nextSrc
            .offset_from((*cdict).matchState.window.base) as U32;
        let cdictLen: U32 = cdictEnd.wrapping_sub((*cdict).matchState.window.dictLimit);
        if cdictLen == 0 {
            /* don't even attach dictionaries with no contents */
        } else {
            (*cctx).blockState.matchState.dictMatchState = &(*cdict).matchState;

            /* prep working match state so dict matches never have negative
             * indices when they are translated to the working context's index
             * space. */
            if (*cctx).blockState.matchState.window.dictLimit < cdictEnd {
                (*cctx).blockState.matchState.window.nextSrc = (*cctx)
                    .blockState
                    .matchState
                    .window
                    .base
                    .wrapping_add(cdictEnd as usize);
                ZSTD_window_clear(&mut (*cctx).blockState.matchState.window);
            }
            /* loadedDictEnd is expressed within the referential of the active
             * context */
            (*cctx).blockState.matchState.loadedDictEnd =
                (*cctx).blockState.matchState.window.dictLimit;
        }
    }

    (*cctx).dictID = (*cdict).dictID;
    (*cctx).dictContentSize = (*cdict).dictContentSize;

    /* copy block state */
    ZSTD_memcpy(
        (*cctx).blockState.prevCBlock as *mut c_void,
        &(*cdict).cBlockState as *const ZSTD_compressedBlockState_t as *const c_void,
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
    let cdict_cParams: *const ZSTD_compressionParameters = &(*cdict).matchState.cParams;

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

    ZSTD_cwksp_mark_tables_dirty(&mut (*cctx).workspace);

    /* copy tables */
    {
        let chainSize: usize = if ZSTD_allocateChainTable(
            (*cdict_cParams).strategy,
            (*cdict).useRowMatchFinder,
            0, /* DDS guaranteed disabled */
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

        /* Do not copy cdict's chainTable if cctx has parameters such that it
         * would not use chainTable */
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
        if ZSTD_rowMatchFinderUsed((*cdict_cParams).strategy, (*cdict).useRowMatchFinder)
            != 0
        {
            let tagTableSize: usize = hSize;
            ZSTD_memcpy(
                (*cctx).blockState.matchState.tagTable as *mut c_void,
                (*cdict).matchState.tagTable as *const c_void,
                tagTableSize,
            );
            (*cctx).blockState.matchState.hashSalt = (*cdict).matchState.hashSalt;
        }
    }

    /* Zero the hashTable3, since the cdict never fills it */
    {
        let h3log: U32 = (*cctx).blockState.matchState.hashLog3;
        let h3Size: usize = if h3log != 0 { 1usize << h3log } else { 0 };
        ZSTD_memset(
            (*cctx).blockState.matchState.hashTable3 as *mut c_void,
            0,
            h3Size * core::mem::size_of::<U32>(),
        );
    }

    ZSTD_cwksp_mark_tables_clean(&mut (*cctx).workspace);

    /* copy dictionary offsets */
    {
        let srcMatchState: *const ZSTD_MatchState_t = &(*cdict).matchState;
        let dstMatchState: *mut ZSTD_MatchState_t = &mut (*cctx).blockState.matchState;
        (*dstMatchState).window = (*srcMatchState).window;
        (*dstMatchState).nextToUpdate = (*srcMatchState).nextToUpdate;
        (*dstMatchState).loadedDictEnd = (*srcMatchState).loadedDictEnd;
    }

    (*cctx).dictID = (*cdict).dictID;
    (*cctx).dictContentSize = (*cdict).dictContentSize;

    /* copy block state */
    ZSTD_memcpy(
        (*cctx).blockState.prevCBlock as *mut c_void,
        &(*cdict).cBlockState as *const ZSTD_compressedBlockState_t as *const c_void,
        core::mem::size_of::<ZSTD_compressedBlockState_t>(),
    );

    0
}

/* We have a choice between copying the dictionary context into the working
 * context, or referencing the dictionary context from the working context
 * in-place. We decide here which strategy to use. */
unsafe fn ZSTD_resetCCtx_usingCDict(
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
 *  Only works during stage ZSTDcs_init (i.e. after creation, but before first
 *  call to ZSTD_compressContinue()).
 * `windowLog` value is enforced if != 0, otherwise value is copied from srcCCtx.
 * @return : 0, or an error code */
unsafe fn ZSTD_copyCCtx_internal(
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
        &mut (*dstCCtx).customMem as *mut ZSTD_customMem as *mut c_void,
        &(*srcCCtx).customMem as *const ZSTD_customMem as *const c_void,
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
        ZSTD_resetCCtx_internal(
            dstCCtx,
            &params,
            pledgedSrcSize,
            /* loadedDictSize */ 0,
            ZSTDcrp_leaveDirty,
            zbuff,
        );
    }

    ZSTD_cwksp_mark_tables_dirty(&mut (*dstCCtx).workspace);

    /* copy tables */
    {
        let chainSize: usize = if ZSTD_allocateChainTable(
            (*srcCCtx).appliedParams.cParams.strategy,
            (*srcCCtx).appliedParams.useRowMatchFinder,
            0, /* forDDSDict */
        ) != 0
        {
            1usize << (*srcCCtx).appliedParams.cParams.chainLog
        } else {
            0
        };
        let hSize: usize = 1usize << (*srcCCtx).appliedParams.cParams.hashLog;
        let h3log: U32 = (*srcCCtx).blockState.matchState.hashLog3;
        let h3Size: usize = if h3log != 0 { 1usize << h3log } else { 0 };

        ZSTD_memcpy(
            (*dstCCtx).blockState.matchState.hashTable as *mut c_void,
            (*srcCCtx).blockState.matchState.hashTable as *const c_void,
            hSize * core::mem::size_of::<U32>(),
        );
        ZSTD_memcpy(
            (*dstCCtx).blockState.matchState.chainTable as *mut c_void,
            (*srcCCtx).blockState.matchState.chainTable as *const c_void,
            chainSize * core::mem::size_of::<U32>(),
        );
        ZSTD_memcpy(
            (*dstCCtx).blockState.matchState.hashTable3 as *mut c_void,
            (*srcCCtx).blockState.matchState.hashTable3 as *const c_void,
            h3Size * core::mem::size_of::<U32>(),
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
        (*dstCCtx).blockState.prevCBlock as *mut c_void,
        (*srcCCtx).blockState.prevCBlock as *const c_void,
        core::mem::size_of::<ZSTD_compressedBlockState_t>(),
    );

    0
}

/* ZSTD_copyCCtx() :
 *  Duplicate an existing context `srcCCtx` into another one `dstCCtx`.
 *  Only works during stage ZSTDcs_init (i.e. after creation, but before first
 *  call to ZSTD_compressContinue()).
 *  pledgedSrcSize==0 means "unknown".
 *  @return : 0, or an error code */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_copyCCtx(
    dstCCtx: *mut ZSTD_CCtx,
    srcCCtx: *const ZSTD_CCtx,
    mut pledgedSrcSize: core::ffi::c_ulonglong,
) -> usize {
    let mut fParams: ZSTD_frameParameters = ZSTD_frameParameters {
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
