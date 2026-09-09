//! zstd_decompress_block : this module takes care of decompressing _compressed_ block.
//!
//! Build configuration: DYNAMIC_BMI2=0, no assembly, DEBUGLEVEL 0,
//! no FUZZING_BUILD_MODE_UNSAFE_FOR_PRODUCTION.
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(dead_code)]

use core::ffi::{c_int, c_uint, c_void};

use crate::common::bits::*;
use crate::common::bitstream::*;
use crate::common::error_private::*;
use crate::common::fse::*;
use crate::common::huf::*;
use crate::common::mem::*;
use crate::common::zstd_h::*;
use crate::common::zstd_internal::*;

use crate::common::entropy_common::{FSE_isError, FSE_readNCount, HUF_isError};
use crate::common::zstd_common::ZSTD_isError;

use crate::decompress::huf_decompress::{
    HUF_decompress1X1_DCtx_wksp, HUF_decompress1X_usingDTable, HUF_decompress4X_hufOnly_wksp,
    HUF_decompress4X_usingDTable,
};
use crate::decompress::zstd_decompress_internal::*;

/* streaming_operation : from zstd_decompress_block.h */
pub type streaming_operation = c_uint;
pub const not_streaming: streaming_operation = 0;
pub const is_streaming: streaming_operation = 1;

/* ===  compiler.h helpers  === */
pub const CACHELINE_SIZE: size_t = 64;

#[inline(always)]
pub unsafe fn ZSTD_wrappedPtrAdd(ptr: *const BYTE, add: isize) -> *const BYTE {
    ptr.wrapping_offset(add)
}

#[inline(always)]
pub unsafe fn ZSTD_wrappedPtrSub(ptr: *const BYTE, sub: isize) -> *const BYTE {
    ptr.wrapping_offset(-sub)
}

#[inline(always)]
pub unsafe fn ZSTD_maybeNullPtrAdd(ptr: *mut BYTE, add: isize) -> *mut BYTE {
    if add > 0 {
        ptr.offset(add)
    } else {
        ptr
    }
}

/*_*******************************************************
*  Memory operations
**********************************************************/
pub unsafe fn ZSTD_copy4(dst: *mut c_void, src: *const c_void) {
    ZSTD_memcpy(dst as *mut u8, src as *const u8, 4);
}

/*-*************************************************************
 *   Block decoding
 ***************************************************************/

pub unsafe fn ZSTD_blockSizeMax(dctx: *const ZSTD_DCtx) -> size_t {
    let blockSizeMax: size_t = if (*dctx).isFrameDecompression != 0 {
        (*dctx).fParams.blockSizeMax as size_t
    } else {
        ZSTD_BLOCKSIZE_MAX as size_t
    };
    blockSizeMax
}

/* ZSTD_getcBlockSize() :
 *  Provides the size of compressed block from block header `src` */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_getcBlockSize(
    src: *const c_void,
    srcSize: size_t,
    bpPtr: *mut blockProperties_t,
) -> size_t {
    if srcSize < ZSTD_blockHeaderSize {
        return ERROR(ZSTD_error_srcSize_wrong);
    }

    {
        let cBlockHeader: U32 = MEM_readLE24(src as *const u8);
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
        cSize as size_t
    }
}

/* Allocate buffer for literals, either overlapping current dst, or split between dst and litExtraBuffer, or stored entirely within litExtraBuffer */
pub unsafe fn ZSTD_allocateLiteralsBuffer(
    dctx: *mut ZSTD_DCtx,
    dst: *mut c_void,
    dstCapacity: size_t,
    litSize: size_t,
    streaming: streaming_operation,
    expectedWriteSize: size_t,
    splitImmediately: c_uint,
) {
    let blockSizeMax: size_t = ZSTD_blockSizeMax(dctx);
    if streaming == not_streaming
        && dstCapacity
            > blockSizeMax + WILDCOPY_OVERLENGTH as size_t + litSize + WILDCOPY_OVERLENGTH as size_t
    {
        (*dctx).litBuffer =
            (dst as *mut BYTE).add(blockSizeMax + WILDCOPY_OVERLENGTH as size_t);
        (*dctx).litBufferEnd = (*dctx).litBuffer.add(litSize);
        (*dctx).litBufferLocation = ZSTD_in_dst;
    } else if litSize <= ZSTD_LITBUFFEREXTRASIZE {
        (*dctx).litBuffer = (*dctx).litExtraBuffer.as_mut_ptr();
        (*dctx).litBufferEnd = (*dctx).litBuffer.add(litSize);
        (*dctx).litBufferLocation = ZSTD_not_in_dst;
    } else {
        if splitImmediately != 0 {
            /* won't fit in litExtraBuffer, so it will be split between end of dst and extra buffer */
            (*dctx).litBuffer = (dst as *mut BYTE)
                .add(expectedWriteSize)
                .wrapping_sub(litSize)
                .add(ZSTD_LITBUFFEREXTRASIZE)
                .wrapping_sub(WILDCOPY_OVERLENGTH as size_t);
            (*dctx).litBufferEnd =
                (*dctx).litBuffer.add(litSize).wrapping_sub(ZSTD_LITBUFFEREXTRASIZE);
        } else {
            /* initially this will be stored entirely in dst during huffman decoding, it will partially be shifted to litExtraBuffer after */
            (*dctx).litBuffer =
                (dst as *mut BYTE).add(expectedWriteSize).wrapping_sub(litSize);
            (*dctx).litBufferEnd = (dst as *mut BYTE).add(expectedWriteSize);
        }
        (*dctx).litBufferLocation = ZSTD_split;
    }
}

/* ZSTD_decodeLiteralsBlock() :
 * @return : nb of bytes read from src (< srcSize )
 *  note : symbol not declared but exposed for fullbench */
