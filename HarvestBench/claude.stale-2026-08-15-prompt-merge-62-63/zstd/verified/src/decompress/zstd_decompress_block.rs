//! Translation of c_src/src/decompress/zstd_decompress_block.c
//! Decompression of compressed blocks (literals + sequences).
//!
//! Build configuration assumptions:
//!   DYNAMIC_BMI2 = 0
//!   ZSTD_ENABLE_ASM_X86_64_BMI2 = 0  (no assembly; C default non-bmi2 path)
//! Target: little-endian 64-bit.

#![allow(
    non_snake_case,
    non_camel_case_types,
    non_upper_case_globals,
    dead_code,
    unused_mut,
    unused_assignments,
    unused_parens
)]

use core::ffi::{c_int, c_uint, c_void};

use crate::common::allocations::{memcpy, memmove, memset};
use crate::common::bits::highbit32;
use crate::common::bitstream::{
    bit_end_of_dstream, bit_init_dstream, bit_read_bits, bit_read_bits_fast, bit_reload_dstream,
    BIT_DStream_t, STREAM_ACCUMULATOR_MIN_32, STREAM_ACCUMULATOR_MIN_64,
};
use crate::common::error::{code, err_is_error, error};
use crate::common::fse::{FSE_isError, FSE_readNCount};
use crate::common::huf_common::{HUF_flags_bmi2, HUF_flags_disableAsm, HUF_isError};
use crate::common::mem::{
    mem_32bits, mem_64bits, mem_read_le16, mem_read_le24, mem_read_le32, mem_write64,
};
use crate::common::zstd_common::ZSTD_isError;
use crate::common::zstd_internal::{
    blockProperties_t, bt_reserved, bt_rle, zstd_copy16, zstd_copy8, zstd_wildcopy, LL_bits,
    ML_bits, MaxLL, MaxML, MaxOff, MaxSeq, LONGNBSEQ, MIN_CBLOCK_SIZE,
    MIN_LITERALS_FOR_4_STREAMS, MIN_SEQUENCES_SIZE, WILDCOPY_VECLEN, ZSTD_blockHeaderSize,
    ZSTD_no_overlap, ZSTD_overlap_src_before_dst, set_basic, set_compressed, set_repeat, set_rle,
    LL_DEFAULTNORMLOG, ML_DEFAULTNORMLOG, OF_DEFAULTNORMLOG,
};
use crate::decompress::zstd_decompress_internal::{
    seqsymbol_table_size, ZSTD_seqSymbol, ZSTD_seqSymbol_header, HUF_DTable,
    LLFSELog, LL_base, MLFSELog, ML_base, OffFSELog,
    OF_base, OF_bits, ZSTD_DCtx, ZSTD_LITBUFFEREXTRASIZE, ZSTD_REP_NUM,
    ZSTD_in_dst, ZSTD_not_in_dst, ZSTD_split, WILDCOPY_OVERLENGTH,
};
use crate::zstd_h::ZSTD_BLOCKSIZE_MAX;

type BYTE = u8;
type U16 = u16;
type U32 = u32;
type U64 = u64;
type S16 = i16;

// streaming_operation (zstd_decompress_block.h) — C enum == int
pub type streaming_operation = c_int;
pub const not_streaming: c_int = 0;
pub const is_streaming: c_int = 1;

// ZSTD_longOffset_e
type ZSTD_longOffset_e = c_int;
const ZSTD_lo_isRegularOffset: c_int = 0;
const ZSTD_lo_isLongOffset: c_int = 1;

// ZSTD_overlap_e (matches u32 used by zstd_wildcopy)
type ZSTD_overlap_e = u32;

const CACHELINE_SIZE: usize = 64;
const ZSTD_WINDOWLOG_MAX_32: u32 = 30;

// LONG_OFFSETS_MAX_EXTRA_BITS_32
const LONG_OFFSETS_MAX_EXTRA_BITS_32: u32 = if ZSTD_WINDOWLOG_MAX_32 > STREAM_ACCUMULATOR_MIN_32 {
    ZSTD_WINDOWLOG_MAX_32 - STREAM_ACCUMULATOR_MIN_32
} else {
    0
};

#[inline]
const fn FSE_TABLESTEP(tableSize: usize) -> usize {
    (tableSize >> 1) + (tableSize >> 3) + 3
}

// ZSTD_DCtx_get_bmi2 -> 0 (DYNAMIC_BMI2 = 0)
#[inline]
unsafe fn ZSTD_DCtx_get_bmi2(_dctx: *const ZSTD_DCtx) -> c_int {
    0
}

// compiler.h helpers
#[inline]
unsafe fn ZSTD_wrappedPtrAdd(ptr: *const u8, add: isize) -> *const u8 {
    (ptr as usize).wrapping_add(add as usize) as *const u8
}
#[inline]
unsafe fn ZSTD_wrappedPtrSub(ptr: *const u8, sub: isize) -> *const u8 {
    (ptr as usize).wrapping_sub(sub as usize) as *const u8
}
#[inline]
unsafe fn ZSTD_maybeNullPtrAdd(ptr: *mut u8, add: isize) -> *mut u8 {
    if add > 0 {
        ptr.wrapping_offset(add)
    } else {
        ptr
    }
}

// PREFETCH is disabled in this build configuration.
#[inline]
fn PREFETCH_L1(_ptr: *const u8) {}
#[inline]
fn PREFETCH_AREA(_ptr: *const c_void, _size: usize) {}

// Byte difference of two pointers as ptrdiff_t (wrapping, no UB).
#[inline]
fn ptr_diff(a: *const u8, b: *const u8) -> isize {
    (a as usize).wrapping_sub(b as usize) as isize
}

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
}

// Compact constructor for the default FSE decoding tables.
#[inline]
const fn ss(nextState: u16, nbAdditionalBits: u8, nbBits: u8, baseValue: u32) -> ZSTD_seqSymbol {
    ZSTD_seqSymbol {
        nextState,
        nbAdditionalBits,
        nbBits,
        baseValue,
    }
}

/* Default FSE distribution table for Literal Lengths */
static LL_defaultDTable: [ZSTD_seqSymbol; (1 << LL_DEFAULTNORMLOG) + 1] = [
    ss(1, 1, 1, LL_DEFAULTNORMLOG), /* header : fastMode, tableLog */
    ss(0, 0, 4, 0), ss(16, 0, 4, 0),
    ss(32, 0, 5, 1), ss(0, 0, 5, 3),
    ss(0, 0, 5, 4), ss(0, 0, 5, 6),
    ss(0, 0, 5, 7), ss(0, 0, 5, 9),
    ss(0, 0, 5, 10), ss(0, 0, 5, 12),
    ss(0, 0, 6, 14), ss(0, 1, 5, 16),
    ss(0, 1, 5, 20), ss(0, 1, 5, 22),
    ss(0, 2, 5, 28), ss(0, 3, 5, 32),
    ss(0, 4, 5, 48), ss(32, 6, 5, 64),
    ss(0, 7, 5, 128), ss(0, 8, 6, 256),
    ss(0, 10, 6, 1024), ss(0, 12, 6, 4096),
    ss(32, 0, 4, 0), ss(0, 0, 4, 1),
    ss(0, 0, 5, 2), ss(32, 0, 5, 4),
    ss(0, 0, 5, 5), ss(32, 0, 5, 7),
    ss(0, 0, 5, 8), ss(32, 0, 5, 10),
    ss(0, 0, 5, 11), ss(0, 0, 6, 13),
    ss(32, 1, 5, 16), ss(0, 1, 5, 18),
    ss(32, 1, 5, 22), ss(0, 2, 5, 24),
    ss(32, 3, 5, 32), ss(0, 3, 5, 40),
    ss(0, 6, 4, 64), ss(16, 6, 4, 64),
    ss(32, 7, 5, 128), ss(0, 9, 6, 512),
    ss(0, 11, 6, 2048), ss(48, 0, 4, 0),
    ss(16, 0, 4, 1), ss(32, 0, 5, 2),
    ss(32, 0, 5, 3), ss(32, 0, 5, 5),
    ss(32, 0, 5, 6), ss(32, 0, 5, 8),
    ss(32, 0, 5, 9), ss(32, 0, 5, 11),
    ss(32, 0, 5, 12), ss(0, 0, 6, 15),
    ss(32, 1, 5, 18), ss(32, 1, 5, 20),
    ss(32, 2, 5, 24), ss(32, 2, 5, 28),
    ss(32, 3, 5, 40), ss(32, 4, 5, 48),
    ss(0, 16, 6, 65536), ss(0, 15, 6, 32768),
    ss(0, 14, 6, 16384), ss(0, 13, 6, 8192),
]; /* LL_defaultDTable */

