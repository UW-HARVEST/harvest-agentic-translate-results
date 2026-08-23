//! Transliteration of compress/zstd_compress.c, lines 4695-5925 (PART 3 of 4).
//!
//! Frame header / epilogue writing, compressContinue / compressBlock,
//! dictionary loading, the simple compression API and the CDict API.
#![allow(
    non_snake_case,
    dead_code,
    unused_mut,
    unused_variables,
    non_upper_case_globals,
    non_camel_case_types,
    unused_assignments,
    unused_parens
)]

use core::ffi::{c_char, c_int, c_uint, c_ulonglong, c_void};

use crate::bits::ZSTD_highbit32;
use crate::entropy_common::{FSE_isError, FSE_readNCount, HUF_isError};
use crate::fse::*;
use crate::fse_compress::FSE_buildCTable_wksp;
use crate::huf::*;
use crate::huf_compress::HUF_readCTable;
use crate::error_private::*;
use crate::mem::*;
use crate::xxhash::ZSTD_XXH64_digest;
use crate::zstd_common::ZSTD_isError;
use crate::zstd_compress_internal::*;
use crate::zstd_cwksp::*;
use crate::zstd_h::*;
use crate::zstd_internal::*;

/* #define ZSTD_NO_CLEVEL 0  (zstd_compress.c:366) */
pub const ZSTD_NO_CLEVEL: c_int = 0;

/* ===============================================================
 * zstd_compress.c:4695
 * =============================================================== */

