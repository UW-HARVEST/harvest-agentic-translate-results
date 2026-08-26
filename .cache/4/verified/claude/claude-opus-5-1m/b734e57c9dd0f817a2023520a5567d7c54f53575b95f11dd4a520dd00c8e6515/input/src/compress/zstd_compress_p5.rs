/* zstd_compress.c — part 5 (C lines 4791..5925)
 *
 * Translated from compress/zstd_compress.c.
 * This file is `include!`d into `crate::compress::zstd_compress`; it must
 * contain items only (no `use`, no `extern "C"` blocks).
 */

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

    if (*cctx).stage == ZSTDcs_created {
        return ERROR(ZSTD_error_stage_wrong);
    }

    if frame != 0 && (*cctx).stage == ZSTDcs_init {
        fhSize = ZSTD_writeFrameHeader(
            dst,
            dstCapacity,
            &(*cctx).appliedParams,
            (*cctx).pledgedSrcSizePlusOne.wrapping_sub(1),
            (*cctx).dictID,
        );
        if ERR_isError(fhSize) != 0 {
            return fhSize;
        }
        dstCapacity -= fhSize;
        dst = (dst as *mut c_char).add(fhSize) as *mut c_void;
        (*cctx).stage = ZSTDcs_ongoing;
    }

    if srcSize == 0 {
        return fhSize; /* do not generate an empty block if no input */
    }

    if ZSTD_window_update(
        &mut (*ms).window,
        src,
        srcSize,
        (*ms).forceNonContiguous,
    ) == 0
    {
        (*ms).forceNonContiguous = 0;
        (*ms).nextToUpdate = (*ms).window.dictLimit;
    }
    if (*cctx).appliedParams.ldmParams.enableLdm == ZSTD_ps_enable {
        ZSTD_window_update(
            &mut (*cctx).ldmState.window,
            src,
            srcSize,
            0, /* forceNonContiguous */
        );
    }

    if frame == 0 {
        /* overflow check and correction for block mode */
        ZSTD_overflowCorrectIfNeeded(
            ms,
            &mut (*cctx).workspace,
            &(*cctx).appliedParams,
            src,
            (src as *const BYTE).add(srcSize) as *const c_void,
        );
    }

    {
        let cSize: usize = if frame != 0 {
            ZSTD_compress_frameChunk(cctx, dst, dstCapacity, src, srcSize, lastFrameChunk)
        } else {
            ZSTD_compressBlock_internal(cctx, dst, dstCapacity, src, srcSize, 0 /* frame */)
        };
        if ERR_isError(cSize) != 0 {
            return cSize;
        }
        (*cctx).consumedSrcSize = (*cctx).consumedSrcSize.wrapping_add(srcSize as u64);
        (*cctx).producedCSize = (*cctx)
            .producedCSize
            .wrapping_add((cSize + fhSize) as u64);
        if (*cctx).pledgedSrcSizePlusOne != 0 {
            /* control src size */
            if (*cctx).consumedSrcSize.wrapping_add(1) > (*cctx).pledgedSrcSizePlusOne {
                return ERROR(ZSTD_error_srcSize_wrong);
            }
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
    ZSTD_compressContinue_internal(
        cctx,
        dst,
        dstCapacity,
        src,
        srcSize,
        1, /* frame mode */
        0, /* last chunk */
    )
}

/* NOTE: Must just wrap ZSTD_compressContinue_public() */
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
    let cParams: ZSTD_compressionParameters = (*cctx).appliedParams.cParams;
    MIN(
        (*cctx).appliedParams.maxBlockSize,
        1usize << cParams.windowLog,
    )
}

/* NOTE: Must just wrap ZSTD_getBlockSize_deprecated() */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_getBlockSize(cctx: *const ZSTD_CCtx) -> usize {
    ZSTD_getBlockSize_deprecated(cctx)
}

/* NOTE: Must just wrap ZSTD_compressBlock_deprecated() */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_deprecated(
    cctx: *mut ZSTD_CCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    {
        let blockSizeMax: usize = ZSTD_getBlockSize_deprecated(cctx);
        if srcSize > blockSizeMax {
            return ERROR(ZSTD_error_srcSize_wrong);
        }
    }

    ZSTD_compressContinue_internal(
        cctx,
        dst,
        dstCapacity,
        src,
        srcSize,
        0, /* frame mode */
        0, /* last chunk */
    )
}

/* NOTE: Must just wrap ZSTD_compressBlock_deprecated() */
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

