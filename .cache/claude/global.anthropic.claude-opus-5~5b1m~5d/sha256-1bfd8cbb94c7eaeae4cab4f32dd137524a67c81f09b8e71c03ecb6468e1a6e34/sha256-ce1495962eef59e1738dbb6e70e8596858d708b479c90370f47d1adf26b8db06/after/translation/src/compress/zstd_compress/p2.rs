//! Translation of `compress/zstd_compress.c`, lines 2100..3571
//! (`ZSTD_resetCCtx_internal` .. `ZSTD_mergeBlockDelimiters`).
#![allow(dead_code)]

use core::ffi::{c_int, c_uint, c_void};

use crate::bits::*;
use crate::bitstream::*;
use crate::cmem::*;
use crate::compress::hist::*;
use crate::compress::zstd_compress_internal::*;
use crate::compress::zstd_compress_literals::*;
use crate::compress::zstd_compress_sequences::*;
use crate::compress::zstd_cwksp::*;
use crate::error_private::*;
use crate::fse::*;
use crate::huf::*;
use crate::xxhash::*;
use crate::zstd_common::ZSTD_isError;
use crate::zstd_h::*;
use crate::zstd_internal::*;

/* ZSTD_resetCCtx_internal() :
 * @param loadedDictSize The size of the dictionary to be loaded
 * into the context, if any. If no dictionary is used, or the
 * dictionary is being attached / copied, then pass 0.
 * note : `params` are assumed fully validated at this stage.
 */