/* Default FSE distribution table for Offset Codes */
static OF_defaultDTable: [ZSTD_seqSymbol; (1 << OF_DEFAULTNORMLOG) + 1] = [
    ss(1, 1, 1, OF_DEFAULTNORMLOG), /* header : fastMode, tableLog */
    ss(0, 0, 5, 0), ss(0, 6, 4, 61),
    ss(0, 9, 5, 509), ss(0, 15, 5, 32765),
    ss(0, 21, 5, 2097149), ss(0, 3, 5, 5),
    ss(0, 7, 4, 125), ss(0, 12, 5, 4093),
    ss(0, 18, 5, 262141), ss(0, 23, 5, 8388605),
    ss(0, 5, 5, 29), ss(0, 8, 4, 253),
    ss(0, 14, 5, 16381), ss(0, 20, 5, 1048573),
    ss(0, 2, 5, 1), ss(16, 7, 4, 125),
    ss(0, 11, 5, 2045), ss(0, 17, 5, 131069),
    ss(0, 22, 5, 4194301), ss(0, 4, 5, 13),
    ss(16, 8, 4, 253), ss(0, 13, 5, 8189),
    ss(0, 19, 5, 524285), ss(0, 1, 5, 1),
    ss(16, 6, 4, 61), ss(0, 10, 5, 1021),
    ss(0, 16, 5, 65533), ss(0, 28, 5, 268435453),
    ss(0, 27, 5, 134217725), ss(0, 26, 5, 67108861),
    ss(0, 25, 5, 33554429), ss(0, 24, 5, 16777213),
]; /* OF_defaultDTable */

/* Default FSE distribution table for Match Lengths */
static ML_defaultDTable: [ZSTD_seqSymbol; (1 << ML_DEFAULTNORMLOG) + 1] = [
    ss(1, 1, 1, ML_DEFAULTNORMLOG), /* header : fastMode, tableLog */
    ss(0, 0, 6, 3), ss(0, 0, 4, 4),
    ss(32, 0, 5, 5), ss(0, 0, 5, 6),
    ss(0, 0, 5, 8), ss(0, 0, 5, 9),
    ss(0, 0, 5, 11), ss(0, 0, 6, 13),
    ss(0, 0, 6, 16), ss(0, 0, 6, 19),
    ss(0, 0, 6, 22), ss(0, 0, 6, 25),
    ss(0, 0, 6, 28), ss(0, 0, 6, 31),
    ss(0, 0, 6, 34), ss(0, 1, 6, 37),
    ss(0, 1, 6, 41), ss(0, 2, 6, 47),
    ss(0, 3, 6, 59), ss(0, 4, 6, 83),
    ss(0, 7, 6, 131), ss(0, 9, 6, 515),
    ss(16, 0, 4, 4), ss(0, 0, 4, 5),
    ss(32, 0, 5, 6), ss(0, 0, 5, 7),
    ss(32, 0, 5, 9), ss(0, 0, 5, 10),
    ss(0, 0, 6, 12), ss(0, 0, 6, 15),
    ss(0, 0, 6, 18), ss(0, 0, 6, 21),
    ss(0, 0, 6, 24), ss(0, 0, 6, 27),
    ss(0, 0, 6, 30), ss(0, 0, 6, 33),
    ss(0, 1, 6, 35), ss(0, 1, 6, 39),
    ss(0, 2, 6, 43), ss(0, 3, 6, 51),
    ss(0, 4, 6, 67), ss(0, 5, 6, 99),
    ss(0, 8, 6, 259), ss(32, 0, 4, 4),
    ss(48, 0, 4, 4), ss(16, 0, 4, 5),
    ss(32, 0, 5, 7), ss(32, 0, 5, 8),
    ss(32, 0, 5, 10), ss(32, 0, 5, 11),
    ss(0, 0, 6, 14), ss(0, 0, 6, 17),
    ss(0, 0, 6, 20), ss(0, 0, 6, 23),
    ss(0, 0, 6, 26), ss(0, 0, 6, 29),
    ss(0, 0, 6, 32), ss(0, 16, 6, 65539),
    ss(0, 15, 6, 32771), ss(0, 14, 6, 16387),
    ss(0, 13, 6, 8195), ss(0, 12, 6, 4099),
    ss(0, 11, 6, 2051), ss(0, 10, 6, 1027),
]; /* ML_defaultDTable */

/*-*************************************************************
 *   Memory operations
 ***************************************************************/
#[inline]
unsafe fn ZSTD_copy4(dst: *mut c_void, src: *const c_void) {
    memcpy(dst, src, 4);
}

/*-*************************************************************
 *   Block decoding
 ***************************************************************/
unsafe fn ZSTD_blockSizeMax(dctx: *const ZSTD_DCtx) -> usize {
    let blockSizeMax = if (*dctx).isFrameDecompression != 0 {
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
        return error(code::SRCSIZE_WRONG);
    }

    {
        let cBlockHeader = mem_read_le24(src);
        let cSize = (cBlockHeader >> 3) as usize;
        (*bpPtr).lastBlock = cBlockHeader & 1;
        (*bpPtr).blockType = (cBlockHeader >> 1) & 3;
        (*bpPtr).origSize = cBlockHeader >> 3; /* only useful for RLE */
        if (*bpPtr).blockType == bt_rle {
            return 1;
        }
        if (*bpPtr).blockType == bt_reserved {
            return error(code::CORRUPTION_DETECTED);
        }
        cSize
    }
}

/* Allocate buffer for literals, either overlapping current dst, or split between dst and litExtraBuffer, or stored entirely within litExtraBuffer */
unsafe fn ZSTD_allocateLiteralsBuffer(
    dctx: *mut ZSTD_DCtx,
    dst: *mut c_void,
    dstCapacity: usize,
    litSize: usize,
    streaming: streaming_operation,
    expectedWriteSize: usize,
    splitImmediately: c_uint,
) {
    let blockSizeMax = ZSTD_blockSizeMax(dctx);
    if streaming == not_streaming
        && dstCapacity > blockSizeMax + WILDCOPY_OVERLENGTH + litSize + WILDCOPY_OVERLENGTH
    {
        /* If we aren't streaming, we can just put the literals after the output
         * of the current block. */
        (*dctx).litBuffer = (dst as *mut u8).add(blockSizeMax + WILDCOPY_OVERLENGTH);
        (*dctx).litBufferEnd = (*dctx).litBuffer.add(litSize);
        (*dctx).litBufferLocation = ZSTD_in_dst;
    } else if litSize <= ZSTD_LITBUFFEREXTRASIZE {
        /* Literals fit entirely within the extra buffer */
        (*dctx).litBuffer = (*dctx).litExtraBuffer.as_mut_ptr();
        (*dctx).litBufferEnd = (*dctx).litBuffer.add(litSize);
        (*dctx).litBufferLocation = ZSTD_not_in_dst;
    } else {
        /* Literals must be split between the output block and the extra lit buffer. */
        if splitImmediately != 0 {
            /* won't fit in litExtraBuffer, so it will be split between end of dst and extra buffer */
            (*dctx).litBuffer = (dst as *mut u8)
                .add(expectedWriteSize - litSize + ZSTD_LITBUFFEREXTRASIZE - WILDCOPY_OVERLENGTH);
            (*dctx).litBufferEnd = (*dctx).litBuffer.add(litSize - ZSTD_LITBUFFEREXTRASIZE);
        } else {
            /* initially this will be stored entirely in dst during huffman decoding, it will partially be shifted to litExtraBuffer after */
            (*dctx).litBuffer = (dst as *mut u8).add(expectedWriteSize - litSize);
            (*dctx).litBufferEnd = (dst as *mut u8).add(expectedWriteSize);
        }
        (*dctx).litBufferLocation = ZSTD_split;
    }
}