/* ZSTD_loadDictionaryContent() :
 *  @return : 0, or an error code
 */
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
    let mut ip: *const BYTE = src as *const BYTE;
    let iend: *const BYTE = ip.wrapping_add(srcSize);
    let loadLdmDict: c_int =
        ((*params).ldmParams.enableLdm == ZSTD_ps_enable && !ls.is_null()) as c_int;

    /* Assert that the ms params match the params we're being given */
    ZSTD_assertEqualCParams((*params).cParams, (*ms).cParams);

    {
        /* Ensure large dictionaries can't cause index overflow */

        /* Allow the dictionary to set indices up to exactly ZSTD_CURRENT_MAX.
         * Dictionaries right at the edge will immediately trigger overflow
         * correction, but I don't want to insert extra constraints here.
         */
        let mut maxDictSize: U32 = ZSTD_CURRENT_MAX - ZSTD_WINDOW_START_INDEX;

        let CDictTaggedIndices: c_int = ZSTD_CDictIndicesAreTagged(&(*params).cParams);
        if CDictTaggedIndices != 0 && tfp == ZSTD_tfp_forCDict {
            /* Some dictionary matchfinders in zstd use "short cache",
             * which treats the lower ZSTD_SHORT_CACHE_TAG_BITS of each
             * CDict hashtable entry as a tag rather than as part of an index.
             * When short cache is used, we need to truncate the dictionary
             * so that its indices don't overlap with the tag. */
            let shortCacheMaxDictSize: U32 =
                (1u32 << (32 - ZSTD_SHORT_CACHE_TAG_BITS)) - ZSTD_WINDOW_START_INDEX;
            maxDictSize = MIN(maxDictSize, shortCacheMaxDictSize);
        }

        /* If the dictionary is too large, only load the suffix of the dictionary. */
        if srcSize > maxDictSize as usize {
            ip = iend.wrapping_sub(maxDictSize as usize);
            src = ip as *const c_void;
            srcSize = maxDictSize as usize;
        }
    }

    ZSTD_window_update(&mut (*ms).window, src, srcSize, 0 /* forceNonContiguous */);

    if loadLdmDict != 0 {
        /* Load the entire dict into LDM matchfinders. */
        ZSTD_window_update(&mut (*ls).window, src, srcSize, 0 /* forceNonContiguous */);
        (*ls).loadedDictEnd = if (*params).forceWindow != 0 {
            0
        } else {
            iend.offset_from((*ls).window.base) as U32
        };
        ZSTD_ldm_fillHashTable(ls, ip, iend, &(*params).ldmParams);
    }

    /* If the dict is larger than we can reasonably index in our tables, only load the suffix. */
    {
        let maxDictSize: U32 = 1u32
            << MIN(
                MAX(
                    (*params).cParams.hashLog + 3,
                    (*params).cParams.chainLog + 1,
                ),
                31,
            );
        if srcSize > maxDictSize as usize {
            ip = iend.wrapping_sub(maxDictSize as usize);
            src = ip as *const c_void;
            srcSize = maxDictSize as usize;
        }
    }

    (*ms).nextToUpdate = ip.offset_from((*ms).window.base) as U32;
    (*ms).loadedDictEnd = if (*params).forceWindow != 0 {
        0
    } else {
        iend.offset_from((*ms).window.base) as U32
    };
    (*ms).forceNonContiguous = (*params).deterministicRefPrefix;

    if srcSize <= HASH_READ_SIZE {
        return 0;
    }

    ZSTD_overflowCorrectIfNeeded(
        ms,
        ws,
        params,
        ip as *const c_void,
        iend as *const c_void,
    );

    match (*params).cParams.strategy {
        ZSTD_fast => {
            ZSTD_fillHashTable(ms, iend as *const c_void, dtlm, tfp);
        }
        ZSTD_dfast => {
            ZSTD_fillDoubleHashTable(ms, iend as *const c_void, dtlm, tfp);
        }

        ZSTD_greedy | ZSTD_lazy | ZSTD_lazy2 => {
            if (*ms).dedicatedDictSearch != 0 {
                ZSTD_dedicatedDictSearch_lazy_loadDictionary(
                    ms,
                    iend.wrapping_sub(HASH_READ_SIZE),
                );
            } else {
                if (*params).useRowMatchFinder == ZSTD_ps_enable {
                    let tagTableSize: usize = 1usize << (*params).cParams.hashLog;
                    ZSTD_memset((*ms).tagTable as *mut c_void, 0, tagTableSize);
                    ZSTD_row_update(ms, iend.wrapping_sub(HASH_READ_SIZE));
                } else {
                    ZSTD_insertAndFindFirstIndex(ms, iend.wrapping_sub(HASH_READ_SIZE));
                }
            }
        }

        ZSTD_btlazy2 /* we want the dictionary table fully sorted */
        | ZSTD_btopt | ZSTD_btultra | ZSTD_btultra2 => {
            ZSTD_updateTree(ms, iend.wrapping_sub(HASH_READ_SIZE), iend);
        }

        _ => {
            /* not possible : not a valid strategy id */
        }
    }

    (*ms).nextToUpdate = iend.offset_from((*ms).window.base) as U32;
    0
}

/* Dictionaries that assign zero probability to symbols that show up causes problems
 * when FSE encoding. Mark dictionaries with zero probability symbols as FSE_repeat_check
 * and only dictionaries with 100% valid symbols can be assumed valid.
 */
