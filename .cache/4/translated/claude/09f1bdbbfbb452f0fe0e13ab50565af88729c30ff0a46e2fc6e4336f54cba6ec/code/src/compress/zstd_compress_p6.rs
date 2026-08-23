/* zstd_compress.c — part 6 (C lines 5926..end)
 *
 * Translated from compress/zstd_compress.c.
 * This file is `include!`d into `crate::compress::zstd_compress`; it must
 * contain items only (no `use`, no `extern "C"` blocks).
 */

/* from compress/zstd_lazy.h */
const ZSTD_LAZY_DDSS_BUCKET_LOG: U32 = 2;

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
    workspaceSize: usize,
) -> *mut ZSTD_CStream {
    ZSTD_initStaticCCtx(workspace, workspaceSize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_createCStream_advanced(
    customMem: ZSTD_customMem,
) -> *mut ZSTD_CStream {
    /* CStream and CCtx are now same object */
    ZSTD_createCCtx_advanced(customMem)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_freeCStream(zcs: *mut ZSTD_CStream) -> usize {
    ZSTD_freeCCtx(zcs) /* same object */
}

/*======   Initialization   ======*/

#[unsafe(no_mangle)]
pub extern "C" fn ZSTD_CStreamInSize() -> usize {
    ZSTD_BLOCKSIZE_MAX
}

#[unsafe(no_mangle)]
pub extern "C" fn ZSTD_CStreamOutSize() -> usize {
    ZSTD_compressBound(ZSTD_BLOCKSIZE_MAX) + ZSTD_blockHeaderSize + 4 /* 32-bits hash */
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
        let err_code = ZSTD_CCtx_reset(zcs, ZSTD_reset_session_only);
        if ERR_isError(err_code) != 0 {
            return err_code;
        }
    }
    {
        let err_code = ZSTD_CCtx_setPledgedSrcSize(zcs, pledgedSrcSize);
        if ERR_isError(err_code) != 0 {
            return err_code;
        }
    }
    0
}

/*  ZSTD_initCStream_internal() :
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
        let err_code = ZSTD_CCtx_reset(zcs, ZSTD_reset_session_only);
        if ERR_isError(err_code) != 0 {
            return err_code;
        }
    }
    {
        let err_code = ZSTD_CCtx_setPledgedSrcSize(zcs, pledgedSrcSize);
        if ERR_isError(err_code) != 0 {
            return err_code;
        }
    }
    (*zcs).requestedParams = *params;
    if !dict.is_null() {
        let err_code = ZSTD_CCtx_loadDictionary(zcs, dict, dictSize);
        if ERR_isError(err_code) != 0 {
            return err_code;
        }
    } else {
        /* Dictionary is cleared if !cdict */
        let err_code = ZSTD_CCtx_refCDict(zcs, cdict);
        if ERR_isError(err_code) != 0 {
            return err_code;
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
        let err_code = ZSTD_CCtx_reset(zcs, ZSTD_reset_session_only);
        if ERR_isError(err_code) != 0 {
            return err_code;
        }
    }
    {
        let err_code = ZSTD_CCtx_setPledgedSrcSize(zcs, pledgedSrcSize);
        if ERR_isError(err_code) != 0 {
            return err_code;
        }
    }
    (*zcs).requestedParams.fParams = fParams;
    {
        let err_code = ZSTD_CCtx_refCDict(zcs, cdict);
        if ERR_isError(err_code) != 0 {
            return err_code;
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
        let err_code = ZSTD_CCtx_reset(zcs, ZSTD_reset_session_only);
        if ERR_isError(err_code) != 0 {
            return err_code;
        }
    }
    {
        let err_code = ZSTD_CCtx_refCDict(zcs, cdict);
        if ERR_isError(err_code) != 0 {
            return err_code;
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
        let err_code = ZSTD_CCtx_reset(zcs, ZSTD_reset_session_only);
        if ERR_isError(err_code) != 0 {
            return err_code;
        }
    }
    {
        let err_code = ZSTD_CCtx_setPledgedSrcSize(zcs, pledgedSrcSize);
        if ERR_isError(err_code) != 0 {
            return err_code;
        }
    }
    {
        let err_code = ZSTD_checkCParams(params.cParams);
        if ERR_isError(err_code) != 0 {
            return err_code;
        }
    }
    ZSTD_CCtxParams_setZstdParams(&mut (*zcs).requestedParams, &params);
    {
        let err_code = ZSTD_CCtx_loadDictionary(zcs, dict, dictSize);
        if ERR_isError(err_code) != 0 {
            return err_code;
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
        let err_code = ZSTD_CCtx_reset(zcs, ZSTD_reset_session_only);
        if ERR_isError(err_code) != 0 {
            return err_code;
        }
    }
    {
        let err_code = ZSTD_CCtx_setParameter(zcs, ZSTD_c_compressionLevel, compressionLevel);
        if ERR_isError(err_code) != 0 {
            return err_code;
        }
    }
    {
        let err_code = ZSTD_CCtx_loadDictionary(zcs, dict, dictSize);
        if ERR_isError(err_code) != 0 {
            return err_code;
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
        let err_code = ZSTD_CCtx_reset(zcs, ZSTD_reset_session_only);
        if ERR_isError(err_code) != 0 {
            return err_code;
        }
    }
    {
        let err_code = ZSTD_CCtx_refCDict(zcs, core::ptr::null());
        if ERR_isError(err_code) != 0 {
            return err_code;
        }
    }
    {
        let err_code = ZSTD_CCtx_setParameter(zcs, ZSTD_c_compressionLevel, compressionLevel);
        if ERR_isError(err_code) != 0 {
            return err_code;
        }
    }
    {
        let err_code = ZSTD_CCtx_setPledgedSrcSize(zcs, pledgedSrcSize);
        if ERR_isError(err_code) != 0 {
            return err_code;
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
        let err_code = ZSTD_CCtx_reset(zcs, ZSTD_reset_session_only);
        if ERR_isError(err_code) != 0 {
            return err_code;
        }
    }
    {
        let err_code = ZSTD_CCtx_refCDict(zcs, core::ptr::null());
        if ERR_isError(err_code) != 0 {
            return err_code;
        }
    }
    {
        let err_code = ZSTD_CCtx_setParameter(zcs, ZSTD_c_compressionLevel, compressionLevel);
        if ERR_isError(err_code) != 0 {
            return err_code;
        }
    }
    0
}

/*======   Compression   ======*/

unsafe fn ZSTD_nextInputSizeHint(cctx: *const ZSTD_CCtx) -> usize {
    if (*cctx).appliedParams.inBufferMode == ZSTD_bm_stable {
        return (*cctx).blockSizeMax - (*cctx).stableIn_notConsumed;
    }
    {
        let mut hintInSize: usize = (*cctx).inBuffTarget - (*cctx).inBuffPos;
        if hintInSize == 0 {
            hintInSize = (*cctx).blockSizeMax;
        }
        hintInSize
    }
}

/* ZSTD_compressStream_generic():
 *  internal function for all *compressStream*() variants
 * @return : hint size for next input to complete ongoing block */
unsafe fn ZSTD_compressStream_generic(
    zcs: *mut ZSTD_CStream,
    output: *mut ZSTD_outBuffer,
    input: *mut ZSTD_inBuffer,
    flushMode: ZSTD_EndDirective,
) -> usize {
    let istart: *const c_char = (*input).src as *const c_char;
    let iend: *const c_char = if !istart.is_null() {
        istart.add((*input).size)
    } else {
        istart
    };
    let mut ip: *const c_char = if !istart.is_null() {
        istart.add((*input).pos)
    } else {
        istart
    };
    let ostart: *mut c_char = (*output).dst as *mut c_char;
    let oend: *mut c_char = if !ostart.is_null() {
        ostart.add((*output).size)
    } else {
        ostart
    };
    let mut op: *mut c_char = if !ostart.is_null() {
        ostart.add((*output).pos)
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
        let stage: ZSTD_cStreamStage = (*zcs).streamStage;
        if stage == zcss_init {
            return ERROR(ZSTD_error_init_missing);
        }
        'stage: {
            if stage == zcss_load {
                if flushMode == ZSTD_e_end
                    && (oend.offset_from(op) as usize
                        >= ZSTD_compressBound(iend.offset_from(ip) as usize) /* Enough output space */
                        || (*zcs).appliedParams.outBufferMode == ZSTD_bm_stable)
                    /* OR we are allowed to return dstSizeTooSmall */
                    && (*zcs).inBuffPos == 0
                {
                    /* shortcut to compression pass directly into output buffer */
                    let cSize: usize = ZSTD_compressEnd_public(
                        zcs,
                        op as *mut c_void,
                        oend.offset_from(op) as usize,
                        ip as *const c_void,
                        iend.offset_from(ip) as usize,
                    );
                    if ERR_isError(cSize) != 0 {
                        return cSize;
                    }
                    ip = iend;
                    op = op.add(cSize);
                    (*zcs).frameEnded = 1;
                    ZSTD_CCtx_reset(zcs, ZSTD_reset_session_only);
                    someMoreWork = 0;
                    break 'stage;
                }
                /* complete loading into inBuffer in buffered mode */
                if (*zcs).appliedParams.inBufferMode == ZSTD_bm_buffered {
                    let toLoad: usize = (*zcs).inBuffTarget - (*zcs).inBuffPos;
                    let loaded: usize = ZSTD_limitCopy(
                        (*zcs).inBuff.add((*zcs).inBuffPos) as *mut c_void,
                        toLoad,
                        ip as *const c_void,
                        iend.offset_from(ip) as usize,
                    );
                    (*zcs).inBuffPos += loaded;
                    if !ip.is_null() {
                        ip = ip.add(loaded);
                    }
                    if flushMode == ZSTD_e_continue && (*zcs).inBuffPos < (*zcs).inBuffTarget {
                        /* not enough input to fill full block : stop here */
                        someMoreWork = 0;
                        break 'stage;
                    }
                    if flushMode == ZSTD_e_flush && (*zcs).inBuffPos == (*zcs).inToCompress {
                        /* empty */
                        someMoreWork = 0;
                        break 'stage;
                    }
                } else {
                    if flushMode == ZSTD_e_continue
                        && (iend.offset_from(ip) as usize) < (*zcs).blockSizeMax
                    {
                        /* can't compress a full block : stop here */
                        (*zcs).stableIn_notConsumed = iend.offset_from(ip) as usize;
                        ip = iend; /* pretend to have consumed input */
                        someMoreWork = 0;
                        break 'stage;
                    }
                    if flushMode == ZSTD_e_flush && ip == iend {
                        /* empty */
                        someMoreWork = 0;
                        break 'stage;
                    }
                }
                /* compress current block (note : this stage cannot be stopped in the middle) */
                {
                    let inputBuffered: c_int =
                        ((*zcs).appliedParams.inBufferMode == ZSTD_bm_buffered) as c_int;
                    let cDst: *mut c_void;
                    let cSize: usize;
                    let mut oSize: usize = oend.offset_from(op) as usize;
                    let iSize: usize = if inputBuffered != 0 {
                        (*zcs).inBuffPos - (*zcs).inToCompress
                    } else {
                        MIN(iend.offset_from(ip) as usize, (*zcs).blockSizeMax)
                    };
                    if oSize >= ZSTD_compressBound(iSize)
                        || (*zcs).appliedParams.outBufferMode == ZSTD_bm_stable
                    {
                        cDst = op as *mut c_void; /* compress into output buffer, to skip flush stage */
                    } else {
                        cDst = (*zcs).outBuff as *mut c_void;
                        oSize = (*zcs).outBuffSize;
                    }
                    if inputBuffered != 0 {
                        let lastBlock: c_uint =
                            ((flushMode == ZSTD_e_end) && (ip == iend)) as c_uint;
                        cSize = if lastBlock != 0 {
                            ZSTD_compressEnd_public(
                                zcs,
                                cDst,
                                oSize,
                                (*zcs).inBuff.add((*zcs).inToCompress) as *const c_void,
                                iSize,
                            )
                        } else {
                            ZSTD_compressContinue_public(
                                zcs,
                                cDst,
                                oSize,
                                (*zcs).inBuff.add((*zcs).inToCompress) as *const c_void,
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
                        let lastBlock: c_uint =
                            ((flushMode == ZSTD_e_end) && (ip.add(iSize) == iend)) as c_uint;
                        cSize = if lastBlock != 0 {
                            ZSTD_compressEnd_public(
                                zcs,
                                cDst,
                                oSize,
                                ip as *const c_void,
                                iSize,
                            )
                        } else {
                            ZSTD_compressContinue_public(
                                zcs,
                                cDst,
                                oSize,
                                ip as *const c_void,
                                iSize,
                            )
                        };
                        /* Consume the input prior to error checking to mirror buffered mode. */
                        if !ip.is_null() {
                            ip = ip.add(iSize);
                        }
                        if ERR_isError(cSize) != 0 {
                            return cSize;
                        }
                        (*zcs).frameEnded = lastBlock;
                    }
                    if cDst == op as *mut c_void {
                        /* no need to flush */
                        op = op.add(cSize);
                        if (*zcs).frameEnded != 0 {
                            someMoreWork = 0;
                            ZSTD_CCtx_reset(zcs, ZSTD_reset_session_only);
                        }
                        break 'stage;
                    }
                    (*zcs).outBuffContentSize = cSize;
                    (*zcs).outBuffFlushedSize = 0;
                    (*zcs).streamStage = zcss_flush; /* pass-through to flush stage */
                }
            } else if stage != zcss_flush {
                /* default : impossible */
                break 'stage;
            }
            /* case zcss_flush: */
            {
                let toFlush: usize = (*zcs).outBuffContentSize - (*zcs).outBuffFlushedSize;
                let flushed: usize = ZSTD_limitCopy(
                    op as *mut c_void,
                    oend.offset_from(op) as usize,
                    (*zcs).outBuff.add((*zcs).outBuffFlushedSize) as *const c_void,
                    toFlush,
                );
                if flushed != 0 {
                    op = op.add(flushed);
                }
                (*zcs).outBuffFlushedSize += flushed;
                if toFlush != flushed {
                    /* flush not fully completed, presumably because dst is too small */
                    someMoreWork = 0;
                    break 'stage;
                }
                (*zcs).outBuffFlushedSize = 0;
                (*zcs).outBuffContentSize = 0;
                if (*zcs).frameEnded != 0 {
                    someMoreWork = 0;
                    ZSTD_CCtx_reset(zcs, ZSTD_reset_session_only);
                    break 'stage;
                }
                (*zcs).streamStage = zcss_load;
                break 'stage;
            }
        }
    }

    (*input).pos = ip.offset_from(istart) as usize;
    (*output).pos = op.offset_from(ostart) as usize;
    if (*zcs).frameEnded != 0 {
        return 0;
    }
    ZSTD_nextInputSizeHint(zcs)
}

unsafe fn ZSTD_nextInputSizeHint_MTorST(cctx: *const ZSTD_CCtx) -> usize {
    ZSTD_nextInputSizeHint(cctx)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressStream(
    zcs: *mut ZSTD_CStream,
    output: *mut ZSTD_outBuffer,
    input: *mut ZSTD_inBuffer,
) -> usize {
    {
        let err_code = ZSTD_compressStream2(zcs, output, input, ZSTD_e_continue);
        if ERR_isError(err_code) != 0 {
            return err_code;
        }
    }
    ZSTD_nextInputSizeHint_MTorST(zcs)
}

/* After a compression call set the expected input/output buffer.
 * This is validated at the start of the next compression call.
 */
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

/* Validate that the input/output buffers match the expectations set by
 * ZSTD_setBufferExpectations.
 */
unsafe fn ZSTD_checkBufferStability(
    cctx: *const ZSTD_CCtx,
    output: *const ZSTD_outBuffer,
    input: *const ZSTD_inBuffer,
    endOp: ZSTD_EndDirective,
) -> usize {
    if (*cctx).appliedParams.inBufferMode == ZSTD_bm_stable {
        let expect: ZSTD_inBuffer = (*cctx).expectedInBuffer;
        if expect.src != (*input).src || expect.pos != (*input).pos {
            return ERROR(ZSTD_error_stabilityCondition_notRespected);
        }
    }
    let _ = endOp;
    if (*cctx).appliedParams.outBufferMode == ZSTD_bm_stable {
        let outBufferSize: usize = (*output).size - (*output).pos;
        if (*cctx).expectedOutBufferSize != outBufferSize {
            return ERROR(ZSTD_error_stabilityCondition_notRespected);
        }
    }
    0
}

/*
 * If @endOp == ZSTD_e_end, @inSize becomes pledgedSrcSize.
 * Otherwise, it's ignored.
 * @return: 0 on success, or a ZSTD_error code otherwise.
 */
unsafe fn ZSTD_CCtx_init_compressStream2(
    cctx: *mut ZSTD_CCtx,
    endOp: ZSTD_EndDirective,
    inSize: usize,
) -> usize {
    let mut params: ZSTD_CCtx_params = (*cctx).requestedParams;
    let prefixDict: ZSTD_prefixDict = (*cctx).prefixDict;
    {
        let err_code = ZSTD_initLocalDict(cctx); /* Init the local dict if present. */
        if ERR_isError(err_code) != 0 {
            return err_code;
        }
    }
    ZSTD_memset(
        core::ptr::addr_of_mut!((*cctx).prefixDict) as *mut c_void,
        0,
        core::mem::size_of::<ZSTD_prefixDict>(),
    ); /* single usage */
    if !(*cctx).cdict.is_null() && (*cctx).localDict.cdict.is_null() {
        /* Let the cdict's compression level take priority over the requested params.
         * But do not take the cdict's compression level if the "cdict" is actually a localDict
         * generated from ZSTD_initLocalDict().
         */
        params.compressionLevel = (*(*cctx).cdict).compressionLevel;
    }
    if endOp == ZSTD_e_end {
        (*cctx).pledgedSrcSizePlusOne = (inSize as u64).wrapping_add(1);
        /* auto-determine pledgedSrcSize */
    }

    {
        let dictSize: usize = if !prefixDict.dict.is_null() {
            prefixDict.dictSize
        } else if !(*cctx).cdict.is_null() {
            (*(*cctx).cdict).dictContentSize
        } else {
            0
        };
        let mode: ZSTD_CParamMode_e = ZSTD_getCParamMode(
            (*cctx).cdict,
            &params,
            (*cctx).pledgedSrcSizePlusOne.wrapping_sub(1),
        );
        params.cParams = ZSTD_getCParamsFromCCtxParams(
            &params,
            (*cctx).pledgedSrcSizePlusOne.wrapping_sub(1),
            dictSize,
            mode,
        );
    }

    params.postBlockSplitter =
        ZSTD_resolveBlockSplitterMode(params.postBlockSplitter, &params.cParams);
    params.ldmParams.enableLdm = ZSTD_resolveEnableLdm(params.ldmParams.enableLdm, &params.cParams);
    params.useRowMatchFinder =
        ZSTD_resolveRowMatchFinderMode(params.useRowMatchFinder, &params.cParams);
    params.validateSequences = ZSTD_resolveExternalSequenceValidation(params.validateSequences);
    params.maxBlockSize = ZSTD_resolveMaxBlockSize(params.maxBlockSize);
    params.searchForExternalRepcodes = ZSTD_resolveExternalRepcodeSearch(
        params.searchForExternalRepcodes,
        params.compressionLevel,
    );

    {
        let pledgedSrcSize: U64 = (*cctx).pledgedSrcSizePlusOne.wrapping_sub(1);
        {
            let err_code = ZSTD_compressBegin_internal(
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
            if ERR_isError(err_code) != 0 {
                return err_code;
            }
        }
        (*cctx).inToCompress = 0;
        (*cctx).inBuffPos = 0;
        if (*cctx).appliedParams.inBufferMode == ZSTD_bm_buffered {
            /* for small input: avoid automatic flush on reaching end of block, since
             * it would require to add a 3-bytes null block to end frame
             */
            (*cctx).inBuffTarget = (*cctx).blockSizeMax
                + ((*cctx).blockSizeMax as U64 == pledgedSrcSize) as usize;
        } else {
            (*cctx).inBuffTarget = 0;
        }
        (*cctx).outBuffFlushedSize = 0;
        (*cctx).outBuffContentSize = 0;
        (*cctx).streamStage = zcss_load;
        (*cctx).frameEnded = 0;
    }
    0
}

/* @return provides a minimum amount of data remaining to be flushed from internal buffers
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressStream2(
    cctx: *mut ZSTD_CCtx,
    output: *mut ZSTD_outBuffer,
    input: *mut ZSTD_inBuffer,
    endOp: ZSTD_EndDirective,
) -> usize {
    /* check conditions */
    if (*output).pos > (*output).size {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    if (*input).pos > (*input).size {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    if endOp as U32 > ZSTD_e_end as U32 {
        return ERROR(ZSTD_error_parameter_outOfBound);
    }

    /* transparent initialization stage */
    if (*cctx).streamStage == zcss_init {
        let inputSize: usize = (*input).size - (*input).pos; /* no obligation to start from pos==0 */
        let totalInputSize: usize = inputSize + (*cctx).stableIn_notConsumed;
        if (*cctx).requestedParams.inBufferMode == ZSTD_bm_stable /* input is presumed stable, across invocations */
            && endOp == ZSTD_e_continue /* no flush requested, more input to come */
            && totalInputSize < ZSTD_BLOCKSIZE_MAX
        {
            /* not even reached one block yet */
            if (*cctx).stableIn_notConsumed != 0 {
                /* not the first time */
                /* check stable source guarantees */
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
            /* but actually input wasn't consumed, so keep track of position from where compression shall resume */
            (*cctx).stableIn_notConsumed += inputSize;
            /* don't initialize yet, wait for the first block of flush() order, for better parameters adaptation */
            return ZSTD_FRAMEHEADERSIZE_MIN((*cctx).requestedParams.format);
            /* at least some header to produce */
        }
        {
            let err_code = ZSTD_CCtx_init_compressStream2(cctx, endOp, totalInputSize);
            if ERR_isError(err_code) != 0 {
                return err_code;
            }
        }
        ZSTD_setBufferExpectations(cctx, output, input);
        /* Set initial buffer expectations now that we've initialized */
    }
    /* end of transparent initialization stage */

    {
        let err_code = ZSTD_checkBufferStability(cctx, output, input, endOp);
        if ERR_isError(err_code) != 0 {
            return err_code;
        }
    }
    /* compression stage */
    {
        let err_code = ZSTD_compressStream_generic(cctx, output, input, endOp);
        if ERR_isError(err_code) != 0 {
            return err_code;
        }
    }
    ZSTD_setBufferExpectations(cctx, output, input);
    (*cctx).outBuffContentSize - (*cctx).outBuffFlushedSize /* remaining to flush */
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressStream2_simpleArgs(
    cctx: *mut ZSTD_CCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    dstPos: *mut usize,
    src: *const c_void,
    srcSize: usize,
    srcPos: *mut usize,
    endOp: ZSTD_EndDirective,
) -> usize {
    let mut output: ZSTD_outBuffer = ZSTD_outBuffer {
        dst: core::ptr::null_mut(),
        size: 0,
        pos: 0,
    };
    let mut input: ZSTD_inBuffer = ZSTD_inBuffer {
        src: core::ptr::null(),
        size: 0,
        pos: 0,
    };
    output.dst = dst;
    output.size = dstCapacity;
    output.pos = *dstPos;
    input.src = src;
    input.size = srcSize;
    input.pos = *srcPos;
    /* ZSTD_compressStream2() will check validity of dstPos and srcPos */
    {
        let cErr: usize = ZSTD_compressStream2(cctx, &mut output, &mut input, endOp);
        *dstPos = output.pos;
        *srcPos = input.pos;
        cErr
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compress2(
    cctx: *mut ZSTD_CCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let originalInBufferMode: ZSTD_bufferMode_e = (*cctx).requestedParams.inBufferMode;
    let originalOutBufferMode: ZSTD_bufferMode_e = (*cctx).requestedParams.outBufferMode;
    ZSTD_CCtx_reset(cctx, ZSTD_reset_session_only);
    /* Enable stable input/output buffers. */
    (*cctx).requestedParams.inBufferMode = ZSTD_bm_stable;
    (*cctx).requestedParams.outBufferMode = ZSTD_bm_stable;
    {
        let mut oPos: usize = 0;
        let mut iPos: usize = 0;
        let result: usize = ZSTD_compressStream2_simpleArgs(
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

/* ZSTD_validateSequence() :
 * @offBase : must use the format required by ZSTD_storeSeq()
 * @returns a ZSTD error code if sequence is not valid
 */
unsafe fn ZSTD_validateSequence(
    offBase: U32,
    matchLength: U32,
    minMatch: U32,
    posInSrc: usize,
    windowLog: U32,
    dictSize: usize,
    useSequenceProducer: c_int,
) -> usize {
    let windowSize: U32 = 1u32 << windowLog;
    /* posInSrc represents the amount of data the decoder would decode up to this point.
     * As long as the amount of data decoded is less than or equal to window size, offsets may be
     * larger than the total length of output decoded in order to reference the dict, even larger than
     * window size. After output surpasses windowSize, we're limited to windowSize offsets again.
     */
    let offsetBound: usize = if posInSrc > windowSize as usize {
        windowSize as usize
    } else {
        posInSrc + dictSize
    };
    let matchLenLowerBound: usize = if minMatch == 3 || useSequenceProducer != 0 {
        3
    } else {
        4
    };
    if (offBase as usize) > offsetBound.wrapping_add(ZSTD_REP_NUM as usize) {
        return ERROR(ZSTD_error_externalSequences_invalid);
    }
    /* Validate maxNbSeq is large enough for the given matchLength and minMatch */
    if (matchLength as usize) < matchLenLowerBound {
        return ERROR(ZSTD_error_externalSequences_invalid);
    }
    0
}

/* Returns an offset code, given a sequence's raw offset, the ongoing repcode array, and whether litLength == 0 */
unsafe fn ZSTD_finalizeOffBase(rawOffset: U32, rep: *const U32, ll0: U32) -> U32 {
    let mut offBase: U32 = OFFSET_TO_OFFBASE(rawOffset);

    if ll0 == 0 && rawOffset == *rep.add(0) {
        offBase = REPCODE1_TO_OFFBASE;
    } else if rawOffset == *rep.add(1) {
        offBase = REPCODE_TO_OFFBASE(2u32.wrapping_sub(ll0));
    } else if rawOffset == *rep.add(2) {
        offBase = REPCODE_TO_OFFBASE(3u32.wrapping_sub(ll0));
    } else if ll0 != 0 && rawOffset == (*rep.add(0)).wrapping_sub(1) {
        offBase = REPCODE3_TO_OFFBASE;
    }
    offBase
}

/* This function scans through an array of ZSTD_Sequence,
 * storing the sequences it reads, until it reaches a block delimiter.
 * Note that the block delimiter includes the last literals of the block.
 * @blockSize must be == sum(sequence_lengths).
 * @returns @blockSize on success, and a ZSTD_error otherwise.
 */
unsafe fn ZSTD_transferSequences_wBlockDelim(
    cctx: *mut ZSTD_CCtx,
    seqPos: *mut ZSTD_SequencePosition,
    inSeqs: *const ZSTD_Sequence,
    inSeqsSize: usize,
    src: *const c_void,
    blockSize: usize,
    externalRepSearch: ZSTD_ParamSwitch_e,
) -> usize {
    let mut idx: U32 = (*seqPos).idx;
    let startIdx: U32 = idx;
    let mut ip: *const BYTE = src as *const BYTE;
    let iend: *const BYTE = ip.add(blockSize);
    let mut updatedRepcodes: Repcodes_t = Repcodes_t::default();
    let dictSize: U32;

    if !(*cctx).cdict.is_null() {
        dictSize = (*(*cctx).cdict).dictContentSize as U32;
    } else if !(*cctx).prefixDict.dict.is_null() {
        dictSize = (*cctx).prefixDict.dictSize as U32;
    } else {
        dictSize = 0;
    }
    ZSTD_memcpy(
        updatedRepcodes.rep.as_mut_ptr() as *mut c_void,
        (*(*cctx).blockState.prevCBlock).rep.as_ptr() as *const c_void,
        core::mem::size_of::<Repcodes_t>(),
    );
    while (idx as usize) < inSeqsSize
        && ((*inSeqs.add(idx as usize)).matchLength != 0
            || (*inSeqs.add(idx as usize)).offset != 0)
    {
        let litLength: U32 = (*inSeqs.add(idx as usize)).litLength;
        let matchLength: U32 = (*inSeqs.add(idx as usize)).matchLength;
        let offBase: U32;

        if externalRepSearch == ZSTD_ps_disable {
            offBase = OFFSET_TO_OFFBASE((*inSeqs.add(idx as usize)).offset);
        } else {
            let ll0: U32 = (litLength == 0) as U32;
            offBase = ZSTD_finalizeOffBase(
                (*inSeqs.add(idx as usize)).offset,
                updatedRepcodes.rep.as_ptr(),
                ll0,
            );
            ZSTD_updateRep(updatedRepcodes.rep.as_mut_ptr(), offBase, ll0);
        }

        if (*cctx).appliedParams.validateSequences != 0 {
            (*seqPos).posInSrc += (litLength + matchLength) as usize;
            let err_code = ZSTD_validateSequence(
                offBase,
                matchLength,
                (*cctx).appliedParams.cParams.minMatch,
                (*seqPos).posInSrc,
                (*cctx).appliedParams.cParams.windowLog,
                dictSize as usize,
                ZSTD_hasExtSeqProd(&(*cctx).appliedParams),
            );
            if ERR_isError(err_code) != 0 {
                return err_code;
            }
        }
        if idx.wrapping_sub((*seqPos).idx) as usize >= (*cctx).seqStore.maxNbSeq {
            return ERROR(ZSTD_error_externalSequences_invalid);
        }
        ZSTD_storeSeq(
            &mut (*cctx).seqStore,
            litLength as usize,
            ip,
            iend,
            offBase,
            matchLength as usize,
        );
        ip = ip.add((matchLength + litLength) as usize);
        idx = idx.wrapping_add(1);
    }
    if idx as usize == inSeqsSize {
        return ERROR(ZSTD_error_externalSequences_invalid);
    }

    /* If we skipped repcode search while parsing, we need to update repcodes now */
    if externalRepSearch == ZSTD_ps_disable && idx != startIdx {
        let rep: *mut U32 = updatedRepcodes.rep.as_mut_ptr();
        let lastSeqIdx: U32 = idx.wrapping_sub(1); /* index of last non-block-delimiter sequence */

        if lastSeqIdx >= startIdx.wrapping_add(2) {
            *rep.add(2) = (*inSeqs.add(lastSeqIdx.wrapping_sub(2) as usize)).offset;
            *rep.add(1) = (*inSeqs.add(lastSeqIdx.wrapping_sub(1) as usize)).offset;
            *rep.add(0) = (*inSeqs.add(lastSeqIdx as usize)).offset;
        } else if lastSeqIdx == startIdx.wrapping_add(1) {
            *rep.add(2) = *rep.add(0);
            *rep.add(1) = (*inSeqs.add(lastSeqIdx.wrapping_sub(1) as usize)).offset;
            *rep.add(0) = (*inSeqs.add(lastSeqIdx as usize)).offset;
        } else {
            *rep.add(2) = *rep.add(1);
            *rep.add(1) = *rep.add(0);
            *rep.add(0) = (*inSeqs.add(lastSeqIdx as usize)).offset;
        }
    }

    ZSTD_memcpy(
        (*(*cctx).blockState.nextCBlock).rep.as_mut_ptr() as *mut c_void,
        updatedRepcodes.rep.as_ptr() as *const c_void,
        core::mem::size_of::<Repcodes_t>(),
    );

    if (*inSeqs.add(idx as usize)).litLength != 0 {
        ZSTD_storeLastLiterals(
            &mut (*cctx).seqStore,
            ip,
            (*inSeqs.add(idx as usize)).litLength as usize,
        );
        ip = ip.add((*inSeqs.add(idx as usize)).litLength as usize);
        (*seqPos).posInSrc += (*inSeqs.add(idx as usize)).litLength as usize;
    }
    if ip != iend {
        return ERROR(ZSTD_error_externalSequences_invalid);
    }
    (*seqPos).idx = idx.wrapping_add(1);
    blockSize
}

/*
 * This function attempts to scan through @blockSize bytes in @src
 * represented by the sequences in @inSeqs,
 * storing any (partial) sequences.
 *
 * Occasionally, we may want to reduce the actual number of bytes consumed from @src
 * to avoid splitting a match, notably if it would produce a match smaller than MINMATCH.
 *
 * @returns the number of bytes consumed from @src, necessarily <= @blockSize.
 * Otherwise, it may return a ZSTD error if something went wrong.
 */
unsafe fn ZSTD_transferSequences_noDelim(
    cctx: *mut ZSTD_CCtx,
    seqPos: *mut ZSTD_SequencePosition,
    inSeqs: *const ZSTD_Sequence,
    inSeqsSize: usize,
    src: *const c_void,
    blockSize: usize,
    externalRepSearch: ZSTD_ParamSwitch_e,
) -> usize {
    let mut idx: U32 = (*seqPos).idx;
    let mut startPosInSequence: U32 = (*seqPos).posInSequence;
    let mut endPosInSequence: U32 = (*seqPos).posInSequence.wrapping_add(blockSize as U32);
    let dictSize: usize;
    let istart: *const BYTE = src as *const BYTE;
    let mut ip: *const BYTE = istart;
    let mut iend: *const BYTE = istart.add(blockSize); /* May be adjusted if we decide to process fewer than blockSize bytes */
    let mut updatedRepcodes: Repcodes_t = Repcodes_t::default();
    let mut bytesAdjustment: U32 = 0;
    let mut finalMatchSplit: U32 = 0;

    /* TODO(embg) support fast parsing mode in noBlockDelim mode */
    let _ = externalRepSearch;

    if !(*cctx).cdict.is_null() {
        dictSize = (*(*cctx).cdict).dictContentSize;
    } else if !(*cctx).prefixDict.dict.is_null() {
        dictSize = (*cctx).prefixDict.dictSize;
    } else {
        dictSize = 0;
    }
    ZSTD_memcpy(
        updatedRepcodes.rep.as_mut_ptr() as *mut c_void,
        (*(*cctx).blockState.prevCBlock).rep.as_ptr() as *const c_void,
        core::mem::size_of::<Repcodes_t>(),
    );
    while endPosInSequence != 0 && (idx as usize) < inSeqsSize && finalMatchSplit == 0 {
        let currSeq: ZSTD_Sequence = *inSeqs.add(idx as usize);
        let mut litLength: U32 = currSeq.litLength;
        let mut matchLength: U32 = currSeq.matchLength;
        let rawOffset: U32 = currSeq.offset;
        let offBase: U32;

        /* Modify the sequence depending on where endPosInSequence lies */
        if endPosInSequence >= currSeq.litLength.wrapping_add(currSeq.matchLength) {
            if startPosInSequence >= litLength {
                startPosInSequence = startPosInSequence.wrapping_sub(litLength);
                litLength = 0;
                matchLength = matchLength.wrapping_sub(startPosInSequence);
            } else {
                litLength = litLength.wrapping_sub(startPosInSequence);
            }
            /* Move to the next sequence */
            endPosInSequence = endPosInSequence
                .wrapping_sub(currSeq.litLength.wrapping_add(currSeq.matchLength));
            startPosInSequence = 0;
        } else {
            /* This is the final (partial) sequence we're adding from inSeqs, and endPosInSequence
            does not reach the end of the match. So, we have to split the sequence */
            if endPosInSequence > litLength {
                let mut firstHalfMatchLength: U32;
                litLength = if startPosInSequence >= litLength {
                    0
                } else {
                    litLength.wrapping_sub(startPosInSequence)
                };
                firstHalfMatchLength = endPosInSequence
                    .wrapping_sub(startPosInSequence)
                    .wrapping_sub(litLength);
                if matchLength as usize > blockSize
                    && firstHalfMatchLength >= (*cctx).appliedParams.cParams.minMatch
                {
                    /* Only ever split the match if it is larger than the block size */
                    let secondHalfMatchLength: U32 = currSeq
                        .matchLength
                        .wrapping_add(currSeq.litLength)
                        .wrapping_sub(endPosInSequence);
                    if secondHalfMatchLength < (*cctx).appliedParams.cParams.minMatch {
                        /* Move the endPosInSequence backward so that it creates match of minMatch length */
                        endPosInSequence = endPosInSequence.wrapping_sub(
                            (*cctx)
                                .appliedParams
                                .cParams
                                .minMatch
                                .wrapping_sub(secondHalfMatchLength),
                        );
                        bytesAdjustment = (*cctx)
                            .appliedParams
                            .cParams
                            .minMatch
                            .wrapping_sub(secondHalfMatchLength);
                        firstHalfMatchLength = firstHalfMatchLength.wrapping_sub(bytesAdjustment);
                    }
                    matchLength = firstHalfMatchLength;
                    /* Flag that we split the last match - after storing the sequence, exit the loop,
                    but keep the value of endPosInSequence */
                    finalMatchSplit = 1;
                } else {
                    /* Move the position in sequence backwards so that we don't split match, and break to store
                     * the last literals. We use the original currSeq.litLength as a marker for where endPosInSequence
                     * should go. We prefer to do this whenever it is not necessary to split the match, or if doing so
                     * would cause the first half of the match to be too small
                     */
                    bytesAdjustment = endPosInSequence.wrapping_sub(currSeq.litLength);
                    endPosInSequence = currSeq.litLength;
                    break;
                }
            } else {
                /* This sequence ends inside the literals, break to store the last literals */
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
            (*seqPos).posInSrc += litLength.wrapping_add(matchLength) as usize;
            let err_code = ZSTD_validateSequence(
                offBase,
                matchLength,
                (*cctx).appliedParams.cParams.minMatch,
                (*seqPos).posInSrc,
                (*cctx).appliedParams.cParams.windowLog,
                dictSize,
                ZSTD_hasExtSeqProd(&(*cctx).appliedParams),
            );
            if ERR_isError(err_code) != 0 {
                return err_code;
            }
        }
        if idx.wrapping_sub((*seqPos).idx) as usize >= (*cctx).seqStore.maxNbSeq {
            return ERROR(ZSTD_error_externalSequences_invalid);
        }
        ZSTD_storeSeq(
            &mut (*cctx).seqStore,
            litLength as usize,
            ip,
            iend,
            offBase,
            matchLength as usize,
        );
        ip = ip.add(matchLength.wrapping_add(litLength) as usize);
        if finalMatchSplit == 0 {
            idx = idx.wrapping_add(1); /* Next Sequence */
        }
    }
    (*seqPos).idx = idx;
    (*seqPos).posInSequence = endPosInSequence;
    ZSTD_memcpy(
        (*(*cctx).blockState.nextCBlock).rep.as_mut_ptr() as *mut c_void,
        updatedRepcodes.rep.as_ptr() as *const c_void,
        core::mem::size_of::<Repcodes_t>(),
    );

    iend = iend.wrapping_sub(bytesAdjustment as usize);
    if ip != iend {
        /* Store any last literals */
        let lastLLSize: U32 = iend.offset_from(ip) as U32;
        ZSTD_storeLastLiterals(&mut (*cctx).seqStore, ip, lastLLSize as usize);
        (*seqPos).posInSrc += lastLLSize as usize;
    }

    iend.offset_from(istart) as usize
}

/* @seqPos represents a position within @inSeqs,
 * it is read and updated by this function,
 * once the goal to produce a block of size @blockSize is reached.
 * @return: nb of bytes consumed from @src, necessarily <= @blockSize.
 */
type ZSTD_SequenceCopier_f = unsafe fn(
    *mut ZSTD_CCtx,
    *mut ZSTD_SequencePosition,
    *const ZSTD_Sequence,
    usize,
    *const c_void,
    usize,
    ZSTD_ParamSwitch_e,
) -> usize;

unsafe fn ZSTD_selectSequenceCopier(mode: ZSTD_SequenceFormat_e) -> ZSTD_SequenceCopier_f {
    if mode == ZSTD_sf_explicitBlockDelimiters {
        return ZSTD_transferSequences_wBlockDelim;
    }
    ZSTD_transferSequences_noDelim
}

/* Discover the size of next block by searching for the delimiter.
 * Note that a block delimiter **must** exist in this mode,
 * otherwise it's an input error.
 * The block size retrieved will be later compared to ensure it remains within bounds */
unsafe fn blockSize_explicitDelimiter(
    inSeqs: *const ZSTD_Sequence,
    inSeqsSize: usize,
    seqPos: ZSTD_SequencePosition,
) -> usize {
    let mut end: c_int = 0;
    let mut blockSize: usize = 0;
    let mut spos: usize = seqPos.idx as usize;
    while spos < inSeqsSize {
        end = ((*inSeqs.add(spos)).offset == 0) as c_int;
        blockSize +=
            ((*inSeqs.add(spos)).litLength + (*inSeqs.add(spos)).matchLength) as usize;
        if end != 0 {
            if (*inSeqs.add(spos)).matchLength != 0 {
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
    blockSize: usize,
    remaining: usize,
    inSeqs: *const ZSTD_Sequence,
    inSeqsSize: usize,
    seqPos: ZSTD_SequencePosition,
) -> usize {
    if mode == ZSTD_sf_noBlockDelimiters {
        /* Note: more a "target" block size */
        return MIN(remaining, blockSize);
    }
    {
        let explicitBlockSize: usize = blockSize_explicitDelimiter(inSeqs, inSeqsSize, seqPos);
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

/* Compress all provided sequences, block-by-block.
 *
 * Returns the cumulative size of all compressed blocks (including their headers),
 * otherwise a ZSTD error.
 */
unsafe fn ZSTD_compressSequences_internal(
    cctx: *mut ZSTD_CCtx,
    dst: *mut c_void,
    mut dstCapacity: usize,
    inSeqs: *const ZSTD_Sequence,
    inSeqsSize: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let mut cSize: usize = 0;
    let mut remaining: usize = srcSize;
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
        let cBlockHeader24: U32 = 1 /* last block */ + ((bt_raw as U32) << 1);
        if dstCapacity < 4 {
            return ERROR(ZSTD_error_dstSize_tooSmall);
        }
        MEM_writeLE32(op as *mut c_void, cBlockHeader24);
        op = op.add(ZSTD_blockHeaderSize);
        dstCapacity -= ZSTD_blockHeaderSize;
        cSize += ZSTD_blockHeaderSize;
    }

    while remaining != 0 {
        let mut compressedSeqsSize: usize;
        let cBlockSize: usize;
        let mut blockSize: usize = determine_blockSize(
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
        /* TODO: See 3090. We reduced MIN_CBLOCK_SIZE from 3 to 2 so to compensate we are adding
         * additional 1. We need to revisit and change this logic to be more consistent */
        if blockSize < MIN_CBLOCK_SIZE + ZSTD_blockHeaderSize + 1 + 1 {
            let cBlockSize2: usize = ZSTD_noCompressBlock(
                op as *mut c_void,
                dstCapacity,
                ip as *const c_void,
                blockSize,
                lastBlock,
            );
            if ERR_isError(cBlockSize2) != 0 {
                return cBlockSize2;
            }
            cSize += cBlockSize2;
            ip = ip.add(blockSize);
            op = op.add(cBlockSize2);
            remaining -= blockSize;
            dstCapacity -= cBlockSize2;
            continue;
        }

        if dstCapacity < ZSTD_blockHeaderSize {
            return ERROR(ZSTD_error_dstSize_tooSmall);
        }
        compressedSeqsSize = ZSTD_entropyCompressSeqStore(
            &(*cctx).seqStore,
            &(*(*cctx).blockState.prevCBlock).entropy,
            &mut (*(*cctx).blockState.nextCBlock).entropy,
            &(*cctx).appliedParams,
            op.add(ZSTD_blockHeaderSize) as *mut c_void, /* Leave space for block header */
            dstCapacity - ZSTD_blockHeaderSize,
            blockSize,
            (*cctx).tmpWorkspace,
            (*cctx).tmpWkspSize, /* statically allocated in resetCCtx */
            (*cctx).bmi2,
        );
        if ERR_isError(compressedSeqsSize) != 0 {
            return compressedSeqsSize;
        }

        if (*cctx).isFirstBlock == 0
            && ZSTD_maybeRLE(&(*cctx).seqStore) != 0
            && ZSTD_isRLE(ip, blockSize) != 0
        {
            /* Note: don't emit the first block as RLE even if it qualifies because
             * doing so will cause the decoder (cli <= v1.4.3 only) to throw an (invalid) error
             * "should consume all input error."
             */
            compressedSeqsSize = 1;
        }

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

            /* Write block header into beginning of block*/
            cBlockHeader = lastBlock
                .wrapping_add((bt_compressed as U32) << 1)
                .wrapping_add((compressedSeqsSize << 3) as U32);
            MEM_writeLE24(op as *mut c_void, cBlockHeader);
            cBlockSize = ZSTD_blockHeaderSize + compressedSeqsSize;
        }

        cSize += cBlockSize;

        if lastBlock != 0 {
            break;
        } else {
            ip = ip.add(blockSize);
            op = op.add(cBlockSize);
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
    mut dstCapacity: usize,
    inSeqs: *const ZSTD_Sequence,
    inSeqsSize: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let mut op: *mut BYTE = dst as *mut BYTE;
    let mut cSize: usize = 0;

    /* Transparent initialization stage, same as compressStream2() */
    {
        let err_code = ZSTD_CCtx_init_compressStream2(cctx, ZSTD_e_end, srcSize);
        if ERR_isError(err_code) != 0 {
            return err_code;
        }
    }

    /* Begin writing output, starting with frame header */
    {
        let frameHeaderSize: usize = ZSTD_writeFrameHeader(
            op as *mut c_void,
            dstCapacity,
            &(*cctx).appliedParams,
            srcSize as U64,
            (*cctx).dictID,
        );
        op = op.add(frameHeaderSize);
        dstCapacity -= frameHeaderSize;
        cSize += frameHeaderSize;
    }
    if (*cctx).appliedParams.fParams.checksumFlag != 0 && srcSize != 0 {
        ZSTD_XXH64_update(&mut (*cctx).xxhState, src, srcSize);
    }

    /* Now generate compressed blocks */
    {
        let cBlocksSize: usize = ZSTD_compressSequences_internal(
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
        MEM_writeLE32((dst as *mut c_char).add(cSize) as *mut c_void, checksum);
        cSize += 4;
    }

    cSize
}

/* no AVX2 in this build: scalar version only */
unsafe fn convertSequences_noRepcodes(
    dstSeqs: *mut SeqDef,
    inSeqs: *const ZSTD_Sequence,
    nbSequences: usize,
) -> usize {
    let mut longLen: usize = 0;
    let mut n: usize = 0;
    while n < nbSequences {
        (*dstSeqs.add(n)).offBase = OFFSET_TO_OFFBASE((*inSeqs.add(n)).offset);
        (*dstSeqs.add(n)).litLength = (*inSeqs.add(n)).litLength as U16;
        (*dstSeqs.add(n)).mlBase =
            ((*inSeqs.add(n)).matchLength).wrapping_sub(MINMATCH as U32) as U16;
        /* check for long length > 65535 */
        if (*inSeqs.add(n)).matchLength > 65535 + MINMATCH as U32 {
            longLen = n + 1;
        }
        if (*inSeqs.add(n)).litLength > 65535 {
            longLen = n + nbSequences + 1;
        }
        n += 1;
    }
    longLen
}

/*
 * Precondition: Sequences must end on an explicit Block Delimiter
 * @return: 0 on success, or an error code.
 * Note: Sequence validation functionality has been disabled (removed).
 * This is helpful to generate a lean main pipeline, improving performance.
 * It may be re-inserted later.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_convertBlockSequences(
    cctx: *mut ZSTD_CCtx,
    inSeqs: *const ZSTD_Sequence,
    nbSequences: usize,
    repcodeResolution: c_int,
) -> usize {
    let mut updatedRepcodes: Repcodes_t = Repcodes_t::default();
    let mut seqNb: usize = 0;

    if nbSequences >= (*cctx).seqStore.maxNbSeq {
        return ERROR(ZSTD_error_externalSequences_invalid);
    }

    ZSTD_memcpy(
        updatedRepcodes.rep.as_mut_ptr() as *mut c_void,
        (*(*cctx).blockState.prevCBlock).rep.as_ptr() as *const c_void,
        core::mem::size_of::<Repcodes_t>(),
    );

    /* Convert Sequences from public format to internal format */
    if repcodeResolution == 0 {
        let longl: usize = convertSequences_noRepcodes(
            (*cctx).seqStore.sequencesStart,
            inSeqs,
            nbSequences - 1,
        );
        (*cctx).seqStore.sequences = (*cctx).seqStore.sequencesStart.add(nbSequences - 1);
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
            let litLength: U32 = (*inSeqs.add(seqNb)).litLength;
            let matchLength: U32 = (*inSeqs.add(seqNb)).matchLength;
            let ll0: U32 = (litLength == 0) as U32;
            let offBase: U32 = ZSTD_finalizeOffBase(
                (*inSeqs.add(seqNb)).offset,
                updatedRepcodes.rep.as_ptr(),
                ll0,
            );

            ZSTD_storeSeqOnly(
                &mut (*cctx).seqStore,
                litLength as usize,
                offBase,
                matchLength as usize,
            );
            ZSTD_updateRep(updatedRepcodes.rep.as_mut_ptr(), offBase, ll0);
            seqNb += 1;
        }
    }

    /* If we skipped repcode search while parsing, we need to update repcodes now */
    if repcodeResolution == 0 && nbSequences > 1 {
        let rep: *mut U32 = updatedRepcodes.rep.as_mut_ptr();

        if nbSequences >= 4 {
            let lastSeqIdx: U32 = (nbSequences as U32).wrapping_sub(2); /* index of last full sequence */
            *rep.add(2) = (*inSeqs.add(lastSeqIdx.wrapping_sub(2) as usize)).offset;
            *rep.add(1) = (*inSeqs.add(lastSeqIdx.wrapping_sub(1) as usize)).offset;
            *rep.add(0) = (*inSeqs.add(lastSeqIdx as usize)).offset;
        } else if nbSequences == 3 {
            *rep.add(2) = *rep.add(0);
            *rep.add(1) = (*inSeqs.add(0)).offset;
            *rep.add(0) = (*inSeqs.add(1)).offset;
        } else {
            *rep.add(2) = *rep.add(1);
            *rep.add(1) = *rep.add(0);
            *rep.add(0) = (*inSeqs.add(0)).offset;
        }
    }

    ZSTD_memcpy(
        (*(*cctx).blockState.nextCBlock).rep.as_mut_ptr() as *mut c_void,
        updatedRepcodes.rep.as_ptr() as *const c_void,
        core::mem::size_of::<Repcodes_t>(),
    );

    0
}

/* no ZSTD_ARCH_X86_AVX2 in this build: scalar version only */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_get1BlockSummary(
    seqs: *const ZSTD_Sequence,
    nbSeqs: usize,
) -> BlockSummary {
    let mut totalMatchSize: usize = 0;
    let mut litSize: usize = 0;
    let mut n: usize = 0;
    while n < nbSeqs {
        totalMatchSize += (*seqs.add(n)).matchLength as usize;
        litSize += (*seqs.add(n)).litLength as usize;
        if (*seqs.add(n)).matchLength == 0 {
            break;
        }
        n += 1;
    }
    if n == nbSeqs {
        let mut bs: BlockSummary = BlockSummary::default();
        bs.nbSequences = ERROR(ZSTD_error_externalSequences_invalid);
        return bs;
    }
    {
        let mut bs: BlockSummary = BlockSummary::default();
        bs.nbSequences = n + 1;
        bs.blockSize = litSize + totalMatchSize;
        bs.litSize = litSize;
        bs
    }
}

unsafe fn ZSTD_compressSequencesAndLiterals_internal(
    cctx: *mut ZSTD_CCtx,
    dst: *mut c_void,
    mut dstCapacity: usize,
    mut inSeqs: *const ZSTD_Sequence,
    mut nbSequences: usize,
    mut literals: *const c_void,
    mut litSize: usize,
    srcSize: usize,
) -> usize {
    let mut remaining: usize = srcSize;
    let mut cSize: usize = 0;
    let mut op: *mut BYTE = dst as *mut BYTE;
    let repcodeResolution: c_int =
        ((*cctx).appliedParams.searchForExternalRepcodes == ZSTD_ps_enable) as c_int;

    if nbSequences == 0 {
        return ERROR(ZSTD_error_externalSequences_invalid);
    }

    /* Special case: empty frame */
    if nbSequences == 1 && (*inSeqs.add(0)).litLength == 0 {
        let cBlockHeader24: U32 = 1 /* last block */ + ((bt_raw as U32) << 1);
        if dstCapacity < 3 {
            return ERROR(ZSTD_error_dstSize_tooSmall);
        }
        MEM_writeLE24(op as *mut c_void, cBlockHeader24);
        op = op.add(ZSTD_blockHeaderSize);
        dstCapacity -= ZSTD_blockHeaderSize;
        cSize += ZSTD_blockHeaderSize;
    }

    while nbSequences != 0 {
        let mut compressedSeqsSize: usize;
        let cBlockSize: usize;
        let conversionStatus: usize;
        let block: BlockSummary = ZSTD_get1BlockSummary(inSeqs, nbSequences);
        let lastBlock: U32 = (block.nbSequences == nbSequences) as U32;
        if ERR_isError(block.nbSequences) != 0 {
            return block.nbSequences;
        }
        if block.litSize > litSize {
            return ERROR(ZSTD_error_externalSequences_invalid);
        }
        ZSTD_resetSeqStore(&mut (*cctx).seqStore);

        conversionStatus =
            ZSTD_convertBlockSequences(cctx, inSeqs, block.nbSequences, repcodeResolution);
        if ERR_isError(conversionStatus) != 0 {
            return conversionStatus;
        }
        inSeqs = inSeqs.add(block.nbSequences);
        nbSequences -= block.nbSequences;
        remaining -= block.blockSize;

        /* Note: when blockSize is very small, other variant send it uncompressed.
         * Here, we still send the sequences, because we don't have the original source to send it uncompressed.
         * One could imagine in theory reproducing the source from the sequences,
         * but that's complex and costly memory intensive, and goes against the objectives of this variant. */

        if dstCapacity < ZSTD_blockHeaderSize {
            return ERROR(ZSTD_error_dstSize_tooSmall);
        }

        compressedSeqsSize = ZSTD_entropyCompressSeqStore_internal(
            op.add(ZSTD_blockHeaderSize) as *mut c_void, /* Leave space for block header */
            dstCapacity - ZSTD_blockHeaderSize,
            literals,
            block.litSize,
            &(*cctx).seqStore,
            &(*(*cctx).blockState.prevCBlock).entropy,
            &mut (*(*cctx).blockState.nextCBlock).entropy,
            &(*cctx).appliedParams,
            (*cctx).tmpWorkspace,
            (*cctx).tmpWkspSize, /* statically allocated in resetCCtx */
            (*cctx).bmi2,
        );
        if ERR_isError(compressedSeqsSize) != 0 {
            return compressedSeqsSize;
        }
        /* note: the spec forbids for any compressed block to be larger than maximum block size */
        if compressedSeqsSize > (*cctx).blockSizeMax {
            compressedSeqsSize = 0;
        }
        litSize -= block.litSize;
        literals = (literals as *const c_char).add(block.litSize) as *const c_void;

        /* Note: difficult to check source for RLE block when only Literals are provided,
         * but it could be considered from analyzing the sequence directly */

        if compressedSeqsSize == 0 {
            /* Sending uncompressed blocks is out of reach, because the source is not provided.
             * In theory, one could use the sequences to regenerate the source, like a decompressor,
             * but it's complex, and memory hungry, killing the purpose of this variant.
             * Current outcome: generate an error code.
             */
            return ERROR(ZSTD_error_cannotProduce_uncompressedBlock);
        } else {
            let cBlockHeader: U32;
            /* Error checking and repcodes update */
            ZSTD_blockState_confirmRepcodesAndEntropyTables(&mut (*cctx).blockState);
            if (*(*cctx).blockState.prevCBlock).entropy.fse.offcode_repeatMode == FSE_repeat_valid {
                (*(*cctx).blockState.prevCBlock).entropy.fse.offcode_repeatMode = FSE_repeat_check;
            }

            /* Write block header into beginning of block*/
            cBlockHeader = lastBlock
                .wrapping_add((bt_compressed as U32) << 1)
                .wrapping_add((compressedSeqsSize << 3) as U32);
            MEM_writeLE24(op as *mut c_void, cBlockHeader);
            cBlockSize = ZSTD_blockHeaderSize + compressedSeqsSize;
        }

        cSize += cBlockSize;
        op = op.add(cBlockSize);
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
    mut dstCapacity: usize,
    inSeqs: *const ZSTD_Sequence,
    inSeqsSize: usize,
    literals: *const c_void,
    litSize: usize,
    litCapacity: usize,
    decompressedSize: usize,
) -> usize {
    let mut op: *mut BYTE = dst as *mut BYTE;
    let mut cSize: usize = 0;

    /* Transparent initialization stage, same as compressStream2() */
    if litCapacity < litSize {
        return ERROR(ZSTD_error_workSpace_tooSmall);
    }
    {
        let err_code = ZSTD_CCtx_init_compressStream2(cctx, ZSTD_e_end, decompressedSize);
        if ERR_isError(err_code) != 0 {
            return err_code;
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
        let frameHeaderSize: usize = ZSTD_writeFrameHeader(
            op as *mut c_void,
            dstCapacity,
            &(*cctx).appliedParams,
            decompressedSize as U64,
            (*cctx).dictID,
        );
        op = op.add(frameHeaderSize);
        dstCapacity -= frameHeaderSize;
        cSize += frameHeaderSize;
    }

    /* Now generate compressed blocks */
    {
        let cBlocksSize: usize = ZSTD_compressSequencesAndLiterals_internal(
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
        src: core::ptr::null(),
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

/* ZSTD_flushStream() :
 * @return : amount of data remaining to flush */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_flushStream(
    zcs: *mut ZSTD_CStream,
    output: *mut ZSTD_outBuffer,
) -> usize {
    let mut input: ZSTD_inBuffer = inBuffer_forEndFlush(zcs);
    input.size = input.pos; /* do not ingest more input during flush */
    ZSTD_compressStream2(zcs, output, &mut input, ZSTD_e_flush)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_endStream(
    zcs: *mut ZSTD_CStream,
    output: *mut ZSTD_outBuffer,
) -> usize {
    let mut input: ZSTD_inBuffer = inBuffer_forEndFlush(zcs);
    let remainingToFlush: usize = ZSTD_compressStream2(zcs, output, &mut input, ZSTD_e_end);
    if ERR_isError(remainingToFlush) != 0 {
        return remainingToFlush;
    }
    if (*zcs).appliedParams.nbWorkers > 0 {
        return remainingToFlush; /* minimal estimation */
    }
    /* single thread mode : attempt to calculate remaining to flush more precisely */
    {
        let lastBlockSize: usize = if (*zcs).frameEnded != 0 {
            0
        } else {
            ZSTD_BLOCKHEADERSIZE
        };
        let checksumSize: usize = if (*zcs).frameEnded != 0 {
            0
        } else {
            ((*zcs).appliedParams.fParams.checksumFlag * 4) as usize
        };
        let toFlush: usize = remainingToFlush + lastBlockSize + checksumSize;
        toFlush
    }
}

/*-=====  Pre-defined compression levels  =====-*/
/* `clevels.h` lives in `crate::compress::clevels`, already imported. */

#[unsafe(no_mangle)]
pub extern "C" fn ZSTD_maxCLevel() -> c_int {
    ZSTD_MAX_CLEVEL
}

#[unsafe(no_mangle)]
pub extern "C" fn ZSTD_minCLevel() -> c_int {
    -(ZSTD_TARGETLENGTH_MAX as c_int)
}

#[unsafe(no_mangle)]
pub extern "C" fn ZSTD_defaultCLevel() -> c_int {
    ZSTD_CLEVEL_DEFAULT
}

unsafe fn ZSTD_dedicatedDictSearch_getCParams(
    compressionLevel: c_int,
    dictSize: usize,
) -> ZSTD_compressionParameters {
    let mut cParams: ZSTD_compressionParameters =
        ZSTD_getCParams_internal(compressionLevel, 0, dictSize, ZSTD_cpm_createCDict);
    match cParams.strategy {
        ZSTD_fast | ZSTD_dfast => {}
        ZSTD_greedy | ZSTD_lazy | ZSTD_lazy2 => {
            cParams.hashLog += ZSTD_LAZY_DDSS_BUCKET_LOG;
        }
        ZSTD_btlazy2 | ZSTD_btopt | ZSTD_btultra | ZSTD_btultra2 => {}
        _ => {}
    }
    cParams
}

unsafe fn ZSTD_dedicatedDictSearch_isSupported(
    cParams: *const ZSTD_compressionParameters,
) -> c_int {
    ((*cParams).strategy >= ZSTD_greedy
        && (*cParams).strategy <= ZSTD_lazy2
        && (*cParams).hashLog > (*cParams).chainLog
        && (*cParams).chainLog <= 24) as c_int
}

/*
 * Reverses the adjustment applied to cparams when enabling dedicated dict
 * search. This is used to recover the params set to be used in the working
 * context. (Otherwise, those tables would also grow.)
 */
unsafe fn ZSTD_dedicatedDictSearch_revertCParams(cParams: *mut ZSTD_compressionParameters) {
    match (*cParams).strategy {
        ZSTD_fast | ZSTD_dfast => {}
        ZSTD_greedy | ZSTD_lazy | ZSTD_lazy2 => {
            (*cParams).hashLog -= ZSTD_LAZY_DDSS_BUCKET_LOG;
            if (*cParams).hashLog < ZSTD_HASHLOG_MIN as c_uint {
                (*cParams).hashLog = ZSTD_HASHLOG_MIN as c_uint;
            }
        }
        ZSTD_btlazy2 | ZSTD_btopt | ZSTD_btultra | ZSTD_btultra2 => {}
        _ => {}
    }
}

unsafe fn ZSTD_getCParamRowSize(
    srcSizeHint: U64,
    mut dictSize: usize,
    mode: ZSTD_CParamMode_e,
) -> U64 {
    match mode {
        ZSTD_cpm_unknown | ZSTD_cpm_noAttachDict | ZSTD_cpm_createCDict => {}
        ZSTD_cpm_attachDict => {
            dictSize = 0;
        }
        _ => {}
    }
    {
        let unknown: c_int = (srcSizeHint == ZSTD_CONTENTSIZE_UNKNOWN) as c_int;
        let addedSize: usize = if unknown != 0 && dictSize > 0 { 500 } else { 0 };
        if unknown != 0 && dictSize == 0 {
            ZSTD_CONTENTSIZE_UNKNOWN
        } else {
            srcSizeHint
                .wrapping_add(dictSize as U64)
                .wrapping_add(addedSize as U64)
        }
    }
}

/* ZSTD_getCParams_internal() :
 * @return ZSTD_compressionParameters structure for a selected compression level, srcSize and dictSize.
 *  Note: srcSizeHint 0 means 0, use ZSTD_CONTENTSIZE_UNKNOWN for unknown.
 *        Use dictSize == 0 for unknown or unused.
 *  Note: `mode` controls how we treat the `dictSize`. See docs for `ZSTD_CParamMode_e`. */
unsafe fn ZSTD_getCParams_internal(
    compressionLevel: c_int,
    srcSizeHint: u64,
    dictSize: usize,
    mode: ZSTD_CParamMode_e,
) -> ZSTD_compressionParameters {
    let rSize: U64 = ZSTD_getCParamRowSize(srcSizeHint, dictSize, mode);
    let tableID: U32 = (rSize <= 256 * 1024) as U32
        + (rSize <= 128 * 1024) as U32
        + (rSize <= 16 * 1024) as U32;
    let row: c_int;

    /* row */
    if compressionLevel == 0 {
        row = ZSTD_CLEVEL_DEFAULT; /* 0 == default */
    } else if compressionLevel < 0 {
        row = 0; /* entry 0 is baseline for fast mode */
    } else if compressionLevel > ZSTD_MAX_CLEVEL {
        row = ZSTD_MAX_CLEVEL;
    } else {
        row = compressionLevel;
    }

    {
        let mut cp: ZSTD_compressionParameters =
            ZSTD_defaultCParameters[tableID as usize][row as usize];
        /* acceleration factor */
        if compressionLevel < 0 {
            let clampedCompressionLevel: c_int = MAX(ZSTD_minCLevel(), compressionLevel);
            cp.targetLength = clampedCompressionLevel.wrapping_neg() as c_uint;
        }
        /* refine parameters based on srcSize & dictSize */
        ZSTD_adjustCParams_internal(cp, srcSizeHint, dictSize, mode, ZSTD_ps_auto)
    }
}

/* ZSTD_getCParams() :
 * @return ZSTD_compressionParameters structure for a selected compression level, srcSize and dictSize.
 *  Size values are optional, provide 0 if not known or unused */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_getCParams(
    compressionLevel: c_int,
    mut srcSizeHint: u64,
    dictSize: usize,
) -> ZSTD_compressionParameters {
    if srcSizeHint == 0 {
        srcSizeHint = ZSTD_CONTENTSIZE_UNKNOWN;
    }
    ZSTD_getCParams_internal(compressionLevel, srcSizeHint, dictSize, ZSTD_cpm_unknown)
}

/* ZSTD_getParams() :
 *  same idea as ZSTD_getCParams()
 * @return a `ZSTD_parameters` structure (instead of `ZSTD_compressionParameters`).
 *  Fields of `ZSTD_frameParameters` are set to default values */
unsafe fn ZSTD_getParams_internal(
    compressionLevel: c_int,
    srcSizeHint: u64,
    dictSize: usize,
    mode: ZSTD_CParamMode_e,
) -> ZSTD_parameters {
    let mut params: ZSTD_parameters;
    let cParams: ZSTD_compressionParameters =
        ZSTD_getCParams_internal(compressionLevel, srcSizeHint, dictSize, mode);
    params = core::mem::zeroed();
    ZSTD_memset(
        core::ptr::addr_of_mut!(params) as *mut c_void,
        0,
        core::mem::size_of::<ZSTD_parameters>(),
    );
    params.cParams = cParams;
    params.fParams.contentSizeFlag = 1;
    params
}

/* ZSTD_getParams() :
 *  same idea as ZSTD_getCParams()
 * @return a `ZSTD_parameters` structure (instead of `ZSTD_compressionParameters`).
 *  Fields of `ZSTD_frameParameters` are set to default values */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_getParams(
    compressionLevel: c_int,
    mut srcSizeHint: u64,
    dictSize: usize,
) -> ZSTD_parameters {
    if srcSizeHint == 0 {
        srcSizeHint = ZSTD_CONTENTSIZE_UNKNOWN;
    }
    ZSTD_getParams_internal(compressionLevel, srcSizeHint, dictSize, ZSTD_cpm_unknown)
}

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
        (*params).extSeqProdState = core::ptr::null_mut();
    }
}
