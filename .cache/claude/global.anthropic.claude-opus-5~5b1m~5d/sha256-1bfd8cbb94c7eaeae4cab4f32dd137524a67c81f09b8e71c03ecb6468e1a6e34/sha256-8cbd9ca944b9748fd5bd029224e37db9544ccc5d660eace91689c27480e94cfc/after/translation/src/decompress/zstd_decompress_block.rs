//! Translation of `decompress/zstd_decompress_block.c` (+ `decompress/zstd_decompress_block.h`)
#![allow(dead_code)]

use core::ffi::{c_char, c_int, c_uint, c_void};

use crate::bits::ZSTD_highbit32;
use crate::bitstream::*;
use crate::cmem::*;
use crate::error_private::*;
use crate::fse::FSE_TABLESTEP;
use crate::huf::*;
use crate::zstd_common::ZSTD_isError;
use crate::zstd_h::*;
use crate::zstd_internal::*;

use crate::entropy_common::{FSE_isError, FSE_readNCount, HUF_isError};

use crate::decompress::huf_decompress::{
    HUF_decompress1X1_DCtx_wksp, HUF_decompress1X_usingDTable, HUF_decompress4X_hufOnly_wksp,
    HUF_decompress4X_usingDTable,
};

use crate::decompress::zstd_decompress_internal::*;

/* ===================================================================== */
/* `zstd_decompress_block.h`                                             */
/* ===================================================================== */

/* Streaming state is used to inform allocation of the literal buffer */
pub type streaming_operation = c_uint;
pub const not_streaming: streaming_operation = 0;
pub const is_streaming: streaming_operation = 1;

/* ===================================================================== */
/* `common/compiler.h` helpers (MEM_STATIC -> module local)              */
/* ===================================================================== */

/// `ZSTD_wrappedPtrDiff()` : `lhs - rhs` with wrapping.
#[inline(always)]
unsafe fn ZSTD_wrappedPtrDiff(lhs: *const BYTE, rhs: *const BYTE) -> isize {
    (lhs as isize).wrapping_sub(rhs as isize)
}

/// `ZSTD_wrappedPtrAdd()` : `ptr + add` with wrapping.
#[inline(always)]
unsafe fn ZSTD_wrappedPtrAdd(ptr: *const BYTE, add: isize) -> *const BYTE {
    (ptr as isize).wrapping_add(add) as *const BYTE
}

/// `ZSTD_wrappedPtrSub()` : `ptr - sub` with wrapping.
#[inline(always)]
unsafe fn ZSTD_wrappedPtrSub(ptr: *const BYTE, sub: isize) -> *const BYTE {
    (ptr as isize).wrapping_sub(sub) as *const BYTE
}

/// `ZSTD_maybeNullPtrAdd()` : `ptr + add` except `NULL + 0 == NULL`.
#[inline(always)]
unsafe fn ZSTD_maybeNullPtrAdd(ptr: *mut BYTE, add: isize) -> *mut BYTE {
    if add > 0 {
        (ptr as isize).wrapping_add(add) as *mut BYTE
    } else {
        ptr
    }
}

/*-*******************************************************
 *  Memory operations
 **********************************************************/
unsafe fn ZSTD_copy4(dst: *mut c_void, src: *const c_void) {
    ZSTD_memcpy(dst, src, 4);
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

/* ZSTD_getcBlockSize() :
 *  Provides the size of compressed block from block header `src` */
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
            > blockSizeMax
                .wrapping_add(WILDCOPY_OVERLENGTH)
                .wrapping_add(litSize)
                .wrapping_add(WILDCOPY_OVERLENGTH)
    {
        /* If we aren't streaming, we can just put the literals after the output
         * of the current block. */
        (*dctx).litBuffer = (dst as usize)
            .wrapping_add(blockSizeMax)
            .wrapping_add(WILDCOPY_OVERLENGTH) as *mut BYTE;
        (*dctx).litBufferEnd = ((*dctx).litBuffer as usize).wrapping_add(litSize) as *const BYTE;
        (*dctx).litBufferLocation = ZSTD_in_dst;
    } else if litSize <= ZSTD_LITBUFFEREXTRASIZE {
        /* Literals fit entirely within the extra buffer */
        (*dctx).litBuffer = core::ptr::addr_of_mut!((*dctx).litExtraBuffer) as *mut BYTE;
        (*dctx).litBufferEnd = ((*dctx).litBuffer as usize).wrapping_add(litSize) as *const BYTE;
        (*dctx).litBufferLocation = ZSTD_not_in_dst;
    } else {
        /* Literals must be split between the output block and the extra lit buffer. */
        if splitImmediately != 0 {
            /* won't fit in litExtraBuffer, so it will be split between end of dst and extra buffer */
            (*dctx).litBuffer = (dst as usize)
                .wrapping_add(expectedWriteSize)
                .wrapping_sub(litSize)
                .wrapping_add(ZSTD_LITBUFFEREXTRASIZE)
                .wrapping_sub(WILDCOPY_OVERLENGTH) as *mut BYTE;
            (*dctx).litBufferEnd = ((*dctx).litBuffer as usize)
                .wrapping_add(litSize)
                .wrapping_sub(ZSTD_LITBUFFEREXTRASIZE) as *const BYTE;
        } else {
            /* initially this will be stored entirely in dst during huffman decoding,
             * it will partially be shifted to litExtraBuffer after */
            (*dctx).litBuffer = (dst as usize)
                .wrapping_add(expectedWriteSize)
                .wrapping_sub(litSize) as *mut BYTE;
            (*dctx).litBufferEnd =
                (dst as usize).wrapping_add(expectedWriteSize) as *const BYTE;
        }
        (*dctx).litBufferLocation = ZSTD_split;
    }
}