unsafe fn ZSTD_dictNCountRepeat(
    normalizedCounter: *mut i16,
    dictMaxSymbolValue: c_uint,
    maxSymbolValue: c_uint,
) -> FSE_repeat {
    let mut s: U32;
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
    let mut offcodeNCount: [i16; (MaxOff + 1) as usize] = [0; (MaxOff + 1) as usize];
    let mut offcodeMaxValue: c_uint = MaxOff;
    let mut dictPtr: *const BYTE = dict as *const BYTE; /* skip magic num and dict ID */
    let dictEnd: *const BYTE = dictPtr.wrapping_add(dictSize);
    dictPtr = dictPtr.wrapping_add(8);
    (*bs).entropy.huf.repeatMode = HUF_repeat_check;

    {
        let mut maxSymbolValue: c_uint = 255;
        let mut hasZeroWeights: c_uint = 1;
        let hufHeaderSize: usize = HUF_readCTable(
            (*bs).entropy.huf.CTable.as_mut_ptr() as *mut HUF_CElt,
            &mut maxSymbolValue,
            dictPtr as *const c_void,
            dictEnd.offset_from(dictPtr) as usize,
            &mut hasZeroWeights,
        );

        /* We only set the loaded table as valid if it contains all non-zero
         * weights. Otherwise, we set it to check */
        if hasZeroWeights == 0 && maxSymbolValue == 255 {
            (*bs).entropy.huf.repeatMode = HUF_repeat_valid;
        }

        if ERR_isError(hufHeaderSize) != 0 {
            return ERROR(ZSTD_error_dictionary_corrupted);
        }
        dictPtr = dictPtr.wrapping_add(hufHeaderSize);
    }

    {
        let mut offcodeLog: c_uint = 0;
        let offcodeHeaderSize: usize = FSE_readNCount(
            offcodeNCount.as_mut_ptr(),
            &mut offcodeMaxValue,
            &mut offcodeLog,
            dictPtr as *const c_void,
            dictEnd.offset_from(dictPtr) as usize,
        );
        if ERR_isError(offcodeHeaderSize) != 0 {
            return ERROR(ZSTD_error_dictionary_corrupted);
        }
        if offcodeLog > OffFSELog {
            return ERROR(ZSTD_error_dictionary_corrupted);
        }
        /* fill all offset symbols to avoid garbage at end of table */
        if ERR_isError(FSE_buildCTable_wksp(
            (*bs).entropy.fse.offcodeCTable.as_mut_ptr(),
            offcodeNCount.as_ptr(),
            MaxOff,
            offcodeLog,
            workspace,
            HUF_WORKSPACE_SIZE,
        )) != 0
        {
            return ERROR(ZSTD_error_dictionary_corrupted);
        }
        /* Defer checking offcodeMaxValue because we need to know the size of the dictionary content */
        dictPtr = dictPtr.wrapping_add(offcodeHeaderSize);
    }

    {
        let mut matchlengthNCount: [i16; (MaxML + 1) as usize] = [0; (MaxML + 1) as usize];
        let mut matchlengthMaxValue: c_uint = MaxML;
        let mut matchlengthLog: c_uint = 0;
        let matchlengthHeaderSize: usize = FSE_readNCount(
            matchlengthNCount.as_mut_ptr(),
            &mut matchlengthMaxValue,
            &mut matchlengthLog,
            dictPtr as *const c_void,
            dictEnd.offset_from(dictPtr) as usize,
        );
        if ERR_isError(matchlengthHeaderSize) != 0 {
            return ERROR(ZSTD_error_dictionary_corrupted);
        }
        if matchlengthLog > MLFSELog {
            return ERROR(ZSTD_error_dictionary_corrupted);
        }
        if ERR_isError(FSE_buildCTable_wksp(
            (*bs).entropy.fse.matchlengthCTable.as_mut_ptr(),
            matchlengthNCount.as_ptr(),
            matchlengthMaxValue,
            matchlengthLog,
            workspace,
            HUF_WORKSPACE_SIZE,
        )) != 0
        {
            return ERROR(ZSTD_error_dictionary_corrupted);
        }
        (*bs).entropy.fse.matchlength_repeatMode = ZSTD_dictNCountRepeat(
            matchlengthNCount.as_mut_ptr(),
            matchlengthMaxValue,
            MaxML,
        );
        dictPtr = dictPtr.wrapping_add(matchlengthHeaderSize);
    }

    {
        let mut litlengthNCount: [i16; (MaxLL + 1) as usize] = [0; (MaxLL + 1) as usize];
        let mut litlengthMaxValue: c_uint = MaxLL;
        let mut litlengthLog: c_uint = 0;
        let litlengthHeaderSize: usize = FSE_readNCount(
            litlengthNCount.as_mut_ptr(),
            &mut litlengthMaxValue,
            &mut litlengthLog,
            dictPtr as *const c_void,
            dictEnd.offset_from(dictPtr) as usize,
        );
        if ERR_isError(litlengthHeaderSize) != 0 {
            return ERROR(ZSTD_error_dictionary_corrupted);
        }
        if litlengthLog > LLFSELog {
            return ERROR(ZSTD_error_dictionary_corrupted);
        }
        if ERR_isError(FSE_buildCTable_wksp(
            (*bs).entropy.fse.litlengthCTable.as_mut_ptr(),
            litlengthNCount.as_ptr(),
            litlengthMaxValue,
            litlengthLog,
            workspace,
            HUF_WORKSPACE_SIZE,
        )) != 0
        {
            return ERROR(ZSTD_error_dictionary_corrupted);
        }
        (*bs).entropy.fse.litlength_repeatMode =
            ZSTD_dictNCountRepeat(litlengthNCount.as_mut_ptr(), litlengthMaxValue, MaxLL);
        dictPtr = dictPtr.wrapping_add(litlengthHeaderSize);
    }

    if dictPtr.wrapping_add(12) > dictEnd {
        return ERROR(ZSTD_error_dictionary_corrupted);
    }
    (*bs).rep[0] = MEM_readLE32(dictPtr.wrapping_add(0) as *const c_void);
    (*bs).rep[1] = MEM_readLE32(dictPtr.wrapping_add(4) as *const c_void);
    (*bs).rep[2] = MEM_readLE32(dictPtr.wrapping_add(8) as *const c_void);
    dictPtr = dictPtr.wrapping_add(12);

    {
        let dictContentSize: usize = dictEnd.offset_from(dictPtr) as usize;
        let mut offcodeMax: U32 = MaxOff;
        if dictContentSize <= ((0u32.wrapping_sub(1)) - (128 * 1024)) as usize {
            let maxOffset: U32 = (dictContentSize as U32).wrapping_add(128 * 1024); /* The maximum offset that must be supported */
            offcodeMax = ZSTD_highbit32(maxOffset); /* Calculate minimum offset code required to represent maxOffset */
        }
        /* All offset values <= dictContentSize + 128 KB must be representable for a valid table */
        (*bs).entropy.fse.offcode_repeatMode = ZSTD_dictNCountRepeat(
            offcodeNCount.as_mut_ptr(),
            offcodeMaxValue,
            MIN(offcodeMax, MaxOff),
        );

        /* All repCodes must be <= dictContentSize and != 0 */
        {
            let mut u: U32 = 0;
            while u < 3 {
                if (*bs).rep[u as usize] == 0 {
                    return ERROR(ZSTD_error_dictionary_corrupted);
                }
                if (*bs).rep[u as usize] as usize > dictContentSize {
                    return ERROR(ZSTD_error_dictionary_corrupted);
                }
                u += 1;
            }
        }
    }

    dictPtr.offset_from(dict as *const BYTE) as usize
}

