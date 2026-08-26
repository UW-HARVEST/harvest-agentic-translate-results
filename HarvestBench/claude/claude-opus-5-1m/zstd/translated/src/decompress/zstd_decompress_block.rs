//! Translation of `decompress/zstd_decompress_block.c`
//!
//! this module takes care of decompressing _compressed_ block

use core::ffi::{c_char, c_int, c_uint, c_void};

use crate::common::bits::ZSTD_highbit32;
use crate::common::bitstream::*;
use crate::common::error_private::*;
use crate::common::fse::*;
use crate::common::huf::*;
use crate::common::mem::*;
use crate::common::zstd_internal::*;
use crate::decompress::zstd_decompress_internal::*;
use crate::libc::{ZSTD_memcpy, ZSTD_memmove, ZSTD_memset};

/* ===   Cross-module functions (exported symbols defined elsewhere)   === */
extern "C" {
    fn HUF_decompress1X_usingDTable(
        dst: *mut c_void,
        maxDstSize: usize,
        cSrc: *const c_void,
        cSrcSize: usize,
        DTable: *const HUF_DTable,
        flags: c_int,
    ) -> usize;
    fn HUF_decompress4X_usingDTable(
        dst: *mut c_void,
        maxDstSize: usize,
        cSrc: *const c_void,
        cSrcSize: usize,
        DTable: *const HUF_DTable,
        flags: c_int,
    ) -> usize;
    fn HUF_decompress1X1_DCtx_wksp(
        dctx: *mut HUF_DTable,
        dst: *mut c_void,
        dstSize: usize,
        cSrc: *const c_void,
        cSrcSize: usize,
        workSpace: *mut c_void,
        wkspSize: usize,
        flags: c_int,
    ) -> usize;
    fn HUF_decompress4X_hufOnly_wksp(
        dctx: *mut HUF_DTable,
        dst: *mut c_void,
        dstSize: usize,
        cSrc: *const c_void,
        cSrcSize: usize,
        workSpace: *mut c_void,
        wkspSize: usize,
        flags: c_int,
    ) -> usize;
    fn FSE_readNCount(
        normalizedCounter: *mut i16,
        maxSVPtr: *mut c_uint,
        tableLogPtr: *mut c_uint,
        headerBuffer: *const c_void,
        hbSize: usize,
    ) -> usize;
}

/* Streaming state is used to inform allocation of the literal buffer */
pub type streaming_operation = c_int;
pub const not_streaming: streaming_operation = 0;
pub const is_streaming: streaming_operation = 1;

/* `LONG_OFFSETS_MAX_EXTRA_BITS_32` */
const LONG_OFFSETS_MAX_EXTRA_BITS_32: u32 = if (ZSTD_WINDOWLOG_MAX_32 as u32) > STREAM_ACCUMULATOR_MIN_32
{
    ZSTD_WINDOWLOG_MAX_32 as u32 - STREAM_ACCUMULATOR_MIN_32
} else {
    0
};

/* ZSTD_longOffset_e */
type ZSTD_longOffset_e = c_int;
const ZSTD_lo_isRegularOffset: ZSTD_longOffset_e = 0;
const ZSTD_lo_isLongOffset: ZSTD_longOffset_e = 1;

const CACHELINE_SIZE: usize = 64;

/*-*******************************************************
*  Memory operations
**********************************************************/
#[inline(always)]
unsafe fn ZSTD_copy4(dst: *mut c_void, src: *const c_void) {
    ZSTD_memcpy(dst, src, 4);
}

/// `ZSTD_maybeNullPtrAdd()` : `add > 0 ? ptr + add : ptr`
#[inline(always)]
unsafe fn ZSTD_maybeNullPtrAdd(ptr: *mut BYTE, add: isize) -> *mut BYTE {
    if add > 0 {
        ptr.wrapping_offset(add)
    } else {
        ptr
    }
}

/// `ZSTD_wrappedPtrAdd()`
#[inline(always)]
unsafe fn ZSTD_wrappedPtrAdd(ptr: *const BYTE, add: isize) -> *const BYTE {
    ptr.wrapping_offset(add)
}

/// `ZSTD_wrappedPtrSub()`
#[inline(always)]
unsafe fn ZSTD_wrappedPtrSub(ptr: *const BYTE, sub: isize) -> *const BYTE {
    ptr.wrapping_offset(-sub)
}

/*-*************************************************************
 *   Block decoding
 ***************************************************************/

unsafe fn ZSTD_blockSizeMax(dctx: *const ZSTD_DCtx) -> usize {
    let blockSizeMax: usize = if (*dctx).isFrameDecompression != 0 {
        (*dctx).fParams.blockSizeMax as usize
    } else {
        ZSTD_BLOCKSIZE_MAX
    };
    blockSizeMax
}

/// `ZSTD_getcBlockSize()` :
///  Provides the size of compressed block from block header `src`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_getcBlockSize(
    src: *const c_void,
    srcSize: usize,
    bpPtr: *mut blockProperties_t,
) -> usize {
    if srcSize < ZSTD_blockHeaderSize {
        return ERROR(ZSTD_error_srcSize_wrong);
    }

    {
        let cBlockHeader: U32 = MEM_readLE24(src);
        let cSize: U32 = cBlockHeader >> 3;
        (*bpPtr).lastBlock = cBlockHeader & 1;
        (*bpPtr).blockType = ((cBlockHeader >> 1) & 3) as blockType_e;
        (*bpPtr).origSize = cSize; /* only useful for RLE */
        if (*bpPtr).blockType == bt_rle {
            return 1;
        }
        if (*bpPtr).blockType == bt_reserved {
            return ERROR(ZSTD_error_corruption_detected);
        }
        cSize as usize
    }
}

/* Allocate buffer for literals, either overlapping current dst, or split between dst and
 * litExtraBuffer, or stored entirely within litExtraBuffer */
unsafe fn ZSTD_allocateLiteralsBuffer(
    dctx: *mut ZSTD_DCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    litSize: usize,
    streaming: streaming_operation,
    expectedWriteSize: usize,
    splitImmediately: c_uint,
) {
    let blockSizeMax: usize = ZSTD_blockSizeMax(dctx);
    if streaming == not_streaming
        && dstCapacity
            > blockSizeMax + WILDCOPY_OVERLENGTH as usize + litSize + WILDCOPY_OVERLENGTH as usize
    {
        /* If we aren't streaming, we can just put the literals after the output
         * of the current block. We don't need to worry about overwriting the
         * extDict of our window, because it doesn't exist.
         * So if we have space after the end of the block, just put it there.
         */
        (*dctx).litBuffer = (dst as *mut BYTE).add(blockSizeMax + WILDCOPY_OVERLENGTH as usize);
        (*dctx).litBufferEnd = (*dctx).litBuffer.add(litSize);
        (*dctx).litBufferLocation = ZSTD_in_dst;
    } else if litSize <= ZSTD_LITBUFFEREXTRASIZE {
        /* Literals fit entirely within the extra buffer, put them there to avoid
         * having to split the literals.
         */
        (*dctx).litBuffer = (*dctx).litExtraBuffer.as_mut_ptr();
        (*dctx).litBufferEnd = (*dctx).litBuffer.add(litSize);
        (*dctx).litBufferLocation = ZSTD_not_in_dst;
    } else {
        /* Literals must be split between the output block and the extra lit
         * buffer. We fill the extra lit buffer with the tail of the literals,
         * and put the rest of the literals at the end of the block, with
         * WILDCOPY_OVERLENGTH of buffer room to allow for overreads.
         * This MUST not write more than our maxBlockSize beyond dst, because in
         * streaming mode, that could overwrite part of our extDict window.
         */
        if splitImmediately != 0 {
            /* won't fit in litExtraBuffer, so it will be split between end of dst and extra buffer */
            (*dctx).litBuffer = (dst as *mut BYTE).wrapping_offset(
                expectedWriteSize as isize - litSize as isize + ZSTD_LITBUFFEREXTRASIZE as isize
                    - WILDCOPY_OVERLENGTH,
            );
            (*dctx).litBufferEnd = (*dctx)
                .litBuffer
                .wrapping_offset(litSize as isize - ZSTD_LITBUFFEREXTRASIZE as isize);
        } else {
            /* initially this will be stored entirely in dst during huffman decoding, it will
             * partially be shifted to litExtraBuffer after */
            (*dctx).litBuffer = (dst as *mut BYTE)
                .wrapping_offset(expectedWriteSize as isize - litSize as isize);
            (*dctx).litBufferEnd = (dst as *mut BYTE).wrapping_add(expectedWriteSize);
        }
        (*dctx).litBufferLocation = ZSTD_split;
    }
}