/* ZSTD_decodeLiteralsBlock() :
 * @return : nb of bytes read from src (< srcSize ) */
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
        let istart: *const BYTE = src as *const BYTE;
        let litEncType: SymbolEncodingType_e = (*istart & 3) as SymbolEncodingType_e;
        let blockSizeMax: usize = ZSTD_blockSizeMax(dctx);

        if litEncType == set_repeat || litEncType == set_compressed {
            if litEncType == set_repeat {
                /* re-using stats from previous compressed literals block */
                if (*dctx).litEntropy == 0 {
                    return ERROR(ZSTD_error_dictionary_corrupted);
                }
                /* ZSTD_FALLTHROUGH */
            }

            /* case set_compressed: */
            if srcSize < 5 {
                return ERROR(ZSTD_error_corruption_detected);
            }
            {
                let lhSize: usize;
                let litSize: usize;
                let litCSize: usize;
                let mut singleStream: U32 = 0;
                let lhlCode: U32 = (((*istart) >> 2) & 3) as U32;
                let lhc: U32 = MEM_readLE32(istart as *const c_void);
                let hufSuccess: usize;
                let expectedWriteSize: usize = if blockSizeMax < dstCapacity {
                    blockSizeMax
                } else {
                    dstCapacity
                };
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
                        litCSize =
                            ((lhc >> 22) as usize).wrapping_add((*istart.add(4) as usize) << 10);
                    }
                    _ => {
                        /* case 0, case 1, default : 2 - 2 - 10 - 10 */
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
                if litCSize.wrapping_add(lhSize) > srcSize {
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

                /* prefetch huffman table if cold : PREFETCH_AREA() is a no-op here */

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
                            core::ptr::addr_of_mut!((*dctx).entropy.hufTable) as *mut HUF_DTable,
                            (*dctx).litBuffer as *mut c_void,
                            litSize,
                            istart.add(lhSize) as *const c_void,
                            litCSize,
                            core::ptr::addr_of_mut!((*dctx).workspace) as *mut c_void,
                            core::mem::size_of::<[U32; HUF_DECOMPRESS_WORKSPACE_SIZE_U32]>(),
                            flags,
                        );
                    } else {
                        hufSuccess = HUF_decompress4X_hufOnly_wksp(
                            core::ptr::addr_of_mut!((*dctx).entropy.hufTable) as *mut HUF_DTable,
                            (*dctx).litBuffer as *mut c_void,
                            litSize,
                            istart.add(lhSize) as *const c_void,
                            litCSize,
                            core::ptr::addr_of_mut!((*dctx).workspace) as *mut c_void,
                            core::mem::size_of::<[U32; HUF_DECOMPRESS_WORKSPACE_SIZE_U32]>(),
                            flags,
                        );
                    }
                }
                if (*dctx).litBufferLocation == ZSTD_split {
                    ZSTD_memcpy(
                        core::ptr::addr_of_mut!((*dctx).litExtraBuffer) as *mut c_void,
                        ((*dctx).litBufferEnd as usize).wrapping_sub(ZSTD_LITBUFFEREXTRASIZE)
                            as *const c_void,
                        ZSTD_LITBUFFEREXTRASIZE,
                    );
                    ZSTD_memmove(
                        ((*dctx).litBuffer as usize)
                            .wrapping_add(ZSTD_LITBUFFEREXTRASIZE)
                            .wrapping_sub(WILDCOPY_OVERLENGTH) as *mut c_void,
                        (*dctx).litBuffer as *const c_void,
                        litSize.wrapping_sub(ZSTD_LITBUFFEREXTRASIZE),
                    );
                    (*dctx).litBuffer = ((*dctx).litBuffer as usize)
                        .wrapping_add(ZSTD_LITBUFFEREXTRASIZE)
                        .wrapping_sub(WILDCOPY_OVERLENGTH) as *mut BYTE;
                    (*dctx).litBufferEnd =
                        ((*dctx).litBufferEnd as usize).wrapping_sub(WILDCOPY_OVERLENGTH)
                            as *const BYTE;
                }

                if HUF_isError(hufSuccess) != 0 {
                    return ERROR(ZSTD_error_corruption_detected);
                }

                (*dctx).litPtr = (*dctx).litBuffer;
                (*dctx).litSize = litSize;
                (*dctx).litEntropy = 1;
                if litEncType == set_compressed {
                    (*dctx).HUFptr =
                        core::ptr::addr_of!((*dctx).entropy.hufTable) as *const HUF_DTable;
                }
                return litCSize.wrapping_add(lhSize);
            }
        } else if litEncType == set_basic {
            {
                let litSize: usize;
                let lhSize: usize;
                let lhlCode: U32 = (((*istart) >> 2) & 3) as U32;
                let expectedWriteSize: usize = if blockSizeMax < dstCapacity {
                    blockSizeMax
                } else {
                    dstCapacity
                };
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
                        /* case 0, case 2, default */
                        lhSize = 1;
                        litSize = ((*istart) >> 3) as usize;
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
                if lhSize
                    .wrapping_add(litSize)
                    .wrapping_add(WILDCOPY_OVERLENGTH)
                    > srcSize
                {
                    /* risk reading beyond src buffer with wildcopy */
                    if litSize.wrapping_add(lhSize) > srcSize {
                        return ERROR(ZSTD_error_corruption_detected);
                    }
                    if (*dctx).litBufferLocation == ZSTD_split {
                        ZSTD_memcpy(
                            (*dctx).litBuffer as *mut c_void,
                            istart.add(lhSize) as *const c_void,
                            litSize.wrapping_sub(ZSTD_LITBUFFEREXTRASIZE),
                        );
                        ZSTD_memcpy(
                            core::ptr::addr_of_mut!((*dctx).litExtraBuffer) as *mut c_void,
                            istart.add(
                                lhSize
                                    .wrapping_add(litSize)
                                    .wrapping_sub(ZSTD_LITBUFFEREXTRASIZE),
                            ) as *const c_void,
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
                    return lhSize.wrapping_add(litSize);
                }
                /* direct reference into compressed stream */
                (*dctx).litPtr = istart.add(lhSize);
                (*dctx).litSize = litSize;
                (*dctx).litBufferEnd = ((*dctx).litPtr as usize).wrapping_add(litSize) as *const BYTE;
                (*dctx).litBufferLocation = ZSTD_not_in_dst;
                return lhSize.wrapping_add(litSize);
            }
        } else if litEncType == set_rle {
            {
                let lhlCode: U32 = (((*istart) >> 2) & 3) as U32;
                let litSize: usize;
                let lhSize: usize;
                let expectedWriteSize: usize = if blockSizeMax < dstCapacity {
                    blockSizeMax
                } else {
                    dstCapacity
                };
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
                        /* case 0, case 2, default */
                        lhSize = 1;
                        litSize = ((*istart) >> 3) as usize;
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
                        litSize.wrapping_sub(ZSTD_LITBUFFEREXTRASIZE),
                    );
                    ZSTD_memset(
                        core::ptr::addr_of_mut!((*dctx).litExtraBuffer) as *mut c_void,
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
                return lhSize.wrapping_add(1);
            }
        } else {
            return ERROR(ZSTD_error_corruption_detected);
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

/* Default FSE distribution tables. */

#[inline(always)]
const fn SS(nextState: U16, nbAdditionalBits: BYTE, nbBits: BYTE, baseValue: U32) -> ZSTD_seqSymbol {
    ZSTD_seqSymbol {
        nextState,
        nbAdditionalBits,
        nbBits,
        baseValue,
    }
}

/* Default FSE distribution table for Literal Lengths */
static LL_defaultDTable: [ZSTD_seqSymbol; (1 << LL_DEFAULTNORMLOG) + 1] = [
    SS(1, 1, 1, LL_DEFAULTNORMLOG), /* header : fastMode, tableLog */
    /* nextState, nbAddBits, nbBits, baseVal */
    SS(0, 0, 4, 0),
    SS(16, 0, 4, 0),
    SS(32, 0, 5, 1),
    SS(0, 0, 5, 3),
    SS(0, 0, 5, 4),
    SS(0, 0, 5, 6),
    SS(0, 0, 5, 7),
    SS(0, 0, 5, 9),
    SS(0, 0, 5, 10),
    SS(0, 0, 5, 12),
    SS(0, 0, 6, 14),
    SS(0, 1, 5, 16),
    SS(0, 1, 5, 20),
    SS(0, 1, 5, 22),
    SS(0, 2, 5, 28),
    SS(0, 3, 5, 32),
    SS(0, 4, 5, 48),
    SS(32, 6, 5, 64),
    SS(0, 7, 5, 128),
    SS(0, 8, 6, 256),
    SS(0, 10, 6, 1024),
    SS(0, 12, 6, 4096),
    SS(32, 0, 4, 0),
    SS(0, 0, 4, 1),
    SS(0, 0, 5, 2),
    SS(32, 0, 5, 4),
    SS(0, 0, 5, 5),
    SS(32, 0, 5, 7),
    SS(0, 0, 5, 8),
    SS(32, 0, 5, 10),
    SS(0, 0, 5, 11),
    SS(0, 0, 6, 13),
    SS(32, 1, 5, 16),
    SS(0, 1, 5, 18),
    SS(32, 1, 5, 22),
    SS(0, 2, 5, 24),
    SS(32, 3, 5, 32),
    SS(0, 3, 5, 40),
    SS(0, 6, 4, 64),
    SS(16, 6, 4, 64),
    SS(32, 7, 5, 128),
    SS(0, 9, 6, 512),
    SS(0, 11, 6, 2048),
    SS(48, 0, 4, 0),
    SS(16, 0, 4, 1),
    SS(32, 0, 5, 2),
    SS(32, 0, 5, 3),
    SS(32, 0, 5, 5),
    SS(32, 0, 5, 6),
    SS(32, 0, 5, 8),
    SS(32, 0, 5, 9),
    SS(32, 0, 5, 11),
    SS(32, 0, 5, 12),
    SS(0, 0, 6, 15),
    SS(32, 1, 5, 18),
    SS(32, 1, 5, 20),
    SS(32, 2, 5, 24),
    SS(32, 2, 5, 28),
    SS(32, 3, 5, 40),
    SS(32, 4, 5, 48),
    SS(0, 16, 6, 65536),
    SS(0, 15, 6, 32768),
    SS(0, 14, 6, 16384),
    SS(0, 13, 6, 8192),
]; /* LL_defaultDTable */

/* Default FSE distribution table for Offset Codes */
static OF_defaultDTable: [ZSTD_seqSymbol; (1 << OF_DEFAULTNORMLOG) + 1] = [
    SS(1, 1, 1, OF_DEFAULTNORMLOG), /* header : fastMode, tableLog */
    /* nextState, nbAddBits, nbBits, baseVal */
    SS(0, 0, 5, 0),
    SS(0, 6, 4, 61),
    SS(0, 9, 5, 509),
    SS(0, 15, 5, 32765),
    SS(0, 21, 5, 2097149),
    SS(0, 3, 5, 5),
    SS(0, 7, 4, 125),
    SS(0, 12, 5, 4093),
    SS(0, 18, 5, 262141),
    SS(0, 23, 5, 8388605),
    SS(0, 5, 5, 29),
    SS(0, 8, 4, 253),
    SS(0, 14, 5, 16381),
    SS(0, 20, 5, 1048573),
    SS(0, 2, 5, 1),
    SS(16, 7, 4, 125),
    SS(0, 11, 5, 2045),
    SS(0, 17, 5, 131069),
    SS(0, 22, 5, 4194301),
    SS(0, 4, 5, 13),
    SS(16, 8, 4, 253),
    SS(0, 13, 5, 8189),
    SS(0, 19, 5, 524285),
    SS(0, 1, 5, 1),
    SS(16, 6, 4, 61),
    SS(0, 10, 5, 1021),
    SS(0, 16, 5, 65533),
    SS(0, 28, 5, 268435453),
    SS(0, 27, 5, 134217725),
    SS(0, 26, 5, 67108861),
    SS(0, 25, 5, 33554429),
    SS(0, 24, 5, 16777213),
]; /* OF_defaultDTable */

/* Default FSE distribution table for Match Lengths */
static ML_defaultDTable: [ZSTD_seqSymbol; (1 << ML_DEFAULTNORMLOG) + 1] = [
    SS(1, 1, 1, ML_DEFAULTNORMLOG), /* header : fastMode, tableLog */
    /* nextState, nbAddBits, nbBits, baseVal */
    SS(0, 0, 6, 3),
    SS(0, 0, 4, 4),
    SS(32, 0, 5, 5),
    SS(0, 0, 5, 6),
    SS(0, 0, 5, 8),
    SS(0, 0, 5, 9),
    SS(0, 0, 5, 11),
    SS(0, 0, 6, 13),
    SS(0, 0, 6, 16),
    SS(0, 0, 6, 19),
    SS(0, 0, 6, 22),
    SS(0, 0, 6, 25),
    SS(0, 0, 6, 28),
    SS(0, 0, 6, 31),
    SS(0, 0, 6, 34),
    SS(0, 1, 6, 37),
    SS(0, 1, 6, 41),
    SS(0, 2, 6, 47),
    SS(0, 3, 6, 59),
    SS(0, 4, 6, 83),
    SS(0, 7, 6, 131),
    SS(0, 9, 6, 515),
    SS(16, 0, 4, 4),
    SS(0, 0, 4, 5),
    SS(32, 0, 5, 6),
    SS(0, 0, 5, 7),
    SS(32, 0, 5, 9),
    SS(0, 0, 5, 10),
    SS(0, 0, 6, 12),
    SS(0, 0, 6, 15),
    SS(0, 0, 6, 18),
    SS(0, 0, 6, 21),
    SS(0, 0, 6, 24),
    SS(0, 0, 6, 27),
    SS(0, 0, 6, 30),
    SS(0, 0, 6, 33),
    SS(0, 1, 6, 35),
    SS(0, 1, 6, 39),
    SS(0, 2, 6, 43),
    SS(0, 3, 6, 51),
    SS(0, 4, 6, 67),
    SS(0, 5, 6, 99),
    SS(0, 8, 6, 259),
    SS(32, 0, 4, 4),
    SS(48, 0, 4, 4),
    SS(16, 0, 4, 5),
    SS(32, 0, 5, 7),
    SS(32, 0, 5, 8),
    SS(32, 0, 5, 10),
    SS(32, 0, 5, 11),
    SS(0, 0, 6, 14),
    SS(0, 0, 6, 17),
    SS(0, 0, 6, 20),
    SS(0, 0, 6, 23),
    SS(0, 0, 6, 26),
    SS(0, 0, 6, 29),
    SS(0, 0, 6, 32),
    SS(0, 16, 6, 65539),
    SS(0, 15, 6, 32771),
    SS(0, 14, 6, 16387),
    SS(0, 13, 6, 8195),
    SS(0, 12, 6, 4099),
    SS(0, 11, 6, 2051),
    SS(0, 10, 6, 1027),
]; /* ML_defaultDTable */

unsafe fn ZSTD_buildSeqTable_rle(dt: *mut ZSTD_seqSymbol, baseValue: U32, nbAddBits: U8) {
    let ptr: *mut c_void = dt as *mut c_void;
    let DTableH: *mut ZSTD_seqSymbol_header = ptr as *mut ZSTD_seqSymbol_header;
    let cell: *mut ZSTD_seqSymbol = dt.add(1);

    (*DTableH).tableLog = 0;
    (*DTableH).fastMode = 0;

    (*cell).nbBits = 0;
    (*cell).nextState = 0;
    (*cell).nbAdditionalBits = nbAddBits;
    (*cell).baseValue = baseValue;
}

/* ZSTD_buildFSETable() :
 * generate FSE decoding table for one symbol (ll, ml or off) */
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
    let tableDecode: *mut ZSTD_seqSymbol = dt.add(1);
    let maxSV1: U32 = maxSymbolValue.wrapping_add(1);
    let tableSize: U32 = 1u32 << tableLog;

    let symbolNext: *mut U16 = wksp as *mut U16;
    let spread: *mut BYTE = symbolNext.add((MaxSeq + 1) as usize) as *mut BYTE;
    let mut highThreshold: U32 = tableSize.wrapping_sub(1);

    let _ = wkspSize;

    /* Init, lay down lowprob symbols */
    {
        let mut DTableH = ZSTD_seqSymbol_header {
            fastMode: 0,
            tableLog: 0,
        };
        DTableH.tableLog = tableLog;
        DTableH.fastMode = 1;
        {
            let largeLimit: S16 = (1i32 << (tableLog.wrapping_sub(1))) as S16;
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
                s = s.wrapping_add(1);
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
     * no low probability (-1 count) symbols. */
    if highThreshold == tableSize.wrapping_sub(1) {
        let tableMask: usize = tableSize.wrapping_sub(1) as usize;
        let step: usize = FSE_TABLESTEP(tableSize) as usize;
        /* First lay down the symbols in order. */
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
                    MEM_write64(spread.add(pos.wrapping_add(i as usize)) as *mut c_void, sv);
                    i += 8;
                }
                pos = pos.wrapping_add(n as usize);
                s = s.wrapping_add(1);
                sv = sv.wrapping_add(add);
            }
        }
        /* Now we spread those positions across the table. */
        {
            let mut position: usize = 0;
            let unroll: usize = 2;
            let mut s: usize = 0;
            while s < tableSize as usize {
                let mut u: usize = 0;
                while u < unroll {
                    let uPosition: usize =
                        (position.wrapping_add(u.wrapping_mul(step))) & tableMask;
                    (*tableDecode.add(uPosition)).baseValue = *spread.add(s.wrapping_add(u)) as U32;
                    u = u.wrapping_add(1);
                }
                position = (position.wrapping_add(unroll.wrapping_mul(step))) & tableMask;
                s = s.wrapping_add(unroll);
            }
        }
    } else {
        let tableMask: U32 = tableSize.wrapping_sub(1);
        let step: U32 = FSE_TABLESTEP(tableSize);
        let mut position: U32 = 0;
        let mut s: U32 = 0;
        while s < maxSV1 {
            let n: c_int = *normalizedCounter.add(s as usize) as c_int;
            let mut i: c_int = 0;
            while i < n {
                (*tableDecode.add(position as usize)).baseValue = s;
                position = (position.wrapping_add(step)) & tableMask;
                while position > highThreshold {
                    position = (position.wrapping_add(step)) & tableMask; /* lowprob area */
                }
                i += 1;
            }
            s = s.wrapping_add(1);
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
                tableLog.wrapping_sub(ZSTD_highbit32(nextState)) as BYTE;
            (*tableDecode.add(u as usize)).nextState = ((nextState
                << (*tableDecode.add(u as usize)).nbBits as U32)
                .wrapping_sub(tableSize)) as U16;
            (*tableDecode.add(u as usize)).nbAdditionalBits = *nbAdditionalBits.add(symbol as usize);
            (*tableDecode.add(u as usize)).baseValue = *baseValue.add(symbol as usize);
            u = u.wrapping_add(1);
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

/* DYNAMIC_BMI2 == 0 : ZSTD_buildFSETable_body_bmi2() is not compiled in. */

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
    let _ = bmi2;
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

/* ZSTD_buildSeqTable() :
 * @return : nb bytes read from src, or an error code if it fails */
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
    if type_ == set_rle {
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
        return 1;
    } else if type_ == set_basic {
        *DTablePtr = defaultTable;
        return 0;
    } else if type_ == set_repeat {
        if flagRepeatTable == 0 {
            return ERROR(ZSTD_error_corruption_detected);
        }
        /* prefetch FSE table if used : PREFETCH_AREA() is a no-op here */
        let _ = ddictIsCold;
        let _ = nbSeq;
        let _ = maxLog;
        return 0;
    } else if type_ == set_compressed {
        let mut tableLog: c_uint = 0;
        let mut norm: [S16; MaxSeq as usize + 1] = [0; MaxSeq as usize + 1];
        let headerSize: usize = FSE_readNCount(
            norm.as_mut_ptr(),
            &mut max as *mut c_uint,
            &mut tableLog as *mut c_uint,
            src,
            srcSize,
        );
        if FSE_isError(headerSize) != 0 {
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
        return headerSize;
    } else {
        return ERROR(ZSTD_error_GENERIC);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_decodeSeqHeaders(
    dctx: *mut ZSTD_DCtx,
    nbSeqPtr: *mut c_int,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let istart: *const BYTE = src as *const BYTE;
    let iend: *const BYTE = (istart as usize).wrapping_add(srcSize) as *const BYTE;
    let mut ip: *const BYTE = istart;
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
            if (ip as usize).wrapping_add(2) > iend as usize {
                return ERROR(ZSTD_error_srcSize_wrong);
            }
            nbSeq = (MEM_readLE16(ip as *const c_void) as c_int).wrapping_add(LONGNBSEQ as c_int);
            ip = ip.add(2);
        } else {
            if ip as usize >= iend as usize {
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
        return (ip as usize).wrapping_sub(istart as usize);
    }

    /* FSE table descriptors */
    if (ip as usize).wrapping_add(1) > iend as usize {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    if (*ip & 3) != 0 {
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
                core::ptr::addr_of_mut!((*dctx).entropy.LLTable) as *mut ZSTD_seqSymbol,
                core::ptr::addr_of_mut!((*dctx).LLTptr),
                LLtype,
                MaxLL,
                LLFSELog,
                ip as *const c_void,
                (iend as usize).wrapping_sub(ip as usize),
                LL_base.as_ptr(),
                LL_bits.as_ptr(),
                LL_defaultDTable.as_ptr(),
                (*dctx).fseEntropy,
                (*dctx).ddictIsCold,
                nbSeq,
                core::ptr::addr_of_mut!((*dctx).workspace) as *mut U32,
                core::mem::size_of::<[U32; HUF_DECOMPRESS_WORKSPACE_SIZE_U32]>(),
                ZSTD_DCtx_get_bmi2(dctx),
            );
            if ZSTD_isError(llhSize) != 0 {
                return ERROR(ZSTD_error_corruption_detected);
            }
            ip = ip.add(llhSize);
        }

        {
            let ofhSize: usize = ZSTD_buildSeqTable(
                core::ptr::addr_of_mut!((*dctx).entropy.OFTable) as *mut ZSTD_seqSymbol,
                core::ptr::addr_of_mut!((*dctx).OFTptr),
                OFtype,
                MaxOff,
                OffFSELog,
                ip as *const c_void,
                (iend as usize).wrapping_sub(ip as usize),
                OF_base.as_ptr(),
                OF_bits.as_ptr(),
                OF_defaultDTable.as_ptr(),
                (*dctx).fseEntropy,
                (*dctx).ddictIsCold,
                nbSeq,
                core::ptr::addr_of_mut!((*dctx).workspace) as *mut U32,
                core::mem::size_of::<[U32; HUF_DECOMPRESS_WORKSPACE_SIZE_U32]>(),
                ZSTD_DCtx_get_bmi2(dctx),
            );
            if ZSTD_isError(ofhSize) != 0 {
                return ERROR(ZSTD_error_corruption_detected);
            }
            ip = ip.add(ofhSize);
        }

        {
            let mlhSize: usize = ZSTD_buildSeqTable(
                core::ptr::addr_of_mut!((*dctx).entropy.MLTable) as *mut ZSTD_seqSymbol,
                core::ptr::addr_of_mut!((*dctx).MLTptr),
                MLtype,
                MaxML,
                MLFSELog,
                ip as *const c_void,
                (iend as usize).wrapping_sub(ip as usize),
                ML_base.as_ptr(),
                ML_bits.as_ptr(),
                ML_defaultDTable.as_ptr(),
                (*dctx).fseEntropy,
                (*dctx).ddictIsCold,
                nbSeq,
                core::ptr::addr_of_mut!((*dctx).workspace) as *mut U32,
                core::mem::size_of::<[U32; HUF_DECOMPRESS_WORKSPACE_SIZE_U32]>(),
                ZSTD_DCtx_get_bmi2(dctx),
            );
            if ZSTD_isError(mlhSize) != 0 {
                return ERROR(ZSTD_error_corruption_detected);
            }
            ip = ip.add(mlhSize);
        }
    }

    (ip as usize).wrapping_sub(istart as usize)
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct seq_t {
    pub litLength: usize,
    pub matchLength: usize,
    pub offset: usize,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct ZSTD_fseState {
    pub state: usize,
    pub table: *const ZSTD_seqSymbol,
}

impl Default for ZSTD_fseState {
    fn default() -> Self {
        ZSTD_fseState {
            state: 0,
            table: core::ptr::null(),
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct seqState_t {
    pub DStream: BIT_DStream_t,
    pub stateLL: ZSTD_fseState,
    pub stateOffb: ZSTD_fseState,
    pub stateML: ZSTD_fseState,
    pub prevOffset: [usize; ZSTD_REP_NUM],
}

static dec32table: [U32; 8] = [0, 1, 2, 1, 4, 4, 4, 4]; /* added */
static dec64table: [c_int; 8] = [8, 8, 8, 7, 8, 9, 10, 11]; /* subtracted */

/* ZSTD_overlapCopy8() :
 *  Copies 8 bytes from ip to op and updates op and ip where ip <= op. */
#[inline(always)]
unsafe fn ZSTD_overlapCopy8(op: *mut *mut BYTE, ip: *mut *const BYTE, offset: usize) {
    if offset < 8 {
        /* close range match, overlap */
        let sub2: c_int = dec64table[offset];
        *(*op).add(0) = *(*ip).add(0);
        *(*op).add(1) = *(*ip).add(1);
        *(*op).add(2) = *(*ip).add(2);
        *(*op).add(3) = *(*ip).add(3);
        *ip = ((*ip) as usize).wrapping_add(dec32table[offset] as usize) as *const BYTE;
        ZSTD_copy4((*op).add(4) as *mut c_void, *ip as *const c_void);
        *ip = ((*ip) as isize).wrapping_sub(sub2 as isize) as *const BYTE;
    } else {
        ZSTD_copy8(*op as *mut c_void, *ip as *const c_void);
    }
    *ip = ((*ip) as usize).wrapping_add(8) as *const BYTE;
    *op = ((*op) as usize).wrapping_add(8) as *mut BYTE;
}

/* ZSTD_safecopy() :
 *  Specialized version of memcpy() that is allowed to READ up to WILDCOPY_OVERLENGTH past the
 *  input buffer and write up to 16 bytes past oend_w. */
unsafe fn ZSTD_safecopy(
    mut op: *mut BYTE,
    oend_w: *const BYTE,
    mut ip: *const BYTE,
    mut length: isize,
    ovtype: ZSTD_overlap_e,
) {
    let diff: isize = (op as isize).wrapping_sub(ip as isize);
    let oend: *mut BYTE = (op as isize).wrapping_add(length) as *mut BYTE;

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

    if (oend as *const BYTE) <= oend_w {
        /* No risk of overwrite. */
        ZSTD_wildcopy(op as *mut c_void, ip as *const c_void, length, ovtype);
        return;
    }
    if (op as *const BYTE) <= oend_w {
        /* Wildcopy until we get close to the end. */
        let d: isize = (oend_w as isize).wrapping_sub(op as isize);
        ZSTD_wildcopy(op as *mut c_void, ip as *const c_void, d, ovtype);
        ip = (ip as isize).wrapping_add(d) as *const BYTE;
        op = (op as isize).wrapping_add(d) as *mut BYTE;
    }
    /* Handle the leftovers. */
    while op < oend {
        *op = *ip;
        op = op.add(1);
        ip = ip.add(1);
    }
}

/* ZSTD_safecopyDstBeforeSrc():
 * This version allows overlap with dst before src, or handles the non-overlap case with dst
 * after src */
unsafe fn ZSTD_safecopyDstBeforeSrc(mut op: *mut BYTE, mut ip: *const BYTE, length: isize) {
    let diff: isize = (op as isize).wrapping_sub(ip as isize);
    let oend: *mut BYTE = (op as isize).wrapping_add(length) as *mut BYTE;

    if length < 8 || diff > -8 {
        /* Handle short lengths, close overlaps, and dst not before src. */
        while op < oend {
            *op = *ip;
            op = op.add(1);
            ip = ip.add(1);
        }
        return;
    }

    if (op as isize) <= (oend as isize).wrapping_sub(WILDCOPY_OVERLENGTH as isize)
        && diff < -WILDCOPY_VECLEN
    {
        let d: isize = (oend as isize)
            .wrapping_sub(WILDCOPY_OVERLENGTH as isize)
            .wrapping_sub(op as isize);
        ZSTD_wildcopy(op as *mut c_void, ip as *const c_void, d, ZSTD_no_overlap);
        ip = (ip as isize).wrapping_add(d) as *const BYTE;
        op = (op as isize).wrapping_add(d) as *mut BYTE;
    }

    /* Handle the leftovers. */
    while op < oend {
        *op = *ip;
        op = op.add(1);
        ip = ip.add(1);
    }
}

/* ZSTD_execSequenceEnd():
 * This version handles cases that are near the end of the output buffer. */
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
    let oLitEnd: *mut BYTE = (op as usize).wrapping_add(sequence.litLength) as *mut BYTE;
    let sequenceLength: usize = sequence.litLength.wrapping_add(sequence.matchLength);
    let iLitEnd: *const BYTE =
        ((*litPtr) as usize).wrapping_add(sequence.litLength) as *const BYTE;
    let mut mtch: *const BYTE = (oLitEnd as usize).wrapping_sub(sequence.offset) as *const BYTE;
    let oend_w: *const BYTE = (oend as usize).wrapping_sub(WILDCOPY_OVERLENGTH) as *const BYTE;

    /* bounds checks : careful of address space overflow in 32-bit mode */
    if sequenceLength > (oend as usize).wrapping_sub(op as usize) {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    if sequence.litLength > (litLimit as usize).wrapping_sub((*litPtr) as usize) {
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
    if sequence.offset > (oLitEnd as usize).wrapping_sub(prefixStart as usize) {
        /* offset beyond prefix */
        if sequence.offset > (oLitEnd as usize).wrapping_sub(virtualStart as usize) {
            return ERROR(ZSTD_error_corruption_detected);
        }
        mtch = (dictEnd as usize)
            .wrapping_sub((prefixStart as usize).wrapping_sub(mtch as usize))
            as *const BYTE;
        if (mtch as usize).wrapping_add(sequence.matchLength) <= dictEnd as usize {
            ZSTD_memmove(
                oLitEnd as *mut c_void,
                mtch as *const c_void,
                sequence.matchLength,
            );
            return sequenceLength;
        }
        /* span extDict & currentPrefixSegment */
        {
            let length1: usize = (dictEnd as usize).wrapping_sub(mtch as usize);
            ZSTD_memmove(oLitEnd as *mut c_void, mtch as *const c_void, length1);
            op = (oLitEnd as usize).wrapping_add(length1) as *mut BYTE;
            sequence.matchLength = sequence.matchLength.wrapping_sub(length1);
            mtch = prefixStart;
        }
    }
    ZSTD_safecopy(
        op,
        oend_w,
        mtch,
        sequence.matchLength as isize,
        ZSTD_overlap_src_before_dst,
    );
    sequenceLength
}

/* ZSTD_execSequenceEndSplitLitBuffer():
 * This version is intended to be used during instances where the litBuffer is still split. */
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
    let oLitEnd: *mut BYTE = (op as usize).wrapping_add(sequence.litLength) as *mut BYTE;
    let sequenceLength: usize = sequence.litLength.wrapping_add(sequence.matchLength);
    let iLitEnd: *const BYTE =
        ((*litPtr) as usize).wrapping_add(sequence.litLength) as *const BYTE;
    let mut mtch: *const BYTE = (oLitEnd as usize).wrapping_sub(sequence.offset) as *const BYTE;

    /* bounds checks : careful of address space overflow in 32-bit mode */
    if sequenceLength > (oend as usize).wrapping_sub(op as usize) {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    if sequence.litLength > (litLimit as usize).wrapping_sub((*litPtr) as usize) {
        return ERROR(ZSTD_error_corruption_detected);
    }

    /* copy literals */
    if (op as *const BYTE) > *litPtr
        && (op as usize) < ((*litPtr) as usize).wrapping_add(sequence.litLength)
    {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    ZSTD_safecopyDstBeforeSrc(op, *litPtr, sequence.litLength as isize);
    op = oLitEnd;
    *litPtr = iLitEnd;

    /* copy Match */
    if sequence.offset > (oLitEnd as usize).wrapping_sub(prefixStart as usize) {
        /* offset beyond prefix */
        if sequence.offset > (oLitEnd as usize).wrapping_sub(virtualStart as usize) {
            return ERROR(ZSTD_error_corruption_detected);
        }
        mtch = (dictEnd as usize)
            .wrapping_sub((prefixStart as usize).wrapping_sub(mtch as usize))
            as *const BYTE;
        if (mtch as usize).wrapping_add(sequence.matchLength) <= dictEnd as usize {
            ZSTD_memmove(
                oLitEnd as *mut c_void,
                mtch as *const c_void,
                sequence.matchLength,
            );
            return sequenceLength;
        }
        /* span extDict & currentPrefixSegment */
        {
            let length1: usize = (dictEnd as usize).wrapping_sub(mtch as usize);
            ZSTD_memmove(oLitEnd as *mut c_void, mtch as *const c_void, length1);
            op = (oLitEnd as usize).wrapping_add(length1) as *mut BYTE;
            sequence.matchLength = sequence.matchLength.wrapping_sub(length1);
            mtch = prefixStart;
        }
    }
    ZSTD_safecopy(
        op,
        oend_w,
        mtch,
        sequence.matchLength as isize,
        ZSTD_overlap_src_before_dst,
    );
    sequenceLength
}

#[inline(always)]
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
    let oLitEnd: *mut BYTE = (op as usize).wrapping_add(sequence.litLength) as *mut BYTE;
    let sequenceLength: usize = sequence.litLength.wrapping_add(sequence.matchLength);
    /* risk : address space overflow (32-bits) */
    let oMatchEnd: *mut BYTE = (op as usize).wrapping_add(sequenceLength) as *mut BYTE;
    /* risk : address space underflow on oend=NULL */
    let oend_w: *const BYTE = (oend as usize).wrapping_sub(WILDCOPY_OVERLENGTH) as *const BYTE;
    let iLitEnd: *const BYTE =
        ((*litPtr) as usize).wrapping_add(sequence.litLength) as *const BYTE;
    let mut mtch: *const BYTE = (oLitEnd as usize).wrapping_sub(sequence.offset) as *const BYTE;

    /* Handle edge cases in a slow path */
    if iLitEnd > litLimit
        || (oMatchEnd as *const BYTE) > oend_w
        || (MEM_32bits() != 0
            && (oend as usize).wrapping_sub(op as usize)
                < sequenceLength.wrapping_add(WILDCOPY_OVERLENGTH))
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

    /* Copy Literals */
    ZSTD_copy16(op as *mut c_void, *litPtr as *const c_void);
    if sequence.litLength > 16 {
        ZSTD_wildcopy(
            op.add(16) as *mut c_void,
            (*litPtr).add(16) as *const c_void,
            sequence.litLength.wrapping_sub(16) as isize,
            ZSTD_no_overlap,
        );
    }
    op = oLitEnd;
    *litPtr = iLitEnd; /* update for next sequence */

    /* Copy Match */
    if sequence.offset > (oLitEnd as usize).wrapping_sub(prefixStart as usize) {
        /* offset beyond prefix -> go into extDict */
        if sequence.offset > (oLitEnd as usize).wrapping_sub(virtualStart as usize) {
            return ERROR(ZSTD_error_corruption_detected);
        }
        mtch = (dictEnd as usize)
            .wrapping_add((mtch as usize).wrapping_sub(prefixStart as usize)) as *const BYTE;
        if (mtch as usize).wrapping_add(sequence.matchLength) <= dictEnd as usize {
            ZSTD_memmove(
                oLitEnd as *mut c_void,
                mtch as *const c_void,
                sequence.matchLength,
            );
            return sequenceLength;
        }
        /* span extDict & currentPrefixSegment */
        {
            let length1: usize = (dictEnd as usize).wrapping_sub(mtch as usize);
            ZSTD_memmove(oLitEnd as *mut c_void, mtch as *const c_void, length1);
            op = (oLitEnd as usize).wrapping_add(length1) as *mut BYTE;
            sequence.matchLength = sequence.matchLength.wrapping_sub(length1);
            mtch = prefixStart;
        }
    }

    /* Nearly all offsets are >= WILDCOPY_VECLEN bytes, which means we can use wildcopy
     * without overlap checking. */
    if sequence.offset >= WILDCOPY_VECLEN as usize {
        ZSTD_wildcopy(
            op as *mut c_void,
            mtch as *const c_void,
            sequence.matchLength as isize,
            ZSTD_no_overlap,
        );
        return sequenceLength;
    }

    /* Copy 8 bytes and spread the offset to be >= 8. */
    ZSTD_overlapCopy8(&mut op, &mut mtch, sequence.offset);

    /* If the match length is > 8 bytes, then continue with the wildcopy. */
    if sequence.matchLength > 8 {
        ZSTD_wildcopy(
            op as *mut c_void,
            mtch as *const c_void,
            (sequence.matchLength as isize).wrapping_sub(8),
            ZSTD_overlap_src_before_dst,
        );
    }
    sequenceLength
}

#[inline(always)]
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
    let oLitEnd: *mut BYTE = (op as usize).wrapping_add(sequence.litLength) as *mut BYTE;
    let sequenceLength: usize = sequence.litLength.wrapping_add(sequence.matchLength);
    /* risk : address space overflow (32-bits) */
    let oMatchEnd: *mut BYTE = (op as usize).wrapping_add(sequenceLength) as *mut BYTE;
    let iLitEnd: *const BYTE =
        ((*litPtr) as usize).wrapping_add(sequence.litLength) as *const BYTE;
    let mut mtch: *const BYTE = (oLitEnd as usize).wrapping_sub(sequence.offset) as *const BYTE;

    /* Handle edge cases in a slow path */
    if iLitEnd > litLimit
        || (oMatchEnd as *const BYTE) > oend_w
        || (MEM_32bits() != 0
            && (oend as usize).wrapping_sub(op as usize)
                < sequenceLength.wrapping_add(WILDCOPY_OVERLENGTH))
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

    /* Copy Literals */
    ZSTD_copy16(op as *mut c_void, *litPtr as *const c_void);
    if sequence.litLength > 16 {
        ZSTD_wildcopy(
            op.add(16) as *mut c_void,
            (*litPtr).add(16) as *const c_void,
            sequence.litLength.wrapping_sub(16) as isize,
            ZSTD_no_overlap,
        );
    }
    op = oLitEnd;
    *litPtr = iLitEnd; /* update for next sequence */

    /* Copy Match */
    if sequence.offset > (oLitEnd as usize).wrapping_sub(prefixStart as usize) {
        /* offset beyond prefix -> go into extDict */
        if sequence.offset > (oLitEnd as usize).wrapping_sub(virtualStart as usize) {
            return ERROR(ZSTD_error_corruption_detected);
        }
        mtch = (dictEnd as usize)
            .wrapping_add((mtch as usize).wrapping_sub(prefixStart as usize)) as *const BYTE;
        if (mtch as usize).wrapping_add(sequence.matchLength) <= dictEnd as usize {
            ZSTD_memmove(
                oLitEnd as *mut c_void,
                mtch as *const c_void,
                sequence.matchLength,
            );
            return sequenceLength;
        }
        /* span extDict & currentPrefixSegment */
        {
            let length1: usize = (dictEnd as usize).wrapping_sub(mtch as usize);
            ZSTD_memmove(oLitEnd as *mut c_void, mtch as *const c_void, length1);
            op = (oLitEnd as usize).wrapping_add(length1) as *mut BYTE;
            sequence.matchLength = sequence.matchLength.wrapping_sub(length1);
            mtch = prefixStart;
        }
    }

    /* Nearly all offsets are >= WILDCOPY_VECLEN bytes, which means we can use wildcopy
     * without overlap checking. */
    if sequence.offset >= WILDCOPY_VECLEN as usize {
        ZSTD_wildcopy(
            op as *mut c_void,
            mtch as *const c_void,
            sequence.matchLength as isize,
            ZSTD_no_overlap,
        );
        return sequenceLength;
    }

    /* Copy 8 bytes and spread the offset to be >= 8. */
    ZSTD_overlapCopy8(&mut op, &mut mtch, sequence.offset);

    /* If the match length is > 8 bytes, then continue with the wildcopy. */
    if sequence.matchLength > 8 {
        ZSTD_wildcopy(
            op as *mut c_void,
            mtch as *const c_void,
            (sequence.matchLength as isize).wrapping_sub(8),
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
    let ptr: *const c_void = dt as *const c_void;
    let DTableH: *const ZSTD_seqSymbol_header = ptr as *const ZSTD_seqSymbol_header;
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
    (*DStatePtr).state = (nextState as usize).wrapping_add(lowBits);
}

/* We need to add at most (ZSTD_WINDOWLOG_MAX_32 - 1) bits to read the maximum
 * offset bits. */
pub const LONG_OFFSETS_MAX_EXTRA_BITS_32: u32 =
    if (ZSTD_WINDOWLOG_MAX_32 as u32) > STREAM_ACCUMULATOR_MIN_32 {
        ZSTD_WINDOWLOG_MAX_32 as u32 - STREAM_ACCUMULATOR_MIN_32
    } else {
        0
    };

pub type ZSTD_longOffset_e = c_uint;
pub const ZSTD_lo_isRegularOffset: ZSTD_longOffset_e = 0;
pub const ZSTD_lo_isLongOffset: ZSTD_longOffset_e = 1;

/**
 * ZSTD_decodeSequence():
 * @return : Sequence (litL + matchL + offset)
 */
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
                    /* Always read extra bits, this keeps the logic simple. */
                    let extraBits: U32 = LONG_OFFSETS_MAX_EXTRA_BITS_32;
                    offset = (ofBase as usize).wrapping_add(
                        BIT_readBitsFast(
                            core::ptr::addr_of_mut!((*seqState).DStream),
                            (ofBits as u32).wrapping_sub(extraBits),
                        ) << extraBits,
                    );
                    BIT_reloadDStream(core::ptr::addr_of_mut!((*seqState).DStream));
                    offset = offset.wrapping_add(BIT_readBitsFast(
                        core::ptr::addr_of_mut!((*seqState).DStream),
                        extraBits,
                    ));
                } else {
                    /* <=  (ZSTD_WINDOWLOG_MAX-1) bits */
                    offset = (ofBase as usize).wrapping_add(BIT_readBitsFast(
                        core::ptr::addr_of_mut!((*seqState).DStream),
                        ofBits as u32,
                    ));
                    if MEM_32bits() != 0 {
                        BIT_reloadDStream(core::ptr::addr_of_mut!((*seqState).DStream));
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
                    offset = (ofBase.wrapping_add(ll0) as usize).wrapping_add(BIT_readBitsFast(
                        core::ptr::addr_of_mut!((*seqState).DStream),
                        1,
                    ));
                    {
                        let mut temp: usize = if offset == 3 {
                            (*seqState).prevOffset[0].wrapping_sub(1)
                        } else {
                            (*seqState).prevOffset[offset]
                        };
                        /* 0 is not valid: input corrupted => force offset to -1 */
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
                core::ptr::addr_of_mut!((*seqState).DStream),
                mlBits as u32,
            ));
        }

        if MEM_32bits() != 0
            && (mlBits as u32).wrapping_add(llBits as u32)
                >= STREAM_ACCUMULATOR_MIN_32 - LONG_OFFSETS_MAX_EXTRA_BITS_32
        {
            BIT_reloadDStream(core::ptr::addr_of_mut!((*seqState).DStream));
        }
        if MEM_64bits() != 0
            && (totalBits as u32)
                >= STREAM_ACCUMULATOR_MIN_64 - (LLFSELog + MLFSELog + OffFSELog)
        {
            BIT_reloadDStream(core::ptr::addr_of_mut!((*seqState).DStream));
        }

        if llBits > 0 {
            seq.litLength = seq.litLength.wrapping_add(BIT_readBitsFast(
                core::ptr::addr_of_mut!((*seqState).DStream),
                llBits as u32,
            ));
        }

        if MEM_32bits() != 0 {
            BIT_reloadDStream(core::ptr::addr_of_mut!((*seqState).DStream));
        }

        if isLastSeq == 0 {
            /* don't update FSE state for last Sequence */
            ZSTD_updateFseStateWithDInfo(
                core::ptr::addr_of_mut!((*seqState).stateLL),
                core::ptr::addr_of_mut!((*seqState).DStream),
                llNext,
                llnbBits,
            ); /* <=  9 bits */
            ZSTD_updateFseStateWithDInfo(
                core::ptr::addr_of_mut!((*seqState).stateML),
                core::ptr::addr_of_mut!((*seqState).DStream),
                mlNext,
                mlnbBits,
            ); /* <=  9 bits */
            if MEM_32bits() != 0 {
                BIT_reloadDStream(core::ptr::addr_of_mut!((*seqState).DStream));
            } /* <= 18 bits */
            ZSTD_updateFseStateWithDInfo(
                core::ptr::addr_of_mut!((*seqState).stateOffb),
                core::ptr::addr_of_mut!((*seqState).DStream),
                ofNext,
                ofnbBits,
            ); /* <=  8 bits */
            BIT_reloadDStream(core::ptr::addr_of_mut!((*seqState).DStream));
        }
    }

    seq
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
    let ip: *const BYTE = seqStart as *const BYTE;
    let iend: *const BYTE = (ip as usize).wrapping_add(seqSize) as *const BYTE;
    let ostart: *mut BYTE = dst as *mut BYTE;
    let oend: *mut BYTE = ZSTD_maybeNullPtrAdd(ostart, maxDstSize as isize);
    let mut op: *mut BYTE = ostart;
    let mut litPtr: *const BYTE = (*dctx).litPtr;
    let mut litBufferEnd: *const BYTE = (*dctx).litBufferEnd;
    let prefixStart: *const BYTE = (*dctx).prefixStart as *const BYTE;
    let vBase: *const BYTE = (*dctx).virtualStart as *const BYTE;
    let dictEnd: *const BYTE = (*dctx).dictEnd as *const BYTE;

    /* Literals are split between internal buffer & output buffer */
    if nbSeq != 0 {
        let mut seqState = seqState_t::default();
        (*dctx).fseEntropy = 1;
        {
            let mut i: U32 = 0;
            while (i as usize) < ZSTD_REP_NUM {
                seqState.prevOffset[i as usize] = (*dctx).entropy.rep[i as usize] as usize;
                i += 1;
            }
        }
        if ERR_isError(BIT_initDStream(
            core::ptr::addr_of_mut!(seqState.DStream),
            ip as *const c_void,
            (iend as usize).wrapping_sub(ip as usize),
        )) != 0
        {
            return ERROR(ZSTD_error_corruption_detected);
        }
        ZSTD_initFseState(
            core::ptr::addr_of_mut!(seqState.stateLL),
            core::ptr::addr_of_mut!(seqState.DStream),
            (*dctx).LLTptr,
        );
        ZSTD_initFseState(
            core::ptr::addr_of_mut!(seqState.stateOffb),
            core::ptr::addr_of_mut!(seqState.DStream),
            (*dctx).OFTptr,
        );
        ZSTD_initFseState(
            core::ptr::addr_of_mut!(seqState.stateML),
            core::ptr::addr_of_mut!(seqState.DStream),
            (*dctx).MLTptr,
        );

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
                    core::ptr::addr_of_mut!(seqState),
                    isLongOffset,
                    (nbSeq == 1) as c_int,
                );
                if (litPtr as usize).wrapping_add(sequence.litLength)
                    > (*dctx).litBufferEnd as usize
                {
                    break;
                }
                {
                    let oneSeqSize: usize = ZSTD_execSequenceSplitLitBuffer(
                        op,
                        oend,
                        (litPtr as usize)
                            .wrapping_add(sequence.litLength)
                            .wrapping_sub(WILDCOPY_OVERLENGTH) as *const BYTE,
                        sequence,
                        core::ptr::addr_of_mut!(litPtr),
                        litBufferEnd,
                        prefixStart,
                        vBase,
                        dictEnd,
                    );
                    if ZSTD_isError(oneSeqSize) != 0 {
                        return oneSeqSize;
                    }
                    op = op.add(oneSeqSize);
                }
                nbSeq -= 1;
            }

            /* If there are more sequences, they will need to read literals from
             * litExtraBuffer; copy over the remainder from dst and update litPtr and litEnd */
            if nbSeq > 0 {
                let leftoverLit: usize =
                    ((*dctx).litBufferEnd as usize).wrapping_sub(litPtr as usize);
                if leftoverLit != 0 {
                    if leftoverLit > (oend as usize).wrapping_sub(op as usize) {
                        return ERROR(ZSTD_error_dstSize_tooSmall);
                    }
                    ZSTD_safecopyDstBeforeSrc(op, litPtr, leftoverLit as isize);
                    sequence.litLength = sequence.litLength.wrapping_sub(leftoverLit);
                    op = op.add(leftoverLit);
                }
                litPtr = core::ptr::addr_of!((*dctx).litExtraBuffer) as *const BYTE;
                litBufferEnd = (core::ptr::addr_of!((*dctx).litExtraBuffer) as usize)
                    .wrapping_add(ZSTD_LITBUFFEREXTRASIZE) as *const BYTE;
                (*dctx).litBufferLocation = ZSTD_not_in_dst;
                {
                    let oneSeqSize: usize = ZSTD_execSequence(
                        op,
                        oend,
                        sequence,
                        core::ptr::addr_of_mut!(litPtr),
                        litBufferEnd,
                        prefixStart,
                        vBase,
                        dictEnd,
                    );
                    if ZSTD_isError(oneSeqSize) != 0 {
                        return oneSeqSize;
                    }
                    op = op.add(oneSeqSize);
                }
                nbSeq -= 1;
            }
        }

        if nbSeq > 0 {
            /* there is remaining lit from extra buffer */
            while nbSeq != 0 {
                let sequence: seq_t = ZSTD_decodeSequence(
                    core::ptr::addr_of_mut!(seqState),
                    isLongOffset,
                    (nbSeq == 1) as c_int,
                );
                let oneSeqSize: usize = ZSTD_execSequence(
                    op,
                    oend,
                    sequence,
                    core::ptr::addr_of_mut!(litPtr),
                    litBufferEnd,
                    prefixStart,
                    vBase,
                    dictEnd,
                );
                if ZSTD_isError(oneSeqSize) != 0 {
                    return oneSeqSize;
                }
                op = op.add(oneSeqSize);
                nbSeq -= 1;
            }
        }

        /* check if reached exact end */
        if nbSeq != 0 {
            return ERROR(ZSTD_error_corruption_detected);
        }
        if BIT_endOfDStream(core::ptr::addr_of!(seqState.DStream)) == 0 {
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
        let lastLLSize: usize = (litBufferEnd as usize).wrapping_sub(litPtr as usize);
        if lastLLSize > (oend as usize).wrapping_sub(op as usize) {
            return ERROR(ZSTD_error_dstSize_tooSmall);
        }
        if !op.is_null() {
            ZSTD_memmove(op as *mut c_void, litPtr as *const c_void, lastLLSize);
            op = op.add(lastLLSize);
        }
        litPtr = core::ptr::addr_of!((*dctx).litExtraBuffer) as *const BYTE;
        litBufferEnd = (core::ptr::addr_of!((*dctx).litExtraBuffer) as usize)
            .wrapping_add(ZSTD_LITBUFFEREXTRASIZE) as *const BYTE;
        (*dctx).litBufferLocation = ZSTD_not_in_dst;
    }
    /* copy last literals from internal buffer */
    {
        let lastLLSize: usize = (litBufferEnd as usize).wrapping_sub(litPtr as usize);
        if lastLLSize > (oend as usize).wrapping_sub(op as usize) {
            return ERROR(ZSTD_error_dstSize_tooSmall);
        }
        if !op.is_null() {
            ZSTD_memcpy(op as *mut c_void, litPtr as *const c_void, lastLLSize);
            op = op.add(lastLLSize);
        }
    }

    (op as usize).wrapping_sub(ostart as usize)
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
    let ip: *const BYTE = seqStart as *const BYTE;
    let iend: *const BYTE = (ip as usize).wrapping_add(seqSize) as *const BYTE;
    let ostart: *mut BYTE = dst as *mut BYTE;
    let oend: *mut BYTE = if (*dctx).litBufferLocation == ZSTD_not_in_dst {
        ZSTD_maybeNullPtrAdd(ostart, maxDstSize as isize)
    } else {
        (*dctx).litBuffer
    };
    let mut op: *mut BYTE = ostart;
    let mut litPtr: *const BYTE = (*dctx).litPtr;
    let litEnd: *const BYTE = (litPtr as usize).wrapping_add((*dctx).litSize) as *const BYTE;
    let prefixStart: *const BYTE = (*dctx).prefixStart as *const BYTE;
    let vBase: *const BYTE = (*dctx).virtualStart as *const BYTE;
    let dictEnd: *const BYTE = (*dctx).dictEnd as *const BYTE;

    /* Regen sequences */
    if nbSeq != 0 {
        let mut seqState = seqState_t::default();
        (*dctx).fseEntropy = 1;
        {
            let mut i: U32 = 0;
            while (i as usize) < ZSTD_REP_NUM {
                seqState.prevOffset[i as usize] = (*dctx).entropy.rep[i as usize] as usize;
                i += 1;
            }
        }
        if ERR_isError(BIT_initDStream(
            core::ptr::addr_of_mut!(seqState.DStream),
            ip as *const c_void,
            (iend as usize).wrapping_sub(ip as usize),
        )) != 0
        {
            return ERROR(ZSTD_error_corruption_detected);
        }
        ZSTD_initFseState(
            core::ptr::addr_of_mut!(seqState.stateLL),
            core::ptr::addr_of_mut!(seqState.DStream),
            (*dctx).LLTptr,
        );
        ZSTD_initFseState(
            core::ptr::addr_of_mut!(seqState.stateOffb),
            core::ptr::addr_of_mut!(seqState.DStream),
            (*dctx).OFTptr,
        );
        ZSTD_initFseState(
            core::ptr::addr_of_mut!(seqState.stateML),
            core::ptr::addr_of_mut!(seqState.DStream),
            (*dctx).MLTptr,
        );

        while nbSeq != 0 {
            let sequence: seq_t = ZSTD_decodeSequence(
                core::ptr::addr_of_mut!(seqState),
                isLongOffset,
                (nbSeq == 1) as c_int,
            );
            let oneSeqSize: usize = ZSTD_execSequence(
                op,
                oend,
                sequence,
                core::ptr::addr_of_mut!(litPtr),
                litEnd,
                prefixStart,
                vBase,
                dictEnd,
            );
            if ZSTD_isError(oneSeqSize) != 0 {
                return oneSeqSize;
            }
            op = op.add(oneSeqSize);
            nbSeq -= 1;
        }

        /* check if reached exact end */
        if BIT_endOfDStream(core::ptr::addr_of!(seqState.DStream)) == 0 {
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
        let lastLLSize: usize = (litEnd as usize).wrapping_sub(litPtr as usize);
        if lastLLSize > (oend as usize).wrapping_sub(op as usize) {
            return ERROR(ZSTD_error_dstSize_tooSmall);
        }
        if !op.is_null() {
            ZSTD_memcpy(op as *mut c_void, litPtr as *const c_void, lastLLSize);
            op = op.add(lastLLSize);
        }
    }

    (op as usize).wrapping_sub(ostart as usize)
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

/* PREFETCH_L1() is a no-op in this build, so ZSTD_prefetchMatch() only advances
 * @prefetchPos. */
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
        /* note : this operation can overflow when seq.offset is really too large */
        let _mtch: *const BYTE = ZSTD_wrappedPtrSub(
            ZSTD_wrappedPtrAdd(matchBase, prefetchPos as isize),
            sequence.offset as isize,
        );
        /* PREFETCH_L1(match); PREFETCH_L1(match+CACHELINE_SIZE); -> no-ops */
    }
    prefetchPos.wrapping_add(sequence.matchLength)
}

pub const STORED_SEQS: usize = 8;
pub const STORED_SEQS_MASK: c_int = (STORED_SEQS - 1) as c_int;
pub const ADVANCED_SEQS: c_int = STORED_SEQS as c_int;

/* This decoding function employs prefetching to reduce latency impact of cache misses. */
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
    let ip: *const BYTE = seqStart as *const BYTE;
    let iend: *const BYTE = (ip as usize).wrapping_add(seqSize) as *const BYTE;
    let ostart: *mut BYTE = dst as *mut BYTE;
    let oend: *mut BYTE = if (*dctx).litBufferLocation == ZSTD_in_dst {
        (*dctx).litBuffer
    } else {
        ZSTD_maybeNullPtrAdd(ostart, maxDstSize as isize)
    };
    let mut op: *mut BYTE = ostart;
    let mut litPtr: *const BYTE = (*dctx).litPtr;
    let mut litBufferEnd: *const BYTE = (*dctx).litBufferEnd;
    let prefixStart: *const BYTE = (*dctx).prefixStart as *const BYTE;
    let dictStart: *const BYTE = (*dctx).virtualStart as *const BYTE;
    let dictEnd: *const BYTE = (*dctx).dictEnd as *const BYTE;

    /* Regen sequences */
    if nbSeq != 0 {
        let mut sequences: [seq_t; STORED_SEQS] = [seq_t {
            litLength: 0,
            matchLength: 0,
            offset: 0,
        }; STORED_SEQS];
        let seqAdvance: c_int = if nbSeq < ADVANCED_SEQS {
            nbSeq
        } else {
            ADVANCED_SEQS
        };
        let mut seqState = seqState_t::default();
        let mut seqNb: c_int;
        /* track position relative to prefixStart */
        let mut prefetchPos: usize = (op as usize).wrapping_sub(prefixStart as usize);

        (*dctx).fseEntropy = 1;
        {
            let mut i: c_int = 0;
            while (i as usize) < ZSTD_REP_NUM {
                seqState.prevOffset[i as usize] = (*dctx).entropy.rep[i as usize] as usize;
                i += 1;
            }
        }
        if ERR_isError(BIT_initDStream(
            core::ptr::addr_of_mut!(seqState.DStream),
            ip as *const c_void,
            (iend as usize).wrapping_sub(ip as usize),
        )) != 0
        {
            return ERROR(ZSTD_error_corruption_detected);
        }
        ZSTD_initFseState(
            core::ptr::addr_of_mut!(seqState.stateLL),
            core::ptr::addr_of_mut!(seqState.DStream),
            (*dctx).LLTptr,
        );
        ZSTD_initFseState(
            core::ptr::addr_of_mut!(seqState.stateOffb),
            core::ptr::addr_of_mut!(seqState.DStream),
            (*dctx).OFTptr,
        );
        ZSTD_initFseState(
            core::ptr::addr_of_mut!(seqState.stateML),
            core::ptr::addr_of_mut!(seqState.DStream),
            (*dctx).MLTptr,
        );

        /* prepare in advance */
        seqNb = 0;
        while seqNb < seqAdvance {
            let sequence: seq_t = ZSTD_decodeSequence(
                core::ptr::addr_of_mut!(seqState),
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
                core::ptr::addr_of_mut!(seqState),
                isLongOffset,
                (seqNb == nbSeq - 1) as c_int,
            );
            let slot: usize = ((seqNb - ADVANCED_SEQS) & STORED_SEQS_MASK) as usize;

            if (*dctx).litBufferLocation == ZSTD_split
                && (litPtr as usize).wrapping_add(sequences[slot].litLength)
                    > (*dctx).litBufferEnd as usize
            {
                /* lit buffer is reaching split point, empty out the first buffer and
                 * transition to litExtraBuffer */
                let leftoverLit: usize =
                    ((*dctx).litBufferEnd as usize).wrapping_sub(litPtr as usize);
                if leftoverLit != 0 {
                    if leftoverLit > (oend as usize).wrapping_sub(op as usize) {
                        return ERROR(ZSTD_error_dstSize_tooSmall);
                    }
                    ZSTD_safecopyDstBeforeSrc(op, litPtr, leftoverLit as isize);
                    sequences[slot].litLength =
                        sequences[slot].litLength.wrapping_sub(leftoverLit);
                    op = op.add(leftoverLit);
                }
                litPtr = core::ptr::addr_of!((*dctx).litExtraBuffer) as *const BYTE;
                litBufferEnd = (core::ptr::addr_of!((*dctx).litExtraBuffer) as usize)
                    .wrapping_add(ZSTD_LITBUFFEREXTRASIZE) as *const BYTE;
                (*dctx).litBufferLocation = ZSTD_not_in_dst;
                {
                    let oneSeqSize: usize = ZSTD_execSequence(
                        op,
                        oend,
                        sequences[slot],
                        core::ptr::addr_of_mut!(litPtr),
                        litBufferEnd,
                        prefixStart,
                        dictStart,
                        dictEnd,
                    );
                    if ZSTD_isError(oneSeqSize) != 0 {
                        return oneSeqSize;
                    }

                    prefetchPos =
                        ZSTD_prefetchMatch(prefetchPos, sequence, prefixStart, dictEnd);
                    sequences[(seqNb & STORED_SEQS_MASK) as usize] = sequence;
                    op = op.add(oneSeqSize);
                }
            } else {
                /* lit buffer is either wholly contained in first or second split, or not
                 * split at all */
                let oneSeqSize: usize = if (*dctx).litBufferLocation == ZSTD_split {
                    ZSTD_execSequenceSplitLitBuffer(
                        op,
                        oend,
                        (litPtr as usize)
                            .wrapping_add(sequences[slot].litLength)
                            .wrapping_sub(WILDCOPY_OVERLENGTH) as *const BYTE,
                        sequences[slot],
                        core::ptr::addr_of_mut!(litPtr),
                        litBufferEnd,
                        prefixStart,
                        dictStart,
                        dictEnd,
                    )
                } else {
                    ZSTD_execSequence(
                        op,
                        oend,
                        sequences[slot],
                        core::ptr::addr_of_mut!(litPtr),
                        litBufferEnd,
                        prefixStart,
                        dictStart,
                        dictEnd,
                    )
                };
                if ZSTD_isError(oneSeqSize) != 0 {
                    return oneSeqSize;
                }

                prefetchPos = ZSTD_prefetchMatch(prefetchPos, sequence, prefixStart, dictEnd);
                sequences[(seqNb & STORED_SEQS_MASK) as usize] = sequence;
                op = op.add(oneSeqSize);
            }
            seqNb += 1;
        }
        if BIT_endOfDStream(core::ptr::addr_of!(seqState.DStream)) == 0 {
            return ERROR(ZSTD_error_corruption_detected);
        }

        /* finish queue */
        seqNb -= seqAdvance;
        while seqNb < nbSeq {
            let slot: usize = (seqNb & STORED_SEQS_MASK) as usize;
            let sequence: *mut seq_t = core::ptr::addr_of_mut!(sequences[slot]);
            if (*dctx).litBufferLocation == ZSTD_split
                && (litPtr as usize).wrapping_add((*sequence).litLength)
                    > (*dctx).litBufferEnd as usize
            {
                let leftoverLit: usize =
                    ((*dctx).litBufferEnd as usize).wrapping_sub(litPtr as usize);
                if leftoverLit != 0 {
                    if leftoverLit > (oend as usize).wrapping_sub(op as usize) {
                        return ERROR(ZSTD_error_dstSize_tooSmall);
                    }
                    ZSTD_safecopyDstBeforeSrc(op, litPtr, leftoverLit as isize);
                    (*sequence).litLength = (*sequence).litLength.wrapping_sub(leftoverLit);
                    op = op.add(leftoverLit);
                }
                litPtr = core::ptr::addr_of!((*dctx).litExtraBuffer) as *const BYTE;
                litBufferEnd = (core::ptr::addr_of!((*dctx).litExtraBuffer) as usize)
                    .wrapping_add(ZSTD_LITBUFFEREXTRASIZE) as *const BYTE;
                (*dctx).litBufferLocation = ZSTD_not_in_dst;
                {
                    let oneSeqSize: usize = ZSTD_execSequence(
                        op,
                        oend,
                        *sequence,
                        core::ptr::addr_of_mut!(litPtr),
                        litBufferEnd,
                        prefixStart,
                        dictStart,
                        dictEnd,
                    );
                    if ZSTD_isError(oneSeqSize) != 0 {
                        return oneSeqSize;
                    }
                    op = op.add(oneSeqSize);
                }
            } else {
                let oneSeqSize: usize = if (*dctx).litBufferLocation == ZSTD_split {
                    ZSTD_execSequenceSplitLitBuffer(
                        op,
                        oend,
                        (litPtr as usize)
                            .wrapping_add((*sequence).litLength)
                            .wrapping_sub(WILDCOPY_OVERLENGTH) as *const BYTE,
                        *sequence,
                        core::ptr::addr_of_mut!(litPtr),
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
                        core::ptr::addr_of_mut!(litPtr),
                        litBufferEnd,
                        prefixStart,
                        dictStart,
                        dictEnd,
                    )
                };
                if ZSTD_isError(oneSeqSize) != 0 {
                    return oneSeqSize;
                }
                op = op.add(oneSeqSize);
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
        let lastLLSize: usize = (litBufferEnd as usize).wrapping_sub(litPtr as usize);
        if lastLLSize > (oend as usize).wrapping_sub(op as usize) {
            return ERROR(ZSTD_error_dstSize_tooSmall);
        }
        if !op.is_null() {
            ZSTD_memmove(op as *mut c_void, litPtr as *const c_void, lastLLSize);
            op = op.add(lastLLSize);
        }
        litPtr = core::ptr::addr_of!((*dctx).litExtraBuffer) as *const BYTE;
        litBufferEnd = (core::ptr::addr_of!((*dctx).litExtraBuffer) as usize)
            .wrapping_add(ZSTD_LITBUFFEREXTRASIZE) as *const BYTE;
    }
    {
        let lastLLSize: usize = (litBufferEnd as usize).wrapping_sub(litPtr as usize);
        if lastLLSize > (oend as usize).wrapping_sub(op as usize) {
            return ERROR(ZSTD_error_dstSize_tooSmall);
        }
        if !op.is_null() {
            ZSTD_memmove(op as *mut c_void, litPtr as *const c_void, lastLLSize);
            op = op.add(lastLLSize);
        }
    }

    (op as usize).wrapping_sub(ostart as usize)
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

/* DYNAMIC_BMI2 == 0 : the *_bmi2() variants are not compiled in. */

unsafe fn ZSTD_decompressSequences(
    dctx: *mut ZSTD_DCtx,
    dst: *mut c_void,
    maxDstSize: usize,
    seqStart: *const c_void,
    seqSize: usize,
    nbSeq: c_int,
    isLongOffset: ZSTD_longOffset_e,
) -> usize {
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

/* ZSTD_decompressSequencesLong() :
 * decompression function triggered when a minimum share of offsets is considered "long". */
unsafe fn ZSTD_decompressSequencesLong(
    dctx: *mut ZSTD_DCtx,
    dst: *mut c_void,
    maxDstSize: usize,
    seqStart: *const c_void,
    seqSize: usize,
    nbSeq: c_int,
    isLongOffset: ZSTD_longOffset_e,
) -> usize {
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

/**
 * @returns The total size of the history referenceable by zstd, including
 * both the prefix and the extDict.
 */
unsafe fn ZSTD_totalHistorySize(op: *mut BYTE, virtualStart: *const BYTE) -> usize {
    (op as usize).wrapping_sub(virtualStart as usize)
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct ZSTD_OffsetInfo {
    pub longOffsetShare: c_uint,
    pub maxNbAdditionalBits: c_uint,
}

/* ZSTD_getOffsetInfo() :
 * condition : offTable must be valid */
unsafe fn ZSTD_getOffsetInfo(offTable: *const ZSTD_seqSymbol, nbSeq: c_int) -> ZSTD_OffsetInfo {
    let mut info = ZSTD_OffsetInfo {
        longOffsetShare: 0,
        maxNbAdditionalBits: 0,
    };
    /* If nbSeq == 0, then the offTable is uninitialized, but we have
     * no sequences, so both values should be 0. */
    if nbSeq != 0 {
        let ptr: *const c_void = offTable as *const c_void;
        let tableLog: U32 = (*(ptr as *const ZSTD_seqSymbol_header).add(0)).tableLog;
        let table: *const ZSTD_seqSymbol = offTable.add(1);
        let max: U32 = 1u32 << tableLog;
        let mut u: U32 = 0;
        while u < max {
            let nab: c_uint = (*table.add(u as usize)).nbAdditionalBits as c_uint;
            info.maxNbAdditionalBits = if info.maxNbAdditionalBits > nab {
                info.maxNbAdditionalBits
            } else {
                nab
            };
            if (*table.add(u as usize)).nbAdditionalBits > 22 {
                info.longOffsetShare += 1;
            }
            u += 1;
        }

        info.longOffsetShare <<= OffFSELog - tableLog; /* scale to OffFSELog */
    }

    info
}

/**
 * @returns The maximum offset we can decode in one read of our bitstream, without
 * reloading more bits in the middle of the offset bits read.
 */
unsafe fn ZSTD_maxShortOffset() -> usize {
    if MEM_64bits() != 0 {
        /* We can decode any offset without reloading bits. */
        (0usize).wrapping_sub(1)
    } else {
        let maxOffbase: usize = ((1usize << (STREAM_ACCUMULATOR_MIN() + 1))).wrapping_sub(1);
        let maxOffset: usize = maxOffbase.wrapping_sub(ZSTD_REP_NUM);
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
    let mut ip: *const BYTE = src as *const BYTE;

    if srcSize > ZSTD_blockSizeMax(dctx) {
        return ERROR(ZSTD_error_srcSize_wrong);
    }

    /* Decode literals section */
    {
        let litCSize: usize =
            ZSTD_decodeLiteralsBlock(dctx, src, srcSize, dst, dstCapacity, streaming);
        if ZSTD_isError(litCSize) != 0 {
            return litCSize;
        }
        ip = ip.add(litCSize);
        srcSize = srcSize.wrapping_sub(litCSize);
    }

    /* Build Decoding Tables */
    {
        let blockSizeMaxDctx = ZSTD_blockSizeMax(dctx);
        let blockSizeMax: usize = if dstCapacity < blockSizeMaxDctx {
            dstCapacity
        } else {
            blockSizeMaxDctx
        };
        let totalHistorySize: usize = ZSTD_totalHistorySize(
            ZSTD_maybeNullPtrAdd(dst as *mut BYTE, blockSizeMax as isize),
            (*dctx).virtualStart as *const BYTE,
        );
        /* isLongOffset must be true if there are long offsets. */
        let mut isLongOffset: ZSTD_longOffset_e =
            (MEM_32bits() != 0 && (totalHistorySize > ZSTD_maxShortOffset()))
                as ZSTD_longOffset_e;
        let mut usePrefetchDecoder: c_int = (*dctx).ddictIsCold;
        let mut nbSeq: c_int = 0;
        let seqHSize: usize =
            ZSTD_decodeSeqHeaders(dctx, &mut nbSeq as *mut c_int, ip as *const c_void, srcSize);
        if ZSTD_isError(seqHSize) != 0 {
            return seqHSize;
        }
        ip = ip.add(seqHSize);
        srcSize = srcSize.wrapping_sub(seqHSize);

        if (dst.is_null() || dstCapacity == 0) && nbSeq > 0 {
            return ERROR(ZSTD_error_dstSize_tooSmall);
        }
        if MEM_64bits() != 0
            && core::mem::size_of::<usize>() == core::mem::size_of::<*const c_void>()
            && ((0usize).wrapping_sub(1)).wrapping_sub(dst as usize) < (1usize << 20)
        {
            return ERROR(ZSTD_error_dstSize_tooSmall);
        }

        /* If we could potentially have long offsets, or we might want to use the prefetch
         * decoder, compute information about the share of long offsets. */
        if isLongOffset != 0
            || (usePrefetchDecoder == 0
                && (totalHistorySize > (1u32 << 24) as usize)
                && (nbSeq > 8))
        {
            let info: ZSTD_OffsetInfo = ZSTD_getOffsetInfo((*dctx).OFTptr, nbSeq);
            if isLongOffset != 0 && info.maxNbAdditionalBits <= STREAM_ACCUMULATOR_MIN() {
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
            return ZSTD_decompressSequencesSplitLitBuffer(
                dctx,
                dst,
                dstCapacity,
                ip as *const c_void,
                srcSize,
                nbSeq,
                isLongOffset,
            );
        } else {
            return ZSTD_decompressSequences(
                dctx,
                dst,
                dstCapacity,
                ip as *const c_void,
                srcSize,
                nbSeq,
                isLongOffset,
            );
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
        (*dctx).virtualStart = (dst as *const c_char as usize).wrapping_sub(
            ((*dctx).previousDstEnd as *const c_char as usize)
                .wrapping_sub((*dctx).prefixStart as *const c_char as usize),
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
    (*dctx).previousDstEnd = (dst as *mut c_char as usize).wrapping_add(dSize) as *const c_void;
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