/* Dictionary format :
 * See :
 * https://github.com/facebook/zstd/blob/release/doc/zstd_compression_format.md#dictionary-format
 */
/* ZSTD_loadZstdDictionary() :
 * @return : dictID, or an error code
 *  assumptions : magic number supposed already checked
 *                dictSize supposed >= 8
 */
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
    let mut dictPtr: *const BYTE = dict as *const BYTE;
    let dictEnd: *const BYTE = dictPtr.wrapping_add(dictSize);
    let dictID: usize;
    let eSize: usize;

    dictID = if (*params).fParams.noDictIDFlag != 0 {
        0
    } else {
        MEM_readLE32(dictPtr.wrapping_add(4) as *const c_void) as usize /* skip magic number */
    };
    eSize = ZSTD_loadCEntropy(bs, workspace, dict, dictSize);
    if ERR_isError(eSize) != 0 {
        return eSize;
    }
    dictPtr = dictPtr.wrapping_add(eSize);

    {
        let dictContentSize: usize = dictEnd.offset_from(dictPtr) as usize;
        let err_code = ZSTD_loadDictionaryContent(
            ms,
            core::ptr::null_mut(),
            ws,
            params,
            dictPtr as *const c_void,
            dictContentSize,
            dtlm,
            tfp,
        );
        if ERR_isError(err_code) != 0 {
            return err_code;
        }
    }
    dictID
}

/** ZSTD_compress_insertDictionary() :
*   @return : dictID, or an error code */
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
        if dictContentType == ZSTD_dct_fullDict {
            return ERROR(ZSTD_error_dictionary_wrong);
        }
        return 0;
    }

    ZSTD_reset_compressedBlockState(bs);

    /* dict restricted modes */
    if dictContentType == ZSTD_dct_rawContent {
        return ZSTD_loadDictionaryContent(ms, ls, ws, params, dict, dictSize, dtlm, tfp);
    }

    if MEM_readLE32(dict) != ZSTD_MAGIC_DICTIONARY {
        if dictContentType == ZSTD_dct_auto {
            return ZSTD_loadDictionaryContent(ms, ls, ws, params, dict, dictSize, dtlm, tfp);
        }
        if dictContentType == ZSTD_dct_fullDict {
            return ERROR(ZSTD_error_dictionary_wrong);
        }
        /* impossible */
    }

    /* dict as full zstd dictionary */
    ZSTD_loadZstdDictionary(bs, ms, ws, params, dict, dictSize, dtlm, tfp, workspace)
}

const ZSTD_USE_CDICT_PARAMS_SRCSIZE_CUTOFF: U64 = (128 * 1024) as U64;
const ZSTD_USE_CDICT_PARAMS_DICTSIZE_MULTIPLIER: U64 = 6;

