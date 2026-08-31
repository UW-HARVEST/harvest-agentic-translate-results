//! Translation of `compress/zstd_compress.c`, part 4:
//! lines 4902..6086 of the C file — dictionary loading, `ZSTD_compressBegin*`,
//! `ZSTD_compressEnd*`, the simple compression API, the CDict API and the
//! CStream initialisation functions.
#![allow(dead_code)]

use core::ffi::{c_char, c_int, c_uint, c_void};

use crate::bits::*;
use crate::cmem::*;
use crate::compress::zstd_compress_internal::*;
use crate::compress::zstd_cwksp::*;
use crate::error_private::*;
use crate::fse::*;
use crate::huf::*;
use crate::xxhash::XXH64_digest;
use crate::zstd_h::*;
use crate::zstd_internal::*;
use crate::zstd_trace::*;

/* ZSTD_loadDictionaryContent() :
 *  @return : 0, or an error code
 */
pub unsafe fn ZSTD_loadDictionaryContent(
    ms: *mut ZSTD_MatchState_t,
    ls: *mut ldmState_t,
    ws: *mut ZSTD_cwksp,
    params: *const ZSTD_CCtx_params,
    mut src: *const c_void,
    mut srcSize: usize,
    dtlm: ZSTD_dictTableLoadMethod_e,
    tfp: ZSTD_tableFillPurpose_e,
) -> usize {
    let mut ip = src as *const BYTE;
    let iend = ip.add(srcSize);
    let loadLdmDict: c_int =
        ((*params).ldmParams.enableLdm == ZSTD_ps_enable && !ls.is_null()) as c_int;

    /* Assert that the ms params match the params we're being given
     * (ZSTD_assertEqualCParams is assert-only: no-op at DEBUGLEVEL 0) */

    {
        /* Ensure large dictionaries can't cause index overflow */

        /* Allow the dictionary to set indices up to exactly ZSTD_CURRENT_MAX.
         * Dictionaries right at the edge will immediately trigger overflow
         * correction, but I don't want to insert extra constraints here.
         */
        let mut maxDictSize: U32 = ZSTD_CURRENT_MAX().wrapping_sub(ZSTD_WINDOW_START_INDEX);

        let CDictTaggedIndices: c_int =
            crate::compress::zstd_compress::ZSTD_CDictIndicesAreTagged(&(*params).cParams);
        if CDictTaggedIndices != 0 && tfp == ZSTD_tfp_forCDict {
            /* Some dictionary matchfinders in zstd use "short cache",
             * which treats the lower ZSTD_SHORT_CACHE_TAG_BITS of each
             * CDict hashtable entry as a tag rather than as part of an index.
             * When short cache is used, we need to truncate the dictionary
             * so that its indices don't overlap with the tag. */
            let shortCacheMaxDictSize: U32 = (1u32 << (32 - ZSTD_SHORT_CACHE_TAG_BITS))
                .wrapping_sub(ZSTD_WINDOW_START_INDEX);
            maxDictSize = if maxDictSize < shortCacheMaxDictSize {
                maxDictSize
            } else {
                shortCacheMaxDictSize
            };
        }

        /* If the dictionary is too large, only load the suffix of the dictionary. */
        if srcSize > maxDictSize as usize {
            ip = iend.sub(maxDictSize as usize);
            src = ip as *const c_void;
            srcSize = maxDictSize as usize;
        }
    }

    /* (the `srcSize > ZSTD_CHUNKSIZE_MAX` block only contained asserts) */

    ZSTD_window_update(&mut (*ms).window, src, srcSize, /* forceNonContiguous */ 0);

    if loadLdmDict != 0 {
        /* Load the entire dict into LDM matchfinders. */
        ZSTD_window_update(&mut (*ls).window, src, srcSize, /* forceNonContiguous */ 0);
        (*ls).loadedDictEnd = if (*params).forceWindow != 0 {
            0
        } else {
            iend.offset_from((*ls).window.base) as U32
        };
        crate::compress::zstd_ldm::ZSTD_ldm_fillHashTable(ls, ip, iend, &(*params).ldmParams);
    }

    /* If the dict is larger than we can reasonably index in our tables, only load the suffix. */
    {
        let a: U32 = (*params).cParams.hashLog.wrapping_add(3);
        let b: U32 = (*params).cParams.chainLog.wrapping_add(1);
        let m: U32 = if a > b { a } else { b };
        let shift: U32 = if m < 31 { m } else { 31 };
        let maxDictSize: U32 = 1u32 << shift;
        if srcSize > maxDictSize as usize {
            ip = iend.sub(maxDictSize as usize);
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

    crate::compress::zstd_compress::ZSTD_overflowCorrectIfNeeded(
        ms,
        ws,
        params,
        ip as *const c_void,
        iend as *const c_void,
    );

    match (*params).cParams.strategy {
        ZSTD_fast => {
            crate::compress::zstd_fast::ZSTD_fillHashTable(ms, iend as *const c_void, dtlm, tfp);
        }
        ZSTD_dfast => {
            crate::compress::zstd_double_fast::ZSTD_fillDoubleHashTable(
                ms,
                iend as *const c_void,
                dtlm,
                tfp,
            );
        }

        ZSTD_greedy | ZSTD_lazy | ZSTD_lazy2 => {
            if (*ms).dedicatedDictSearch != 0 {
                crate::compress::zstd_lazy::ZSTD_dedicatedDictSearch_lazy_loadDictionary(
                    ms,
                    iend.sub(HASH_READ_SIZE),
                );
            } else {
                if (*params).useRowMatchFinder == ZSTD_ps_enable {
                    let tagTableSize: usize = 1usize << (*params).cParams.hashLog;
                    ZSTD_memset((*ms).tagTable as *mut c_void, 0, tagTableSize);
                    crate::compress::zstd_lazy::ZSTD_row_update(ms, iend.sub(HASH_READ_SIZE));
                } else {
                    crate::compress::zstd_lazy::ZSTD_insertAndFindFirstIndex(
                        ms,
                        iend.sub(HASH_READ_SIZE),
                    );
                }
            }
        }

        /* we want the dictionary table fully sorted */
        ZSTD_btlazy2 | ZSTD_btopt | ZSTD_btultra | ZSTD_btultra2 => {
            crate::compress::zstd_opt::ZSTD_updateTree(ms, iend.sub(HASH_READ_SIZE), iend);
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
pub unsafe fn ZSTD_dictNCountRepeat(
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
    /* skip magic num and dict ID */
    let mut dictPtr = dict as *const BYTE;
    let dictEnd = dictPtr.add(dictSize);
    dictPtr = dictPtr.add(8);
    (*bs).entropy.huf.repeatMode = HUF_repeat_check;

    {
        let mut maxSymbolValue: c_uint = 255;
        let mut hasZeroWeights: c_uint = 1;
        let hufHeaderSize = crate::compress::huf_compress::HUF_readCTable(
            (*bs).entropy.huf.CTable.as_mut_ptr() as *mut HUF_CElt,
            &mut maxSymbolValue,
            dictPtr as *const c_void,
            (dictEnd as usize) - (dictPtr as usize),
            &mut hasZeroWeights,
        );

        /* We only set the loaded table as valid if it contains all non-zero
         * weights. Otherwise, we set it to check */
        if hasZeroWeights == 0 && maxSymbolValue == 255 {
            (*bs).entropy.huf.repeatMode = HUF_repeat_valid;
        }

        if crate::entropy_common::HUF_isError(hufHeaderSize) != 0 {
            return ERROR(ZSTD_error_dictionary_corrupted);
        }
        dictPtr = dictPtr.add(hufHeaderSize);
    }

    {
        let mut offcodeLog: c_uint = 0;
        let offcodeHeaderSize = crate::entropy_common::FSE_readNCount(
            offcodeNCount.as_mut_ptr(),
            &mut offcodeMaxValue,
            &mut offcodeLog,
            dictPtr as *const c_void,
            (dictEnd as usize) - (dictPtr as usize),
        );
        if crate::entropy_common::FSE_isError(offcodeHeaderSize) != 0 {
            return ERROR(ZSTD_error_dictionary_corrupted);
        }
        if offcodeLog > OffFSELog {
            return ERROR(ZSTD_error_dictionary_corrupted);
        }
        /* fill all offset symbols to avoid garbage at end of table */
        if crate::entropy_common::FSE_isError(crate::compress::fse_compress::FSE_buildCTable_wksp(
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
        dictPtr = dictPtr.add(offcodeHeaderSize);
    }

    {
        let mut matchlengthNCount: [i16; (MaxML + 1) as usize] = [0; (MaxML + 1) as usize];
        let mut matchlengthMaxValue: c_uint = MaxML;
        let mut matchlengthLog: c_uint = 0;
        let matchlengthHeaderSize = crate::entropy_common::FSE_readNCount(
            matchlengthNCount.as_mut_ptr(),
            &mut matchlengthMaxValue,
            &mut matchlengthLog,
            dictPtr as *const c_void,
            (dictEnd as usize) - (dictPtr as usize),
        );
        if crate::entropy_common::FSE_isError(matchlengthHeaderSize) != 0 {
            return ERROR(ZSTD_error_dictionary_corrupted);
        }
        if matchlengthLog > MLFSELog {
            return ERROR(ZSTD_error_dictionary_corrupted);
        }
        if crate::entropy_common::FSE_isError(crate::compress::fse_compress::FSE_buildCTable_wksp(
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
        dictPtr = dictPtr.add(matchlengthHeaderSize);
    }

    {
        let mut litlengthNCount: [i16; (MaxLL + 1) as usize] = [0; (MaxLL + 1) as usize];
        let mut litlengthMaxValue: c_uint = MaxLL;
        let mut litlengthLog: c_uint = 0;
        let litlengthHeaderSize = crate::entropy_common::FSE_readNCount(
            litlengthNCount.as_mut_ptr(),
            &mut litlengthMaxValue,
            &mut litlengthLog,
            dictPtr as *const c_void,
            (dictEnd as usize) - (dictPtr as usize),
        );
        if crate::entropy_common::FSE_isError(litlengthHeaderSize) != 0 {
            return ERROR(ZSTD_error_dictionary_corrupted);
        }
        if litlengthLog > LLFSELog {
            return ERROR(ZSTD_error_dictionary_corrupted);
        }
        if crate::entropy_common::FSE_isError(crate::compress::fse_compress::FSE_buildCTable_wksp(
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
        dictPtr = dictPtr.add(litlengthHeaderSize);
    }

    if dictPtr.add(12) > dictEnd {
        return ERROR(ZSTD_error_dictionary_corrupted);
    }
    (*bs).rep[0] = MEM_readLE32(dictPtr.add(0) as *const c_void);
    (*bs).rep[1] = MEM_readLE32(dictPtr.add(4) as *const c_void);
    (*bs).rep[2] = MEM_readLE32(dictPtr.add(8) as *const c_void);
    dictPtr = dictPtr.add(12);

    {
        let dictContentSize: usize = (dictEnd as usize) - (dictPtr as usize);
        let mut offcodeMax: U32 = MaxOff;
        if dictContentSize <= (u32::MAX as usize) - (128 * 1024) {
            /* The maximum offset that must be supported */
            let maxOffset: U32 = (dictContentSize as U32).wrapping_add(128 * 1024);
            /* Calculate minimum offset code required to represent maxOffset */
            offcodeMax = ZSTD_highbit32(maxOffset);
        }
        /* All offset values <= dictContentSize + 128 KB must be representable for a valid table */
        (*bs).entropy.fse.offcode_repeatMode = ZSTD_dictNCountRepeat(
            offcodeNCount.as_mut_ptr(),
            offcodeMaxValue,
            if offcodeMax < MaxOff { offcodeMax } else { MaxOff },
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

    (dictPtr as usize) - (dict as *const BYTE as usize)
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
pub unsafe fn ZSTD_loadZstdDictionary(
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
    let mut dictPtr = dict as *const BYTE;
    let dictEnd = dictPtr.add(dictSize);
    let dictID: usize;
    let eSize: usize;

    dictID = if (*params).fParams.noDictIDFlag != 0 {
        0
    } else {
        /* skip magic number */
        MEM_readLE32(dictPtr.add(4) as *const c_void) as usize
    };
    eSize = ZSTD_loadCEntropy(bs, workspace, dict, dictSize);
    if ERR_isError(eSize) != 0 {
        return eSize;
    }
    dictPtr = dictPtr.add(eSize);

    {
        let dictContentSize: usize = (dictEnd as usize) - (dictPtr as usize);
        let e = ZSTD_loadDictionaryContent(
            ms,
            core::ptr::null_mut(),
            ws,
            params,
            dictPtr as *const c_void,
            dictContentSize,
            dtlm,
            tfp,
        );
        if ERR_isError(e) != 0 {
            return e;
        }
    }
    dictID
}

/** ZSTD_compress_insertDictionary() :
 *   @return : dictID, or an error code */
pub unsafe fn ZSTD_compress_insertDictionary(
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

    crate::compress::zstd_compress::ZSTD_reset_compressedBlockState(bs);

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
        /* assert(0); impossible */
    }

    /* dict as full zstd dictionary */
    ZSTD_loadZstdDictionary(bs, ms, ws, params, dict, dictSize, dtlm, tfp, workspace)
}

pub const ZSTD_USE_CDICT_PARAMS_SRCSIZE_CUTOFF: u64 = 128 * 1024;
pub const ZSTD_USE_CDICT_PARAMS_DICTSIZE_MULTIPLIER: u64 = 6;

/* ZSTD_compressBegin_internal() :
 * Assumption : either @dict OR @cdict (or none) is non-NULL, never both
 * @return : 0, or an error code */
pub unsafe fn ZSTD_compressBegin_internal(
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
    /* ZSTD_TRACE == 1, but the hook is always NULL in this build. */
    (*cctx).traceCtx = if let Some(f) = ZSTD_trace_compress_begin {
        f(cctx)
    } else {
        0
    };
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
        return crate::compress::zstd_compress::ZSTD_resetCCtx_usingCDict(
            cctx,
            cdict,
            params,
            pledgedSrcSize,
            zbuff,
        );
    }

    {
        let e = crate::compress::zstd_compress::ZSTD_resetCCtx_internal(
            cctx,
            params,
            pledgedSrcSize,
            dictContentSize,
            crate::compress::zstd_compress::ZSTDcrp_makeClean,
            zbuff,
        );
        if ERR_isError(e) != 0 {
            return e;
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
        let e = crate::compress::zstd_compress::ZSTD_checkCParams((*params).cParams);
        if ERR_isError(e) != 0 {
            return e;
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
    let mut cctxParams = ZSTD_CCtx_params::default();
    crate::compress::zstd_compress::ZSTD_CCtxParams_init_internal(
        &mut cctxParams,
        &params,
        crate::compress::zstd_compress::ZSTD_NO_CLEVEL,
    );
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

pub unsafe fn ZSTD_compressBegin_usingDict_deprecated(
    cctx: *mut ZSTD_CCtx,
    dict: *const c_void,
    dictSize: usize,
    compressionLevel: c_int,
) -> usize {
    let mut cctxParams = ZSTD_CCtx_params::default();
    {
        let params: ZSTD_parameters = crate::compress::zstd_compress::ZSTD_getParams_internal(
            compressionLevel,
            ZSTD_CONTENTSIZE_UNKNOWN,
            dictSize,
            ZSTD_cpm_noAttachDict,
        );
        crate::compress::zstd_compress::ZSTD_CCtxParams_init_internal(
            &mut cctxParams,
            &params,
            if compressionLevel == 0 {
                crate::compress::zstd_compress::ZSTD_CLEVEL_DEFAULT
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
pub unsafe fn ZSTD_writeEpilogue(
    cctx: *mut ZSTD_CCtx,
    dst: *mut c_void,
    mut dstCapacity: usize,
) -> usize {
    let ostart = dst as *mut BYTE;
    let mut op = ostart;

    if (*cctx).stage == ZSTDcs_created {
        return ERROR(ZSTD_error_stage_wrong);
    }

    /* special case : empty frame */
    if (*cctx).stage == ZSTDcs_init {
        let fhSize = crate::compress::zstd_compress::ZSTD_writeFrameHeader(
            dst,
            dstCapacity,
            &(*cctx).appliedParams,
            0,
            0,
        );
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
        let checksum: U32 = XXH64_digest(&(*cctx).xxhState) as U32;
        if dstCapacity < 4 {
            return ERROR(ZSTD_error_dstSize_tooSmall);
        }
        MEM_writeLE32(op as *mut c_void, checksum);
        op = op.add(4);
    }

    (*cctx).stage = ZSTDcs_created; /* return to "created but no init" status */
    (op as usize) - (ostart as usize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_CCtx_trace(cctx: *mut ZSTD_CCtx, extraCSize: usize) {
    /* ZSTD_TRACE == 1, but `ZSTD_trace_compress_end` is always NULL here. */
    if (*cctx).traceCtx != 0 {
        if let Some(f) = ZSTD_trace_compress_end {
            let streaming: c_int = ((*cctx).inBuffSize > 0
                || (*cctx).outBuffSize > 0
                || (*cctx).appliedParams.nbWorkers > 0) as c_int;
            let mut trace = core::mem::MaybeUninit::<ZSTD_Trace>::uninit();
            let trace = trace.as_mut_ptr();
            ZSTD_memset(
                trace as *mut c_void,
                0,
                core::mem::size_of::<ZSTD_Trace>(),
            );
            (*trace).version = ZSTD_VERSION_NUMBER;
            (*trace).streaming = streaming;
            (*trace).dictionaryID = (*cctx).dictID;
            (*trace).dictionarySize = (*cctx).dictContentSize;
            (*trace).uncompressedSize = (*cctx).consumedSrcSize as usize;
            (*trace).compressedSize = ((*cctx).producedCSize as usize).wrapping_add(extraCSize);
            (*trace).params = &(*cctx).appliedParams;
            (*trace).cctx = cctx;
            f((*cctx).traceCtx, trace);
        }
    }
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
    let cSize = crate::compress::zstd_compress::ZSTD_compressContinue_internal(
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
        let e = crate::compress::zstd_compress::ZSTD_checkCParams(params.cParams);
        if ERR_isError(e) != 0 {
            return e;
        }
    }
    crate::compress::zstd_compress::ZSTD_CCtxParams_init_internal(
        &mut (*cctx).simpleApiParams,
        &params,
        crate::compress::zstd_compress::ZSTD_NO_CLEVEL,
    );
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
        let e = ZSTD_compressBegin_internal(
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
        if ERR_isError(e) != 0 {
            return e;
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
        let params: ZSTD_parameters = crate::compress::zstd_compress::ZSTD_getParams_internal(
            compressionLevel,
            srcSize as u64,
            if !dict.is_null() { dictSize } else { 0 },
            ZSTD_cpm_noAttachDict,
        );
        crate::compress::zstd_compress::ZSTD_CCtxParams_init_internal(
            &mut (*cctx).simpleApiParams,
            &params,
            if compressionLevel == 0 {
                crate::compress::zstd_compress::ZSTD_CLEVEL_DEFAULT
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
    /* ZSTD_COMPRESS_HEAPMODE == 0 */
    let mut ctxBody = core::mem::MaybeUninit::<ZSTD_CCtx>::uninit();
    let ctxBody = ctxBody.as_mut_ptr();
    crate::compress::zstd_compress::ZSTD_initCCtx(
        ctxBody,
        crate::compress::zstd_compress::ZSTD_defaultCMem,
    );
    result = ZSTD_compressCCtx(ctxBody, dst, dstCapacity, src, srcSize, compressionLevel);
    /* can't free ctxBody itself, as it's on stack; free only heap content */
    crate::compress::zstd_compress::ZSTD_freeCCtxContent(ctxBody);
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
        + crate::compress::zstd_compress::ZSTD_sizeof_matchState(
            &cParams,
            crate::compress::zstd_compress::ZSTD_resolveRowMatchFinderMode(ZSTD_ps_auto, &cParams),
            /* enableDedicatedDictSearch */ 1,
            /* forCCtx */ 0,
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
    let cParams: ZSTD_compressionParameters =
        crate::compress::zstd_compress::ZSTD_getCParams_internal(
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

pub unsafe fn ZSTD_initCDict_internal(
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
        let internalBuffer = ZSTD_cwksp_reserve_object(
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
    crate::compress::zstd_compress::ZSTD_reset_compressedBlockState(&mut (*cdict).cBlockState);
    {
        let e = crate::compress::zstd_compress::ZSTD_reset_matchState(
            &mut (*cdict).matchState,
            &mut (*cdict).workspace,
            &params.cParams,
            params.useRowMatchFinder,
            crate::compress::zstd_compress::ZSTDcrp_makeClean,
            crate::compress::zstd_compress::ZSTDirp_reset,
            crate::compress::zstd_compress::ZSTD_resetTarget_CDict,
        );
        if ERR_isError(e) != 0 {
            return e;
        }
    }
    /* (Maybe) load the dictionary
     * Skips loading the dictionary if it is < 8 bytes.
     */
    {
        params.compressionLevel = crate::compress::zstd_compress::ZSTD_CLEVEL_DEFAULT;
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

pub unsafe fn ZSTD_createCDict_advanced_internal(
    dictSize: usize,
    dictLoadMethod: ZSTD_dictLoadMethod_e,
    cParams: ZSTD_compressionParameters,
    useRowMatchFinder: ZSTD_ParamSwitch_e,
    enableDedicatedDictSearch: c_int,
    customMem: ZSTD_customMem,
) -> *mut ZSTD_CDict {
    if ((customMem.customAlloc.is_none() as c_int) ^ (customMem.customFree.is_none() as c_int)) != 0
    {
        return core::ptr::null_mut();
    }

    {
        let workspaceSize: usize = ZSTD_cwksp_alloc_size(core::mem::size_of::<ZSTD_CDict>())
            + ZSTD_cwksp_alloc_size(HUF_WORKSPACE_SIZE)
            + crate::compress::zstd_compress::ZSTD_sizeof_matchState(
                &cParams,
                useRowMatchFinder,
                enableDedicatedDictSearch,
                /* forCCtx */ 0,
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
        let mut ws = ZSTD_cwksp::default();
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
        /* signals advanced API usage */
        (*cdict).compressionLevel = crate::compress::zstd_compress::ZSTD_NO_CLEVEL;
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
    let mut cctxParams = ZSTD_CCtx_params::default();
    ZSTD_memset(
        &mut cctxParams as *mut ZSTD_CCtx_params as *mut c_void,
        0,
        core::mem::size_of::<ZSTD_CCtx_params>(),
    );
    crate::compress::zstd_compress::ZSTD_CCtxParams_init(&mut cctxParams, 0);
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

    if ((customMem.customAlloc.is_none() as c_int) ^ (customMem.customFree.is_none() as c_int)) != 0
    {
        return core::ptr::null_mut();
    }

    if cctxParams.enableDedicatedDictSearch != 0 {
        cParams = crate::compress::zstd_compress::ZSTD_dedicatedDictSearch_getCParams(
            cctxParams.compressionLevel,
            dictSize,
        );
        crate::compress::zstd_compress::ZSTD_overrideCParams(&mut cParams, &cctxParams.cParams);
    } else {
        cParams = crate::compress::zstd_compress::ZSTD_getCParamsFromCCtxParams(
            &cctxParams,
            ZSTD_CONTENTSIZE_UNKNOWN,
            dictSize,
            ZSTD_cpm_createCDict,
        );
    }

    if crate::compress::zstd_compress::ZSTD_dedicatedDictSearch_isSupported(&cParams) == 0 {
        /* Fall back to non-DDSS params */
        cctxParams.enableDedicatedDictSearch = 0;
        cParams = crate::compress::zstd_compress::ZSTD_getCParamsFromCCtxParams(
            &cctxParams,
            ZSTD_CONTENTSIZE_UNKNOWN,
            dictSize,
            ZSTD_cpm_createCDict,
        );
    }

    cctxParams.cParams = cParams;
    cctxParams.useRowMatchFinder = crate::compress::zstd_compress::ZSTD_resolveRowMatchFinderMode(
        cctxParams.useRowMatchFinder,
        &cParams,
    );

    cdict = ZSTD_createCDict_advanced_internal(
        dictSize,
        dictLoadMethod,
        cctxParams.cParams,
        cctxParams.useRowMatchFinder,
        cctxParams.enableDedicatedDictSearch,
        customMem,
    );

    if cdict.is_null()
        || crate::zstd_common::ZSTD_isError(ZSTD_initCDict_internal(
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
    let cParams: ZSTD_compressionParameters =
        crate::compress::zstd_compress::ZSTD_getCParams_internal(
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
        crate::compress::zstd_compress::ZSTD_defaultCMem,
    );
    if !cdict.is_null() {
        (*cdict).compressionLevel = if compressionLevel == 0 {
            crate::compress::zstd_compress::ZSTD_CLEVEL_DEFAULT
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
    let cParams: ZSTD_compressionParameters =
        crate::compress::zstd_compress::ZSTD_getCParams_internal(
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
        crate::compress::zstd_compress::ZSTD_defaultCMem,
    );
    if !cdict.is_null() {
        (*cdict).compressionLevel = if compressionLevel == 0 {
            crate::compress::zstd_compress::ZSTD_CLEVEL_DEFAULT
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
        let cdictInWorkspace =
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
        crate::compress::zstd_compress::ZSTD_resolveRowMatchFinderMode(ZSTD_ps_auto, &cParams);
    /* enableDedicatedDictSearch == 1 ensures matchstate is not too small in case this CDict will be used for DDS + row hash */
    let matchStateSize: usize = crate::compress::zstd_compress::ZSTD_sizeof_matchState(
        &cParams,
        useRowMatchFinder,
        /* enableDedicatedDictSearch */ 1,
        /* forCCtx */ 0,
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
    let mut params = ZSTD_CCtx_params::default();

    if (workspace as usize) & 7 != 0 {
        return core::ptr::null(); /* 8-aligned */
    }

    {
        let mut ws = ZSTD_cwksp::default();
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

    crate::compress::zstd_compress::ZSTD_CCtxParams_init(&mut params, 0);
    params.cParams = cParams;
    params.useRowMatchFinder = useRowMatchFinder;
    (*cdict).useRowMatchFinder = useRowMatchFinder;
    (*cdict).compressionLevel = crate::compress::zstd_compress::ZSTD_NO_CLEVEL;

    if crate::zstd_common::ZSTD_isError(ZSTD_initCDict_internal(
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
pub unsafe fn ZSTD_compressBegin_usingCDict_internal(
    cctx: *mut ZSTD_CCtx,
    cdict: *const ZSTD_CDict,
    fParams: ZSTD_frameParameters,
    pledgedSrcSize: u64,
) -> usize {
    let mut cctxParams = ZSTD_CCtx_params::default();
    if cdict.is_null() {
        return ERROR(ZSTD_error_dictionary_wrong);
    }
    /* Initialize the cctxParams from the cdict */
    {
        let mut params = ZSTD_parameters::default();
        params.fParams = fParams;
        params.cParams = if pledgedSrcSize < ZSTD_USE_CDICT_PARAMS_SRCSIZE_CUTOFF
            || pledgedSrcSize
                < ((*cdict).dictContentSize as u64)
                    .wrapping_mul(ZSTD_USE_CDICT_PARAMS_DICTSIZE_MULTIPLIER)
            || pledgedSrcSize == ZSTD_CONTENTSIZE_UNKNOWN
            || (*cdict).compressionLevel == 0
        {
            ZSTD_getCParamsFromCDict(cdict)
        } else {
            crate::compress::zstd_compress::ZSTD_getCParams(
                (*cdict).compressionLevel,
                pledgedSrcSize,
                (*cdict).dictContentSize,
            )
        };
        crate::compress::zstd_compress::ZSTD_CCtxParams_init_internal(
            &mut cctxParams,
            &params,
            (*cdict).compressionLevel,
        );
    }
    /* Increase window log to fit the entire dictionary and source if the
     * source size is known. Limit the increase to 19, which is the
     * window log for compression level 1 with the largest source size.
     */
    if pledgedSrcSize != ZSTD_CONTENTSIZE_UNKNOWN {
        let limit: u64 = 1u64 << 19;
        let limitedSrcSize: U32 = (if pledgedSrcSize < limit {
            pledgedSrcSize
        } else {
            limit
        }) as U32;
        let limitedSrcLog: U32 = if limitedSrcSize > 1 {
            ZSTD_highbit32(limitedSrcSize - 1) + 1
        } else {
            1
        };
        cctxParams.cParams.windowLog = if cctxParams.cParams.windowLog > limitedSrcLog {
            cctxParams.cParams.windowLog
        } else {
            limitedSrcLog
        };
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
    let fParams = ZSTD_frameParameters {
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
pub unsafe fn ZSTD_compress_usingCDict_internal(
    cctx: *mut ZSTD_CCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
    cdict: *const ZSTD_CDict,
    fParams: ZSTD_frameParameters,
) -> usize {
    /* will check if cdict != NULL */
    let e = ZSTD_compressBegin_usingCDict_internal(cctx, cdict, fParams, srcSize as u64);
    if ERR_isError(e) != 0 {
        return e;
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
    let fParams = ZSTD_frameParameters {
        contentSizeFlag: 1, /*content*/
        checksumFlag: 0,    /*checksum*/
        noDictIDFlag: 0,    /*noDictID*/
    };
    ZSTD_compress_usingCDict_internal(cctx, dst, dstCapacity, src, srcSize, cdict, fParams)
}

/* ******************************************************************
 *  Streaming
 ********************************************************************/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_createCStream() -> *mut ZSTD_CStream {
    ZSTD_createCStream_advanced(crate::compress::zstd_compress::ZSTD_defaultCMem)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_initStaticCStream(
    workspace: *mut c_void,
    workspaceSize: usize,
) -> *mut ZSTD_CStream {
    crate::compress::zstd_compress::ZSTD_initStaticCCtx(workspace, workspaceSize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_createCStream_advanced(
    customMem: ZSTD_customMem,
) -> *mut ZSTD_CStream {
    /* CStream and CCtx are now same object */
    crate::compress::zstd_compress::ZSTD_createCCtx_advanced(customMem)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_freeCStream(zcs: *mut ZSTD_CStream) -> usize {
    crate::compress::zstd_compress::ZSTD_freeCCtx(zcs) /* same object */
}

/*======   Initialization   ======*/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_CStreamInSize() -> usize {
    ZSTD_BLOCKSIZE_MAX
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_CStreamOutSize() -> usize {
    crate::compress::zstd_compress::ZSTD_compressBound(ZSTD_BLOCKSIZE_MAX)
        + ZSTD_blockHeaderSize
        + 4 /* 32-bits hash */
}

pub unsafe fn ZSTD_getCParamMode(
    cdict: *const ZSTD_CDict,
    params: *const ZSTD_CCtx_params,
    pledgedSrcSize: U64,
) -> ZSTD_CParamMode_e {
    if !cdict.is_null()
        && crate::compress::zstd_compress::ZSTD_shouldAttachDict(cdict, params, pledgedSrcSize) != 0
    {
        ZSTD_cpm_attachDict
    } else {
        ZSTD_cpm_noAttachDict
    }
}

/* ZSTD_resetCStream():
 * pledgedSrcSize == 0 means "unknown" */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_resetCStream(zcs: *mut ZSTD_CStream, pss: u64) -> usize {
    /* temporary : 0 interpreted as "unknown" during transition period.
     * Users willing to specify "unknown" **must** use ZSTD_CONTENTSIZE_UNKNOWN.
     * 0 will be interpreted as "empty" in the future.
     */
    let pledgedSrcSize: U64 = if pss == 0 {
        ZSTD_CONTENTSIZE_UNKNOWN
    } else {
        pss
    };
    {
        let e = crate::compress::zstd_compress::ZSTD_CCtx_reset(zcs, ZSTD_reset_session_only);
        if ERR_isError(e) != 0 {
            return e;
        }
    }
    {
        let e = crate::compress::zstd_compress::ZSTD_CCtx_setPledgedSrcSize(zcs, pledgedSrcSize);
        if ERR_isError(e) != 0 {
            return e;
        }
    }
    0
}

/* ZSTD_initCStream_internal() :
 *  Note : for lib/compress only. Used by zstdmt_compress.c.
 *  Assumption 1 : params are valid
 *  Assumption 2 : either dict, or cdict, is defined, not both */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_initCStream_internal(
    zcs: *mut ZSTD_CStream,
    dict: *const c_void,
    dictSize: usize,
    cdict: *const ZSTD_CDict,
    params: *const ZSTD_CCtx_params,
    pledgedSrcSize: u64,
) -> usize {
    {
        let e = crate::compress::zstd_compress::ZSTD_CCtx_reset(zcs, ZSTD_reset_session_only);
        if ERR_isError(e) != 0 {
            return e;
        }
    }
    {
        let e = crate::compress::zstd_compress::ZSTD_CCtx_setPledgedSrcSize(zcs, pledgedSrcSize);
        if ERR_isError(e) != 0 {
            return e;
        }
    }
    (*zcs).requestedParams = *params;
    if !dict.is_null() {
        let e = crate::compress::zstd_compress::ZSTD_CCtx_loadDictionary(zcs, dict, dictSize);
        if ERR_isError(e) != 0 {
            return e;
        }
    } else {
        /* Dictionary is cleared if !cdict */
        let e = crate::compress::zstd_compress::ZSTD_CCtx_refCDict(zcs, cdict);
        if ERR_isError(e) != 0 {
            return e;
        }
    }
    0
}

/* ZSTD_initCStream_usingCDict_advanced() :
 * same as ZSTD_initCStream_usingCDict(), with control over frame parameters */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_initCStream_usingCDict_advanced(
    zcs: *mut ZSTD_CStream,
    cdict: *const ZSTD_CDict,
    fParams: ZSTD_frameParameters,
    pledgedSrcSize: u64,
) -> usize {
    {
        let e = crate::compress::zstd_compress::ZSTD_CCtx_reset(zcs, ZSTD_reset_session_only);
        if ERR_isError(e) != 0 {
            return e;
        }
    }
    {
        let e = crate::compress::zstd_compress::ZSTD_CCtx_setPledgedSrcSize(zcs, pledgedSrcSize);
        if ERR_isError(e) != 0 {
            return e;
        }
    }
    (*zcs).requestedParams.fParams = fParams;
    {
        let e = crate::compress::zstd_compress::ZSTD_CCtx_refCDict(zcs, cdict);
        if ERR_isError(e) != 0 {
            return e;
        }
    }
    0
}

/* note : cdict must outlive compression session */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_initCStream_usingCDict(
    zcs: *mut ZSTD_CStream,
    cdict: *const ZSTD_CDict,
) -> usize {
    {
        let e = crate::compress::zstd_compress::ZSTD_CCtx_reset(zcs, ZSTD_reset_session_only);
        if ERR_isError(e) != 0 {
            return e;
        }
    }
    {
        let e = crate::compress::zstd_compress::ZSTD_CCtx_refCDict(zcs, cdict);
        if ERR_isError(e) != 0 {
            return e;
        }
    }
    0
}

/* ZSTD_initCStream_advanced() :
 * pledgedSrcSize must be exact.
 * if srcSize is not known at init time, use value ZSTD_CONTENTSIZE_UNKNOWN.
 * dict is loaded with default parameters ZSTD_dct_auto and ZSTD_dlm_byCopy. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_initCStream_advanced(
    zcs: *mut ZSTD_CStream,
    dict: *const c_void,
    dictSize: usize,
    params: ZSTD_parameters,
    pss: u64,
) -> usize {
    /* for compatibility with older programs relying on this behavior.
     * Users should now specify ZSTD_CONTENTSIZE_UNKNOWN.
     * This line will be removed in the future.
     */
    let pledgedSrcSize: U64 = if pss == 0 && params.fParams.contentSizeFlag == 0 {
        ZSTD_CONTENTSIZE_UNKNOWN
    } else {
        pss
    };
    {
        let e = crate::compress::zstd_compress::ZSTD_CCtx_reset(zcs, ZSTD_reset_session_only);
        if ERR_isError(e) != 0 {
            return e;
        }
    }
    {
        let e = crate::compress::zstd_compress::ZSTD_CCtx_setPledgedSrcSize(zcs, pledgedSrcSize);
        if ERR_isError(e) != 0 {
            return e;
        }
    }
    {
        let e = crate::compress::zstd_compress::ZSTD_checkCParams(params.cParams);
        if ERR_isError(e) != 0 {
            return e;
        }
    }
    crate::compress::zstd_compress::ZSTD_CCtxParams_setZstdParams(
        &mut (*zcs).requestedParams,
        &params,
    );
    {
        let e = crate::compress::zstd_compress::ZSTD_CCtx_loadDictionary(zcs, dict, dictSize);
        if ERR_isError(e) != 0 {
            return e;
        }
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_initCStream_usingDict(
    zcs: *mut ZSTD_CStream,
    dict: *const c_void,
    dictSize: usize,
    compressionLevel: c_int,
) -> usize {
    {
        let e = crate::compress::zstd_compress::ZSTD_CCtx_reset(zcs, ZSTD_reset_session_only);
        if ERR_isError(e) != 0 {
            return e;
        }
    }
    {
        let e = crate::compress::zstd_compress::ZSTD_CCtx_setParameter(
            zcs,
            ZSTD_c_compressionLevel,
            compressionLevel,
        );
        if ERR_isError(e) != 0 {
            return e;
        }
    }
    {
        let e = crate::compress::zstd_compress::ZSTD_CCtx_loadDictionary(zcs, dict, dictSize);
        if ERR_isError(e) != 0 {
            return e;
        }
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_initCStream_srcSize(
    zcs: *mut ZSTD_CStream,
    compressionLevel: c_int,
    pss: u64,
) -> usize {
    /* temporary : 0 interpreted as "unknown" during transition period.
     * Users willing to specify "unknown" **must** use ZSTD_CONTENTSIZE_UNKNOWN.
     * 0 will be interpreted as "empty" in the future.
     */
    let pledgedSrcSize: U64 = if pss == 0 {
        ZSTD_CONTENTSIZE_UNKNOWN
    } else {
        pss
    };
    {
        let e = crate::compress::zstd_compress::ZSTD_CCtx_reset(zcs, ZSTD_reset_session_only);
        if ERR_isError(e) != 0 {
            return e;
        }
    }
    {
        let e = crate::compress::zstd_compress::ZSTD_CCtx_refCDict(zcs, core::ptr::null());
        if ERR_isError(e) != 0 {
            return e;
        }
    }
    {
        let e = crate::compress::zstd_compress::ZSTD_CCtx_setParameter(
            zcs,
            ZSTD_c_compressionLevel,
            compressionLevel,
        );
        if ERR_isError(e) != 0 {
            return e;
        }
    }
    {
        let e = crate::compress::zstd_compress::ZSTD_CCtx_setPledgedSrcSize(zcs, pledgedSrcSize);
        if ERR_isError(e) != 0 {
            return e;
        }
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_initCStream(
    zcs: *mut ZSTD_CStream,
    compressionLevel: c_int,
) -> usize {
    {
        let e = crate::compress::zstd_compress::ZSTD_CCtx_reset(zcs, ZSTD_reset_session_only);
        if ERR_isError(e) != 0 {
            return e;
        }
    }
    {
        let e = crate::compress::zstd_compress::ZSTD_CCtx_refCDict(zcs, core::ptr::null());
        if ERR_isError(e) != 0 {
            return e;
        }
    }
    {
        let e = crate::compress::zstd_compress::ZSTD_CCtx_setParameter(
            zcs,
            ZSTD_c_compressionLevel,
            compressionLevel,
        );
        if ERR_isError(e) != 0 {
            return e;
        }
    }
    0
}