pub unsafe fn ZSTD_decodeLiteralsBlock(
    dctx: *mut ZSTD_DCtx,
    src: *const c_void,
    srcSize: size_t,
    dst: *mut c_void,
    dstCapacity: size_t,
    streaming: streaming_operation,
) -> size_t {
    if srcSize < MIN_CBLOCK_SIZE {
        return ERROR(ZSTD_error_corruption_detected);
    }

    {
        let istart: *const BYTE = src as *const BYTE;
        let litEncType: SymbolEncodingType_e = (*istart.add(0) & 3) as SymbolEncodingType_e;
        let blockSizeMax: size_t = ZSTD_blockSizeMax(dctx);

        // switch(litEncType)  --  set_repeat falls through into set_compressed
        if litEncType == set_repeat || litEncType == set_compressed {
            if litEncType == set_repeat {
                if (*dctx).litEntropy == 0 {
                    return ERROR(ZSTD_error_dictionary_corrupted);
                }
                /* ZSTD_FALLTHROUGH into set_compressed */
            }

            /* case set_compressed: */
            if srcSize < 5 {
                return ERROR(ZSTD_error_corruption_detected);
            }
            {
                let lhSize: size_t;
                let litSize: size_t;
                let litCSize: size_t;
                let mut singleStream: U32 = 0;
                let lhlCode: U32 = ((*istart.add(0) >> 2) & 3) as U32;
                let lhc: U32 = MEM_readLE32(istart);
                let hufSuccess: size_t;
                let expectedWriteSize: size_t = MIN(blockSizeMax, dstCapacity);
                let flags: c_int = 0
                    | (if ZSTD_DCtx_get_bmi2(dctx) != 0 {
                        HUF_flags_bmi2 as c_int
                    } else {
                        0
                    })
                    | (if (*dctx).disableHufAsm != 0 {
                        HUF_flags_disableAsm as c_int
                    } else {
                        0
                    });
                match lhlCode {
                    2 => {
                        /* 2 - 2 - 14 - 14 */
                        lhSize = 4;
                        litSize = ((lhc >> 4) & 0x3FFF) as size_t;
                        litCSize = (lhc >> 18) as size_t;
                    }
                    3 => {
                        /* 2 - 2 - 18 - 18 */
                        lhSize = 5;
                        litSize = ((lhc >> 4) & 0x3FFFF) as size_t;
                        litCSize =
                            (lhc >> 22) as size_t + ((*istart.add(4) as size_t) << 10);
                    }
                    _ => {
                        /* case 0: case 1: default: 2 - 2 - 10 - 10 */
                        singleStream = (lhlCode == 0) as U32;
                        lhSize = 3;
                        litSize = ((lhc >> 4) & 0x3FF) as size_t;
                        litCSize = ((lhc >> 14) & 0x3FF) as size_t;
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

                /* prefetch huffman table if cold : PREFETCH_AREA is a no-op prefetch, dropped */

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
                        (*dctx).litExtraBuffer.as_mut_ptr(),
                        ((*dctx).litBufferEnd as *const BYTE)
                            .wrapping_sub(ZSTD_LITBUFFEREXTRASIZE),
                        ZSTD_LITBUFFEREXTRASIZE,
                    );
                    ZSTD_memmove(
                        (*dctx)
                            .litBuffer
                            .add(ZSTD_LITBUFFEREXTRASIZE)
                            .wrapping_sub(WILDCOPY_OVERLENGTH as size_t),
                        (*dctx).litBuffer,
                        litSize - ZSTD_LITBUFFEREXTRASIZE,
                    );
                    (*dctx).litBuffer = (*dctx)
                        .litBuffer
                        .add(ZSTD_LITBUFFEREXTRASIZE)
                        .wrapping_sub(WILDCOPY_OVERLENGTH as size_t);
                    (*dctx).litBufferEnd = ((*dctx).litBufferEnd as *const BYTE)
                        .wrapping_sub(WILDCOPY_OVERLENGTH as size_t);
                }

                if HUF_isError(hufSuccess) != 0 {
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
        } else if litEncType == set_basic {
            {
                let litSize: size_t;
                let lhSize: size_t;
                let lhlCode: U32 = ((*istart.add(0) >> 2) & 3) as U32;
                let expectedWriteSize: size_t = MIN(blockSizeMax, dstCapacity);
                match lhlCode {
                    1 => {
                        lhSize = 2;
                        litSize = (MEM_readLE16(istart) >> 4) as size_t;
                    }
                    3 => {
                        lhSize = 3;
                        if srcSize < 3 {
                            return ERROR(ZSTD_error_corruption_detected);
                        }
                        litSize = (MEM_readLE24(istart) >> 4) as size_t;
                    }
                    _ => {
                        /* case 0: case 2: default: */
                        lhSize = 1;
                        litSize = (*istart.add(0) >> 3) as size_t;
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
                if lhSize + litSize + WILDCOPY_OVERLENGTH as size_t > srcSize {
                    /* risk reading beyond src buffer with wildcopy */
                    if litSize + lhSize > srcSize {
                        return ERROR(ZSTD_error_corruption_detected);
                    }
                    if (*dctx).litBufferLocation == ZSTD_split {
                        ZSTD_memcpy(
                            (*dctx).litBuffer,
                            istart.add(lhSize),
                            litSize - ZSTD_LITBUFFEREXTRASIZE,
                        );
                        ZSTD_memcpy(
                            (*dctx).litExtraBuffer.as_mut_ptr(),
                            istart.add(lhSize).add(litSize).wrapping_sub(ZSTD_LITBUFFEREXTRASIZE),
                            ZSTD_LITBUFFEREXTRASIZE,
                        );
                    } else {
                        ZSTD_memcpy((*dctx).litBuffer, istart.add(lhSize), litSize);
                    }
                    (*dctx).litPtr = (*dctx).litBuffer;
                    (*dctx).litSize = litSize;
                    return lhSize + litSize;
                }
                /* direct reference into compressed stream */
                (*dctx).litPtr = istart.add(lhSize);
                (*dctx).litSize = litSize;
                (*dctx).litBufferEnd = ((*dctx).litPtr as *const BYTE).add(litSize);
                (*dctx).litBufferLocation = ZSTD_not_in_dst;
                return lhSize + litSize;
            }
        } else if litEncType == set_rle {
            {
                let lhlCode: U32 = ((*istart.add(0) >> 2) & 3) as U32;
                let litSize: size_t;
                let lhSize: size_t;
                let expectedWriteSize: size_t = MIN(blockSizeMax, dstCapacity);
                match lhlCode {
                    1 => {
                        lhSize = 2;
                        if srcSize < 3 {
                            return ERROR(ZSTD_error_corruption_detected);
                        }
                        litSize = (MEM_readLE16(istart) >> 4) as size_t;
                    }
                    3 => {
                        lhSize = 3;
                        if srcSize < 4 {
                            return ERROR(ZSTD_error_corruption_detected);
                        }
                        litSize = (MEM_readLE24(istart) >> 4) as size_t;
                    }
                    _ => {
                        /* case 0: case 2: default: */
                        lhSize = 1;
                        litSize = (*istart.add(0) >> 3) as size_t;
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
                        (*dctx).litBuffer,
                        *istart.add(lhSize) as c_int,
                        litSize - ZSTD_LITBUFFEREXTRASIZE,
                    );
                    ZSTD_memset(
                        (*dctx).litExtraBuffer.as_mut_ptr(),
                        *istart.add(lhSize) as c_int,
                        ZSTD_LITBUFFEREXTRASIZE,
                    );
                } else {
                    ZSTD_memset((*dctx).litBuffer, *istart.add(lhSize) as c_int, litSize);
                }
                (*dctx).litPtr = (*dctx).litBuffer;
                (*dctx).litSize = litSize;
                return lhSize + 1;
            }
        } else {
            /* default: */
            return ERROR(ZSTD_error_corruption_detected);
        }
    }
}

/* Hidden declaration for fullbench */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_decodeLiteralsBlock_wrapper(
    dctx: *mut ZSTD_DCtx,
    src: *const c_void,
    srcSize: size_t,
    dst: *mut c_void,
    dstCapacity: size_t,
) -> size_t {
    (*dctx).isFrameDecompression = 0;
    ZSTD_decodeLiteralsBlock(dctx, src, srcSize, dst, dstCapacity, not_streaming)
}

/* Helper to build a ZSTD_seqSymbol literal from (nextState, nbAdditionalBits, nbBits, baseValue). */
const fn sq(nextState: U16, nbAdditionalBits: BYTE, nbBits: BYTE, baseValue: U32) -> ZSTD_seqSymbol {
    ZSTD_seqSymbol {
        nextState,
        nbAdditionalBits,
        nbBits,
        baseValue,
    }
}

/* Default FSE distribution table for Literal Lengths */
static LL_defaultDTable: [ZSTD_seqSymbol; (1 << LL_DEFAULTNORMLOG) + 1] = [
    sq(1, 1, 1, LL_DEFAULTNORMLOG as U32), /* header : fastMode, tableLog */
    sq(0, 0, 4, 0), sq(16, 0, 4, 0),
    sq(32, 0, 5, 1), sq(0, 0, 5, 3),
    sq(0, 0, 5, 4), sq(0, 0, 5, 6),
    sq(0, 0, 5, 7), sq(0, 0, 5, 9),
    sq(0, 0, 5, 10), sq(0, 0, 5, 12),
    sq(0, 0, 6, 14), sq(0, 1, 5, 16),
    sq(0, 1, 5, 20), sq(0, 1, 5, 22),
    sq(0, 2, 5, 28), sq(0, 3, 5, 32),
    sq(0, 4, 5, 48), sq(32, 6, 5, 64),
    sq(0, 7, 5, 128), sq(0, 8, 6, 256),
    sq(0, 10, 6, 1024), sq(0, 12, 6, 4096),
    sq(32, 0, 4, 0), sq(0, 0, 4, 1),
    sq(0, 0, 5, 2), sq(32, 0, 5, 4),
    sq(0, 0, 5, 5), sq(32, 0, 5, 7),
    sq(0, 0, 5, 8), sq(32, 0, 5, 10),
    sq(0, 0, 5, 11), sq(0, 0, 6, 13),
    sq(32, 1, 5, 16), sq(0, 1, 5, 18),
    sq(32, 1, 5, 22), sq(0, 2, 5, 24),
    sq(32, 3, 5, 32), sq(0, 3, 5, 40),
    sq(0, 6, 4, 64), sq(16, 6, 4, 64),
    sq(32, 7, 5, 128), sq(0, 9, 6, 512),
    sq(0, 11, 6, 2048), sq(48, 0, 4, 0),
    sq(16, 0, 4, 1), sq(32, 0, 5, 2),
    sq(32, 0, 5, 3), sq(32, 0, 5, 5),
    sq(32, 0, 5, 6), sq(32, 0, 5, 8),
    sq(32, 0, 5, 9), sq(32, 0, 5, 11),
    sq(32, 0, 5, 12), sq(0, 0, 6, 15),
    sq(32, 1, 5, 18), sq(32, 1, 5, 20),
    sq(32, 2, 5, 24), sq(32, 2, 5, 28),
    sq(32, 3, 5, 40), sq(32, 4, 5, 48),
    sq(0, 16, 6, 65536), sq(0, 15, 6, 32768),
    sq(0, 14, 6, 16384), sq(0, 13, 6, 8192),
]; /* LL_defaultDTable */

/* Default FSE distribution table for Offset Codes */
static OF_defaultDTable: [ZSTD_seqSymbol; (1 << OF_DEFAULTNORMLOG) + 1] = [
    sq(1, 1, 1, OF_DEFAULTNORMLOG as U32), /* header : fastMode, tableLog */
    sq(0, 0, 5, 0), sq(0, 6, 4, 61),
    sq(0, 9, 5, 509), sq(0, 15, 5, 32765),
    sq(0, 21, 5, 2097149), sq(0, 3, 5, 5),
    sq(0, 7, 4, 125), sq(0, 12, 5, 4093),
    sq(0, 18, 5, 262141), sq(0, 23, 5, 8388605),
    sq(0, 5, 5, 29), sq(0, 8, 4, 253),
    sq(0, 14, 5, 16381), sq(0, 20, 5, 1048573),
    sq(0, 2, 5, 1), sq(16, 7, 4, 125),
    sq(0, 11, 5, 2045), sq(0, 17, 5, 131069),
    sq(0, 22, 5, 4194301), sq(0, 4, 5, 13),
    sq(16, 8, 4, 253), sq(0, 13, 5, 8189),
    sq(0, 19, 5, 524285), sq(0, 1, 5, 1),
    sq(16, 6, 4, 61), sq(0, 10, 5, 1021),
    sq(0, 16, 5, 65533), sq(0, 28, 5, 268435453),
    sq(0, 27, 5, 134217725), sq(0, 26, 5, 67108861),
    sq(0, 25, 5, 33554429), sq(0, 24, 5, 16777213),
]; /* OF_defaultDTable */

/* Default FSE distribution table for Match Lengths */
static ML_defaultDTable: [ZSTD_seqSymbol; (1 << ML_DEFAULTNORMLOG) + 1] = [
    sq(1, 1, 1, ML_DEFAULTNORMLOG as U32), /* header : fastMode, tableLog */
    sq(0, 0, 6, 3), sq(0, 0, 4, 4),
    sq(32, 0, 5, 5), sq(0, 0, 5, 6),
    sq(0, 0, 5, 8), sq(0, 0, 5, 9),
    sq(0, 0, 5, 11), sq(0, 0, 6, 13),
    sq(0, 0, 6, 16), sq(0, 0, 6, 19),
    sq(0, 0, 6, 22), sq(0, 0, 6, 25),
    sq(0, 0, 6, 28), sq(0, 0, 6, 31),
    sq(0, 0, 6, 34), sq(0, 1, 6, 37),
    sq(0, 1, 6, 41), sq(0, 2, 6, 47),
    sq(0, 3, 6, 59), sq(0, 4, 6, 83),
    sq(0, 7, 6, 131), sq(0, 9, 6, 515),
    sq(16, 0, 4, 4), sq(0, 0, 4, 5),
    sq(32, 0, 5, 6), sq(0, 0, 5, 7),
    sq(32, 0, 5, 9), sq(0, 0, 5, 10),
    sq(0, 0, 6, 12), sq(0, 0, 6, 15),
    sq(0, 0, 6, 18), sq(0, 0, 6, 21),
    sq(0, 0, 6, 24), sq(0, 0, 6, 27),
    sq(0, 0, 6, 30), sq(0, 0, 6, 33),
    sq(0, 1, 6, 35), sq(0, 1, 6, 39),
    sq(0, 2, 6, 43), sq(0, 3, 6, 51),
    sq(0, 4, 6, 67), sq(0, 5, 6, 99),
    sq(0, 8, 6, 259), sq(32, 0, 4, 4),
    sq(48, 0, 4, 4), sq(16, 0, 4, 5),
    sq(32, 0, 5, 7), sq(32, 0, 5, 8),
    sq(32, 0, 5, 10), sq(32, 0, 5, 11),
    sq(0, 0, 6, 14), sq(0, 0, 6, 17),
    sq(0, 0, 6, 20), sq(0, 0, 6, 23),
    sq(0, 0, 6, 26), sq(0, 0, 6, 29),
    sq(0, 0, 6, 32), sq(0, 16, 6, 65539),
    sq(0, 15, 6, 32771), sq(0, 14, 6, 16387),
    sq(0, 13, 6, 8195), sq(0, 12, 6, 4099),
    sq(0, 11, 6, 2051), sq(0, 10, 6, 1027),
]; /* ML_defaultDTable */

pub unsafe fn ZSTD_buildSeqTable_rle(dt: *mut ZSTD_seqSymbol, baseValue: U32, nbAddBits: U8) {
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
 * generate FSE decoding table for one symbol (ll, ml or off)
 * cannot fail if input is valid =>
 * all inputs are presumed validated at this stage */
pub unsafe fn ZSTD_buildFSETable_body(
    dt: *mut ZSTD_seqSymbol,
    normalizedCounter: *const S16,
    maxSymbolValue: c_uint,
    baseValue: *const U32,
    nbAdditionalBits: *const U8,
    tableLog: c_uint,
    wksp: *mut c_void,
    wkspSize: size_t,
) {
    let tableDecode: *mut ZSTD_seqSymbol = dt.add(1);
    let maxSV1: U32 = maxSymbolValue + 1;
    let tableSize: U32 = 1 << tableLog;

    let symbolNext: *mut U16 = wksp as *mut U16;
    let spread: *mut BYTE = symbolNext.add(MaxSeq as usize + 1) as *mut BYTE;
    let mut highThreshold: U32 = tableSize - 1;

    let _ = wkspSize;
    /* Init, lay down lowprob symbols */
    {
        let mut DTableH: ZSTD_seqSymbol_header = ZSTD_seqSymbol_header {
            fastMode: 0,
            tableLog: 0,
        };
        DTableH.tableLog = tableLog;
        DTableH.fastMode = 1;
        {
            let largeLimit: S16 = (1 << (tableLog - 1)) as S16;
            let mut s: U32 = 0;
            while s < maxSV1 {
                if *normalizedCounter.add(s as usize) == -1 {
                    (*tableDecode.add(highThreshold as usize)).baseValue = s;
                    highThreshold -= 1;
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
            dt as *mut u8,
            &DTableH as *const ZSTD_seqSymbol_header as *const u8,
            core::mem::size_of::<ZSTD_seqSymbol_header>(),
        );
    }

    /* Spread symbols */
    if highThreshold == tableSize - 1 {
        let tableMask: size_t = (tableSize - 1) as size_t;
        let step: size_t = FSE_TABLESTEP(tableSize) as size_t;
        {
            let add: U64 = 0x0101010101010101u64;
            let mut pos: size_t = 0;
            let mut sv: U64 = 0;
            let mut s: U32 = 0;
            while s < maxSV1 {
                let n: c_int = *normalizedCounter.add(s as usize) as c_int;
                MEM_write64(spread.add(pos), sv);
                let mut i: c_int = 8;
                while i < n {
                    MEM_write64(spread.add(pos).offset(i as isize), sv);
                    i += 8;
                }
                pos += n as size_t;
                s += 1;
                sv = sv.wrapping_add(add);
            }
        }
        {
            let mut position: size_t = 0;
            let mut s: size_t = 0;
            let unroll: size_t = 2;
            while s < tableSize as size_t {
                let mut u: size_t = 0;
                while u < unroll {
                    let uPosition: size_t = (position + (u * step)) & tableMask;
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
        let mut s: U32 = 0;
        let mut position: U32 = 0;
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
                (tableLog - ZSTD_highbit32(nextState)) as BYTE;
            (*tableDecode.add(u as usize)).nextState =
                ((nextState << (*tableDecode.add(u as usize)).nbBits) - tableSize) as U16;
            (*tableDecode.add(u as usize)).nbAdditionalBits = *nbAdditionalBits.add(symbol as usize);
            (*tableDecode.add(u as usize)).baseValue = *baseValue.add(symbol as usize);
            u += 1;
        }
    }
}

/* Avoids the FORCE_INLINE of the _body() function. */
pub unsafe fn ZSTD_buildFSETable_body_default(
    dt: *mut ZSTD_seqSymbol,
    normalizedCounter: *const S16,
    maxSymbolValue: c_uint,
    baseValue: *const U32,
    nbAdditionalBits: *const U8,
    tableLog: c_uint,
    wksp: *mut c_void,
    wkspSize: size_t,
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

/* DYNAMIC_BMI2 == 0 : no _bmi2 variant compiled */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_buildFSETable(
    dt: *mut ZSTD_seqSymbol,
    normalizedCounter: *const S16,
    maxSymbolValue: c_uint,
    baseValue: *const U32,
    nbAdditionalBits: *const U8,
    tableLog: c_uint,
    wksp: *mut c_void,
    wkspSize: size_t,
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
 * @return : nb bytes read from src,
 *           or an error code if it fails */
pub unsafe fn ZSTD_buildSeqTable(
    DTableSpace: *mut ZSTD_seqSymbol,
    DTablePtr: *mut *const ZSTD_seqSymbol,
    type_: SymbolEncodingType_e,
    mut max: c_uint,
    maxLog: U32,
    src: *const c_void,
    srcSize: size_t,
    baseValue: *const U32,
    nbAdditionalBits: *const U8,
    defaultTable: *const ZSTD_seqSymbol,
    flagRepeatTable: U32,
    ddictIsCold: c_int,
    nbSeq: c_int,
    wksp: *mut U32,
    wkspSize: size_t,
    bmi2: c_int,
) -> size_t {
    let _ = ddictIsCold;
    let _ = nbSeq;
    match type_ {
        x if x == set_rle => {
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
        x if x == set_basic => {
            *DTablePtr = defaultTable;
            0
        }
        x if x == set_repeat => {
            if flagRepeatTable == 0 {
                return ERROR(ZSTD_error_corruption_detected);
            }
            /* prefetch FSE table if used : PREFETCH_AREA is a no-op prefetch, dropped */
            let _ = maxLog;
            0
        }
        x if x == set_compressed => {
            let mut tableLog: c_uint = 0;
            let mut norm: [S16; MaxSeq as usize + 1] = [0; MaxSeq as usize + 1];
            let headerSize: size_t = FSE_readNCount(
                norm.as_mut_ptr(),
                &mut max,
                &mut tableLog,
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
            headerSize
        }
        _ => {
            /* default: assert(0) dropped */
            ERROR(ZSTD_error_GENERIC)
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_decodeSeqHeaders(
    dctx: *mut ZSTD_DCtx,
    nbSeqPtr: *mut c_int,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    let istart: *const BYTE = src as *const BYTE;
    let iend: *const BYTE = istart.add(srcSize);
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
            if ip.add(2) > iend {
                return ERROR(ZSTD_error_srcSize_wrong);
            }
            nbSeq = MEM_readLE16(ip) as c_int + LONGNBSEQ as c_int;
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
        return ip.offset_from(istart) as size_t;
    }

    /* FSE table descriptors */
    if ip.add(1) > iend {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    if *ip & 3 != 0 {
        return ERROR(ZSTD_error_corruption_detected);
    }
    {
        let LLtype: SymbolEncodingType_e = (*ip >> 6) as SymbolEncodingType_e;
        let OFtype: SymbolEncodingType_e = ((*ip >> 4) & 3) as SymbolEncodingType_e;
        let MLtype: SymbolEncodingType_e = ((*ip >> 2) & 3) as SymbolEncodingType_e;
        ip = ip.add(1);

        /* Build DTables */
        {
            let llhSize: size_t = ZSTD_buildSeqTable(
                (*dctx).entropy.LLTable.as_mut_ptr(),
                &mut (*dctx).LLTptr,
                LLtype,
                MaxLL,
                LLFSELog,
                ip as *const c_void,
                iend.offset_from(ip) as size_t,
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
            if ZSTD_isError(llhSize) != 0 {
                return ERROR(ZSTD_error_corruption_detected);
            }
            ip = ip.add(llhSize);
        }

        {
            let ofhSize: size_t = ZSTD_buildSeqTable(
                (*dctx).entropy.OFTable.as_mut_ptr(),
                &mut (*dctx).OFTptr,
                OFtype,
                MaxOff,
                OffFSELog,
                ip as *const c_void,
                iend.offset_from(ip) as size_t,
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
            if ZSTD_isError(ofhSize) != 0 {
                return ERROR(ZSTD_error_corruption_detected);
            }
            ip = ip.add(ofhSize);
        }

        {
            let mlhSize: size_t = ZSTD_buildSeqTable(
                (*dctx).entropy.MLTable.as_mut_ptr(),
                &mut (*dctx).MLTptr,
                MLtype,
                MaxML,
                MLFSELog,
                ip as *const c_void,
                iend.offset_from(ip) as size_t,
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
            if ZSTD_isError(mlhSize) != 0 {
                return ERROR(ZSTD_error_corruption_detected);
            }
            ip = ip.add(mlhSize);
        }
    }

    ip.offset_from(istart) as size_t
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct seq_t {
    pub litLength: size_t,
    pub matchLength: size_t,
    pub offset: size_t,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ZSTD_fseState {
    pub state: size_t,
    pub table: *const ZSTD_seqSymbol,
}

#[repr(C)]
pub struct seqState_t {
    pub DStream: BIT_DStream_t,
    pub stateLL: ZSTD_fseState,
    pub stateOffb: ZSTD_fseState,
    pub stateML: ZSTD_fseState,
    pub prevOffset: [size_t; ZSTD_REP_NUM],
}

/* ZSTD_overlapCopy8() :
 *  Copies 8 bytes from ip to op and updates op and ip where ip <= op. */
pub unsafe fn ZSTD_overlapCopy8(op: *mut *mut BYTE, ip: *mut *const BYTE, offset: size_t) {
    if offset < 8 {
        /* close range match, overlap */
        static dec32table: [U32; 8] = [0, 1, 2, 1, 4, 4, 4, 4]; /* added */
        static dec64table: [c_int; 8] = [8, 8, 8, 7, 8, 9, 10, 11]; /* subtracted */
        let sub2: c_int = dec64table[offset as usize];
        *(*op).add(0) = *(*ip).add(0);
        *(*op).add(1) = *(*ip).add(1);
        *(*op).add(2) = *(*ip).add(2);
        *(*op).add(3) = *(*ip).add(3);
        *ip = (*ip).offset(dec32table[offset as usize] as isize);
        ZSTD_copy4((*op).add(4) as *mut c_void, *ip as *const c_void);
        *ip = (*ip).offset(-(sub2 as isize));
    } else {
        ZSTD_copy8(*op, *ip);
    }
    *ip = (*ip).add(8);
    *op = (*op).add(8);
}

/* ZSTD_safecopy() : */
pub unsafe fn ZSTD_safecopy(
    mut op: *mut BYTE,
    oend_w: *const BYTE,
    mut ip: *const BYTE,
    mut length: isize,
    ovtype: ZSTD_overlap_e,
) {
    let diff: isize = (op as isize) - (ip as isize);
    let oend: *mut BYTE = op.offset(length);

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
        ZSTD_overlapCopy8(&mut op, &mut ip, diff as size_t);
        length -= 8;
    }

    if oend <= oend_w as *mut BYTE {
        /* No risk of overwrite. */
        ZSTD_wildcopy(op, ip, length, ovtype);
        return;
    }
    if op <= oend_w as *mut BYTE {
        /* Wildcopy until we get close to the end. */
        ZSTD_wildcopy(op, ip, (oend_w as isize) - (op as isize), ovtype);
        ip = ip.offset((oend_w as isize) - (op as isize));
        op = op.offset((oend_w as isize) - (op as isize));
    }
    /* Handle the leftovers. */
    while op < oend {
        *op = *ip;
        op = op.add(1);
        ip = ip.add(1);
    }
}

/* ZSTD_safecopyDstBeforeSrc(): */
pub unsafe fn ZSTD_safecopyDstBeforeSrc(mut op: *mut BYTE, mut ip: *const BYTE, length: isize) {
    let diff: isize = (op as isize) - (ip as isize);
    let oend: *mut BYTE = op.offset(length);

    if length < 8 || diff > -8 {
        /* Handle short lengths, close overlaps, and dst not before src. */
        while op < oend {
            *op = *ip;
            op = op.add(1);
            ip = ip.add(1);
        }
        return;
    }

    if op <= oend.offset(-WILDCOPY_OVERLENGTH) && diff < -WILDCOPY_VECLEN {
        ZSTD_wildcopy(
            op,
            ip,
            (oend.offset(-WILDCOPY_OVERLENGTH) as isize) - (op as isize),
            ZSTD_no_overlap,
        );
        ip = ip.offset((oend.offset(-WILDCOPY_OVERLENGTH) as isize) - (op as isize));
        op = op.offset((oend.offset(-WILDCOPY_OVERLENGTH) as isize) - (op as isize));
    }

    /* Handle the leftovers. */
    while op < oend {
        *op = *ip;
        op = op.add(1);
        ip = ip.add(1);
    }
}

/* ZSTD_execSequenceEnd(): */
pub unsafe fn ZSTD_execSequenceEnd(
    mut op: *mut BYTE,
    oend: *mut BYTE,
    mut sequence: seq_t,
    litPtr: *mut *const BYTE,
    litLimit: *const BYTE,
    prefixStart: *const BYTE,
    virtualStart: *const BYTE,
    dictEnd: *const BYTE,
) -> size_t {
    let oLitEnd: *mut BYTE = op.add(sequence.litLength);
    let sequenceLength: size_t = sequence.litLength + sequence.matchLength;
    let iLitEnd: *const BYTE = (*litPtr).add(sequence.litLength);
    let mut match_: *const BYTE = oLitEnd.wrapping_sub(sequence.offset) as *const BYTE;
    let oend_w: *const BYTE = oend.offset(-WILDCOPY_OVERLENGTH);

    /* bounds checks : careful of address space overflow in 32-bit mode */
    if sequenceLength > (oend as isize - op as isize) as size_t {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    if sequence.litLength > (litLimit as isize - *litPtr as isize) as size_t {
        return ERROR(ZSTD_error_corruption_detected);
    }

    /* copy literals */
    ZSTD_safecopy(op, oend_w, *litPtr, sequence.litLength as isize, ZSTD_no_overlap);
    op = oLitEnd;
    *litPtr = iLitEnd;

    /* copy Match */
    if sequence.offset > (oLitEnd as isize - prefixStart as isize) as size_t {
        /* offset beyond prefix */
        if sequence.offset > (oLitEnd as isize - virtualStart as isize) as size_t {
            return ERROR(ZSTD_error_corruption_detected);
        }
        match_ = dictEnd.wrapping_sub((prefixStart as isize - match_ as isize) as size_t);
        if match_.add(sequence.matchLength) <= dictEnd {
            ZSTD_memmove(oLitEnd, match_, sequence.matchLength);
            return sequenceLength;
        }
        /* span extDict & currentPrefixSegment */
        {
            let length1: size_t = (dictEnd as isize - match_ as isize) as size_t;
            ZSTD_memmove(oLitEnd, match_, length1);
            op = oLitEnd.add(length1);
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

/* ZSTD_execSequenceEndSplitLitBuffer(): */
pub unsafe fn ZSTD_execSequenceEndSplitLitBuffer(
    mut op: *mut BYTE,
    oend: *mut BYTE,
    oend_w: *const BYTE,
    mut sequence: seq_t,
    litPtr: *mut *const BYTE,
    litLimit: *const BYTE,
    prefixStart: *const BYTE,
    virtualStart: *const BYTE,
    dictEnd: *const BYTE,
) -> size_t {
    let oLitEnd: *mut BYTE = op.add(sequence.litLength);
    let sequenceLength: size_t = sequence.litLength + sequence.matchLength;
    let iLitEnd: *const BYTE = (*litPtr).add(sequence.litLength);
    let mut match_: *const BYTE = oLitEnd.wrapping_sub(sequence.offset) as *const BYTE;

    /* bounds checks : careful of address space overflow in 32-bit mode */
    if sequenceLength > (oend as isize - op as isize) as size_t {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    if sequence.litLength > (litLimit as isize - *litPtr as isize) as size_t {
        return ERROR(ZSTD_error_corruption_detected);
    }

    /* copy literals */
    if op > *litPtr as *mut BYTE && op < (*litPtr).add(sequence.litLength) as *mut BYTE {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    ZSTD_safecopyDstBeforeSrc(op, *litPtr, sequence.litLength as isize);
    op = oLitEnd;
    *litPtr = iLitEnd;

    /* copy Match */
    if sequence.offset > (oLitEnd as isize - prefixStart as isize) as size_t {
        /* offset beyond prefix */
        if sequence.offset > (oLitEnd as isize - virtualStart as isize) as size_t {
            return ERROR(ZSTD_error_corruption_detected);
        }
        match_ = dictEnd.wrapping_sub((prefixStart as isize - match_ as isize) as size_t);
        if match_.add(sequence.matchLength) <= dictEnd {
            ZSTD_memmove(oLitEnd, match_, sequence.matchLength);
            return sequenceLength;
        }
        /* span extDict & currentPrefixSegment */
        {
            let length1: size_t = (dictEnd as isize - match_ as isize) as size_t;
            ZSTD_memmove(oLitEnd, match_, length1);
            op = oLitEnd.add(length1);
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

pub unsafe fn ZSTD_execSequence(
    mut op: *mut BYTE,
    oend: *mut BYTE,
    mut sequence: seq_t,
    litPtr: *mut *const BYTE,
    litLimit: *const BYTE,
    prefixStart: *const BYTE,
    virtualStart: *const BYTE,
    dictEnd: *const BYTE,
) -> size_t {
    let oLitEnd: *mut BYTE = op.add(sequence.litLength);
    let sequenceLength: size_t = sequence.litLength + sequence.matchLength;
    let oMatchEnd: *mut BYTE = op.add(sequenceLength); /* risk : address space overflow (32-bits) */
    let oend_w: *const BYTE = oend.offset(-WILDCOPY_OVERLENGTH); /* risk : address space underflow on oend=NULL */
    let iLitEnd: *const BYTE = (*litPtr).add(sequence.litLength);
    let mut match_: *const BYTE = oLitEnd.wrapping_sub(sequence.offset) as *const BYTE;

    /* Handle edge cases in a slow path */
    if iLitEnd > litLimit
        || oMatchEnd > oend_w as *mut BYTE
        || (MEM_32bits() != 0
            && ((oend as isize - op as isize) as size_t)
                < sequenceLength + WILDCOPY_OVERLENGTH as size_t)
    {
        return ZSTD_execSequenceEnd(
            op, oend, sequence, litPtr, litLimit, prefixStart, virtualStart, dictEnd,
        );
    }

    /* Copy Literals */
    ZSTD_copy16(op, *litPtr);
    if sequence.litLength > 16 {
        ZSTD_wildcopy(
            op.add(16),
            (*litPtr).add(16),
            sequence.litLength as isize - 16,
            ZSTD_no_overlap,
        );
    }
    op = oLitEnd;
    *litPtr = iLitEnd; /* update for next sequence */

    /* Copy Match */
    if sequence.offset > (oLitEnd as isize - prefixStart as isize) as size_t {
        /* offset beyond prefix -> go into extDict */
        if sequence.offset > (oLitEnd as isize - virtualStart as isize) as size_t {
            return ERROR(ZSTD_error_corruption_detected);
        }
        match_ = dictEnd.wrapping_offset(match_ as isize - prefixStart as isize);
        if match_.add(sequence.matchLength) <= dictEnd {
            ZSTD_memmove(oLitEnd, match_, sequence.matchLength);
            return sequenceLength;
        }
        /* span extDict & currentPrefixSegment */
        {
            let length1: size_t = (dictEnd as isize - match_ as isize) as size_t;
            ZSTD_memmove(oLitEnd, match_, length1);
            op = oLitEnd.add(length1);
            sequence.matchLength -= length1;
            match_ = prefixStart;
        }
    }
    /* Match within prefix of 1 or more bytes */

    if sequence.offset >= WILDCOPY_VECLEN as size_t {
        ZSTD_wildcopy(op, match_, sequence.matchLength as isize, ZSTD_no_overlap);
        return sequenceLength;
    }

    /* Copy 8 bytes and spread the offset to be >= 8. */
    {
        let mut op2 = op;
        let mut match2 = match_;
        ZSTD_overlapCopy8(&mut op2, &mut match2, sequence.offset);
        op = op2;
        match_ = match2;
    }

    /* If the match length is > 8 bytes, then continue with the wildcopy. */
    if sequence.matchLength > 8 {
        ZSTD_wildcopy(
            op,
            match_,
            sequence.matchLength as isize - 8,
            ZSTD_overlap_src_before_dst,
        );
    }
    sequenceLength
}

pub unsafe fn ZSTD_execSequenceSplitLitBuffer(
    mut op: *mut BYTE,
    oend: *mut BYTE,
    oend_w: *const BYTE,
    mut sequence: seq_t,
    litPtr: *mut *const BYTE,
    litLimit: *const BYTE,
    prefixStart: *const BYTE,
    virtualStart: *const BYTE,
    dictEnd: *const BYTE,
) -> size_t {
    let oLitEnd: *mut BYTE = op.add(sequence.litLength);
    let sequenceLength: size_t = sequence.litLength + sequence.matchLength;
    let oMatchEnd: *mut BYTE = op.add(sequenceLength); /* risk : address space overflow (32-bits) */
    let iLitEnd: *const BYTE = (*litPtr).add(sequence.litLength);
    let mut match_: *const BYTE = oLitEnd.wrapping_sub(sequence.offset) as *const BYTE;

    /* Handle edge cases in a slow path */
    if iLitEnd > litLimit
        || oMatchEnd > oend_w as *mut BYTE
        || (MEM_32bits() != 0
            && ((oend as isize - op as isize) as size_t)
                < sequenceLength + WILDCOPY_OVERLENGTH as size_t)
    {
        return ZSTD_execSequenceEndSplitLitBuffer(
            op, oend, oend_w, sequence, litPtr, litLimit, prefixStart, virtualStart, dictEnd,
        );
    }

    /* Copy Literals */
    ZSTD_copy16(op, *litPtr);
    if sequence.litLength > 16 {
        ZSTD_wildcopy(
            op.add(16),
            (*litPtr).add(16),
            sequence.litLength as isize - 16,
            ZSTD_no_overlap,
        );
    }
    op = oLitEnd;
    *litPtr = iLitEnd; /* update for next sequence */

    /* Copy Match */
    if sequence.offset > (oLitEnd as isize - prefixStart as isize) as size_t {
        /* offset beyond prefix -> go into extDict */
        if sequence.offset > (oLitEnd as isize - virtualStart as isize) as size_t {
            return ERROR(ZSTD_error_corruption_detected);
        }
        match_ = dictEnd.wrapping_offset(match_ as isize - prefixStart as isize);
        if match_.add(sequence.matchLength) <= dictEnd {
            ZSTD_memmove(oLitEnd, match_, sequence.matchLength);
            return sequenceLength;
        }
        /* span extDict & currentPrefixSegment */
        {
            let length1: size_t = (dictEnd as isize - match_ as isize) as size_t;
            ZSTD_memmove(oLitEnd, match_, length1);
            op = oLitEnd.add(length1);
            sequence.matchLength -= length1;
            match_ = prefixStart;
        }
    }
    /* Match within prefix of 1 or more bytes */

    if sequence.offset >= WILDCOPY_VECLEN as size_t {
        ZSTD_wildcopy(op, match_, sequence.matchLength as isize, ZSTD_no_overlap);
        return sequenceLength;
    }

    /* Copy 8 bytes and spread the offset to be >= 8. */
    {
        let mut op2 = op;
        let mut match2 = match_;
        ZSTD_overlapCopy8(&mut op2, &mut match2, sequence.offset);
        op = op2;
        match_ = match2;
    }

    /* If the match length is > 8 bytes, then continue with the wildcopy. */
    if sequence.matchLength > 8 {
        ZSTD_wildcopy(
            op,
            match_,
            sequence.matchLength as isize - 8,
            ZSTD_overlap_src_before_dst,
        );
    }
    sequenceLength
}

pub unsafe fn ZSTD_initFseState(
    DStatePtr: *mut ZSTD_fseState,
    bitD: *mut BIT_DStream_t,
    dt: *const ZSTD_seqSymbol,
) {
    let ptr: *const c_void = dt as *const c_void;
    let DTableH: *const ZSTD_seqSymbol_header = ptr as *const ZSTD_seqSymbol_header;
    (*DStatePtr).state = BIT_readBits(bitD, (*DTableH).tableLog) as size_t;
    BIT_reloadDStream(bitD);
    (*DStatePtr).table = dt.add(1);
}

#[inline(always)]
pub unsafe fn ZSTD_updateFseStateWithDInfo(
    DStatePtr: *mut ZSTD_fseState,
    bitD: *mut BIT_DStream_t,
    nextState: U16,
    nbBits: U32,
) {
    let lowBits: size_t = BIT_readBits(bitD, nbBits) as size_t;
    (*DStatePtr).state = nextState as size_t + lowBits;
}

/* LONG_OFFSETS_MAX_EXTRA_BITS_32 */
pub const LONG_OFFSETS_MAX_EXTRA_BITS_32: U32 =
    if ZSTD_WINDOWLOG_MAX_32 as u32 > STREAM_ACCUMULATOR_MIN_32 {
        ZSTD_WINDOWLOG_MAX_32 as u32 - STREAM_ACCUMULATOR_MIN_32
    } else {
        0
    };

pub type ZSTD_longOffset_e = c_uint;
pub const ZSTD_lo_isRegularOffset: ZSTD_longOffset_e = 0;
pub const ZSTD_lo_isLongOffset: ZSTD_longOffset_e = 1;

/* ZSTD_decodeSequence(): */
#[inline(always)]
pub unsafe fn ZSTD_decodeSequence(
    seqState: *mut seqState_t,
    longOffsets: ZSTD_longOffset_e,
    isLastSeq: c_int,
) -> seq_t {
    let mut seq: seq_t = seq_t {
        litLength: 0,
        matchLength: 0,
        offset: 0,
    };
    /* not aarch64+gcc : direct pointers into the table */
    let llDInfo: *const ZSTD_seqSymbol =
        (*seqState).stateLL.table.add((*seqState).stateLL.state);
    let mlDInfo: *const ZSTD_seqSymbol =
        (*seqState).stateML.table.add((*seqState).stateML.state);
    let ofDInfo: *const ZSTD_seqSymbol =
        (*seqState).stateOffb.table.add((*seqState).stateOffb.state);

    seq.matchLength = (*mlDInfo).baseValue as size_t;
    seq.litLength = (*llDInfo).baseValue as size_t;
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
            let mut offset: size_t;
            if ofBits > 1 {
                if MEM_32bits() != 0
                    && longOffsets != 0
                    && (ofBits as u32 >= STREAM_ACCUMULATOR_MIN_32)
                {
                    let extraBits: U32 = LONG_OFFSETS_MAX_EXTRA_BITS_32;
                    offset = ofBase as size_t
                        + ((BIT_readBitsFast(&mut (*seqState).DStream, ofBits as u32 - extraBits)
                            as size_t)
                            << extraBits);
                    BIT_reloadDStream(&mut (*seqState).DStream);
                    offset += BIT_readBitsFast(&mut (*seqState).DStream, extraBits) as size_t;
                } else {
                    offset = ofBase as size_t
                        + BIT_readBitsFast(&mut (*seqState).DStream, ofBits as u32) as size_t; /* <=  (ZSTD_WINDOWLOG_MAX-1) bits */
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
                    let mut offset_v: size_t = ofBase as size_t
                        + ll0 as size_t
                        + BIT_readBitsFast(&mut (*seqState).DStream, 1) as size_t;
                    {
                        let mut temp: size_t = if offset_v == 3 {
                            (*seqState).prevOffset[0] - 1
                        } else {
                            (*seqState).prevOffset[offset_v]
                        };
                        temp -= (temp == 0) as size_t; /* 0 is not valid */
                        if offset_v != 1 {
                            (*seqState).prevOffset[2] = (*seqState).prevOffset[1];
                        }
                        (*seqState).prevOffset[1] = (*seqState).prevOffset[0];
                        offset_v = temp;
                        (*seqState).prevOffset[0] = offset_v;
                        offset = offset_v;
                    }
                }
            }
            seq.offset = offset;
        }

        if mlBits > 0 {
            seq.matchLength += BIT_readBitsFast(&mut (*seqState).DStream, mlBits as u32) as size_t;
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
            seq.litLength += BIT_readBitsFast(&mut (*seqState).DStream, llBits as u32) as size_t;
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
                BIT_reloadDStream(&mut (*seqState).DStream);
            } /* <= 18 bits */
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

/* ZSTD_assertValidSequence: entirely inside #if DEBUGLEVEL>=1 (and FUZZING build);
 * DEBUGLEVEL==0 and no fuzzing => compiled out entirely. Omitted. */

pub unsafe fn ZSTD_decompressSequences_bodySplitLitBuffer(
    dctx: *mut ZSTD_DCtx,
    dst: *mut c_void,
    maxDstSize: size_t,
    seqStart: *const c_void,
    seqSize: size_t,
    mut nbSeq: c_int,
    isLongOffset: ZSTD_longOffset_e,
) -> size_t {
    let ip: *const BYTE = seqStart as *const BYTE;
    let iend: *const BYTE = ip.add(seqSize);
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
        let mut seqState: seqState_t = core::mem::zeroed();
        (*dctx).fseEntropy = 1;
        {
            let mut i: U32 = 0;
            while (i as usize) < ZSTD_REP_NUM {
                seqState.prevOffset[i as usize] = (*dctx).entropy.rep[i as usize] as size_t;
                i += 1;
            }
        }
        if ERR_isError(BIT_initDStream(
            &mut seqState.DStream,
            ip,
            iend.offset_from(ip) as size_t,
        )) != 0
        {
            return ERROR(ZSTD_error_corruption_detected);
        }
        ZSTD_initFseState(&mut seqState.stateLL, &mut seqState.DStream, (*dctx).LLTptr);
        ZSTD_initFseState(&mut seqState.stateOffb, &mut seqState.DStream, (*dctx).OFTptr);
        ZSTD_initFseState(&mut seqState.stateML, &mut seqState.DStream, (*dctx).MLTptr);

        /* decompress without overrunning litPtr begins */
        {
            let mut sequence: seq_t = seq_t {
                litLength: 0,
                matchLength: 0,
                offset: 0,
            };
            /* Handle the initial state where litBuffer is currently split between dst and litExtraBuffer */
            while nbSeq != 0 {
                sequence = ZSTD_decodeSequence(&mut seqState, isLongOffset, (nbSeq == 1) as c_int);
                if litPtr.add(sequence.litLength) > (*dctx).litBufferEnd {
                    break;
                }
                {
                    let oneSeqSize: size_t = ZSTD_execSequenceSplitLitBuffer(
                        op,
                        oend,
                        litPtr
                            .add(sequence.litLength)
                            .wrapping_sub(WILDCOPY_OVERLENGTH as size_t),
                        sequence,
                        &mut litPtr,
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

            /* If there are more sequences, they will need to read literals from litExtraBuffer */
            if nbSeq > 0 {
                let leftoverLit: size_t =
                    (*dctx).litBufferEnd.offset_from(litPtr) as size_t;
                if leftoverLit != 0 {
                    if leftoverLit > (oend as isize - op as isize) as size_t {
                        return ERROR(ZSTD_error_dstSize_tooSmall);
                    }
                    ZSTD_safecopyDstBeforeSrc(op, litPtr, leftoverLit as isize);
                    sequence.litLength -= leftoverLit;
                    op = op.add(leftoverLit);
                }
                litPtr = (*dctx).litExtraBuffer.as_ptr();
                litBufferEnd = (*dctx).litExtraBuffer.as_ptr().add(ZSTD_LITBUFFEREXTRASIZE);
                (*dctx).litBufferLocation = ZSTD_not_in_dst;
                {
                    let oneSeqSize: size_t = ZSTD_execSequence(
                        op, oend, sequence, &mut litPtr, litBufferEnd, prefixStart, vBase,
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
                let sequence: seq_t =
                    ZSTD_decodeSequence(&mut seqState, isLongOffset, (nbSeq == 1) as c_int);
                let oneSeqSize: size_t = ZSTD_execSequence(
                    op, oend, sequence, &mut litPtr, litBufferEnd, prefixStart, vBase, dictEnd,
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
        let lastLLSize: size_t = (litBufferEnd as isize - litPtr as isize) as size_t;
        if lastLLSize > (oend as isize - op as isize) as size_t {
            return ERROR(ZSTD_error_dstSize_tooSmall);
        }
        if !op.is_null() {
            ZSTD_memmove(op, litPtr, lastLLSize);
            op = op.add(lastLLSize);
        }
        litPtr = (*dctx).litExtraBuffer.as_ptr();
        litBufferEnd = (*dctx).litExtraBuffer.as_ptr().add(ZSTD_LITBUFFEREXTRASIZE);
        (*dctx).litBufferLocation = ZSTD_not_in_dst;
    }
    /* copy last literals from internal buffer */
    {
        let lastLLSize: size_t = (litBufferEnd as isize - litPtr as isize) as size_t;
        if lastLLSize > (oend as isize - op as isize) as size_t {
            return ERROR(ZSTD_error_dstSize_tooSmall);
        }
        if !op.is_null() {
            ZSTD_memcpy(op, litPtr, lastLLSize);
            op = op.add(lastLLSize);
        }
    }

    (op as isize - ostart as isize) as size_t
}

pub unsafe fn ZSTD_decompressSequences_body(
    dctx: *mut ZSTD_DCtx,
    dst: *mut c_void,
    maxDstSize: size_t,
    seqStart: *const c_void,
    seqSize: size_t,
    mut nbSeq: c_int,
    isLongOffset: ZSTD_longOffset_e,
) -> size_t {
    let ip: *const BYTE = seqStart as *const BYTE;
    let iend: *const BYTE = ip.add(seqSize);
    let ostart: *mut BYTE = dst as *mut BYTE;
    let oend: *mut BYTE = if (*dctx).litBufferLocation == ZSTD_not_in_dst {
        ZSTD_maybeNullPtrAdd(ostart, maxDstSize as isize)
    } else {
        (*dctx).litBuffer
    };
    let mut op: *mut BYTE = ostart;
    let mut litPtr: *const BYTE = (*dctx).litPtr;
    let litEnd: *const BYTE = litPtr.add((*dctx).litSize);
    let prefixStart: *const BYTE = (*dctx).prefixStart as *const BYTE;
    let vBase: *const BYTE = (*dctx).virtualStart as *const BYTE;
    let dictEnd: *const BYTE = (*dctx).dictEnd as *const BYTE;

    /* Regen sequences */
    if nbSeq != 0 {
        let mut seqState: seqState_t = core::mem::zeroed();
        (*dctx).fseEntropy = 1;
        {
            let mut i: U32 = 0;
            while (i as usize) < ZSTD_REP_NUM {
                seqState.prevOffset[i as usize] = (*dctx).entropy.rep[i as usize] as size_t;
                i += 1;
            }
        }
        if ERR_isError(BIT_initDStream(
            &mut seqState.DStream,
            ip,
            iend.offset_from(ip) as size_t,
        )) != 0
        {
            return ERROR(ZSTD_error_corruption_detected);
        }
        ZSTD_initFseState(&mut seqState.stateLL, &mut seqState.DStream, (*dctx).LLTptr);
        ZSTD_initFseState(&mut seqState.stateOffb, &mut seqState.DStream, (*dctx).OFTptr);
        ZSTD_initFseState(&mut seqState.stateML, &mut seqState.DStream, (*dctx).MLTptr);

        while nbSeq != 0 {
            let sequence: seq_t =
                ZSTD_decodeSequence(&mut seqState, isLongOffset, (nbSeq == 1) as c_int);
            let oneSeqSize: size_t = ZSTD_execSequence(
                op, oend, sequence, &mut litPtr, litEnd, prefixStart, vBase, dictEnd,
            );
            if ZSTD_isError(oneSeqSize) != 0 {
                return oneSeqSize;
            }
            op = op.add(oneSeqSize);
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
        let lastLLSize: size_t = (litEnd as isize - litPtr as isize) as size_t;
        if lastLLSize > (oend as isize - op as isize) as size_t {
            return ERROR(ZSTD_error_dstSize_tooSmall);
        }
        if !op.is_null() {
            ZSTD_memcpy(op, litPtr, lastLLSize);
            op = op.add(lastLLSize);
        }
    }

    (op as isize - ostart as isize) as size_t
}

pub unsafe fn ZSTD_decompressSequences_default(
    dctx: *mut ZSTD_DCtx,
    dst: *mut c_void,
    maxDstSize: size_t,
    seqStart: *const c_void,
    seqSize: size_t,
    nbSeq: c_int,
    isLongOffset: ZSTD_longOffset_e,
) -> size_t {
    ZSTD_decompressSequences_body(dctx, dst, maxDstSize, seqStart, seqSize, nbSeq, isLongOffset)
}

pub unsafe fn ZSTD_decompressSequencesSplitLitBuffer_default(
    dctx: *mut ZSTD_DCtx,
    dst: *mut c_void,
    maxDstSize: size_t,
    seqStart: *const c_void,
    seqSize: size_t,
    nbSeq: c_int,
    isLongOffset: ZSTD_longOffset_e,
) -> size_t {
    ZSTD_decompressSequences_bodySplitLitBuffer(
        dctx, dst, maxDstSize, seqStart, seqSize, nbSeq, isLongOffset,
    )
}

#[inline(always)]
pub unsafe fn ZSTD_prefetchMatch(
    mut prefetchPos: size_t,
    sequence: seq_t,
    prefixStart: *const BYTE,
    dictEnd: *const BYTE,
) -> size_t {
    prefetchPos += sequence.litLength;
    {
        let matchBase: *const BYTE = if sequence.offset > prefetchPos {
            dictEnd
        } else {
            prefixStart
        };
        /* note : this operation can overflow when seq.offset is really too large */
        let _match: *const BYTE = ZSTD_wrappedPtrSub(
            ZSTD_wrappedPtrAdd(matchBase, prefetchPos as isize),
            sequence.offset as isize,
        );
        /* PREFETCH_L1 : no-op prefetch, dropped */
        let _ = _match;
    }
    prefetchPos + sequence.matchLength
}

pub const STORED_SEQS: usize = 8;
pub const STORED_SEQS_MASK: usize = STORED_SEQS - 1;
pub const ADVANCED_SEQS: c_int = STORED_SEQS as c_int;

pub unsafe fn ZSTD_decompressSequencesLong_body(
    dctx: *mut ZSTD_DCtx,
    dst: *mut c_void,
    maxDstSize: size_t,
    seqStart: *const c_void,
    seqSize: size_t,
    nbSeq: c_int,
    isLongOffset: ZSTD_longOffset_e,
) -> size_t {
    let ip: *const BYTE = seqStart as *const BYTE;
    let iend: *const BYTE = ip.add(seqSize);
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
        let seqAdvance: c_int = MIN(nbSeq, ADVANCED_SEQS);
        let mut seqState: seqState_t = core::mem::zeroed();
        let mut seqNb: c_int;
        let mut prefetchPos: size_t = (op as isize - prefixStart as isize) as size_t;

        (*dctx).fseEntropy = 1;
        {
            let mut i: c_int = 0;
            while (i as usize) < ZSTD_REP_NUM {
                seqState.prevOffset[i as usize] = (*dctx).entropy.rep[i as usize] as size_t;
                i += 1;
            }
        }
        if ERR_isError(BIT_initDStream(
            &mut seqState.DStream,
            ip,
            iend.offset_from(ip) as size_t,
        )) != 0
        {
            return ERROR(ZSTD_error_corruption_detected);
        }
        ZSTD_initFseState(&mut seqState.stateLL, &mut seqState.DStream, (*dctx).LLTptr);
        ZSTD_initFseState(&mut seqState.stateOffb, &mut seqState.DStream, (*dctx).OFTptr);
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

            if (*dctx).litBufferLocation == ZSTD_split
                && litPtr.add(
                    sequences[((seqNb - ADVANCED_SEQS) as usize) & STORED_SEQS_MASK].litLength,
                ) > (*dctx).litBufferEnd
            {
                /* lit buffer is reaching split point, empty out the first buffer and transition to litExtraBuffer */
                let leftoverLit: size_t = (*dctx).litBufferEnd.offset_from(litPtr) as size_t;
                if leftoverLit != 0 {
                    if leftoverLit > (oend as isize - op as isize) as size_t {
                        return ERROR(ZSTD_error_dstSize_tooSmall);
                    }
                    ZSTD_safecopyDstBeforeSrc(op, litPtr, leftoverLit as isize);
                    sequences[((seqNb - ADVANCED_SEQS) as usize) & STORED_SEQS_MASK].litLength -=
                        leftoverLit;
                    op = op.add(leftoverLit);
                }
                litPtr = (*dctx).litExtraBuffer.as_ptr();
                litBufferEnd = (*dctx).litExtraBuffer.as_ptr().add(ZSTD_LITBUFFEREXTRASIZE);
                (*dctx).litBufferLocation = ZSTD_not_in_dst;
                {
                    let oneSeqSize: size_t = ZSTD_execSequence(
                        op,
                        oend,
                        sequences[((seqNb - ADVANCED_SEQS) as usize) & STORED_SEQS_MASK],
                        &mut litPtr,
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
                    sequences[(seqNb as usize) & STORED_SEQS_MASK] = sequence;
                    op = op.add(oneSeqSize);
                }
            } else {
                /* lit buffer is either wholly contained in first or second split, or not split at all */
                let oneSeqSize: size_t = if (*dctx).litBufferLocation == ZSTD_split {
                    ZSTD_execSequenceSplitLitBuffer(
                        op,
                        oend,
                        litPtr
                            .add(
                                sequences[((seqNb - ADVANCED_SEQS) as usize) & STORED_SEQS_MASK]
                                    .litLength,
                            )
                            .wrapping_sub(WILDCOPY_OVERLENGTH as size_t),
                        sequences[((seqNb - ADVANCED_SEQS) as usize) & STORED_SEQS_MASK],
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
                        sequences[((seqNb - ADVANCED_SEQS) as usize) & STORED_SEQS_MASK],
                        &mut litPtr,
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
                sequences[(seqNb as usize) & STORED_SEQS_MASK] = sequence;
                op = op.add(oneSeqSize);
            }
            seqNb += 1;
        }
        if BIT_endOfDStream(&seqState.DStream) == 0 {
            return ERROR(ZSTD_error_corruption_detected);
        }

        /* finish queue */
        seqNb -= seqAdvance;
        while seqNb < nbSeq {
            let sequence: *mut seq_t = &mut sequences[(seqNb as usize) & STORED_SEQS_MASK];
            if (*dctx).litBufferLocation == ZSTD_split
                && litPtr.add((*sequence).litLength) > (*dctx).litBufferEnd
            {
                let leftoverLit: size_t = (*dctx).litBufferEnd.offset_from(litPtr) as size_t;
                if leftoverLit != 0 {
                    if leftoverLit > (oend as isize - op as isize) as size_t {
                        return ERROR(ZSTD_error_dstSize_tooSmall);
                    }
                    ZSTD_safecopyDstBeforeSrc(op, litPtr, leftoverLit as isize);
                    (*sequence).litLength -= leftoverLit;
                    op = op.add(leftoverLit);
                }
                litPtr = (*dctx).litExtraBuffer.as_ptr();
                litBufferEnd = (*dctx).litExtraBuffer.as_ptr().add(ZSTD_LITBUFFEREXTRASIZE);
                (*dctx).litBufferLocation = ZSTD_not_in_dst;
                {
                    let oneSeqSize: size_t = ZSTD_execSequence(
                        op, oend, *sequence, &mut litPtr, litBufferEnd, prefixStart, dictStart,
                        dictEnd,
                    );
                    if ZSTD_isError(oneSeqSize) != 0 {
                        return oneSeqSize;
                    }
                    op = op.add(oneSeqSize);
                }
            } else {
                let oneSeqSize: size_t = if (*dctx).litBufferLocation == ZSTD_split {
                    ZSTD_execSequenceSplitLitBuffer(
                        op,
                        oend,
                        litPtr
                            .add((*sequence).litLength)
                            .wrapping_sub(WILDCOPY_OVERLENGTH as size_t),
                        *sequence,
                        &mut litPtr,
                        litBufferEnd,
                        prefixStart,
                        dictStart,
                        dictEnd,
                    )
                } else {
                    ZSTD_execSequence(
                        op, oend, *sequence, &mut litPtr, litBufferEnd, prefixStart, dictStart,
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
        let lastLLSize: size_t = (litBufferEnd as isize - litPtr as isize) as size_t;
        if lastLLSize > (oend as isize - op as isize) as size_t {
            return ERROR(ZSTD_error_dstSize_tooSmall);
        }
        if !op.is_null() {
            ZSTD_memmove(op, litPtr, lastLLSize);
            op = op.add(lastLLSize);
        }
        litPtr = (*dctx).litExtraBuffer.as_ptr();
        litBufferEnd = (*dctx).litExtraBuffer.as_ptr().add(ZSTD_LITBUFFEREXTRASIZE);
    }
    {
        let lastLLSize: size_t = (litBufferEnd as isize - litPtr as isize) as size_t;
        if lastLLSize > (oend as isize - op as isize) as size_t {
            return ERROR(ZSTD_error_dstSize_tooSmall);
        }
        if !op.is_null() {
            ZSTD_memmove(op, litPtr, lastLLSize);
            op = op.add(lastLLSize);
        }
    }

    (op as isize - ostart as isize) as size_t
}

pub unsafe fn ZSTD_decompressSequencesLong_default(
    dctx: *mut ZSTD_DCtx,
    dst: *mut c_void,
    maxDstSize: size_t,
    seqStart: *const c_void,
    seqSize: size_t,
    nbSeq: c_int,
    isLongOffset: ZSTD_longOffset_e,
) -> size_t {
    ZSTD_decompressSequencesLong_body(dctx, dst, maxDstSize, seqStart, seqSize, nbSeq, isLongOffset)
}

/* DYNAMIC_BMI2 == 0 : no _bmi2 variants compiled */

pub unsafe fn ZSTD_decompressSequences(
    dctx: *mut ZSTD_DCtx,
    dst: *mut c_void,
    maxDstSize: size_t,
    seqStart: *const c_void,
    seqSize: size_t,
    nbSeq: c_int,
    isLongOffset: ZSTD_longOffset_e,
) -> size_t {
    ZSTD_decompressSequences_default(dctx, dst, maxDstSize, seqStart, seqSize, nbSeq, isLongOffset)
}

pub unsafe fn ZSTD_decompressSequencesSplitLitBuffer(
    dctx: *mut ZSTD_DCtx,
    dst: *mut c_void,
    maxDstSize: size_t,
    seqStart: *const c_void,
    seqSize: size_t,
    nbSeq: c_int,
    isLongOffset: ZSTD_longOffset_e,
) -> size_t {
    ZSTD_decompressSequencesSplitLitBuffer_default(
        dctx, dst, maxDstSize, seqStart, seqSize, nbSeq, isLongOffset,
    )
}

pub unsafe fn ZSTD_decompressSequencesLong(
    dctx: *mut ZSTD_DCtx,
    dst: *mut c_void,
    maxDstSize: size_t,
    seqStart: *const c_void,
    seqSize: size_t,
    nbSeq: c_int,
    isLongOffset: ZSTD_longOffset_e,
) -> size_t {
    ZSTD_decompressSequencesLong_default(
        dctx, dst, maxDstSize, seqStart, seqSize, nbSeq, isLongOffset,
    )
}

/**
 * @returns The total size of the history referenceable by zstd. */
pub unsafe fn ZSTD_totalHistorySize(op: *mut BYTE, virtualStart: *const BYTE) -> size_t {
    (op as isize - virtualStart as isize) as size_t
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ZSTD_OffsetInfo {
    pub longOffsetShare: c_uint,
    pub maxNbAdditionalBits: c_uint,
}

/* ZSTD_getOffsetInfo() : */
pub unsafe fn ZSTD_getOffsetInfo(offTable: *const ZSTD_seqSymbol, nbSeq: c_int) -> ZSTD_OffsetInfo {
    let mut info: ZSTD_OffsetInfo = ZSTD_OffsetInfo {
        longOffsetShare: 0,
        maxNbAdditionalBits: 0,
    };
    if nbSeq != 0 {
        let ptr: *const c_void = offTable as *const c_void;
        let tableLog: U32 = (*(ptr as *const ZSTD_seqSymbol_header)).tableLog;
        let table: *const ZSTD_seqSymbol = offTable.add(1);
        let max: U32 = 1 << tableLog;
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

        info.longOffsetShare <<= OffFSELog - tableLog; /* scale to OffFSELog */
    }

    info
}

/**
 * @returns The maximum offset we can decode in one read of our bitstream. */
pub unsafe fn ZSTD_maxShortOffset() -> size_t {
    if MEM_64bits() != 0 {
        (-1isize) as size_t
    } else {
        let maxOffbase: size_t = ((1 as size_t) << (STREAM_ACCUMULATOR_MIN() + 1)) - 1;
        let maxOffset: size_t = maxOffbase - ZSTD_REP_NUM;
        maxOffset
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_decompressBlock_internal(
    dctx: *mut ZSTD_DCtx,
    dst: *mut c_void,
    dstCapacity: size_t,
    src: *const c_void,
    mut srcSize: size_t,
    streaming: streaming_operation,
) -> size_t {
    /* blockType == blockCompressed */
    let mut ip: *const BYTE = src as *const BYTE;

    if srcSize > ZSTD_blockSizeMax(dctx) {
        return ERROR(ZSTD_error_srcSize_wrong);
    }

    /* Decode literals section */
    {
        let litCSize: size_t =
            ZSTD_decodeLiteralsBlock(dctx, src, srcSize, dst, dstCapacity, streaming);
        if ZSTD_isError(litCSize) != 0 {
            return litCSize;
        }
        ip = ip.add(litCSize);
        srcSize -= litCSize;
    }

    /* Build Decoding Tables */
    {
        let blockSizeMax: size_t = MIN(dstCapacity, ZSTD_blockSizeMax(dctx));
        let totalHistorySize: size_t = ZSTD_totalHistorySize(
            ZSTD_maybeNullPtrAdd(dst as *mut BYTE, blockSizeMax as isize),
            (*dctx).virtualStart as *const BYTE,
        );
        let mut isLongOffset: ZSTD_longOffset_e =
            (MEM_32bits() != 0 && (totalHistorySize > ZSTD_maxShortOffset())) as ZSTD_longOffset_e;
        let mut usePrefetchDecoder: c_int = (*dctx).ddictIsCold;
        let nbSeq: c_int;
        let mut nbSeq_v: c_int = 0;
        let seqHSize: size_t =
            ZSTD_decodeSeqHeaders(dctx, &mut nbSeq_v, ip as *const c_void, srcSize);
        if ZSTD_isError(seqHSize) != 0 {
            return seqHSize;
        }
        nbSeq = nbSeq_v;
        ip = ip.add(seqHSize);
        srcSize -= seqHSize;

        if (dst.is_null() || dstCapacity == 0) && nbSeq > 0 {
            return ERROR(ZSTD_error_dstSize_tooSmall);
        }
        if MEM_64bits() != 0
            && core::mem::size_of::<size_t>() == core::mem::size_of::<*const c_void>()
            && (!0 as size_t) - (dst as size_t) < (1 << 20) as size_t
        {
            return ERROR(ZSTD_error_dstSize_tooSmall);
        }

        if isLongOffset != 0
            || (usePrefetchDecoder == 0
                && (totalHistorySize > (1u32 << 24) as size_t)
                && (nbSeq > 8))
        {
            let info: ZSTD_OffsetInfo = ZSTD_getOffsetInfo((*dctx).OFTptr, nbSeq);
            if isLongOffset != 0 && info.maxNbAdditionalBits <= STREAM_ACCUMULATOR_MIN() {
                isLongOffset = ZSTD_lo_isRegularOffset;
            }
            if usePrefetchDecoder == 0 {
                let minShare: U32 = if MEM_64bits() != 0 { 7 } else { 20 };
                usePrefetchDecoder = (info.longOffsetShare >= minShare) as c_int;
            }
        }

        (*dctx).ddictIsCold = 0;

        if usePrefetchDecoder != 0 {
            return ZSTD_decompressSequencesLong(
                dctx, dst, dstCapacity, ip as *const c_void, srcSize, nbSeq, isLongOffset,
            );
        }

        /* else */
        if (*dctx).litBufferLocation == ZSTD_split {
            ZSTD_decompressSequencesSplitLitBuffer(
                dctx, dst, dstCapacity, ip as *const c_void, srcSize, nbSeq, isLongOffset,
            )
        } else {
            ZSTD_decompressSequences(
                dctx, dst, dstCapacity, ip as *const c_void, srcSize, nbSeq, isLongOffset,
            )
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_checkContinuity(
    dctx: *mut ZSTD_DCtx,
    dst: *const c_void,
    dstSize: size_t,
) {
    if dst != (*dctx).previousDstEnd && dstSize > 0 {
        /* not contiguous */
        (*dctx).dictEnd = (*dctx).previousDstEnd;
        (*dctx).virtualStart = ((dst as *const core::ffi::c_char) as isize
            - ((*dctx).previousDstEnd as *const core::ffi::c_char as isize
                - (*dctx).prefixStart as *const core::ffi::c_char as isize))
            as *const c_void;
        (*dctx).prefixStart = dst;
        (*dctx).previousDstEnd = dst;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_decompressBlock_deprecated(
    dctx: *mut ZSTD_DCtx,
    dst: *mut c_void,
    dstCapacity: size_t,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    let dSize: size_t;
    (*dctx).isFrameDecompression = 0;
    ZSTD_checkContinuity(dctx, dst, dstCapacity);
    dSize = ZSTD_decompressBlock_internal(dctx, dst, dstCapacity, src, srcSize, not_streaming);
    if ZSTD_isError(dSize) != 0 {
        return dSize;
    }
    (*dctx).previousDstEnd = (dst as *mut core::ffi::c_char).add(dSize) as *const c_void;
    dSize
}

/* NOTE: Must just wrap ZSTD_decompressBlock_deprecated() */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_decompressBlock(
    dctx: *mut ZSTD_DCtx,
    dst: *mut c_void,
    dstCapacity: size_t,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    ZSTD_decompressBlock_deprecated(dctx, dst, dstCapacity, src, srcSize)
}