/* ZSTD_compressBegin_internal() :
 * Assumption : either @dict OR @cdict (or none) is non-NULL, never both
 * @return : 0, or an error code */
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
    let dictContentSize: usize = if !cdict.is_null() {
        (*cdict).dictContentSize
    } else {
        dictSize
    };
    /* ZSTD_TRACE == 1, but `ZSTD_trace_compress_begin` is NULL in this build */
    (*cctx).traceCtx = 0;
    /* params are supposed to be fully validated at this point */
    if !cdict.is_null()
        && (*cdict).dictContentSize > 0
        && (pledgedSrcSize < ZSTD_USE_CDICT_PARAMS_SRCSIZE_CUTOFF
            || pledgedSrcSize
                < ((*cdict).dictContentSize as U64)
                    .wrapping_mul(ZSTD_USE_CDICT_PARAMS_DICTSIZE_MULTIPLIER)
            || pledgedSrcSize == ZSTD_CONTENTSIZE_UNKNOWN
            || (*cdict).compressionLevel == 0)
        && (*params).attachDictPref != ZSTD_dictForceLoad
    {
        return ZSTD_resetCCtx_usingCDict(cctx, cdict, params, pledgedSrcSize, zbuff);
    }

    {
        let err_code = ZSTD_resetCCtx_internal(
            cctx,
            params,
            pledgedSrcSize,
            dictContentSize,
            ZSTDcrp_makeClean,
            zbuff,
        );
        if ERR_isError(err_code) != 0 {
            return err_code;
        }
    }
    {
        let dictID: usize = if !cdict.is_null() {
            ZSTD_compress_insertDictionary(
                (*cctx).blockState.prevCBlock,
                &mut (*cctx).blockState.matchState,
                &mut (*cctx).ldmState,
                &mut (*cctx).workspace,
                &(*cctx).appliedParams,
                (*cdict).dictContent,
                (*cdict).dictContentSize,
                (*cdict).dictContentType,
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
        if ERR_isError(dictID) != 0 {
            return dictID;
        }
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
    pledgedSrcSize: u64,
) -> usize {
    /* compression parameters verification and optimization */
    {
        let err_code = ZSTD_checkCParams((*params).cParams);
        if ERR_isError(err_code) != 0 {
            return err_code;
        }
    }
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

/* ZSTD_compressBegin_advanced() :
*   @return : 0, or an error code */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBegin_advanced(
    cctx: *mut ZSTD_CCtx,
    dict: *const c_void,
    dictSize: usize,
    params: ZSTD_parameters,
    pledgedSrcSize: u64,
) -> usize {
    let mut cctxParams: ZSTD_CCtx_params = core::mem::zeroed();
    ZSTD_CCtxParams_init_internal(&mut cctxParams, &params, ZSTD_NO_CLEVEL);
    ZSTD_compressBegin_advanced_internal(
        cctx,
        dict,
        dictSize,
        ZSTD_dct_auto,
        ZSTD_dtlm_fast,
        core::ptr::null(), /*cdict*/
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
        let params: ZSTD_parameters = ZSTD_getParams_internal(
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
pub unsafe extern "C" fn ZSTD_compressBegin(
    cctx: *mut ZSTD_CCtx,
    compressionLevel: c_int,
) -> usize {
    ZSTD_compressBegin_usingDict_deprecated(cctx, core::ptr::null(), 0, compressionLevel)
}

/* ZSTD_writeEpilogue() :
*   Ends a frame.
*   @return : nb of bytes written into dst (or an error code) */
unsafe fn ZSTD_writeEpilogue(
    cctx: *mut ZSTD_CCtx,
    dst: *mut c_void,
    mut dstCapacity: usize,
) -> usize {
    let ostart: *mut BYTE = dst as *mut BYTE;
    let mut op: *mut BYTE = ostart;

    if (*cctx).stage == ZSTDcs_created {
        return ERROR(ZSTD_error_stage_wrong);
    }

    /* special case : empty frame */
    if (*cctx).stage == ZSTDcs_init {
        let fhSize: usize =
            ZSTD_writeFrameHeader(dst, dstCapacity, &(*cctx).appliedParams, 0, 0);
        if ERR_isError(fhSize) != 0 {
            return fhSize;
        }
        dstCapacity -= fhSize;
        op = op.add(fhSize);
        (*cctx).stage = ZSTDcs_ongoing;
    }

    if (*cctx).stage != ZSTDcs_ending {
        /* write one last empty block, make it the "last" block */
        let cBlockHeader24: U32 = 1 /* last block */ + ((bt_raw as U32) << 1) + 0;
        if dstCapacity < 3 {
            return ERROR(ZSTD_error_dstSize_tooSmall);
        }
        MEM_writeLE24(op as *mut c_void, cBlockHeader24);
        op = op.add(ZSTD_blockHeaderSize);
        dstCapacity -= ZSTD_blockHeaderSize;
    }

    if (*cctx).appliedParams.fParams.checksumFlag != 0 {
        let checksum: U32 = ZSTD_XXH64_digest(&(*cctx).xxhState) as U32;
        if dstCapacity < 4 {
            return ERROR(ZSTD_error_dstSize_tooSmall);
        }
        MEM_writeLE32(op as *mut c_void, checksum);
        op = op.add(4);
    }

    (*cctx).stage = ZSTDcs_created; /* return to "created but no init" status */
    op.offset_from(ostart) as usize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_CCtx_trace(cctx: *mut ZSTD_CCtx, extraCSize: usize) {
    /* ZSTD_TRACE == 1, but `ZSTD_trace_compress_end` is NULL in this build, so
     * the whole body reduces to nothing but the `traceCtx` reset. */
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
    let cSize: usize = ZSTD_compressContinue_internal(
        cctx,
        dst,
        dstCapacity,
        src,
        srcSize,
        1, /* frame mode */
        1, /* last chunk */
    );
    if ERR_isError(cSize) != 0 {
        return cSize;
    }
    endResult = ZSTD_writeEpilogue(
        cctx,
        (dst as *mut c_char).add(cSize) as *mut c_void,
        dstCapacity - cSize,
    );
    if ERR_isError(endResult) != 0 {
        return endResult;
    }
    if (*cctx).pledgedSrcSizePlusOne != 0 {
        /* control src size */
        if (*cctx).pledgedSrcSizePlusOne != (*cctx).consumedSrcSize.wrapping_add(1) {
            return ERROR(ZSTD_error_srcSize_wrong);
        }
    }
    ZSTD_CCtx_trace(cctx, endResult);
    cSize + endResult
}

/* NOTE: Must just wrap ZSTD_compressEnd_public() */
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
    {
        let err_code = ZSTD_checkCParams(params.cParams);
        if ERR_isError(err_code) != 0 {
            return err_code;
        }
    }
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

/* Internal */
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
    {
        let err_code = ZSTD_compressBegin_internal(
            cctx,
            dict,
            dictSize,
            ZSTD_dct_auto,
            ZSTD_dtlm_fast,
            core::ptr::null(),
            params,
            srcSize as U64,
            ZSTDb_not_buffered,
        );
        if ERR_isError(err_code) != 0 {
            return err_code;
        }
    }
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
        let params: ZSTD_parameters = ZSTD_getParams_internal(
            compressionLevel,
            srcSize as U64,
            if !dict.is_null() { dictSize } else { 0 },
            ZSTD_cpm_noAttachDict,
        );
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
    /* ZSTD_COMPRESS_HEAPMODE == 0 : use a CCtx on the stack */
    let mut ctxBody: ZSTD_CCtx = core::mem::zeroed();
    ZSTD_initCCtx(&mut ctxBody, ZSTD_defaultCMem);
    result = ZSTD_compressCCtx(
        &mut ctxBody,
        dst,
        dstCapacity,
        src,
        srcSize,
        compressionLevel,
    );
    ZSTD_freeCCtxContent(&mut ctxBody); /* can't free ctxBody itself, as it's on stack; free only heap content */
    result
}

/* =====  Dictionary API  ===== */

/* ZSTD_estimateCDictSize_advanced() :
 *  Estimate amount of memory that will be needed to create a dictionary with following arguments */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_estimateCDictSize_advanced(
    dictSize: usize,
    cParams: ZSTD_compressionParameters,
    dictLoadMethod: ZSTD_dictLoadMethod_e,
) -> usize {
    ZSTD_cwksp_alloc_size(core::mem::size_of::<ZSTD_CDict>())
        + ZSTD_cwksp_alloc_size(HUF_WORKSPACE_SIZE)
        /* enableDedicatedDictSearch == 1 ensures that CDict estimation will not be too small
         * in case we are using DDS with row-hash. */
        + ZSTD_sizeof_matchState(
            &cParams,
            ZSTD_resolveRowMatchFinderMode(ZSTD_ps_auto, &cParams),
            1, /* enableDedicatedDictSearch */
            0, /* forCCtx */
        )
        + (if dictLoadMethod == ZSTD_dlm_byRef {
            0
        } else {
            ZSTD_cwksp_alloc_size(ZSTD_cwksp_align(
                dictSize,
                core::mem::size_of::<*mut c_void>(),
            ))
        })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_estimateCDictSize(
    dictSize: usize,
    compressionLevel: c_int,
) -> usize {
    let cParams: ZSTD_compressionParameters = ZSTD_getCParams_internal(
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
        return 0; /* support sizeof on NULL */
    }
    /* cdict may be in the workspace */
    (if (*cdict).workspace.workspace == cdict as *mut c_void {
        0
    } else {
        core::mem::size_of::<ZSTD_CDict>()
    }) + ZSTD_cwksp_sizeof(&(*cdict).workspace)
}

unsafe fn ZSTD_initCDict_internal(
    cdict: *mut ZSTD_CDict,
    dictBuffer: *const c_void,
    dictSize: usize,
    dictLoadMethod: ZSTD_dictLoadMethod_e,
    dictContentType: ZSTD_dictContentType_e,
    mut params: ZSTD_CCtx_params,
) -> usize {
    (*cdict).matchState.cParams = params.cParams;
    (*cdict).matchState.dedicatedDictSearch = params.enableDedicatedDictSearch;
    if dictLoadMethod == ZSTD_dlm_byRef || dictBuffer.is_null() || dictSize == 0 {
        (*cdict).dictContent = dictBuffer;
    } else {
        let internalBuffer: *mut c_void = ZSTD_cwksp_reserve_object(
            &mut (*cdict).workspace,
            ZSTD_cwksp_align(dictSize, core::mem::size_of::<*mut c_void>()),
        );
        if internalBuffer.is_null() {
            return ERROR(ZSTD_error_memory_allocation);
        }
        (*cdict).dictContent = internalBuffer;
        ZSTD_memcpy(internalBuffer, dictBuffer, dictSize);
    }
    (*cdict).dictContentSize = dictSize;
    (*cdict).dictContentType = dictContentType;

    (*cdict).entropyWorkspace =
        ZSTD_cwksp_reserve_object(&mut (*cdict).workspace, HUF_WORKSPACE_SIZE) as *mut U32;

    /* Reset the state to no dictionary */
    ZSTD_reset_compressedBlockState(&mut (*cdict).cBlockState);
    {
        let err_code = ZSTD_reset_matchState(
            &mut (*cdict).matchState,
            &mut (*cdict).workspace,
            &params.cParams,
            params.useRowMatchFinder,
            ZSTDcrp_makeClean,
            ZSTDirp_reset,
            ZSTD_resetTarget_CDict,
        );
        if ERR_isError(err_code) != 0 {
            return err_code;
        }
    }
    /* (Maybe) load the dictionary
     * Skips loading the dictionary if it is < 8 bytes.
     */
    {
        params.compressionLevel = ZSTD_CLEVEL_DEFAULT;
        params.fParams.contentSizeFlag = 1;
        {
            let dictID: usize = ZSTD_compress_insertDictionary(
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
            if ERR_isError(dictID) != 0 {
                return dictID;
            }
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
    if ((customMem.customAlloc.is_none()) as c_int) ^ ((customMem.customFree.is_none()) as c_int)
        != 0
    {
        return core::ptr::null_mut();
    }

    {
        let workspaceSize: usize = ZSTD_cwksp_alloc_size(core::mem::size_of::<ZSTD_CDict>())
            + ZSTD_cwksp_alloc_size(HUF_WORKSPACE_SIZE)
            + ZSTD_sizeof_matchState(
                &cParams,
                useRowMatchFinder,
                enableDedicatedDictSearch,
                0, /* forCCtx */
            )
            + (if dictLoadMethod == ZSTD_dlm_byRef {
                0
            } else {
                ZSTD_cwksp_alloc_size(ZSTD_cwksp_align(
                    dictSize,
                    core::mem::size_of::<*mut c_void>(),
                ))
            });
        let workspace: *mut c_void = ZSTD_customMalloc(workspaceSize, customMem);
        let mut ws: ZSTD_cwksp = core::mem::zeroed();
        let cdict: *mut ZSTD_CDict;

        if workspace.is_null() {
            ZSTD_customFree(workspace, customMem);
            return core::ptr::null_mut();
        }

        ZSTD_cwksp_init(&mut ws, workspace, workspaceSize, ZSTD_cwksp_dynamic_alloc);

        cdict = ZSTD_cwksp_reserve_object(&mut ws, core::mem::size_of::<ZSTD_CDict>())
            as *mut ZSTD_CDict;
        ZSTD_cwksp_move(&mut (*cdict).workspace, &mut ws);
        (*cdict).customMem = customMem;
        (*cdict).compressionLevel = ZSTD_NO_CLEVEL; /* signals advanced API usage */
        (*cdict).useRowMatchFinder = useRowMatchFinder;
        cdict
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
    ZSTD_memset(
        &mut cctxParams as *mut ZSTD_CCtx_params as *mut c_void,
        0,
        core::mem::size_of::<ZSTD_CCtx_params>(),
    );
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
    let mut cctxParams: ZSTD_CCtx_params = *originalCctxParams;
    let mut cParams: ZSTD_compressionParameters;
    let cdict: *mut ZSTD_CDict;

    if ((customMem.customAlloc.is_none()) as c_int) ^ ((customMem.customFree.is_none()) as c_int)
        != 0
    {
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
        /* Fall back to non-DDSS params */
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
        || ERR_isError(ZSTD_initCDict_internal(
            cdict,
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
    let cParams: ZSTD_compressionParameters = ZSTD_getCParams_internal(
        compressionLevel,
        ZSTD_CONTENTSIZE_UNKNOWN,
        dictSize,
        ZSTD_cpm_createCDict,
    );
    let cdict: *mut ZSTD_CDict = ZSTD_createCDict_advanced(
        dict,
        dictSize,
        ZSTD_dlm_byCopy,
        ZSTD_dct_auto,
        cParams,
        ZSTD_defaultCMem,
    );
    if !cdict.is_null() {
        (*cdict).compressionLevel = if compressionLevel == 0 {
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
    let cParams: ZSTD_compressionParameters = ZSTD_getCParams_internal(
        compressionLevel,
        ZSTD_CONTENTSIZE_UNKNOWN,
        dictSize,
        ZSTD_cpm_createCDict,
    );
    let cdict: *mut ZSTD_CDict = ZSTD_createCDict_advanced(
        dict,
        dictSize,
        ZSTD_dlm_byRef,
        ZSTD_dct_auto,
        cParams,
        ZSTD_defaultCMem,
    );
    if !cdict.is_null() {
        (*cdict).compressionLevel = if compressionLevel == 0 {
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
        return 0; /* support free on NULL */
    }
    {
        let cMem: ZSTD_customMem = (*cdict).customMem;
        let cdictInWorkspace: c_int =
            ZSTD_cwksp_owns_buffer(&(*cdict).workspace, cdict as *const c_void);
        ZSTD_cwksp_free(&mut (*cdict).workspace, cMem);
        if cdictInWorkspace == 0 {
            ZSTD_customFree(cdict as *mut c_void, cMem);
        }
        0
    }
}

/* ZSTD_initStaticCDict_advanced() :
 *  Generate a digested dictionary in provided memory area.
 *  workspace: The memory area to emplace the dictionary into.
 *             Provided pointer must 8-bytes aligned.
 *             It must outlive dictionary usage.
 *  workspaceSize: Use ZSTD_estimateCDictSize()
 *                 to determine how large workspace must be.
 *  cParams : use ZSTD_getCParams() to transform a compression level
 *            into its relevant cParams.
 * @return : pointer to ZSTD_CDict*, or NULL if error (size too small)
 *  Note : there is no corresponding "free" function.
 *         Since workspace was allocated externally, it must be freed externally.
 */
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
    let useRowMatchFinder: ZSTD_ParamSwitch_e =
        ZSTD_resolveRowMatchFinderMode(ZSTD_ps_auto, &cParams);
    /* enableDedicatedDictSearch == 1 ensures matchstate is not too small in case this CDict will be used for DDS + row hash */
    let matchStateSize: usize = ZSTD_sizeof_matchState(
        &cParams,
        useRowMatchFinder,
        1, /* enableDedicatedDictSearch */
        0, /* forCCtx */
    );
    let neededSize: usize = ZSTD_cwksp_alloc_size(core::mem::size_of::<ZSTD_CDict>())
        + (if dictLoadMethod == ZSTD_dlm_byRef {
            0
        } else {
            ZSTD_cwksp_alloc_size(ZSTD_cwksp_align(
                dictSize,
                core::mem::size_of::<*mut c_void>(),
            ))
        })
        + ZSTD_cwksp_alloc_size(HUF_WORKSPACE_SIZE)
        + matchStateSize;
    let cdict: *mut ZSTD_CDict;
    let mut params: ZSTD_CCtx_params = core::mem::zeroed();

    if (workspace as usize) & 7 != 0 {
        return core::ptr::null(); /* 8-aligned */
    }

    {
        let mut ws: ZSTD_cwksp = core::mem::zeroed();
        ZSTD_cwksp_init(&mut ws, workspace, workspaceSize, ZSTD_cwksp_static_alloc);
        cdict = ZSTD_cwksp_reserve_object(&mut ws, core::mem::size_of::<ZSTD_CDict>())
            as *mut ZSTD_CDict;
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

    if ERR_isError(ZSTD_initCDict_internal(
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

    cdict
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_getCParamsFromCDict(
    cdict: *const ZSTD_CDict,
) -> ZSTD_compressionParameters {
    (*cdict).matchState.cParams
}

/* ZSTD_getDictID_fromCDict() :
 *  Provides the dictID of the dictionary loaded into `cdict`.
 *  If @return == 0, the dictionary is not conformant to Zstandard specification, or empty.
 *  Non-conformant dictionaries can still be loaded, but as content-only dictionaries. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_getDictID_fromCDict(cdict: *const ZSTD_CDict) -> c_uint {
    if cdict.is_null() {
        return 0;
    }
    (*cdict).dictID
}

/* ZSTD_compressBegin_usingCDict_internal() :
 * Implementation of various ZSTD_compressBegin_usingCDict* functions.
 */
unsafe fn ZSTD_compressBegin_usingCDict_internal(
    cctx: *mut ZSTD_CCtx,
    cdict: *const ZSTD_CDict,
    fParams: ZSTD_frameParameters,
    pledgedSrcSize: u64,
) -> usize {
    let mut cctxParams: ZSTD_CCtx_params = core::mem::zeroed();
    if cdict.is_null() {
        return ERROR(ZSTD_error_dictionary_wrong);
    }
    /* Initialize the cctxParams from the cdict */
    {
        let mut params: ZSTD_parameters = core::mem::zeroed();
        params.fParams = fParams;
        params.cParams = if pledgedSrcSize < ZSTD_USE_CDICT_PARAMS_SRCSIZE_CUTOFF
            || pledgedSrcSize
                < ((*cdict).dictContentSize as U64)
                    .wrapping_mul(ZSTD_USE_CDICT_PARAMS_DICTSIZE_MULTIPLIER)
            || pledgedSrcSize == ZSTD_CONTENTSIZE_UNKNOWN
            || (*cdict).compressionLevel == 0
        {
            ZSTD_getCParamsFromCDict(cdict)
        } else {
            ZSTD_getCParams(
                (*cdict).compressionLevel,
                pledgedSrcSize,
                (*cdict).dictContentSize,
            )
        };
        ZSTD_CCtxParams_init_internal(&mut cctxParams, &params, (*cdict).compressionLevel);
    }
    /* Increase window log to fit the entire dictionary and source if the
     * source size is known. Limit the increase to 19, which is the
     * window log for compression level 1 with the largest source size.
     */
    if pledgedSrcSize != ZSTD_CONTENTSIZE_UNKNOWN {
        let limitedSrcSize: U32 = MIN(pledgedSrcSize, (1u32 << 19) as U64) as U32;
        let limitedSrcLog: U32 = if limitedSrcSize > 1 {
            ZSTD_highbit32(limitedSrcSize - 1) + 1
        } else {
            1
        };
        cctxParams.cParams.windowLog = MAX(cctxParams.cParams.windowLog, limitedSrcLog);
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

/* ZSTD_compressBegin_usingCDict_advanced() :
 * This function is DEPRECATED.
 * cdict must be != NULL */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBegin_usingCDict_advanced(
    cctx: *mut ZSTD_CCtx,
    cdict: *const ZSTD_CDict,
    fParams: ZSTD_frameParameters,
    pledgedSrcSize: u64,
) -> usize {
    ZSTD_compressBegin_usingCDict_internal(cctx, cdict, fParams, pledgedSrcSize)
}

/* ZSTD_compressBegin_usingCDict() :
 * cdict must be != NULL */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBegin_usingCDict_deprecated(
    cctx: *mut ZSTD_CCtx,
    cdict: *const ZSTD_CDict,
) -> usize {
    let fParams: ZSTD_frameParameters = ZSTD_frameParameters {
        contentSizeFlag: 0, /*content*/
        checksumFlag: 0,    /*checksum*/
        noDictIDFlag: 0,    /*noDictID*/
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

/* ZSTD_compress_usingCDict_internal():
 * Implementation of various ZSTD_compress_usingCDict* functions.
 */
unsafe fn ZSTD_compress_usingCDict_internal(
    cctx: *mut ZSTD_CCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
    cdict: *const ZSTD_CDict,
    fParams: ZSTD_frameParameters,
) -> usize {
    {
        /* will check if cdict != NULL */
        let err_code =
            ZSTD_compressBegin_usingCDict_internal(cctx, cdict, fParams, srcSize as U64);
        if ERR_isError(err_code) != 0 {
            return err_code;
        }
    }
    ZSTD_compressEnd_public(cctx, dst, dstCapacity, src, srcSize)
}

/* ZSTD_compress_usingCDict_advanced():
 * This function is DEPRECATED.
 */
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

/* ZSTD_compress_usingCDict() :
 *  Compression using a digested Dictionary.
 *  Faster startup than ZSTD_compress_usingDict(), recommended when same dictionary is used multiple times.
 *  Note that compression parameters are decided at CDict creation time
 *  while frame parameters are hardcoded */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compress_usingCDict(
    cctx: *mut ZSTD_CCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
    cdict: *const ZSTD_CDict,
) -> usize {
    let fParams: ZSTD_frameParameters = ZSTD_frameParameters {
        contentSizeFlag: 1, /*content*/
        checksumFlag: 0,    /*checksum*/
        noDictIDFlag: 0,    /*noDictID*/
    };
    ZSTD_compress_usingCDict_internal(cctx, dst, dstCapacity, src, srcSize, cdict, fParams)
}