/// `ZSTD_decodeLiteralsBlock()` :
/// Where it is possible to do so without being stomped by the output during decompression, the
/// literals block will be stored in the dstBuffer.
///
/// @return : nb of bytes read from src (< srcSize )
unsafe fn ZSTD_decodeLiteralsBlock(
    dctx: *mut ZSTD_DCtx,
    src: *const c_void,
    srcSize: usize,
    dst: *mut c_void,
    dstCapacity: usize,
    streaming: streaming_operation,
) -> usize {
    if srcSize < MIN_CBLOCK_SIZE {
        return ERROR(ZSTD_error_corruption_detected);
    }

    {
        let istart = src as *const BYTE;
        let litEncType: SymbolEncodingType_e = (*istart.add(0) & 3) as SymbolEncodingType_e;
        let blockSizeMax: usize = ZSTD_blockSizeMax(dctx);

        match litEncType {
            /* set_repeat falls through into set_compressed */
            set_repeat | set_compressed => {
                if litEncType == set_repeat {
                    if (*dctx).litEntropy == 0 {
                        return ERROR(ZSTD_error_dictionary_corrupted);
                    }
                }
                if srcSize < 5 {
                    return ERROR(ZSTD_error_corruption_detected);
                }
                {
                    let lhSize: usize;
                    let litSize: usize;
                    let litCSize: usize;
                    let mut singleStream: U32 = 0;
                    let lhlCode: U32 = ((*istart.add(0) >> 2) & 3) as U32;
                    let lhc: U32 = MEM_readLE32(istart as *const c_void);
                    let hufSuccess: usize;
                    let expectedWriteSize: usize = MIN(blockSizeMax, dstCapacity);
                    let flags: c_int = 0
                        | (if ZSTD_DCtx_get_bmi2(dctx) != 0 {
                            HUF_flags_bmi2
                        } else {
                            0
                        })
                        | (if (*dctx).disableHufAsm != 0 {
                            HUF_flags_disableAsm
                        } else {
                            0
                        });
                    match lhlCode {
                        2 => {
                            /* 2 - 2 - 14 - 14 */
                            lhSize = 4;
                            litSize = ((lhc >> 4) & 0x3FFF) as usize;
                            litCSize = (lhc >> 18) as usize;
                        }
                        3 => {
                            /* 2 - 2 - 18 - 18 */
                            lhSize = 5;
                            litSize = ((lhc >> 4) & 0x3FFFF) as usize;
                            litCSize = (lhc >> 22) as usize + ((*istart.add(4) as usize) << 10);
                        }
                        _ => {
                            /* 0, 1 and default : 2 - 2 - 10 - 10 */
                            singleStream = (lhlCode == 0) as U32;
                            lhSize = 3;
                            litSize = ((lhc >> 4) & 0x3FF) as usize;
                            litCSize = ((lhc >> 14) & 0x3FF) as usize;
                        }
                    }
                    if litSize > 0 && dst.is_null() {
                        return ERROR(ZSTD_error_dstSize_tooSmall);
                    }
                    if litSize > blockSizeMax {
                        return ERROR(ZSTD_error_corruption_detected);
                    }
                    if singleStream == 0 {
                        if litSize < MIN_LITERALS_FOR_4_STREAMS {
                            return ERROR(ZSTD_error_literals_headerWrong);
                        }
                    }
                    if litCSize + lhSize > srcSize {
                        return ERROR(ZSTD_error_corruption_detected);
                    }
                    if expectedWriteSize < litSize {
                        return ERROR(ZSTD_error_dstSize_tooSmall);
                    }
                    ZSTD_allocateLiteralsBuffer(
                        dctx,
                        dst,
                        dstCapacity,
                        litSize,
                        streaming,
                        expectedWriteSize,
                        0,
                    );

                    /* prefetch huffman table if cold : PREFETCH_AREA is a no-op in this port */

                    if litEncType == set_repeat {
                        if singleStream != 0 {
                            hufSuccess = HUF_decompress1X_usingDTable(
                                (*dctx).litBuffer as *mut c_void,
                                litSize,
                                istart.add(lhSize) as *const c_void,
                                litCSize,
                                (*dctx).HUFptr,
                                flags,
                            );
                        } else {
                            hufSuccess = HUF_decompress4X_usingDTable(
                                (*dctx).litBuffer as *mut c_void,
                                litSize,
                                istart.add(lhSize) as *const c_void,
                                litCSize,
                                (*dctx).HUFptr,
                                flags,
                            );
                        }
                    } else {
                        if singleStream != 0 {
                            hufSuccess = HUF_decompress1X1_DCtx_wksp(
                                (*dctx).entropy.hufTable.as_mut_ptr(),
                                (*dctx).litBuffer as *mut c_void,
                                litSize,
                                istart.add(lhSize) as *const c_void,
                                litCSize,
                                (*dctx).workspace.as_mut_ptr() as *mut c_void,
                                core::mem::size_of_val(&(*dctx).workspace),
                                flags,
                            );
                        } else {
                            hufSuccess = HUF_decompress4X_hufOnly_wksp(
                                (*dctx).entropy.hufTable.as_mut_ptr(),
                                (*dctx).litBuffer as *mut c_void,
                                litSize,
                                istart.add(lhSize) as *const c_void,
                                litCSize,
                                (*dctx).workspace.as_mut_ptr() as *mut c_void,
                                core::mem::size_of_val(&(*dctx).workspace),
                                flags,
                            );
                        }
                    }
                    if (*dctx).litBufferLocation == ZSTD_split {
                        ZSTD_memcpy(
                            (*dctx).litExtraBuffer.as_mut_ptr() as *mut c_void,
                            (*dctx).litBufferEnd.wrapping_sub(ZSTD_LITBUFFEREXTRASIZE)
                                as *const c_void,
                            ZSTD_LITBUFFEREXTRASIZE,
                        );
                        ZSTD_memmove(
                            (*dctx).litBuffer.wrapping_offset(
                                ZSTD_LITBUFFEREXTRASIZE as isize - WILDCOPY_OVERLENGTH,
                            ) as *mut c_void,
                            (*dctx).litBuffer as *const c_void,
                            litSize - ZSTD_LITBUFFEREXTRASIZE,
                        );
                        (*dctx).litBuffer = (*dctx)
                            .litBuffer
                            .wrapping_offset(ZSTD_LITBUFFEREXTRASIZE as isize - WILDCOPY_OVERLENGTH);
                        (*dctx).litBufferEnd =
                            (*dctx).litBufferEnd.wrapping_offset(-WILDCOPY_OVERLENGTH);
                    }

                    if ERR_isError(hufSuccess) != 0 {
                        return ERROR(ZSTD_error_corruption_detected);
                    }

                    (*dctx).litPtr = (*dctx).litBuffer;
                    (*dctx).litSize = litSize;
                    (*dctx).litEntropy = 1;
                    if litEncType == set_compressed {
                        (*dctx).HUFptr = (*dctx).entropy.hufTable.as_ptr();
                    }
                    return litCSize + lhSize;
                }
            }

            set_basic => {
                let litSize: usize;
                let lhSize: usize;
                let lhlCode: U32 = ((*istart.add(0) >> 2) & 3) as U32;
                let expectedWriteSize: usize = MIN(blockSizeMax, dstCapacity);
                match lhlCode {
                    1 => {
                        lhSize = 2;
                        litSize = (MEM_readLE16(istart as *const c_void) >> 4) as usize;
                    }
                    3 => {
                        lhSize = 3;
                        if srcSize < 3 {
                            return ERROR(ZSTD_error_corruption_detected);
                        }
                        litSize = (MEM_readLE24(istart as *const c_void) >> 4) as usize;
                    }
                    _ => {
                        /* 0, 2 and default */
                        lhSize = 1;
                        litSize = (*istart.add(0) >> 3) as usize;
                    }
                }

                if litSize > 0 && dst.is_null() {
                    return ERROR(ZSTD_error_dstSize_tooSmall);
                }
                if litSize > blockSizeMax {
                    return ERROR(ZSTD_error_corruption_detected);
                }
                if expectedWriteSize < litSize {
                    return ERROR(ZSTD_error_dstSize_tooSmall);
                }
                ZSTD_allocateLiteralsBuffer(
                    dctx,
                    dst,
                    dstCapacity,
                    litSize,
                    streaming,
                    expectedWriteSize,
                    1,
                );
                if lhSize + litSize + WILDCOPY_OVERLENGTH as usize > srcSize {
                    /* risk reading beyond src buffer with wildcopy */
                    if litSize + lhSize > srcSize {
                        return ERROR(ZSTD_error_corruption_detected);
                    }
                    if (*dctx).litBufferLocation == ZSTD_split {
                        ZSTD_memcpy(
                            (*dctx).litBuffer as *mut c_void,
                            istart.add(lhSize) as *const c_void,
                            litSize - ZSTD_LITBUFFEREXTRASIZE,
                        );
                        ZSTD_memcpy(
                            (*dctx).litExtraBuffer.as_mut_ptr() as *mut c_void,
                            istart.add(lhSize + litSize - ZSTD_LITBUFFEREXTRASIZE) as *const c_void,
                            ZSTD_LITBUFFEREXTRASIZE,
                        );
                    } else {
                        ZSTD_memcpy(
                            (*dctx).litBuffer as *mut c_void,
                            istart.add(lhSize) as *const c_void,
                            litSize,
                        );
                    }
                    (*dctx).litPtr = (*dctx).litBuffer;
                    (*dctx).litSize = litSize;
                    return lhSize + litSize;
                }
                /* direct reference into compressed stream */
                (*dctx).litPtr = istart.add(lhSize);
                (*dctx).litSize = litSize;
                (*dctx).litBufferEnd = (*dctx).litPtr.add(litSize);
                (*dctx).litBufferLocation = ZSTD_not_in_dst;
                return lhSize + litSize;
            }

            set_rle => {
                let lhlCode: U32 = ((*istart.add(0) >> 2) & 3) as U32;
                let litSize: usize;
                let lhSize: usize;
                let expectedWriteSize: usize = MIN(blockSizeMax, dstCapacity);
                match lhlCode {
                    1 => {
                        lhSize = 2;
                        if srcSize < 3 {
                            return ERROR(ZSTD_error_corruption_detected);
                        }
                        litSize = (MEM_readLE16(istart as *const c_void) >> 4) as usize;
                    }
                    3 => {
                        lhSize = 3;
                        if srcSize < 4 {
                            return ERROR(ZSTD_error_corruption_detected);
                        }
                        litSize = (MEM_readLE24(istart as *const c_void) >> 4) as usize;
                    }
                    _ => {
                        /* 0, 2 and default */
                        lhSize = 1;
                        litSize = (*istart.add(0) >> 3) as usize;
                    }
                }
                if litSize > 0 && dst.is_null() {
                    return ERROR(ZSTD_error_dstSize_tooSmall);
                }
                if litSize > blockSizeMax {
                    return ERROR(ZSTD_error_corruption_detected);
                }
                if expectedWriteSize < litSize {
                    return ERROR(ZSTD_error_dstSize_tooSmall);
                }
                ZSTD_allocateLiteralsBuffer(
                    dctx,
                    dst,
                    dstCapacity,
                    litSize,
                    streaming,
                    expectedWriteSize,
                    1,
                );
                if (*dctx).litBufferLocation == ZSTD_split {
                    ZSTD_memset(
                        (*dctx).litBuffer as *mut c_void,
                        *istart.add(lhSize) as c_int,
                        litSize - ZSTD_LITBUFFEREXTRASIZE,
                    );
                    ZSTD_memset(
                        (*dctx).litExtraBuffer.as_mut_ptr() as *mut c_void,
                        *istart.add(lhSize) as c_int,
                        ZSTD_LITBUFFEREXTRASIZE,
                    );
                } else {
                    ZSTD_memset(
                        (*dctx).litBuffer as *mut c_void,
                        *istart.add(lhSize) as c_int,
                        litSize,
                    );
                }
                (*dctx).litPtr = (*dctx).litBuffer;
                (*dctx).litSize = litSize;
                return lhSize + 1;
            }
            _ => {
                return ERROR(ZSTD_error_corruption_detected);
            }
        }
    }
}

/* Hidden declaration for fullbench */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_decodeLiteralsBlock_wrapper(
    dctx: *mut ZSTD_DCtx,
    src: *const c_void,
    srcSize: usize,
    dst: *mut c_void,
    dstCapacity: usize,
) -> usize {
    (*dctx).isFrameDecompression = 0;
    ZSTD_decodeLiteralsBlock(dctx, src, srcSize, dst, dstCapacity, not_streaming)
}

/* Default FSE distribution tables.
 * These are pre-calculated FSE decoding tables using default distributions as defined in
 * specification :
 * https://github.com/facebook/zstd/blob/release/doc/zstd_compression_format.md#default-distributions
 */

/* Default FSE distribution table for Literal Lengths */
static LL_defaultDTable: [ZSTD_seqSymbol; (1 << LL_DEFAULTNORMLOG) + 1] = [
    ZSTD_seqSymbol { nextState: 1, nbAdditionalBits: 1, nbBits: 1, baseValue: LL_DEFAULTNORMLOG },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 0, nbBits: 4, baseValue: 0 },
    ZSTD_seqSymbol { nextState: 16, nbAdditionalBits: 0, nbBits: 4, baseValue: 0 },
    ZSTD_seqSymbol { nextState: 32, nbAdditionalBits: 0, nbBits: 5, baseValue: 1 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 0, nbBits: 5, baseValue: 3 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 0, nbBits: 5, baseValue: 4 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 0, nbBits: 5, baseValue: 6 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 0, nbBits: 5, baseValue: 7 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 0, nbBits: 5, baseValue: 9 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 0, nbBits: 5, baseValue: 10 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 0, nbBits: 5, baseValue: 12 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 0, nbBits: 6, baseValue: 14 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 1, nbBits: 5, baseValue: 16 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 1, nbBits: 5, baseValue: 20 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 1, nbBits: 5, baseValue: 22 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 2, nbBits: 5, baseValue: 28 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 3, nbBits: 5, baseValue: 32 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 4, nbBits: 5, baseValue: 48 },
    ZSTD_seqSymbol { nextState: 32, nbAdditionalBits: 6, nbBits: 5, baseValue: 64 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 7, nbBits: 5, baseValue: 128 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 8, nbBits: 6, baseValue: 256 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 10, nbBits: 6, baseValue: 1024 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 12, nbBits: 6, baseValue: 4096 },
    ZSTD_seqSymbol { nextState: 32, nbAdditionalBits: 0, nbBits: 4, baseValue: 0 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 0, nbBits: 4, baseValue: 1 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 0, nbBits: 5, baseValue: 2 },
    ZSTD_seqSymbol { nextState: 32, nbAdditionalBits: 0, nbBits: 5, baseValue: 4 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 0, nbBits: 5, baseValue: 5 },
    ZSTD_seqSymbol { nextState: 32, nbAdditionalBits: 0, nbBits: 5, baseValue: 7 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 0, nbBits: 5, baseValue: 8 },
    ZSTD_seqSymbol { nextState: 32, nbAdditionalBits: 0, nbBits: 5, baseValue: 10 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 0, nbBits: 5, baseValue: 11 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 0, nbBits: 6, baseValue: 13 },
    ZSTD_seqSymbol { nextState: 32, nbAdditionalBits: 1, nbBits: 5, baseValue: 16 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 1, nbBits: 5, baseValue: 18 },
    ZSTD_seqSymbol { nextState: 32, nbAdditionalBits: 1, nbBits: 5, baseValue: 22 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 2, nbBits: 5, baseValue: 24 },
    ZSTD_seqSymbol { nextState: 32, nbAdditionalBits: 3, nbBits: 5, baseValue: 32 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 3, nbBits: 5, baseValue: 40 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 6, nbBits: 4, baseValue: 64 },
    ZSTD_seqSymbol { nextState: 16, nbAdditionalBits: 6, nbBits: 4, baseValue: 64 },
    ZSTD_seqSymbol { nextState: 32, nbAdditionalBits: 7, nbBits: 5, baseValue: 128 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 9, nbBits: 6, baseValue: 512 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 11, nbBits: 6, baseValue: 2048 },
    ZSTD_seqSymbol { nextState: 48, nbAdditionalBits: 0, nbBits: 4, baseValue: 0 },
    ZSTD_seqSymbol { nextState: 16, nbAdditionalBits: 0, nbBits: 4, baseValue: 1 },
    ZSTD_seqSymbol { nextState: 32, nbAdditionalBits: 0, nbBits: 5, baseValue: 2 },
    ZSTD_seqSymbol { nextState: 32, nbAdditionalBits: 0, nbBits: 5, baseValue: 3 },
    ZSTD_seqSymbol { nextState: 32, nbAdditionalBits: 0, nbBits: 5, baseValue: 5 },
    ZSTD_seqSymbol { nextState: 32, nbAdditionalBits: 0, nbBits: 5, baseValue: 6 },
    ZSTD_seqSymbol { nextState: 32, nbAdditionalBits: 0, nbBits: 5, baseValue: 8 },
    ZSTD_seqSymbol { nextState: 32, nbAdditionalBits: 0, nbBits: 5, baseValue: 9 },
    ZSTD_seqSymbol { nextState: 32, nbAdditionalBits: 0, nbBits: 5, baseValue: 11 },
    ZSTD_seqSymbol { nextState: 32, nbAdditionalBits: 0, nbBits: 5, baseValue: 12 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 0, nbBits: 6, baseValue: 15 },
    ZSTD_seqSymbol { nextState: 32, nbAdditionalBits: 1, nbBits: 5, baseValue: 18 },
    ZSTD_seqSymbol { nextState: 32, nbAdditionalBits: 1, nbBits: 5, baseValue: 20 },
    ZSTD_seqSymbol { nextState: 32, nbAdditionalBits: 2, nbBits: 5, baseValue: 24 },
    ZSTD_seqSymbol { nextState: 32, nbAdditionalBits: 2, nbBits: 5, baseValue: 28 },
    ZSTD_seqSymbol { nextState: 32, nbAdditionalBits: 3, nbBits: 5, baseValue: 40 },
    ZSTD_seqSymbol { nextState: 32, nbAdditionalBits: 4, nbBits: 5, baseValue: 48 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 16, nbBits: 6, baseValue: 65536 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 15, nbBits: 6, baseValue: 32768 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 14, nbBits: 6, baseValue: 16384 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 13, nbBits: 6, baseValue: 8192 },
];

