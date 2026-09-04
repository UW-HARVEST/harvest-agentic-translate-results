//! Literal, semantics-preserving transliteration of PART 3 (final part) of
//! `compress/zstd_compress.c` (C lines ~4525..7843): frame header/chunk
//! assembly, epilogue, dictionary loading (raw + zstd dict), CDict lifecycle,
//! the streaming API (compressStream / compressStream2 state machine) and the
//! explicit-sequences compression API.
//!
//! Build config: DYNAMIC_BMI2=0, no ZSTD_MULTITHREAD (all MT branches compiled
//! out, nbWorkers forced to 0), DEBUGLEVEL 0 (asserts/DEBUGLOG dropped),
//! ZSTD_TRACE==1 (traceCtx field present, trace call sites are guarded no-ops
//! because ZSTD_trace_compress_* are undefined weak symbols -> NULL).
//! __AVX2__ / ZSTD_ARCH_X86_AVX2 are NOT defined (no -mavx2), so the scalar
//! bodies of convertSequences_noRepcodes and ZSTD_get1BlockSummary are used.
//!
//! CDict layout note: ZSTD_CDict_s is declared as an opaque enum in
//! zstd_compress_internal.rs. Part 1 (zstd_compress.rs) owns the real
//! `#[repr(C)] ZSTD_CDict_s_layout` mirror and exposes it `pub`. This file
//! reuses that same layout type (imported) and casts opaque `*mut ZSTD_CDict`
//! pointers to `*mut ZSTD_CDict_s_layout` to read/write fields.
#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(unused_parens)]
#![allow(unused_assignments)]
#![allow(unused_variables)]

use core::ffi::{c_char, c_int, c_uint, c_void};
use core::ptr::{null, null_mut};

use crate::common::bits::ZSTD_highbit32;
use crate::common::error_private::*;
use crate::common::mem::*;
use crate::common::xxhash::{ZSTD_XXH64_digest, ZSTD_XXH64_update};
use crate::common::zstd_common::ZSTD_isError;
use crate::common::zstd_h::*;
use crate::common::zstd_internal::*;
/* Disambiguate: ZSTD_customMem is declared in both zstd_h and zstd_internal;
 * the CCtx/CCtxParams/CDict struct fields use the zstd_internal one. */
use crate::common::zstd_internal::ZSTD_customMem;

use crate::common::entropy_common::{FSE_isError, FSE_readNCount, HUF_isError};
use crate::common::fse::{FSE_repeat, FSE_repeat_check, FSE_repeat_valid};
use crate::common::huf::{HUF_repeat_check, HUF_repeat_valid, HUF_WORKSPACE_SIZE};
use crate::compress::fse_compress::FSE_buildCTable_wksp;
use crate::compress::huf_compress::HUF_readCTable;

use crate::compress::zstd_compress_internal::*;
use crate::compress::zstd_cwksp::*;

/* Part 1 (zstd_compress.rs) functions we reuse. */
use crate::compress::zstd_compress::{
    ZSTD_assertEqualCParams, ZSTD_CCtx_loadDictionary, ZSTD_CCtx_refCDict, ZSTD_CCtx_reset,
    ZSTD_CCtx_setParameter, ZSTD_CCtx_setPledgedSrcSize, ZSTD_CCtxParams_init,
    ZSTD_CCtxParams_init_internal, ZSTD_CCtxParams_setZstdParams, ZSTD_CDict_s_layout,
    ZSTD_CDictIndicesAreTagged, ZSTD_checkCParams, ZSTD_clampCParams, ZSTD_compressBound, ZSTD_createCCtx, ZSTD_createCCtx_advanced,
    ZSTD_cycleLog, ZSTD_freeCCtx, ZSTD_freeCCtxContent, ZSTD_getCParams,
    ZSTD_getCParamsFromCCtxParams, ZSTD_initCCtx, ZSTD_initLocalDict, ZSTD_initStaticCCtx,
    ZSTD_NO_CLEVEL, ZSTD_overrideCParams, ZSTD_reduceIndex, ZSTD_reset_compressedBlockState,
    ZSTD_reset_matchState, ZSTD_resetCCtx_internal, ZSTD_resetCCtx_usingCDict,
    ZSTD_resetTarget_CDict, ZSTDcrp_makeClean, ZSTDirp_reset,
    ZSTD_resolveBlockSplitterMode, ZSTD_resolveEnableLdm, ZSTD_resolveExternalRepcodeSearch,
    ZSTD_resolveExternalSequenceValidation, ZSTD_resolveMaxBlockSize, ZSTD_resolveRowMatchFinderMode,
    ZSTD_shouldAttachDict, ZSTD_sizeof_matchState,
};

/* Functions from part 1 that are physically located in the C excluded region
 * (dedicatedDictSearch / getCParams(_internal) / getParams(_internal)). */
use crate::compress::zstd_compress::{
    ZSTD_dedicatedDictSearch_getCParams, ZSTD_dedicatedDictSearch_isSupported, ZSTD_getCParams_internal,
    ZSTD_getParams_internal, ZSTD_minCLevel,
};

/* Part 2 (zstd_compress_blocks.rs) block-level functions we call. */
use crate::compress::zstd_compress_blocks::{
    ZSTD_blockSplitterEnabled, ZSTD_blockState_confirmRepcodesAndEntropyTables,
    ZSTD_compressBlock_internal, ZSTD_compressBlock_splitBlock, ZSTD_compressBlock_targetCBlockSize,
    ZSTD_entropyCompressSeqStore, ZSTD_entropyCompressSeqStore_internal, ZSTD_isRLE, ZSTD_maybeRLE,
    ZSTD_resetSeqStore, ZSTD_storeLastLiterals, ZSTD_useTargetCBlockSize,
};

/* Strategy match-finder / dictionary fillers. */
use crate::compress::zstd_double_fast::ZSTD_fillDoubleHashTable;
use crate::compress::zstd_fast::ZSTD_fillHashTable;
use crate::compress::zstd_lazy::{
    ZSTD_dedicatedDictSearch_lazy_loadDictionary, ZSTD_insertAndFindFirstIndex, ZSTD_row_update,
};
use crate::compress::zstd_ldm::ZSTD_ldm_fillHashTable;
use crate::compress::zstd_opt::ZSTD_updateTree;
use crate::compress::zstd_preSplit::ZSTD_splitBlock;

/* clevels.h constant used by dedicatedDictSearch (also imported by part1). */
use crate::compress::clevels::ZSTD_MAX_CLEVEL;

/* ------------------------------------------------------------------ */
/* helper: cast an opaque CDict pointer to its real layout             */
/* ------------------------------------------------------------------ */
#[inline]
unsafe fn cdl(cdict: *const ZSTD_CDict) -> *const ZSTD_CDict_s_layout {
    cdict as *const ZSTD_CDict_s_layout
}
#[inline]
unsafe fn cdl_mut(cdict: *mut ZSTD_CDict) -> *mut ZSTD_CDict_s_layout {
    cdict as *mut ZSTD_CDict_s_layout
}

/* #define ZSTD_USE_CDICT_PARAMS_SRCSIZE_CUTOFF (128 KB) */
const ZSTD_USE_CDICT_PARAMS_SRCSIZE_CUTOFF: U64 = 128 * (1u64 << 10);
/* #define ZSTD_USE_CDICT_PARAMS_DICTSIZE_MULTIPLIER (6ULL) */
const ZSTD_USE_CDICT_PARAMS_DICTSIZE_MULTIPLIER: U64 = 6;

/* ================================================================== */
/*  ZSTD_overflowCorrectIfNeeded                                       */
/* ================================================================== */
pub unsafe fn ZSTD_overflowCorrectIfNeeded(
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
        let correction: U32 = ZSTD_window_correctOverflow(&mut (*ms).window, cycleLog, maxDist, ip);
        ZSTD_cwksp_mark_tables_dirty(ws);
        ZSTD_reduceIndex(ms, params, correction);
        ZSTD_cwksp_mark_tables_clean(ws);
        if (*ms).nextToUpdate < correction {
            (*ms).nextToUpdate = 0;
        } else {
            (*ms).nextToUpdate -= correction;
        }
        /* invalidate dictionaries on overflow correction */
        (*ms).loadedDictEnd = 0;
        (*ms).dictMatchState = null();
    }
}