pub unsafe fn ZSTD_resetCCtx_internal(
    zc: *mut ZSTD_CCtx,
    params: *const ZSTD_CCtx_params,
    pledgedSrcSize: U64,
    loadedDictSize: usize,
    crp: crate::compress::zstd_compress::ZSTD_compResetPolicy_e,
    zbuff: ZSTD_buffered_policy_e,
) -> usize {
    let ws: *mut ZSTD_cwksp = &mut (*zc).workspace;

    (*zc).isFirstBlock = 1;

    /* Set applied params early so we can modify them for LDM,
     * and point params at the applied params.
     */
    (*zc).appliedParams = *params;
    let params: *const ZSTD_CCtx_params = &(*zc).appliedParams;

    if (*params).ldmParams.enableLdm == ZSTD_ps_enable {
        /* Adjust long distance matching parameters */
        crate::compress::zstd_ldm::ZSTD_ldm_adjustParameters(
            &mut (*zc).appliedParams.ldmParams,
            &(*params).cParams,
        );
    }

    {
        let windowSize: usize = {
            let m: U64 = if (1u64 << (*params).cParams.windowLog) < pledgedSrcSize {
                1u64 << (*params).cParams.windowLog
            } else {
                pledgedSrcSize
            };
            let m = m as usize;
            if 1 > m {
                1
            } else {
                m
            }
        };
        let blockSize: usize = if (*params).maxBlockSize < windowSize {
            (*params).maxBlockSize
        } else {
            windowSize
        };
        let maxNbSeq: usize = crate::compress::zstd_compress::ZSTD_maxNbSeq(
            blockSize,
            (*params).cParams.minMatch,
            ZSTD_hasExtSeqProd(params),
        );
        let buffOutSize: usize =
            if zbuff == ZSTDb_buffered && (*params).outBufferMode == ZSTD_bm_buffered {
                crate::compress::zstd_compress::ZSTD_compressBound(blockSize) + 1
            } else {
                0
            };
        let buffInSize: usize =
            if zbuff == ZSTDb_buffered && (*params).inBufferMode == ZSTD_bm_buffered {
                windowSize + blockSize
            } else {
                0
            };
        let maxNbLdmSeq: usize =
            crate::compress::zstd_ldm::ZSTD_ldm_getMaxNbSeq((*params).ldmParams, blockSize);

        let indexTooClose: c_int = crate::compress::zstd_compress::ZSTD_indexTooCloseToMax(
            (*zc).blockState.matchState.window,
        );
        let dictTooBig: c_int = crate::compress::zstd_compress::ZSTD_dictTooBig(loadedDictSize);
        let mut needsIndexReset: crate::compress::zstd_compress::ZSTD_indexResetPolicy_e =
            if indexTooClose != 0 || dictTooBig != 0 || (*zc).initialized == 0 {
                crate::compress::zstd_compress::ZSTDirp_reset
            } else {
                crate::compress::zstd_compress::ZSTDirp_continue
            };

        let neededSpace: usize =
            crate::compress::zstd_compress::ZSTD_estimateCCtxSize_usingCCtxParams_internal(
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

                needsIndexReset = crate::compress::zstd_compress::ZSTDirp_reset;

                ZSTD_cwksp_free(ws, (*zc).customMem);
                {
                    let e = ZSTD_cwksp_create(ws, neededSpace, (*zc).customMem);
                    if ERR_isError(e) != 0 {
                        return e;
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

        XXH64_reset(&mut (*zc).xxhState, 0);
        (*zc).stage = ZSTDcs_init;
        (*zc).dictID = 0;
        (*zc).dictContentSize = 0;

        crate::compress::zstd_compress::ZSTD_reset_compressedBlockState(
            (*zc).blockState.prevCBlock,
        );

        {
            let e = crate::compress::zstd_compress::ZSTD_reset_matchState(
                &mut (*zc).blockState.matchState,
                ws,
                &(*params).cParams,
                (*params).useRowMatchFinder,
                crp,
                needsIndexReset,
                crate::compress::zstd_compress::ZSTD_resetTarget_CCtx,
            );
            if ERR_isError(e) != 0 {
                return e;
            }
        }

        (*zc).seqStore.sequencesStart = ZSTD_cwksp_reserve_aligned64(
            ws,
            maxNbSeq * core::mem::size_of::<SeqDef>(),
        ) as *mut SeqDef;

        /* ldm hash table */
        if (*params).ldmParams.enableLdm == ZSTD_ps_enable {
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
        (*zc).seqStore.litStart = ZSTD_cwksp_reserve_buffer(ws, blockSize + WILDCOPY_OVERLENGTH);
        (*zc).seqStore.maxNbLit = blockSize;

        (*zc).bufferedPolicy = zbuff;
        (*zc).inBuffSize = buffInSize;
        (*zc).inBuff = ZSTD_cwksp_reserve_buffer(ws, buffInSize) as *mut i8;
        (*zc).outBuffSize = buffOutSize;
        (*zc).outBuff = ZSTD_cwksp_reserve_buffer(ws, buffOutSize) as *mut i8;

        /* ldm bucketOffsets table */
        if (*params).ldmParams.enableLdm == ZSTD_ps_enable {
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
        crate::compress::zstd_compress::ZSTD_referenceExternalSequences(
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
    while i < ZSTD_REP_NUM as c_int {
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
            crate::compress::zstd_compress::ZSTD_dedicatedDictSearch_revertCParams(
                &mut adjusted_cdict_cParams,
            );
        }

        params.cParams = crate::compress::zstd_compress::ZSTD_adjustCParams_internal(
            adjusted_cdict_cParams,
            pledgedSrcSize,
            (*cdict).dictContentSize,
            ZSTD_cpm_attachDict,
            params.useRowMatchFinder,
        );
        params.cParams.windowLog = windowLog;
        params.useRowMatchFinder = (*cdict).useRowMatchFinder; /* cdict overrides */
        {
            let e = ZSTD_resetCCtx_internal(
                cctx,
                &params,
                pledgedSrcSize,
                /* loadedDictSize */ 0,
                crate::compress::zstd_compress::ZSTDcrp_makeClean,
                zbuff,
            );
            if ERR_isError(e) != 0 {
                return e;
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
            (*cctx).blockState.matchState.dictMatchState = &(*cdict).matchState;

            /* prep working match state so dict matches never have negative indices
             * when they are translated to the working context's index space. */
            if (*cctx).blockState.matchState.window.dictLimit < cdictEnd {
                (*cctx).blockState.matchState.window.nextSrc =
                    (*cctx).blockState.matchState.window.base.add(cdictEnd as usize);
                ZSTD_window_clear(&mut (*cctx).blockState.matchState.window);
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
        (*cctx).blockState.prevCBlock as *mut c_void,
        &(*cdict).cBlockState as *const ZSTD_compressedBlockState_t as *const c_void,
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
    if crate::compress::zstd_compress::ZSTD_CDictIndicesAreTagged(cParams) != 0 {
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

pub unsafe fn ZSTD_resetCCtx_byCopyingCDict(
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
            let e = ZSTD_resetCCtx_internal(
                cctx,
                &params,
                pledgedSrcSize,
                /* loadedDictSize */ 0,
                crate::compress::zstd_compress::ZSTDcrp_leaveDirty,
                zbuff,
            );
            if ERR_isError(e) != 0 {
                return e;
            }
        }
    }

    ZSTD_cwksp_mark_tables_dirty(&mut (*cctx).workspace);

    /* copy tables */
    {
        let chainSize: usize = if crate::compress::zstd_compress::ZSTD_allocateChainTable(
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

        /* Do not copy cdict's chainTable if cctx has parameters such that it would not use chainTable */
        if crate::compress::zstd_compress::ZSTD_allocateChainTable(
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
        if crate::compress::zstd_compress::ZSTD_rowMatchFinderUsed(
            (*cdict_cParams).strategy,
            (*cdict).useRowMatchFinder,
        ) != 0
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
 *  Only works during stage ZSTDcs_init (i.e. after creation, but before first call to ZSTD_compressContinue()).
 *  The "context", in this case, refers to the hash and chain tables,
 *  entropy tables, and dictionary references.
 * `windowLog` value is enforced if != 0, otherwise value is copied from srcCCtx.
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
            crate::compress::zstd_compress::ZSTDcrp_leaveDirty,
            zbuff,
        );
    }

    ZSTD_cwksp_mark_tables_dirty(&mut (*dstCCtx).workspace);

    /* copy tables */
    {
        let chainSize: usize = if crate::compress::zstd_compress::ZSTD_allocateChainTable(
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
 *  Only works during stage ZSTDcs_init (i.e. after creation, but before first call to ZSTD_compressContinue()).
 *  pledgedSrcSize==0 means "unknown".
*   @return : 0, or an error code */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_copyCCtx(
    dstCCtx: *mut ZSTD_CCtx,
    srcCCtx: *const ZSTD_CCtx,
    mut pledgedSrcSize: u64,
) -> usize {
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

pub const ZSTD_ROWSIZE: usize = 16;

/* ZSTD_reduceTable() :
 *  reduce table indexes by `reducerValue`, or squash to zero.
 *  PreserveMark preserves "unsorted mark" for btlazy2 strategy.
 *  It must be set to a clear 0/1 value, to remove branch during inlining.
 *  Presume table size is a multiple of ZSTD_ROWSIZE
 *  to help auto-vectorization */
#[inline(always)]
pub unsafe fn ZSTD_reduceTable_internal(
    table: *mut U32,
    size: U32,
    reducerValue: U32,
    preserveMark: c_int,
) {
    let nbRows: c_int = (size as c_int) / ZSTD_ROWSIZE as c_int;
    let mut cellNb: c_int = 0;
    /* Protect special index values < ZSTD_WINDOW_START_INDEX. */
    let reducerThreshold: U32 = reducerValue.wrapping_add(ZSTD_WINDOW_START_INDEX);

    let mut rowNb: c_int = 0;
    while rowNb < nbRows {
        let mut column: c_int = 0;
        while column < ZSTD_ROWSIZE as c_int {
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
        let hSize: U32 = 1u32 << (*params).cParams.hashLog;
        ZSTD_reduceTable((*ms).hashTable, hSize, reducerValue);
    }

    if crate::compress::zstd_compress::ZSTD_allocateChainTable(
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

/*-*******************************************************
*  Block entropic compression
*********************************************************/

/* See doc/zstd_compression_format.md for detailed format description */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_seqToCodes(seqStorePtr: *const SeqStore_t) -> c_int {
    let sequences: *const SeqDef = (*seqStorePtr).sequencesStart;
    let llCodeTable: *mut BYTE = (*seqStorePtr).llCode;
    let ofCodeTable: *mut BYTE = (*seqStorePtr).ofCode;
    let mlCodeTable: *mut BYTE = (*seqStorePtr).mlCode;
    let nbSeq: U32 = (*seqStorePtr)
        .sequences
        .offset_from((*seqStorePtr).sequencesStart) as U32;
    let mut u: U32;
    let mut longOffsets: c_int = 0;
    u = 0;
    while u < nbSeq {
        let llv: U32 = (*sequences.add(u as usize)).litLength as U32;
        let ofCode: U32 = ZSTD_highbit32((*sequences.add(u as usize)).offBase);
        let mlv: U32 = (*sequences.add(u as usize)).mlBase as U32;
        *llCodeTable.add(u as usize) = ZSTD_LLcode(llv) as BYTE;
        *ofCodeTable.add(u as usize) = ofCode as BYTE;
        *mlCodeTable.add(u as usize) = ZSTD_MLcode(mlv) as BYTE;
        if MEM_32bits() != 0 && ofCode >= STREAM_ACCUMULATOR_MIN() {
            longOffsets = 1;
        }
        u += 1;
    }
    if (*seqStorePtr).longLengthType == ZSTD_llt_literalLength {
        *llCodeTable.add((*seqStorePtr).longLengthPos as usize) = MaxLL as BYTE;
    }
    if (*seqStorePtr).longLengthType == ZSTD_llt_matchLength {
        *mlCodeTable.add((*seqStorePtr).longLengthPos as usize) = MaxML as BYTE;
    }
    longOffsets
}

/* ZSTD_useTargetCBlockSize():
 * Returns if target compressed block size param is being used.
 * If used, compression will do best effort to make a compressed block size to be around targetCBlockSize.
 * Returns 1 if true, 0 otherwise. */
pub unsafe fn ZSTD_useTargetCBlockSize(cctxParams: *const ZSTD_CCtx_params) -> c_int {
    ((*cctxParams).targetCBlockSize != 0) as c_int
}

/* ZSTD_blockSplitterEnabled():
 * Returns if block splitting param is being used
 * If used, compression will do best effort to split a block in order to improve compression ratio.
 * At the time this function is called, the parameter must be finalized.
 * Returns 1 if true, 0 otherwise. */
pub unsafe fn ZSTD_blockSplitterEnabled(cctxParams: *mut ZSTD_CCtx_params) -> c_int {
    ((*cctxParams).postBlockSplitter == ZSTD_ps_enable) as c_int
}

/* Type returned by ZSTD_buildSequencesStatistics containing finalized symbol encoding types
 * and size of the sequences statistics
 */
#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct ZSTD_symbolEncodingTypeStats_t {
    pub LLtype: U32,
    pub Offtype: U32,
    pub MLtype: U32,
    pub size: usize,
    /* Accounts for bug in 1.3.4. More detail in ZSTD_entropyCompressSeqStore_internal() */
    pub lastCountSize: usize,
    pub longOffsets: c_int,
}

/* ZSTD_buildSequencesStatistics():
 * Returns a ZSTD_symbolEncodingTypeStats_t, or a zstd error code in the `size` field.
 * Modifies `nextEntropy` to have the appropriate values as a side effect.
 * nbSeq must be greater than 0.
 *
 * entropyWkspSize must be of size at least ENTROPY_WORKSPACE_SIZE - (MaxSeq + 1)*sizeof(U32)
 */
pub unsafe fn ZSTD_buildSequencesStatistics(
    seqStorePtr: *const SeqStore_t,
    nbSeq: usize,
    prevEntropy: *const ZSTD_fseCTables_t,
    nextEntropy: *mut ZSTD_fseCTables_t,
    dst: *mut BYTE,
    dstEnd: *const BYTE,
    strategy: ZSTD_strategy,
    countWorkspace: *mut c_uint,
    entropyWorkspace: *mut c_void,
    entropyWkspSize: usize,
) -> ZSTD_symbolEncodingTypeStats_t {
    let ostart: *mut BYTE = dst;
    let oend: *const BYTE = dstEnd;
    let mut op: *mut BYTE = ostart;
    let CTable_LitLength: *mut FSE_CTable = (*nextEntropy).litlengthCTable.as_mut_ptr();
    let CTable_OffsetBits: *mut FSE_CTable = (*nextEntropy).offcodeCTable.as_mut_ptr();
    let CTable_MatchLength: *mut FSE_CTable = (*nextEntropy).matchlengthCTable.as_mut_ptr();
    let ofCodeTable: *const BYTE = (*seqStorePtr).ofCode;
    let llCodeTable: *const BYTE = (*seqStorePtr).llCode;
    let mlCodeTable: *const BYTE = (*seqStorePtr).mlCode;
    let mut stats = ZSTD_symbolEncodingTypeStats_t::default();

    stats.lastCountSize = 0;
    /* convert length/distances into codes */
    stats.longOffsets = ZSTD_seqToCodes(seqStorePtr);
    /* build CTable for Literal Lengths */
    {
        let mut max: c_uint = MaxLL;
        let mostFrequent: usize = HIST_countFast_wksp(
            countWorkspace,
            &mut max,
            llCodeTable as *const c_void,
            nbSeq,
            entropyWorkspace,
            entropyWkspSize,
        ); /* can't fail */
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
            let countSize: usize = ZSTD_buildCTable(
                op as *mut c_void,
                (oend as usize).wrapping_sub(op as usize),
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
                LITLENGTH_CTABLE_SIZE_U32 * core::mem::size_of::<FSE_CTable>(),
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
        }
    }
    /* build CTable for Offsets */
    {
        let mut max: c_uint = MaxOff;
        let mostFrequent: usize = HIST_countFast_wksp(
            countWorkspace,
            &mut max,
            ofCodeTable as *const c_void,
            nbSeq,
            entropyWorkspace,
            entropyWkspSize,
        ); /* can't fail */
        /* We can only use the basic table if max <= DefaultMaxOff, otherwise the offsets are too large */
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
            let countSize: usize = ZSTD_buildCTable(
                op as *mut c_void,
                (oend as usize).wrapping_sub(op as usize),
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
                OFFCODE_CTABLE_SIZE_U32 * core::mem::size_of::<FSE_CTable>(),
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
        }
    }
    /* build CTable for MatchLengths */
    {
        let mut max: c_uint = MaxML;
        let mostFrequent: usize = HIST_countFast_wksp(
            countWorkspace,
            &mut max,
            mlCodeTable as *const c_void,
            nbSeq,
            entropyWorkspace,
            entropyWkspSize,
        ); /* can't fail */
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
            let countSize: usize = ZSTD_buildCTable(
                op as *mut c_void,
                (oend as usize).wrapping_sub(op as usize),
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
                MATCHLENGTH_CTABLE_SIZE_U32 * core::mem::size_of::<FSE_CTable>(),
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
        }
    }
    stats.size = (op as usize).wrapping_sub(ostart as usize);
    stats
}

/* ZSTD_entropyCompressSeqStore_internal():
 * compresses both literals and sequences
 * Returns compressed size of block, or a zstd error.
 */
pub const SUSPECT_UNCOMPRESSIBLE_LITERAL_RATIO: usize = 20;

pub unsafe fn ZSTD_entropyCompressSeqStore_internal(
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
    let strategy: ZSTD_strategy = (*cctxParams).cParams.strategy;
    let count: *mut c_uint = entropyWorkspace as *mut c_uint;
    let CTable_LitLength: *mut FSE_CTable = (*nextEntropy).fse.litlengthCTable.as_mut_ptr();
    let CTable_OffsetBits: *mut FSE_CTable = (*nextEntropy).fse.offcodeCTable.as_mut_ptr();
    let CTable_MatchLength: *mut FSE_CTable = (*nextEntropy).fse.matchlengthCTable.as_mut_ptr();
    let sequences: *const SeqDef = (*seqStorePtr).sequencesStart;
    let nbSeq: usize = (*seqStorePtr)
        .sequences
        .offset_from((*seqStorePtr).sequencesStart) as usize;
    let ofCodeTable: *const BYTE = (*seqStorePtr).ofCode;
    let llCodeTable: *const BYTE = (*seqStorePtr).llCode;
    let mlCodeTable: *const BYTE = (*seqStorePtr).mlCode;
    let ostart: *mut BYTE = dst as *mut BYTE;
    let oend: *const BYTE = ostart.add(dstCapacity);
    let mut op: *mut BYTE = ostart;
    let lastCountSize: usize;
    let mut longOffsets: c_int = 0;

    entropyWorkspace = count.add(MaxSeq as usize + 1) as *mut c_void;
    entropyWkspSize -= (MaxSeq as usize + 1) * core::mem::size_of::<c_uint>();

    /* Compress literals */
    {
        let numSequences: usize = (*seqStorePtr)
            .sequences
            .offset_from((*seqStorePtr).sequencesStart) as usize;
        /* Base suspicion of uncompressibility on ratio of literals to sequences */
        let suspectUncompressible: c_int = ((numSequences == 0)
            || (litSize / numSequences >= SUSPECT_UNCOMPRESSIBLE_LITERAL_RATIO))
            as c_int;

        let cSize: usize = ZSTD_compressLiterals(
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
        if ERR_isError(cSize) != 0 {
            return cSize;
        }
        op = op.add(cSize);
    }

    /* Sequences Header */
    if oend.offset_from(op) < (3 /*max nbSeq Size*/ + 1 /*seqHead*/) {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    if nbSeq < 128 {
        *op = nbSeq as BYTE;
        op = op.add(1);
    } else if (nbSeq as U32) < LONGNBSEQ {
        *op.add(0) = ((nbSeq >> 8) + 0x80) as BYTE;
        *op.add(1) = nbSeq as BYTE;
        op = op.add(2);
    } else {
        *op.add(0) = 0xFF;
        MEM_writeLE16(
            op.add(1) as *mut c_void,
            (nbSeq.wrapping_sub(LONGNBSEQ as usize)) as U16,
        );
        op = op.add(3);
    }
    if nbSeq == 0 {
        /* Copy the old tables over as if we repeated them */
        ZSTD_memcpy(
            &mut (*nextEntropy).fse as *mut ZSTD_fseCTables_t as *mut c_void,
            &(*prevEntropy).fse as *const ZSTD_fseCTables_t as *const c_void,
            core::mem::size_of::<ZSTD_fseCTables_t>(),
        );
        return (op as usize).wrapping_sub(ostart as usize);
    }
    {
        let seqHead: *mut BYTE = op;
        op = op.add(1);
        /* build stats for sequences */
        let stats: ZSTD_symbolEncodingTypeStats_t = ZSTD_buildSequencesStatistics(
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
        if ERR_isError(stats.size) != 0 {
            return stats.size;
        }
        *seqHead =
            ((stats.LLtype << 6) + (stats.Offtype << 4) + (stats.MLtype << 2)) as BYTE;
        lastCountSize = stats.lastCountSize;
        op = op.add(stats.size);
        longOffsets = stats.longOffsets;
    }

    {
        let bitstreamSize: usize = ZSTD_encodeSequences(
            op as *mut c_void,
            (oend as usize).wrapping_sub(op as usize),
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
        if ERR_isError(bitstreamSize) != 0 {
            return bitstreamSize;
        }
        op = op.add(bitstreamSize);
        /* zstd versions <= 1.3.4 mistakenly report corruption when
         * FSE_readNCount() receives a buffer < 4 bytes.
         * Fixed by https://github.com/facebook/zstd/pull/1146.
         * This can happen when the last set_compressed table present is 2
         * bytes and the bitstream is only one byte.
         * In this exceedingly rare case, we will simply emit an uncompressed
         * block, since it isn't worth optimizing.
         */
        if lastCountSize != 0 && (lastCountSize + bitstreamSize) < 4 {
            /* lastCountSize >= 2 && bitstreamSize > 0 ==> lastCountSize == 3 */
            return 0;
        }
    }

    (op as usize).wrapping_sub(ostart as usize)
}

pub unsafe fn ZSTD_entropyCompressSeqStore_wExtLitBuffer(
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
    let cSize: usize = ZSTD_entropyCompressSeqStore_internal(
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
    /* When srcSize <= dstCapacity, there is enough space to write a raw uncompressed block.
     * Since we ran out of space, block must be not compressible, so fall back to raw uncompressed block.
     */
    if (((cSize == ERROR(ZSTD_error_dstSize_tooSmall)) as c_int)
        & ((blockSize <= dstCapacity) as c_int))
        != 0
    {
        return 0; /* block not compressed */
    }
    if ERR_isError(cSize) != 0 {
        return cSize;
    }

    /* Check compressibility */
    {
        let maxCSize: usize =
            blockSize - ZSTD_minGain(blockSize, (*cctxParams).cParams.strategy);
        if cSize >= maxCSize {
            return 0; /* block not compressed */
        }
    }
    /* libzstd decoder before  > v1.5.4 is not compatible with compressed blocks of size ZSTD_BLOCKSIZE_MAX exactly.
     * This restriction is indirectly already fulfilled by respecting ZSTD_minGain() condition above.
     */
    cSize
}

pub unsafe fn ZSTD_entropyCompressSeqStore(
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
        (*seqStorePtr).lit.offset_from((*seqStorePtr).litStart) as usize,
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

pub static blockCompressor: [[ZSTD_BlockCompressor_f; (ZSTD_STRATEGY_MAX + 1) as usize]; 4] = [
    [
        Some(crate::compress::zstd_fast::ZSTD_compressBlock_fast), /* default for 0 */
        Some(crate::compress::zstd_fast::ZSTD_compressBlock_fast),
        Some(crate::compress::zstd_double_fast::ZSTD_compressBlock_doubleFast),
        Some(crate::compress::zstd_lazy::ZSTD_compressBlock_greedy),
        Some(crate::compress::zstd_lazy::ZSTD_compressBlock_lazy),
        Some(crate::compress::zstd_lazy::ZSTD_compressBlock_lazy2),
        Some(crate::compress::zstd_lazy::ZSTD_compressBlock_btlazy2),
        Some(crate::compress::zstd_opt::ZSTD_compressBlock_btopt),
        Some(crate::compress::zstd_opt::ZSTD_compressBlock_btultra),
        Some(crate::compress::zstd_opt::ZSTD_compressBlock_btultra2),
    ],
    [
        Some(crate::compress::zstd_fast::ZSTD_compressBlock_fast_extDict), /* default for 0 */
        Some(crate::compress::zstd_fast::ZSTD_compressBlock_fast_extDict),
        Some(crate::compress::zstd_double_fast::ZSTD_compressBlock_doubleFast_extDict),
        Some(crate::compress::zstd_lazy::ZSTD_compressBlock_greedy_extDict),
        Some(crate::compress::zstd_lazy::ZSTD_compressBlock_lazy_extDict),
        Some(crate::compress::zstd_lazy::ZSTD_compressBlock_lazy2_extDict),
        Some(crate::compress::zstd_lazy::ZSTD_compressBlock_btlazy2_extDict),
        Some(crate::compress::zstd_opt::ZSTD_compressBlock_btopt_extDict),
        Some(crate::compress::zstd_opt::ZSTD_compressBlock_btultra_extDict),
        Some(crate::compress::zstd_opt::ZSTD_compressBlock_btultra_extDict),
    ],
    [
        Some(crate::compress::zstd_fast::ZSTD_compressBlock_fast_dictMatchState), /* default for 0 */
        Some(crate::compress::zstd_fast::ZSTD_compressBlock_fast_dictMatchState),
        Some(crate::compress::zstd_double_fast::ZSTD_compressBlock_doubleFast_dictMatchState),
        Some(crate::compress::zstd_lazy::ZSTD_compressBlock_greedy_dictMatchState),
        Some(crate::compress::zstd_lazy::ZSTD_compressBlock_lazy_dictMatchState),
        Some(crate::compress::zstd_lazy::ZSTD_compressBlock_lazy2_dictMatchState),
        Some(crate::compress::zstd_lazy::ZSTD_compressBlock_btlazy2_dictMatchState),
        Some(crate::compress::zstd_opt::ZSTD_compressBlock_btopt_dictMatchState),
        Some(crate::compress::zstd_opt::ZSTD_compressBlock_btultra_dictMatchState),
        Some(crate::compress::zstd_opt::ZSTD_compressBlock_btultra_dictMatchState),
    ],
    [
        None, /* default for 0 */
        None,
        None,
        Some(crate::compress::zstd_lazy::ZSTD_compressBlock_greedy_dedicatedDictSearch),
        Some(crate::compress::zstd_lazy::ZSTD_compressBlock_lazy_dedicatedDictSearch),
        Some(crate::compress::zstd_lazy::ZSTD_compressBlock_lazy2_dedicatedDictSearch),
        None,
        None,
        None,
        None,
    ],
];

pub static rowBasedBlockCompressors: [[ZSTD_BlockCompressor_f; 3]; 4] = [
    [
        Some(crate::compress::zstd_lazy::ZSTD_compressBlock_greedy_row),
        Some(crate::compress::zstd_lazy::ZSTD_compressBlock_lazy_row),
        Some(crate::compress::zstd_lazy::ZSTD_compressBlock_lazy2_row),
    ],
    [
        Some(crate::compress::zstd_lazy::ZSTD_compressBlock_greedy_extDict_row),
        Some(crate::compress::zstd_lazy::ZSTD_compressBlock_lazy_extDict_row),
        Some(crate::compress::zstd_lazy::ZSTD_compressBlock_lazy2_extDict_row),
    ],
    [
        Some(crate::compress::zstd_lazy::ZSTD_compressBlock_greedy_dictMatchState_row),
        Some(crate::compress::zstd_lazy::ZSTD_compressBlock_lazy_dictMatchState_row),
        Some(crate::compress::zstd_lazy::ZSTD_compressBlock_lazy2_dictMatchState_row),
    ],
    [
        Some(crate::compress::zstd_lazy::ZSTD_compressBlock_greedy_dedicatedDictSearch_row),
        Some(crate::compress::zstd_lazy::ZSTD_compressBlock_lazy_dedicatedDictSearch_row),
        Some(crate::compress::zstd_lazy::ZSTD_compressBlock_lazy2_dedicatedDictSearch_row),
    ],
];

/* ZSTD_selectBlockCompressor() :
 * Not static, but internal use only (used by long distance matcher)
 * assumption : strat is a valid strategy */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_selectBlockCompressor(
    strat: ZSTD_strategy,
    useRowMatchFinder: ZSTD_ParamSwitch_e,
    dictMode: ZSTD_dictMode_e,
) -> ZSTD_BlockCompressor_f {
    let selectedCompressor: ZSTD_BlockCompressor_f;

    if crate::compress::zstd_compress::ZSTD_rowMatchFinderUsed(strat, useRowMatchFinder) != 0 {
        selectedCompressor = rowBasedBlockCompressors[dictMode as usize]
            [(strat as c_int - ZSTD_greedy as c_int) as usize];
    } else {
        selectedCompressor = blockCompressor[dictMode as usize][strat as usize];
    }
    selectedCompressor
}

pub unsafe fn ZSTD_storeLastLiterals(
    seqStorePtr: *mut SeqStore_t,
    anchor: *const BYTE,
    lastLLSize: usize,
) {
    ZSTD_memcpy(
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

/* ZSTD_postProcessSequenceProducerResult() :
 * Validates and post-processes sequences obtained through the external matchfinder API:
 *   - Checks whether nbExternalSeqs represents an error condition.
 *   - Appends a block delimiter to outSeqs if one is not already present.
 *     See zstd.h for context regarding block delimiters.
 * Returns the number of sequences after post-processing, or an error code. */
pub unsafe fn ZSTD_postProcessSequenceProducerResult(
    outSeqs: *mut ZSTD_Sequence,
    nbExternalSeqs: usize,
    outSeqsCapacity: usize,
    srcSize: usize,
) -> usize {
    if nbExternalSeqs > outSeqsCapacity {
        return ERROR(ZSTD_error_sequenceProducer_failed);
    }

    if nbExternalSeqs == 0 && srcSize > 0 {
        return ERROR(ZSTD_error_sequenceProducer_failed);
    }

    if srcSize == 0 {
        ZSTD_memset(
            outSeqs as *mut c_void,
            0,
            core::mem::size_of::<ZSTD_Sequence>(),
        );
        return 1;
    }

    {
        let lastSeq: ZSTD_Sequence = *outSeqs.add(nbExternalSeqs - 1);

        /* We can return early if lastSeq is already a block delimiter. */
        if lastSeq.offset == 0 && lastSeq.matchLength == 0 {
            return nbExternalSeqs;
        }

        /* This error condition is only possible if the external matchfinder
         * produced an invalid parse, by definition of ZSTD_sequenceBound(). */
        if nbExternalSeqs == outSeqsCapacity {
            return ERROR(ZSTD_error_sequenceProducer_failed);
        }

        /* lastSeq is not a block delimiter, so we need to append one. */
        ZSTD_memset(
            outSeqs.add(nbExternalSeqs) as *mut c_void,
            0,
            core::mem::size_of::<ZSTD_Sequence>(),
        );
        nbExternalSeqs + 1
    }
}

/* ZSTD_fastSequenceLengthSum() :
 * Returns sum(litLen) + sum(matchLen) + lastLits for *seqBuf*.
 * Similar to another function in zstd_compress.c (determine_blockSize),
 * except it doesn't check for a block delimiter to end summation.
 * Removing the early exit allows the compiler to auto-vectorize (https://godbolt.org/z/cY1cajz9P).
 * This function can be deleted and replaced by determine_blockSize after we resolve issue #3456. */
pub unsafe fn ZSTD_fastSequenceLengthSum(
    seqBuf: *const ZSTD_Sequence,
    seqBufSize: usize,
) -> usize {
    let mut matchLenSum: usize;
    let mut litLenSum: usize;
    let mut i: usize;
    matchLenSum = 0;
    litLenSum = 0;
    i = 0;
    while i < seqBufSize {
        litLenSum = litLenSum.wrapping_add((*seqBuf.add(i)).litLength as usize);
        matchLenSum = matchLenSum.wrapping_add((*seqBuf.add(i)).matchLength as usize);
        i += 1;
    }
    litLenSum.wrapping_add(matchLenSum)
}

/**
 * Function to validate sequences produced by a block compressor.
 */
pub unsafe fn ZSTD_validateSeqStore(
    seqStore: *const SeqStore_t,
    cParams: *const ZSTD_compressionParameters,
) {
    /* DEBUGLEVEL == 0 : nothing to do */
    let _ = seqStore;
    let _ = cParams;
}

pub type ZSTD_BuildSeqStore_e = c_uint;
pub const ZSTDbss_compress: ZSTD_BuildSeqStore_e = 0;
pub const ZSTDbss_noCompress: ZSTD_BuildSeqStore_e = 1;

pub unsafe fn ZSTD_buildSeqStore(
    zc: *mut ZSTD_CCtx,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let ms: *mut ZSTD_MatchState_t = &mut (*zc).blockState.matchState;
    /* TODO: See 3090. We reduced MIN_CBLOCK_SIZE from 3 to 2 so to compensate we are adding
     * additional 1. We need to revisit and change this logic to be more consistent */
    if srcSize < MIN_CBLOCK_SIZE + ZSTD_blockHeaderSize + 1 + 1 {
        if (*zc).appliedParams.cParams.strategy >= ZSTD_btopt {
            crate::compress::zstd_ldm::ZSTD_ldm_skipRawSeqStoreBytes(
                &mut (*zc).externSeqStore,
                srcSize,
            );
        } else {
            crate::compress::zstd_ldm::ZSTD_ldm_skipSequences(
                &mut (*zc).externSeqStore,
                srcSize,
                (*zc).appliedParams.cParams.minMatch,
            );
        }
        return ZSTDbss_noCompress as usize; /* don't even attempt compression below a certain srcSize */
    }
    ZSTD_resetSeqStore(&mut (*zc).seqStore);
    /* required for optimal parser to read stats from dictionary */
    (*ms).opt.symbolCosts = &(*(*zc).blockState.prevCBlock).entropy;
    /* tell the optimal parser how we expect to compress literals */
    (*ms).opt.literalCompressionMode = (*zc).appliedParams.literalCompressionMode;

    /* limited update after a very long match */
    {
        let base: *const BYTE = (*ms).window.base;
        let istart: *const BYTE = src as *const BYTE;
        let curr: U32 = (istart as usize).wrapping_sub(base as usize) as U32;
        if curr > (*ms).nextToUpdate.wrapping_add(384) {
            let d: U32 = curr.wrapping_sub((*ms).nextToUpdate).wrapping_sub(384);
            (*ms).nextToUpdate = curr.wrapping_sub(if 192 < d { 192 } else { d });
        }
    }

    /* select and store sequences */
    {
        let dictMode: ZSTD_dictMode_e = ZSTD_matchState_dictMode(ms);
        let mut lastLLSize: usize;
        {
            let mut i: c_int = 0;
            while i < ZSTD_REP_NUM as c_int {
                (*(*zc).blockState.nextCBlock).rep[i as usize] =
                    (*(*zc).blockState.prevCBlock).rep[i as usize];
                i += 1;
            }
        }
        if (*zc).externSeqStore.pos < (*zc).externSeqStore.size {
            /* External matchfinder + LDM is technically possible, just not implemented yet.
             * We need to revisit soon and implement it. */
            if ZSTD_hasExtSeqProd(&(*zc).appliedParams) != 0 {
                return ERROR(ZSTD_error_parameter_combination_unsupported);
            }

            /* Updates ldmSeqStore.pos */
            lastLLSize = crate::compress::zstd_ldm::ZSTD_ldm_blockCompress(
                &mut (*zc).externSeqStore,
                ms,
                &mut (*zc).seqStore,
                (*(*zc).blockState.nextCBlock).rep.as_mut_ptr(),
                (*zc).appliedParams.useRowMatchFinder,
                src,
                srcSize,
            );
        } else if (*zc).appliedParams.ldmParams.enableLdm == ZSTD_ps_enable {
            let mut ldmSeqStore: RawSeqStore_t = kNullRawSeqStore;

            /* External matchfinder + LDM is technically possible, just not implemented yet.
             * We need to revisit soon and implement it. */
            if ZSTD_hasExtSeqProd(&(*zc).appliedParams) != 0 {
                return ERROR(ZSTD_error_parameter_combination_unsupported);
            }

            ldmSeqStore.seq = (*zc).ldmSequences;
            ldmSeqStore.capacity = (*zc).maxNbLdmSequences;
            /* Updates ldmSeqStore.size */
            {
                let e = crate::compress::zstd_ldm::ZSTD_ldm_generateSequences(
                    &mut (*zc).ldmState,
                    &mut ldmSeqStore,
                    &(*zc).appliedParams.ldmParams,
                    src,
                    srcSize,
                );
                if ERR_isError(e) != 0 {
                    return e;
                }
            }
            /* Updates ldmSeqStore.pos */
            lastLLSize = crate::compress::zstd_ldm::ZSTD_ldm_blockCompress(
                &mut ldmSeqStore,
                ms,
                &mut (*zc).seqStore,
                (*(*zc).blockState.nextCBlock).rep.as_mut_ptr(),
                (*zc).appliedParams.useRowMatchFinder,
                src,
                srcSize,
            );
        } else if ZSTD_hasExtSeqProd(&(*zc).appliedParams) != 0 {
            {
                let windowSize: U32 = 1u32 << (*zc).appliedParams.cParams.windowLog;

                let nbExternalSeqs: usize =
                    ((*zc).appliedParams.extSeqProdFunc.unwrap_unchecked())(
                        (*zc).appliedParams.extSeqProdState,
                        (*zc).extSeqBuf,
                        (*zc).extSeqBufCapacity,
                        src,
                        srcSize,
                        core::ptr::null(),
                        0, /* dict and dictSize, currently not supported */
                        (*zc).appliedParams.compressionLevel,
                        windowSize as usize,
                    );

                let nbPostProcessedSeqs: usize = ZSTD_postProcessSequenceProducerResult(
                    (*zc).extSeqBuf,
                    nbExternalSeqs,
                    (*zc).extSeqBufCapacity,
                    srcSize,
                );

                /* Return early if there is no error, since we don't need to worry about last literals */
                if ZSTD_isError(nbPostProcessedSeqs) == 0 {
                    let mut seqPos = ZSTD_SequencePosition {
                        idx: 0,
                        posInSequence: 0,
                        posInSrc: 0,
                    };
                    let seqLenSum: usize =
                        ZSTD_fastSequenceLengthSum((*zc).extSeqBuf, nbPostProcessedSeqs);
                    if seqLenSum > srcSize {
                        return ERROR(ZSTD_error_externalSequences_invalid);
                    }
                    {
                        let e = crate::compress::zstd_compress::ZSTD_transferSequences_wBlockDelim(
                            zc,
                            &mut seqPos,
                            (*zc).extSeqBuf,
                            nbPostProcessedSeqs,
                            src,
                            srcSize,
                            (*zc).appliedParams.searchForExternalRepcodes,
                        );
                        if ERR_isError(e) != 0 {
                            return e;
                        }
                    }
                    (*ms).ldmSeqStore = core::ptr::null();
                    return ZSTDbss_compress as usize;
                }

                /* Propagate the error if fallback is disabled */
                if (*zc).appliedParams.enableMatchFinderFallback == 0 {
                    return nbPostProcessedSeqs;
                }

                /* Fallback to software matchfinder */
                {
                    let blockCompressor_: ZSTD_BlockCompressor_f = ZSTD_selectBlockCompressor(
                        (*zc).appliedParams.cParams.strategy,
                        (*zc).appliedParams.useRowMatchFinder,
                        dictMode,
                    );
                    (*ms).ldmSeqStore = core::ptr::null();
                    lastLLSize = (blockCompressor_.unwrap_unchecked())(
                        ms,
                        &mut (*zc).seqStore,
                        (*(*zc).blockState.nextCBlock).rep.as_mut_ptr(),
                        src,
                        srcSize,
                    );
                }
            }
        } else {
            /* not long range mode and no external matchfinder */
            let blockCompressor_: ZSTD_BlockCompressor_f = ZSTD_selectBlockCompressor(
                (*zc).appliedParams.cParams.strategy,
                (*zc).appliedParams.useRowMatchFinder,
                dictMode,
            );
            (*ms).ldmSeqStore = core::ptr::null();
            lastLLSize = (blockCompressor_.unwrap_unchecked())(
                ms,
                &mut (*zc).seqStore,
                (*(*zc).blockState.nextCBlock).rep.as_mut_ptr(),
                src,
                srcSize,
            );
        }
        {
            let lastLiterals: *const BYTE = (src as *const BYTE).add(srcSize).sub(lastLLSize);
            ZSTD_storeLastLiterals(&mut (*zc).seqStore, lastLiterals, lastLLSize);
        }
    }
    ZSTD_validateSeqStore(&(*zc).seqStore, &(*zc).appliedParams.cParams);
    ZSTDbss_compress as usize
}

pub unsafe fn ZSTD_copyBlockSequences(
    seqCollector: *mut SeqCollector,
    seqStore: *const SeqStore_t,
    prevRepcodes: *const U32,
) -> usize {
    let inSeqs: *const SeqDef = (*seqStore).sequencesStart;
    let nbInSequences: usize = (*seqStore).sequences.offset_from(inSeqs) as usize;
    let nbInLiterals: usize = (*seqStore).lit.offset_from((*seqStore).litStart) as usize;

    let outSeqs: *mut ZSTD_Sequence = if (*seqCollector).seqIndex == 0 {
        (*seqCollector).seqStart
    } else {
        (*seqCollector).seqStart.add((*seqCollector).seqIndex)
    };
    let nbOutSequences: usize = nbInSequences + 1;
    let mut nbOutLiterals: usize = 0;
    let mut repcodes = Repcodes_t::default();
    let mut i: usize;

    /* Bounds check that we have enough space for every input sequence
     * and the block delimiter
     */
    if nbOutSequences > ((*seqCollector).maxSequences.wrapping_sub((*seqCollector).seqIndex)) {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }

    ZSTD_memcpy(
        &mut repcodes as *mut Repcodes_t as *mut c_void,
        prevRepcodes as *const c_void,
        core::mem::size_of::<Repcodes_t>(),
    );
    i = 0;
    while i < nbInSequences {
        let rawOffset: U32;
        (*outSeqs.add(i)).litLength = (*inSeqs.add(i)).litLength as c_uint;
        (*outSeqs.add(i)).matchLength = ((*inSeqs.add(i)).mlBase as c_uint)
            .wrapping_add(MINMATCH as c_uint);
        (*outSeqs.add(i)).rep = 0;

        /* Handle the possible single length >= 64K
         * There can only be one because we add MINMATCH to every match length,
         * and blocks are at most 128K.
         */
        if i == (*seqStore).longLengthPos as usize {
            if (*seqStore).longLengthType == ZSTD_llt_literalLength {
                (*outSeqs.add(i)).litLength += 0x10000;
            } else if (*seqStore).longLengthType == ZSTD_llt_matchLength {
                (*outSeqs.add(i)).matchLength += 0x10000;
            }
        }

        /* Determine the raw offset given the offBase, which may be a repcode. */
        if OFFBASE_IS_REPCODE((*inSeqs.add(i)).offBase) {
            let repcode: U32 = OFFBASE_TO_REPCODE((*inSeqs.add(i)).offBase);
            (*outSeqs.add(i)).rep = repcode;
            if (*outSeqs.add(i)).litLength != 0 {
                rawOffset = repcodes.rep[(repcode - 1) as usize];
            } else {
                if repcode == 3 {
                    rawOffset = repcodes.rep[0] - 1;
                } else {
                    rawOffset = repcodes.rep[repcode as usize];
                }
            }
        } else {
            rawOffset = OFFBASE_TO_OFFSET((*inSeqs.add(i)).offBase);
        }
        (*outSeqs.add(i)).offset = rawOffset;

        /* Update repcode history for the sequence */
        ZSTD_updateRep(
            repcodes.rep.as_mut_ptr(),
            (*inSeqs.add(i)).offBase,
            ((*inSeqs.add(i)).litLength == 0) as U32,
        );

        nbOutLiterals = nbOutLiterals.wrapping_add((*outSeqs.add(i)).litLength as usize);
        i += 1;
    }
    /* Insert last literals (if any exist) in the block as a sequence with ml == off == 0.
     * If there are no last literals, then we'll emit (of: 0, ml: 0, ll: 0), which is a marker
     * for the block boundary, according to the API.
     */
    {
        let lastLLSize: usize = nbInLiterals.wrapping_sub(nbOutLiterals);
        (*outSeqs.add(nbInSequences)).litLength = lastLLSize as U32;
        (*outSeqs.add(nbInSequences)).matchLength = 0;
        (*outSeqs.add(nbInSequences)).offset = 0;
    }
    (*seqCollector).seqIndex += nbOutSequences;

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_sequenceBound(srcSize: usize) -> usize {
    let maxNbSeq: usize = (srcSize / ZSTD_MINMATCH_MIN as usize) + 1;
    let maxNbDelims: usize = (srcSize / ZSTD_BLOCKSIZE_MAX_MIN) + 1;
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
    let dstCapacity: usize = crate::compress::zstd_compress::ZSTD_compressBound(srcSize);
    let dst: *mut c_void; /* Make C90 happy. */
    let mut seqCollector = SeqCollector::default();
    {
        let mut targetCBlockSize: c_int = 0;
        {
            let e = crate::compress::zstd_compress::ZSTD_CCtx_getParameter(
                zc,
                ZSTD_c_targetCBlockSize,
                &mut targetCBlockSize,
            );
            if ERR_isError(e) != 0 {
                return e;
            }
        }
        if targetCBlockSize != 0 {
            return ERROR(ZSTD_error_parameter_unsupported);
        }
    }
    {
        let mut nbWorkers: c_int = 0;
        {
            let e = crate::compress::zstd_compress::ZSTD_CCtx_getParameter(
                zc,
                ZSTD_c_nbWorkers,
                &mut nbWorkers,
            );
            if ERR_isError(e) != 0 {
                return e;
            }
        }
        if nbWorkers != 0 {
            return ERROR(ZSTD_error_parameter_unsupported);
        }
    }

    dst = ZSTD_customMalloc(
        dstCapacity,
        crate::compress::zstd_compress::ZSTD_defaultCMem,
    );
    if dst.is_null() {
        return ERROR(ZSTD_error_memory_allocation);
    }

    seqCollector.collectSequences = 1;
    seqCollector.seqStart = outSeqs;
    seqCollector.seqIndex = 0;
    seqCollector.maxSequences = outSeqsSize;
    (*zc).seqCollector = seqCollector;

    {
        let ret: usize =
            crate::compress::zstd_compress::ZSTD_compress2(zc, dst, dstCapacity, src, srcSize);
        ZSTD_customFree(dst, crate::compress::zstd_compress::ZSTD_defaultCMem);
        if ERR_isError(ret) != 0 {
            return ret;
        }
    }
    (*zc).seqCollector.seqIndex
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_mergeBlockDelimiters(
    sequences: *mut ZSTD_Sequence,
    seqsSize: usize,
) -> usize {
    let mut i_n: usize = 0;
    let mut out: usize = 0;
    while i_n < seqsSize {
        if (*sequences.add(i_n)).offset == 0 && (*sequences.add(i_n)).matchLength == 0 {
            if i_n != seqsSize - 1 {
                (*sequences.add(i_n + 1)).litLength += (*sequences.add(i_n)).litLength;
            }
        } else {
            *sequences.add(out) = *sequences.add(i_n);
            out += 1;
        }
        i_n += 1;
    }
    out
}