/* Default FSE distribution table for Offset Codes */
static OF_defaultDTable: [ZSTD_seqSymbol; (1 << OF_DEFAULTNORMLOG) + 1] = [
    ZSTD_seqSymbol { nextState: 1, nbAdditionalBits: 1, nbBits: 1, baseValue: OF_DEFAULTNORMLOG },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 0, nbBits: 5, baseValue: 0 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 6, nbBits: 4, baseValue: 61 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 9, nbBits: 5, baseValue: 509 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 15, nbBits: 5, baseValue: 32765 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 21, nbBits: 5, baseValue: 2097149 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 3, nbBits: 5, baseValue: 5 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 7, nbBits: 4, baseValue: 125 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 12, nbBits: 5, baseValue: 4093 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 18, nbBits: 5, baseValue: 262141 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 23, nbBits: 5, baseValue: 8388605 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 5, nbBits: 5, baseValue: 29 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 8, nbBits: 4, baseValue: 253 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 14, nbBits: 5, baseValue: 16381 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 20, nbBits: 5, baseValue: 1048573 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 2, nbBits: 5, baseValue: 1 },
    ZSTD_seqSymbol { nextState: 16, nbAdditionalBits: 7, nbBits: 4, baseValue: 125 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 11, nbBits: 5, baseValue: 2045 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 17, nbBits: 5, baseValue: 131069 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 22, nbBits: 5, baseValue: 4194301 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 4, nbBits: 5, baseValue: 13 },
    ZSTD_seqSymbol { nextState: 16, nbAdditionalBits: 8, nbBits: 4, baseValue: 253 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 13, nbBits: 5, baseValue: 8189 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 19, nbBits: 5, baseValue: 524285 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 1, nbBits: 5, baseValue: 1 },
    ZSTD_seqSymbol { nextState: 16, nbAdditionalBits: 6, nbBits: 4, baseValue: 61 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 10, nbBits: 5, baseValue: 1021 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 16, nbBits: 5, baseValue: 65533 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 28, nbBits: 5, baseValue: 268435453 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 27, nbBits: 5, baseValue: 134217725 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 26, nbBits: 5, baseValue: 67108861 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 25, nbBits: 5, baseValue: 33554429 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 24, nbBits: 5, baseValue: 16777213 },
];

/* Default FSE distribution table for Match Lengths */
static ML_defaultDTable: [ZSTD_seqSymbol; (1 << ML_DEFAULTNORMLOG) + 1] = [
    ZSTD_seqSymbol { nextState: 1, nbAdditionalBits: 1, nbBits: 1, baseValue: ML_DEFAULTNORMLOG },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 0, nbBits: 6, baseValue: 3 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 0, nbBits: 4, baseValue: 4 },
    ZSTD_seqSymbol { nextState: 32, nbAdditionalBits: 0, nbBits: 5, baseValue: 5 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 0, nbBits: 5, baseValue: 6 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 0, nbBits: 5, baseValue: 8 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 0, nbBits: 5, baseValue: 9 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 0, nbBits: 5, baseValue: 11 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 0, nbBits: 6, baseValue: 13 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 0, nbBits: 6, baseValue: 16 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 0, nbBits: 6, baseValue: 19 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 0, nbBits: 6, baseValue: 22 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 0, nbBits: 6, baseValue: 25 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 0, nbBits: 6, baseValue: 28 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 0, nbBits: 6, baseValue: 31 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 0, nbBits: 6, baseValue: 34 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 1, nbBits: 6, baseValue: 37 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 1, nbBits: 6, baseValue: 41 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 2, nbBits: 6, baseValue: 47 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 3, nbBits: 6, baseValue: 59 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 4, nbBits: 6, baseValue: 83 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 7, nbBits: 6, baseValue: 131 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 9, nbBits: 6, baseValue: 515 },
    ZSTD_seqSymbol { nextState: 16, nbAdditionalBits: 0, nbBits: 4, baseValue: 4 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 0, nbBits: 4, baseValue: 5 },
    ZSTD_seqSymbol { nextState: 32, nbAdditionalBits: 0, nbBits: 5, baseValue: 6 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 0, nbBits: 5, baseValue: 7 },
    ZSTD_seqSymbol { nextState: 32, nbAdditionalBits: 0, nbBits: 5, baseValue: 9 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 0, nbBits: 5, baseValue: 10 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 0, nbBits: 6, baseValue: 12 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 0, nbBits: 6, baseValue: 15 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 0, nbBits: 6, baseValue: 18 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 0, nbBits: 6, baseValue: 21 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 0, nbBits: 6, baseValue: 24 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 0, nbBits: 6, baseValue: 27 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 0, nbBits: 6, baseValue: 30 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 0, nbBits: 6, baseValue: 33 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 1, nbBits: 6, baseValue: 35 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 1, nbBits: 6, baseValue: 39 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 2, nbBits: 6, baseValue: 43 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 3, nbBits: 6, baseValue: 51 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 4, nbBits: 6, baseValue: 67 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 5, nbBits: 6, baseValue: 99 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 8, nbBits: 6, baseValue: 259 },
    ZSTD_seqSymbol { nextState: 32, nbAdditionalBits: 0, nbBits: 4, baseValue: 4 },
    ZSTD_seqSymbol { nextState: 48, nbAdditionalBits: 0, nbBits: 4, baseValue: 4 },
    ZSTD_seqSymbol { nextState: 16, nbAdditionalBits: 0, nbBits: 4, baseValue: 5 },
    ZSTD_seqSymbol { nextState: 32, nbAdditionalBits: 0, nbBits: 5, baseValue: 7 },
    ZSTD_seqSymbol { nextState: 32, nbAdditionalBits: 0, nbBits: 5, baseValue: 8 },
    ZSTD_seqSymbol { nextState: 32, nbAdditionalBits: 0, nbBits: 5, baseValue: 10 },
    ZSTD_seqSymbol { nextState: 32, nbAdditionalBits: 0, nbBits: 5, baseValue: 11 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 0, nbBits: 6, baseValue: 14 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 0, nbBits: 6, baseValue: 17 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 0, nbBits: 6, baseValue: 20 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 0, nbBits: 6, baseValue: 23 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 0, nbBits: 6, baseValue: 26 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 0, nbBits: 6, baseValue: 29 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 0, nbBits: 6, baseValue: 32 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 16, nbBits: 6, baseValue: 65539 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 15, nbBits: 6, baseValue: 32771 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 14, nbBits: 6, baseValue: 16387 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 13, nbBits: 6, baseValue: 8195 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 12, nbBits: 6, baseValue: 4099 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 11, nbBits: 6, baseValue: 2051 },
    ZSTD_seqSymbol { nextState: 0, nbAdditionalBits: 10, nbBits: 6, baseValue: 1027 },
];

unsafe fn ZSTD_buildSeqTable_rle(dt: *mut ZSTD_seqSymbol, baseValue: U32, nbAddBits: U8) {
    let ptr = dt as *mut c_void;
    let DTableH = ptr as *mut ZSTD_seqSymbol_header;
    let cell = dt.add(1);

    (*DTableH).tableLog = 0;
    (*DTableH).fastMode = 0;

    (*cell).nbBits = 0;
    (*cell).nextState = 0;
    (*cell).nbAdditionalBits = nbAddBits;
    (*cell).baseValue = baseValue;
}

/// `ZSTD_buildFSETable()` :
/// generate FSE decoding table for one symbol (ll, ml or off)
/// cannot fail if input is valid =>
/// all inputs are presumed validated at this stage
#[inline(always)]
unsafe fn ZSTD_buildFSETable_body(
    dt: *mut ZSTD_seqSymbol,
    normalizedCounter: *const i16,
    maxSymbolValue: c_uint,
    baseValue: *const U32,
    nbAdditionalBits: *const U8,
    tableLog: c_uint,
    wksp: *mut c_void,
    wkspSize: usize,
) {
    let tableDecode = dt.add(1);
    let maxSV1: U32 = maxSymbolValue + 1;
    let tableSize: U32 = 1u32 << tableLog;

    let symbolNext = wksp as *mut U16;
    let spread = symbolNext.add(MaxSeq as usize + 1) as *mut BYTE;
    let mut highThreshold: U32 = tableSize - 1;

    /* Init, lay down lowprob symbols */
    {
        let mut DTableH = ZSTD_seqSymbol_header {
            fastMode: 0,
            tableLog: 0,
        };
        DTableH.tableLog = tableLog as U32;
        DTableH.fastMode = 1;
        {
            let largeLimit: S16 = (1i32 << (tableLog - 1)) as S16;
            let mut s: U32 = 0;
            while s < maxSV1 {
                if *normalizedCounter.add(s as usize) == -1 {
                    (*tableDecode.add(highThreshold as usize)).baseValue = s;
                    highThreshold = highThreshold.wrapping_sub(1);
                    *symbolNext.add(s as usize) = 1;
                } else {
                    if *normalizedCounter.add(s as usize) >= largeLimit {
                        DTableH.fastMode = 0;
                    }
                    *symbolNext.add(s as usize) = *normalizedCounter.add(s as usize) as U16;
                }
                s += 1;
            }
        }
        ZSTD_memcpy(
            dt as *mut c_void,
            &DTableH as *const ZSTD_seqSymbol_header as *const c_void,
            core::mem::size_of::<ZSTD_seqSymbol_header>(),
        );
    }

    /* Spread symbols */
    /* Specialized symbol spreading for the case when there are
     * no low probability (-1 count) symbols. When compressing
     * small blocks we avoid low probability symbols to hit this
     * case, since header decoding speed matters more.
     */
    if highThreshold == tableSize - 1 {
        let tableMask: usize = (tableSize - 1) as usize;
        let step: usize = FSE_TABLESTEP(tableSize) as usize;
        /* First lay down the symbols in order.
         * We use a uint64_t to lay down 8 bytes at a time. This reduces branch
         * misses since small blocks generally have small table logs, so nearly
         * all symbols have counts <= 8. We ensure we have 8 bytes at the end of
         * our buffer to handle the over-write.
         */
        {
            let add: U64 = 0x0101010101010101u64;
            let mut pos: usize = 0;
            let mut sv: U64 = 0;
            let mut s: U32 = 0;
            while s < maxSV1 {
                let n: c_int = *normalizedCounter.add(s as usize) as c_int;
                MEM_write64(spread.add(pos) as *mut c_void, sv);
                let mut i: c_int = 8;
                while i < n {
                    MEM_write64(spread.add(pos + i as usize) as *mut c_void, sv);
                    i += 8;
                }
                pos += n as usize;
                s += 1;
                sv = sv.wrapping_add(add);
            }
        }
        /* Now we spread those positions across the table.
         * The benefit of doing it in two stages is that we avoid the
         * variable size inner loop, which caused lots of branch misses.
         * Now we can run through all the positions without any branch misses.
         * We unroll the loop twice, since that is what empirically worked best.
         */
        {
            let mut position: usize = 0;
            let mut s: usize;
            let unroll: usize = 2;
            s = 0;
            while s < tableSize as usize {
                let mut u: usize = 0;
                while u < unroll {
                    let uPosition: usize = (position + (u * step)) & tableMask;
                    (*tableDecode.add(uPosition)).baseValue = *spread.add(s + u) as U32;
                    u += 1;
                }
                position = (position + (unroll * step)) & tableMask;
                s += unroll;
            }
        }
    } else {
        let tableMask: U32 = tableSize - 1;
        let step: U32 = FSE_TABLESTEP(tableSize);
        let mut s: U32;
        let mut position: U32 = 0;
        s = 0;
        while s < maxSV1 {
            let n: c_int = *normalizedCounter.add(s as usize) as c_int;
            let mut i: c_int = 0;
            while i < n {
                (*tableDecode.add(position as usize)).baseValue = s;
                position = (position + step) & tableMask;
                while position > highThreshold {
                    position = (position + step) & tableMask; /* lowprob area */
                }
                i += 1;
            }
            s += 1;
        }
    }

    /* Build Decoding table */
    {
        let mut u: U32 = 0;
        while u < tableSize {
            let symbol: U32 = (*tableDecode.add(u as usize)).baseValue;
            let nextState: U32 = *symbolNext.add(symbol as usize) as U32;
            *symbolNext.add(symbol as usize) =
                (*symbolNext.add(symbol as usize)).wrapping_add(1);
            (*tableDecode.add(u as usize)).nbBits =
                (tableLog as U32).wrapping_sub(ZSTD_highbit32(nextState)) as BYTE;
            (*tableDecode.add(u as usize)).nextState = ((nextState
                << (*tableDecode.add(u as usize)).nbBits as U32)
                .wrapping_sub(tableSize)) as U16;
            (*tableDecode.add(u as usize)).nbAdditionalBits =
                *nbAdditionalBits.add(symbol as usize);
            (*tableDecode.add(u as usize)).baseValue = *baseValue.add(symbol as usize);
            u += 1;
        }
    }
}