pub unsafe fn ZSTD_writeFrameHeader(
    dst: *mut c_void,
    dstCapacity: usize,
    params: *const ZSTD_CCtx_params,
    pledgedSrcSize: U64,
    dictID: U32,
) -> usize {
    let op: *mut BYTE = dst as *mut BYTE;
    /* 0-3 */
    let dictIDSizeCodeLength: U32 = ((dictID > 0) as U32)
        .wrapping_add((dictID >= 256) as U32)
        .wrapping_add((dictID >= 65536) as U32);
    /* 0-3 */
    let dictIDSizeCode: U32 = if (*params).fParams.noDictIDFlag != 0 {
        0
    } else {
        dictIDSizeCodeLength
    };
    let checksumFlag: U32 = ((*params).fParams.checksumFlag > 0) as U32;
    let windowSize: U32 = (1u32) << (*params).cParams.windowLog;
    let singleSegment: U32 = ((*params).fParams.contentSizeFlag != 0
        && (windowSize as U64) >= pledgedSrcSize) as U32;
    let windowLogByte: BYTE = ((*params)
        .cParams
        .windowLog
        .wrapping_sub(ZSTD_WINDOWLOG_ABSOLUTEMIN)
        << 3) as BYTE;
    /* 0-3 */
    let fcsCode: U32 = if (*params).fParams.contentSizeFlag != 0 {
        ((pledgedSrcSize >= 256) as U32)
            .wrapping_add((pledgedSrcSize >= 65536 + 256) as U32)
            .wrapping_add((pledgedSrcSize >= 0xFFFFFFFFu32 as U64) as U32)
    } else {
        0
    };
    let frameHeaderDescriptionByte: BYTE = dictIDSizeCode
        .wrapping_add(checksumFlag << 2)
        .wrapping_add(singleSegment << 5)
        .wrapping_add(fcsCode << 6) as BYTE;
    let mut pos: usize = 0;

    if dstCapacity < ZSTD_FRAMEHEADERSIZE_MAX {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    if (*params).format == ZSTD_f_zstd1 {
        MEM_writeLE32(dst as *mut u8, ZSTD_MAGICNUMBER);
        pos = 4;
    }
    *op.wrapping_add(pos) = frameHeaderDescriptionByte;
    pos += 1;
    if singleSegment == 0 {
        *op.wrapping_add(pos) = windowLogByte;
        pos += 1;
    }
    match dictIDSizeCode {
        1 => {
            *op.wrapping_add(pos) = dictID as BYTE;
            pos += 1;
        }
        2 => {
            MEM_writeLE16(op.wrapping_add(pos), dictID as U16);
            pos += 2;
        }
        3 => {
            MEM_writeLE32(op.wrapping_add(pos), dictID);
            pos += 4;
        }
        /* case 0 and (impossible) default */
        _ => {}
    }
    match fcsCode {
        1 => {
            MEM_writeLE16(op.wrapping_add(pos), pledgedSrcSize.wrapping_sub(256) as U16);
            pos += 2;
        }
        2 => {
            MEM_writeLE32(op.wrapping_add(pos), pledgedSrcSize as U32);
            pos += 4;
        }
        3 => {
            MEM_writeLE64(op.wrapping_add(pos), pledgedSrcSize as U64);
            pos += 8;
        }
        /* case 0 and (impossible) default */
        _ => {
            if singleSegment != 0 {
                *op.wrapping_add(pos) = pledgedSrcSize as BYTE;
                pos += 1;
            }
        }
    }
    pos
}

/* ZSTD_writeSkippableFrame_advanced() :
 * Writes out a skippable frame with the specified magic number variant (16 are supported),
 * from ZSTD_MAGIC_SKIPPABLE_START to ZSTD_MAGIC_SKIPPABLE_START+15, and the desired source data.
 *
 * Returns the total number of bytes written, or a ZSTD error code.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_writeSkippableFrame(
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
    magicVariant: c_uint,
) -> usize {
    let op: *mut BYTE = dst as *mut BYTE;
    if dstCapacity < srcSize.wrapping_add(ZSTD_SKIPPABLEHEADERSIZE) {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    if srcSize > 0xFFFFFFFFu32 as usize {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    if magicVariant > 15 {
        return ERROR(ZSTD_error_parameter_outOfBound);
    }

    MEM_writeLE32(op, ZSTD_MAGIC_SKIPPABLE_START.wrapping_add(magicVariant) as U32);
    MEM_writeLE32(op.wrapping_add(4), srcSize as U32);
    ZSTD_memcpy(op.wrapping_add(8), src as *const u8, srcSize);
    srcSize.wrapping_add(ZSTD_SKIPPABLEHEADERSIZE)
}

/* ZSTD_writeLastEmptyBlock() :
 * output an empty Block with end-of-frame mark to complete a frame
 * @return : size of data written into `dst` (== ZSTD_blockHeaderSize (defined in zstd_internal.h))
 *           or an error code if `dstCapacity` is too small (<ZSTD_blockHeaderSize)
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_writeLastEmptyBlock(dst: *mut c_void, dstCapacity: usize) -> usize {
    if dstCapacity < ZSTD_blockHeaderSize {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    {
        /* 0 size */
        let cBlockHeader24: U32 = 1 /*lastBlock*/ + (((bt_raw as U32)) << 1);
        MEM_writeLE24(dst as *mut u8, cBlockHeader24);
        return ZSTD_blockHeaderSize;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_referenceExternalSequences(
    cctx: *mut ZSTD_CCtx,
    seq: *mut rawSeq,
    nbSeq: usize,
) {
    (*cctx).externSeqStore.seq = seq;
    (*cctx).externSeqStore.size = nbSeq;
    (*cctx).externSeqStore.capacity = nbSeq;
    (*cctx).externSeqStore.pos = 0;
    (*cctx).externSeqStore.posInSequence = 0;
}

pub unsafe fn ZSTD_compressContinue_internal(
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
            (*cctx).pledgedSrcSizePlusOne.wrapping_sub(1) as U64,
            (*cctx).dictID,
        );
        {
            let err_code = fhSize;
            if ERR_isError(err_code) != 0 {
                return err_code;
            }
        }
        dstCapacity -= fhSize;
        dst = (dst as *mut c_char).wrapping_add(fhSize) as *mut c_void;
        (*cctx).stage = ZSTDcs_ongoing;
    }

    /* do not generate an empty block if no input */
    if srcSize == 0 {
        return fhSize;
    }

    if ZSTD_window_update(&mut (*ms).window, src, srcSize, (*ms).forceNonContiguous) == 0 {
        (*ms).forceNonContiguous = 0;
        (*ms).nextToUpdate = (*ms).window.dictLimit;
    }
    if (*cctx).appliedParams.ldmParams.enableLdm == ZSTD_ps_enable {
        ZSTD_window_update(
            &mut (*cctx).ldmState.window,
            src,
            srcSize,
            /* forceNonContiguous */ 0,
        );
    }

    if frame == 0 {
        /* overflow check and correction for block mode */
        crate::zstd_compress_p2::ZSTD_overflowCorrectIfNeeded(
            ms,
            &mut (*cctx).workspace,
            &(*cctx).appliedParams,
            src,
            (src as *const BYTE).wrapping_add(srcSize) as *const c_void,
        );
    }

    {
        let cSize: usize = if frame != 0 {
            crate::zstd_compress_p2::ZSTD_compress_frameChunk(
                cctx,
                dst,
                dstCapacity,
                src,
                srcSize,
                lastFrameChunk,
            )
        } else {
            crate::zstd_compress_p2::ZSTD_compressBlock_internal(
                cctx,
                dst,
                dstCapacity,
                src,
                srcSize,
                0, /* frame */
            )
        };
        {
            let err_code = cSize;
            if ERR_isError(err_code) != 0 {
                return err_code;
            }
        }
        (*cctx).consumedSrcSize = (*cctx)
            .consumedSrcSize
            .wrapping_add(srcSize as c_ulonglong);
        (*cctx).producedCSize = (*cctx)
            .producedCSize
            .wrapping_add((cSize.wrapping_add(fhSize)) as c_ulonglong);
        /* control src size */
        if (*cctx).pledgedSrcSizePlusOne != 0 {
            if (*cctx).consumedSrcSize.wrapping_add(1) > (*cctx).pledgedSrcSizePlusOne {
                return ERROR(ZSTD_error_srcSize_wrong);
            }
        }
        return cSize.wrapping_add(fhSize);
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

pub unsafe fn ZSTD_getBlockSize_deprecated(cctx: *const ZSTD_CCtx) -> usize {
    let cParams: ZSTD_compressionParameters = (*cctx).appliedParams.cParams;
    MIN(
        (*cctx).appliedParams.maxBlockSize,
        (1usize) << cParams.windowLog,
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
    let mut ip: *const BYTE = src as *const BYTE;
    let iend: *const BYTE = ip.wrapping_add(srcSize);
    let loadLdmDict: c_int =
        ((*params).ldmParams.enableLdm == ZSTD_ps_enable && !ls.is_null()) as c_int;

    /* Ensure large dictionaries can't cause index overflow */
    {
        /* Allow the dictionary to set indices up to exactly ZSTD_CURRENT_MAX.
         * Dictionaries right at the edge will immediately trigger overflow
         * correction, but I don't want to insert extra constraints here.
         */
        let mut maxDictSize: U32 = ZSTD_CURRENT_MAX.wrapping_sub(ZSTD_WINDOW_START_INDEX);

        let CDictTaggedIndices: c_int =
            crate::zstd_compress::ZSTD_CDictIndicesAreTagged(&(*params).cParams);
        if CDictTaggedIndices != 0 && tfp == ZSTD_tfp_forCDict {
            /* Some dictionary matchfinders in zstd use "short cache",
             * which treats the lower ZSTD_SHORT_CACHE_TAG_BITS of each
             * CDict hashtable entry as a tag rather than as part of an index.
             * When short cache is used, we need to truncate the dictionary
             * so that its indices don't overlap with the tag. */
            let shortCacheMaxDictSize: U32 = ((1u32) << (32 - ZSTD_SHORT_CACHE_TAG_BITS))
                .wrapping_sub(ZSTD_WINDOW_START_INDEX);
            maxDictSize = MIN(maxDictSize, shortCacheMaxDictSize);
        }

        /* If the dictionary is too large, only load the suffix of the dictionary. */
        if srcSize > maxDictSize as usize {
            ip = iend.wrapping_sub(maxDictSize as usize);
            src = ip as *const c_void;
            srcSize = maxDictSize as usize;
        }
    }

    ZSTD_window_update(
        &mut (*ms).window,
        src,
        srcSize,
        /* forceNonContiguous */ 0,
    );

    /* Load the entire dict into LDM matchfinders. */
    if loadLdmDict != 0 {
        ZSTD_window_update(
            &mut (*ls).window,
            src,
            srcSize,
            /* forceNonContiguous */ 0,
        );
        (*ls).loadedDictEnd = if (*params).forceWindow != 0 {
            0
        } else {
            (iend as usize).wrapping_sub((*ls).window.base as usize) as U32
        };
        crate::zstd_ldm::ZSTD_ldm_fillHashTable(ls, ip, iend, &(*params).ldmParams);
    }

    /* If the dict is larger than we can reasonably index in our tables, only load the suffix. */
    {
        let maxDictSize: U32 = (1u32)
            << MIN(
                MAX(
                    (*params).cParams.hashLog.wrapping_add(3),
                    (*params).cParams.chainLog.wrapping_add(1),
                ),
                31,
            );
        if srcSize > maxDictSize as usize {
            ip = iend.wrapping_sub(maxDictSize as usize);
            src = ip as *const c_void;
            srcSize = maxDictSize as usize;
        }
    }

    (*ms).nextToUpdate = (ip as usize).wrapping_sub((*ms).window.base as usize) as U32;
    (*ms).loadedDictEnd = if (*params).forceWindow != 0 {
        0
    } else {
        (iend as usize).wrapping_sub((*ms).window.base as usize) as U32
    };
    (*ms).forceNonContiguous = (*params).deterministicRefPrefix;

    if srcSize <= HASH_READ_SIZE {
        return 0;
    }

    crate::zstd_compress_p2::ZSTD_overflowCorrectIfNeeded(
        ms,
        ws,
        params,
        ip as *const c_void,
        iend as *const c_void,
    );

    match (*params).cParams.strategy {
        ZSTD_fast => {
            crate::zstd_fast::ZSTD_fillHashTable(ms, iend as *const c_void, dtlm, tfp);
        }
        ZSTD_dfast => {
            crate::zstd_double_fast::ZSTD_fillDoubleHashTable(
                ms,
                iend as *const c_void,
                dtlm,
                tfp,
            );
        }

        ZSTD_greedy | ZSTD_lazy | ZSTD_lazy2 => {
            if (*ms).dedicatedDictSearch != 0 {
                crate::zstd_lazy::ZSTD_dedicatedDictSearch_lazy_loadDictionary(
                    ms,
                    iend.wrapping_sub(HASH_READ_SIZE),
                );
            } else {
                if (*params).useRowMatchFinder == ZSTD_ps_enable {
                    let tagTableSize: usize = (1usize) << (*params).cParams.hashLog;
                    ZSTD_memset((*ms).tagTable, 0, tagTableSize);
                    crate::zstd_lazy::ZSTD_row_update(ms, iend.wrapping_sub(HASH_READ_SIZE));
                } else {
                    crate::zstd_lazy::ZSTD_insertAndFindFirstIndex(
                        ms,
                        iend.wrapping_sub(HASH_READ_SIZE),
                    );
                }
            }
        }

        /* we want the dictionary table fully sorted */
        ZSTD_btlazy2 | ZSTD_btopt | ZSTD_btultra | ZSTD_btultra2 => {
            crate::zstd_opt::ZSTD_updateTree(ms, iend.wrapping_sub(HASH_READ_SIZE), iend);
        }

        _ => {
            /* not possible : not a valid strategy id */
        }
    }

    (*ms).nextToUpdate = (iend as usize).wrapping_sub((*ms).window.base as usize) as U32;
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
        if *normalizedCounter.wrapping_add(s as usize) == 0 {
            return FSE_repeat_check;
        }
        s = s.wrapping_add(1);
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
    let mut offcodeNCount: [i16; MaxOff as usize + 1] = [0; MaxOff as usize + 1];
    let mut offcodeMaxValue: c_uint = MaxOff;
    /* skip magic num and dict ID */
    let mut dictPtr: *const BYTE = dict as *const BYTE;
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
            (dictEnd as usize).wrapping_sub(dictPtr as usize),
            &mut hasZeroWeights,
        );

        /* We only set the loaded table as valid if it contains all non-zero
         * weights. Otherwise, we set it to check */
        if hasZeroWeights == 0 && maxSymbolValue == 255 {
            (*bs).entropy.huf.repeatMode = HUF_repeat_valid;
        }

        if HUF_isError(hufHeaderSize) != 0 {
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
            (dictEnd as usize).wrapping_sub(dictPtr as usize),
        );
        if FSE_isError(offcodeHeaderSize) != 0 {
            return ERROR(ZSTD_error_dictionary_corrupted);
        }
        if offcodeLog > OffFSELog {
            return ERROR(ZSTD_error_dictionary_corrupted);
        }
        /* fill all offset symbols to avoid garbage at end of table */
        if FSE_isError(FSE_buildCTable_wksp(
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
        let mut matchlengthNCount: [i16; MaxML as usize + 1] = [0; MaxML as usize + 1];
        let mut matchlengthMaxValue: c_uint = MaxML;
        let mut matchlengthLog: c_uint = 0;
        let matchlengthHeaderSize: usize = FSE_readNCount(
            matchlengthNCount.as_mut_ptr(),
            &mut matchlengthMaxValue,
            &mut matchlengthLog,
            dictPtr as *const c_void,
            (dictEnd as usize).wrapping_sub(dictPtr as usize),
        );
        if FSE_isError(matchlengthHeaderSize) != 0 {
            return ERROR(ZSTD_error_dictionary_corrupted);
        }
        if matchlengthLog > MLFSELog {
            return ERROR(ZSTD_error_dictionary_corrupted);
        }
        if FSE_isError(FSE_buildCTable_wksp(
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
        let mut litlengthNCount: [i16; MaxLL as usize + 1] = [0; MaxLL as usize + 1];
        let mut litlengthMaxValue: c_uint = MaxLL;
        let mut litlengthLog: c_uint = 0;
        let litlengthHeaderSize: usize = FSE_readNCount(
            litlengthNCount.as_mut_ptr(),
            &mut litlengthMaxValue,
            &mut litlengthLog,
            dictPtr as *const c_void,
            (dictEnd as usize).wrapping_sub(dictPtr as usize),
        );
        if FSE_isError(litlengthHeaderSize) != 0 {
            return ERROR(ZSTD_error_dictionary_corrupted);
        }
        if litlengthLog > LLFSELog {
            return ERROR(ZSTD_error_dictionary_corrupted);
        }
        if FSE_isError(FSE_buildCTable_wksp(
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
    (*bs).rep[0] = MEM_readLE32(dictPtr.wrapping_add(0));
    (*bs).rep[1] = MEM_readLE32(dictPtr.wrapping_add(4));
    (*bs).rep[2] = MEM_readLE32(dictPtr.wrapping_add(8));
    dictPtr = dictPtr.wrapping_add(12);

    {
        let dictContentSize: usize = (dictEnd as usize).wrapping_sub(dictPtr as usize);
        let mut offcodeMax: U32 = MaxOff;
        if dictContentSize <= (0xFFFFFFFFu32 as usize).wrapping_sub(128 * 1024) {
            /* The maximum offset that must be supported */
            let maxOffset: U32 = (dictContentSize as U32).wrapping_add(128 * 1024);
            /* Calculate minimum offset code required to represent maxOffset */
            offcodeMax = ZSTD_highbit32(maxOffset);
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
                u = u.wrapping_add(1);
            }
        }
    }

    (dictPtr as usize).wrapping_sub(dict as *const BYTE as usize)
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
    let mut dictPtr: *const BYTE = dict as *const BYTE;
    let dictEnd: *const BYTE = dictPtr.wrapping_add(dictSize);
    let dictID: usize;
    let eSize: usize;

    dictID = if (*params).fParams.noDictIDFlag != 0 {
        0
    } else {
        /* skip magic number */
        MEM_readLE32(dictPtr.wrapping_add(4)) as usize
    };
    eSize = ZSTD_loadCEntropy(bs, workspace, dict, dictSize);
    {
        let err_code = eSize;
        if ERR_isError(err_code) != 0 {
            return err_code;
        }
    }
    dictPtr = dictPtr.wrapping_add(eSize);

    {
        let dictContentSize: usize = (dictEnd as usize).wrapping_sub(dictPtr as usize);
        {
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
    }
    dictID
}

/* ZSTD_compress_insertDictionary() :
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

    crate::zstd_compress::ZSTD_reset_compressedBlockState(bs);

    /* dict restricted modes */
    if dictContentType == ZSTD_dct_rawContent {
        return ZSTD_loadDictionaryContent(ms, ls, ws, params, dict, dictSize, dtlm, tfp);
    }

    if MEM_readLE32(dict as *const u8) != ZSTD_MAGIC_DICTIONARY {
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

pub const ZSTD_USE_CDICT_PARAMS_SRCSIZE_CUTOFF: usize = 128 * 1024;
pub const ZSTD_USE_CDICT_PARAMS_DICTSIZE_MULTIPLIER: u64 = 6u64;

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
    /* params are supposed to be fully validated at this point */
    if !cdict.is_null()
        && (*cdict).dictContentSize > 0
        && (pledgedSrcSize < ZSTD_USE_CDICT_PARAMS_SRCSIZE_CUTOFF as U64
            || pledgedSrcSize
                < ((*cdict).dictContentSize as U64)
                    .wrapping_mul(ZSTD_USE_CDICT_PARAMS_DICTSIZE_MULTIPLIER)
            || pledgedSrcSize == ZSTD_CONTENTSIZE_UNKNOWN
            || (*cdict).compressionLevel == 0)
        && (*params).attachDictPref != ZSTD_dictForceLoad
    {
        return crate::zstd_compress::ZSTD_resetCCtx_usingCDict(
            cctx,
            cdict,
            params,
            pledgedSrcSize,
            zbuff,
        );
    }

    {
        let err_code = crate::zstd_compress::ZSTD_resetCCtx_internal(
            cctx,
            params,
            pledgedSrcSize,
            dictContentSize,
            crate::zstd_compress::ZSTDcrp_makeClean,
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
        {
            let err_code = dictID;
            if ERR_isError(err_code) != 0 {
                return err_code;
            }
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
    pledgedSrcSize: c_ulonglong,
) -> usize {
    /* compression parameters verification and optimization */
    {
        let err_code = crate::zstd_compress::ZSTD_checkCParams((*params).cParams);
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
        pledgedSrcSize as U64,
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
    pledgedSrcSize: c_ulonglong,
) -> usize {
    let mut cctxParams: ZSTD_CCtx_params = core::mem::zeroed();
    crate::zstd_compress::ZSTD_CCtxParams_init_internal(&mut cctxParams, &params, ZSTD_NO_CLEVEL);
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
    let mut cctxParams: ZSTD_CCtx_params = core::mem::zeroed();
    {
        let params: ZSTD_parameters = crate::zstd_compress_p4::ZSTD_getParams_internal(
            compressionLevel,
            ZSTD_CONTENTSIZE_UNKNOWN as c_ulonglong,
            dictSize,
            ZSTD_cpm_noAttachDict,
        );
        crate::zstd_compress::ZSTD_CCtxParams_init_internal(
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

/* ZSTD_writeEpilogue() :
 *   Ends a frame.
 *   @return : nb of bytes written into dst (or an error code) */
pub unsafe fn ZSTD_writeEpilogue(
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
        {
            let err_code = fhSize;
            if ERR_isError(err_code) != 0 {
                return err_code;
            }
        }
        dstCapacity -= fhSize;
        op = op.wrapping_add(fhSize);
        (*cctx).stage = ZSTDcs_ongoing;
    }

    if (*cctx).stage != ZSTDcs_ending {
        /* write one last empty block, make it the "last" block */
        let cBlockHeader24: U32 = 1 /* last block */ + (((bt_raw as U32)) << 1) + 0;
        if dstCapacity < 3 {
            return ERROR(ZSTD_error_dstSize_tooSmall);
        }
        MEM_writeLE24(op, cBlockHeader24);
        op = op.wrapping_add(ZSTD_blockHeaderSize);
        dstCapacity -= ZSTD_blockHeaderSize;
    }

    if (*cctx).appliedParams.fParams.checksumFlag != 0 {
        let checksum: U32 = ZSTD_XXH64_digest(&(*cctx).xxhState) as U32;
        if dstCapacity < 4 {
            return ERROR(ZSTD_error_dstSize_tooSmall);
        }
        MEM_writeLE32(op, checksum);
        op = op.wrapping_add(4);
    }

    /* return to "created but no init" status */
    (*cctx).stage = ZSTDcs_created;
    (op as usize).wrapping_sub(ostart as usize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_CCtx_trace(cctx: *mut ZSTD_CCtx, extraCSize: usize) {
    /* ZSTD_TRACE == 0 : (void)cctx; (void)extraCSize; */
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
    {
        let err_code = cSize;
        if ERR_isError(err_code) != 0 {
            return err_code;
        }
    }
    endResult = ZSTD_writeEpilogue(
        cctx,
        (dst as *mut c_char).wrapping_add(cSize) as *mut c_void,
        dstCapacity.wrapping_sub(cSize),
    );
    {
        let err_code = endResult;
        if ERR_isError(err_code) != 0 {
            return err_code;
        }
    }
    /* control src size */
    if (*cctx).pledgedSrcSizePlusOne != 0 {
        if (*cctx).pledgedSrcSizePlusOne != (*cctx).consumedSrcSize.wrapping_add(1) {
            return ERROR(ZSTD_error_srcSize_wrong);
        }
    }
    ZSTD_CCtx_trace(cctx, endResult);
    cSize.wrapping_add(endResult)
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
        let err_code = crate::zstd_compress::ZSTD_checkCParams(params.cParams);
        if ERR_isError(err_code) != 0 {
            return err_code;
        }
    }
    crate::zstd_compress::ZSTD_CCtxParams_init_internal(
        &mut (*cctx).simpleApiParams,
        &params,
        ZSTD_NO_CLEVEL,
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
        let params: ZSTD_parameters = crate::zstd_compress_p4::ZSTD_getParams_internal(
            compressionLevel,
            srcSize as c_ulonglong,
            if !dict.is_null() { dictSize } else { 0 },
            ZSTD_cpm_noAttachDict,
        );
        crate::zstd_compress::ZSTD_CCtxParams_init_internal(
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
    /* ZSTD_COMPRESS_HEAPMODE == 0 */
    let mut ctxBody: core::mem::MaybeUninit<ZSTD_CCtx> = core::mem::MaybeUninit::uninit();
    let ctxBodyPtr: *mut ZSTD_CCtx = ctxBody.as_mut_ptr();
    crate::zstd_compress::ZSTD_initCCtx(ctxBodyPtr, ZSTD_defaultCMem);
    result = ZSTD_compressCCtx(ctxBodyPtr, dst, dstCapacity, src, srcSize, compressionLevel);
    /* can't free ctxBody itself, as it's on stack; free only heap content */
    crate::zstd_compress::ZSTD_freeCCtxContent(ctxBodyPtr);
    result
}

/* =====  Dictionary API  ===== */

/* ZSTD_estimateCDictSize_advanced() :
 *  Estimate amount of memory that will be needed to create a dictionary with following arguments */
#[unsafe(no_mangle)]
pub extern "C" fn ZSTD_estimateCDictSize_advanced(
    dictSize: usize,
    cParams: ZSTD_compressionParameters,
    dictLoadMethod: ZSTD_dictLoadMethod_e,
) -> usize {
    unsafe {
        ZSTD_cwksp_alloc_size(core::mem::size_of::<ZSTD_CDict>())
            + ZSTD_cwksp_alloc_size(HUF_WORKSPACE_SIZE)
            /* enableDedicatedDictSearch == 1 ensures that CDict estimation will not be too small
             * in case we are using DDS with row-hash. */
            + crate::zstd_compress::ZSTD_sizeof_matchState(
                &cParams,
                crate::zstd_compress::ZSTD_resolveRowMatchFinderMode(ZSTD_ps_auto, &cParams),
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
}

#[unsafe(no_mangle)]
pub extern "C" fn ZSTD_estimateCDictSize(dictSize: usize, compressionLevel: c_int) -> usize {
    let cParams: ZSTD_compressionParameters = unsafe {
        crate::zstd_compress_p4::ZSTD_getCParams_internal(
            compressionLevel,
            ZSTD_CONTENTSIZE_UNKNOWN as c_ulonglong,
            dictSize,
            ZSTD_cpm_createCDict,
        )
    };
    ZSTD_estimateCDictSize_advanced(dictSize, cParams, ZSTD_dlm_byCopy)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_sizeof_CDict(cdict: *const ZSTD_CDict) -> usize {
    /* support sizeof on NULL */
    if cdict.is_null() {
        return 0;
    }
    /* cdict may be in the workspace */
    (if (*cdict).workspace.workspace as *const c_void == cdict as *const c_void {
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
        let internalBuffer: *mut c_void = ZSTD_cwksp_reserve_object(
            &mut (*cdict).workspace,
            ZSTD_cwksp_align(dictSize, core::mem::size_of::<*mut c_void>()),
        );
        if internalBuffer.is_null() {
            return ERROR(ZSTD_error_memory_allocation);
        }
        (*cdict).dictContent = internalBuffer;
        ZSTD_memcpy(internalBuffer as *mut u8, dictBuffer as *const u8, dictSize);
    }
    (*cdict).dictContentSize = dictSize;
    (*cdict).dictContentType = dictContentType;

    (*cdict).entropyWorkspace =
        ZSTD_cwksp_reserve_object(&mut (*cdict).workspace, HUF_WORKSPACE_SIZE) as *mut U32;

    /* Reset the state to no dictionary */
    crate::zstd_compress::ZSTD_reset_compressedBlockState(&mut (*cdict).cBlockState);
    {
        let err_code = crate::zstd_compress::ZSTD_reset_matchState(
            &mut (*cdict).matchState,
            &mut (*cdict).workspace,
            &params.cParams,
            params.useRowMatchFinder,
            crate::zstd_compress::ZSTDcrp_makeClean,
            crate::zstd_compress::ZSTDirp_reset,
            crate::zstd_compress::ZSTD_resetTarget_CDict,
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
            {
                let err_code = dictID;
                if ERR_isError(err_code) != 0 {
                    return err_code;
                }
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
            + crate::zstd_compress::ZSTD_sizeof_matchState(
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
        let workspace: *mut u8 = ZSTD_customMalloc(workspaceSize, customMem);
        let mut ws: ZSTD_cwksp = core::mem::zeroed();
        let cdict: *mut ZSTD_CDict;

        if workspace.is_null() {
            ZSTD_customFree(workspace, customMem);
            return core::ptr::null_mut();
        }

        ZSTD_cwksp_init(
            &mut ws,
            workspace as *mut c_void,
            workspaceSize,
            ZSTD_cwksp_dynamic_alloc,
        );

        cdict =
            ZSTD_cwksp_reserve_object(&mut ws, core::mem::size_of::<ZSTD_CDict>()) as *mut ZSTD_CDict;
        ZSTD_cwksp_move(&mut (*cdict).workspace, &mut ws);
        (*cdict).customMem = customMem;
        /* signals advanced API usage */
        (*cdict).compressionLevel = ZSTD_NO_CLEVEL;
        (*cdict).useRowMatchFinder = useRowMatchFinder;
        return cdict;
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
        &mut cctxParams as *mut ZSTD_CCtx_params as *mut u8,
        0,
        core::mem::size_of::<ZSTD_CCtx_params>(),
    );
    crate::zstd_compress::ZSTD_CCtxParams_init(&mut cctxParams, 0);
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
        cParams = crate::zstd_compress_p4::ZSTD_dedicatedDictSearch_getCParams(
            cctxParams.compressionLevel,
            dictSize,
        );
        crate::zstd_compress::ZSTD_overrideCParams(&mut cParams, &cctxParams.cParams);
    } else {
        cParams = crate::zstd_compress::ZSTD_getCParamsFromCCtxParams(
            &cctxParams,
            ZSTD_CONTENTSIZE_UNKNOWN,
            dictSize,
            ZSTD_cpm_createCDict,
        );
    }

    if crate::zstd_compress_p4::ZSTD_dedicatedDictSearch_isSupported(&cParams) == 0 {
        /* Fall back to non-DDSS params */
        cctxParams.enableDedicatedDictSearch = 0;
        cParams = crate::zstd_compress::ZSTD_getCParamsFromCCtxParams(
            &cctxParams,
            ZSTD_CONTENTSIZE_UNKNOWN,
            dictSize,
            ZSTD_cpm_createCDict,
        );
    }

    cctxParams.cParams = cParams;
    cctxParams.useRowMatchFinder = crate::zstd_compress::ZSTD_resolveRowMatchFinderMode(
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
        || ZSTD_isError(ZSTD_initCDict_internal(
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
    let cParams: ZSTD_compressionParameters = crate::zstd_compress_p4::ZSTD_getCParams_internal(
        compressionLevel,
        ZSTD_CONTENTSIZE_UNKNOWN as c_ulonglong,
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
    let cParams: ZSTD_compressionParameters = crate::zstd_compress_p4::ZSTD_getCParams_internal(
        compressionLevel,
        ZSTD_CONTENTSIZE_UNKNOWN as c_ulonglong,
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
    /* support free on NULL */
    if cdict.is_null() {
        return 0;
    }
    {
        let cMem: ZSTD_customMem = (*cdict).customMem;
        let cdictInWorkspace: c_int =
            ZSTD_cwksp_owns_buffer(&(*cdict).workspace, cdict as *const c_void);
        ZSTD_cwksp_free(&mut (*cdict).workspace, cMem);
        if cdictInWorkspace == 0 {
            ZSTD_customFree(cdict as *mut u8, cMem);
        }
        return 0;
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
        crate::zstd_compress::ZSTD_resolveRowMatchFinderMode(ZSTD_ps_auto, &cParams);
    /* enableDedicatedDictSearch == 1 ensures matchstate is not too small in case this CDict will be used for DDS + row hash */
    let matchStateSize: usize = crate::zstd_compress::ZSTD_sizeof_matchState(
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
    let mut params: ZSTD_CCtx_params = core::mem::zeroed();

    /* 8-aligned */
    if (workspace as usize) & 7 != 0 {
        return core::ptr::null();
    }

    {
        let mut ws: ZSTD_cwksp = core::mem::zeroed();
        ZSTD_cwksp_init(&mut ws, workspace, workspaceSize, ZSTD_cwksp_static_alloc);
        cdict =
            ZSTD_cwksp_reserve_object(&mut ws, core::mem::size_of::<ZSTD_CDict>()) as *mut ZSTD_CDict;
        if cdict.is_null() {
            return core::ptr::null();
        }
        ZSTD_cwksp_move(&mut (*cdict).workspace, &mut ws);
    }

    if workspaceSize < neededSize {
        return core::ptr::null();
    }

    crate::zstd_compress::ZSTD_CCtxParams_init(&mut params, 0);
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
    pledgedSrcSize: c_ulonglong,
) -> usize {
    let mut cctxParams: ZSTD_CCtx_params = core::mem::zeroed();
    if cdict.is_null() {
        return ERROR(ZSTD_error_dictionary_wrong);
    }
    /* Initialize the cctxParams from the cdict */
    {
        let mut params: ZSTD_parameters = core::mem::zeroed();
        params.fParams = fParams;
        params.cParams = if pledgedSrcSize
            < ZSTD_USE_CDICT_PARAMS_SRCSIZE_CUTOFF as c_ulonglong
            || pledgedSrcSize
                < ((*cdict).dictContentSize as c_ulonglong)
                    .wrapping_mul(ZSTD_USE_CDICT_PARAMS_DICTSIZE_MULTIPLIER as c_ulonglong)
            || pledgedSrcSize == ZSTD_CONTENTSIZE_UNKNOWN as c_ulonglong
            || (*cdict).compressionLevel == 0
        {
            ZSTD_getCParamsFromCDict(cdict)
        } else {
            crate::zstd_compress_p4::ZSTD_getCParams(
                (*cdict).compressionLevel,
                pledgedSrcSize,
                (*cdict).dictContentSize,
            )
        };
        crate::zstd_compress::ZSTD_CCtxParams_init_internal(
            &mut cctxParams,
            &params,
            (*cdict).compressionLevel,
        );
    }
    /* Increase window log to fit the entire dictionary and source if the
     * source size is known. Limit the increase to 19, which is the
     * window log for compression level 1 with the largest source size.
     */
    if pledgedSrcSize != ZSTD_CONTENTSIZE_UNKNOWN as c_ulonglong {
        let limitedSrcSize: U32 =
            MIN(pledgedSrcSize, ((1u32) << 19) as c_ulonglong) as U32;
        let limitedSrcLog: U32 = if limitedSrcSize > 1 {
            ZSTD_highbit32(limitedSrcSize.wrapping_sub(1)).wrapping_add(1)
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
        pledgedSrcSize as U64,
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
    pledgedSrcSize: c_ulonglong,
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
    ZSTD_compressBegin_usingCDict_internal(
        cctx,
        cdict,
        fParams,
        ZSTD_CONTENTSIZE_UNKNOWN as c_ulonglong,
    )
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
    {
        let err_code = ZSTD_compressBegin_usingCDict_internal(
            cctx,
            cdict,
            fParams,
            srcSize as c_ulonglong,
        );
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