#[inline]
fn MIN_usize(a: usize, b: usize) -> usize {
    if a < b {
        a
    } else {
        b
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
        return error(code::CORRUPTION_DETECTED);
    }

    {
        let istart = src as *const BYTE;
        let litEncType: u32 = (*istart.add(0) & 3) as u32;
        let blockSizeMax = ZSTD_blockSizeMax(dctx);

        if litEncType == set_repeat || litEncType == set_compressed {
            if litEncType == set_repeat {
                /* set_repeat flag : re-using stats from previous compressed literals block */
                if (*dctx).litEntropy == 0 {
                    return error(code::DICTIONARY_CORRUPTED);
                }
            }
            /* FALLTHROUGH into set_compressed */
            if srcSize < 5 {
                return error(code::CORRUPTION_DETECTED);
            }
            {
                let lhSize: usize;
                let litSize: usize;
                let litCSize: usize;
                let mut singleStream: U32 = 0;
                let lhlCode: U32 = ((*istart.add(0) >> 2) & 3) as U32;
                let lhc: U32 = mem_read_le32(istart as *const c_void);
                let hufSuccess: usize;
                let expectedWriteSize = MIN_usize(blockSizeMax, dstCapacity);
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
                        /* case 0, 1, default: 2 - 2 - 10 - 10 */
                        singleStream = (lhlCode == 0) as U32;
                        lhSize = 3;
                        litSize = ((lhc >> 4) & 0x3FF) as usize;
                        litCSize = ((lhc >> 14) & 0x3FF) as usize;
                    }
                }
                if litSize > 0 && dst.is_null() {
                    return error(code::DSTSIZE_TOOSMALL);
                }
                if litSize > blockSizeMax {
                    return error(code::CORRUPTION_DETECTED);
                }
                if singleStream == 0 {
                    if litSize < MIN_LITERALS_FOR_4_STREAMS {
                        return error(code::LITERALS_HEADERWRONG);
                    }
                }
                if litCSize + lhSize > srcSize {
                    return error(code::CORRUPTION_DETECTED);
                }
                if expectedWriteSize < litSize {
                    return error(code::DSTSIZE_TOOSMALL);
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

                /* prefetch huffman table if cold */
                if (*dctx).ddictIsCold != 0 && (litSize > 768) {
                    PREFETCH_AREA(
                        (*dctx).HUFptr as *const c_void,
                        core::mem::size_of_val(&(*dctx).entropy.hufTable),
                    );
                }

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
                    memcpy(
                        (*dctx).litExtraBuffer.as_mut_ptr() as *mut c_void,
                        ((*dctx).litBufferEnd as *const u8).sub(ZSTD_LITBUFFEREXTRASIZE)
                            as *const c_void,
                        ZSTD_LITBUFFEREXTRASIZE,
                    );
                    memmove(
                        (*dctx)
                            .litBuffer
                            .add(ZSTD_LITBUFFEREXTRASIZE - WILDCOPY_OVERLENGTH)
                            as *mut c_void,
                        (*dctx).litBuffer as *const c_void,
                        litSize - ZSTD_LITBUFFEREXTRASIZE,
                    );
                    (*dctx).litBuffer = (*dctx)
                        .litBuffer
                        .add(ZSTD_LITBUFFEREXTRASIZE - WILDCOPY_OVERLENGTH);
                    (*dctx).litBufferEnd = ((*dctx).litBufferEnd as *const u8).sub(WILDCOPY_OVERLENGTH);
                }

                if HUF_isError(hufSuccess) != 0 {
                    return error(code::CORRUPTION_DETECTED);
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
                let litSize: usize;
                let lhSize: usize;
                let lhlCode: U32 = ((*istart.add(0) >> 2) & 3) as U32;
                let expectedWriteSize = MIN_usize(blockSizeMax, dstCapacity);
                match lhlCode {
                    1 => {
                        lhSize = 2;
                        litSize = (mem_read_le16(istart as *const c_void) >> 4) as usize;
                    }
                    3 => {
                        lhSize = 3;
                        if srcSize < 3 {
                            return error(code::CORRUPTION_DETECTED);
                        }
                        litSize = (mem_read_le24(istart as *const c_void) >> 4) as usize;
                    }
                    _ => {
                        /* case 0, 2, default */
                        lhSize = 1;
                        litSize = (*istart.add(0) >> 3) as usize;
                    }
                }

                if litSize > 0 && dst.is_null() {
                    return error(code::DSTSIZE_TOOSMALL);
                }
                if litSize > blockSizeMax {
                    return error(code::CORRUPTION_DETECTED);
                }
                if expectedWriteSize < litSize {
                    return error(code::DSTSIZE_TOOSMALL);
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
                if lhSize + litSize + WILDCOPY_OVERLENGTH > srcSize {
                    /* risk reading beyond src buffer with wildcopy */
                    if litSize + lhSize > srcSize {
                        return error(code::CORRUPTION_DETECTED);
                    }
                    if (*dctx).litBufferLocation == ZSTD_split {
                        memcpy(
                            (*dctx).litBuffer as *mut c_void,
                            istart.add(lhSize) as *const c_void,
                            litSize - ZSTD_LITBUFFEREXTRASIZE,
                        );
                        memcpy(
                            (*dctx).litExtraBuffer.as_mut_ptr() as *mut c_void,
                            istart.add(lhSize + litSize - ZSTD_LITBUFFEREXTRASIZE) as *const c_void,
                            ZSTD_LITBUFFEREXTRASIZE,
                        );
                    } else {
                        memcpy(
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
        } else if litEncType == set_rle {
            {
                let lhlCode: U32 = ((*istart.add(0) >> 2) & 3) as U32;
                let litSize: usize;
                let lhSize: usize;
                let expectedWriteSize = MIN_usize(blockSizeMax, dstCapacity);
                match lhlCode {
                    1 => {
                        lhSize = 2;
                        if srcSize < 3 {
                            return error(code::CORRUPTION_DETECTED);
                        }
                        litSize = (mem_read_le16(istart as *const c_void) >> 4) as usize;
                    }
                    3 => {
                        lhSize = 3;
                        if srcSize < 4 {
                            return error(code::CORRUPTION_DETECTED);
                        }
                        litSize = (mem_read_le24(istart as *const c_void) >> 4) as usize;
                    }
                    _ => {
                        /* case 0, 2, default */
                        lhSize = 1;
                        litSize = (*istart.add(0) >> 3) as usize;
                    }
                }
                if litSize > 0 && dst.is_null() {
                    return error(code::DSTSIZE_TOOSMALL);
                }
                if litSize > blockSizeMax {
                    return error(code::CORRUPTION_DETECTED);
                }
                if expectedWriteSize < litSize {
                    return error(code::DSTSIZE_TOOSMALL);
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
                    memset(
                        (*dctx).litBuffer as *mut c_void,
                        *istart.add(lhSize) as c_int,
                        litSize - ZSTD_LITBUFFEREXTRASIZE,
                    );
                    memset(
                        (*dctx).litExtraBuffer.as_mut_ptr() as *mut c_void,
                        *istart.add(lhSize) as c_int,
                        ZSTD_LITBUFFEREXTRASIZE,
                    );
                } else {
                    memset(
                        (*dctx).litBuffer as *mut c_void,
                        *istart.add(lhSize) as c_int,
                        litSize,
                    );
                }
                (*dctx).litPtr = (*dctx).litBuffer;
                (*dctx).litSize = litSize;
                return lhSize + 1;
            }
        } else {
            return error(code::CORRUPTION_DETECTED);
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

type U8 = u8;

/* ZSTD_buildFSETable() :
 * generate FSE decoding table for one symbol (ll, ml or off) */
unsafe fn ZSTD_buildFSETable_body(
    dt: *mut ZSTD_seqSymbol,
    normalizedCounter: *const S16,
    maxSymbolValue: c_uint,
    baseValue: *const U32,
    nbAdditionalBits: *const U8,
    tableLog: c_uint,
    wksp: *mut c_void,
    _wkspSize: usize,
) {
    let tableDecode = dt.add(1);
    let maxSV1: U32 = maxSymbolValue + 1;
    let tableSize: U32 = 1 << tableLog;

    let symbolNext = wksp as *mut U16;
    let spread = symbolNext.add(MaxSeq as usize + 1) as *mut BYTE;
    let mut highThreshold: U32 = tableSize - 1;

    /* Init, lay down lowprob symbols */
    {
        let mut DTableH = ZSTD_seqSymbol_header {
            fastMode: 1,
            tableLog: tableLog,
        };
        {
            let largeLimit: S16 = (1_i32 << (tableLog - 1)) as S16;
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
        core::ptr::copy_nonoverlapping(
            &DTableH as *const ZSTD_seqSymbol_header as *const u8,
            dt as *mut u8,
            core::mem::size_of::<ZSTD_seqSymbol_header>(),
        );
    }

    /* Spread symbols */
    if highThreshold == tableSize - 1 {
        let tableMask = (tableSize - 1) as usize;
        let step = FSE_TABLESTEP(tableSize as usize);
        {
            let add: U64 = 0x0101010101010101;
            let mut pos: usize = 0;
            let mut sv: U64 = 0;
            let mut s: U32 = 0;
            while s < maxSV1 {
                let n: c_int = *normalizedCounter.add(s as usize) as c_int;
                mem_write64(spread.add(pos) as *mut c_void, sv);
                let mut i: c_int = 8;
                while i < n {
                    mem_write64(spread.add(pos + i as usize) as *mut c_void, sv);
                    i += 8;
                }
                pos += n as usize;
                s += 1;
                sv = sv.wrapping_add(add);
            }
        }
        {
            let mut position: usize = 0;
            let unroll: usize = 2;
            let mut s: usize = 0;
            while s < tableSize as usize {
                let mut u: usize = 0;
                while u < unroll {
                    let uPosition = (position + (u * step)) & tableMask;
                    (*tableDecode.add(uPosition)).baseValue = *spread.add(s + u) as U32;
                    u += 1;
                }
                position = (position + (unroll * step)) & tableMask;
                s += unroll;
            }
        }
    } else {
        let tableMask: U32 = tableSize - 1;
        let step: U32 = FSE_TABLESTEP(tableSize as usize) as U32;
        let mut position: U32 = 0;
        let mut s: U32 = 0;
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
            *symbolNext.add(symbol as usize) = (nextState + 1) as U16;
            (*tableDecode.add(u as usize)).nbBits = (tableLog - highbit32(nextState)) as BYTE;
            (*tableDecode.add(u as usize)).nextState =
                ((nextState << (*tableDecode.add(u as usize)).nbBits) - tableSize) as U16;
            (*tableDecode.add(u as usize)).nbAdditionalBits = *nbAdditionalBits.add(symbol as usize);
            (*tableDecode.add(u as usize)).baseValue = *baseValue.add(symbol as usize);
            u += 1;
        }
    }
}

/* Avoids the FORCE_INLINE of the _body() function. */
unsafe fn ZSTD_buildFSETable_body_default(
    dt: *mut ZSTD_seqSymbol,
    normalizedCounter: *const S16,
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
    normalizedCounter: *const S16,
    maxSymbolValue: c_uint,
    baseValue: *const U32,
    nbAdditionalBits: *const U8,
    tableLog: c_uint,
    wksp: *mut c_void,
    wkspSize: usize,
    _bmi2: c_int,
) {
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
    type_: u32,
    max: c_uint,
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
            return error(code::SRCSIZE_WRONG);
        }
        if (*(src as *const BYTE)) as c_uint > max {
            return error(code::CORRUPTION_DETECTED);
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
            return error(code::CORRUPTION_DETECTED);
        }
        /* prefetch FSE table if used */
        if ddictIsCold != 0 && (nbSeq > 24) {
            let pStart = *DTablePtr as *const c_void;
            let pSize =
                core::mem::size_of::<ZSTD_seqSymbol>() * seqsymbol_table_size(maxLog);
            PREFETCH_AREA(pStart, pSize);
        }
        return 0;
    } else if type_ == set_compressed {
        let mut tableLog: c_uint = 0;
        let mut norm: [S16; MaxSeq as usize + 1] = [0; MaxSeq as usize + 1];
        let mut maxv: c_uint = max;
        let headerSize =
            FSE_readNCount(norm.as_mut_ptr(), &mut maxv, &mut tableLog, src, srcSize);
        if FSE_isError(headerSize) != 0 {
            return error(code::CORRUPTION_DETECTED);
        }
        if tableLog > maxLog {
            return error(code::CORRUPTION_DETECTED);
        }
        ZSTD_buildFSETable(
            DTableSpace,
            norm.as_ptr(),
            maxv,
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
        return error(code::GENERIC);
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
    let iend = istart.add(srcSize);
    let mut ip = istart;
    let mut nbSeq: c_int;

    /* check */
    if srcSize < MIN_SEQUENCES_SIZE {
        return error(code::SRCSIZE_WRONG);
    }

    /* SeqHead */
    nbSeq = *ip as c_int;
    ip = ip.add(1);
    if nbSeq > 0x7F {
        if nbSeq == 0xFF {
            if ip.add(2) > iend {
                return error(code::SRCSIZE_WRONG);
            }
            nbSeq = mem_read_le16(ip as *const c_void) as c_int + LONGNBSEQ as c_int;
            ip = ip.add(2);
        } else {
            if ip >= iend {
                return error(code::SRCSIZE_WRONG);
            }
            nbSeq = ((nbSeq - 0x80) << 8) + *ip as c_int;
            ip = ip.add(1);
        }
    }
    *nbSeqPtr = nbSeq;

    if nbSeq == 0 {
        /* No sequence : section ends immediately */
        if ip != iend {
            return error(code::CORRUPTION_DETECTED);
        }
        return ptr_diff(ip, istart) as usize;
    }

    /* FSE table descriptors */
    if ip.add(1) > iend {
        return error(code::SRCSIZE_WRONG);
    }
    if *ip & 3 != 0 {
        return error(code::CORRUPTION_DETECTED);
    }
    {
        let LLtype: u32 = (*ip >> 6) as u32;
        let OFtype: u32 = ((*ip >> 4) & 3) as u32;
        let MLtype: u32 = ((*ip >> 2) & 3) as u32;
        ip = ip.add(1);

        /* Build DTables */
        {
            let llhSize = ZSTD_buildSeqTable(
                (*dctx).entropy.LLTable.as_mut_ptr(),
                &mut (*dctx).LLTptr,
                LLtype,
                MaxLL,
                LLFSELog,
                ip as *const c_void,
                ptr_diff(iend, ip) as usize,
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
                return error(code::CORRUPTION_DETECTED);
            }
            ip = ip.add(llhSize);
        }

        {
            let ofhSize = ZSTD_buildSeqTable(
                (*dctx).entropy.OFTable.as_mut_ptr(),
                &mut (*dctx).OFTptr,
                OFtype,
                MaxOff,
                OffFSELog,
                ip as *const c_void,
                ptr_diff(iend, ip) as usize,
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
                return error(code::CORRUPTION_DETECTED);
            }
            ip = ip.add(ofhSize);
        }

        {
            let mlhSize = ZSTD_buildSeqTable(
                (*dctx).entropy.MLTable.as_mut_ptr(),
                &mut (*dctx).MLTptr,
                MLtype,
                MaxML,
                MLFSELog,
                ip as *const c_void,
                ptr_diff(iend, ip) as usize,
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
                return error(code::CORRUPTION_DETECTED);
            }
            ip = ip.add(mlhSize);
        }
    }

    ptr_diff(ip, istart) as usize
}

#[derive(Clone, Copy)]
struct seq_t {
    litLength: usize,
    matchLength: usize,
    offset: usize,
}

#[derive(Clone, Copy)]
struct ZSTD_fseState {
    state: usize,
    table: *const ZSTD_seqSymbol,
}

struct seqState_t {
    DStream: BIT_DStream_t,
    stateLL: ZSTD_fseState,
    stateOffb: ZSTD_fseState,
    stateML: ZSTD_fseState,
    prevOffset: [usize; ZSTD_REP_NUM],
}

/* ZSTD_overlapCopy8() :
 *  Copies 8 bytes from ip to op and updates op and ip where ip <= op. */
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
        *ip = (*ip).add(dec32table[offset] as usize);
        ZSTD_copy4((*op).add(4) as *mut c_void, *ip as *const c_void);
        *ip = (*ip).offset(-(sub2 as isize));
    } else {
        zstd_copy8(*op as *mut c_void, *ip as *const c_void);
    }
    *ip = (*ip).add(8);
    *op = (*op).add(8);
}

/* ZSTD_safecopy() :
 *  Specialized version of memcpy() that is allowed to READ up to WILDCOPY_OVERLENGTH past the input buffer. */
unsafe fn ZSTD_safecopy(
    mut op: *mut BYTE,
    oend_w: *const BYTE,
    mut ip: *const BYTE,
    mut length: isize,
    ovtype: ZSTD_overlap_e,
) {
    let diff = ptr_diff(op, ip);
    let oend = op.offset(length);

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
        zstd_wildcopy(op as *mut c_void, ip as *const c_void, length, ovtype);
        return;
    }
    if op <= oend_w as *mut BYTE {
        /* Wildcopy until we get close to the end. */
        zstd_wildcopy(
            op as *mut c_void,
            ip as *const c_void,
            ptr_diff(oend_w, op),
            ovtype,
        );
        ip = ip.offset(ptr_diff(oend_w, op));
        op = op.offset(ptr_diff(oend_w, op));
    }
    /* Handle the leftovers. */
    while op < oend {
        *op = *ip;
        op = op.add(1);
        ip = ip.add(1);
    }
}

/* ZSTD_safecopyDstBeforeSrc():
 * This version allows overlap with dst before src, or handles the non-overlap case with dst after src */
unsafe fn ZSTD_safecopyDstBeforeSrc(mut op: *mut BYTE, mut ip: *const BYTE, length: isize) {
    let diff = ptr_diff(op, ip);
    let oend = op.offset(length);

    if length < 8 || diff > -8 {
        /* Handle short lengths, close overlaps, and dst not before src. */
        while op < oend {
            *op = *ip;
            op = op.add(1);
            ip = ip.add(1);
        }
        return;
    }

    if op <= oend.offset(-(WILDCOPY_OVERLENGTH as isize)) && diff < -(WILDCOPY_VECLEN as isize) {
        zstd_wildcopy(
            op as *mut c_void,
            ip as *const c_void,
            ptr_diff(oend.offset(-(WILDCOPY_OVERLENGTH as isize)), op),
            ZSTD_no_overlap,
        );
        ip = ip.offset(ptr_diff(oend.offset(-(WILDCOPY_OVERLENGTH as isize)), op));
        op = op.offset(ptr_diff(oend.offset(-(WILDCOPY_OVERLENGTH as isize)), op));
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
    let oLitEnd = op.add(sequence.litLength);
    let sequenceLength = sequence.litLength + sequence.matchLength;
    let iLitEnd = (*litPtr).add(sequence.litLength);
    let mut match_: *const BYTE = oLitEnd.offset(-(sequence.offset as isize)) as *const BYTE;
    let oend_w = oend.offset(-(WILDCOPY_OVERLENGTH as isize));

    /* bounds checks : careful of address space overflow in 32-bit mode */
    if sequenceLength > ptr_diff(oend, op) as usize {
        return error(code::DSTSIZE_TOOSMALL);
    }
    if sequence.litLength > ptr_diff(litLimit, *litPtr) as usize {
        return error(code::CORRUPTION_DETECTED);
    }

    /* copy literals */
    ZSTD_safecopy(op, oend_w, *litPtr, sequence.litLength as isize, ZSTD_no_overlap);
    op = oLitEnd;
    *litPtr = iLitEnd;

    /* copy Match */
    if sequence.offset > ptr_diff(oLitEnd, prefixStart) as usize {
        /* offset beyond prefix */
        if sequence.offset > ptr_diff(oLitEnd, virtualStart) as usize {
            return error(code::CORRUPTION_DETECTED);
        }
        match_ = (dictEnd as *const BYTE)
            .offset(-(ptr_diff(prefixStart, match_)));
        if match_.add(sequence.matchLength) <= dictEnd {
            memmove(
                oLitEnd as *mut c_void,
                match_ as *const c_void,
                sequence.matchLength,
            );
            return sequenceLength;
        }
        /* span extDict & currentPrefixSegment */
        {
            let length1 = ptr_diff(dictEnd, match_) as usize;
            memmove(oLitEnd as *mut c_void, match_ as *const c_void, length1);
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
    let oLitEnd = op.add(sequence.litLength);
    let sequenceLength = sequence.litLength + sequence.matchLength;
    let iLitEnd = (*litPtr).add(sequence.litLength);
    let mut match_: *const BYTE = oLitEnd.offset(-(sequence.offset as isize)) as *const BYTE;

    /* bounds checks : careful of address space overflow in 32-bit mode */
    if sequenceLength > ptr_diff(oend, op) as usize {
        return error(code::DSTSIZE_TOOSMALL);
    }
    if sequence.litLength > ptr_diff(litLimit, *litPtr) as usize {
        return error(code::CORRUPTION_DETECTED);
    }

    /* copy literals */
    if op > *litPtr as *mut BYTE && op < (*litPtr).add(sequence.litLength) as *mut BYTE {
        return error(code::DSTSIZE_TOOSMALL);
    }
    ZSTD_safecopyDstBeforeSrc(op, *litPtr, sequence.litLength as isize);
    op = oLitEnd;
    *litPtr = iLitEnd;

    /* copy Match */
    if sequence.offset > ptr_diff(oLitEnd, prefixStart) as usize {
        /* offset beyond prefix */
        if sequence.offset > ptr_diff(oLitEnd, virtualStart) as usize {
            return error(code::CORRUPTION_DETECTED);
        }
        match_ = (dictEnd as *const BYTE).offset(-(ptr_diff(prefixStart, match_)));
        if match_.add(sequence.matchLength) <= dictEnd {
            memmove(
                oLitEnd as *mut c_void,
                match_ as *const c_void,
                sequence.matchLength,
            );
            return sequenceLength;
        }
        /* span extDict & currentPrefixSegment */
        {
            let length1 = ptr_diff(dictEnd, match_) as usize;
            memmove(oLitEnd as *mut c_void, match_ as *const c_void, length1);
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
    let oLitEnd = op.add(sequence.litLength);
    let sequenceLength = sequence.litLength + sequence.matchLength;
    let oMatchEnd = op.add(sequenceLength); /* risk : address space overflow (32-bits) */
    let oend_w = oend.offset(-(WILDCOPY_OVERLENGTH as isize)); /* risk : address space underflow on oend=NULL */
    let iLitEnd = (*litPtr).add(sequence.litLength);
    let mut match_: *const BYTE = oLitEnd.offset(-(sequence.offset as isize)) as *const BYTE;

    /* Handle edge cases in a slow path */
    if iLitEnd > litLimit
        || (oMatchEnd as *const BYTE) > oend_w
        || (mem_32bits() != 0
            && (ptr_diff(oend, op) as usize) < sequenceLength + WILDCOPY_OVERLENGTH)
    {
        return ZSTD_execSequenceEnd(
            op, oend, sequence, litPtr, litLimit, prefixStart, virtualStart, dictEnd,
        );
    }

    /* Copy Literals */
    zstd_copy16(op as *mut c_void, *litPtr as *const c_void);
    if sequence.litLength > 16 {
        zstd_wildcopy(
            op.add(16) as *mut c_void,
            (*litPtr).add(16) as *const c_void,
            (sequence.litLength - 16) as isize,
            ZSTD_no_overlap,
        );
    }
    op = oLitEnd;
    *litPtr = iLitEnd; /* update for next sequence */

    /* Copy Match */
    if sequence.offset > ptr_diff(oLitEnd, prefixStart) as usize {
        /* offset beyond prefix -> go into extDict */
        if sequence.offset > ptr_diff(oLitEnd, virtualStart) as usize {
            return error(code::CORRUPTION_DETECTED);
        }
        match_ = (dictEnd as *const BYTE).offset(ptr_diff(match_, prefixStart));
        if match_.add(sequence.matchLength) <= dictEnd {
            memmove(
                oLitEnd as *mut c_void,
                match_ as *const c_void,
                sequence.matchLength,
            );
            return sequenceLength;
        }
        /* span extDict & currentPrefixSegment */
        {
            let length1 = ptr_diff(dictEnd, match_) as usize;
            memmove(oLitEnd as *mut c_void, match_ as *const c_void, length1);
            op = oLitEnd.add(length1);
            sequence.matchLength -= length1;
            match_ = prefixStart;
        }
    }
    /* Match within prefix of 1 or more bytes */

    if sequence.offset >= WILDCOPY_VECLEN {
        zstd_wildcopy(
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
        zstd_wildcopy(
            op as *mut c_void,
            match_ as *const c_void,
            (sequence.matchLength - 8) as isize,
            ZSTD_overlap_src_before_dst,
        );
    }
    sequenceLength
}

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
    let oLitEnd = op.add(sequence.litLength);
    let sequenceLength = sequence.litLength + sequence.matchLength;
    let oMatchEnd = op.add(sequenceLength); /* risk : address space overflow (32-bits) */
    let iLitEnd = (*litPtr).add(sequence.litLength);
    let mut match_: *const BYTE = oLitEnd.offset(-(sequence.offset as isize)) as *const BYTE;

    /* Handle edge cases in a slow path */
    if iLitEnd > litLimit
        || (oMatchEnd as *const BYTE) > oend_w
        || (mem_32bits() != 0
            && (ptr_diff(oend, op) as usize) < sequenceLength + WILDCOPY_OVERLENGTH)
    {
        return ZSTD_execSequenceEndSplitLitBuffer(
            op, oend, oend_w, sequence, litPtr, litLimit, prefixStart, virtualStart, dictEnd,
        );
    }

    /* Copy Literals */
    zstd_copy16(op as *mut c_void, *litPtr as *const c_void);
    if sequence.litLength > 16 {
        zstd_wildcopy(
            op.add(16) as *mut c_void,
            (*litPtr).add(16) as *const c_void,
            (sequence.litLength - 16) as isize,
            ZSTD_no_overlap,
        );
    }
    op = oLitEnd;
    *litPtr = iLitEnd; /* update for next sequence */

    /* Copy Match */
    if sequence.offset > ptr_diff(oLitEnd, prefixStart) as usize {
        /* offset beyond prefix -> go into extDict */
        if sequence.offset > ptr_diff(oLitEnd, virtualStart) as usize {
            return error(code::CORRUPTION_DETECTED);
        }
        match_ = (dictEnd as *const BYTE).offset(ptr_diff(match_, prefixStart));
        if match_.add(sequence.matchLength) <= dictEnd {
            memmove(
                oLitEnd as *mut c_void,
                match_ as *const c_void,
                sequence.matchLength,
            );
            return sequenceLength;
        }
        /* span extDict & currentPrefixSegment */
        {
            let length1 = ptr_diff(dictEnd, match_) as usize;
            memmove(oLitEnd as *mut c_void, match_ as *const c_void, length1);
            op = oLitEnd.add(length1);
            sequence.matchLength -= length1;
            match_ = prefixStart;
        }
    }
    /* Match within prefix of 1 or more bytes */

    if sequence.offset >= WILDCOPY_VECLEN {
        zstd_wildcopy(
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
        zstd_wildcopy(
            op as *mut c_void,
            match_ as *const c_void,
            (sequence.matchLength - 8) as isize,
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
    (*DStatePtr).state = bit_read_bits(bitD, (*DTableH).tableLog);
    bit_reload_dstream(bitD);
    (*DStatePtr).table = dt.add(1);
}

#[inline]
unsafe fn ZSTD_updateFseStateWithDInfo(
    DStatePtr: *mut ZSTD_fseState,
    bitD: *mut BIT_DStream_t,
    nextState: U16,
    nbBits: U32,
) {
    let lowBits = bit_read_bits(bitD, nbBits);
    (*DStatePtr).state = nextState as usize + lowBits;
}

/**
 * ZSTD_decodeSequence():
 */
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
    let llDInfo = (*seqState).stateLL.table.add((*seqState).stateLL.state);
    let mlDInfo = (*seqState).stateML.table.add((*seqState).stateML.state);
    let ofDInfo = (*seqState).stateOffb.table.add((*seqState).stateOffb.state);
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
                if mem_32bits() != 0
                    && longOffsets != 0
                    && (ofBits as u32 >= STREAM_ACCUMULATOR_MIN_32)
                {
                    let extraBits: U32 = LONG_OFFSETS_MAX_EXTRA_BITS_32;
                    offset = ofBase as usize
                        + ((bit_read_bits_fast(&mut (*seqState).DStream, ofBits as u32 - extraBits)
                            as usize)
                            << extraBits);
                    bit_reload_dstream(&mut (*seqState).DStream);
                    offset += bit_read_bits_fast(&mut (*seqState).DStream, extraBits) as usize;
                } else {
                    offset = ofBase as usize
                        + bit_read_bits_fast(&mut (*seqState).DStream, ofBits as u32) as usize;
                    if mem_32bits() != 0 {
                        bit_reload_dstream(&mut (*seqState).DStream);
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
                    offset = ofBase as usize
                        + ll0 as usize
                        + bit_read_bits_fast(&mut (*seqState).DStream, 1) as usize;
                    {
                        let mut temp: usize = if offset == 3 {
                            (*seqState).prevOffset[0] - 1
                        } else {
                            (*seqState).prevOffset[offset]
                        };
                        temp -= (temp == 0) as usize;
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
            seq.matchLength +=
                bit_read_bits_fast(&mut (*seqState).DStream, mlBits as u32) as usize;
        }

        if mem_32bits() != 0
            && (mlBits as u32 + llBits as u32
                >= STREAM_ACCUMULATOR_MIN_32 - LONG_OFFSETS_MAX_EXTRA_BITS_32)
        {
            bit_reload_dstream(&mut (*seqState).DStream);
        }
        if mem_64bits() != 0
            && (totalBits as u32
                >= STREAM_ACCUMULATOR_MIN_64 - (LLFSELog + MLFSELog + OffFSELog))
        {
            bit_reload_dstream(&mut (*seqState).DStream);
        }

        if llBits > 0 {
            seq.litLength +=
                bit_read_bits_fast(&mut (*seqState).DStream, llBits as u32) as usize;
        }

        if mem_32bits() != 0 {
            bit_reload_dstream(&mut (*seqState).DStream);
        }

        if isLastSeq == 0 {
            /* don't update FSE state for last Sequence */
            ZSTD_updateFseStateWithDInfo(
                &mut (*seqState).stateLL,
                &mut (*seqState).DStream,
                llNext,
                llnbBits,
            );
            ZSTD_updateFseStateWithDInfo(
                &mut (*seqState).stateML,
                &mut (*seqState).DStream,
                mlNext,
                mlnbBits,
            );
            if mem_32bits() != 0 {
                bit_reload_dstream(&mut (*seqState).DStream);
            }
            ZSTD_updateFseStateWithDInfo(
                &mut (*seqState).stateOffb,
                &mut (*seqState).DStream,
                ofNext,
                ofnbBits,
            );
            bit_reload_dstream(&mut (*seqState).DStream);
        }
    }

    seq
}

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
    let iend = ip.add(seqSize);
    let ostart = dst as *mut BYTE;
    let oend = ZSTD_maybeNullPtrAdd(ostart, maxDstSize as isize);
    let mut op = ostart;
    let mut litPtr = (*dctx).litPtr;
    let mut litBufferEnd = (*dctx).litBufferEnd;
    let prefixStart = (*dctx).prefixStart as *const BYTE;
    let vBase = (*dctx).virtualStart as *const BYTE;
    let dictEnd = (*dctx).dictEnd as *const BYTE;

    /* Literals are split between internal buffer & output buffer */
    if nbSeq != 0 {
        let mut seqState = seqState_t {
            DStream: core::mem::zeroed(),
            stateLL: ZSTD_fseState { state: 0, table: core::ptr::null() },
            stateOffb: ZSTD_fseState { state: 0, table: core::ptr::null() },
            stateML: ZSTD_fseState { state: 0, table: core::ptr::null() },
            prevOffset: [0; ZSTD_REP_NUM],
        };
        (*dctx).fseEntropy = 1;
        {
            let mut i: u32 = 0;
            while i < ZSTD_REP_NUM as u32 {
                seqState.prevOffset[i as usize] = (*dctx).entropy.rep[i as usize] as usize;
                i += 1;
            }
        }
        if err_is_error(bit_init_dstream(
            &mut seqState.DStream,
            ip as *const c_void,
            ptr_diff(iend, ip) as usize,
        )) != 0
        {
            return error(code::CORRUPTION_DETECTED);
        }
        ZSTD_initFseState(&mut seqState.stateLL, &mut seqState.DStream, (*dctx).LLTptr);
        ZSTD_initFseState(&mut seqState.stateOffb, &mut seqState.DStream, (*dctx).OFTptr);
        ZSTD_initFseState(&mut seqState.stateML, &mut seqState.DStream, (*dctx).MLTptr);

        /* decompress without overrunning litPtr begins */
        {
            let mut sequence = seq_t {
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
                    let oneSeqSize = ZSTD_execSequenceSplitLitBuffer(
                        op,
                        oend,
                        litPtr.add(sequence.litLength).offset(-(WILDCOPY_OVERLENGTH as isize)),
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
                let leftoverLit = ptr_diff((*dctx).litBufferEnd, litPtr) as usize;
                if leftoverLit != 0 {
                    if leftoverLit > ptr_diff(oend, op) as usize {
                        return error(code::DSTSIZE_TOOSMALL);
                    }
                    ZSTD_safecopyDstBeforeSrc(op, litPtr, leftoverLit as isize);
                    sequence.litLength -= leftoverLit;
                    op = op.add(leftoverLit);
                }
                litPtr = (*dctx).litExtraBuffer.as_ptr();
                litBufferEnd = (*dctx).litExtraBuffer.as_ptr().add(ZSTD_LITBUFFEREXTRASIZE);
                (*dctx).litBufferLocation = ZSTD_not_in_dst;
                {
                    let oneSeqSize = ZSTD_execSequence(
                        op,
                        oend,
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
        }

        if nbSeq > 0 {
            /* there is remaining lit from extra buffer */
            while nbSeq != 0 {
                let sequence =
                    ZSTD_decodeSequence(&mut seqState, isLongOffset, (nbSeq == 1) as c_int);
                let oneSeqSize = ZSTD_execSequence(
                    op,
                    oend,
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
                nbSeq -= 1;
            }
        }

        /* check if reached exact end */
        if nbSeq != 0 {
            return error(code::CORRUPTION_DETECTED);
        }
        if bit_end_of_dstream(&seqState.DStream) == 0 {
            return error(code::CORRUPTION_DETECTED);
        }
        /* save reps for next block */
        {
            let mut i: u32 = 0;
            while i < ZSTD_REP_NUM as u32 {
                (*dctx).entropy.rep[i as usize] = seqState.prevOffset[i as usize] as U32;
                i += 1;
            }
        }
    }

    /* last literal segment */
    if (*dctx).litBufferLocation == ZSTD_split {
        /* split hasn't been reached yet, first get dst then copy litExtraBuffer */
        let lastLLSize = ptr_diff(litBufferEnd, litPtr) as usize;
        if lastLLSize > ptr_diff(oend, op) as usize {
            return error(code::DSTSIZE_TOOSMALL);
        }
        if !op.is_null() {
            memmove(op as *mut c_void, litPtr as *const c_void, lastLLSize);
            op = op.add(lastLLSize);
        }
        litPtr = (*dctx).litExtraBuffer.as_ptr();
        litBufferEnd = (*dctx).litExtraBuffer.as_ptr().add(ZSTD_LITBUFFEREXTRASIZE);
        (*dctx).litBufferLocation = ZSTD_not_in_dst;
    }
    /* copy last literals from internal buffer */
    {
        let lastLLSize = ptr_diff(litBufferEnd, litPtr) as usize;
        if lastLLSize > ptr_diff(oend, op) as usize {
            return error(code::DSTSIZE_TOOSMALL);
        }
        if !op.is_null() {
            memcpy(op as *mut c_void, litPtr as *const c_void, lastLLSize);
            op = op.add(lastLLSize);
        }
    }

    ptr_diff(op, ostart) as usize
}

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
    let iend = ip.add(seqSize);
    let ostart = dst as *mut BYTE;
    let oend = if (*dctx).litBufferLocation == ZSTD_not_in_dst {
        ZSTD_maybeNullPtrAdd(ostart, maxDstSize as isize)
    } else {
        (*dctx).litBuffer
    };
    let mut op = ostart;
    let mut litPtr = (*dctx).litPtr;
    let litEnd = litPtr.add((*dctx).litSize);
    let prefixStart = (*dctx).prefixStart as *const BYTE;
    let vBase = (*dctx).virtualStart as *const BYTE;
    let dictEnd = (*dctx).dictEnd as *const BYTE;

    /* Regen sequences */
    if nbSeq != 0 {
        let mut seqState = seqState_t {
            DStream: core::mem::zeroed(),
            stateLL: ZSTD_fseState { state: 0, table: core::ptr::null() },
            stateOffb: ZSTD_fseState { state: 0, table: core::ptr::null() },
            stateML: ZSTD_fseState { state: 0, table: core::ptr::null() },
            prevOffset: [0; ZSTD_REP_NUM],
        };
        (*dctx).fseEntropy = 1;
        {
            let mut i: u32 = 0;
            while i < ZSTD_REP_NUM as u32 {
                seqState.prevOffset[i as usize] = (*dctx).entropy.rep[i as usize] as usize;
                i += 1;
            }
        }
        if err_is_error(bit_init_dstream(
            &mut seqState.DStream,
            ip as *const c_void,
            ptr_diff(iend, ip) as usize,
        )) != 0
        {
            return error(code::CORRUPTION_DETECTED);
        }
        ZSTD_initFseState(&mut seqState.stateLL, &mut seqState.DStream, (*dctx).LLTptr);
        ZSTD_initFseState(&mut seqState.stateOffb, &mut seqState.DStream, (*dctx).OFTptr);
        ZSTD_initFseState(&mut seqState.stateML, &mut seqState.DStream, (*dctx).MLTptr);

        while nbSeq != 0 {
            let sequence = ZSTD_decodeSequence(&mut seqState, isLongOffset, (nbSeq == 1) as c_int);
            let oneSeqSize = ZSTD_execSequence(
                op,
                oend,
                sequence,
                &mut litPtr,
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
        if bit_end_of_dstream(&seqState.DStream) == 0 {
            return error(code::CORRUPTION_DETECTED);
        }
        /* save reps for next block */
        {
            let mut i: u32 = 0;
            while i < ZSTD_REP_NUM as u32 {
                (*dctx).entropy.rep[i as usize] = seqState.prevOffset[i as usize] as U32;
                i += 1;
            }
        }
    }

    /* last literal segment */
    {
        let lastLLSize = ptr_diff(litEnd, litPtr) as usize;
        if lastLLSize > ptr_diff(oend, op) as usize {
            return error(code::DSTSIZE_TOOSMALL);
        }
        if !op.is_null() {
            memcpy(op as *mut c_void, litPtr as *const c_void, lastLLSize);
            op = op.add(lastLLSize);
        }
    }

    ptr_diff(op, ostart) as usize
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
    ZSTD_decompressSequences_body(dctx, dst, maxDstSize, seqStart, seqSize, nbSeq, isLongOffset)
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
        dctx, dst, maxDstSize, seqStart, seqSize, nbSeq, isLongOffset,
    )
}

#[inline]
unsafe fn ZSTD_prefetchMatch(
    mut prefetchPos: usize,
    sequence: seq_t,
    prefixStart: *const BYTE,
    dictEnd: *const BYTE,
) -> usize {
    prefetchPos += sequence.litLength;
    {
        let matchBase = if sequence.offset > prefetchPos {
            dictEnd
        } else {
            prefixStart
        };
        let match_ = ZSTD_wrappedPtrSub(
            ZSTD_wrappedPtrAdd(matchBase, prefetchPos as isize),
            sequence.offset as isize,
        );
        PREFETCH_L1(match_);
        PREFETCH_L1(match_.wrapping_add(CACHELINE_SIZE));
    }
    prefetchPos + sequence.matchLength
}

const STORED_SEQS: usize = 8;
const STORED_SEQS_MASK: usize = STORED_SEQS - 1;
const ADVANCED_SEQS: c_int = STORED_SEQS as c_int;

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
    let iend = ip.add(seqSize);
    let ostart = dst as *mut BYTE;
    let oend = if (*dctx).litBufferLocation == ZSTD_in_dst {
        (*dctx).litBuffer
    } else {
        ZSTD_maybeNullPtrAdd(ostart, maxDstSize as isize)
    };
    let mut op = ostart;
    let mut litPtr = (*dctx).litPtr;
    let mut litBufferEnd = (*dctx).litBufferEnd;
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
        let seqAdvance: c_int = if nbSeq < ADVANCED_SEQS {
            nbSeq
        } else {
            ADVANCED_SEQS
        };
        let mut seqState = seqState_t {
            DStream: core::mem::zeroed(),
            stateLL: ZSTD_fseState { state: 0, table: core::ptr::null() },
            stateOffb: ZSTD_fseState { state: 0, table: core::ptr::null() },
            stateML: ZSTD_fseState { state: 0, table: core::ptr::null() },
            prevOffset: [0; ZSTD_REP_NUM],
        };
        let mut seqNb: c_int;
        let mut prefetchPos: usize = ptr_diff(op, prefixStart) as usize; /* track position relative to prefixStart */

        (*dctx).fseEntropy = 1;
        {
            let mut i: c_int = 0;
            while i < ZSTD_REP_NUM as c_int {
                seqState.prevOffset[i as usize] = (*dctx).entropy.rep[i as usize] as usize;
                i += 1;
            }
        }
        if err_is_error(bit_init_dstream(
            &mut seqState.DStream,
            ip as *const c_void,
            ptr_diff(iend, ip) as usize,
        )) != 0
        {
            return error(code::CORRUPTION_DETECTED);
        }
        ZSTD_initFseState(&mut seqState.stateLL, &mut seqState.DStream, (*dctx).LLTptr);
        ZSTD_initFseState(&mut seqState.stateOffb, &mut seqState.DStream, (*dctx).OFTptr);
        ZSTD_initFseState(&mut seqState.stateML, &mut seqState.DStream, (*dctx).MLTptr);

        /* prepare in advance */
        seqNb = 0;
        while seqNb < seqAdvance {
            let sequence =
                ZSTD_decodeSequence(&mut seqState, isLongOffset, (seqNb == nbSeq - 1) as c_int);
            prefetchPos = ZSTD_prefetchMatch(prefetchPos, sequence, prefixStart, dictEnd);
            sequences[seqNb as usize] = sequence;
            seqNb += 1;
        }

        /* decompress without stomping litBuffer */
        while seqNb < nbSeq {
            let sequence =
                ZSTD_decodeSequence(&mut seqState, isLongOffset, (seqNb == nbSeq - 1) as c_int);

            if (*dctx).litBufferLocation == ZSTD_split
                && litPtr.add(
                    sequences[((seqNb - ADVANCED_SEQS) as usize) & STORED_SEQS_MASK].litLength,
                ) > (*dctx).litBufferEnd
            {
                /* lit buffer is reaching split point, empty out the first buffer and transition to litExtraBuffer */
                let leftoverLit = ptr_diff((*dctx).litBufferEnd, litPtr) as usize;
                if leftoverLit != 0 {
                    if leftoverLit > ptr_diff(oend, op) as usize {
                        return error(code::DSTSIZE_TOOSMALL);
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
                    let oneSeqSize = ZSTD_execSequence(
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
                let oneSeqSize = if (*dctx).litBufferLocation == ZSTD_split {
                    ZSTD_execSequenceSplitLitBuffer(
                        op,
                        oend,
                        litPtr
                            .add(
                                sequences[((seqNb - ADVANCED_SEQS) as usize) & STORED_SEQS_MASK]
                                    .litLength,
                            )
                            .offset(-(WILDCOPY_OVERLENGTH as isize)),
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
        if bit_end_of_dstream(&seqState.DStream) == 0 {
            return error(code::CORRUPTION_DETECTED);
        }

        /* finish queue */
        seqNb -= seqAdvance;
        while seqNb < nbSeq {
            let sequence_ptr = &mut sequences[(seqNb as usize) & STORED_SEQS_MASK] as *mut seq_t;
            if (*dctx).litBufferLocation == ZSTD_split
                && litPtr.add((*sequence_ptr).litLength) > (*dctx).litBufferEnd
            {
                let leftoverLit = ptr_diff((*dctx).litBufferEnd, litPtr) as usize;
                if leftoverLit != 0 {
                    if leftoverLit > ptr_diff(oend, op) as usize {
                        return error(code::DSTSIZE_TOOSMALL);
                    }
                    ZSTD_safecopyDstBeforeSrc(op, litPtr, leftoverLit as isize);
                    (*sequence_ptr).litLength -= leftoverLit;
                    op = op.add(leftoverLit);
                }
                litPtr = (*dctx).litExtraBuffer.as_ptr();
                litBufferEnd = (*dctx).litExtraBuffer.as_ptr().add(ZSTD_LITBUFFEREXTRASIZE);
                (*dctx).litBufferLocation = ZSTD_not_in_dst;
                {
                    let oneSeqSize = ZSTD_execSequence(
                        op,
                        oend,
                        *sequence_ptr,
                        &mut litPtr,
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
                let oneSeqSize = if (*dctx).litBufferLocation == ZSTD_split {
                    ZSTD_execSequenceSplitLitBuffer(
                        op,
                        oend,
                        litPtr
                            .add((*sequence_ptr).litLength)
                            .offset(-(WILDCOPY_OVERLENGTH as isize)),
                        *sequence_ptr,
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
                        *sequence_ptr,
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
                op = op.add(oneSeqSize);
            }
            seqNb += 1;
        }

        /* save reps for next block */
        {
            let mut i: c_int = 0;
            while i < ZSTD_REP_NUM as c_int {
                (*dctx).entropy.rep[i as usize] = seqState.prevOffset[i as usize] as U32;
                i += 1;
            }
        }
    }

    /* last literal segment */
    if (*dctx).litBufferLocation == ZSTD_split {
        /* first deplete literal buffer in dst, then copy litExtraBuffer */
        let lastLLSize = ptr_diff(litBufferEnd, litPtr) as usize;
        if lastLLSize > ptr_diff(oend, op) as usize {
            return error(code::DSTSIZE_TOOSMALL);
        }
        if !op.is_null() {
            memmove(op as *mut c_void, litPtr as *const c_void, lastLLSize);
            op = op.add(lastLLSize);
        }
        litPtr = (*dctx).litExtraBuffer.as_ptr();
        litBufferEnd = (*dctx).litExtraBuffer.as_ptr().add(ZSTD_LITBUFFEREXTRASIZE);
    }
    {
        let lastLLSize = ptr_diff(litBufferEnd, litPtr) as usize;
        if lastLLSize > ptr_diff(oend, op) as usize {
            return error(code::DSTSIZE_TOOSMALL);
        }
        if !op.is_null() {
            memmove(op as *mut c_void, litPtr as *const c_void, lastLLSize);
            op = op.add(lastLLSize);
        }
    }

    ptr_diff(op, ostart) as usize
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
        dctx, dst, maxDstSize, seqStart, seqSize, nbSeq, isLongOffset,
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
    ZSTD_decompressSequences_default(dctx, dst, maxDstSize, seqStart, seqSize, nbSeq, isLongOffset)
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
        dctx, dst, maxDstSize, seqStart, seqSize, nbSeq, isLongOffset,
    )
}

/* ZSTD_decompressSequencesLong() : */
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
        dctx, dst, maxDstSize, seqStart, seqSize, nbSeq, isLongOffset,
    )
}

unsafe fn ZSTD_totalHistorySize(op: *mut BYTE, virtualStart: *const BYTE) -> usize {
    ptr_diff(op, virtualStart) as usize
}

#[derive(Clone, Copy)]
struct ZSTD_OffsetInfo {
    longOffsetShare: c_uint,
    maxNbAdditionalBits: c_uint,
}

/* ZSTD_getOffsetInfo() : */
unsafe fn ZSTD_getOffsetInfo(offTable: *const ZSTD_seqSymbol, nbSeq: c_int) -> ZSTD_OffsetInfo {
    let mut info = ZSTD_OffsetInfo {
        longOffsetShare: 0,
        maxNbAdditionalBits: 0,
    };
    if nbSeq != 0 {
        let ptr = offTable as *const c_void;
        let tableLog: U32 = (*(ptr as *const ZSTD_seqSymbol_header)).tableLog;
        let table = offTable.add(1);
        let max: U32 = 1 << tableLog;
        let mut u: U32 = 0;

        while u < max {
            let nab = (*table.add(u as usize)).nbAdditionalBits as c_uint;
            if nab > info.maxNbAdditionalBits {
                info.maxNbAdditionalBits = nab;
            }
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
unsafe fn ZSTD_maxShortOffset() -> usize {
    if mem_64bits() != 0 {
        (-1_isize) as usize
    } else {
        let maxOffbase: usize = ((1usize) << (STREAM_ACCUMULATOR_MIN_32 + 1)) - 1;
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

    if srcSize > ZSTD_blockSizeMax(dctx) {
        return error(code::SRCSIZE_WRONG);
    }

    /* Decode literals section */
    {
        let litCSize = ZSTD_decodeLiteralsBlock(dctx, src, srcSize, dst, dstCapacity, streaming);
        if ZSTD_isError(litCSize) != 0 {
            return litCSize;
        }
        ip = ip.add(litCSize);
        srcSize -= litCSize;
    }

    /* Build Decoding Tables */
    {
        let blockSizeMax = MIN_usize(dstCapacity, ZSTD_blockSizeMax(dctx));
        let totalHistorySize = ZSTD_totalHistorySize(
            ZSTD_maybeNullPtrAdd(dst as *mut BYTE, blockSizeMax as isize),
            (*dctx).virtualStart as *const BYTE,
        );
        let mut isLongOffset: ZSTD_longOffset_e =
            (mem_32bits() != 0 && (totalHistorySize > ZSTD_maxShortOffset())) as ZSTD_longOffset_e;
        let mut usePrefetchDecoder: c_int = (*dctx).ddictIsCold;
        let mut nbSeq: c_int = 0;
        let seqHSize = ZSTD_decodeSeqHeaders(dctx, &mut nbSeq, ip as *const c_void, srcSize);
        if ZSTD_isError(seqHSize) != 0 {
            return seqHSize;
        }
        ip = ip.add(seqHSize);
        srcSize -= seqHSize;

        if (dst.is_null() || dstCapacity == 0) && nbSeq > 0 {
            return error(code::DSTSIZE_TOOSMALL);
        }
        if mem_64bits() != 0
            && core::mem::size_of::<usize>() == core::mem::size_of::<*const c_void>()
            && ((-1_isize) as usize).wrapping_sub(dst as usize) < (1usize << 20)
        {
            return error(code::DSTSIZE_TOOSMALL);
        }

        if isLongOffset != 0
            || (usePrefetchDecoder == 0 && (totalHistorySize > (1u32 << 24) as usize) && (nbSeq > 8))
        {
            let info = ZSTD_getOffsetInfo((*dctx).OFTptr, nbSeq);
            if isLongOffset != 0 && info.maxNbAdditionalBits <= STREAM_ACCUMULATOR_MIN {
                isLongOffset = ZSTD_lo_isRegularOffset;
            }
            if usePrefetchDecoder == 0 {
                let minShare: U32 = if mem_64bits() != 0 { 7 } else { 20 };
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
        (*dctx).virtualStart = (dst as *const u8).offset(
            -(((*dctx).previousDstEnd as *const u8 as isize)
                - ((*dctx).prefixStart as *const u8 as isize)),
        ) as *const c_void;
        (*dctx).prefixStart = dst;
        (*dctx).previousDstEnd = dst;
    }
}

const STREAM_ACCUMULATOR_MIN: c_uint = if core::mem::size_of::<usize>() == 4 {
    STREAM_ACCUMULATOR_MIN_32
} else {
    STREAM_ACCUMULATOR_MIN_64
};

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
    ZSTD_checkContinuity(dctx, dst, dstCapacity);
    dSize = ZSTD_decompressBlock_internal(dctx, dst, dstCapacity, src, srcSize, not_streaming);
    if ZSTD_isError(dSize) != 0 {
        return dSize;
    }
    (*dctx).previousDstEnd = (dst as *mut u8).add(dSize) as *const c_void;
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