/* ================================================================== */
/*  ZSTD_optimalBlockSize                                              */
/* ================================================================== */
unsafe fn ZSTD_optimalBlockSize(
    cctx: *mut ZSTD_CCtx,
    src: *const c_void,
    srcSize: size_t,
    blockSizeMax: size_t,
    splitLevel: c_int,
    strat: ZSTD_strategy,
    savings: S64,
) -> size_t {
    /* split level based on compression strategy, from `fast` to `btultra2` */
    static splitLevels: [c_int; 10] = [0, 0, 1, 2, 2, 3, 3, 4, 4, 4];
    let mut splitLevel = splitLevel;
    if srcSize < 128 * (1 << 10) || blockSizeMax < 128 * (1 << 10) {
        return MIN(srcSize, blockSizeMax);
    }
    if savings < 3 {
        return 128 * (1 << 10);
    }
    if splitLevel == 1 {
        return 128 * (1 << 10);
    }
    if splitLevel == 0 {
        splitLevel = splitLevels[strat as usize];
    } else {
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

/* ================================================================== */
/*  ZSTD_compress_frameChunk                                           */
/* ================================================================== */
unsafe fn ZSTD_compress_frameChunk(
    cctx: *mut ZSTD_CCtx,
    dst: *mut c_void,
    mut dstCapacity: size_t,
    src: *const c_void,
    srcSize: size_t,
    lastFrameChunk: U32,
) -> size_t {
    let blockSizeMax: size_t = (*cctx).blockSizeMax;
    let mut remaining: size_t = srcSize;
    let mut ip: *const BYTE = src as *const BYTE;
    let ostart: *mut BYTE = dst as *mut BYTE;
    let mut op: *mut BYTE = ostart;
    let maxDist: U32 = 1u32 << (*cctx).appliedParams.cParams.windowLog;
    let mut savings: S64 = (*cctx).consumedSrcSize as S64 - (*cctx).producedCSize as S64;

    if (*cctx).appliedParams.fParams.checksumFlag != 0 && srcSize != 0 {
        ZSTD_XXH64_update(&mut (*cctx).xxhState, src, srcSize);
    }

    while remaining != 0 {
        let ms: *mut ZSTD_MatchState_t = &mut (*cctx).blockState.matchState;
        let blockSize: size_t = ZSTD_optimalBlockSize(
            cctx,
            ip as *const c_void,
            remaining,
            blockSizeMax,
            (*cctx).appliedParams.preBlockSplitter_level,
            (*cctx).appliedParams.cParams.strategy,
            savings,
        );
        let lastBlock: U32 = lastFrameChunk & (blockSize == remaining) as U32;

        if dstCapacity < ZSTD_blockHeaderSize + MIN_CBLOCK_SIZE as size_t + 1 {
            return ERROR(ZSTD_error_dstSize_tooSmall);
        }

        ZSTD_overflowCorrectIfNeeded(
            ms,
            &mut (*cctx).workspace,
            &(*cctx).appliedParams,
            ip as *const c_void,
            ip.wrapping_add(blockSize) as *const c_void,
        );
        ZSTD_checkDictValidity(
            &(*ms).window,
            ip.wrapping_add(blockSize) as *const c_void,
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

        /* Ensure hash/chain table insertion resumes no sooner than lowlimit */
        if (*ms).nextToUpdate < (*ms).window.lowLimit {
            (*ms).nextToUpdate = (*ms).window.lowLimit;
        }

        {
            let mut cSize: size_t;
            if ZSTD_useTargetCBlockSize(&(*cctx).appliedParams) != 0 {
                cSize = ZSTD_compressBlock_targetCBlockSize(
                    cctx,
                    op as *mut c_void,
                    dstCapacity,
                    ip as *const c_void,
                    blockSize,
                    lastBlock,
                );
                if ERR_isError(cSize) != 0 {
                    return cSize;
                }
            } else if ZSTD_blockSplitterEnabled(&mut (*cctx).appliedParams) != 0 {
                cSize = ZSTD_compressBlock_splitBlock(
                    cctx,
                    op as *mut c_void,
                    dstCapacity,
                    ip as *const c_void,
                    blockSize,
                    lastBlock,
                );
                if ERR_isError(cSize) != 0 {
                    return cSize;
                }
            } else {
                cSize = ZSTD_compressBlock_internal(
                    cctx,
                    op.wrapping_add(ZSTD_blockHeaderSize) as *mut c_void,
                    dstCapacity - ZSTD_blockHeaderSize,
                    ip as *const c_void,
                    blockSize,
                    1, /* frame */
                );
                if ERR_isError(cSize) != 0 {
                    return cSize;
                }

                if cSize == 0 {
                    /* block is not compressible */
                    cSize = ZSTD_noCompressBlock(
                        op as *mut c_void,
                        dstCapacity,
                        ip as *const c_void,
                        blockSize,
                        lastBlock,
                    );
                    if ERR_isError(cSize) != 0 {
                        return cSize;
                    }
                } else {
                    let cBlockHeader: U32 = if cSize == 1 {
                        lastBlock + (((bt_rle as U32)) << 1) + ((blockSize << 3) as U32)
                    } else {
                        lastBlock + (((bt_compressed as U32)) << 1) + ((cSize << 3) as U32)
                    };
                    MEM_writeLE24(op, cBlockHeader);
                    cSize += ZSTD_blockHeaderSize;
                }
            }

            savings += (blockSize as S64) - (cSize as S64);

            ip = ip.wrapping_add(blockSize);
            remaining -= blockSize;
            op = op.wrapping_add(cSize);
            dstCapacity -= cSize;
            (*cctx).isFirstBlock = 0;
        }
    }

    if lastFrameChunk != 0 && (op as *const BYTE) > (ostart as *const BYTE) {
        (*cctx).stage = ZSTDcs_ending;
    }
    op.offset_from(ostart) as size_t
}

/* ================================================================== */
/*  ZSTD_writeFrameHeader                                              */
/* ================================================================== */
unsafe fn ZSTD_writeFrameHeader(
    dst: *mut c_void,
    dstCapacity: size_t,
    params: *const ZSTD_CCtx_params,
    pledgedSrcSize: U64,
    dictID: U32,
) -> size_t {
    let op: *mut BYTE = dst as *mut BYTE;
    let dictIDSizeCodeLength: U32 =
        (dictID > 0) as U32 + (dictID >= 256) as U32 + (dictID >= 65536) as U32; /* 0-3 */
    let dictIDSizeCode: U32 = if (*params).fParams.noDictIDFlag != 0 {
        0
    } else {
        dictIDSizeCodeLength
    }; /* 0-3 */
    let checksumFlag: U32 = ((*params).fParams.checksumFlag > 0) as U32;
    let windowSize: U32 = 1u32 << (*params).cParams.windowLog;
    let singleSegment: U32 = ((*params).fParams.contentSizeFlag != 0
        && (windowSize as U64 >= pledgedSrcSize)) as U32;
    let windowLogByte: BYTE =
        (((*params).cParams.windowLog - ZSTD_WINDOWLOG_ABSOLUTEMIN) << 3) as BYTE;
    let fcsCode: U32 = if (*params).fParams.contentSizeFlag != 0 {
        (pledgedSrcSize >= 256) as U32
            + (pledgedSrcSize >= 65536 + 256) as U32
            + (pledgedSrcSize >= 0xFFFFFFFFu64) as U32
    } else {
        0
    }; /* 0-3 */
    let frameHeaderDescriptionByte: BYTE =
        (dictIDSizeCode + (checksumFlag << 2) + (singleSegment << 5) + (fcsCode << 6)) as BYTE;
    let mut pos: size_t = 0;

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
        _ => { /* case 0 (and impossible default) : break */ }
    }
    match fcsCode {
        1 => {
            MEM_writeLE16(op.wrapping_add(pos), (pledgedSrcSize.wrapping_sub(256)) as U16);
            pos += 2;
        }
        2 => {
            MEM_writeLE32(op.wrapping_add(pos), pledgedSrcSize as U32);
            pos += 4;
        }
        3 => {
            MEM_writeLE64(op.wrapping_add(pos), pledgedSrcSize);
            pos += 8;
        }
        _ => {
            /* case 0 (and impossible default) */
            if singleSegment != 0 {
                *op.wrapping_add(pos) = pledgedSrcSize as BYTE;
                pos += 1;
            }
        }
    }
    pos
}

/* ================================================================== */
/*  ZSTD_writeSkippableFrame                                           */
/* ================================================================== */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_writeSkippableFrame(
    dst: *mut c_void,
    dstCapacity: size_t,
    src: *const c_void,
    srcSize: size_t,
    magicVariant: c_uint,
) -> size_t {
    let op: *mut BYTE = dst as *mut BYTE;
    if dstCapacity < srcSize + ZSTD_SKIPPABLEHEADERSIZE {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    if srcSize > 0xFFFFFFFFu32 as size_t {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    if magicVariant > 15 {
        return ERROR(ZSTD_error_parameter_outOfBound);
    }

    MEM_writeLE32(op, ZSTD_MAGIC_SKIPPABLE_START + magicVariant);
    MEM_writeLE32(op.wrapping_add(4), srcSize as U32);
    ZSTD_memcpy(op.wrapping_add(8), src as *const u8, srcSize);
    srcSize + ZSTD_SKIPPABLEHEADERSIZE
}

/* ================================================================== */
/*  ZSTD_writeLastEmptyBlock                                           */
/* ================================================================== */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_writeLastEmptyBlock(
    dst: *mut c_void,
    dstCapacity: size_t,
) -> size_t {
    if dstCapacity < ZSTD_blockHeaderSize {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    {
        let cBlockHeader24: U32 = 1 /*lastBlock*/ + (((bt_raw as U32)) << 1); /* 0 size */
        MEM_writeLE24(dst as *mut u8, cBlockHeader24);
        ZSTD_blockHeaderSize
    }
}

/* ================================================================== */
/*  ZSTD_referenceExternalSequences                                    */
/* ================================================================== */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_referenceExternalSequences(
    cctx: *mut ZSTD_CCtx,
    seq: *mut rawSeq,
    nbSeq: size_t,
) {
    (*cctx).externSeqStore.seq = seq;
    (*cctx).externSeqStore.size = nbSeq;
    (*cctx).externSeqStore.capacity = nbSeq;
    (*cctx).externSeqStore.pos = 0;
    (*cctx).externSeqStore.posInSequence = 0;
}

/* ================================================================== */
/*  ZSTD_compressContinue_internal                                     */
/* ================================================================== */
unsafe fn ZSTD_compressContinue_internal(
    cctx: *mut ZSTD_CCtx,
    mut dst: *mut c_void,
    mut dstCapacity: size_t,
    src: *const c_void,
    srcSize: size_t,
    frame: U32,
    lastFrameChunk: U32,
) -> size_t {
    let ms: *mut ZSTD_MatchState_t = &mut (*cctx).blockState.matchState;
    let mut fhSize: size_t = 0;

    if (*cctx).stage == ZSTDcs_created {
        return ERROR(ZSTD_error_stage_wrong);
    }

    if frame != 0 && (*cctx).stage == ZSTDcs_init {
        fhSize = ZSTD_writeFrameHeader(
            dst,
            dstCapacity,
            &(*cctx).appliedParams,
            (*cctx).pledgedSrcSizePlusOne - 1,
            (*cctx).dictID,
        );
        if ERR_isError(fhSize) != 0 {
            return fhSize;
        }
        dstCapacity -= fhSize;
        dst = (dst as *mut c_char).wrapping_add(fhSize) as *mut c_void;
        (*cctx).stage = ZSTDcs_ongoing;
    }

    if srcSize == 0 {
        return fhSize; /* do not generate an empty block if no input */
    }

    if ZSTD_window_update(&mut (*ms).window, src, srcSize, (*ms).forceNonContiguous as c_int) == 0 {
        (*ms).forceNonContiguous = 0;
        (*ms).nextToUpdate = (*ms).window.dictLimit;
    }
    if (*cctx).appliedParams.ldmParams.enableLdm == ZSTD_ps_enable {
        ZSTD_window_update(&mut (*cctx).ldmState.window, src, srcSize, 0);
    }

    if frame == 0 {
        /* overflow check and correction for block mode */
        ZSTD_overflowCorrectIfNeeded(
            ms,
            &mut (*cctx).workspace,
            &(*cctx).appliedParams,
            src,
            (src as *const BYTE).wrapping_add(srcSize) as *const c_void,
        );
    }

    {
        let cSize: size_t = if frame != 0 {
            ZSTD_compress_frameChunk(cctx, dst, dstCapacity, src, srcSize, lastFrameChunk)
        } else {
            ZSTD_compressBlock_internal(cctx, dst, dstCapacity, src, srcSize, 0 /* frame */)
        };
        if ERR_isError(cSize) != 0 {
            return cSize;
        }
        (*cctx).consumedSrcSize += srcSize as U64;
        (*cctx).producedCSize += (cSize + fhSize) as U64;
        if (*cctx).pledgedSrcSizePlusOne != 0 {
            /* control src size */
            if (*cctx).consumedSrcSize + 1 > (*cctx).pledgedSrcSizePlusOne {
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
    dstCapacity: size_t,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    ZSTD_compressContinue_internal(
        cctx, dst, dstCapacity, src, srcSize, 1, /* frame mode */
        0, /* last chunk */
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressContinue(
    cctx: *mut ZSTD_CCtx,
    dst: *mut c_void,
    dstCapacity: size_t,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    ZSTD_compressContinue_public(cctx, dst, dstCapacity, src, srcSize)
}

unsafe fn ZSTD_getBlockSize_deprecated(cctx: *const ZSTD_CCtx) -> size_t {
    let cParams: ZSTD_compressionParameters = (*cctx).appliedParams.cParams;
    MIN((*cctx).appliedParams.maxBlockSize, (1 as size_t) << cParams.windowLog)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_getBlockSize(cctx: *const ZSTD_CCtx) -> size_t {
    ZSTD_getBlockSize_deprecated(cctx)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock_deprecated(
    cctx: *mut ZSTD_CCtx,
    dst: *mut c_void,
    dstCapacity: size_t,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    {
        let blockSizeMax: size_t = ZSTD_getBlockSize_deprecated(cctx);
        if srcSize > blockSizeMax {
            return ERROR(ZSTD_error_srcSize_wrong);
        }
    }
    ZSTD_compressContinue_internal(
        cctx, dst, dstCapacity, src, srcSize, 0, /* frame mode */
        0, /* last chunk */
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBlock(
    cctx: *mut ZSTD_CCtx,
    dst: *mut c_void,
    dstCapacity: size_t,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    ZSTD_compressBlock_deprecated(cctx, dst, dstCapacity, src, srcSize)
}

/* ================================================================== */
/*  ZSTD_loadDictionaryContent                                         */
/* ================================================================== */
unsafe fn ZSTD_loadDictionaryContent(
    ms: *mut ZSTD_MatchState_t,
    ls: *mut ldmState_t,
    ws: *mut ZSTD_cwksp,
    params: *const ZSTD_CCtx_params,
    mut src: *const c_void,
    mut srcSize: size_t,
    dtlm: ZSTD_dictTableLoadMethod_e,
    tfp: ZSTD_tableFillPurpose_e,
) -> size_t {
    let mut ip: *const BYTE = src as *const BYTE;
    let iend: *const BYTE = ip.wrapping_add(srcSize);
    let loadLdmDict: c_int =
        ((*params).ldmParams.enableLdm == ZSTD_ps_enable && ls != null_mut()) as c_int;

    /* Assert that the ms params match the params we're being given */
    ZSTD_assertEqualCParams((*params).cParams, (*ms).cParams);

    {
        /* Ensure large dictionaries can't cause index overflow */
        let mut maxDictSize: U32 = ZSTD_CURRENT_MAX - ZSTD_WINDOW_START_INDEX;

        let CDictTaggedIndices: c_int = ZSTD_CDictIndicesAreTagged(&(*params).cParams);
        if CDictTaggedIndices != 0 && tfp == ZSTD_tfp_forCDict {
            let shortCacheMaxDictSize: U32 =
                (1u32 << (32 - ZSTD_SHORT_CACHE_TAG_BITS)) - ZSTD_WINDOW_START_INDEX;
            maxDictSize = MIN(maxDictSize, shortCacheMaxDictSize);
        }

        /* If the dictionary is too large, only load the suffix of the dictionary. */
        if srcSize > maxDictSize as size_t {
            ip = iend.wrapping_sub(maxDictSize as usize);
            src = ip as *const c_void;
            srcSize = maxDictSize as size_t;
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
                MAX((*params).cParams.hashLog + 3, (*params).cParams.chainLog + 1),
                31,
            );
        if srcSize > maxDictSize as size_t {
            ip = iend.wrapping_sub(maxDictSize as usize);
            src = ip as *const c_void;
            srcSize = maxDictSize as size_t;
        }
    }

    (*ms).nextToUpdate = ip.offset_from((*ms).window.base) as U32;
    (*ms).loadedDictEnd = if (*params).forceWindow != 0 {
        0
    } else {
        iend.offset_from((*ms).window.base) as U32
    };
    (*ms).forceNonContiguous = (*params).deterministicRefPrefix;

    if srcSize <= HASH_READ_SIZE as size_t {
        return 0;
    }

    ZSTD_overflowCorrectIfNeeded(ms, ws, params, ip as *const c_void, iend as *const c_void);

    match (*params).cParams.strategy {
        s if s == ZSTD_fast => {
            ZSTD_fillHashTable(ms, iend as *const c_void, dtlm, tfp);
        }
        s if s == ZSTD_dfast => {
            ZSTD_fillDoubleHashTable(ms, iend as *const c_void, dtlm, tfp);
        }
        s if s == ZSTD_greedy || s == ZSTD_lazy || s == ZSTD_lazy2 => {
            if (*ms).dedicatedDictSearch != 0 {
                ZSTD_dedicatedDictSearch_lazy_loadDictionary(
                    ms,
                    iend.wrapping_sub(HASH_READ_SIZE as usize),
                );
            } else {
                if (*params).useRowMatchFinder == ZSTD_ps_enable {
                    let tagTableSize: size_t = (1 as size_t) << (*params).cParams.hashLog;
                    ZSTD_memset((*ms).tagTable, 0, tagTableSize);
                    ZSTD_row_update(ms, iend.wrapping_sub(HASH_READ_SIZE as usize));
                } else {
                    ZSTD_insertAndFindFirstIndex(ms, iend.wrapping_sub(HASH_READ_SIZE as usize));
                }
            }
        }
        s if s == ZSTD_btlazy2 || s == ZSTD_btopt || s == ZSTD_btultra || s == ZSTD_btultra2 => {
            ZSTD_updateTree(ms, iend.wrapping_sub(HASH_READ_SIZE as usize), iend);
        }
        _ => { /* default : impossible */ }
    }

    (*ms).nextToUpdate = iend.offset_from((*ms).window.base) as U32;
    0
}

/* ZSTD_dictNCountRepeat */
unsafe fn ZSTD_dictNCountRepeat(
    normalizedCounter: *mut S16,
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
        s += 1;
    }
    FSE_repeat_valid
}

/* ================================================================== */
/*  ZSTD_loadCEntropy                                                  */
/* ================================================================== */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_loadCEntropy(
    bs: *mut ZSTD_compressedBlockState_t,
    workspace: *mut c_void,
    dict: *const c_void,
    dictSize: size_t,
) -> size_t {
    let mut offcodeNCount: [S16; (MaxOff + 1) as usize] = [0; (MaxOff + 1) as usize];
    let mut offcodeMaxValue: c_uint = MaxOff as c_uint;
    let mut dictPtr: *const BYTE = dict as *const BYTE; /* skip magic num and dict ID */
    let dictEnd: *const BYTE = dictPtr.wrapping_add(dictSize);
    dictPtr = dictPtr.wrapping_add(8);
    (*bs).entropy.huf.repeatMode = HUF_repeat_check;

    {
        let mut maxSymbolValue: c_uint = 255;
        let mut hasZeroWeights: c_uint = 1;
        let hufHeaderSize: size_t = HUF_readCTable(
            (*bs).entropy.huf.CTable.as_mut_ptr(),
            &mut maxSymbolValue,
            dictPtr as *const c_void,
            dictEnd.offset_from(dictPtr) as size_t,
            &mut hasZeroWeights,
        );

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
        let offcodeHeaderSize: size_t = FSE_readNCount(
            offcodeNCount.as_mut_ptr(),
            &mut offcodeMaxValue,
            &mut offcodeLog,
            dictPtr as *const c_void,
            dictEnd.offset_from(dictPtr) as size_t,
        );
        if FSE_isError(offcodeHeaderSize) != 0 {
            return ERROR(ZSTD_error_dictionary_corrupted);
        }
        if offcodeLog > OffFSELog as c_uint {
            return ERROR(ZSTD_error_dictionary_corrupted);
        }
        if FSE_isError(FSE_buildCTable_wksp(
            (*bs).entropy.fse.offcodeCTable.as_mut_ptr(),
            offcodeNCount.as_ptr(),
            MaxOff as c_uint,
            offcodeLog,
            workspace,
            HUF_WORKSPACE_SIZE,
        )) != 0
        {
            return ERROR(ZSTD_error_dictionary_corrupted);
        }
        dictPtr = dictPtr.wrapping_add(offcodeHeaderSize);
    }

    {
        let mut matchlengthNCount: [S16; (MaxML + 1) as usize] = [0; (MaxML + 1) as usize];
        let mut matchlengthMaxValue: c_uint = MaxML as c_uint;
        let mut matchlengthLog: c_uint = 0;
        let matchlengthHeaderSize: size_t = FSE_readNCount(
            matchlengthNCount.as_mut_ptr(),
            &mut matchlengthMaxValue,
            &mut matchlengthLog,
            dictPtr as *const c_void,
            dictEnd.offset_from(dictPtr) as size_t,
        );
        if FSE_isError(matchlengthHeaderSize) != 0 {
            return ERROR(ZSTD_error_dictionary_corrupted);
        }
        if matchlengthLog > MLFSELog as c_uint {
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
        (*bs).entropy.fse.matchlength_repeatMode =
            ZSTD_dictNCountRepeat(matchlengthNCount.as_mut_ptr(), matchlengthMaxValue, MaxML as c_uint);
        dictPtr = dictPtr.wrapping_add(matchlengthHeaderSize);
    }

    {
        let mut litlengthNCount: [S16; (MaxLL + 1) as usize] = [0; (MaxLL + 1) as usize];
        let mut litlengthMaxValue: c_uint = MaxLL as c_uint;
        let mut litlengthLog: c_uint = 0;
        let litlengthHeaderSize: size_t = FSE_readNCount(
            litlengthNCount.as_mut_ptr(),
            &mut litlengthMaxValue,
            &mut litlengthLog,
            dictPtr as *const c_void,
            dictEnd.offset_from(dictPtr) as size_t,
        );
        if FSE_isError(litlengthHeaderSize) != 0 {
            return ERROR(ZSTD_error_dictionary_corrupted);
        }
        if litlengthLog > LLFSELog as c_uint {
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
            ZSTD_dictNCountRepeat(litlengthNCount.as_mut_ptr(), litlengthMaxValue, MaxLL as c_uint);
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
        let dictContentSize: size_t = dictEnd.offset_from(dictPtr) as size_t;
        let mut offcodeMax: U32 = MaxOff as U32;
        if dictContentSize <= ((u32::MAX) as size_t) - 128 * (1 << 10) {
            let maxOffset: U32 = dictContentSize as U32 + 128 * (1 << 10); /* max offset to support */
            offcodeMax = ZSTD_highbit32(maxOffset);
        }
        (*bs).entropy.fse.offcode_repeatMode = ZSTD_dictNCountRepeat(
            offcodeNCount.as_mut_ptr(),
            offcodeMaxValue,
            MIN(offcodeMax, MaxOff as U32),
        );

        {
            let mut u: U32 = 0;
            while u < 3 {
                if (*bs).rep[u as usize] == 0 {
                    return ERROR(ZSTD_error_dictionary_corrupted);
                }
                if (*bs).rep[u as usize] as size_t > dictContentSize {
                    return ERROR(ZSTD_error_dictionary_corrupted);
                }
                u += 1;
            }
        }
    }

    dictPtr.offset_from(dict as *const BYTE) as size_t
}

/* ================================================================== */
/*  ZSTD_loadZstdDictionary                                            */
/* ================================================================== */
unsafe fn ZSTD_loadZstdDictionary(
    bs: *mut ZSTD_compressedBlockState_t,
    ms: *mut ZSTD_MatchState_t,
    ws: *mut ZSTD_cwksp,
    params: *const ZSTD_CCtx_params,
    dict: *const c_void,
    dictSize: size_t,
    dtlm: ZSTD_dictTableLoadMethod_e,
    tfp: ZSTD_tableFillPurpose_e,
    workspace: *mut c_void,
) -> size_t {
    let mut dictPtr: *const BYTE = dict as *const BYTE;
    let dictEnd: *const BYTE = dictPtr.wrapping_add(dictSize);
    let dictID: size_t;
    let eSize: size_t;

    dictID = if (*params).fParams.noDictIDFlag != 0 {
        0
    } else {
        MEM_readLE32((dictPtr.wrapping_add(4)) as *const u8) as size_t
    };
    eSize = ZSTD_loadCEntropy(bs, workspace, dict, dictSize);
    if ERR_isError(eSize) != 0 {
        return eSize;
    }
    dictPtr = dictPtr.wrapping_add(eSize);

    {
        let dictContentSize: size_t = dictEnd.offset_from(dictPtr) as size_t;
        let err = ZSTD_loadDictionaryContent(
            ms,
            null_mut(),
            ws,
            params,
            dictPtr as *const c_void,
            dictContentSize,
            dtlm,
            tfp,
        );
        if ERR_isError(err) != 0 {
            return err;
        }
    }
    dictID
}

/* ================================================================== */
/*  ZSTD_compress_insertDictionary                                     */
/* ================================================================== */
unsafe fn ZSTD_compress_insertDictionary(
    bs: *mut ZSTD_compressedBlockState_t,
    ms: *mut ZSTD_MatchState_t,
    ls: *mut ldmState_t,
    ws: *mut ZSTD_cwksp,
    params: *const ZSTD_CCtx_params,
    dict: *const c_void,
    dictSize: size_t,
    dictContentType: ZSTD_dictContentType_e,
    dtlm: ZSTD_dictTableLoadMethod_e,
    tfp: ZSTD_tableFillPurpose_e,
    workspace: *mut c_void,
) -> size_t {
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

    if MEM_readLE32(dict as *const u8) != ZSTD_MAGIC_DICTIONARY {
        if dictContentType == ZSTD_dct_auto {
            return ZSTD_loadDictionaryContent(ms, ls, ws, params, dict, dictSize, dtlm, tfp);
        }
        if dictContentType == ZSTD_dct_fullDict {
            return ERROR(ZSTD_error_dictionary_wrong);
        }
        /* assert(0) : impossible */
    }

    /* dict as full zstd dictionary */
    ZSTD_loadZstdDictionary(bs, ms, ws, params, dict, dictSize, dtlm, tfp, workspace)
}

/* ================================================================== */
/*  ZSTD_compressBegin_internal and friends                            */
/* ================================================================== */
unsafe fn ZSTD_compressBegin_internal(
    cctx: *mut ZSTD_CCtx,
    dict: *const c_void,
    dictSize: size_t,
    dictContentType: ZSTD_dictContentType_e,
    dtlm: ZSTD_dictTableLoadMethod_e,
    cdict: *const ZSTD_CDict,
    params: *const ZSTD_CCtx_params,
    pledgedSrcSize: U64,
    zbuff: ZSTD_buffered_policy_e,
) -> size_t {
    let dictContentSize: size_t = if !cdict.is_null() {
        (*cdl(cdict)).dictContentSize
    } else {
        dictSize
    };
    /* ZSTD_TRACE: trace_compress_begin is an undefined weak symbol (NULL),
     * so this evaluates to 0 (no-op). */
    (*cctx).traceCtx = 0;

    if !cdict.is_null()
        && (*cdl(cdict)).dictContentSize > 0
        && (pledgedSrcSize < ZSTD_USE_CDICT_PARAMS_SRCSIZE_CUTOFF
            || pledgedSrcSize
                < (*cdl(cdict)).dictContentSize as U64 * ZSTD_USE_CDICT_PARAMS_DICTSIZE_MULTIPLIER
            || pledgedSrcSize == ZSTD_CONTENTSIZE_UNKNOWN
            || (*cdl(cdict)).compressionLevel == 0)
        && (*params).attachDictPref != ZSTD_dictForceLoad
    {
        return ZSTD_resetCCtx_usingCDict(cctx, cdict, params, pledgedSrcSize, zbuff);
    }

    {
        let err = ZSTD_resetCCtx_internal(
            cctx,
            params,
            pledgedSrcSize,
            dictContentSize,
            ZSTDcrp_makeClean,
            zbuff,
        );
        if ERR_isError(err) != 0 {
            return err;
        }
    }
    {
        let dictID: size_t = if !cdict.is_null() {
            ZSTD_compress_insertDictionary(
                (*cctx).blockState.prevCBlock,
                &mut (*cctx).blockState.matchState,
                &mut (*cctx).ldmState,
                &mut (*cctx).workspace,
                &(*cctx).appliedParams,
                (*cdl(cdict)).dictContent,
                (*cdl(cdict)).dictContentSize,
                (*cdl(cdict)).dictContentType,
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
    dictSize: size_t,
    dictContentType: ZSTD_dictContentType_e,
    dtlm: ZSTD_dictTableLoadMethod_e,
    cdict: *const ZSTD_CDict,
    params: *const ZSTD_CCtx_params,
    pledgedSrcSize: core::ffi::c_ulonglong,
) -> size_t {
    {
        let err = ZSTD_checkCParams((*params).cParams);
        if ERR_isError(err) != 0 {
            return err;
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBegin_advanced(
    cctx: *mut ZSTD_CCtx,
    dict: *const c_void,
    dictSize: size_t,
    params: ZSTD_parameters,
    pledgedSrcSize: core::ffi::c_ulonglong,
) -> size_t {
    let mut cctxParams: ZSTD_CCtx_params = core::mem::zeroed();
    ZSTD_CCtxParams_init_internal(&mut cctxParams, &params, ZSTD_NO_CLEVEL);
    ZSTD_compressBegin_advanced_internal(
        cctx,
        dict,
        dictSize,
        ZSTD_dct_auto,
        ZSTD_dtlm_fast,
        null(), /*cdict*/
        &cctxParams,
        pledgedSrcSize,
    )
}

unsafe fn ZSTD_compressBegin_usingDict_deprecated(
    cctx: *mut ZSTD_CCtx,
    dict: *const c_void,
    dictSize: size_t,
    compressionLevel: c_int,
) -> size_t {
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
        null(),
        &cctxParams,
        ZSTD_CONTENTSIZE_UNKNOWN,
        ZSTDb_not_buffered,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBegin_usingDict(
    cctx: *mut ZSTD_CCtx,
    dict: *const c_void,
    dictSize: size_t,
    compressionLevel: c_int,
) -> size_t {
    ZSTD_compressBegin_usingDict_deprecated(cctx, dict, dictSize, compressionLevel)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBegin(cctx: *mut ZSTD_CCtx, compressionLevel: c_int) -> size_t {
    ZSTD_compressBegin_usingDict_deprecated(cctx, null(), 0, compressionLevel)
}

/* ================================================================== */
/*  ZSTD_writeEpilogue                                                 */
/* ================================================================== */
unsafe fn ZSTD_writeEpilogue(cctx: *mut ZSTD_CCtx, dst: *mut c_void, mut dstCapacity: size_t) -> size_t {
    let ostart: *mut BYTE = dst as *mut BYTE;
    let mut op: *mut BYTE = ostart;

    if (*cctx).stage == ZSTDcs_created {
        return ERROR(ZSTD_error_stage_wrong);
    }

    /* special case : empty frame */
    if (*cctx).stage == ZSTDcs_init {
        let fhSize: size_t = ZSTD_writeFrameHeader(dst, dstCapacity, &(*cctx).appliedParams, 0, 0);
        if ERR_isError(fhSize) != 0 {
            return fhSize;
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

    (*cctx).stage = ZSTDcs_created; /* return to "created but no init" status */
    op.offset_from(ostart) as size_t
}

/* ================================================================== */
/*  ZSTD_CCtx_trace : exported symbol, body is a no-op (weak trace     */
/*  symbols are undefined/NULL in this build).                         */
/* ================================================================== */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_CCtx_trace(cctx: *mut ZSTD_CCtx, extraCSize: size_t) {
    /* ZSTD_TRACE==1 but ZSTD_trace_compress_end is an undefined weak symbol,
     * so the guard `cctx->traceCtx && ZSTD_trace_compress_end != NULL` is
     * always false. The only observable side effect is clearing traceCtx. */
    let _ = extraCSize;
    (*cctx).traceCtx = 0;
}

/* ================================================================== */
/*  ZSTD_compressEnd_public / ZSTD_compressEnd                         */
/* ================================================================== */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressEnd_public(
    cctx: *mut ZSTD_CCtx,
    dst: *mut c_void,
    dstCapacity: size_t,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    let endResult: size_t;
    let cSize: size_t = ZSTD_compressContinue_internal(
        cctx, dst, dstCapacity, src, srcSize, 1, /* frame mode */
        1, /* last chunk */
    );
    if ERR_isError(cSize) != 0 {
        return cSize;
    }
    endResult = ZSTD_writeEpilogue(
        cctx,
        (dst as *mut c_char).wrapping_add(cSize) as *mut c_void,
        dstCapacity - cSize,
    );
    if ERR_isError(endResult) != 0 {
        return endResult;
    }
    if (*cctx).pledgedSrcSizePlusOne != 0 {
        /* control src size */
        if (*cctx).pledgedSrcSizePlusOne != (*cctx).consumedSrcSize + 1 {
            return ERROR(ZSTD_error_srcSize_wrong);
        }
    }
    ZSTD_CCtx_trace(cctx, endResult);
    cSize + endResult
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressEnd(
    cctx: *mut ZSTD_CCtx,
    dst: *mut c_void,
    dstCapacity: size_t,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    ZSTD_compressEnd_public(cctx, dst, dstCapacity, src, srcSize)
}

/* ================================================================== */
/*  ZSTD_compress_advanced and simple compress functions               */
/* ================================================================== */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compress_advanced(
    cctx: *mut ZSTD_CCtx,
    dst: *mut c_void,
    dstCapacity: size_t,
    src: *const c_void,
    srcSize: size_t,
    dict: *const c_void,
    dictSize: size_t,
    params: ZSTD_parameters,
) -> size_t {
    {
        let err = ZSTD_checkCParams(params.cParams);
        if ERR_isError(err) != 0 {
            return err;
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compress_advanced_internal(
    cctx: *mut ZSTD_CCtx,
    dst: *mut c_void,
    dstCapacity: size_t,
    src: *const c_void,
    srcSize: size_t,
    dict: *const c_void,
    dictSize: size_t,
    params: *const ZSTD_CCtx_params,
) -> size_t {
    {
        let err = ZSTD_compressBegin_internal(
            cctx,
            dict,
            dictSize,
            ZSTD_dct_auto,
            ZSTD_dtlm_fast,
            null(),
            params,
            srcSize as U64,
            ZSTDb_not_buffered,
        );
        if ERR_isError(err) != 0 {
            return err;
        }
    }
    ZSTD_compressEnd_public(cctx, dst, dstCapacity, src, srcSize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compress_usingDict(
    cctx: *mut ZSTD_CCtx,
    dst: *mut c_void,
    dstCapacity: size_t,
    src: *const c_void,
    srcSize: size_t,
    dict: *const c_void,
    dictSize: size_t,
    compressionLevel: c_int,
) -> size_t {
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
    dstCapacity: size_t,
    src: *const c_void,
    srcSize: size_t,
    compressionLevel: c_int,
) -> size_t {
    ZSTD_compress_usingDict(cctx, dst, dstCapacity, src, srcSize, null(), 0, compressionLevel)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compress(
    dst: *mut c_void,
    dstCapacity: size_t,
    src: *const c_void,
    srcSize: size_t,
    compressionLevel: c_int,
) -> size_t {
    let result: size_t;
    /* ZSTD_COMPRESS_HEAPMODE is 0 by default: stack CCtx. */
    let mut ctxBody: ZSTD_CCtx = core::mem::zeroed();
    ZSTD_initCCtx(&mut ctxBody, ZSTD_defaultCMem);
    result = ZSTD_compressCCtx(&mut ctxBody, dst, dstCapacity, src, srcSize, compressionLevel);
    ZSTD_freeCCtxContent(&mut ctxBody); /* free only heap content */
    result
}

/* =====  Dictionary API  ===== */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_estimateCDictSize_advanced(
    dictSize: size_t,
    cParams: ZSTD_compressionParameters,
    dictLoadMethod: ZSTD_dictLoadMethod_e,
) -> size_t {
    ZSTD_cwksp_alloc_size(core::mem::size_of::<ZSTD_CDict_s_layout>() as size_t)
        + ZSTD_cwksp_alloc_size(HUF_WORKSPACE_SIZE)
        + ZSTD_sizeof_matchState(
            &cParams,
            ZSTD_resolveRowMatchFinderMode(ZSTD_ps_auto, &cParams),
            /* enableDedicatedDictSearch */ 1,
            /* forCCtx */ 0,
        )
        + (if dictLoadMethod == ZSTD_dlm_byRef {
            0
        } else {
            ZSTD_cwksp_alloc_size(ZSTD_cwksp_align(dictSize, core::mem::size_of::<*mut c_void>() as size_t))
        })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_estimateCDictSize(dictSize: size_t, compressionLevel: c_int) -> size_t {
    let cParams: ZSTD_compressionParameters = ZSTD_getCParams_internal(
        compressionLevel,
        ZSTD_CONTENTSIZE_UNKNOWN,
        dictSize,
        ZSTD_cpm_createCDict,
    );
    ZSTD_estimateCDictSize_advanced(dictSize, cParams, ZSTD_dlm_byCopy)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_sizeof_CDict(cdict: *const ZSTD_CDict) -> size_t {
    if cdict.is_null() {
        return 0; /* support sizeof on NULL */
    }
    /* cdict may be in the workspace */
    (if (*cdl(cdict)).workspace.workspace == cdict as *mut c_void {
        0
    } else {
        core::mem::size_of::<ZSTD_CDict_s_layout>() as size_t
    }) + ZSTD_cwksp_sizeof(&(*cdl(cdict)).workspace)
}

unsafe fn ZSTD_initCDict_internal(
    cdict: *mut ZSTD_CDict,
    dictBuffer: *const c_void,
    dictSize: size_t,
    dictLoadMethod: ZSTD_dictLoadMethod_e,
    dictContentType: ZSTD_dictContentType_e,
    mut params: ZSTD_CCtx_params,
) -> size_t {
    let cd: *mut ZSTD_CDict_s_layout = cdl_mut(cdict);
    (*cd).matchState.cParams = params.cParams;
    (*cd).matchState.dedicatedDictSearch = params.enableDedicatedDictSearch;
    if dictLoadMethod == ZSTD_dlm_byRef || dictBuffer.is_null() || dictSize == 0 {
        (*cd).dictContent = dictBuffer;
    } else {
        let internalBuffer: *mut c_void = ZSTD_cwksp_reserve_object(
            &mut (*cd).workspace,
            ZSTD_cwksp_align(dictSize, core::mem::size_of::<*mut c_void>() as size_t),
        );
        if internalBuffer.is_null() {
            return ERROR(ZSTD_error_memory_allocation);
        }
        (*cd).dictContent = internalBuffer;
        ZSTD_memcpy(internalBuffer as *mut u8, dictBuffer as *const u8, dictSize);
    }
    (*cd).dictContentSize = dictSize;
    (*cd).dictContentType = dictContentType;

    (*cd).entropyWorkspace =
        ZSTD_cwksp_reserve_object(&mut (*cd).workspace, HUF_WORKSPACE_SIZE) as *mut U32;

    /* Reset the state to no dictionary */
    ZSTD_reset_compressedBlockState(&mut (*cd).cBlockState);
    {
        let err = ZSTD_reset_matchState(
            &mut (*cd).matchState,
            &mut (*cd).workspace,
            &params.cParams,
            params.useRowMatchFinder,
            ZSTDcrp_makeClean,
            ZSTDirp_reset,
            ZSTD_resetTarget_CDict,
        );
        if ERR_isError(err) != 0 {
            return err;
        }
    }
    {
        params.compressionLevel = ZSTD_CLEVEL_DEFAULT;
        params.fParams.contentSizeFlag = 1;
        {
            let dictID: size_t = ZSTD_compress_insertDictionary(
                &mut (*cd).cBlockState,
                &mut (*cd).matchState,
                null_mut(),
                &mut (*cd).workspace,
                &params,
                (*cd).dictContent,
                (*cd).dictContentSize,
                dictContentType,
                ZSTD_dtlm_full,
                ZSTD_tfp_forCDict,
                (*cd).entropyWorkspace as *mut c_void,
            );
            if ERR_isError(dictID) != 0 {
                return dictID;
            }
            (*cd).dictID = dictID as U32;
        }
    }

    0
}

unsafe fn ZSTD_createCDict_advanced_internal(
    dictSize: size_t,
    dictLoadMethod: ZSTD_dictLoadMethod_e,
    cParams: ZSTD_compressionParameters,
    useRowMatchFinder: ZSTD_ParamSwitch_e,
    enableDedicatedDictSearch: c_int,
    customMem: ZSTD_customMem,
) -> *mut ZSTD_CDict {
    if (customMem.customAlloc.is_none()) ^ (customMem.customFree.is_none()) {
        return null_mut();
    }

    {
        let workspaceSize: size_t = ZSTD_cwksp_alloc_size(core::mem::size_of::<ZSTD_CDict_s_layout>() as size_t)
            + ZSTD_cwksp_alloc_size(HUF_WORKSPACE_SIZE)
            + ZSTD_sizeof_matchState(&cParams, useRowMatchFinder, enableDedicatedDictSearch, 0)
            + (if dictLoadMethod == ZSTD_dlm_byRef {
                0
            } else {
                ZSTD_cwksp_alloc_size(ZSTD_cwksp_align(
                    dictSize,
                    core::mem::size_of::<*mut c_void>() as size_t,
                ))
            });
        let workspace: *mut c_void = ZSTD_customMalloc(workspaceSize, customMem);
        let mut ws: ZSTD_cwksp = core::mem::zeroed();
        let cdict: *mut ZSTD_CDict;

        if workspace.is_null() {
            ZSTD_customFree(workspace, customMem);
            return null_mut();
        }

        ZSTD_cwksp_init(&mut ws, workspace, workspaceSize, ZSTD_cwksp_dynamic_alloc);

        cdict = ZSTD_cwksp_reserve_object(&mut ws, core::mem::size_of::<ZSTD_CDict_s_layout>() as size_t)
            as *mut ZSTD_CDict;
        if cdict.is_null() {
            /* assert(cdict != NULL) : allocation cannot fail at object phase since
             * we sized it exactly; keep behaviour identical to C (no NULL check). */
        }
        let cd: *mut ZSTD_CDict_s_layout = cdl_mut(cdict);
        ZSTD_cwksp_move(&mut (*cd).workspace, &mut ws);
        (*cd).customMem = customMem;
        (*cd).compressionLevel = ZSTD_NO_CLEVEL; /* signals advanced API usage */
        (*cd).useRowMatchFinder = useRowMatchFinder;
        cdict
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_createCDict_advanced(
    dictBuffer: *const c_void,
    dictSize: size_t,
    dictLoadMethod: ZSTD_dictLoadMethod_e,
    dictContentType: ZSTD_dictContentType_e,
    cParams: ZSTD_compressionParameters,
    customMem: ZSTD_customMem,
) -> *mut ZSTD_CDict {
    let mut cctxParams: ZSTD_CCtx_params = core::mem::zeroed();
    ZSTD_memset(
        &mut cctxParams as *mut ZSTD_CCtx_params as *mut u8,
        0,
        core::mem::size_of::<ZSTD_CCtx_params>() as size_t,
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
    dictSize: size_t,
    dictLoadMethod: ZSTD_dictLoadMethod_e,
    dictContentType: ZSTD_dictContentType_e,
    originalCctxParams: *const ZSTD_CCtx_params,
    customMem: ZSTD_customMem,
) -> *mut ZSTD_CDict {
    let mut cctxParams: ZSTD_CCtx_params = core::ptr::read(originalCctxParams);
    let mut cParams: ZSTD_compressionParameters;
    let cdict: *mut ZSTD_CDict;

    if (customMem.customAlloc.is_none()) ^ (customMem.customFree.is_none()) {
        return null_mut();
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
        || ZSTD_isError(ZSTD_initCDict_internal(
            cdict,
            dict,
            dictSize,
            dictLoadMethod,
            dictContentType,
            core::ptr::read(&cctxParams),
        )) != 0
    {
        ZSTD_freeCDict(cdict);
        return null_mut();
    }

    cdict
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_createCDict(
    dict: *const c_void,
    dictSize: size_t,
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
        (*cdl_mut(cdict)).compressionLevel = if compressionLevel == 0 {
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
    dictSize: size_t,
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
        (*cdl_mut(cdict)).compressionLevel = if compressionLevel == 0 {
            ZSTD_CLEVEL_DEFAULT
        } else {
            compressionLevel
        };
    }
    cdict
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_freeCDict(cdict: *mut ZSTD_CDict) -> size_t {
    if cdict.is_null() {
        return 0; /* support free on NULL */
    }
    {
        let cMem: ZSTD_customMem = (*cdl(cdict)).customMem;
        let cdictInWorkspace: c_int =
            ZSTD_cwksp_owns_buffer(&(*cdl(cdict)).workspace, cdict as *const c_void);
        ZSTD_cwksp_free(&mut (*cdl_mut(cdict)).workspace, cMem);
        if cdictInWorkspace == 0 {
            ZSTD_customFree(cdict as *mut c_void, cMem);
        }
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_initStaticCDict(
    workspace: *mut c_void,
    workspaceSize: size_t,
    dict: *const c_void,
    dictSize: size_t,
    dictLoadMethod: ZSTD_dictLoadMethod_e,
    dictContentType: ZSTD_dictContentType_e,
    cParams: ZSTD_compressionParameters,
) -> *const ZSTD_CDict {
    let useRowMatchFinder: ZSTD_ParamSwitch_e =
        ZSTD_resolveRowMatchFinderMode(ZSTD_ps_auto, &cParams);
    let matchStateSize: size_t =
        ZSTD_sizeof_matchState(&cParams, useRowMatchFinder, 1, 0);
    let neededSize: size_t = ZSTD_cwksp_alloc_size(core::mem::size_of::<ZSTD_CDict_s_layout>() as size_t)
        + (if dictLoadMethod == ZSTD_dlm_byRef {
            0
        } else {
            ZSTD_cwksp_alloc_size(ZSTD_cwksp_align(
                dictSize,
                core::mem::size_of::<*mut c_void>() as size_t,
            ))
        })
        + ZSTD_cwksp_alloc_size(HUF_WORKSPACE_SIZE)
        + matchStateSize;
    let cdict: *mut ZSTD_CDict;
    let mut params: ZSTD_CCtx_params = core::mem::zeroed();

    if (workspace as size_t) & 7 != 0 {
        return null(); /* 8-aligned */
    }

    {
        let mut ws: ZSTD_cwksp = core::mem::zeroed();
        ZSTD_cwksp_init(&mut ws, workspace, workspaceSize, ZSTD_cwksp_static_alloc);
        cdict = ZSTD_cwksp_reserve_object(&mut ws, core::mem::size_of::<ZSTD_CDict_s_layout>() as size_t)
            as *mut ZSTD_CDict;
        if cdict.is_null() {
            return null();
        }
        ZSTD_cwksp_move(&mut (*cdl_mut(cdict)).workspace, &mut ws);
    }

    if workspaceSize < neededSize {
        return null();
    }

    ZSTD_CCtxParams_init(&mut params, 0);
    params.cParams = cParams;
    params.useRowMatchFinder = useRowMatchFinder;
    (*cdl_mut(cdict)).useRowMatchFinder = useRowMatchFinder;
    (*cdl_mut(cdict)).compressionLevel = ZSTD_NO_CLEVEL;

    if ZSTD_isError(ZSTD_initCDict_internal(
        cdict,
        dict,
        dictSize,
        dictLoadMethod,
        dictContentType,
        params,
    )) != 0
    {
        return null();
    }

    cdict
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_getCParamsFromCDict(
    cdict: *const ZSTD_CDict,
) -> ZSTD_compressionParameters {
    (*cdl(cdict)).matchState.cParams
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_getDictID_fromCDict(cdict: *const ZSTD_CDict) -> c_uint {
    if cdict.is_null() {
        return 0;
    }
    (*cdl(cdict)).dictID
}

/* ================================================================== */
/*  compressBegin/compress usingCDict                                  */
/* ================================================================== */
unsafe fn ZSTD_compressBegin_usingCDict_internal(
    cctx: *mut ZSTD_CCtx,
    cdict: *const ZSTD_CDict,
    fParams: ZSTD_frameParameters,
    pledgedSrcSize: core::ffi::c_ulonglong,
) -> size_t {
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
                < (*cdl(cdict)).dictContentSize as U64 * ZSTD_USE_CDICT_PARAMS_DICTSIZE_MULTIPLIER
            || pledgedSrcSize == ZSTD_CONTENTSIZE_UNKNOWN
            || (*cdl(cdict)).compressionLevel == 0
        {
            ZSTD_getCParamsFromCDict(cdict)
        } else {
            ZSTD_getCParams(
                (*cdl(cdict)).compressionLevel,
                pledgedSrcSize,
                (*cdl(cdict)).dictContentSize,
            )
        };
        ZSTD_CCtxParams_init_internal(&mut cctxParams, &params, (*cdl(cdict)).compressionLevel);
    }
    /* Increase window log to fit the entire dictionary and source if the
     * source size is known. Limit the increase to 19. */
    if pledgedSrcSize != ZSTD_CONTENTSIZE_UNKNOWN {
        let limitedSrcSize: U32 = MIN(pledgedSrcSize, 1u64 << 19) as U32;
        let limitedSrcLog: U32 = if limitedSrcSize > 1 {
            ZSTD_highbit32(limitedSrcSize - 1) + 1
        } else {
            1
        };
        cctxParams.cParams.windowLog = MAX(cctxParams.cParams.windowLog, limitedSrcLog);
    }
    ZSTD_compressBegin_internal(
        cctx,
        null(),
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
    pledgedSrcSize: core::ffi::c_ulonglong,
) -> size_t {
    ZSTD_compressBegin_usingCDict_internal(cctx, cdict, fParams, pledgedSrcSize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressBegin_usingCDict_deprecated(
    cctx: *mut ZSTD_CCtx,
    cdict: *const ZSTD_CDict,
) -> size_t {
    let fParams: ZSTD_frameParameters = ZSTD_frameParameters {
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
) -> size_t {
    ZSTD_compressBegin_usingCDict_deprecated(cctx, cdict)
}

unsafe fn ZSTD_compress_usingCDict_internal(
    cctx: *mut ZSTD_CCtx,
    dst: *mut c_void,
    dstCapacity: size_t,
    src: *const c_void,
    srcSize: size_t,
    cdict: *const ZSTD_CDict,
    fParams: ZSTD_frameParameters,
) -> size_t {
    {
        let err = ZSTD_compressBegin_usingCDict_internal(cctx, cdict, fParams, srcSize as U64);
        if ERR_isError(err) != 0 {
            return err;
        }
    }
    ZSTD_compressEnd_public(cctx, dst, dstCapacity, src, srcSize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compress_usingCDict_advanced(
    cctx: *mut ZSTD_CCtx,
    dst: *mut c_void,
    dstCapacity: size_t,
    src: *const c_void,
    srcSize: size_t,
    cdict: *const ZSTD_CDict,
    fParams: ZSTD_frameParameters,
) -> size_t {
    ZSTD_compress_usingCDict_internal(cctx, dst, dstCapacity, src, srcSize, cdict, fParams)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compress_usingCDict(
    cctx: *mut ZSTD_CCtx,
    dst: *mut c_void,
    dstCapacity: size_t,
    src: *const c_void,
    srcSize: size_t,
    cdict: *const ZSTD_CDict,
) -> size_t {
    let fParams: ZSTD_frameParameters = ZSTD_frameParameters {
        contentSizeFlag: 1,
        checksumFlag: 0,
        noDictIDFlag: 0,
    };
    ZSTD_compress_usingCDict_internal(cctx, dst, dstCapacity, src, srcSize, cdict, fParams)
}

/* ******************************************************************
*  Streaming
********************************************************************/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_createCStream() -> *mut ZSTD_CStream {
    ZSTD_createCStream_advanced(ZSTD_defaultCMem)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_initStaticCStream(
    workspace: *mut c_void,
    workspaceSize: size_t,
) -> *mut ZSTD_CStream {
    ZSTD_initStaticCCtx(workspace, workspaceSize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_createCStream_advanced(customMem: ZSTD_customMem) -> *mut ZSTD_CStream {
    ZSTD_createCCtx_advanced(customMem)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_freeCStream(zcs: *mut ZSTD_CStream) -> size_t {
    ZSTD_freeCCtx(zcs)
}

/*======   Initialization   ======*/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_CStreamInSize() -> size_t {
    ZSTD_BLOCKSIZE_MAX as size_t
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_CStreamOutSize() -> size_t {
    ZSTD_compressBound(ZSTD_BLOCKSIZE_MAX as size_t) + ZSTD_blockHeaderSize + 4 /* 32-bits hash */
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
pub unsafe extern "C" fn ZSTD_resetCStream(
    zcs: *mut ZSTD_CStream,
    pss: core::ffi::c_ulonglong,
) -> size_t {
    let pledgedSrcSize: U64 = if pss == 0 { ZSTD_CONTENTSIZE_UNKNOWN } else { pss };
    {
        let err = ZSTD_CCtx_reset(zcs, ZSTD_reset_session_only);
        if ERR_isError(err) != 0 {
            return err;
        }
    }
    {
        let err = ZSTD_CCtx_setPledgedSrcSize(zcs, pledgedSrcSize);
        if ERR_isError(err) != 0 {
            return err;
        }
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_initCStream_internal(
    zcs: *mut ZSTD_CStream,
    dict: *const c_void,
    dictSize: size_t,
    cdict: *const ZSTD_CDict,
    params: *const ZSTD_CCtx_params,
    pledgedSrcSize: core::ffi::c_ulonglong,
) -> size_t {
    {
        let err = ZSTD_CCtx_reset(zcs, ZSTD_reset_session_only);
        if ERR_isError(err) != 0 {
            return err;
        }
    }
    {
        let err = ZSTD_CCtx_setPledgedSrcSize(zcs, pledgedSrcSize);
        if ERR_isError(err) != 0 {
            return err;
        }
    }
    (*zcs).requestedParams = core::ptr::read(params);
    if !dict.is_null() {
        let err = ZSTD_CCtx_loadDictionary(zcs, dict, dictSize);
        if ERR_isError(err) != 0 {
            return err;
        }
    } else {
        /* Dictionary is cleared if !cdict */
        let err = ZSTD_CCtx_refCDict(zcs, cdict);
        if ERR_isError(err) != 0 {
            return err;
        }
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_initCStream_usingCDict_advanced(
    zcs: *mut ZSTD_CStream,
    cdict: *const ZSTD_CDict,
    fParams: ZSTD_frameParameters,
    pledgedSrcSize: core::ffi::c_ulonglong,
) -> size_t {
    {
        let err = ZSTD_CCtx_reset(zcs, ZSTD_reset_session_only);
        if ERR_isError(err) != 0 {
            return err;
        }
    }
    {
        let err = ZSTD_CCtx_setPledgedSrcSize(zcs, pledgedSrcSize);
        if ERR_isError(err) != 0 {
            return err;
        }
    }
    (*zcs).requestedParams.fParams = fParams;
    {
        let err = ZSTD_CCtx_refCDict(zcs, cdict);
        if ERR_isError(err) != 0 {
            return err;
        }
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_initCStream_usingCDict(
    zcs: *mut ZSTD_CStream,
    cdict: *const ZSTD_CDict,
) -> size_t {
    {
        let err = ZSTD_CCtx_reset(zcs, ZSTD_reset_session_only);
        if ERR_isError(err) != 0 {
            return err;
        }
    }
    {
        let err = ZSTD_CCtx_refCDict(zcs, cdict);
        if ERR_isError(err) != 0 {
            return err;
        }
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_initCStream_advanced(
    zcs: *mut ZSTD_CStream,
    dict: *const c_void,
    dictSize: size_t,
    params: ZSTD_parameters,
    pss: core::ffi::c_ulonglong,
) -> size_t {
    let pledgedSrcSize: U64 =
        if pss == 0 && params.fParams.contentSizeFlag == 0 {
            ZSTD_CONTENTSIZE_UNKNOWN
        } else {
            pss
        };
    {
        let err = ZSTD_CCtx_reset(zcs, ZSTD_reset_session_only);
        if ERR_isError(err) != 0 {
            return err;
        }
    }
    {
        let err = ZSTD_CCtx_setPledgedSrcSize(zcs, pledgedSrcSize);
        if ERR_isError(err) != 0 {
            return err;
        }
    }
    {
        let err = ZSTD_checkCParams(params.cParams);
        if ERR_isError(err) != 0 {
            return err;
        }
    }
    ZSTD_CCtxParams_setZstdParams(&mut (*zcs).requestedParams, &params);
    {
        let err = ZSTD_CCtx_loadDictionary(zcs, dict, dictSize);
        if ERR_isError(err) != 0 {
            return err;
        }
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_initCStream_usingDict(
    zcs: *mut ZSTD_CStream,
    dict: *const c_void,
    dictSize: size_t,
    compressionLevel: c_int,
) -> size_t {
    {
        let err = ZSTD_CCtx_reset(zcs, ZSTD_reset_session_only);
        if ERR_isError(err) != 0 {
            return err;
        }
    }
    {
        let err = ZSTD_CCtx_setParameter(zcs, ZSTD_c_compressionLevel, compressionLevel);
        if ERR_isError(err) != 0 {
            return err;
        }
    }
    {
        let err = ZSTD_CCtx_loadDictionary(zcs, dict, dictSize);
        if ERR_isError(err) != 0 {
            return err;
        }
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_initCStream_srcSize(
    zcs: *mut ZSTD_CStream,
    compressionLevel: c_int,
    pss: core::ffi::c_ulonglong,
) -> size_t {
    let pledgedSrcSize: U64 = if pss == 0 { ZSTD_CONTENTSIZE_UNKNOWN } else { pss };
    {
        let err = ZSTD_CCtx_reset(zcs, ZSTD_reset_session_only);
        if ERR_isError(err) != 0 {
            return err;
        }
    }
    {
        let err = ZSTD_CCtx_refCDict(zcs, null());
        if ERR_isError(err) != 0 {
            return err;
        }
    }
    {
        let err = ZSTD_CCtx_setParameter(zcs, ZSTD_c_compressionLevel, compressionLevel);
        if ERR_isError(err) != 0 {
            return err;
        }
    }
    {
        let err = ZSTD_CCtx_setPledgedSrcSize(zcs, pledgedSrcSize);
        if ERR_isError(err) != 0 {
            return err;
        }
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_initCStream(zcs: *mut ZSTD_CStream, compressionLevel: c_int) -> size_t {
    {
        let err = ZSTD_CCtx_reset(zcs, ZSTD_reset_session_only);
        if ERR_isError(err) != 0 {
            return err;
        }
    }
    {
        let err = ZSTD_CCtx_refCDict(zcs, null());
        if ERR_isError(err) != 0 {
            return err;
        }
    }
    {
        let err = ZSTD_CCtx_setParameter(zcs, ZSTD_c_compressionLevel, compressionLevel);
        if ERR_isError(err) != 0 {
            return err;
        }
    }
    0
}

/*======   Compression   ======*/

unsafe fn ZSTD_nextInputSizeHint(cctx: *const ZSTD_CCtx) -> size_t {
    if (*cctx).appliedParams.inBufferMode == ZSTD_bm_stable {
        return (*cctx).blockSizeMax - (*cctx).stableIn_notConsumed;
    }
    {
        let mut hintInSize: size_t = (*cctx).inBuffTarget - (*cctx).inBuffPos;
        if hintInSize == 0 {
            hintInSize = (*cctx).blockSizeMax;
        }
        hintInSize
    }
}

/* ================================================================== */
/*  ZSTD_compressStream_generic : the zcss state machine               */
/* ================================================================== */
unsafe fn ZSTD_compressStream_generic(
    zcs: *mut ZSTD_CStream,
    output: *mut ZSTD_outBuffer,
    input: *mut ZSTD_inBuffer,
    flushMode: ZSTD_EndDirective,
) -> size_t {
    let istart: *const c_char = (*input).src as *const c_char;
    let iend: *const c_char = if !istart.is_null() {
        istart.wrapping_add((*input).size)
    } else {
        istart
    };
    let mut ip: *const c_char = if !istart.is_null() {
        istart.wrapping_add((*input).pos)
    } else {
        istart
    };
    let ostart: *mut c_char = (*output).dst as *mut c_char;
    let oend: *mut c_char = if !ostart.is_null() {
        ostart.wrapping_add((*output).size)
    } else {
        ostart
    };
    let mut op: *mut c_char = if !ostart.is_null() {
        ostart.wrapping_add((*output).pos)
    } else {
        ostart
    };
    let mut someMoreWork: U32 = 1;

    /* check expectations */
    if (*zcs).appliedParams.inBufferMode == ZSTD_bm_stable {
        (*input).pos -= (*zcs).stableIn_notConsumed;
        if !ip.is_null() {
            ip = ip.wrapping_sub((*zcs).stableIn_notConsumed);
        }
        (*zcs).stableIn_notConsumed = 0;
    }

    while someMoreWork != 0 {
        match (*zcs).streamStage {
            s if s == zcss_init => {
                return ERROR(ZSTD_error_init_missing);
            }

            s if s == zcss_load => {
                if flushMode == ZSTD_e_end
                    && ((oend.offset_from(op) as size_t)
                        >= ZSTD_compressBound(iend.offset_from(ip) as size_t)
                        || (*zcs).appliedParams.outBufferMode == ZSTD_bm_stable)
                    && (*zcs).inBuffPos == 0
                {
                    /* shortcut to compression pass directly into output buffer */
                    let cSize: size_t = ZSTD_compressEnd_public(
                        zcs,
                        op as *mut c_void,
                        oend.offset_from(op) as size_t,
                        ip as *const c_void,
                        iend.offset_from(ip) as size_t,
                    );
                    if ERR_isError(cSize) != 0 {
                        return cSize;
                    }
                    ip = iend;
                    op = op.wrapping_add(cSize);
                    (*zcs).frameEnded = 1;
                    ZSTD_CCtx_reset(zcs, ZSTD_reset_session_only);
                    someMoreWork = 0;
                    break;
                }
                /* complete loading into inBuffer in buffered mode */
                if (*zcs).appliedParams.inBufferMode == ZSTD_bm_buffered {
                    let toLoad: size_t = (*zcs).inBuffTarget - (*zcs).inBuffPos;
                    let loaded: size_t = ZSTD_limitCopy(
                        ((*zcs).inBuff as *mut u8).wrapping_add((*zcs).inBuffPos),
                        toLoad,
                        ip as *const u8,
                        iend.offset_from(ip) as size_t,
                    );
                    (*zcs).inBuffPos += loaded;
                    if !ip.is_null() {
                        ip = ip.wrapping_add(loaded);
                    }
                    if flushMode == ZSTD_e_continue && (*zcs).inBuffPos < (*zcs).inBuffTarget {
                        /* not enough input to fill full block : stop here */
                        someMoreWork = 0;
                        break;
                    }
                    if flushMode == ZSTD_e_flush && (*zcs).inBuffPos == (*zcs).inToCompress {
                        /* empty */
                        someMoreWork = 0;
                        break;
                    }
                } else {
                    /* ZSTD_bm_stable */
                    if flushMode == ZSTD_e_continue
                        && ((iend.offset_from(ip) as size_t) < (*zcs).blockSizeMax)
                    {
                        (*zcs).stableIn_notConsumed = iend.offset_from(ip) as size_t;
                        ip = iend; /* pretend to have consumed input */
                        someMoreWork = 0;
                        break;
                    }
                    if flushMode == ZSTD_e_flush && ip == iend {
                        /* empty */
                        someMoreWork = 0;
                        break;
                    }
                }
                /* compress current block */
                {
                    let inputBuffered: c_int =
                        ((*zcs).appliedParams.inBufferMode == ZSTD_bm_buffered) as c_int;
                    let cDst: *mut c_void;
                    let cSize: size_t;
                    let mut oSize: size_t = oend.offset_from(op) as size_t;
                    let iSize: size_t = if inputBuffered != 0 {
                        (*zcs).inBuffPos - (*zcs).inToCompress
                    } else {
                        MIN(iend.offset_from(ip) as size_t, (*zcs).blockSizeMax)
                    };
                    if oSize >= ZSTD_compressBound(iSize)
                        || (*zcs).appliedParams.outBufferMode == ZSTD_bm_stable
                    {
                        cDst = op as *mut c_void; /* compress into output buffer, skip flush */
                    } else {
                        cDst = (*zcs).outBuff as *mut c_void;
                        oSize = (*zcs).outBuffSize;
                    }
                    if inputBuffered != 0 {
                        let lastBlock: U32 =
                            (flushMode == ZSTD_e_end && ip == iend) as U32;
                        cSize = if lastBlock != 0 {
                            ZSTD_compressEnd_public(
                                zcs,
                                cDst,
                                oSize,
                                ((*zcs).inBuff as *const u8).wrapping_add((*zcs).inToCompress)
                                    as *const c_void,
                                iSize,
                            )
                        } else {
                            ZSTD_compressContinue_public(
                                zcs,
                                cDst,
                                oSize,
                                ((*zcs).inBuff as *const u8).wrapping_add((*zcs).inToCompress)
                                    as *const c_void,
                                iSize,
                            )
                        };
                        if ERR_isError(cSize) != 0 {
                            return cSize;
                        }
                        (*zcs).frameEnded = lastBlock;
                        /* prepare next block */
                        (*zcs).inBuffTarget = (*zcs).inBuffPos + (*zcs).blockSizeMax;
                        if (*zcs).inBuffTarget > (*zcs).inBuffSize {
                            (*zcs).inBuffPos = 0;
                            (*zcs).inBuffTarget = (*zcs).blockSizeMax;
                        }
                        (*zcs).inToCompress = (*zcs).inBuffPos;
                    } else {
                        /* !inputBuffered, hence ZSTD_bm_stable */
                        let lastBlock: U32 =
                            (flushMode == ZSTD_e_end && ip.wrapping_add(iSize) == iend) as U32;
                        cSize = if lastBlock != 0 {
                            ZSTD_compressEnd_public(zcs, cDst, oSize, ip as *const c_void, iSize)
                        } else {
                            ZSTD_compressContinue_public(zcs, cDst, oSize, ip as *const c_void, iSize)
                        };
                        /* Consume the input prior to error checking to mirror buffered mode. */
                        if !ip.is_null() {
                            ip = ip.wrapping_add(iSize);
                        }
                        if ERR_isError(cSize) != 0 {
                            return cSize;
                        }
                        (*zcs).frameEnded = lastBlock;
                    }
                    if cDst == op as *mut c_void {
                        /* no need to flush */
                        op = op.wrapping_add(cSize);
                        if (*zcs).frameEnded != 0 {
                            someMoreWork = 0;
                            ZSTD_CCtx_reset(zcs, ZSTD_reset_session_only);
                        }
                        break;
                    }
                    (*zcs).outBuffContentSize = cSize;
                    (*zcs).outBuffFlushedSize = 0;
                    (*zcs).streamStage = zcss_flush; /* pass-through to flush stage */
                }
                /* ZSTD_FALLTHROUGH to zcss_flush */
                {
                    let toFlush: size_t = (*zcs).outBuffContentSize - (*zcs).outBuffFlushedSize;
                    let flushed: size_t = ZSTD_limitCopy(
                        op as *mut u8,
                        oend.offset_from(op) as size_t,
                        ((*zcs).outBuff as *const u8).wrapping_add((*zcs).outBuffFlushedSize),
                        toFlush,
                    );
                    if flushed != 0 {
                        op = op.wrapping_add(flushed);
                    }
                    (*zcs).outBuffFlushedSize += flushed;
                    if toFlush != flushed {
                        someMoreWork = 0;
                        break;
                    }
                    (*zcs).outBuffContentSize = 0;
                    (*zcs).outBuffFlushedSize = 0;
                    if (*zcs).frameEnded != 0 {
                        someMoreWork = 0;
                        ZSTD_CCtx_reset(zcs, ZSTD_reset_session_only);
                        break;
                    }
                    (*zcs).streamStage = zcss_load;
                    break;
                }
            }

            s if s == zcss_flush => {
                let toFlush: size_t = (*zcs).outBuffContentSize - (*zcs).outBuffFlushedSize;
                let flushed: size_t = ZSTD_limitCopy(
                    op as *mut u8,
                    oend.offset_from(op) as size_t,
                    ((*zcs).outBuff as *const u8).wrapping_add((*zcs).outBuffFlushedSize),
                    toFlush,
                );
                if flushed != 0 {
                    op = op.wrapping_add(flushed);
                }
                (*zcs).outBuffFlushedSize += flushed;
                if toFlush != flushed {
                    /* flush not fully completed, presumably because dst is too small */
                    someMoreWork = 0;
                    break;
                }
                (*zcs).outBuffContentSize = 0;
                (*zcs).outBuffFlushedSize = 0;
                if (*zcs).frameEnded != 0 {
                    someMoreWork = 0;
                    ZSTD_CCtx_reset(zcs, ZSTD_reset_session_only);
                    break;
                }
                (*zcs).streamStage = zcss_load;
                break;
            }

            _ => { /* impossible */ }
        }
    }

    (*input).pos = ip.offset_from(istart) as size_t;
    (*output).pos = op.offset_from(ostart) as size_t;
    if (*zcs).frameEnded != 0 {
        return 0;
    }
    ZSTD_nextInputSizeHint(zcs)
}

unsafe fn ZSTD_nextInputSizeHint_MTorST(cctx: *const ZSTD_CCtx) -> size_t {
    /* ZSTD_MULTITHREAD undefined */
    ZSTD_nextInputSizeHint(cctx)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressStream(
    zcs: *mut ZSTD_CStream,
    output: *mut ZSTD_outBuffer,
    input: *mut ZSTD_inBuffer,
) -> size_t {
    {
        let err = ZSTD_compressStream2(zcs, output, input, ZSTD_e_continue);
        if ERR_isError(err) != 0 {
            return err;
        }
    }
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
) -> size_t {
    if (*cctx).appliedParams.inBufferMode == ZSTD_bm_stable {
        let expect: ZSTD_inBuffer = (*cctx).expectedInBuffer;
        if expect.src != (*input).src || expect.pos != (*input).pos {
            return ERROR(ZSTD_error_stabilityCondition_notRespected);
        }
    }
    let _ = endOp;
    if (*cctx).appliedParams.outBufferMode == ZSTD_bm_stable {
        let outBufferSize: size_t = (*output).size - (*output).pos;
        if (*cctx).expectedOutBufferSize != outBufferSize {
            return ERROR(ZSTD_error_stabilityCondition_notRespected);
        }
    }
    0
}

unsafe fn ZSTD_CCtx_init_compressStream2(
    cctx: *mut ZSTD_CCtx,
    endOp: ZSTD_EndDirective,
    inSize: size_t,
) -> size_t {
    let mut params: ZSTD_CCtx_params = core::ptr::read(&(*cctx).requestedParams);
    let prefixDict: ZSTD_prefixDict = core::ptr::read(&(*cctx).prefixDict);
    {
        let err = ZSTD_initLocalDict(cctx); /* Init the local dict if present. */
        if ERR_isError(err) != 0 {
            return err;
        }
    }
    ZSTD_memset(
        &mut (*cctx).prefixDict as *mut ZSTD_prefixDict as *mut u8,
        0,
        core::mem::size_of::<ZSTD_prefixDict>() as size_t,
    ); /* single usage */
    if !(*cctx).cdict.is_null() && (*cctx).localDict.cdict.is_null() {
        params.compressionLevel = (*cdl((*cctx).cdict)).compressionLevel;
    }
    if endOp == ZSTD_e_end {
        (*cctx).pledgedSrcSizePlusOne = (inSize as core::ffi::c_ulonglong) + 1;
    }

    {
        let dictSize: size_t = if !prefixDict.dict.is_null() {
            prefixDict.dictSize
        } else if !(*cctx).cdict.is_null() {
            (*cdl((*cctx).cdict)).dictContentSize
        } else {
            0
        };
        let mode: ZSTD_CParamMode_e =
            ZSTD_getCParamMode((*cctx).cdict, &params, (*cctx).pledgedSrcSizePlusOne - 1);
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

    /* ZSTD_MULTITHREAD undefined: nbWorkers always 0, MT branch absent */
    {
        let pledgedSrcSize: U64 = (*cctx).pledgedSrcSizePlusOne - 1;
        {
            let err = ZSTD_compressBegin_internal(
                cctx,
                prefixDict.dict,
                prefixDict.dictSize,
                prefixDict.dictContentType,
                ZSTD_dtlm_fast,
                (*cctx).cdict,
                &params,
                pledgedSrcSize,
                ZSTDb_buffered,
            );
            if ERR_isError(err) != 0 {
                return err;
            }
        }
        (*cctx).inToCompress = 0;
        (*cctx).inBuffPos = 0;
        if (*cctx).appliedParams.inBufferMode == ZSTD_bm_buffered {
            (*cctx).inBuffTarget =
                (*cctx).blockSizeMax + ((*cctx).blockSizeMax as U64 == pledgedSrcSize) as size_t;
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
) -> size_t {
    /* check conditions */
    if (*output).pos > (*output).size {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    if (*input).pos > (*input).size {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    if (endOp as U32) > (ZSTD_e_end as U32) {
        return ERROR(ZSTD_error_parameter_outOfBound);
    }

    /* transparent initialization stage */
    if (*cctx).streamStage == zcss_init {
        let inputSize: size_t = (*input).size - (*input).pos;
        let totalInputSize: size_t = inputSize + (*cctx).stableIn_notConsumed;
        if (*cctx).requestedParams.inBufferMode == ZSTD_bm_stable
            && endOp == ZSTD_e_continue
            && totalInputSize < ZSTD_BLOCKSIZE_MAX as size_t
        {
            if (*cctx).stableIn_notConsumed != 0 {
                /* not the first time */
                if (*input).src != (*cctx).expectedInBuffer.src {
                    return ERROR(ZSTD_error_stabilityCondition_notRespected);
                }
                if (*input).pos != (*cctx).expectedInBuffer.size {
                    return ERROR(ZSTD_error_stabilityCondition_notRespected);
                }
            }
            /* pretend input was consumed, to give a sense forward progress */
            (*input).pos = (*input).size;
            /* save stable inBuffer, for later control, and flush/end */
            (*cctx).expectedInBuffer = *input;
            /* but actually input wasn't consumed */
            (*cctx).stableIn_notConsumed += inputSize;
            return ZSTD_FRAMEHEADERSIZE_MIN((*cctx).requestedParams.format);
        }
        {
            let err = ZSTD_CCtx_init_compressStream2(cctx, endOp, totalInputSize);
            if ERR_isError(err) != 0 {
                return err;
            }
        }
        ZSTD_setBufferExpectations(cctx, output, input);
    }
    /* end of transparent initialization stage */

    {
        let err = ZSTD_checkBufferStability(cctx, output, input, endOp);
        if ERR_isError(err) != 0 {
            return err;
        }
    }
    /* compression stage : ZSTD_MULTITHREAD undefined, nbWorkers==0 */
    {
        let err = ZSTD_compressStream_generic(cctx, output, input, endOp);
        if ERR_isError(err) != 0 {
            return err;
        }
    }
    ZSTD_setBufferExpectations(cctx, output, input);
    (*cctx).outBuffContentSize - (*cctx).outBuffFlushedSize /* remaining to flush */
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressStream2_simpleArgs(
    cctx: *mut ZSTD_CCtx,
    dst: *mut c_void,
    dstCapacity: size_t,
    dstPos: *mut size_t,
    src: *const c_void,
    srcSize: size_t,
    srcPos: *mut size_t,
    endOp: ZSTD_EndDirective,
) -> size_t {
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
        let cErr: size_t = ZSTD_compressStream2(cctx, &mut output, &mut input, endOp);
        *dstPos = output.pos;
        *srcPos = input.pos;
        cErr
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compress2(
    cctx: *mut ZSTD_CCtx,
    dst: *mut c_void,
    dstCapacity: size_t,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    let originalInBufferMode: ZSTD_bufferMode_e = (*cctx).requestedParams.inBufferMode;
    let originalOutBufferMode: ZSTD_bufferMode_e = (*cctx).requestedParams.outBufferMode;
    ZSTD_CCtx_reset(cctx, ZSTD_reset_session_only);
    /* Enable stable input/output buffers. */
    (*cctx).requestedParams.inBufferMode = ZSTD_bm_stable;
    (*cctx).requestedParams.outBufferMode = ZSTD_bm_stable;
    {
        let mut oPos: size_t = 0;
        let mut iPos: size_t = 0;
        let result: size_t = ZSTD_compressStream2_simpleArgs(
            cctx,
            dst,
            dstCapacity,
            &mut oPos,
            src,
            srcSize,
            &mut iPos,
            ZSTD_e_end,
        );
        /* Reset to the original values. */
        (*cctx).requestedParams.inBufferMode = originalInBufferMode;
        (*cctx).requestedParams.outBufferMode = originalOutBufferMode;

        if ERR_isError(result) != 0 {
            return result;
        }
        if result != 0 {
            /* compression not completed, due to lack of output space */
            return ERROR(ZSTD_error_dstSize_tooSmall);
        }
        oPos
    }
}

/* ================================================================== */
/*  Explicit-sequences compression API                                */
/* ================================================================== */
unsafe fn ZSTD_validateSequence(
    offBase: U32,
    matchLength: U32,
    minMatch: U32,
    posInSrc: size_t,
    windowLog: U32,
    dictSize: size_t,
    useSequenceProducer: c_int,
) -> size_t {
    let windowSize: U32 = 1u32 << windowLog;
    let offsetBound: size_t = if posInSrc > windowSize as size_t {
        windowSize as size_t
    } else {
        posInSrc + dictSize
    };
    let matchLenLowerBound: size_t = if minMatch == 3 || useSequenceProducer != 0 {
        3
    } else {
        4
    };
    if offBase > OFFSET_TO_OFFBASE(offsetBound as U32) {
        return ERROR(ZSTD_error_externalSequences_invalid);
    }
    if (matchLength as size_t) < matchLenLowerBound {
        return ERROR(ZSTD_error_externalSequences_invalid);
    }
    0
}

/* Returns an offset code, given a sequence's raw offset, the ongoing repcode
 * array, and whether litLength == 0 */
unsafe fn ZSTD_finalizeOffBase(rawOffset: U32, rep: *const U32, ll0: U32) -> U32 {
    let mut offBase: U32 = OFFSET_TO_OFFBASE(rawOffset);

    if ll0 == 0 && rawOffset == *rep.wrapping_add(0) {
        offBase = REPCODE1_TO_OFFBASE;
    } else if rawOffset == *rep.wrapping_add(1) {
        offBase = REPCODE_TO_OFFBASE(2 - ll0);
    } else if rawOffset == *rep.wrapping_add(2) {
        offBase = REPCODE_TO_OFFBASE(3 - ll0);
    } else if ll0 != 0 && rawOffset == *rep.wrapping_add(0) - 1 {
        offBase = REPCODE3_TO_OFFBASE;
    }
    offBase
}

/* REQUIRED cross-file symbol: zstd_compress_blocks.rs imports this. */
pub unsafe fn ZSTD_transferSequences_wBlockDelim(
    cctx: *mut ZSTD_CCtx,
    seqPos: *mut ZSTD_SequencePosition,
    inSeqs: *const ZSTD_Sequence,
    inSeqsSize: size_t,
    src: *const c_void,
    blockSize: size_t,
    externalRepSearch: ZSTD_ParamSwitch_e,
) -> size_t {
    let mut idx: U32 = (*seqPos).idx;
    let startIdx: U32 = idx;
    let mut ip: *const BYTE = src as *const BYTE;
    let iend: *const BYTE = ip.wrapping_add(blockSize);
    let mut updatedRepcodes: Repcodes_t = core::mem::zeroed();
    let dictSize: U32;

    if !(*cctx).cdict.is_null() {
        dictSize = (*cdl((*cctx).cdict)).dictContentSize as U32;
    } else if !(*cctx).prefixDict.dict.is_null() {
        dictSize = (*cctx).prefixDict.dictSize as U32;
    } else {
        dictSize = 0;
    }
    ZSTD_memcpy(
        updatedRepcodes.rep.as_mut_ptr() as *mut u8,
        (*(*cctx).blockState.prevCBlock).rep.as_ptr() as *const u8,
        core::mem::size_of::<Repcodes_t>() as size_t,
    );
    while (idx as size_t) < inSeqsSize
        && ((*inSeqs.wrapping_add(idx as usize)).matchLength != 0
            || (*inSeqs.wrapping_add(idx as usize)).offset != 0)
    {
        let litLength: U32 = (*inSeqs.wrapping_add(idx as usize)).litLength;
        let matchLength: U32 = (*inSeqs.wrapping_add(idx as usize)).matchLength;
        let offBase: U32;

        if externalRepSearch == ZSTD_ps_disable {
            offBase = OFFSET_TO_OFFBASE((*inSeqs.wrapping_add(idx as usize)).offset);
        } else {
            let ll0: U32 = (litLength == 0) as U32;
            offBase = ZSTD_finalizeOffBase(
                (*inSeqs.wrapping_add(idx as usize)).offset,
                updatedRepcodes.rep.as_ptr(),
                ll0,
            );
            ZSTD_updateRep(updatedRepcodes.rep.as_mut_ptr(), offBase, ll0);
        }

        if (*cctx).appliedParams.validateSequences != 0 {
            (*seqPos).posInSrc += (litLength + matchLength) as size_t;
            let err = ZSTD_validateSequence(
                offBase,
                matchLength,
                (*cctx).appliedParams.cParams.minMatch,
                (*seqPos).posInSrc,
                (*cctx).appliedParams.cParams.windowLog,
                dictSize as size_t,
                ZSTD_hasExtSeqProd(&(*cctx).appliedParams),
            );
            if ERR_isError(err) != 0 {
                return err;
            }
        }
        if (idx - (*seqPos).idx) as size_t >= (*cctx).seqStore.maxNbSeq {
            return ERROR(ZSTD_error_externalSequences_invalid);
        }
        ZSTD_storeSeq(
            &mut (*cctx).seqStore,
            litLength as size_t,
            ip,
            iend,
            offBase,
            matchLength as size_t,
        );
        ip = ip.wrapping_add((matchLength + litLength) as usize);
        idx += 1;
    }
    if idx as size_t == inSeqsSize {
        return ERROR(ZSTD_error_externalSequences_invalid);
    }

    /* If we skipped repcode search while parsing, we need to update repcodes now */
    if externalRepSearch == ZSTD_ps_disable && idx != startIdx {
        let rep: *mut U32 = updatedRepcodes.rep.as_mut_ptr();
        let lastSeqIdx: U32 = idx - 1; /* index of last non-block-delimiter sequence */

        if lastSeqIdx >= startIdx + 2 {
            *rep.wrapping_add(2) = (*inSeqs.wrapping_add((lastSeqIdx - 2) as usize)).offset;
            *rep.wrapping_add(1) = (*inSeqs.wrapping_add((lastSeqIdx - 1) as usize)).offset;
            *rep.wrapping_add(0) = (*inSeqs.wrapping_add(lastSeqIdx as usize)).offset;
        } else if lastSeqIdx == startIdx + 1 {
            *rep.wrapping_add(2) = *rep.wrapping_add(0);
            *rep.wrapping_add(1) = (*inSeqs.wrapping_add((lastSeqIdx - 1) as usize)).offset;
            *rep.wrapping_add(0) = (*inSeqs.wrapping_add(lastSeqIdx as usize)).offset;
        } else {
            *rep.wrapping_add(2) = *rep.wrapping_add(1);
            *rep.wrapping_add(1) = *rep.wrapping_add(0);
            *rep.wrapping_add(0) = (*inSeqs.wrapping_add(lastSeqIdx as usize)).offset;
        }
    }

    ZSTD_memcpy(
        (*(*cctx).blockState.nextCBlock).rep.as_mut_ptr() as *mut u8,
        updatedRepcodes.rep.as_ptr() as *const u8,
        core::mem::size_of::<Repcodes_t>() as size_t,
    );

    if (*inSeqs.wrapping_add(idx as usize)).litLength != 0 {
        ZSTD_storeLastLiterals(
            &mut (*cctx).seqStore,
            ip,
            (*inSeqs.wrapping_add(idx as usize)).litLength as size_t,
        );
        ip = ip.wrapping_add((*inSeqs.wrapping_add(idx as usize)).litLength as usize);
        (*seqPos).posInSrc += (*inSeqs.wrapping_add(idx as usize)).litLength as size_t;
    }
    if ip != iend {
        return ERROR(ZSTD_error_externalSequences_invalid);
    }
    (*seqPos).idx = idx + 1;
    blockSize
}

unsafe fn ZSTD_transferSequences_noDelim(
    cctx: *mut ZSTD_CCtx,
    seqPos: *mut ZSTD_SequencePosition,
    inSeqs: *const ZSTD_Sequence,
    inSeqsSize: size_t,
    src: *const c_void,
    blockSize: size_t,
    externalRepSearch: ZSTD_ParamSwitch_e,
) -> size_t {
    let mut idx: U32 = (*seqPos).idx;
    let mut startPosInSequence: U32 = (*seqPos).posInSequence;
    let mut endPosInSequence: U32 = (*seqPos).posInSequence + blockSize as U32;
    let dictSize: size_t;
    let istart: *const BYTE = src as *const BYTE;
    let mut ip: *const BYTE = istart;
    let mut iend: *const BYTE = istart.wrapping_add(blockSize);
    let mut updatedRepcodes: Repcodes_t = core::mem::zeroed();
    let mut bytesAdjustment: U32 = 0;
    let mut finalMatchSplit: U32 = 0;

    let _ = externalRepSearch;

    if !(*cctx).cdict.is_null() {
        dictSize = (*cdl((*cctx).cdict)).dictContentSize;
    } else if !(*cctx).prefixDict.dict.is_null() {
        dictSize = (*cctx).prefixDict.dictSize;
    } else {
        dictSize = 0;
    }
    ZSTD_memcpy(
        updatedRepcodes.rep.as_mut_ptr() as *mut u8,
        (*(*cctx).blockState.prevCBlock).rep.as_ptr() as *const u8,
        core::mem::size_of::<Repcodes_t>() as size_t,
    );
    while endPosInSequence != 0 && (idx as size_t) < inSeqsSize && finalMatchSplit == 0 {
        let currSeq: ZSTD_Sequence = *inSeqs.wrapping_add(idx as usize);
        let mut litLength: U32 = currSeq.litLength;
        let mut matchLength: U32 = currSeq.matchLength;
        let rawOffset: U32 = currSeq.offset;
        let offBase: U32;

        /* Modify the sequence depending on where endPosInSequence lies */
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
            /* final (partial) sequence : may have to split */
            if endPosInSequence > litLength {
                let mut firstHalfMatchLength: U32;
                litLength = if startPosInSequence >= litLength {
                    0
                } else {
                    litLength - startPosInSequence
                };
                firstHalfMatchLength = endPosInSequence - startPosInSequence - litLength;
                if matchLength as size_t > blockSize
                    && firstHalfMatchLength >= (*cctx).appliedParams.cParams.minMatch
                {
                    let secondHalfMatchLength: U32 =
                        currSeq.matchLength + currSeq.litLength - endPosInSequence;
                    if secondHalfMatchLength < (*cctx).appliedParams.cParams.minMatch {
                        endPosInSequence -=
                            (*cctx).appliedParams.cParams.minMatch - secondHalfMatchLength;
                        bytesAdjustment =
                            (*cctx).appliedParams.cParams.minMatch - secondHalfMatchLength;
                        firstHalfMatchLength -= bytesAdjustment;
                    }
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
        /* Check if this offset can be represented with a repcode */
        {
            let ll0: U32 = (litLength == 0) as U32;
            offBase = ZSTD_finalizeOffBase(rawOffset, updatedRepcodes.rep.as_ptr(), ll0);
            ZSTD_updateRep(updatedRepcodes.rep.as_mut_ptr(), offBase, ll0);
        }

        if (*cctx).appliedParams.validateSequences != 0 {
            (*seqPos).posInSrc += (litLength + matchLength) as size_t;
            let err = ZSTD_validateSequence(
                offBase,
                matchLength,
                (*cctx).appliedParams.cParams.minMatch,
                (*seqPos).posInSrc,
                (*cctx).appliedParams.cParams.windowLog,
                dictSize,
                ZSTD_hasExtSeqProd(&(*cctx).appliedParams),
            );
            if ERR_isError(err) != 0 {
                return err;
            }
        }
        if (idx - (*seqPos).idx) as size_t >= (*cctx).seqStore.maxNbSeq {
            return ERROR(ZSTD_error_externalSequences_invalid);
        }
        ZSTD_storeSeq(
            &mut (*cctx).seqStore,
            litLength as size_t,
            ip,
            iend,
            offBase,
            matchLength as size_t,
        );
        ip = ip.wrapping_add((matchLength + litLength) as usize);
        if finalMatchSplit == 0 {
            idx += 1; /* Next Sequence */
        }
    }
    (*seqPos).idx = idx;
    (*seqPos).posInSequence = endPosInSequence;
    ZSTD_memcpy(
        (*(*cctx).blockState.nextCBlock).rep.as_mut_ptr() as *mut u8,
        updatedRepcodes.rep.as_ptr() as *const u8,
        core::mem::size_of::<Repcodes_t>() as size_t,
    );

    iend = iend.wrapping_sub(bytesAdjustment as usize);
    if ip != iend {
        /* Store any last literals */
        let lastLLSize: U32 = iend.offset_from(ip) as U32;
        ZSTD_storeLastLiterals(&mut (*cctx).seqStore, ip, lastLLSize as size_t);
        (*seqPos).posInSrc += lastLLSize as size_t;
    }

    iend.offset_from(istart) as size_t
}

type ZSTD_SequenceCopier_f = unsafe fn(
    cctx: *mut ZSTD_CCtx,
    seqPos: *mut ZSTD_SequencePosition,
    inSeqs: *const ZSTD_Sequence,
    inSeqsSize: size_t,
    src: *const c_void,
    blockSize: size_t,
    externalRepSearch: ZSTD_ParamSwitch_e,
) -> size_t;

unsafe fn ZSTD_selectSequenceCopier(mode: ZSTD_SequenceFormat_e) -> ZSTD_SequenceCopier_f {
    if mode == ZSTD_sf_explicitBlockDelimiters {
        return ZSTD_transferSequences_wBlockDelim;
    }
    ZSTD_transferSequences_noDelim
}

unsafe fn blockSize_explicitDelimiter(
    inSeqs: *const ZSTD_Sequence,
    inSeqsSize: size_t,
    seqPos: ZSTD_SequencePosition,
) -> size_t {
    let mut end: c_int = 0;
    let mut blockSize: size_t = 0;
    let mut spos: size_t = seqPos.idx as size_t;
    while spos < inSeqsSize {
        end = ((*inSeqs.wrapping_add(spos)).offset == 0) as c_int;
        blockSize += ((*inSeqs.wrapping_add(spos)).litLength
            + (*inSeqs.wrapping_add(spos)).matchLength) as size_t;
        if end != 0 {
            if (*inSeqs.wrapping_add(spos)).matchLength != 0 {
                return ERROR(ZSTD_error_externalSequences_invalid);
            }
            break;
        }
        spos += 1;
    }
    if end == 0 {
        return ERROR(ZSTD_error_externalSequences_invalid);
    }
    blockSize
}

unsafe fn determine_blockSize(
    mode: ZSTD_SequenceFormat_e,
    blockSize: size_t,
    remaining: size_t,
    inSeqs: *const ZSTD_Sequence,
    inSeqsSize: size_t,
    seqPos: ZSTD_SequencePosition,
) -> size_t {
    if mode == ZSTD_sf_noBlockDelimiters {
        return MIN(remaining, blockSize);
    }
    {
        let explicitBlockSize: size_t = blockSize_explicitDelimiter(inSeqs, inSeqsSize, seqPos);
        if ERR_isError(explicitBlockSize) != 0 {
            return explicitBlockSize;
        }
        if explicitBlockSize > blockSize {
            return ERROR(ZSTD_error_externalSequences_invalid);
        }
        if explicitBlockSize > remaining {
            return ERROR(ZSTD_error_externalSequences_invalid);
        }
        explicitBlockSize
    }
}

unsafe fn ZSTD_compressSequences_internal(
    cctx: *mut ZSTD_CCtx,
    dst: *mut c_void,
    mut dstCapacity: size_t,
    inSeqs: *const ZSTD_Sequence,
    inSeqsSize: size_t,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    let mut cSize: size_t = 0;
    let mut remaining: size_t = srcSize;
    let mut seqPos: ZSTD_SequencePosition = ZSTD_SequencePosition {
        idx: 0,
        posInSequence: 0,
        posInSrc: 0,
    };

    let mut ip: *const BYTE = src as *const BYTE;
    let mut op: *mut BYTE = dst as *mut BYTE;
    let sequenceCopier: ZSTD_SequenceCopier_f =
        ZSTD_selectSequenceCopier((*cctx).appliedParams.blockDelimiters);

    /* Special case: empty frame */
    if remaining == 0 {
        let cBlockHeader24: U32 = 1 /* last block */ + (((bt_raw as U32)) << 1);
        if dstCapacity < 4 {
            return ERROR(ZSTD_error_dstSize_tooSmall);
        }
        MEM_writeLE32(op, cBlockHeader24);
        op = op.wrapping_add(ZSTD_blockHeaderSize);
        dstCapacity -= ZSTD_blockHeaderSize;
        cSize += ZSTD_blockHeaderSize;
    }

    while remaining != 0 {
        let compressedSeqsSize: size_t;
        let cBlockSize: size_t;
        let mut blockSize: size_t = determine_blockSize(
            (*cctx).appliedParams.blockDelimiters,
            (*cctx).blockSizeMax,
            remaining,
            inSeqs,
            inSeqsSize,
            seqPos,
        );
        let lastBlock: U32 = (blockSize == remaining) as U32;
        if ERR_isError(blockSize) != 0 {
            return blockSize;
        }
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
        if ERR_isError(blockSize) != 0 {
            return blockSize;
        }

        /* If blocks are too small, emit as a nocompress block */
        if blockSize < MIN_CBLOCK_SIZE as size_t + ZSTD_blockHeaderSize + 1 + 1 {
            let cbs = ZSTD_noCompressBlock(
                op as *mut c_void,
                dstCapacity,
                ip as *const c_void,
                blockSize,
                lastBlock,
            );
            if ERR_isError(cbs) != 0 {
                return cbs;
            }
            cSize += cbs;
            ip = ip.wrapping_add(blockSize);
            op = op.wrapping_add(cbs);
            remaining -= blockSize;
            dstCapacity -= cbs;
            continue;
        }

        if dstCapacity < ZSTD_blockHeaderSize {
            return ERROR(ZSTD_error_dstSize_tooSmall);
        }
        let mut compressedSeqsSize_v: size_t = ZSTD_entropyCompressSeqStore(
            &(*cctx).seqStore,
            &(*(*cctx).blockState.prevCBlock).entropy,
            &mut (*(*cctx).blockState.nextCBlock).entropy,
            &(*cctx).appliedParams,
            op.wrapping_add(ZSTD_blockHeaderSize) as *mut c_void,
            dstCapacity - ZSTD_blockHeaderSize,
            blockSize,
            (*cctx).tmpWorkspace,
            (*cctx).tmpWkspSize,
            (*cctx).bmi2,
        );
        if ERR_isError(compressedSeqsSize_v) != 0 {
            return compressedSeqsSize_v;
        }

        if (*cctx).isFirstBlock == 0
            && ZSTD_maybeRLE(&(*cctx).seqStore) != 0
            && ZSTD_isRLE(ip, blockSize) != 0
        {
            compressedSeqsSize_v = 1;
        }
        compressedSeqsSize = compressedSeqsSize_v;

        if compressedSeqsSize == 0 {
            /* ZSTD_noCompressBlock writes the block header as well */
            cBlockSize = ZSTD_noCompressBlock(
                op as *mut c_void,
                dstCapacity,
                ip as *const c_void,
                blockSize,
                lastBlock,
            );
            if ERR_isError(cBlockSize) != 0 {
                return cBlockSize;
            }
        } else if compressedSeqsSize == 1 {
            cBlockSize =
                ZSTD_rleCompressBlock(op as *mut c_void, dstCapacity, *ip, blockSize, lastBlock);
            if ERR_isError(cBlockSize) != 0 {
                return cBlockSize;
            }
        } else {
            let cBlockHeader: U32;
            /* Error checking and repcodes update */
            ZSTD_blockState_confirmRepcodesAndEntropyTables(&mut (*cctx).blockState);
            if (*(*cctx).blockState.prevCBlock).entropy.fse.offcode_repeatMode == FSE_repeat_valid {
                (*(*cctx).blockState.prevCBlock).entropy.fse.offcode_repeatMode = FSE_repeat_check;
            }

            /* Write block header into beginning of block */
            cBlockHeader =
                lastBlock + (((bt_compressed as U32)) << 1) + ((compressedSeqsSize << 3) as U32);
            MEM_writeLE24(op, cBlockHeader);
            cBlockSize = ZSTD_blockHeaderSize + compressedSeqsSize;
        }

        cSize += cBlockSize;

        if lastBlock != 0 {
            break;
        } else {
            ip = ip.wrapping_add(blockSize);
            op = op.wrapping_add(cBlockSize);
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
    mut dstCapacity: size_t,
    inSeqs: *const ZSTD_Sequence,
    inSeqsSize: size_t,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    let mut op: *mut BYTE = dst as *mut BYTE;
    let mut cSize: size_t = 0;

    {
        let err = ZSTD_CCtx_init_compressStream2(cctx, ZSTD_e_end, srcSize);
        if ERR_isError(err) != 0 {
            return err;
        }
    }

    /* Begin writing output, starting with frame header */
    {
        let frameHeaderSize: size_t = ZSTD_writeFrameHeader(
            op as *mut c_void,
            dstCapacity,
            &(*cctx).appliedParams,
            srcSize as U64,
            (*cctx).dictID,
        );
        op = op.wrapping_add(frameHeaderSize);
        dstCapacity -= frameHeaderSize;
        cSize += frameHeaderSize;
    }
    if (*cctx).appliedParams.fParams.checksumFlag != 0 && srcSize != 0 {
        ZSTD_XXH64_update(&mut (*cctx).xxhState, src, srcSize);
    }

    /* Now generate compressed blocks */
    {
        let cBlocksSize: size_t = ZSTD_compressSequences_internal(
            cctx,
            op as *mut c_void,
            dstCapacity,
            inSeqs,
            inSeqsSize,
            src,
            srcSize,
        );
        if ERR_isError(cBlocksSize) != 0 {
            return cBlocksSize;
        }
        cSize += cBlocksSize;
        dstCapacity -= cBlocksSize;
    }

    /* Complete with frame checksum, if needed */
    if (*cctx).appliedParams.fParams.checksumFlag != 0 {
        let checksum: U32 = ZSTD_XXH64_digest(&(*cctx).xxhState) as U32;
        if dstCapacity < 4 {
            return ERROR(ZSTD_error_dstSize_tooSmall);
        }
        MEM_writeLE32((dst as *mut c_char).wrapping_add(cSize) as *mut u8, checksum);
        cSize += 4;
    }

    cSize
}

/* No AVX2 in this build: scalar convertSequences_noRepcodes. */
unsafe fn convertSequences_noRepcodes(
    dstSeqs: *mut SeqDef,
    inSeqs: *const ZSTD_Sequence,
    nbSequences: size_t,
) -> size_t {
    let mut longLen: size_t = 0;
    let mut n: size_t = 0;
    while n < nbSequences {
        (*dstSeqs.wrapping_add(n)).offBase = OFFSET_TO_OFFBASE((*inSeqs.wrapping_add(n)).offset);
        (*dstSeqs.wrapping_add(n)).litLength = (*inSeqs.wrapping_add(n)).litLength as U16;
        (*dstSeqs.wrapping_add(n)).mlBase =
            ((*inSeqs.wrapping_add(n)).matchLength - MINMATCH) as U16;
        if (*inSeqs.wrapping_add(n)).matchLength > 65535 + MINMATCH {
            longLen = n + 1;
        }
        if (*inSeqs.wrapping_add(n)).litLength > 65535 {
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
    nbSequences: size_t,
    repcodeResolution: c_int,
) -> size_t {
    let mut updatedRepcodes: Repcodes_t = core::mem::zeroed();
    let mut seqNb: size_t = 0;

    if nbSequences >= (*cctx).seqStore.maxNbSeq {
        return ERROR(ZSTD_error_externalSequences_invalid);
    }

    ZSTD_memcpy(
        updatedRepcodes.rep.as_mut_ptr() as *mut u8,
        (*(*cctx).blockState.prevCBlock).rep.as_ptr() as *const u8,
        core::mem::size_of::<Repcodes_t>() as size_t,
    );

    /* Convert Sequences from public format to internal format */
    if repcodeResolution == 0 {
        let longl: size_t = convertSequences_noRepcodes(
            (*cctx).seqStore.sequencesStart,
            inSeqs,
            nbSequences - 1,
        );
        (*cctx).seqStore.sequences =
            (*cctx).seqStore.sequencesStart.wrapping_add(nbSequences - 1);
        if longl != 0 {
            if longl <= nbSequences - 1 {
                (*cctx).seqStore.longLengthType = ZSTD_llt_matchLength;
                (*cctx).seqStore.longLengthPos = (longl - 1) as U32;
            } else {
                (*cctx).seqStore.longLengthType = ZSTD_llt_literalLength;
                (*cctx).seqStore.longLengthPos = (longl - (nbSequences - 1) - 1) as U32;
            }
        }
    } else {
        seqNb = 0;
        while seqNb < nbSequences - 1 {
            let litLength: U32 = (*inSeqs.wrapping_add(seqNb)).litLength;
            let matchLength: U32 = (*inSeqs.wrapping_add(seqNb)).matchLength;
            let ll0: U32 = (litLength == 0) as U32;
            let offBase: U32 = ZSTD_finalizeOffBase(
                (*inSeqs.wrapping_add(seqNb)).offset,
                updatedRepcodes.rep.as_ptr(),
                ll0,
            );

            ZSTD_storeSeqOnly(
                &mut (*cctx).seqStore,
                litLength as size_t,
                offBase,
                matchLength as size_t,
            );
            ZSTD_updateRep(updatedRepcodes.rep.as_mut_ptr(), offBase, ll0);
            seqNb += 1;
        }
    }

    /* If we skipped repcode search while parsing, we need to update repcodes now */
    if repcodeResolution == 0 && nbSequences > 1 {
        let rep: *mut U32 = updatedRepcodes.rep.as_mut_ptr();

        if nbSequences >= 4 {
            let lastSeqIdx: U32 = (nbSequences as U32) - 2; /* index of last full sequence */
            *rep.wrapping_add(2) = (*inSeqs.wrapping_add((lastSeqIdx - 2) as usize)).offset;
            *rep.wrapping_add(1) = (*inSeqs.wrapping_add((lastSeqIdx - 1) as usize)).offset;
            *rep.wrapping_add(0) = (*inSeqs.wrapping_add(lastSeqIdx as usize)).offset;
        } else if nbSequences == 3 {
            *rep.wrapping_add(2) = *rep.wrapping_add(0);
            *rep.wrapping_add(1) = (*inSeqs.wrapping_add(0)).offset;
            *rep.wrapping_add(0) = (*inSeqs.wrapping_add(1)).offset;
        } else {
            *rep.wrapping_add(2) = *rep.wrapping_add(1);
            *rep.wrapping_add(1) = *rep.wrapping_add(0);
            *rep.wrapping_add(0) = (*inSeqs.wrapping_add(0)).offset;
        }
    }

    ZSTD_memcpy(
        (*(*cctx).blockState.nextCBlock).rep.as_mut_ptr() as *mut u8,
        updatedRepcodes.rep.as_ptr() as *const u8,
        core::mem::size_of::<Repcodes_t>() as size_t,
    );

    0
}

/* No AVX2 in this build: scalar ZSTD_get1BlockSummary. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_get1BlockSummary(
    seqs: *const ZSTD_Sequence,
    nbSeqs: size_t,
) -> BlockSummary {
    let mut totalMatchSize: size_t = 0;
    let mut litSize: size_t = 0;
    let mut n: size_t = 0;
    while n < nbSeqs {
        totalMatchSize += (*seqs.wrapping_add(n)).matchLength as size_t;
        litSize += (*seqs.wrapping_add(n)).litLength as size_t;
        if (*seqs.wrapping_add(n)).matchLength == 0 {
            break;
        }
        n += 1;
    }
    if n == nbSeqs {
        let bs = BlockSummary {
            nbSequences: ERROR(ZSTD_error_externalSequences_invalid),
            blockSize: 0,
            litSize: 0,
        };
        return bs;
    }
    {
        let bs = BlockSummary {
            nbSequences: n + 1,
            blockSize: litSize + totalMatchSize,
            litSize,
        };
        bs
    }
}

unsafe fn ZSTD_compressSequencesAndLiterals_internal(
    cctx: *mut ZSTD_CCtx,
    dst: *mut c_void,
    mut dstCapacity: size_t,
    mut inSeqs: *const ZSTD_Sequence,
    mut nbSequences: size_t,
    mut literals: *const c_void,
    mut litSize: size_t,
    srcSize: size_t,
) -> size_t {
    let mut remaining: size_t = srcSize;
    let mut cSize: size_t = 0;
    let mut op: *mut BYTE = dst as *mut BYTE;
    let repcodeResolution: c_int =
        ((*cctx).appliedParams.searchForExternalRepcodes == ZSTD_ps_enable) as c_int;

    if nbSequences == 0 {
        return ERROR(ZSTD_error_externalSequences_invalid);
    }

    /* Special case: empty frame */
    if nbSequences == 1 && (*inSeqs.wrapping_add(0)).litLength == 0 {
        let cBlockHeader24: U32 = 1 /* last block */ + (((bt_raw as U32)) << 1);
        if dstCapacity < 3 {
            return ERROR(ZSTD_error_dstSize_tooSmall);
        }
        MEM_writeLE24(op, cBlockHeader24);
        op = op.wrapping_add(ZSTD_blockHeaderSize);
        dstCapacity -= ZSTD_blockHeaderSize;
        cSize += ZSTD_blockHeaderSize;
    }

    while nbSequences != 0 {
        let mut compressedSeqsSize: size_t;
        let cBlockSize: size_t;
        let conversionStatus: size_t;
        let block: BlockSummary = ZSTD_get1BlockSummary(inSeqs, nbSequences);
        let lastBlock: U32 = (block.nbSequences == nbSequences) as U32;
        if ERR_isError(block.nbSequences) != 0 {
            return block.nbSequences;
        }
        if block.litSize > litSize {
            return ERROR(ZSTD_error_externalSequences_invalid);
        }
        ZSTD_resetSeqStore(&mut (*cctx).seqStore);

        conversionStatus = ZSTD_convertBlockSequences(cctx, inSeqs, block.nbSequences, repcodeResolution);
        if ERR_isError(conversionStatus) != 0 {
            return conversionStatus;
        }
        inSeqs = inSeqs.wrapping_add(block.nbSequences);
        nbSequences -= block.nbSequences;
        remaining -= block.blockSize;

        if dstCapacity < ZSTD_blockHeaderSize {
            return ERROR(ZSTD_error_dstSize_tooSmall);
        }

        compressedSeqsSize = ZSTD_entropyCompressSeqStore_internal(
            op.wrapping_add(ZSTD_blockHeaderSize) as *mut c_void,
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
        if ERR_isError(compressedSeqsSize) != 0 {
            return compressedSeqsSize;
        }
        /* the spec forbids any compressed block to be larger than maximum block size */
        if compressedSeqsSize > (*cctx).blockSizeMax {
            compressedSeqsSize = 0;
        }
        litSize -= block.litSize;
        literals = (literals as *const c_char).wrapping_add(block.litSize) as *const c_void;

        if compressedSeqsSize == 0 {
            return ERROR(ZSTD_error_cannotProduce_uncompressedBlock);
        } else {
            let cBlockHeader: U32;
            /* Error checking and repcodes update */
            ZSTD_blockState_confirmRepcodesAndEntropyTables(&mut (*cctx).blockState);
            if (*(*cctx).blockState.prevCBlock).entropy.fse.offcode_repeatMode == FSE_repeat_valid {
                (*(*cctx).blockState.prevCBlock).entropy.fse.offcode_repeatMode = FSE_repeat_check;
            }

            /* Write block header into beginning of block */
            cBlockHeader =
                lastBlock + (((bt_compressed as U32)) << 1) + ((compressedSeqsSize << 3) as U32);
            MEM_writeLE24(op, cBlockHeader);
            cBlockSize = ZSTD_blockHeaderSize + compressedSeqsSize;
        }

        cSize += cBlockSize;
        op = op.wrapping_add(cBlockSize);
        dstCapacity -= cBlockSize;
        (*cctx).isFirstBlock = 0;

        if lastBlock != 0 {
            break;
        }
    }

    if litSize != 0 {
        return ERROR(ZSTD_error_externalSequences_invalid);
    }
    if remaining != 0 {
        return ERROR(ZSTD_error_externalSequences_invalid);
    }
    cSize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressSequencesAndLiterals(
    cctx: *mut ZSTD_CCtx,
    dst: *mut c_void,
    mut dstCapacity: size_t,
    inSeqs: *const ZSTD_Sequence,
    inSeqsSize: size_t,
    literals: *const c_void,
    litSize: size_t,
    litCapacity: size_t,
    decompressedSize: size_t,
) -> size_t {
    let mut op: *mut BYTE = dst as *mut BYTE;
    let mut cSize: size_t = 0;

    if litCapacity < litSize {
        return ERROR(ZSTD_error_workSpace_tooSmall);
    }
    {
        let err = ZSTD_CCtx_init_compressStream2(cctx, ZSTD_e_end, decompressedSize);
        if ERR_isError(err) != 0 {
            return err;
        }
    }

    if (*cctx).appliedParams.blockDelimiters == ZSTD_sf_noBlockDelimiters {
        return ERROR(ZSTD_error_frameParameter_unsupported);
    }
    if (*cctx).appliedParams.validateSequences != 0 {
        return ERROR(ZSTD_error_parameter_unsupported);
    }
    if (*cctx).appliedParams.fParams.checksumFlag != 0 {
        return ERROR(ZSTD_error_frameParameter_unsupported);
    }

    /* Begin writing output, starting with frame header */
    {
        let frameHeaderSize: size_t = ZSTD_writeFrameHeader(
            op as *mut c_void,
            dstCapacity,
            &(*cctx).appliedParams,
            decompressedSize as U64,
            (*cctx).dictID,
        );
        op = op.wrapping_add(frameHeaderSize);
        dstCapacity -= frameHeaderSize;
        cSize += frameHeaderSize;
    }

    /* Now generate compressed blocks */
    {
        let cBlocksSize: size_t = ZSTD_compressSequencesAndLiterals_internal(
            cctx,
            op as *mut c_void,
            dstCapacity,
            inSeqs,
            inSeqsSize,
            literals,
            litSize,
            decompressedSize,
        );
        if ERR_isError(cBlocksSize) != 0 {
            return cBlocksSize;
        }
        cSize += cBlocksSize;
        dstCapacity -= cBlocksSize;
    }

    cSize
}

/*======   Finalize   ======*/

unsafe fn inBuffer_forEndFlush(zcs: *const ZSTD_CStream) -> ZSTD_inBuffer {
    let nullInput: ZSTD_inBuffer = ZSTD_inBuffer {
        src: null(),
        size: 0,
        pos: 0,
    };
    let stableInput: c_int = ((*zcs).appliedParams.inBufferMode == ZSTD_bm_stable) as c_int;
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
) -> size_t {
    let mut input: ZSTD_inBuffer = inBuffer_forEndFlush(zcs);
    input.size = input.pos; /* do not ingest more input during flush */
    ZSTD_compressStream2(zcs, output, &mut input, ZSTD_e_flush)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_endStream(
    zcs: *mut ZSTD_CStream,
    output: *mut ZSTD_outBuffer,
) -> size_t {
    let mut input: ZSTD_inBuffer = inBuffer_forEndFlush(zcs);
    let remainingToFlush: size_t = ZSTD_compressStream2(zcs, output, &mut input, ZSTD_e_end);
    if ERR_isError(remainingToFlush) != 0 {
        return remainingToFlush;
    }
    if (*zcs).appliedParams.nbWorkers > 0 {
        return remainingToFlush; /* minimal estimation */
    }
    /* single thread mode : attempt to calculate remaining to flush more precisely */
    {
        let lastBlockSize: size_t = if (*zcs).frameEnded != 0 {
            0
        } else {
            ZSTD_BLOCKHEADERSIZE
        };
        let checksumSize: size_t = if (*zcs).frameEnded != 0 {
            0
        } else {
            ((*zcs).appliedParams.fParams.checksumFlag * 4) as size_t
        };
        let toFlush: size_t = remainingToFlush + lastBlockSize + checksumSize;
        toFlush
    }
}

/* ================================================================== */
/*  Sequence producer registration                                    */
/* ================================================================== */
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
        (*params).extSeqProdState = null_mut();
    }
}