/* Avoids the FORCE_INLINE of the _body() function. */
unsafe fn ZSTD_buildFSETable_body_default(
    dt: *mut ZSTD_seqSymbol,
    normalizedCounter: *const i16,
    maxSymbolValue: c_uint,
    baseValue: *const U32,
    nbAdditionalBits: *const U8,
    tableLog: c_uint,
    wksp: *mut c_void,
    wkspSize: usize,
) {
    ZSTD_buildFSETable_body(
        dt,
        normalizedCounter,
        maxSymbolValue,
        baseValue,
        nbAdditionalBits,
        tableLog,
        wksp,
        wkspSize,
    );
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_buildFSETable(
    dt: *mut ZSTD_seqSymbol,
    normalizedCounter: *const i16,
    maxSymbolValue: c_uint,
    baseValue: *const U32,
    nbAdditionalBits: *const U8,
    tableLog: c_uint,
    wksp: *mut c_void,
    wkspSize: usize,
    bmi2: c_int,
) {
    /* DYNAMIC_BMI2 == 0 : always the default variant */
    ZSTD_buildFSETable_body_default(
        dt,
        normalizedCounter,
        maxSymbolValue,
        baseValue,
        nbAdditionalBits,
        tableLog,
        wksp,
        wkspSize,
    );
}

/// `ZSTD_buildSeqTable()` :
/// @return : nb bytes read from src,
///           or an error code if it fails
unsafe fn ZSTD_buildSeqTable(
    DTableSpace: *mut ZSTD_seqSymbol,
    DTablePtr: *mut *const ZSTD_seqSymbol,
    type_: SymbolEncodingType_e,
    mut max: c_uint,
    maxLog: U32,
    src: *const c_void,
    srcSize: usize,
    baseValue: *const U32,
    nbAdditionalBits: *const U8,
    defaultTable: *const ZSTD_seqSymbol,
    flagRepeatTable: U32,
    ddictIsCold: c_int,
    nbSeq: c_int,
    wksp: *mut U32,
    wkspSize: usize,
    bmi2: c_int,
) -> usize {
    match type_ {
        set_rle => {
            if srcSize == 0 {
                return ERROR(ZSTD_error_srcSize_wrong);
            }
            if (*(src as *const BYTE)) as c_uint > max {
                return ERROR(ZSTD_error_corruption_detected);
            }
            {
                let symbol: U32 = *(src as *const BYTE) as U32;
                let baseline: U32 = *baseValue.add(symbol as usize);
                let nbBits: U8 = *nbAdditionalBits.add(symbol as usize);
                ZSTD_buildSeqTable_rle(DTableSpace, baseline, nbBits);
            }
            *DTablePtr = DTableSpace;
            1
        }
        set_basic => {
            *DTablePtr = defaultTable;
            0
        }
        set_repeat => {
            if flagRepeatTable == 0 {
                return ERROR(ZSTD_error_corruption_detected);
            }
            /* prefetch FSE table if used : PREFETCH_AREA is a no-op in this port */
            0
        }
        set_compressed => {
            let mut tableLog: c_uint = 0;
            let mut norm: [S16; MaxSeq as usize + 1] = [0; MaxSeq as usize + 1];
            let headerSize: usize = FSE_readNCount(
                norm.as_mut_ptr(),
                &mut max,
                &mut tableLog,
                src,
                srcSize,
            );
            if ERR_isError(headerSize) != 0 {
                return ERROR(ZSTD_error_corruption_detected);
            }
            if tableLog > maxLog {
                return ERROR(ZSTD_error_corruption_detected);
            }
            ZSTD_buildFSETable(
                DTableSpace,
                norm.as_ptr(),
                max,
                baseValue,
                nbAdditionalBits,
                tableLog,
                wksp as *mut c_void,
                wkspSize,
                bmi2,
            );
            *DTablePtr = DTableSpace;
            headerSize
        }
        _ => ERROR(ZSTD_error_GENERIC),
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_decodeSeqHeaders(
    dctx: *mut ZSTD_DCtx,
    nbSeqPtr: *mut c_int,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let istart = src as *const BYTE;
    let iend = istart.wrapping_add(srcSize);
    let mut ip = istart;
    let mut nbSeq: c_int;

    /* check */
    if srcSize < MIN_SEQUENCES_SIZE {
        return ERROR(ZSTD_error_srcSize_wrong);
    }

    /* SeqHead */
    nbSeq = *ip as c_int;
    ip = ip.add(1);
    if nbSeq > 0x7F {
        if nbSeq == 0xFF {
            if ip.wrapping_add(2) > iend {
                return ERROR(ZSTD_error_srcSize_wrong);
            }
            nbSeq =
                (MEM_readLE16(ip as *const c_void) as U32).wrapping_add(LONGNBSEQ) as c_int;
            ip = ip.add(2);
        } else {
            if ip >= iend {
                return ERROR(ZSTD_error_srcSize_wrong);
            }
            nbSeq = ((nbSeq - 0x80) << 8) + *ip as c_int;
            ip = ip.add(1);
        }
    }
    *nbSeqPtr = nbSeq;

    if nbSeq == 0 {
        /* No sequence : section ends immediately */
        if ip != iend {
            return ERROR(ZSTD_error_corruption_detected);
        }
        return ip.offset_from(istart) as usize;
    }

    /* FSE table descriptors */
    if ip.wrapping_add(1) > iend {
        /* minimum possible size: 1 byte for symbol encoding types */
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    if (*ip & 3) != 0 {
        /* The last field, Reserved, must be all-zeroes. */
        return ERROR(ZSTD_error_corruption_detected);
    }
    {
        let LLtype: SymbolEncodingType_e = (*ip >> 6) as SymbolEncodingType_e;
        let OFtype: SymbolEncodingType_e = ((*ip >> 4) & 3) as SymbolEncodingType_e;
        let MLtype: SymbolEncodingType_e = ((*ip >> 2) & 3) as SymbolEncodingType_e;
        ip = ip.add(1);

        /* Build DTables */
        {
            let llhSize: usize = ZSTD_buildSeqTable(
                (*dctx).entropy.LLTable.as_mut_ptr(),
                &mut (*dctx).LLTptr,
                LLtype,
                MaxLL,
                LLFSELog,
                ip as *const c_void,
                iend.offset_from(ip) as usize,
                LL_base.as_ptr(),
                LL_bits.as_ptr(),
                LL_defaultDTable.as_ptr(),
                (*dctx).fseEntropy,
                (*dctx).ddictIsCold,
                nbSeq,
                (*dctx).workspace.as_mut_ptr(),
                core::mem::size_of_val(&(*dctx).workspace),
                ZSTD_DCtx_get_bmi2(dctx),
            );
            if ERR_isError(llhSize) != 0 {
                return ERROR(ZSTD_error_corruption_detected);
            }
            ip = ip.add(llhSize);
        }

        {
            let ofhSize: usize = ZSTD_buildSeqTable(
                (*dctx).entropy.OFTable.as_mut_ptr(),
                &mut (*dctx).OFTptr,
                OFtype,
                MaxOff,
                OffFSELog,
                ip as *const c_void,
                iend.offset_from(ip) as usize,
                OF_base.as_ptr(),
                OF_bits.as_ptr(),
                OF_defaultDTable.as_ptr(),
                (*dctx).fseEntropy,
                (*dctx).ddictIsCold,
                nbSeq,
                (*dctx).workspace.as_mut_ptr(),
                core::mem::size_of_val(&(*dctx).workspace),
                ZSTD_DCtx_get_bmi2(dctx),
            );
            if ERR_isError(ofhSize) != 0 {
                return ERROR(ZSTD_error_corruption_detected);
            }
            ip = ip.add(ofhSize);
        }

        {
            let mlhSize: usize = ZSTD_buildSeqTable(
                (*dctx).entropy.MLTable.as_mut_ptr(),
                &mut (*dctx).MLTptr,
                MLtype,
                MaxML,
                MLFSELog,
                ip as *const c_void,
                iend.offset_from(ip) as usize,
                ML_base.as_ptr(),
                ML_bits.as_ptr(),
                ML_defaultDTable.as_ptr(),
                (*dctx).fseEntropy,
                (*dctx).ddictIsCold,
                nbSeq,
                (*dctx).workspace.as_mut_ptr(),
                core::mem::size_of_val(&(*dctx).workspace),
                ZSTD_DCtx_get_bmi2(dctx),
            );
            if ERR_isError(mlhSize) != 0 {
                return ERROR(ZSTD_error_corruption_detected);
            }
            ip = ip.add(mlhSize);
        }
    }

    ip.offset_from(istart) as usize
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
struct seq_t {
    litLength: usize,
    matchLength: usize,
    offset: usize,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct ZSTD_fseState {
    state: usize,
    table: *const ZSTD_seqSymbol,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct seqState_t {
    DStream: BIT_DStream_t,
    stateLL: ZSTD_fseState,
    stateOffb: ZSTD_fseState,
    stateML: ZSTD_fseState,
    prevOffset: [usize; ZSTD_REP_NUM],
}

/// `ZSTD_overlapCopy8()` :
///  Copies 8 bytes from ip to op and updates op and ip where ip <= op.
///  If the offset is < 8 then the offset is spread to at least 8 bytes.
#[inline]
unsafe fn ZSTD_overlapCopy8(op: *mut *mut BYTE, ip: *mut *const BYTE, offset: usize) {
    if offset < 8 {
        /* close range match, overlap */
        static dec32table: [U32; 8] = [0, 1, 2, 1, 4, 4, 4, 4]; /* added */
        static dec64table: [c_int; 8] = [8, 8, 8, 7, 8, 9, 10, 11]; /* subtracted */
        let sub2: c_int = dec64table[offset];
        *(*op).add(0) = *(*ip).add(0);
        *(*op).add(1) = *(*ip).add(1);
        *(*op).add(2) = *(*ip).add(2);
        *(*op).add(3) = *(*ip).add(3);
        *ip = (*ip).wrapping_add(dec32table[offset] as usize);
        ZSTD_copy4((*op).add(4) as *mut c_void, *ip as *const c_void);
        *ip = (*ip).wrapping_offset(-(sub2 as isize));
    } else {
        ZSTD_copy8(*op as *mut c_void, *ip as *const c_void);
    }
    *ip = (*ip).wrapping_add(8);
    *op = (*op).wrapping_add(8);
}

/// `ZSTD_safecopy()` :
///  Specialized version of memcpy() that is allowed to READ up to WILDCOPY_OVERLENGTH past the
///  input buffer and write up to 16 bytes past oend_w (op >= oend_w is allowed).
unsafe fn ZSTD_safecopy(
    mut op: *mut BYTE,
    oend_w: *const BYTE,
    mut ip: *const BYTE,
    mut length: isize,
    ovtype: ZSTD_overlap_e,
) {
    let diff: isize = op.offset_from(ip);
    let oend = op.wrapping_offset(length);

    if length < 8 {
        /* Handle short lengths. */
        while op < oend {
            *op = *ip;
            op = op.add(1);
            ip = ip.add(1);
        }
        return;
    }
    if ovtype == ZSTD_overlap_src_before_dst {
        /* Copy 8 bytes and ensure the offset >= 8 when there can be overlap. */
        ZSTD_overlapCopy8(&mut op, &mut ip, diff as usize);
        length -= 8;
    }

    if oend <= oend_w as *mut BYTE {
        /* No risk of overwrite. */
        ZSTD_wildcopy(op as *mut c_void, ip as *const c_void, length, ovtype);
        return;
    }
    if op <= oend_w as *mut BYTE {
        /* Wildcopy until we get close to the end. */
        ZSTD_wildcopy(
            op as *mut c_void,
            ip as *const c_void,
            oend_w.offset_from(op),
            ovtype,
        );
        ip = ip.wrapping_offset(oend_w.offset_from(op));
        op = op.wrapping_offset(oend_w.offset_from(op));
    }
    /* Handle the leftovers. */
    while op < oend {
        *op = *ip;
        op = op.add(1);
        ip = ip.add(1);
    }
}

/// `ZSTD_safecopyDstBeforeSrc()`:
/// This version allows overlap with dst before src, or handles the non-overlap case with dst
/// after src
unsafe fn ZSTD_safecopyDstBeforeSrc(mut op: *mut BYTE, mut ip: *const BYTE, length: isize) {
    let diff: isize = op.offset_from(ip);
    let oend = op.wrapping_offset(length);

    if length < 8 || diff > -8 {
        /* Handle short lengths, close overlaps, and dst not before src. */
        while op < oend {
            *op = *ip;
            op = op.add(1);
            ip = ip.add(1);
        }
        return;
    }

    if op <= oend.wrapping_offset(-WILDCOPY_OVERLENGTH) && diff < -WILDCOPY_VECLEN {
        ZSTD_wildcopy(
            op as *mut c_void,
            ip as *const c_void,
            oend.wrapping_offset(-WILDCOPY_OVERLENGTH).offset_from(op),
            ZSTD_no_overlap,
        );
        ip = ip.wrapping_offset(oend.wrapping_offset(-WILDCOPY_OVERLENGTH).offset_from(op));
        op = op.wrapping_offset(oend.wrapping_offset(-WILDCOPY_OVERLENGTH).offset_from(op));
    }

    /* Handle the leftovers. */
    while op < oend {
        *op = *ip;
        op = op.add(1);
        ip = ip.add(1);
    }
}

/// `ZSTD_execSequenceEnd()`:
/// This version handles cases that are near the end of the output buffer.
#[inline(never)]
unsafe fn ZSTD_execSequenceEnd(
    mut op: *mut BYTE,
    oend: *mut BYTE,
    mut sequence: seq_t,
    litPtr: *mut *const BYTE,
    litLimit: *const BYTE,
    prefixStart: *const BYTE,
    virtualStart: *const BYTE,
    dictEnd: *const BYTE,
) -> usize {
    let oLitEnd = op.wrapping_add(sequence.litLength);
    let sequenceLength: usize = sequence.litLength + sequence.matchLength;
    let iLitEnd = (*litPtr).wrapping_add(sequence.litLength);
    let mut match_ = oLitEnd.wrapping_sub(sequence.offset) as *const BYTE;
    let oend_w = oend.wrapping_offset(-WILDCOPY_OVERLENGTH);

    /* bounds checks : careful of address space overflow in 32-bit mode */
    if sequenceLength > oend.offset_from(op) as usize {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    if sequence.litLength > litLimit.offset_from(*litPtr) as usize {
        return ERROR(ZSTD_error_corruption_detected);
    }

    /* copy literals */
    ZSTD_safecopy(
        op,
        oend_w,
        *litPtr,
        sequence.litLength as isize,
        ZSTD_no_overlap,
    );
    op = oLitEnd;
    *litPtr = iLitEnd;

    /* copy Match */
    if sequence.offset > oLitEnd.offset_from(prefixStart) as usize {
        /* offset beyond prefix */
        if sequence.offset > oLitEnd.offset_from(virtualStart) as usize {
            return ERROR(ZSTD_error_corruption_detected);
        }
        match_ = dictEnd.wrapping_offset(-(prefixStart.offset_from(match_)));
        if match_.wrapping_add(sequence.matchLength) <= dictEnd {
            ZSTD_memmove(
                oLitEnd as *mut c_void,
                match_ as *const c_void,
                sequence.matchLength,
            );
            return sequenceLength;
        }
        /* span extDict & currentPrefixSegment */
        {
            let length1: usize = dictEnd.offset_from(match_) as usize;
            ZSTD_memmove(oLitEnd as *mut c_void, match_ as *const c_void, length1);
            op = oLitEnd.wrapping_add(length1);
            sequence.matchLength -= length1;
            match_ = prefixStart;
        }
    }
    ZSTD_safecopy(
        op,
        oend_w,
        match_,
        sequence.matchLength as isize,
        ZSTD_overlap_src_before_dst,
    );
    sequenceLength
}

/// `ZSTD_execSequenceEndSplitLitBuffer()`:
/// This version is intended to be used during instances where the litBuffer is still split.
#[inline(never)]
unsafe fn ZSTD_execSequenceEndSplitLitBuffer(
    mut op: *mut BYTE,
    oend: *mut BYTE,
    oend_w: *const BYTE,
    mut sequence: seq_t,
    litPtr: *mut *const BYTE,
    litLimit: *const BYTE,
    prefixStart: *const BYTE,
    virtualStart: *const BYTE,
    dictEnd: *const BYTE,
) -> usize {
    let oLitEnd = op.wrapping_add(sequence.litLength);
    let sequenceLength: usize = sequence.litLength + sequence.matchLength;
    let iLitEnd = (*litPtr).wrapping_add(sequence.litLength);
    let mut match_ = oLitEnd.wrapping_sub(sequence.offset) as *const BYTE;

    /* bounds checks : careful of address space overflow in 32-bit mode */
    if sequenceLength > oend.offset_from(op) as usize {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    if sequence.litLength > litLimit.offset_from(*litPtr) as usize {
        return ERROR(ZSTD_error_corruption_detected);
    }

    /* copy literals */
    if op as *const BYTE > *litPtr && (op as *const BYTE) < (*litPtr).wrapping_add(sequence.litLength)
    {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    ZSTD_safecopyDstBeforeSrc(op, *litPtr, sequence.litLength as isize);
    op = oLitEnd;
    *litPtr = iLitEnd;

    /* copy Match */
    if sequence.offset > oLitEnd.offset_from(prefixStart) as usize {
        /* offset beyond prefix */
        if sequence.offset > oLitEnd.offset_from(virtualStart) as usize {
            return ERROR(ZSTD_error_corruption_detected);
        }
        match_ = dictEnd.wrapping_offset(-(prefixStart.offset_from(match_)));
        if match_.wrapping_add(sequence.matchLength) <= dictEnd {
            ZSTD_memmove(
                oLitEnd as *mut c_void,
                match_ as *const c_void,
                sequence.matchLength,
            );
            return sequenceLength;
        }
        /* span extDict & currentPrefixSegment */
        {
            let length1: usize = dictEnd.offset_from(match_) as usize;
            ZSTD_memmove(oLitEnd as *mut c_void, match_ as *const c_void, length1);
            op = oLitEnd.wrapping_add(length1);
            sequence.matchLength -= length1;
            match_ = prefixStart;
        }
    }
    ZSTD_safecopy(
        op,
        oend_w,
        match_,
        sequence.matchLength as isize,
        ZSTD_overlap_src_before_dst,
    );
    sequenceLength
}

#[inline]
unsafe fn ZSTD_execSequence(
    mut op: *mut BYTE,
    oend: *mut BYTE,
    mut sequence: seq_t,
    litPtr: *mut *const BYTE,
    litLimit: *const BYTE,
    prefixStart: *const BYTE,
    virtualStart: *const BYTE,
    dictEnd: *const BYTE,
) -> usize {
    let oLitEnd = op.wrapping_add(sequence.litLength);
    let sequenceLength: usize = sequence.litLength + sequence.matchLength;
    /* risk : address space overflow (32-bits) */
    let oMatchEnd = op.wrapping_add(sequenceLength);
    /* risk : address space underflow on oend=NULL */
    let oend_w = oend.wrapping_offset(-WILDCOPY_OVERLENGTH);
    let iLitEnd = (*litPtr).wrapping_add(sequence.litLength);
    let mut match_ = oLitEnd.wrapping_sub(sequence.offset) as *const BYTE;

    /* Handle edge cases in a slow path:
     *   - Read beyond end of literals
     *   - Match end is within WILDCOPY_OVERLIMIT of oend
     *   - 32-bit mode and the match length overflows
     */
    if iLitEnd > litLimit
        || oMatchEnd > oend_w
        || (MEM_32bits() != 0
            && (oend.offset_from(op) as usize) < sequenceLength + WILDCOPY_OVERLENGTH as usize)
    {
        return ZSTD_execSequenceEnd(
            op,
            oend,
            sequence,
            litPtr,
            litLimit,
            prefixStart,
            virtualStart,
            dictEnd,
        );
    }

    /* Copy Literals:
     * Split out litLength <= 16 since it is nearly always true. +1.6% on gcc-9.
     * We likely don't need the full 32-byte wildcopy.
     */
    ZSTD_copy16(op as *mut c_void, *litPtr as *const c_void);
    if sequence.litLength > 16 {
        ZSTD_wildcopy(
            op.add(16) as *mut c_void,
            (*litPtr).add(16) as *const c_void,
            (sequence.litLength - 16) as isize,
            ZSTD_no_overlap,
        );
    }
    op = oLitEnd;
    *litPtr = iLitEnd; /* update for next sequence */

    /* Copy Match */
    if sequence.offset > oLitEnd.offset_from(prefixStart) as usize {
        /* offset beyond prefix -> go into extDict */
        if sequence.offset > oLitEnd.offset_from(virtualStart) as usize {
            return ERROR(ZSTD_error_corruption_detected);
        }
        match_ = dictEnd.wrapping_offset(match_.offset_from(prefixStart));
        if match_.wrapping_add(sequence.matchLength) <= dictEnd {
            ZSTD_memmove(
                oLitEnd as *mut c_void,
                match_ as *const c_void,
                sequence.matchLength,
            );
            return sequenceLength;
        }
        /* span extDict & currentPrefixSegment */
        {
            let length1: usize = dictEnd.offset_from(match_) as usize;
            ZSTD_memmove(oLitEnd as *mut c_void, match_ as *const c_void, length1);
            op = oLitEnd.wrapping_add(length1);
            sequence.matchLength -= length1;
            match_ = prefixStart;
        }
    }
    /* Match within prefix of 1 or more bytes */

    /* Nearly all offsets are >= WILDCOPY_VECLEN bytes, which means we can use wildcopy
     * without overlap checking.
     */
    if sequence.offset >= WILDCOPY_VECLEN as usize {
        /* We bet on a full wildcopy for matches, since we expect matches to be
         * longer than literals (in general). In silesia, ~10% of matches are longer
         * than 16 bytes.
         */
        ZSTD_wildcopy(
            op as *mut c_void,
            match_ as *const c_void,
            sequence.matchLength as isize,
            ZSTD_no_overlap,
        );
        return sequenceLength;
    }

    /* Copy 8 bytes and spread the offset to be >= 8. */
    ZSTD_overlapCopy8(&mut op, &mut match_, sequence.offset);

    /* If the match length is > 8 bytes, then continue with the wildcopy. */
    if sequence.matchLength > 8 {
        ZSTD_wildcopy(
            op as *mut c_void,
            match_ as *const c_void,
            sequence.matchLength as isize - 8,
            ZSTD_overlap_src_before_dst,
        );
    }
    sequenceLength
}

#[inline]
unsafe fn ZSTD_execSequenceSplitLitBuffer(
    mut op: *mut BYTE,
    oend: *mut BYTE,
    oend_w: *const BYTE,
    mut sequence: seq_t,
    litPtr: *mut *const BYTE,
    litLimit: *const BYTE,
    prefixStart: *const BYTE,
    virtualStart: *const BYTE,
    dictEnd: *const BYTE,
) -> usize {
    let oLitEnd = op.wrapping_add(sequence.litLength);
    let sequenceLength: usize = sequence.litLength + sequence.matchLength;
    /* risk : address space overflow (32-bits) */
    let oMatchEnd = op.wrapping_add(sequenceLength);
    let iLitEnd = (*litPtr).wrapping_add(sequence.litLength);
    let mut match_ = oLitEnd.wrapping_sub(sequence.offset) as *const BYTE;

    /* Handle edge cases in a slow path:
     *   - Read beyond end of literals
     *   - Match end is within WILDCOPY_OVERLIMIT of oend
     *   - 32-bit mode and the match length overflows
     */
    if iLitEnd > litLimit
        || oMatchEnd as *const BYTE > oend_w
        || (MEM_32bits() != 0
            && (oend.offset_from(op) as usize) < sequenceLength + WILDCOPY_OVERLENGTH as usize)
    {
        return ZSTD_execSequenceEndSplitLitBuffer(
            op,
            oend,
            oend_w,
            sequence,
            litPtr,
            litLimit,
            prefixStart,
            virtualStart,
            dictEnd,
        );
    }

    /* Copy Literals:
     * Split out litLength <= 16 since it is nearly always true. +1.6% on gcc-9.
     * We likely don't need the full 32-byte wildcopy.
     */
    ZSTD_copy16(op as *mut c_void, *litPtr as *const c_void);
    if sequence.litLength > 16 {
        ZSTD_wildcopy(
            op.add(16) as *mut c_void,
            (*litPtr).add(16) as *const c_void,
            (sequence.litLength - 16) as isize,
            ZSTD_no_overlap,
        );
    }
    op = oLitEnd;
    *litPtr = iLitEnd; /* update for next sequence */

    /* Copy Match */
    if sequence.offset > oLitEnd.offset_from(prefixStart) as usize {
        /* offset beyond prefix -> go into extDict */
        if sequence.offset > oLitEnd.offset_from(virtualStart) as usize {
            return ERROR(ZSTD_error_corruption_detected);
        }
        match_ = dictEnd.wrapping_offset(match_.offset_from(prefixStart));
        if match_.wrapping_add(sequence.matchLength) <= dictEnd {
            ZSTD_memmove(
                oLitEnd as *mut c_void,
                match_ as *const c_void,
                sequence.matchLength,
            );
            return sequenceLength;
        }
        /* span extDict & currentPrefixSegment */
        {
            let length1: usize = dictEnd.offset_from(match_) as usize;
            ZSTD_memmove(oLitEnd as *mut c_void, match_ as *const c_void, length1);
            op = oLitEnd.wrapping_add(length1);
            sequence.matchLength -= length1;
            match_ = prefixStart;
        }
    }
    /* Match within prefix of 1 or more bytes */

    /* Nearly all offsets are >= WILDCOPY_VECLEN bytes, which means we can use wildcopy
     * without overlap checking.
     */
    if sequence.offset >= WILDCOPY_VECLEN as usize {
        /* We bet on a full wildcopy for matches, since we expect matches to be
         * longer than literals (in general). In silesia, ~10% of matches are longer
         * than 16 bytes.
         */
        ZSTD_wildcopy(
            op as *mut c_void,
            match_ as *const c_void,
            sequence.matchLength as isize,
            ZSTD_no_overlap,
        );
        return sequenceLength;
    }

    /* Copy 8 bytes and spread the offset to be >= 8. */
    ZSTD_overlapCopy8(&mut op, &mut match_, sequence.offset);

    /* If the match length is > 8 bytes, then continue with the wildcopy. */
    if sequence.matchLength > 8 {
        ZSTD_wildcopy(
            op as *mut c_void,
            match_ as *const c_void,
            sequence.matchLength as isize - 8,
            ZSTD_overlap_src_before_dst,
        );
    }
    sequenceLength
}

unsafe fn ZSTD_initFseState(
    DStatePtr: *mut ZSTD_fseState,
    bitD: *mut BIT_DStream_t,
    dt: *const ZSTD_seqSymbol,
) {
    let ptr = dt as *const c_void;
    let DTableH = ptr as *const ZSTD_seqSymbol_header;
    (*DStatePtr).state = BIT_readBits(bitD, (*DTableH).tableLog);
    BIT_reloadDStream(bitD);
    (*DStatePtr).table = dt.add(1);
}

#[inline(always)]
unsafe fn ZSTD_updateFseStateWithDInfo(
    DStatePtr: *mut ZSTD_fseState,
    bitD: *mut BIT_DStream_t,
    nextState: U16,
    nbBits: U32,
) {
    let lowBits: usize = BIT_readBits(bitD, nbBits);
    (*DStatePtr).state = nextState as usize + lowBits;
}

/// `ZSTD_decodeSequence()`:
/// @p longOffsets : tells the decoder to reload more bit while decoding large offsets
///                  only used in 32-bit mode
/// @return : Sequence (litL + matchL + offset)
#[inline(always)]
unsafe fn ZSTD_decodeSequence(
    seqState: *mut seqState_t,
    longOffsets: ZSTD_longOffset_e,
    isLastSeq: c_int,
) -> seq_t {
    let mut seq = seq_t {
        litLength: 0,
        matchLength: 0,
        offset: 0,
    };
    let llDInfo: *const ZSTD_seqSymbol =
        (*seqState).stateLL.table.add((*seqState).stateLL.state);
    let mlDInfo: *const ZSTD_seqSymbol =
        (*seqState).stateML.table.add((*seqState).stateML.state);
    let ofDInfo: *const ZSTD_seqSymbol =
        (*seqState).stateOffb.table.add((*seqState).stateOffb.state);
    seq.matchLength = (*mlDInfo).baseValue as usize;
    seq.litLength = (*llDInfo).baseValue as usize;
    {
        let ofBase: U32 = (*ofDInfo).baseValue;
        let llBits: BYTE = (*llDInfo).nbAdditionalBits;
        let mlBits: BYTE = (*mlDInfo).nbAdditionalBits;
        let ofBits: BYTE = (*ofDInfo).nbAdditionalBits;
        let totalBits: BYTE = llBits.wrapping_add(mlBits).wrapping_add(ofBits);

        let llNext: U16 = (*llDInfo).nextState;
        let mlNext: U16 = (*mlDInfo).nextState;
        let ofNext: U16 = (*ofDInfo).nextState;
        let llnbBits: U32 = (*llDInfo).nbBits as U32;
        let mlnbBits: U32 = (*mlDInfo).nbBits as U32;
        let ofnbBits: U32 = (*ofDInfo).nbBits as U32;

        /* sequence */
        {
            let mut offset: usize;
            if ofBits > 1 {
                if MEM_32bits() != 0
                    && longOffsets != 0
                    && (ofBits as u32 >= STREAM_ACCUMULATOR_MIN_32)
                {
                    /* Always read extra bits, this keeps the logic simple,
                     * avoids branches, and avoids accidentally reading 0 bits.
                     */
                    let extraBits: U32 = LONG_OFFSETS_MAX_EXTRA_BITS_32;
                    offset = (ofBase as usize).wrapping_add(
                        BIT_readBitsFast(&mut (*seqState).DStream, ofBits as u32 - extraBits)
                            << extraBits,
                    );
                    BIT_reloadDStream(&mut (*seqState).DStream);
                    offset = offset
                        .wrapping_add(BIT_readBitsFast(&mut (*seqState).DStream, extraBits));
                } else {
                    /* <=  (ZSTD_WINDOWLOG_MAX-1) bits */
                    offset = (ofBase as usize).wrapping_add(BIT_readBitsFast(
                        &mut (*seqState).DStream,
                        ofBits as u32, /*>0*/
                    ));
                    if MEM_32bits() != 0 {
                        BIT_reloadDStream(&mut (*seqState).DStream);
                    }
                }
                (*seqState).prevOffset[2] = (*seqState).prevOffset[1];
                (*seqState).prevOffset[1] = (*seqState).prevOffset[0];
                (*seqState).prevOffset[0] = offset;
            } else {
                let ll0: U32 = ((*llDInfo).baseValue == 0) as U32;
                if ofBits == 0 {
                    offset = (*seqState).prevOffset[ll0 as usize];
                    (*seqState).prevOffset[1] = (*seqState).prevOffset[(ll0 == 0) as usize];
                    (*seqState).prevOffset[0] = offset;
                } else {
                    offset = (ofBase as usize)
                        .wrapping_add(ll0 as usize)
                        .wrapping_add(BIT_readBitsFast(&mut (*seqState).DStream, 1));
                    {
                        let mut temp: usize = if offset == 3 {
                            (*seqState).prevOffset[0].wrapping_sub(1)
                        } else {
                            (*seqState).prevOffset[offset]
                        };
                        /* 0 is not valid: input corrupted => force offset to -1 =>
                         * corruption detected at execSequence */
                        temp = temp.wrapping_sub((temp == 0) as usize);
                        if offset != 1 {
                            (*seqState).prevOffset[2] = (*seqState).prevOffset[1];
                        }
                        (*seqState).prevOffset[1] = (*seqState).prevOffset[0];
                        offset = temp;
                        (*seqState).prevOffset[0] = offset;
                    }
                }
            }
            seq.offset = offset;
        }

        if mlBits > 0 {
            seq.matchLength = seq.matchLength.wrapping_add(BIT_readBitsFast(
                &mut (*seqState).DStream,
                mlBits as u32, /*>0*/
            ));
        }

        if MEM_32bits() != 0
            && (mlBits as u32 + llBits as u32
                >= STREAM_ACCUMULATOR_MIN_32 - LONG_OFFSETS_MAX_EXTRA_BITS_32)
        {
            BIT_reloadDStream(&mut (*seqState).DStream);
        }
        if MEM_64bits() != 0
            && (totalBits as u32
                >= STREAM_ACCUMULATOR_MIN_64 - (LLFSELog + MLFSELog + OffFSELog))
        {
            BIT_reloadDStream(&mut (*seqState).DStream);
        }

        if llBits > 0 {
            seq.litLength = seq.litLength.wrapping_add(BIT_readBitsFast(
                &mut (*seqState).DStream,
                llBits as u32, /*>0*/
            ));
        }

        if MEM_32bits() != 0 {
            BIT_reloadDStream(&mut (*seqState).DStream);
        }

        if isLastSeq == 0 {
            /* don't update FSE state for last Sequence */
            ZSTD_updateFseStateWithDInfo(
                &mut (*seqState).stateLL,
                &mut (*seqState).DStream,
                llNext,
                llnbBits,
            ); /* <=  9 bits */
            ZSTD_updateFseStateWithDInfo(
                &mut (*seqState).stateML,
                &mut (*seqState).DStream,
                mlNext,
                mlnbBits,
            ); /* <=  9 bits */
            if MEM_32bits() != 0 {
                BIT_reloadDStream(&mut (*seqState).DStream); /* <= 18 bits */
            }
            ZSTD_updateFseStateWithDInfo(
                &mut (*seqState).stateOffb,
                &mut (*seqState).DStream,
                ofNext,
                ofnbBits,
            ); /* <=  8 bits */
            BIT_reloadDStream(&mut (*seqState).DStream);
        }
    }

    seq
}

#[inline(always)]
unsafe fn new_seqState() -> seqState_t {
    seqState_t {
        DStream: BIT_DStream_t::default(),
        stateLL: ZSTD_fseState {
            state: 0,
            table: core::ptr::null(),
        },
        stateOffb: ZSTD_fseState {
            state: 0,
            table: core::ptr::null(),
        },
        stateML: ZSTD_fseState {
            state: 0,
            table: core::ptr::null(),
        },
        prevOffset: [0; ZSTD_REP_NUM],
    }
}

#[inline(always)]
unsafe fn ZSTD_decompressSequences_bodySplitLitBuffer(
    dctx: *mut ZSTD_DCtx,
    dst: *mut c_void,
    maxDstSize: usize,
    seqStart: *const c_void,
    seqSize: usize,
    mut nbSeq: c_int,
    isLongOffset: ZSTD_longOffset_e,
) -> usize {
    let ip = seqStart as *const BYTE;
    let iend = ip.wrapping_add(seqSize);
    let ostart = dst as *mut BYTE;
    let oend = ZSTD_maybeNullPtrAdd(ostart, maxDstSize as isize);
    let mut op = ostart;
    let mut litPtr: *const BYTE = (*dctx).litPtr;
    let mut litBufferEnd: *const BYTE = (*dctx).litBufferEnd;
    let prefixStart = (*dctx).prefixStart as *const BYTE;
    let vBase = (*dctx).virtualStart as *const BYTE;
    let dictEnd = (*dctx).dictEnd as *const BYTE;

    /* Literals are split between internal buffer & output buffer */
    if nbSeq != 0 {
        let mut seqState = new_seqState();
        (*dctx).fseEntropy = 1;
        {
            let mut i: U32 = 0;
            while (i as usize) < ZSTD_REP_NUM {
                seqState.prevOffset[i as usize] = (*dctx).entropy.rep[i as usize] as usize;
                i += 1;
            }
        }
        if ERR_isError(BIT_initDStream(
            &mut seqState.DStream,
            ip as *const c_void,
            iend.offset_from(ip) as usize,
        )) != 0
        {
            return ERROR(ZSTD_error_corruption_detected);
        }
        ZSTD_initFseState(&mut seqState.stateLL, &mut seqState.DStream, (*dctx).LLTptr);
        ZSTD_initFseState(
            &mut seqState.stateOffb,
            &mut seqState.DStream,
            (*dctx).OFTptr,
        );
        ZSTD_initFseState(&mut seqState.stateML, &mut seqState.DStream, (*dctx).MLTptr);

        /* decompress without overrunning litPtr begins */
        {
            let mut sequence = seq_t {
                litLength: 0,
                matchLength: 0,
                offset: 0,
            };

            /* Handle the initial state where litBuffer is currently split between dst and
             * litExtraBuffer */
            while nbSeq != 0 {
                sequence = ZSTD_decodeSequence(
                    &mut seqState,
                    isLongOffset,
                    (nbSeq == 1) as c_int,
                );
                if litPtr.wrapping_add(sequence.litLength) > (*dctx).litBufferEnd {
                    break;
                }
                {
                    let oneSeqSize: usize = ZSTD_execSequenceSplitLitBuffer(
                        op,
                        oend,
                        litPtr
                            .wrapping_add(sequence.litLength)
                            .wrapping_offset(-WILDCOPY_OVERLENGTH),
                        sequence,
                        &mut litPtr,
                        litBufferEnd,
                        prefixStart,
                        vBase,
                        dictEnd,
                    );
                    if ERR_isError(oneSeqSize) != 0 {
                        return oneSeqSize;
                    }
                    op = op.wrapping_add(oneSeqSize);
                }
                nbSeq -= 1;
            }

            /* If there are more sequences, they will need to read literals from litExtraBuffer;
             * copy over the remainder from dst and update litPtr and litEnd */
            if nbSeq > 0 {
                let leftoverLit: usize = (*dctx).litBufferEnd.offset_from(litPtr) as usize;
                if leftoverLit != 0 {
                    if leftoverLit > oend.offset_from(op) as usize {
                        return ERROR(ZSTD_error_dstSize_tooSmall);
                    }
                    ZSTD_safecopyDstBeforeSrc(op, litPtr, leftoverLit as isize);
                    sequence.litLength -= leftoverLit;
                    op = op.wrapping_add(leftoverLit);
                }
                litPtr = (*dctx).litExtraBuffer.as_ptr();
                litBufferEnd = (*dctx)
                    .litExtraBuffer
                    .as_ptr()
                    .add(ZSTD_LITBUFFEREXTRASIZE);
                (*dctx).litBufferLocation = ZSTD_not_in_dst;
                {
                    let oneSeqSize: usize = ZSTD_execSequence(
                        op,
                        oend,
                        sequence,
                        &mut litPtr,
                        litBufferEnd,
                        prefixStart,
                        vBase,
                        dictEnd,
                    );
                    if ERR_isError(oneSeqSize) != 0 {
                        return oneSeqSize;
                    }
                    op = op.wrapping_add(oneSeqSize);
                }
                nbSeq -= 1;
            }
        }

        if nbSeq > 0 {
            /* there is remaining lit from extra buffer */
            while nbSeq != 0 {
                let sequence: seq_t = ZSTD_decodeSequence(
                    &mut seqState,
                    isLongOffset,
                    (nbSeq == 1) as c_int,
                );
                let oneSeqSize: usize = ZSTD_execSequence(
                    op,
                    oend,
                    sequence,
                    &mut litPtr,
                    litBufferEnd,
                    prefixStart,
                    vBase,
                    dictEnd,
                );
                if ERR_isError(oneSeqSize) != 0 {
                    return oneSeqSize;
                }
                op = op.wrapping_add(oneSeqSize);
                nbSeq -= 1;
            }
        }

        /* check if reached exact end */
        if nbSeq != 0 {
            return ERROR(ZSTD_error_corruption_detected);
        }
        if BIT_endOfDStream(&seqState.DStream) == 0 {
            return ERROR(ZSTD_error_corruption_detected);
        }
        /* save reps for next block */
        {
            let mut i: U32 = 0;
            while (i as usize) < ZSTD_REP_NUM {
                (*dctx).entropy.rep[i as usize] = seqState.prevOffset[i as usize] as U32;
                i += 1;
            }
        }
    }

    /* last literal segment */
    if (*dctx).litBufferLocation == ZSTD_split {
        /* split hasn't been reached yet, first get dst then copy litExtraBuffer */
        let lastLLSize: usize = litBufferEnd.offset_from(litPtr) as usize;
        if lastLLSize > oend.offset_from(op) as usize {
            return ERROR(ZSTD_error_dstSize_tooSmall);
        }
        if !op.is_null() {
            ZSTD_memmove(op as *mut c_void, litPtr as *const c_void, lastLLSize);
            op = op.wrapping_add(lastLLSize);
        }
        litPtr = (*dctx).litExtraBuffer.as_ptr();
        litBufferEnd = (*dctx)
            .litExtraBuffer
            .as_ptr()
            .add(ZSTD_LITBUFFEREXTRASIZE);
        (*dctx).litBufferLocation = ZSTD_not_in_dst;
    }
    /* copy last literals from internal buffer */
    {
        let lastLLSize: usize = litBufferEnd.offset_from(litPtr) as usize;
        if lastLLSize > oend.offset_from(op) as usize {
            return ERROR(ZSTD_error_dstSize_tooSmall);
        }
        if !op.is_null() {
            ZSTD_memcpy(op as *mut c_void, litPtr as *const c_void, lastLLSize);
            op = op.wrapping_add(lastLLSize);
        }
    }

    op.offset_from(ostart) as usize
}

#[inline(always)]
unsafe fn ZSTD_decompressSequences_body(
    dctx: *mut ZSTD_DCtx,
    dst: *mut c_void,
    maxDstSize: usize,
    seqStart: *const c_void,
    seqSize: usize,
    mut nbSeq: c_int,
    isLongOffset: ZSTD_longOffset_e,
) -> usize {
    let ip = seqStart as *const BYTE;
    let iend = ip.wrapping_add(seqSize);
    let ostart = dst as *mut BYTE;
    let oend: *mut BYTE = if (*dctx).litBufferLocation == ZSTD_not_in_dst {
        ZSTD_maybeNullPtrAdd(ostart, maxDstSize as isize)
    } else {
        (*dctx).litBuffer
    };
    let mut op = ostart;
    let mut litPtr: *const BYTE = (*dctx).litPtr;
    let litEnd: *const BYTE = litPtr.wrapping_add((*dctx).litSize);
    let prefixStart = (*dctx).prefixStart as *const BYTE;
    let vBase = (*dctx).virtualStart as *const BYTE;
    let dictEnd = (*dctx).dictEnd as *const BYTE;

    /* Regen sequences */
    if nbSeq != 0 {
        let mut seqState = new_seqState();
        (*dctx).fseEntropy = 1;
        {
            let mut i: U32 = 0;
            while (i as usize) < ZSTD_REP_NUM {
                seqState.prevOffset[i as usize] = (*dctx).entropy.rep[i as usize] as usize;
                i += 1;
            }
        }
        if ERR_isError(BIT_initDStream(
            &mut seqState.DStream,
            ip as *const c_void,
            iend.offset_from(ip) as usize,
        )) != 0
        {
            return ERROR(ZSTD_error_corruption_detected);
        }
        ZSTD_initFseState(&mut seqState.stateLL, &mut seqState.DStream, (*dctx).LLTptr);
        ZSTD_initFseState(
            &mut seqState.stateOffb,
            &mut seqState.DStream,
            (*dctx).OFTptr,
        );
        ZSTD_initFseState(&mut seqState.stateML, &mut seqState.DStream, (*dctx).MLTptr);

        while nbSeq != 0 {
            let sequence: seq_t =
                ZSTD_decodeSequence(&mut seqState, isLongOffset, (nbSeq == 1) as c_int);
            let oneSeqSize: usize = ZSTD_execSequence(
                op,
                oend,
                sequence,
                &mut litPtr,
                litEnd,
                prefixStart,
                vBase,
                dictEnd,
            );
            if ERR_isError(oneSeqSize) != 0 {
                return oneSeqSize;
            }
            op = op.wrapping_add(oneSeqSize);
            nbSeq -= 1;
        }

        /* check if reached exact end */
        if BIT_endOfDStream(&seqState.DStream) == 0 {
            return ERROR(ZSTD_error_corruption_detected);
        }
        /* save reps for next block */
        {
            let mut i: U32 = 0;
            while (i as usize) < ZSTD_REP_NUM {
                (*dctx).entropy.rep[i as usize] = seqState.prevOffset[i as usize] as U32;
                i += 1;
            }
        }
    }

    /* last literal segment */
    {
        let lastLLSize: usize = litEnd.offset_from(litPtr) as usize;
        if lastLLSize > oend.offset_from(op) as usize {
            return ERROR(ZSTD_error_dstSize_tooSmall);
        }
        if !op.is_null() {
            ZSTD_memcpy(op as *mut c_void, litPtr as *const c_void, lastLLSize);
            op = op.wrapping_add(lastLLSize);
        }
    }

    op.offset_from(ostart) as usize
}

unsafe fn ZSTD_decompressSequences_default(
    dctx: *mut ZSTD_DCtx,
    dst: *mut c_void,
    maxDstSize: usize,
    seqStart: *const c_void,
    seqSize: usize,
    nbSeq: c_int,
    isLongOffset: ZSTD_longOffset_e,
) -> usize {
    ZSTD_decompressSequences_body(
        dctx,
        dst,
        maxDstSize,
        seqStart,
        seqSize,
        nbSeq,
        isLongOffset,
    )
}

unsafe fn ZSTD_decompressSequencesSplitLitBuffer_default(
    dctx: *mut ZSTD_DCtx,
    dst: *mut c_void,
    maxDstSize: usize,
    seqStart: *const c_void,
    seqSize: usize,
    nbSeq: c_int,
    isLongOffset: ZSTD_longOffset_e,
) -> usize {
    ZSTD_decompressSequences_bodySplitLitBuffer(
        dctx,
        dst,
        maxDstSize,
        seqStart,
        seqSize,
        nbSeq,
        isLongOffset,
    )
}

#[inline(always)]
unsafe fn ZSTD_prefetchMatch(
    mut prefetchPos: usize,
    sequence: seq_t,
    prefixStart: *const BYTE,
    dictEnd: *const BYTE,
) -> usize {
    prefetchPos = prefetchPos.wrapping_add(sequence.litLength);
    {
        let matchBase: *const BYTE = if sequence.offset > prefetchPos {
            dictEnd
        } else {
            prefixStart
        };
        /* note : this operation can overflow when seq.offset is really too large, which can only
         * happen when input is corrupted. No consequence though : memory address is only used
         * for prefetching, not for dereferencing.
         * PREFETCH_L1() is a no-op in this port. */
        let match_: *const BYTE = ZSTD_wrappedPtrSub(
            ZSTD_wrappedPtrAdd(matchBase, prefetchPos as isize),
            sequence.offset as isize,
        );
        let _ = match_;
        let _ = CACHELINE_SIZE;
    }
    prefetchPos.wrapping_add(sequence.matchLength)
}

const STORED_SEQS: usize = 8;
const STORED_SEQS_MASK: c_int = (STORED_SEQS - 1) as c_int;
const ADVANCED_SEQS: c_int = STORED_SEQS as c_int;

/// This decoding function employs prefetching
/// to reduce latency impact of cache misses.
/// It's generally employed when block contains a significant portion of long-distance matches
/// or when coupled with a "cold" dictionary
#[inline(always)]
unsafe fn ZSTD_decompressSequencesLong_body(
    dctx: *mut ZSTD_DCtx,
    dst: *mut c_void,
    maxDstSize: usize,
    seqStart: *const c_void,
    seqSize: usize,
    nbSeq: c_int,
    isLongOffset: ZSTD_longOffset_e,
) -> usize {
    let ip = seqStart as *const BYTE;
    let iend = ip.wrapping_add(seqSize);
    let ostart = dst as *mut BYTE;
    let oend: *mut BYTE = if (*dctx).litBufferLocation == ZSTD_in_dst {
        (*dctx).litBuffer
    } else {
        ZSTD_maybeNullPtrAdd(ostart, maxDstSize as isize)
    };
    let mut op = ostart;
    let mut litPtr: *const BYTE = (*dctx).litPtr;
    let mut litBufferEnd: *const BYTE = (*dctx).litBufferEnd;
    let prefixStart = (*dctx).prefixStart as *const BYTE;
    let dictStart = (*dctx).virtualStart as *const BYTE;
    let dictEnd = (*dctx).dictEnd as *const BYTE;

    /* Regen sequences */
    if nbSeq != 0 {
        let mut sequences: [seq_t; STORED_SEQS] = [seq_t {
            litLength: 0,
            matchLength: 0,
            offset: 0,
        }; STORED_SEQS];
        let seqAdvance: c_int = MIN(nbSeq, ADVANCED_SEQS);
        let mut seqState = new_seqState();
        let mut seqNb: c_int;
        /* track position relative to prefixStart */
        let mut prefetchPos: usize = op.offset_from(prefixStart) as usize;

        (*dctx).fseEntropy = 1;
        {
            let mut i: c_int = 0;
            while (i as usize) < ZSTD_REP_NUM {
                seqState.prevOffset[i as usize] = (*dctx).entropy.rep[i as usize] as usize;
                i += 1;
            }
        }
        if ERR_isError(BIT_initDStream(
            &mut seqState.DStream,
            ip as *const c_void,
            iend.offset_from(ip) as usize,
        )) != 0
        {
            return ERROR(ZSTD_error_corruption_detected);
        }
        ZSTD_initFseState(&mut seqState.stateLL, &mut seqState.DStream, (*dctx).LLTptr);
        ZSTD_initFseState(
            &mut seqState.stateOffb,
            &mut seqState.DStream,
            (*dctx).OFTptr,
        );
        ZSTD_initFseState(&mut seqState.stateML, &mut seqState.DStream, (*dctx).MLTptr);

        /* prepare in advance */
        seqNb = 0;
        while seqNb < seqAdvance {
            let sequence: seq_t = ZSTD_decodeSequence(
                &mut seqState,
                isLongOffset,
                (seqNb == nbSeq - 1) as c_int,
            );
            prefetchPos = ZSTD_prefetchMatch(prefetchPos, sequence, prefixStart, dictEnd);
            sequences[seqNb as usize] = sequence;
            seqNb += 1;
        }

        /* decompress without stomping litBuffer */
        while seqNb < nbSeq {
            let sequence: seq_t = ZSTD_decodeSequence(
                &mut seqState,
                isLongOffset,
                (seqNb == nbSeq - 1) as c_int,
            );
            let prev: usize = ((seqNb - ADVANCED_SEQS) & STORED_SEQS_MASK) as usize;

            if (*dctx).litBufferLocation == ZSTD_split
                && litPtr.wrapping_add(sequences[prev].litLength) > (*dctx).litBufferEnd
            {
                /* lit buffer is reaching split point, empty out the first buffer and transition
                 * to litExtraBuffer */
                let leftoverLit: usize = (*dctx).litBufferEnd.offset_from(litPtr) as usize;
                if leftoverLit != 0 {
                    if leftoverLit > oend.offset_from(op) as usize {
                        return ERROR(ZSTD_error_dstSize_tooSmall);
                    }
                    ZSTD_safecopyDstBeforeSrc(op, litPtr, leftoverLit as isize);
                    sequences[prev].litLength -= leftoverLit;
                    op = op.wrapping_add(leftoverLit);
                }
                litPtr = (*dctx).litExtraBuffer.as_ptr();
                litBufferEnd = (*dctx)
                    .litExtraBuffer
                    .as_ptr()
                    .add(ZSTD_LITBUFFEREXTRASIZE);
                (*dctx).litBufferLocation = ZSTD_not_in_dst;
                {
                    let oneSeqSize: usize = ZSTD_execSequence(
                        op,
                        oend,
                        sequences[prev],
                        &mut litPtr,
                        litBufferEnd,
                        prefixStart,
                        dictStart,
                        dictEnd,
                    );
                    if ERR_isError(oneSeqSize) != 0 {
                        return oneSeqSize;
                    }

                    prefetchPos =
                        ZSTD_prefetchMatch(prefetchPos, sequence, prefixStart, dictEnd);
                    sequences[(seqNb & STORED_SEQS_MASK) as usize] = sequence;
                    op = op.wrapping_add(oneSeqSize);
                }
            } else {
                /* lit buffer is either wholly contained in first or second split, or not split
                 * at all */
                let oneSeqSize: usize = if (*dctx).litBufferLocation == ZSTD_split {
                    ZSTD_execSequenceSplitLitBuffer(
                        op,
                        oend,
                        litPtr
                            .wrapping_add(sequences[prev].litLength)
                            .wrapping_offset(-WILDCOPY_OVERLENGTH),
                        sequences[prev],
                        &mut litPtr,
                        litBufferEnd,
                        prefixStart,
                        dictStart,
                        dictEnd,
                    )
                } else {
                    ZSTD_execSequence(
                        op,
                        oend,
                        sequences[prev],
                        &mut litPtr,
                        litBufferEnd,
                        prefixStart,
                        dictStart,
                        dictEnd,
                    )
                };
                if ERR_isError(oneSeqSize) != 0 {
                    return oneSeqSize;
                }

                prefetchPos = ZSTD_prefetchMatch(prefetchPos, sequence, prefixStart, dictEnd);
                sequences[(seqNb & STORED_SEQS_MASK) as usize] = sequence;
                op = op.wrapping_add(oneSeqSize);
            }
            seqNb += 1;
        }
        if BIT_endOfDStream(&seqState.DStream) == 0 {
            return ERROR(ZSTD_error_corruption_detected);
        }

        /* finish queue */
        seqNb -= seqAdvance;
        while seqNb < nbSeq {
            let sequence: *mut seq_t = &mut sequences[(seqNb & STORED_SEQS_MASK) as usize];
            if (*dctx).litBufferLocation == ZSTD_split
                && litPtr.wrapping_add((*sequence).litLength) > (*dctx).litBufferEnd
            {
                let leftoverLit: usize = (*dctx).litBufferEnd.offset_from(litPtr) as usize;
                if leftoverLit != 0 {
                    if leftoverLit > oend.offset_from(op) as usize {
                        return ERROR(ZSTD_error_dstSize_tooSmall);
                    }
                    ZSTD_safecopyDstBeforeSrc(op, litPtr, leftoverLit as isize);
                    (*sequence).litLength -= leftoverLit;
                    op = op.wrapping_add(leftoverLit);
                }
                litPtr = (*dctx).litExtraBuffer.as_ptr();
                litBufferEnd = (*dctx)
                    .litExtraBuffer
                    .as_ptr()
                    .add(ZSTD_LITBUFFEREXTRASIZE);
                (*dctx).litBufferLocation = ZSTD_not_in_dst;
                {
                    let oneSeqSize: usize = ZSTD_execSequence(
                        op,
                        oend,
                        *sequence,
                        &mut litPtr,
                        litBufferEnd,
                        prefixStart,
                        dictStart,
                        dictEnd,
                    );
                    if ERR_isError(oneSeqSize) != 0 {
                        return oneSeqSize;
                    }
                    op = op.wrapping_add(oneSeqSize);
                }
            } else {
                let oneSeqSize: usize = if (*dctx).litBufferLocation == ZSTD_split {
                    ZSTD_execSequenceSplitLitBuffer(
                        op,
                        oend,
                        litPtr
                            .wrapping_add((*sequence).litLength)
                            .wrapping_offset(-WILDCOPY_OVERLENGTH),
                        *sequence,
                        &mut litPtr,
                        litBufferEnd,
                        prefixStart,
                        dictStart,
                        dictEnd,
                    )
                } else {
                    ZSTD_execSequence(
                        op,
                        oend,
                        *sequence,
                        &mut litPtr,
                        litBufferEnd,
                        prefixStart,
                        dictStart,
                        dictEnd,
                    )
                };
                if ERR_isError(oneSeqSize) != 0 {
                    return oneSeqSize;
                }
                op = op.wrapping_add(oneSeqSize);
            }
            seqNb += 1;
        }

        /* save reps for next block */
        {
            let mut i: U32 = 0;
            while (i as usize) < ZSTD_REP_NUM {
                (*dctx).entropy.rep[i as usize] = seqState.prevOffset[i as usize] as U32;
                i += 1;
            }
        }
    }

    /* last literal segment */
    if (*dctx).litBufferLocation == ZSTD_split {
        /* first deplete literal buffer in dst, then copy litExtraBuffer */
        let lastLLSize: usize = litBufferEnd.offset_from(litPtr) as usize;
        if lastLLSize > oend.offset_from(op) as usize {
            return ERROR(ZSTD_error_dstSize_tooSmall);
        }
        if !op.is_null() {
            ZSTD_memmove(op as *mut c_void, litPtr as *const c_void, lastLLSize);
            op = op.wrapping_add(lastLLSize);
        }
        litPtr = (*dctx).litExtraBuffer.as_ptr();
        litBufferEnd = (*dctx)
            .litExtraBuffer
            .as_ptr()
            .add(ZSTD_LITBUFFEREXTRASIZE);
    }
    {
        let lastLLSize: usize = litBufferEnd.offset_from(litPtr) as usize;
        if lastLLSize > oend.offset_from(op) as usize {
            return ERROR(ZSTD_error_dstSize_tooSmall);
        }
        if !op.is_null() {
            ZSTD_memmove(op as *mut c_void, litPtr as *const c_void, lastLLSize);
            op = op.wrapping_add(lastLLSize);
        }
    }

    op.offset_from(ostart) as usize
}

unsafe fn ZSTD_decompressSequencesLong_default(
    dctx: *mut ZSTD_DCtx,
    dst: *mut c_void,
    maxDstSize: usize,
    seqStart: *const c_void,
    seqSize: usize,
    nbSeq: c_int,
    isLongOffset: ZSTD_longOffset_e,
) -> usize {
    ZSTD_decompressSequencesLong_body(
        dctx,
        dst,
        maxDstSize,
        seqStart,
        seqSize,
        nbSeq,
        isLongOffset,
    )
}

unsafe fn ZSTD_decompressSequences(
    dctx: *mut ZSTD_DCtx,
    dst: *mut c_void,
    maxDstSize: usize,
    seqStart: *const c_void,
    seqSize: usize,
    nbSeq: c_int,
    isLongOffset: ZSTD_longOffset_e,
) -> usize {
    /* DYNAMIC_BMI2 == 0 : always the default variant */
    ZSTD_decompressSequences_default(
        dctx,
        dst,
        maxDstSize,
        seqStart,
        seqSize,
        nbSeq,
        isLongOffset,
    )
}

unsafe fn ZSTD_decompressSequencesSplitLitBuffer(
    dctx: *mut ZSTD_DCtx,
    dst: *mut c_void,
    maxDstSize: usize,
    seqStart: *const c_void,
    seqSize: usize,
    nbSeq: c_int,
    isLongOffset: ZSTD_longOffset_e,
) -> usize {
    /* DYNAMIC_BMI2 == 0 : always the default variant */
    ZSTD_decompressSequencesSplitLitBuffer_default(
        dctx,
        dst,
        maxDstSize,
        seqStart,
        seqSize,
        nbSeq,
        isLongOffset,
    )
}

/// `ZSTD_decompressSequencesLong()` :
/// decompression function triggered when a minimum share of offsets is considered "long",
/// aka out of cache.
unsafe fn ZSTD_decompressSequencesLong(
    dctx: *mut ZSTD_DCtx,
    dst: *mut c_void,
    maxDstSize: usize,
    seqStart: *const c_void,
    seqSize: usize,
    nbSeq: c_int,
    isLongOffset: ZSTD_longOffset_e,
) -> usize {
    /* DYNAMIC_BMI2 == 0 : always the default variant */
    ZSTD_decompressSequencesLong_default(
        dctx,
        dst,
        maxDstSize,
        seqStart,
        seqSize,
        nbSeq,
        isLongOffset,
    )
}

/// @returns The total size of the history referenceable by zstd, including
/// both the prefix and the extDict. At @p op any offset larger than this
/// is invalid.
unsafe fn ZSTD_totalHistorySize(op: *mut BYTE, virtualStart: *const BYTE) -> usize {
    op.offset_from(virtualStart) as usize
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
struct ZSTD_OffsetInfo {
    longOffsetShare: c_uint,
    maxNbAdditionalBits: c_uint,
}

/// `ZSTD_getOffsetInfo()` :
/// condition : offTable must be valid
/// @return : "share" of long offsets (arbitrarily defined as > (1<<23))
///           compared to maximum possible of (1<<OffFSELog),
///           as well as the maximum number additional bits required.
unsafe fn ZSTD_getOffsetInfo(offTable: *const ZSTD_seqSymbol, nbSeq: c_int) -> ZSTD_OffsetInfo {
    let mut info = ZSTD_OffsetInfo {
        longOffsetShare: 0,
        maxNbAdditionalBits: 0,
    };
    /* If nbSeq == 0, then the offTable is uninitialized, but we have
     * no sequences, so both values should be 0.
     */
    if nbSeq != 0 {
        let ptr = offTable as *const c_void;
        let tableLog: U32 = (*(ptr as *const ZSTD_seqSymbol_header).add(0)).tableLog;
        let table: *const ZSTD_seqSymbol = offTable.add(1);
        let max: U32 = 1u32 << tableLog;
        let mut u: U32 = 0;

        while u < max {
            info.maxNbAdditionalBits = MAX(
                info.maxNbAdditionalBits,
                (*table.add(u as usize)).nbAdditionalBits as c_uint,
            );
            if (*table.add(u as usize)).nbAdditionalBits > 22 {
                info.longOffsetShare += 1;
            }
            u += 1;
        }

        /* scale to OffFSELog */
        info.longOffsetShare <<= OffFSELog - tableLog;
    }

    info
}

/// @returns The maximum offset we can decode in one read of our bitstream, without
/// reloading more bits in the middle of the offset bits read. Any offsets larger
/// than this must use the long offset decoder.
unsafe fn ZSTD_maxShortOffset() -> usize {
    if MEM_64bits() != 0 {
        /* We can decode any offset without reloading bits.
         * This might change if the max window size grows.
         */
        (-1isize) as usize
    } else {
        /* The maximum offBase is (1 << (STREAM_ACCUMULATOR_MIN + 1)) - 1.
         * This offBase would require STREAM_ACCUMULATOR_MIN extra bits.
         * Then we have to subtract ZSTD_REP_NUM to get the maximum possible offset.
         */
        let maxOffbase: usize = (1usize << (STREAM_ACCUMULATOR_MIN() + 1)) - 1;
        let maxOffset: usize = maxOffbase - ZSTD_REP_NUM;
        maxOffset
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_decompressBlock_internal(
    dctx: *mut ZSTD_DCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    mut srcSize: usize,
    streaming: streaming_operation,
) -> usize {
    /* blockType == blockCompressed */
    let mut ip = src as *const BYTE;

    /* Note : the wording of the specification
     * allows compressed block to be sized exactly ZSTD_blockSizeMax(dctx).
     */
    if srcSize > ZSTD_blockSizeMax(dctx) {
        return ERROR(ZSTD_error_srcSize_wrong);
    }

    /* Decode literals section */
    {
        let litCSize: usize =
            ZSTD_decodeLiteralsBlock(dctx, src, srcSize, dst, dstCapacity, streaming);
        if ERR_isError(litCSize) != 0 {
            return litCSize;
        }
        ip = ip.add(litCSize);
        srcSize -= litCSize;
    }

    /* Build Decoding Tables */
    {
        /* Compute the maximum block size, which must also work when !frame and fParams are unset.
         * Additionally, take the min with dstCapacity to ensure that the totalHistorySize fits in
         * a size_t.
         */
        let blockSizeMax: usize = MIN(dstCapacity, ZSTD_blockSizeMax(dctx));
        let totalHistorySize: usize = ZSTD_totalHistorySize(
            ZSTD_maybeNullPtrAdd(dst as *mut BYTE, blockSizeMax as isize),
            (*dctx).virtualStart as *const BYTE,
        );
        /* isLongOffset must be true if there are long offsets.
         * Offsets are long if they are larger than ZSTD_maxShortOffset().
         */
        let mut isLongOffset: ZSTD_longOffset_e =
            (MEM_32bits() != 0 && (totalHistorySize > ZSTD_maxShortOffset())) as ZSTD_longOffset_e;
        let mut usePrefetchDecoder: c_int = (*dctx).ddictIsCold;
        let mut nbSeq: c_int = 0;
        let seqHSize: usize = ZSTD_decodeSeqHeaders(dctx, &mut nbSeq, ip as *const c_void, srcSize);
        if ERR_isError(seqHSize) != 0 {
            return seqHSize;
        }
        ip = ip.add(seqHSize);
        srcSize -= seqHSize;

        if (dst.is_null() || dstCapacity == 0) && nbSeq > 0 {
            return ERROR(ZSTD_error_dstSize_tooSmall);
        }
        if MEM_64bits() != 0
            && core::mem::size_of::<usize>() == core::mem::size_of::<*const c_void>()
            && (usize::MAX - (dst as usize)) < (1usize << 20)
        {
            return ERROR(ZSTD_error_dstSize_tooSmall);
        }

        /* If we could potentially have long offsets, or we might want to use the prefetch decoder,
         * compute information about the share of long offsets, and the maximum nbAdditionalBits.
         * NOTE: could probably use a larger nbSeq limit
         */
        if isLongOffset != 0
            || (usePrefetchDecoder == 0 && (totalHistorySize > (1usize << 24)) && (nbSeq > 8))
        {
            let info: ZSTD_OffsetInfo = ZSTD_getOffsetInfo((*dctx).OFTptr, nbSeq);
            if isLongOffset != 0 && info.maxNbAdditionalBits <= STREAM_ACCUMULATOR_MIN() {
                /* If isLongOffset, but the maximum number of additional bits that we see in our
                 * table is small enough, then we know it is impossible to have too long an offset
                 * in this block, so we can use the regular offset decoder.
                 */
                isLongOffset = ZSTD_lo_isRegularOffset;
            }
            if usePrefetchDecoder == 0 {
                /* heuristic values, correspond to 2.73% and 7.81% */
                let minShare: U32 = if MEM_64bits() != 0 { 7 } else { 20 };
                usePrefetchDecoder = (info.longOffsetShare >= minShare) as c_int;
            }
        }

        (*dctx).ddictIsCold = 0;

        if usePrefetchDecoder != 0 {
            return ZSTD_decompressSequencesLong(
                dctx,
                dst,
                dstCapacity,
                ip as *const c_void,
                srcSize,
                nbSeq,
                isLongOffset,
            );
        }

        /* else */
        if (*dctx).litBufferLocation == ZSTD_split {
            ZSTD_decompressSequencesSplitLitBuffer(
                dctx,
                dst,
                dstCapacity,
                ip as *const c_void,
                srcSize,
                nbSeq,
                isLongOffset,
            )
        } else {
            ZSTD_decompressSequences(
                dctx,
                dst,
                dstCapacity,
                ip as *const c_void,
                srcSize,
                nbSeq,
                isLongOffset,
            )
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_checkContinuity(
    dctx: *mut ZSTD_DCtx,
    dst: *const c_void,
    dstSize: usize,
) {
    if dst != (*dctx).previousDstEnd && dstSize > 0 {
        /* not contiguous */
        (*dctx).dictEnd = (*dctx).previousDstEnd;
        (*dctx).virtualStart = (dst as *const c_char).wrapping_offset(
            -((*dctx).previousDstEnd as *const c_char)
                .offset_from((*dctx).prefixStart as *const c_char),
        ) as *const c_void;
        (*dctx).prefixStart = dst;
        (*dctx).previousDstEnd = dst;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_decompressBlock_deprecated(
    dctx: *mut ZSTD_DCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let dSize: usize;
    (*dctx).isFrameDecompression = 0;
    ZSTD_checkContinuity(dctx, dst as *const c_void, dstCapacity);
    dSize = ZSTD_decompressBlock_internal(dctx, dst, dstCapacity, src, srcSize, not_streaming);
    if ERR_isError(dSize) != 0 {
        return dSize;
    }
    (*dctx).previousDstEnd = (dst as *mut c_char).wrapping_add(dSize) as *const c_void;
    dSize
}

/* NOTE: Must just wrap ZSTD_decompressBlock_deprecated() */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_decompressBlock(
    dctx: *mut ZSTD_DCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    ZSTD_decompressBlock_deprecated(dctx, dst, dstCapacity, src, srcSize)
}
