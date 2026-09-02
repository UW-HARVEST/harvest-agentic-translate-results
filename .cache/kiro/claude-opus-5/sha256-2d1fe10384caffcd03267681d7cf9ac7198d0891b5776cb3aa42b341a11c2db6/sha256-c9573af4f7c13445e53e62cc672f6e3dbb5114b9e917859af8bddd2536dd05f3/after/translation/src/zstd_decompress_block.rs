//! Translation of `decompress/zstd_decompress_block.c`.
#![allow(dead_code)]

use core::ffi::{c_int, c_uint, c_void};

use crate::bitstream::*;
use crate::error::*;
use crate::huf::*;
use crate::huf_decompress::*;
use crate::entropy_common::FSE_readNCount;
use crate::mem::*;
use crate::zstd_decompress_internal::*;
use crate::zstd_internal::*;
use crate::zstd_public::*;

/// `streaming_operation`
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum streaming_operation {
    not_streaming = 0,
    is_streaming = 1,
}

/// `ZSTD_copy4()`
#[inline(always)]
unsafe fn zstd_copy4(dst: *mut c_void, src: *const c_void) {
    core::ptr::copy_nonoverlapping(src as *const u8, dst as *mut u8, 4);
}

/// `ZSTD_maybeNullPtrAdd(ptr, add)` — `add > 0 ? ptr + add : ptr`.
#[inline(always)]
unsafe fn zstd_maybe_null_ptr_add(ptr: *mut u8, add: isize) -> *mut u8 {
    if add > 0 {
        ptr.offset(add)
    } else {
        ptr
    }
}

/// `ZSTD_wrappedPtrAdd(ptr, add)` — `ptr + add` with wrapping.
#[inline(always)]
unsafe fn zstd_wrapped_ptr_add(ptr: *const u8, add: isize) -> *const u8 {
    ptr.wrapping_offset(add)
}

/// `ZSTD_wrappedPtrSub(ptr, sub)` — `ptr - sub` with wrapping.
#[inline(always)]
unsafe fn zstd_wrapped_ptr_sub(ptr: *const u8, sub: isize) -> *const u8 {
    ptr.wrapping_offset(-sub)
}

const CACHELINE_SIZE: usize = 64;

/// `PREFETCH_L1(ptr)` — disabled (no-op) in this build configuration.
#[inline(always)]
fn prefetch_l1(_ptr: *const u8) {}

/// `ZSTD_blockSizeMax()`
unsafe fn zstd_block_size_max(dctx: *const ZSTD_DCtx) -> usize {
    let block_size_max = if (*dctx).isFrameDecompression != 0 {
        (*dctx).fParams.blockSizeMax as usize
    } else {
        ZSTD_BLOCKSIZE_MAX
    };
    block_size_max
}

/// `ZSTD_getcBlockSize()`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_getcBlockSize(
    src: *const c_void,
    src_size: usize,
    bp_ptr: *mut blockProperties_t,
) -> usize {
    if src_size < ZSTD_blockHeaderSize {
        return err_code(ZSTD_error_srcSize_wrong);
    }

    {
        let c_block_header: U32 = mem_read_le24(src as *const u8);
        let c_size = c_block_header >> 3;
        (*bp_ptr).lastBlock = c_block_header & 1;
        (*bp_ptr).blockType = (c_block_header >> 1) & 3;
        (*bp_ptr).origSize = c_size;
        if (*bp_ptr).blockType == bt_rle {
            return 1;
        }
        if (*bp_ptr).blockType == bt_reserved {
            return err_code(ZSTD_error_corruption_detected);
        }
        c_size as usize
    }
}

/// `ZSTD_allocateLiteralsBuffer()`
unsafe fn zstd_allocate_literals_buffer(
    dctx: *mut ZSTD_DCtx,
    dst: *mut c_void,
    dst_capacity: usize,
    lit_size: usize,
    streaming: streaming_operation,
    expected_write_size: usize,
    split_immediately: c_uint,
) {
    let block_size_max = zstd_block_size_max(dctx);
    if streaming == streaming_operation::not_streaming
        && dst_capacity
            > block_size_max + WILDCOPY_OVERLENGTH + lit_size + WILDCOPY_OVERLENGTH
    {
        (*dctx).litBuffer = (dst as *mut u8).add(block_size_max + WILDCOPY_OVERLENGTH);
        (*dctx).litBufferEnd = (*dctx).litBuffer.add(lit_size);
        (*dctx).litBufferLocation = ZSTD_in_dst;
    } else if lit_size <= ZSTD_LITBUFFEREXTRASIZE {
        (*dctx).litBuffer = (*dctx).litExtraBuffer.as_mut_ptr();
        (*dctx).litBufferEnd = (*dctx).litBuffer.add(lit_size);
        (*dctx).litBufferLocation = ZSTD_not_in_dst;
    } else {
        if split_immediately != 0 {
            (*dctx).litBuffer = (dst as *mut u8).add(
                expected_write_size - lit_size + ZSTD_LITBUFFEREXTRASIZE - WILDCOPY_OVERLENGTH,
            );
            (*dctx).litBufferEnd = (*dctx).litBuffer.add(lit_size - ZSTD_LITBUFFEREXTRASIZE);
        } else {
            (*dctx).litBuffer = (dst as *mut u8).add(expected_write_size - lit_size);
            (*dctx).litBufferEnd = (dst as *mut u8).add(expected_write_size);
        }
        (*dctx).litBufferLocation = ZSTD_split;
    }
}

/// `ZSTD_decodeLiteralsBlock()`
unsafe fn zstd_decode_literals_block(
    dctx: *mut ZSTD_DCtx,
    src: *const c_void,
    src_size: usize,
    dst: *mut c_void,
    dst_capacity: usize,
    streaming: streaming_operation,
) -> usize {
    if src_size < MIN_CBLOCK_SIZE {
        return err_code(ZSTD_error_corruption_detected);
    }

    {
        let istart = src as *const u8;
        let lit_enc_type: u32 = (*istart.add(0) & 3) as u32;
        let block_size_max = zstd_block_size_max(dctx);

        match lit_enc_type {
            x if x == set_repeat || x == set_compressed => {
                if lit_enc_type == set_repeat && (*dctx).litEntropy == 0 {
                    return err_code(ZSTD_error_dictionary_corrupted);
                }
                if src_size < 5 {
                    return err_code(ZSTD_error_corruption_detected);
                }
                let lh_size: usize;
                let lit_size: usize;
                let lit_c_size: usize;
                let mut single_stream: U32 = 0;
                let lhl_code: U32 = ((*istart.add(0) >> 2) & 3) as u32;
                let lhc: U32 = mem_read_le32(istart);
                let huf_success: usize;
                let expected_write_size = min_usize(block_size_max, dst_capacity);
                let flags: c_int = 0
                    | (if zstd_dctx_get_bmi2(dctx) != 0 {
                        HUF_flags_bmi2
                    } else {
                        0
                    })
                    | (if (*dctx).disableHufAsm != 0 {
                        HUF_flags_disableAsm
                    } else {
                        0
                    });
                match lhl_code {
                    2 => {
                        lh_size = 4;
                        lit_size = ((lhc >> 4) & 0x3FFF) as usize;
                        lit_c_size = (lhc >> 18) as usize;
                    }
                    3 => {
                        lh_size = 5;
                        lit_size = ((lhc >> 4) & 0x3FFFF) as usize;
                        lit_c_size = ((lhc >> 22) as usize) + ((*istart.add(4) as usize) << 10);
                    }
                    _ => {
                        /* 0, 1, default */
                        single_stream = (lhl_code == 0) as U32;
                        lh_size = 3;
                        lit_size = ((lhc >> 4) & 0x3FF) as usize;
                        lit_c_size = ((lhc >> 14) & 0x3FF) as usize;
                    }
                }
                if lit_size > 0 && dst.is_null() {
                    return err_code(ZSTD_error_dstSize_tooSmall);
                }
                if lit_size > block_size_max {
                    return err_code(ZSTD_error_corruption_detected);
                }
                if single_stream == 0 && lit_size < MIN_LITERALS_FOR_4_STREAMS {
                    return err_code(ZSTD_error_literals_headerWrong);
                }
                if lit_c_size + lh_size > src_size {
                    return err_code(ZSTD_error_corruption_detected);
                }
                if expected_write_size < lit_size {
                    return err_code(ZSTD_error_dstSize_tooSmall);
                }
                zstd_allocate_literals_buffer(
                    dctx,
                    dst,
                    dst_capacity,
                    lit_size,
                    streaming,
                    expected_write_size,
                    0,
                );

                /* prefetch huffman table if cold */
                if (*dctx).ddictIsCold != 0 && lit_size > 768 {
                    /* PREFETCH_AREA no-op */
                }

                if lit_enc_type == set_repeat {
                    if single_stream != 0 {
                        huf_success = HUF_decompress1X_usingDTable(
                            (*dctx).litBuffer as *mut c_void,
                            lit_size,
                            istart.add(lh_size) as *const c_void,
                            lit_c_size,
                            (*dctx).HUFptr,
                            flags,
                        );
                    } else {
                        huf_success = HUF_decompress4X_usingDTable(
                            (*dctx).litBuffer as *mut c_void,
                            lit_size,
                            istart.add(lh_size) as *const c_void,
                            lit_c_size,
                            (*dctx).HUFptr,
                            flags,
                        );
                    }
                } else {
                    if single_stream != 0 {
                        huf_success = HUF_decompress1X1_DCtx_wksp(
                            (*dctx).entropy.hufTable.as_mut_ptr(),
                            (*dctx).litBuffer as *mut c_void,
                            lit_size,
                            istart.add(lh_size) as *const c_void,
                            lit_c_size,
                            (*dctx).workspace.as_mut_ptr() as *mut c_void,
                            core::mem::size_of_val(&(*dctx).workspace),
                            flags,
                        );
                    } else {
                        huf_success = HUF_decompress4X_hufOnly_wksp(
                            (*dctx).entropy.hufTable.as_mut_ptr(),
                            (*dctx).litBuffer as *mut c_void,
                            lit_size,
                            istart.add(lh_size) as *const c_void,
                            lit_c_size,
                            (*dctx).workspace.as_mut_ptr() as *mut c_void,
                            core::mem::size_of_val(&(*dctx).workspace),
                            flags,
                        );
                    }
                }
                if (*dctx).litBufferLocation == ZSTD_split {
                    core::ptr::copy_nonoverlapping(
                        (*dctx).litBufferEnd.sub(ZSTD_LITBUFFEREXTRASIZE),
                        (*dctx).litExtraBuffer.as_mut_ptr(),
                        ZSTD_LITBUFFEREXTRASIZE,
                    );
                    core::ptr::copy(
                        (*dctx).litBuffer,
                        (*dctx)
                            .litBuffer
                            .add(ZSTD_LITBUFFEREXTRASIZE - WILDCOPY_OVERLENGTH),
                        lit_size - ZSTD_LITBUFFEREXTRASIZE,
                    );
                    (*dctx).litBuffer = (*dctx)
                        .litBuffer
                        .add(ZSTD_LITBUFFEREXTRASIZE - WILDCOPY_OVERLENGTH);
                    (*dctx).litBufferEnd = (*dctx).litBufferEnd.sub(WILDCOPY_OVERLENGTH);
                }

                if err_is_error(huf_success) {
                    return err_code(ZSTD_error_corruption_detected);
                }

                (*dctx).litPtr = (*dctx).litBuffer;
                (*dctx).litSize = lit_size;
                (*dctx).litEntropy = 1;
                if lit_enc_type == set_compressed {
                    (*dctx).HUFptr = (*dctx).entropy.hufTable.as_ptr();
                }
                lit_c_size + lh_size
            }

            x if x == set_basic => {
                let lit_size: usize;
                let lh_size: usize;
                let lhl_code: U32 = ((*istart.add(0) >> 2) & 3) as u32;
                let expected_write_size = min_usize(block_size_max, dst_capacity);
                match lhl_code {
                    1 => {
                        lh_size = 2;
                        lit_size = (mem_read_le16(istart) >> 4) as usize;
                    }
                    3 => {
                        lh_size = 3;
                        if src_size < 3 {
                            return err_code(ZSTD_error_corruption_detected);
                        }
                        lit_size = (mem_read_le24(istart) >> 4) as usize;
                    }
                    _ => {
                        /* 0, 2, default */
                        lh_size = 1;
                        lit_size = (*istart.add(0) >> 3) as usize;
                    }
                }

                if lit_size > 0 && dst.is_null() {
                    return err_code(ZSTD_error_dstSize_tooSmall);
                }
                if lit_size > block_size_max {
                    return err_code(ZSTD_error_corruption_detected);
                }
                if expected_write_size < lit_size {
                    return err_code(ZSTD_error_dstSize_tooSmall);
                }
                zstd_allocate_literals_buffer(
                    dctx,
                    dst,
                    dst_capacity,
                    lit_size,
                    streaming,
                    expected_write_size,
                    1,
                );
                if lh_size + lit_size + WILDCOPY_OVERLENGTH > src_size {
                    if lit_size + lh_size > src_size {
                        return err_code(ZSTD_error_corruption_detected);
                    }
                    if (*dctx).litBufferLocation == ZSTD_split {
                        core::ptr::copy_nonoverlapping(
                            istart.add(lh_size),
                            (*dctx).litBuffer,
                            lit_size - ZSTD_LITBUFFEREXTRASIZE,
                        );
                        core::ptr::copy_nonoverlapping(
                            istart.add(lh_size + lit_size - ZSTD_LITBUFFEREXTRASIZE),
                            (*dctx).litExtraBuffer.as_mut_ptr(),
                            ZSTD_LITBUFFEREXTRASIZE,
                        );
                    } else {
                        core::ptr::copy_nonoverlapping(
                            istart.add(lh_size),
                            (*dctx).litBuffer,
                            lit_size,
                        );
                    }
                    (*dctx).litPtr = (*dctx).litBuffer;
                    (*dctx).litSize = lit_size;
                    return lh_size + lit_size;
                }
                /* direct reference into compressed stream */
                (*dctx).litPtr = istart.add(lh_size);
                (*dctx).litSize = lit_size;
                (*dctx).litBufferEnd = (*dctx).litPtr.add(lit_size);
                (*dctx).litBufferLocation = ZSTD_not_in_dst;
                lh_size + lit_size
            }

            x if x == set_rle => {
                let lhl_code: U32 = ((*istart.add(0) >> 2) & 3) as u32;
                let lit_size: usize;
                let lh_size: usize;
                let expected_write_size = min_usize(block_size_max, dst_capacity);
                match lhl_code {
                    1 => {
                        lh_size = 2;
                        if src_size < 3 {
                            return err_code(ZSTD_error_corruption_detected);
                        }
                        lit_size = (mem_read_le16(istart) >> 4) as usize;
                    }
                    3 => {
                        lh_size = 3;
                        if src_size < 4 {
                            return err_code(ZSTD_error_corruption_detected);
                        }
                        lit_size = (mem_read_le24(istart) >> 4) as usize;
                    }
                    _ => {
                        /* 0, 2, default */
                        lh_size = 1;
                        lit_size = (*istart.add(0) >> 3) as usize;
                    }
                }
                if lit_size > 0 && dst.is_null() {
                    return err_code(ZSTD_error_dstSize_tooSmall);
                }
                if lit_size > block_size_max {
                    return err_code(ZSTD_error_corruption_detected);
                }
                if expected_write_size < lit_size {
                    return err_code(ZSTD_error_dstSize_tooSmall);
                }
                zstd_allocate_literals_buffer(
                    dctx,
                    dst,
                    dst_capacity,
                    lit_size,
                    streaming,
                    expected_write_size,
                    1,
                );
                if (*dctx).litBufferLocation == ZSTD_split {
                    core::ptr::write_bytes(
                        (*dctx).litBuffer,
                        *istart.add(lh_size),
                        lit_size - ZSTD_LITBUFFEREXTRASIZE,
                    );
                    core::ptr::write_bytes(
                        (*dctx).litExtraBuffer.as_mut_ptr(),
                        *istart.add(lh_size),
                        ZSTD_LITBUFFEREXTRASIZE,
                    );
                } else {
                    core::ptr::write_bytes((*dctx).litBuffer, *istart.add(lh_size), lit_size);
                }
                (*dctx).litPtr = (*dctx).litBuffer;
                (*dctx).litSize = lit_size;
                lh_size + 1
            }

            _ => err_code(ZSTD_error_corruption_detected),
        }
    }
}

/// `ZSTD_decodeLiteralsBlock_wrapper()` — hidden declaration for fullbench.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_decodeLiteralsBlock_wrapper(
    dctx: *mut ZSTD_DCtx,
    src: *const c_void,
    src_size: usize,
    dst: *mut c_void,
    dst_capacity: usize,
) -> usize {
    (*dctx).isFrameDecompression = 0;
    zstd_decode_literals_block(
        dctx,
        src,
        src_size,
        dst,
        dst_capacity,
        streaming_operation::not_streaming,
    )
}

/// `LL_defaultDTable` — precomputed FSE table for Literal Lengths.
static LL_DEFAULT_DTABLE: [ZSTD_seqSymbol; (1 << LL_DEFAULTNORMLOG) + 1] = [
    ZSTD_seqSymbol { nextState: 1, nbAdditionalBits: 1, nbBits: 1, baseValue: LL_DEFAULTNORMLOG },
    s(0, 0, 4, 0), s(16, 0, 4, 0),
    s(32, 0, 5, 1), s(0, 0, 5, 3),
    s(0, 0, 5, 4), s(0, 0, 5, 6),
    s(0, 0, 5, 7), s(0, 0, 5, 9),
    s(0, 0, 5, 10), s(0, 0, 5, 12),
    s(0, 0, 6, 14), s(0, 1, 5, 16),
    s(0, 1, 5, 20), s(0, 1, 5, 22),
    s(0, 2, 5, 28), s(0, 3, 5, 32),
    s(0, 4, 5, 48), s(32, 6, 5, 64),
    s(0, 7, 5, 128), s(0, 8, 6, 256),
    s(0, 10, 6, 1024), s(0, 12, 6, 4096),
    s(32, 0, 4, 0), s(0, 0, 4, 1),
    s(0, 0, 5, 2), s(32, 0, 5, 4),
    s(0, 0, 5, 5), s(32, 0, 5, 7),
    s(0, 0, 5, 8), s(32, 0, 5, 10),
    s(0, 0, 5, 11), s(0, 0, 6, 13),
    s(32, 1, 5, 16), s(0, 1, 5, 18),
    s(32, 1, 5, 22), s(0, 2, 5, 24),
    s(32, 3, 5, 32), s(0, 3, 5, 40),
    s(0, 6, 4, 64), s(16, 6, 4, 64),
    s(32, 7, 5, 128), s(0, 9, 6, 512),
    s(0, 11, 6, 2048), s(48, 0, 4, 0),
    s(16, 0, 4, 1), s(32, 0, 5, 2),
    s(32, 0, 5, 3), s(32, 0, 5, 5),
    s(32, 0, 5, 6), s(32, 0, 5, 8),
    s(32, 0, 5, 9), s(32, 0, 5, 11),
    s(32, 0, 5, 12), s(0, 0, 6, 15),
    s(32, 1, 5, 18), s(32, 1, 5, 20),
    s(32, 2, 5, 24), s(32, 2, 5, 28),
    s(32, 3, 5, 40), s(32, 4, 5, 48),
    s(0, 16, 6, 65536), s(0, 15, 6, 32768),
    s(0, 14, 6, 16384), s(0, 13, 6, 8192),
];

/// `OF_defaultDTable` — precomputed FSE table for Offset Codes.
static OF_DEFAULT_DTABLE: [ZSTD_seqSymbol; (1 << OF_DEFAULTNORMLOG) + 1] = [
    ZSTD_seqSymbol { nextState: 1, nbAdditionalBits: 1, nbBits: 1, baseValue: OF_DEFAULTNORMLOG },
    s(0, 0, 5, 0), s(0, 6, 4, 61),
    s(0, 9, 5, 509), s(0, 15, 5, 32765),
    s(0, 21, 5, 2097149), s(0, 3, 5, 5),
    s(0, 7, 4, 125), s(0, 12, 5, 4093),
    s(0, 18, 5, 262141), s(0, 23, 5, 8388605),
    s(0, 5, 5, 29), s(0, 8, 4, 253),
    s(0, 14, 5, 16381), s(0, 20, 5, 1048573),
    s(0, 2, 5, 1), s(16, 7, 4, 125),
    s(0, 11, 5, 2045), s(0, 17, 5, 131069),
    s(0, 22, 5, 4194301), s(0, 4, 5, 13),
    s(16, 8, 4, 253), s(0, 13, 5, 8189),
    s(0, 19, 5, 524285), s(0, 1, 5, 1),
    s(16, 6, 4, 61), s(0, 10, 5, 1021),
    s(0, 16, 5, 65533), s(0, 28, 5, 268435453),
    s(0, 27, 5, 134217725), s(0, 26, 5, 67108861),
    s(0, 25, 5, 33554429), s(0, 24, 5, 16777213),
];

/// `ML_defaultDTable` — precomputed FSE table for Match Lengths.
static ML_DEFAULT_DTABLE: [ZSTD_seqSymbol; (1 << ML_DEFAULTNORMLOG) + 1] = [
    ZSTD_seqSymbol { nextState: 1, nbAdditionalBits: 1, nbBits: 1, baseValue: ML_DEFAULTNORMLOG },
    s(0, 0, 6, 3), s(0, 0, 4, 4),
    s(32, 0, 5, 5), s(0, 0, 5, 6),
    s(0, 0, 5, 8), s(0, 0, 5, 9),
    s(0, 0, 5, 11), s(0, 0, 6, 13),
    s(0, 0, 6, 16), s(0, 0, 6, 19),
    s(0, 0, 6, 22), s(0, 0, 6, 25),
    s(0, 0, 6, 28), s(0, 0, 6, 31),
    s(0, 0, 6, 34), s(0, 1, 6, 37),
    s(0, 1, 6, 41), s(0, 2, 6, 47),
    s(0, 3, 6, 59), s(0, 4, 6, 83),
    s(0, 7, 6, 131), s(0, 9, 6, 515),
    s(16, 0, 4, 4), s(0, 0, 4, 5),
    s(32, 0, 5, 6), s(0, 0, 5, 7),
    s(32, 0, 5, 9), s(0, 0, 5, 10),
    s(0, 0, 6, 12), s(0, 0, 6, 15),
    s(0, 0, 6, 18), s(0, 0, 6, 21),
    s(0, 0, 6, 24), s(0, 0, 6, 27),
    s(0, 0, 6, 30), s(0, 0, 6, 33),
    s(0, 1, 6, 35), s(0, 1, 6, 39),
    s(0, 2, 6, 43), s(0, 3, 6, 51),
    s(0, 4, 6, 67), s(0, 5, 6, 99),
    s(0, 8, 6, 259), s(32, 0, 4, 4),
    s(48, 0, 4, 4), s(16, 0, 4, 5),
    s(32, 0, 5, 7), s(32, 0, 5, 8),
    s(32, 0, 5, 10), s(32, 0, 5, 11),
    s(0, 0, 6, 14), s(0, 0, 6, 17),
    s(0, 0, 6, 20), s(0, 0, 6, 23),
    s(0, 0, 6, 26), s(0, 0, 6, 29),
    s(0, 0, 6, 32), s(0, 16, 6, 65539),
    s(0, 15, 6, 32771), s(0, 14, 6, 16387),
    s(0, 13, 6, 8195), s(0, 12, 6, 4099),
    s(0, 11, 6, 2051), s(0, 10, 6, 1027),
];

/// Helper to build a `ZSTD_seqSymbol` const entry (nextState, nbAddBits, nbBits, baseVal).
const fn s(next_state: u16, nb_add_bits: u8, nb_bits: u8, base_value: u32) -> ZSTD_seqSymbol {
    ZSTD_seqSymbol {
        nextState: next_state,
        nbAdditionalBits: nb_add_bits,
        nbBits: nb_bits,
        baseValue: base_value,
    }
}

/// `ZSTD_buildSeqTable_rle()`
unsafe fn zstd_build_seq_table_rle(dt: *mut ZSTD_seqSymbol, base_value: U32, nb_add_bits: u8) {
    let dtable_h = dt as *mut ZSTD_seqSymbol_header;
    let cell = dt.add(1);

    (*dtable_h).tableLog = 0;
    (*dtable_h).fastMode = 0;

    (*cell).nbBits = 0;
    (*cell).nextState = 0;
    (*cell).nbAdditionalBits = nb_add_bits;
    (*cell).baseValue = base_value;
}

/// `FSE_TABLESTEP(tableSize)`
#[inline(always)]
const fn fse_tablestep(table_size: usize) -> usize {
    (table_size >> 1) + (table_size >> 3) + 3
}

/// `ZSTD_buildFSETable_body()`
#[inline(always)]
unsafe fn zstd_build_fse_table_body(
    dt: *mut ZSTD_seqSymbol,
    normalized_counter: *const i16,
    max_symbol_value: c_uint,
    base_value: *const U32,
    nb_additional_bits: *const u8,
    table_log: c_uint,
    wksp: *mut c_void,
    _wksp_size: usize,
) {
    let table_decode = dt.add(1);
    let max_sv1: U32 = max_symbol_value + 1;
    let table_size: U32 = 1 << table_log;

    let symbol_next = wksp as *mut U16;
    let spread = symbol_next.add(MaxSeq as usize + 1) as *mut u8;
    let mut high_threshold = table_size - 1;

    /* Init, lay down lowprob symbols */
    {
        let mut dtable_h = ZSTD_seqSymbol_header {
            tableLog: table_log,
            fastMode: 1,
        };
        {
            let large_limit: S16 = (1i32 << (table_log - 1)) as S16;
            for sym in 0..max_sv1 {
                let nc = *normalized_counter.add(sym as usize);
                if nc == -1 {
                    (*table_decode.add(high_threshold as usize)).baseValue = sym;
                    high_threshold -= 1;
                    *symbol_next.add(sym as usize) = 1;
                } else {
                    if nc >= large_limit {
                        dtable_h.fastMode = 0;
                    }
                    *symbol_next.add(sym as usize) = nc as U16;
                }
            }
        }
        core::ptr::copy_nonoverlapping(
            &dtable_h as *const ZSTD_seqSymbol_header as *const u8,
            dt as *mut u8,
            core::mem::size_of::<ZSTD_seqSymbol_header>(),
        );
    }

    /* Spread symbols */
    if high_threshold == table_size - 1 {
        let table_mask = (table_size - 1) as usize;
        let step = fse_tablestep(table_size as usize);
        {
            let add: U64 = 0x0101010101010101;
            let mut pos: usize = 0;
            let mut sv: U64 = 0;
            for sym in 0..max_sv1 {
                let n = *normalized_counter.add(sym as usize) as i32;
                mem_write64(spread.add(pos), sv);
                let mut i = 8i32;
                while i < n {
                    mem_write64(spread.add(pos + i as usize), sv);
                    i += 8;
                }
                pos += n as usize;
                sv = sv.wrapping_add(add);
            }
        }
        {
            let mut position: usize = 0;
            let unroll: usize = 2;
            let mut sidx: usize = 0;
            while sidx < table_size as usize {
                for u in 0..unroll {
                    let u_position = (position + (u * step)) & table_mask;
                    (*table_decode.add(u_position)).baseValue = *spread.add(sidx + u) as U32;
                }
                position = (position + (unroll * step)) & table_mask;
                sidx += unroll;
            }
        }
    } else {
        let table_mask = table_size - 1;
        let step = fse_tablestep(table_size as usize) as U32;
        let mut position: U32 = 0;
        for sym in 0..max_sv1 {
            let n = *normalized_counter.add(sym as usize) as i32;
            let mut i = 0i32;
            while i < n {
                (*table_decode.add(position as usize)).baseValue = sym;
                position = (position + step) & table_mask;
                while position > high_threshold {
                    position = (position + step) & table_mask; /* lowprob area */
                }
                i += 1;
            }
        }
    }

    /* Build Decoding table */
    {
        for u in 0..table_size {
            let symbol = (*table_decode.add(u as usize)).baseValue;
            let next_state = *symbol_next.add(symbol as usize);
            *symbol_next.add(symbol as usize) = next_state + 1;
            (*table_decode.add(u as usize)).nbBits =
                (table_log - crate::bits::zstd_highbit32(next_state as U32)) as u8;
            (*table_decode.add(u as usize)).nextState =
                (((next_state as U32) << (*table_decode.add(u as usize)).nbBits) as U32)
                    .wrapping_sub(table_size) as U16;
            (*table_decode.add(u as usize)).nbAdditionalBits = *nb_additional_bits.add(symbol as usize);
            (*table_decode.add(u as usize)).baseValue = *base_value.add(symbol as usize);
        }
    }
}

/// `ZSTD_buildFSETable_body_default()`
unsafe fn zstd_build_fse_table_body_default(
    dt: *mut ZSTD_seqSymbol,
    normalized_counter: *const i16,
    max_symbol_value: c_uint,
    base_value: *const U32,
    nb_additional_bits: *const u8,
    table_log: c_uint,
    wksp: *mut c_void,
    wksp_size: usize,
) {
    zstd_build_fse_table_body(
        dt,
        normalized_counter,
        max_symbol_value,
        base_value,
        nb_additional_bits,
        table_log,
        wksp,
        wksp_size,
    );
}

/// `ZSTD_buildFSETable()`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_buildFSETable(
    dt: *mut ZSTD_seqSymbol,
    normalized_counter: *const i16,
    max_symbol_value: c_uint,
    base_value: *const U32,
    nb_additional_bits: *const u8,
    table_log: c_uint,
    wksp: *mut c_void,
    wksp_size: usize,
    _bmi2: c_int,
) {
    zstd_build_fse_table_body_default(
        dt,
        normalized_counter,
        max_symbol_value,
        base_value,
        nb_additional_bits,
        table_log,
        wksp,
        wksp_size,
    );
}

/// `ZSTD_buildSeqTable()`
unsafe fn zstd_build_seq_table(
    dtable_space: *mut ZSTD_seqSymbol,
    dtable_ptr: *mut *const ZSTD_seqSymbol,
    type_: u32,
    max: c_uint,
    max_log: U32,
    src: *const c_void,
    src_size: usize,
    base_value: *const U32,
    nb_additional_bits: *const u8,
    default_table: *const ZSTD_seqSymbol,
    flag_repeat_table: U32,
    _ddict_is_cold: c_int,
    _nb_seq: c_int,
    wksp: *mut U32,
    wksp_size: usize,
    bmi2: c_int,
) -> usize {
    match type_ {
        x if x == set_rle => {
            if src_size == 0 {
                return err_code(ZSTD_error_srcSize_wrong);
            }
            if (*(src as *const u8) as c_uint) > max {
                return err_code(ZSTD_error_corruption_detected);
            }
            {
                let symbol = *(src as *const u8) as u32;
                let baseline = *base_value.add(symbol as usize);
                let nb_bits = *nb_additional_bits.add(symbol as usize);
                zstd_build_seq_table_rle(dtable_space, baseline, nb_bits);
            }
            *dtable_ptr = dtable_space;
            1
        }
        x if x == set_basic => {
            *dtable_ptr = default_table;
            0
        }
        x if x == set_repeat => {
            if flag_repeat_table == 0 {
                return err_code(ZSTD_error_corruption_detected);
            }
            /* prefetch FSE table if used — PREFETCH_AREA no-op */
            0
        }
        x if x == set_compressed => {
            let mut table_log: c_uint = 0;
            let mut norm: [S16; MaxSeq as usize + 1] = [0; MaxSeq as usize + 1];
            let mut max_mut: c_uint = max;
            let header_size = FSE_readNCount(
                norm.as_mut_ptr(),
                &mut max_mut,
                &mut table_log,
                src,
                src_size,
            );
            if err_is_error(header_size) {
                return err_code(ZSTD_error_corruption_detected);
            }
            if table_log > max_log {
                return err_code(ZSTD_error_corruption_detected);
            }
            ZSTD_buildFSETable(
                dtable_space,
                norm.as_ptr(),
                max_mut,
                base_value,
                nb_additional_bits,
                table_log,
                wksp as *mut c_void,
                wksp_size,
                bmi2,
            );
            *dtable_ptr = dtable_space;
            header_size
        }
        _ => err_code(ZSTD_error_GENERIC),
    }
}

/// `ZSTD_decodeSeqHeaders()`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_decodeSeqHeaders(
    dctx: *mut ZSTD_DCtx,
    nb_seq_ptr: *mut c_int,
    src: *const c_void,
    src_size: usize,
) -> usize {
    let istart = src as *const u8;
    let iend = istart.add(src_size);
    let mut ip = istart;
    let mut nb_seq: c_int;

    /* check */
    if src_size < MIN_SEQUENCES_SIZE {
        return err_code(ZSTD_error_srcSize_wrong);
    }

    /* SeqHead */
    nb_seq = *ip as c_int;
    ip = ip.add(1);
    if nb_seq > 0x7F {
        if nb_seq == 0xFF {
            if ip.add(2) > iend {
                return err_code(ZSTD_error_srcSize_wrong);
            }
            nb_seq = mem_read_le16(ip) as c_int + LONGNBSEQ as c_int;
            ip = ip.add(2);
        } else {
            if ip >= iend {
                return err_code(ZSTD_error_srcSize_wrong);
            }
            nb_seq = ((nb_seq - 0x80) << 8) + *ip as c_int;
            ip = ip.add(1);
        }
    }
    *nb_seq_ptr = nb_seq;

    if nb_seq == 0 {
        /* No sequence : section ends immediately */
        if ip != iend {
            return err_code(ZSTD_error_corruption_detected);
        }
        return (ip as usize) - (istart as usize);
    }

    /* FSE table descriptors */
    if ip.add(1) > iend {
        return err_code(ZSTD_error_srcSize_wrong);
    }
    if *ip & 3 != 0 {
        return err_code(ZSTD_error_corruption_detected);
    }
    {
        let ll_type: u32 = (*ip >> 6) as u32;
        let of_type: u32 = ((*ip >> 4) & 3) as u32;
        let ml_type: u32 = ((*ip >> 2) & 3) as u32;
        ip = ip.add(1);

        /* Build DTables */
        {
            let llh_size = zstd_build_seq_table(
                (*dctx).entropy.LLTable.as_mut_ptr(),
                &mut (*dctx).LLTptr,
                ll_type,
                MaxLL,
                LLFSELog,
                ip as *const c_void,
                (iend as usize) - (ip as usize),
                LL_base.as_ptr(),
                LL_bits.as_ptr(),
                LL_DEFAULT_DTABLE.as_ptr(),
                (*dctx).fseEntropy,
                (*dctx).ddictIsCold,
                nb_seq,
                (*dctx).workspace.as_mut_ptr(),
                core::mem::size_of_val(&(*dctx).workspace),
                zstd_dctx_get_bmi2(dctx),
            );
            if err_is_error(llh_size) {
                return err_code(ZSTD_error_corruption_detected);
            }
            ip = ip.add(llh_size);
        }

        {
            let ofh_size = zstd_build_seq_table(
                (*dctx).entropy.OFTable.as_mut_ptr(),
                &mut (*dctx).OFTptr,
                of_type,
                MaxOff,
                OffFSELog,
                ip as *const c_void,
                (iend as usize) - (ip as usize),
                OF_base.as_ptr(),
                OF_bits.as_ptr(),
                OF_DEFAULT_DTABLE.as_ptr(),
                (*dctx).fseEntropy,
                (*dctx).ddictIsCold,
                nb_seq,
                (*dctx).workspace.as_mut_ptr(),
                core::mem::size_of_val(&(*dctx).workspace),
                zstd_dctx_get_bmi2(dctx),
            );
            if err_is_error(ofh_size) {
                return err_code(ZSTD_error_corruption_detected);
            }
            ip = ip.add(ofh_size);
        }

        {
            let mlh_size = zstd_build_seq_table(
                (*dctx).entropy.MLTable.as_mut_ptr(),
                &mut (*dctx).MLTptr,
                ml_type,
                MaxML,
                MLFSELog,
                ip as *const c_void,
                (iend as usize) - (ip as usize),
                ML_base.as_ptr(),
                ML_bits.as_ptr(),
                ML_DEFAULT_DTABLE.as_ptr(),
                (*dctx).fseEntropy,
                (*dctx).ddictIsCold,
                nb_seq,
                (*dctx).workspace.as_mut_ptr(),
                core::mem::size_of_val(&(*dctx).workspace),
                zstd_dctx_get_bmi2(dctx),
            );
            if err_is_error(mlh_size) {
                return err_code(ZSTD_error_corruption_detected);
            }
            ip = ip.add(mlh_size);
        }
    }

    (ip as usize) - (istart as usize)
}

/// `seq_t`
#[repr(C)]
#[derive(Clone, Copy)]
struct seq_t {
    litLength: usize,
    matchLength: usize,
    offset: usize,
}

/// `ZSTD_fseState`
#[repr(C)]
#[derive(Clone, Copy)]
struct ZSTD_fseState {
    state: usize,
    table: *const ZSTD_seqSymbol,
}

/// `seqState_t`
#[repr(C)]
struct seqState_t {
    DStream: BIT_DStream_t,
    stateLL: ZSTD_fseState,
    stateOffb: ZSTD_fseState,
    stateML: ZSTD_fseState,
    prevOffset: [usize; ZSTD_REP_NUM],
}

/// `ZSTD_overlapCopy8()`
#[inline]
unsafe fn zstd_overlap_copy8(op: &mut *mut u8, ip: &mut *const u8, offset: usize) {
    if offset < 8 {
        /* close range match, overlap */
        const DEC32TABLE: [U32; 8] = [0, 1, 2, 1, 4, 4, 4, 4];
        const DEC64TABLE: [c_int; 8] = [8, 8, 8, 7, 8, 9, 10, 11];
        let sub2 = DEC64TABLE[offset];
        *(*op).add(0) = *(*ip).add(0);
        *(*op).add(1) = *(*ip).add(1);
        *(*op).add(2) = *(*ip).add(2);
        *(*op).add(3) = *(*ip).add(3);
        *ip = (*ip).add(DEC32TABLE[offset] as usize);
        zstd_copy4((*op).add(4) as *mut c_void, *ip as *const c_void);
        *ip = (*ip).offset(-(sub2 as isize));
    } else {
        zstd_copy8(*op, *ip);
    }
    *ip = (*ip).add(8);
    *op = (*op).add(8);
}

/// `ZSTD_safecopy()`
unsafe fn zstd_safecopy(
    mut op: *mut u8,
    oend_w: *const u8,
    mut ip: *const u8,
    mut length: isize,
    ovtype: ZSTD_overlap_e,
) {
    let diff = (op as isize) - (ip as isize);
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
    if ovtype == ZSTD_overlap_e::ZSTD_overlap_src_before_dst {
        /* Copy 8 bytes and ensure the offset >= 8 when there can be overlap. */
        let mut opm = op;
        let mut ipm = ip;
        zstd_overlap_copy8(&mut opm, &mut ipm, diff as usize);
        op = opm;
        ip = ipm;
        length -= 8;
    }

    if oend <= oend_w as *mut u8 {
        /* No risk of overwrite. */
        zstd_wildcopy(op, ip, length, ovtype);
        return;
    }
    if op <= oend_w as *mut u8 {
        /* Wildcopy until we get close to the end. */
        let n = (oend_w as isize) - (op as isize);
        zstd_wildcopy(op, ip, n, ovtype);
        ip = ip.offset(n);
        op = op.offset(n);
    }
    /* Handle the leftovers. */
    while op < oend {
        *op = *ip;
        op = op.add(1);
        ip = ip.add(1);
    }
}

/// `ZSTD_safecopyDstBeforeSrc()`
unsafe fn zstd_safecopy_dst_before_src(mut op: *mut u8, mut ip: *const u8, length: isize) {
    let diff = (op as isize) - (ip as isize);
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

    if op <= oend.offset(-(WILDCOPY_OVERLENGTH as isize)) && diff < -WILDCOPY_VECLEN {
        let n = (oend as isize) - (WILDCOPY_OVERLENGTH as isize) - (op as isize);
        zstd_wildcopy(op, ip, n, ZSTD_overlap_e::ZSTD_no_overlap);
        ip = ip.offset(n);
        op = op.offset(n);
    }

    /* Handle the leftovers. */
    while op < oend {
        *op = *ip;
        op = op.add(1);
        ip = ip.add(1);
    }
}

/// `ZSTD_execSequenceEnd()`
unsafe fn zstd_exec_sequence_end(
    mut op: *mut u8,
    oend: *mut u8,
    mut sequence: seq_t,
    lit_ptr: &mut *const u8,
    lit_limit: *const u8,
    prefix_start: *const u8,
    virtual_start: *const u8,
    dict_end: *const u8,
) -> usize {
    let o_lit_end = op.add(sequence.litLength);
    let sequence_length = sequence.litLength + sequence.matchLength;
    let i_lit_end = (*lit_ptr).add(sequence.litLength);
    let mut match_ = o_lit_end.offset(-(sequence.offset as isize)) as *const u8;
    let oend_w = oend.offset(-(WILDCOPY_OVERLENGTH as isize));

    /* bounds checks */
    if sequence_length > (oend as usize) - (op as usize) {
        return err_code(ZSTD_error_dstSize_tooSmall);
    }
    if sequence.litLength > (lit_limit as usize) - (*lit_ptr as usize) {
        return err_code(ZSTD_error_corruption_detected);
    }

    /* copy literals */
    zstd_safecopy(
        op,
        oend_w,
        *lit_ptr,
        sequence.litLength as isize,
        ZSTD_overlap_e::ZSTD_no_overlap,
    );
    op = o_lit_end;
    *lit_ptr = i_lit_end;

    /* copy Match */
    if sequence.offset > (o_lit_end as usize) - (prefix_start as usize) {
        /* offset beyond prefix */
        if sequence.offset > (o_lit_end as usize) - (virtual_start as usize) {
            return err_code(ZSTD_error_corruption_detected);
        }
        match_ = dict_end.offset(-((prefix_start as isize) - (match_ as isize)));
        if match_.add(sequence.matchLength) <= dict_end {
            core::ptr::copy(match_, o_lit_end, sequence.matchLength);
            return sequence_length;
        }
        /* span extDict & currentPrefixSegment */
        {
            let length1 = (dict_end as usize) - (match_ as usize);
            core::ptr::copy(match_, o_lit_end, length1);
            op = o_lit_end.add(length1);
            sequence.matchLength -= length1;
            match_ = prefix_start;
        }
    }
    zstd_safecopy(
        op,
        oend_w,
        match_,
        sequence.matchLength as isize,
        ZSTD_overlap_e::ZSTD_overlap_src_before_dst,
    );
    sequence_length
}

/// `ZSTD_execSequenceEndSplitLitBuffer()`
unsafe fn zstd_exec_sequence_end_split_lit_buffer(
    mut op: *mut u8,
    oend: *mut u8,
    oend_w: *const u8,
    mut sequence: seq_t,
    lit_ptr: &mut *const u8,
    lit_limit: *const u8,
    prefix_start: *const u8,
    virtual_start: *const u8,
    dict_end: *const u8,
) -> usize {
    let o_lit_end = op.add(sequence.litLength);
    let sequence_length = sequence.litLength + sequence.matchLength;
    let i_lit_end = (*lit_ptr).add(sequence.litLength);
    let mut match_ = o_lit_end.offset(-(sequence.offset as isize)) as *const u8;

    /* bounds checks */
    if sequence_length > (oend as usize) - (op as usize) {
        return err_code(ZSTD_error_dstSize_tooSmall);
    }
    if sequence.litLength > (lit_limit as usize) - (*lit_ptr as usize) {
        return err_code(ZSTD_error_corruption_detected);
    }

    /* copy literals */
    if op > *lit_ptr as *mut u8 && op < (*lit_ptr).add(sequence.litLength) as *mut u8 {
        return err_code(ZSTD_error_dstSize_tooSmall);
    }
    zstd_safecopy_dst_before_src(op, *lit_ptr, sequence.litLength as isize);
    op = o_lit_end;
    *lit_ptr = i_lit_end;

    /* copy Match */
    if sequence.offset > (o_lit_end as usize) - (prefix_start as usize) {
        /* offset beyond prefix */
        if sequence.offset > (o_lit_end as usize) - (virtual_start as usize) {
            return err_code(ZSTD_error_corruption_detected);
        }
        match_ = dict_end.offset(-((prefix_start as isize) - (match_ as isize)));
        if match_.add(sequence.matchLength) <= dict_end {
            core::ptr::copy(match_, o_lit_end, sequence.matchLength);
            return sequence_length;
        }
        /* span extDict & currentPrefixSegment */
        {
            let length1 = (dict_end as usize) - (match_ as usize);
            core::ptr::copy(match_, o_lit_end, length1);
            op = o_lit_end.add(length1);
            sequence.matchLength -= length1;
            match_ = prefix_start;
        }
    }
    zstd_safecopy(
        op,
        oend_w,
        match_,
        sequence.matchLength as isize,
        ZSTD_overlap_e::ZSTD_overlap_src_before_dst,
    );
    sequence_length
}

/// `ZSTD_execSequence()`
unsafe fn zstd_exec_sequence(
    mut op: *mut u8,
    oend: *mut u8,
    mut sequence: seq_t,
    lit_ptr: &mut *const u8,
    lit_limit: *const u8,
    prefix_start: *const u8,
    virtual_start: *const u8,
    dict_end: *const u8,
) -> usize {
    let o_lit_end = op.add(sequence.litLength);
    let sequence_length = sequence.litLength + sequence.matchLength;
    let o_match_end = op.add(sequence_length);
    let oend_w = oend.offset(-(WILDCOPY_OVERLENGTH as isize));
    let i_lit_end = (*lit_ptr).add(sequence.litLength);
    let mut match_ = o_lit_end.offset(-(sequence.offset as isize)) as *const u8;

    /* Handle edge cases in a slow path */
    if i_lit_end > lit_limit
        || o_match_end > oend_w
        || (mem_32bits() && ((oend as usize) - (op as usize)) < sequence_length + WILDCOPY_OVERLENGTH)
    {
        return zstd_exec_sequence_end(
            op, oend, sequence, lit_ptr, lit_limit, prefix_start, virtual_start, dict_end,
        );
    }

    /* Copy Literals */
    zstd_copy16(op, *lit_ptr);
    if sequence.litLength > 16 {
        zstd_wildcopy(
            op.add(16),
            (*lit_ptr).add(16),
            (sequence.litLength - 16) as isize,
            ZSTD_overlap_e::ZSTD_no_overlap,
        );
    }
    op = o_lit_end;
    *lit_ptr = i_lit_end;

    /* Copy Match */
    if sequence.offset > (o_lit_end as usize) - (prefix_start as usize) {
        /* offset beyond prefix -> go into extDict */
        if sequence.offset > (o_lit_end as usize) - (virtual_start as usize) {
            return err_code(ZSTD_error_corruption_detected);
        }
        match_ = dict_end.offset((match_ as isize) - (prefix_start as isize));
        if match_.add(sequence.matchLength) <= dict_end {
            core::ptr::copy(match_, o_lit_end, sequence.matchLength);
            return sequence_length;
        }
        /* span extDict & currentPrefixSegment */
        {
            let length1 = (dict_end as usize) - (match_ as usize);
            core::ptr::copy(match_, o_lit_end, length1);
            op = o_lit_end.add(length1);
            sequence.matchLength -= length1;
            match_ = prefix_start;
        }
    }

    /* Nearly all offsets are >= WILDCOPY_VECLEN bytes */
    if sequence.offset as isize >= WILDCOPY_VECLEN {
        zstd_wildcopy(
            op,
            match_,
            sequence.matchLength as isize,
            ZSTD_overlap_e::ZSTD_no_overlap,
        );
        return sequence_length;
    }

    /* Copy 8 bytes and spread the offset to be >= 8. */
    {
        let mut mp = match_;
        zstd_overlap_copy8(&mut op, &mut mp, sequence.offset);
        match_ = mp;
    }

    /* If the match length is > 8 bytes, then continue with the wildcopy. */
    if sequence.matchLength > 8 {
        zstd_wildcopy(
            op,
            match_,
            (sequence.matchLength as isize) - 8,
            ZSTD_overlap_e::ZSTD_overlap_src_before_dst,
        );
    }
    sequence_length
}

/// `ZSTD_execSequenceSplitLitBuffer()`
unsafe fn zstd_exec_sequence_split_lit_buffer(
    mut op: *mut u8,
    oend: *mut u8,
    oend_w: *const u8,
    mut sequence: seq_t,
    lit_ptr: &mut *const u8,
    lit_limit: *const u8,
    prefix_start: *const u8,
    virtual_start: *const u8,
    dict_end: *const u8,
) -> usize {
    let o_lit_end = op.add(sequence.litLength);
    let sequence_length = sequence.litLength + sequence.matchLength;
    let o_match_end = op.add(sequence_length);
    let i_lit_end = (*lit_ptr).add(sequence.litLength);
    let mut match_ = o_lit_end.offset(-(sequence.offset as isize)) as *const u8;

    /* Handle edge cases in a slow path */
    if i_lit_end > lit_limit
        || o_match_end > oend_w as *mut u8
        || (mem_32bits() && ((oend as usize) - (op as usize)) < sequence_length + WILDCOPY_OVERLENGTH)
    {
        return zstd_exec_sequence_end_split_lit_buffer(
            op, oend, oend_w, sequence, lit_ptr, lit_limit, prefix_start, virtual_start, dict_end,
        );
    }

    /* Copy Literals */
    zstd_copy16(op, *lit_ptr);
    if sequence.litLength > 16 {
        zstd_wildcopy(
            op.add(16),
            (*lit_ptr).add(16),
            (sequence.litLength - 16) as isize,
            ZSTD_overlap_e::ZSTD_no_overlap,
        );
    }
    op = o_lit_end;
    *lit_ptr = i_lit_end;

    /* Copy Match */
    if sequence.offset > (o_lit_end as usize) - (prefix_start as usize) {
        /* offset beyond prefix -> go into extDict */
        if sequence.offset > (o_lit_end as usize) - (virtual_start as usize) {
            return err_code(ZSTD_error_corruption_detected);
        }
        match_ = dict_end.offset((match_ as isize) - (prefix_start as isize));
        if match_.add(sequence.matchLength) <= dict_end {
            core::ptr::copy(match_, o_lit_end, sequence.matchLength);
            return sequence_length;
        }
        /* span extDict & currentPrefixSegment */
        {
            let length1 = (dict_end as usize) - (match_ as usize);
            core::ptr::copy(match_, o_lit_end, length1);
            op = o_lit_end.add(length1);
            sequence.matchLength -= length1;
            match_ = prefix_start;
        }
    }

    /* Nearly all offsets are >= WILDCOPY_VECLEN bytes */
    if sequence.offset as isize >= WILDCOPY_VECLEN {
        zstd_wildcopy(
            op,
            match_,
            sequence.matchLength as isize,
            ZSTD_overlap_e::ZSTD_no_overlap,
        );
        return sequence_length;
    }

    /* Copy 8 bytes and spread the offset to be >= 8. */
    {
        let mut mp = match_;
        zstd_overlap_copy8(&mut op, &mut mp, sequence.offset);
        match_ = mp;
    }

    /* If the match length is > 8 bytes, then continue with the wildcopy. */
    if sequence.matchLength > 8 {
        zstd_wildcopy(
            op,
            match_,
            (sequence.matchLength as isize) - 8,
            ZSTD_overlap_e::ZSTD_overlap_src_before_dst,
        );
    }
    sequence_length
}

/// `ZSTD_initFseState()`
unsafe fn zstd_init_fse_state(
    d_state_ptr: &mut ZSTD_fseState,
    bit_d: &mut BIT_DStream_t,
    dt: *const ZSTD_seqSymbol,
) {
    let dtable_h = dt as *const ZSTD_seqSymbol_header;
    d_state_ptr.state = bit_read_bits(bit_d, (*dtable_h).tableLog) as usize;
    bit_reload_dstream(bit_d);
    d_state_ptr.table = dt.add(1);
}

/// `ZSTD_updateFseStateWithDInfo()`
#[inline(always)]
unsafe fn zstd_update_fse_state_with_dinfo(
    d_state_ptr: &mut ZSTD_fseState,
    bit_d: &mut BIT_DStream_t,
    next_state: U16,
    nb_bits: U32,
) {
    let low_bits = bit_read_bits(bit_d, nb_bits);
    d_state_ptr.state = (next_state as usize) + (low_bits as usize);
}

/// `LONG_OFFSETS_MAX_EXTRA_BITS_32`
const LONG_OFFSETS_MAX_EXTRA_BITS_32: U32 =
    if ZSTD_WINDOWLOG_MAX_32 > STREAM_ACCUMULATOR_MIN_32 {
        ZSTD_WINDOWLOG_MAX_32 - STREAM_ACCUMULATOR_MIN_32
    } else {
        0
    };

/// `ZSTD_longOffset_e`
#[derive(Clone, Copy, PartialEq, Eq)]
enum ZSTD_longOffset_e {
    ZSTD_lo_isRegularOffset = 0,
    ZSTD_lo_isLongOffset = 1,
}

/// `ZSTD_decodeSequence()`
#[inline(always)]
unsafe fn zstd_decode_sequence(
    seq_state: &mut seqState_t,
    long_offsets: ZSTD_longOffset_e,
    is_last_seq: c_int,
) -> seq_t {
    let mut seq = seq_t {
        litLength: 0,
        matchLength: 0,
        offset: 0,
    };

    let ll_dinfo = seq_state.stateLL.table.add(seq_state.stateLL.state);
    let ml_dinfo = seq_state.stateML.table.add(seq_state.stateML.state);
    let of_dinfo = seq_state.stateOffb.table.add(seq_state.stateOffb.state);

    seq.matchLength = (*ml_dinfo).baseValue as usize;
    seq.litLength = (*ll_dinfo).baseValue as usize;
    {
        let of_base: U32 = (*of_dinfo).baseValue;
        let ll_bits: u8 = (*ll_dinfo).nbAdditionalBits;
        let ml_bits: u8 = (*ml_dinfo).nbAdditionalBits;
        let of_bits: u8 = (*of_dinfo).nbAdditionalBits;
        let total_bits: u8 = ll_bits.wrapping_add(ml_bits).wrapping_add(of_bits);

        let ll_next: U16 = (*ll_dinfo).nextState;
        let ml_next: U16 = (*ml_dinfo).nextState;
        let of_next: U16 = (*of_dinfo).nextState;
        let ll_nb_bits: U32 = (*ll_dinfo).nbBits as U32;
        let ml_nb_bits: U32 = (*ml_dinfo).nbBits as U32;
        let of_nb_bits: U32 = (*of_dinfo).nbBits as U32;

        /* sequence */
        {
            let offset: usize;
            if of_bits > 1 {
                if mem_32bits()
                    && long_offsets == ZSTD_longOffset_e::ZSTD_lo_isLongOffset
                    && (of_bits as U32 >= STREAM_ACCUMULATOR_MIN_32)
                {
                    /* Always read extra bits */
                    let extra_bits: U32 = LONG_OFFSETS_MAX_EXTRA_BITS_32;
                    let mut off = (of_base as usize)
                        + ((bit_read_bits_fast(&mut seq_state.DStream, of_bits as U32 - extra_bits)
                            << extra_bits) as usize);
                    bit_reload_dstream(&mut seq_state.DStream);
                    off += bit_read_bits_fast(&mut seq_state.DStream, extra_bits) as usize;
                    offset = off;
                } else {
                    offset = (of_base as usize)
                        + (bit_read_bits_fast(&mut seq_state.DStream, of_bits as U32) as usize);
                    if mem_32bits() {
                        bit_reload_dstream(&mut seq_state.DStream);
                    }
                }
                seq_state.prevOffset[2] = seq_state.prevOffset[1];
                seq_state.prevOffset[1] = seq_state.prevOffset[0];
                seq_state.prevOffset[0] = offset;
                seq.offset = offset;
            } else {
                let ll0: U32 = ((*ll_dinfo).baseValue == 0) as U32;
                if of_bits == 0 {
                    let offset = seq_state.prevOffset[ll0 as usize];
                    seq_state.prevOffset[1] = seq_state.prevOffset[(ll0 == 0) as usize];
                    seq_state.prevOffset[0] = offset;
                    seq.offset = offset;
                } else {
                    let mut offset = (of_base as usize)
                        + (ll0 as usize)
                        + (bit_read_bits_fast(&mut seq_state.DStream, 1) as usize);
                    {
                        let mut temp = if offset == 3 {
                            seq_state.prevOffset[0] - 1
                        } else {
                            seq_state.prevOffset[offset]
                        };
                        temp -= (temp == 0) as usize;
                        if offset != 1 {
                            seq_state.prevOffset[2] = seq_state.prevOffset[1];
                        }
                        seq_state.prevOffset[1] = seq_state.prevOffset[0];
                        offset = temp;
                        seq_state.prevOffset[0] = offset;
                        seq.offset = offset;
                    }
                }
            }
        }

        if ml_bits > 0 {
            seq.matchLength += bit_read_bits_fast(&mut seq_state.DStream, ml_bits as U32) as usize;
        }

        if mem_32bits()
            && (ml_bits as U32 + ll_bits as U32
                >= STREAM_ACCUMULATOR_MIN_32 - LONG_OFFSETS_MAX_EXTRA_BITS_32)
        {
            bit_reload_dstream(&mut seq_state.DStream);
        }
        if mem_64bits()
            && (total_bits as U32
                >= STREAM_ACCUMULATOR_MIN_64 - (LLFSELog + MLFSELog + OffFSELog))
        {
            bit_reload_dstream(&mut seq_state.DStream);
        }

        if ll_bits > 0 {
            seq.litLength += bit_read_bits_fast(&mut seq_state.DStream, ll_bits as U32) as usize;
        }

        if mem_32bits() {
            bit_reload_dstream(&mut seq_state.DStream);
        }

        if is_last_seq == 0 {
            /* don't update FSE state for last Sequence */
            zstd_update_fse_state_with_dinfo(
                &mut seq_state.stateLL,
                &mut seq_state.DStream,
                ll_next,
                ll_nb_bits,
            );
            zstd_update_fse_state_with_dinfo(
                &mut seq_state.stateML,
                &mut seq_state.DStream,
                ml_next,
                ml_nb_bits,
            );
            if mem_32bits() {
                bit_reload_dstream(&mut seq_state.DStream);
            }
            zstd_update_fse_state_with_dinfo(
                &mut seq_state.stateOffb,
                &mut seq_state.DStream,
                of_next,
                of_nb_bits,
            );
            bit_reload_dstream(&mut seq_state.DStream);
        }
    }

    seq
}

/// `ZSTD_decompressSequences_bodySplitLitBuffer()`
unsafe fn zstd_decompress_sequences_body_split_lit_buffer(
    dctx: *mut ZSTD_DCtx,
    dst: *mut c_void,
    max_dst_size: usize,
    seq_start: *const c_void,
    seq_size: usize,
    mut nb_seq: c_int,
    is_long_offset: ZSTD_longOffset_e,
) -> usize {
    let ip = seq_start as *const u8;
    let iend = ip.add(seq_size);
    let ostart = dst as *mut u8;
    let oend = zstd_maybe_null_ptr_add(ostart, max_dst_size as isize);
    let mut op = ostart;
    let mut lit_ptr = (*dctx).litPtr;
    let mut lit_buffer_end = (*dctx).litBufferEnd;
    let prefix_start = (*dctx).prefixStart as *const u8;
    let v_base = (*dctx).virtualStart as *const u8;
    let dict_end = (*dctx).dictEnd as *const u8;

    /* Literals are split between internal buffer & output buffer */
    if nb_seq != 0 {
        let mut seq_state: seqState_t = core::mem::zeroed();
        (*dctx).fseEntropy = 1;
        for i in 0..ZSTD_REP_NUM {
            seq_state.prevOffset[i] = (*dctx).entropy.rep[i] as usize;
        }
        if err_is_error(bit_init_dstream(
            &mut seq_state.DStream,
            ip,
            (iend as usize) - (ip as usize),
        )) {
            return err_code(ZSTD_error_corruption_detected);
        }
        zstd_init_fse_state(&mut seq_state.stateLL, &mut seq_state.DStream, (*dctx).LLTptr);
        zstd_init_fse_state(&mut seq_state.stateOffb, &mut seq_state.DStream, (*dctx).OFTptr);
        zstd_init_fse_state(&mut seq_state.stateML, &mut seq_state.DStream, (*dctx).MLTptr);

        /* decompress without overrunning litPtr begins */
        {
            let mut sequence = seq_t {
                litLength: 0,
                matchLength: 0,
                offset: 0,
            };
            /* Handle the initial state where litBuffer is split between dst and litExtraBuffer */
            while nb_seq != 0 {
                sequence = zstd_decode_sequence(&mut seq_state, is_long_offset, (nb_seq == 1) as c_int);
                if lit_ptr.add(sequence.litLength) > (*dctx).litBufferEnd {
                    break;
                }
                {
                    let one_seq_size = zstd_exec_sequence_split_lit_buffer(
                        op,
                        oend,
                        lit_ptr.add(sequence.litLength).offset(-(WILDCOPY_OVERLENGTH as isize)),
                        sequence,
                        &mut lit_ptr,
                        lit_buffer_end,
                        prefix_start,
                        v_base,
                        dict_end,
                    );
                    if err_is_error(one_seq_size) {
                        return one_seq_size;
                    }
                    op = op.add(one_seq_size);
                }
                nb_seq -= 1;
            }

            /* If there are more sequences, read literals from litExtraBuffer */
            if nb_seq > 0 {
                let leftover_lit = (*dctx).litBufferEnd as usize - lit_ptr as usize;
                if leftover_lit != 0 {
                    if leftover_lit > (oend as usize) - (op as usize) {
                        return err_code(ZSTD_error_dstSize_tooSmall);
                    }
                    zstd_safecopy_dst_before_src(op, lit_ptr, leftover_lit as isize);
                    sequence.litLength -= leftover_lit;
                    op = op.add(leftover_lit);
                }
                lit_ptr = (*dctx).litExtraBuffer.as_ptr();
                lit_buffer_end = (*dctx).litExtraBuffer.as_ptr().add(ZSTD_LITBUFFEREXTRASIZE);
                (*dctx).litBufferLocation = ZSTD_not_in_dst;
                {
                    let one_seq_size = zstd_exec_sequence(
                        op,
                        oend,
                        sequence,
                        &mut lit_ptr,
                        lit_buffer_end,
                        prefix_start,
                        v_base,
                        dict_end,
                    );
                    if err_is_error(one_seq_size) {
                        return one_seq_size;
                    }
                    op = op.add(one_seq_size);
                }
                nb_seq -= 1;
            }
        }

        if nb_seq > 0 {
            /* there is remaining lit from extra buffer */
            while nb_seq != 0 {
                let sequence =
                    zstd_decode_sequence(&mut seq_state, is_long_offset, (nb_seq == 1) as c_int);
                let one_seq_size = zstd_exec_sequence(
                    op,
                    oend,
                    sequence,
                    &mut lit_ptr,
                    lit_buffer_end,
                    prefix_start,
                    v_base,
                    dict_end,
                );
                if err_is_error(one_seq_size) {
                    return one_seq_size;
                }
                op = op.add(one_seq_size);
                nb_seq -= 1;
            }
        }

        /* check if reached exact end */
        if nb_seq != 0 {
            return err_code(ZSTD_error_corruption_detected);
        }
        if !bit_end_of_dstream(&seq_state.DStream) {
            return err_code(ZSTD_error_corruption_detected);
        }
        /* save reps for next block */
        for i in 0..ZSTD_REP_NUM {
            (*dctx).entropy.rep[i] = seq_state.prevOffset[i] as U32;
        }
    }

    /* last literal segment */
    if (*dctx).litBufferLocation == ZSTD_split {
        let last_ll_size = (lit_buffer_end as usize) - (lit_ptr as usize);
        if last_ll_size > (oend as usize) - (op as usize) {
            return err_code(ZSTD_error_dstSize_tooSmall);
        }
        if !op.is_null() {
            core::ptr::copy(lit_ptr, op, last_ll_size);
            op = op.add(last_ll_size);
        }
        lit_ptr = (*dctx).litExtraBuffer.as_ptr();
        lit_buffer_end = (*dctx).litExtraBuffer.as_ptr().add(ZSTD_LITBUFFEREXTRASIZE);
        (*dctx).litBufferLocation = ZSTD_not_in_dst;
    }
    /* copy last literals from internal buffer */
    {
        let last_ll_size = (lit_buffer_end as usize) - (lit_ptr as usize);
        if last_ll_size > (oend as usize) - (op as usize) {
            return err_code(ZSTD_error_dstSize_tooSmall);
        }
        if !op.is_null() {
            core::ptr::copy_nonoverlapping(lit_ptr, op, last_ll_size);
            op = op.add(last_ll_size);
        }
    }

    (op as usize) - (ostart as usize)
}

/// `ZSTD_decompressSequences_body()`
unsafe fn zstd_decompress_sequences_body(
    dctx: *mut ZSTD_DCtx,
    dst: *mut c_void,
    max_dst_size: usize,
    seq_start: *const c_void,
    seq_size: usize,
    mut nb_seq: c_int,
    is_long_offset: ZSTD_longOffset_e,
) -> usize {
    let ip = seq_start as *const u8;
    let iend = ip.add(seq_size);
    let ostart = dst as *mut u8;
    let oend = if (*dctx).litBufferLocation == ZSTD_not_in_dst {
        zstd_maybe_null_ptr_add(ostart, max_dst_size as isize)
    } else {
        (*dctx).litBuffer
    };
    let mut op = ostart;
    let mut lit_ptr = (*dctx).litPtr;
    let lit_end = lit_ptr.add((*dctx).litSize);
    let prefix_start = (*dctx).prefixStart as *const u8;
    let v_base = (*dctx).virtualStart as *const u8;
    let dict_end = (*dctx).dictEnd as *const u8;

    /* Regen sequences */
    if nb_seq != 0 {
        let mut seq_state: seqState_t = core::mem::zeroed();
        (*dctx).fseEntropy = 1;
        for i in 0..ZSTD_REP_NUM {
            seq_state.prevOffset[i] = (*dctx).entropy.rep[i] as usize;
        }
        if err_is_error(bit_init_dstream(
            &mut seq_state.DStream,
            ip,
            (iend as usize) - (ip as usize),
        )) {
            return err_code(ZSTD_error_corruption_detected);
        }
        zstd_init_fse_state(&mut seq_state.stateLL, &mut seq_state.DStream, (*dctx).LLTptr);
        zstd_init_fse_state(&mut seq_state.stateOffb, &mut seq_state.DStream, (*dctx).OFTptr);
        zstd_init_fse_state(&mut seq_state.stateML, &mut seq_state.DStream, (*dctx).MLTptr);

        while nb_seq != 0 {
            let sequence =
                zstd_decode_sequence(&mut seq_state, is_long_offset, (nb_seq == 1) as c_int);
            let one_seq_size = zstd_exec_sequence(
                op,
                oend,
                sequence,
                &mut lit_ptr,
                lit_end,
                prefix_start,
                v_base,
                dict_end,
            );
            if err_is_error(one_seq_size) {
                return one_seq_size;
            }
            op = op.add(one_seq_size);
            nb_seq -= 1;
        }

        /* check if reached exact end */
        if !bit_end_of_dstream(&seq_state.DStream) {
            return err_code(ZSTD_error_corruption_detected);
        }
        /* save reps for next block */
        for i in 0..ZSTD_REP_NUM {
            (*dctx).entropy.rep[i] = seq_state.prevOffset[i] as U32;
        }
    }

    /* last literal segment */
    {
        let last_ll_size = (lit_end as usize) - (lit_ptr as usize);
        if last_ll_size > (oend as usize) - (op as usize) {
            return err_code(ZSTD_error_dstSize_tooSmall);
        }
        if !op.is_null() {
            core::ptr::copy_nonoverlapping(lit_ptr, op, last_ll_size);
            op = op.add(last_ll_size);
        }
    }

    (op as usize) - (ostart as usize)
}

/// `ZSTD_decompressSequences_default()`
unsafe fn zstd_decompress_sequences_default(
    dctx: *mut ZSTD_DCtx,
    dst: *mut c_void,
    max_dst_size: usize,
    seq_start: *const c_void,
    seq_size: usize,
    nb_seq: c_int,
    is_long_offset: ZSTD_longOffset_e,
) -> usize {
    zstd_decompress_sequences_body(dctx, dst, max_dst_size, seq_start, seq_size, nb_seq, is_long_offset)
}

/// `ZSTD_decompressSequencesSplitLitBuffer_default()`
unsafe fn zstd_decompress_sequences_split_lit_buffer_default(
    dctx: *mut ZSTD_DCtx,
    dst: *mut c_void,
    max_dst_size: usize,
    seq_start: *const c_void,
    seq_size: usize,
    nb_seq: c_int,
    is_long_offset: ZSTD_longOffset_e,
) -> usize {
    zstd_decompress_sequences_body_split_lit_buffer(
        dctx, dst, max_dst_size, seq_start, seq_size, nb_seq, is_long_offset,
    )
}

/// `ZSTD_prefetchMatch()`
#[inline(always)]
unsafe fn zstd_prefetch_match(
    mut prefetch_pos: usize,
    sequence: seq_t,
    prefix_start: *const u8,
    dict_end: *const u8,
) -> usize {
    prefetch_pos += sequence.litLength;
    {
        let match_base = if sequence.offset > prefetch_pos {
            dict_end
        } else {
            prefix_start
        };
        let match_ = zstd_wrapped_ptr_sub(
            zstd_wrapped_ptr_add(match_base, prefetch_pos as isize),
            sequence.offset as isize,
        );
        prefetch_l1(match_);
        prefetch_l1(match_.wrapping_add(CACHELINE_SIZE));
    }
    prefetch_pos + sequence.matchLength
}

const STORED_SEQS: usize = 8;
const STORED_SEQS_MASK: usize = STORED_SEQS - 1;
const ADVANCED_SEQS: usize = STORED_SEQS;

/// `ZSTD_decompressSequencesLong_body()`
unsafe fn zstd_decompress_sequences_long_body(
    dctx: *mut ZSTD_DCtx,
    dst: *mut c_void,
    max_dst_size: usize,
    seq_start: *const c_void,
    seq_size: usize,
    nb_seq: c_int,
    is_long_offset: ZSTD_longOffset_e,
) -> usize {
    let ip = seq_start as *const u8;
    let iend = ip.add(seq_size);
    let ostart = dst as *mut u8;
    let oend = if (*dctx).litBufferLocation == ZSTD_in_dst {
        (*dctx).litBuffer
    } else {
        zstd_maybe_null_ptr_add(ostart, max_dst_size as isize)
    };
    let mut op = ostart;
    let mut lit_ptr = (*dctx).litPtr;
    let mut lit_buffer_end = (*dctx).litBufferEnd;
    let prefix_start = (*dctx).prefixStart as *const u8;
    let dict_start = (*dctx).virtualStart as *const u8;
    let dict_end = (*dctx).dictEnd as *const u8;

    /* Regen sequences */
    if nb_seq != 0 {
        let mut sequences: [seq_t; STORED_SEQS] = [seq_t {
            litLength: 0,
            matchLength: 0,
            offset: 0,
        }; STORED_SEQS];
        let seq_advance = min_usize(nb_seq as usize, ADVANCED_SEQS) as c_int;
        let mut seq_state: seqState_t = core::mem::zeroed();
        let mut seq_nb: c_int;
        let mut prefetch_pos: usize = (op as usize) - (prefix_start as usize);

        (*dctx).fseEntropy = 1;
        for i in 0..ZSTD_REP_NUM {
            seq_state.prevOffset[i] = (*dctx).entropy.rep[i] as usize;
        }
        if err_is_error(bit_init_dstream(
            &mut seq_state.DStream,
            ip,
            (iend as usize) - (ip as usize),
        )) {
            return err_code(ZSTD_error_corruption_detected);
        }
        zstd_init_fse_state(&mut seq_state.stateLL, &mut seq_state.DStream, (*dctx).LLTptr);
        zstd_init_fse_state(&mut seq_state.stateOffb, &mut seq_state.DStream, (*dctx).OFTptr);
        zstd_init_fse_state(&mut seq_state.stateML, &mut seq_state.DStream, (*dctx).MLTptr);

        /* prepare in advance */
        seq_nb = 0;
        while seq_nb < seq_advance {
            let sequence = zstd_decode_sequence(
                &mut seq_state,
                is_long_offset,
                (seq_nb == nb_seq - 1) as c_int,
            );
            prefetch_pos = zstd_prefetch_match(prefetch_pos, sequence, prefix_start, dict_end);
            sequences[seq_nb as usize] = sequence;
            seq_nb += 1;
        }

        /* decompress without stomping litBuffer */
        while seq_nb < nb_seq {
            let sequence = zstd_decode_sequence(
                &mut seq_state,
                is_long_offset,
                (seq_nb == nb_seq - 1) as c_int,
            );

            let idx = ((seq_nb - ADVANCED_SEQS as c_int) as usize) & STORED_SEQS_MASK;
            if (*dctx).litBufferLocation == ZSTD_split
                && lit_ptr.add(sequences[idx].litLength) > (*dctx).litBufferEnd
            {
                /* lit buffer reaching split point, transition to litExtraBuffer */
                let leftover_lit = (*dctx).litBufferEnd as usize - lit_ptr as usize;
                if leftover_lit != 0 {
                    if leftover_lit > (oend as usize) - (op as usize) {
                        return err_code(ZSTD_error_dstSize_tooSmall);
                    }
                    zstd_safecopy_dst_before_src(op, lit_ptr, leftover_lit as isize);
                    sequences[idx].litLength -= leftover_lit;
                    op = op.add(leftover_lit);
                }
                lit_ptr = (*dctx).litExtraBuffer.as_ptr();
                lit_buffer_end = (*dctx).litExtraBuffer.as_ptr().add(ZSTD_LITBUFFEREXTRASIZE);
                (*dctx).litBufferLocation = ZSTD_not_in_dst;
                {
                    let one_seq_size = zstd_exec_sequence(
                        op,
                        oend,
                        sequences[idx],
                        &mut lit_ptr,
                        lit_buffer_end,
                        prefix_start,
                        dict_start,
                        dict_end,
                    );
                    if err_is_error(one_seq_size) {
                        return one_seq_size;
                    }
                    prefetch_pos =
                        zstd_prefetch_match(prefetch_pos, sequence, prefix_start, dict_end);
                    sequences[(seq_nb as usize) & STORED_SEQS_MASK] = sequence;
                    op = op.add(one_seq_size);
                }
            } else {
                let one_seq_size = if (*dctx).litBufferLocation == ZSTD_split {
                    zstd_exec_sequence_split_lit_buffer(
                        op,
                        oend,
                        lit_ptr
                            .add(sequences[idx].litLength)
                            .offset(-(WILDCOPY_OVERLENGTH as isize)),
                        sequences[idx],
                        &mut lit_ptr,
                        lit_buffer_end,
                        prefix_start,
                        dict_start,
                        dict_end,
                    )
                } else {
                    zstd_exec_sequence(
                        op,
                        oend,
                        sequences[idx],
                        &mut lit_ptr,
                        lit_buffer_end,
                        prefix_start,
                        dict_start,
                        dict_end,
                    )
                };
                if err_is_error(one_seq_size) {
                    return one_seq_size;
                }
                prefetch_pos = zstd_prefetch_match(prefetch_pos, sequence, prefix_start, dict_end);
                sequences[(seq_nb as usize) & STORED_SEQS_MASK] = sequence;
                op = op.add(one_seq_size);
            }
            seq_nb += 1;
        }
        if !bit_end_of_dstream(&seq_state.DStream) {
            return err_code(ZSTD_error_corruption_detected);
        }

        /* finish queue */
        seq_nb -= seq_advance;
        while seq_nb < nb_seq {
            let idx = (seq_nb as usize) & STORED_SEQS_MASK;
            if (*dctx).litBufferLocation == ZSTD_split
                && lit_ptr.add(sequences[idx].litLength) > (*dctx).litBufferEnd
            {
                let leftover_lit = (*dctx).litBufferEnd as usize - lit_ptr as usize;
                if leftover_lit != 0 {
                    if leftover_lit > (oend as usize) - (op as usize) {
                        return err_code(ZSTD_error_dstSize_tooSmall);
                    }
                    zstd_safecopy_dst_before_src(op, lit_ptr, leftover_lit as isize);
                    sequences[idx].litLength -= leftover_lit;
                    op = op.add(leftover_lit);
                }
                lit_ptr = (*dctx).litExtraBuffer.as_ptr();
                lit_buffer_end = (*dctx).litExtraBuffer.as_ptr().add(ZSTD_LITBUFFEREXTRASIZE);
                (*dctx).litBufferLocation = ZSTD_not_in_dst;
                {
                    let one_seq_size = zstd_exec_sequence(
                        op,
                        oend,
                        sequences[idx],
                        &mut lit_ptr,
                        lit_buffer_end,
                        prefix_start,
                        dict_start,
                        dict_end,
                    );
                    if err_is_error(one_seq_size) {
                        return one_seq_size;
                    }
                    op = op.add(one_seq_size);
                }
            } else {
                let one_seq_size = if (*dctx).litBufferLocation == ZSTD_split {
                    zstd_exec_sequence_split_lit_buffer(
                        op,
                        oend,
                        lit_ptr
                            .add(sequences[idx].litLength)
                            .offset(-(WILDCOPY_OVERLENGTH as isize)),
                        sequences[idx],
                        &mut lit_ptr,
                        lit_buffer_end,
                        prefix_start,
                        dict_start,
                        dict_end,
                    )
                } else {
                    zstd_exec_sequence(
                        op,
                        oend,
                        sequences[idx],
                        &mut lit_ptr,
                        lit_buffer_end,
                        prefix_start,
                        dict_start,
                        dict_end,
                    )
                };
                if err_is_error(one_seq_size) {
                    return one_seq_size;
                }
                op = op.add(one_seq_size);
            }
            seq_nb += 1;
        }

        /* save reps for next block */
        for i in 0..ZSTD_REP_NUM {
            (*dctx).entropy.rep[i] = seq_state.prevOffset[i] as U32;
        }
    }

    /* last literal segment */
    if (*dctx).litBufferLocation == ZSTD_split {
        let last_ll_size = (lit_buffer_end as usize) - (lit_ptr as usize);
        if last_ll_size > (oend as usize) - (op as usize) {
            return err_code(ZSTD_error_dstSize_tooSmall);
        }
        if !op.is_null() {
            core::ptr::copy(lit_ptr, op, last_ll_size);
            op = op.add(last_ll_size);
        }
        lit_ptr = (*dctx).litExtraBuffer.as_ptr();
        lit_buffer_end = (*dctx).litExtraBuffer.as_ptr().add(ZSTD_LITBUFFEREXTRASIZE);
    }
    {
        let last_ll_size = (lit_buffer_end as usize) - (lit_ptr as usize);
        if last_ll_size > (oend as usize) - (op as usize) {
            return err_code(ZSTD_error_dstSize_tooSmall);
        }
        if !op.is_null() {
            core::ptr::copy(lit_ptr, op, last_ll_size);
            op = op.add(last_ll_size);
        }
    }

    (op as usize) - (ostart as usize)
}

/// `ZSTD_decompressSequencesLong_default()`
unsafe fn zstd_decompress_sequences_long_default(
    dctx: *mut ZSTD_DCtx,
    dst: *mut c_void,
    max_dst_size: usize,
    seq_start: *const c_void,
    seq_size: usize,
    nb_seq: c_int,
    is_long_offset: ZSTD_longOffset_e,
) -> usize {
    zstd_decompress_sequences_long_body(
        dctx, dst, max_dst_size, seq_start, seq_size, nb_seq, is_long_offset,
    )
}

/// `ZSTD_decompressSequences()`
unsafe fn zstd_decompress_sequences(
    dctx: *mut ZSTD_DCtx,
    dst: *mut c_void,
    max_dst_size: usize,
    seq_start: *const c_void,
    seq_size: usize,
    nb_seq: c_int,
    is_long_offset: ZSTD_longOffset_e,
) -> usize {
    zstd_decompress_sequences_default(dctx, dst, max_dst_size, seq_start, seq_size, nb_seq, is_long_offset)
}

/// `ZSTD_decompressSequencesSplitLitBuffer()`
unsafe fn zstd_decompress_sequences_split_lit_buffer(
    dctx: *mut ZSTD_DCtx,
    dst: *mut c_void,
    max_dst_size: usize,
    seq_start: *const c_void,
    seq_size: usize,
    nb_seq: c_int,
    is_long_offset: ZSTD_longOffset_e,
) -> usize {
    zstd_decompress_sequences_split_lit_buffer_default(
        dctx, dst, max_dst_size, seq_start, seq_size, nb_seq, is_long_offset,
    )
}

/// `ZSTD_decompressSequencesLong()`
unsafe fn zstd_decompress_sequences_long(
    dctx: *mut ZSTD_DCtx,
    dst: *mut c_void,
    max_dst_size: usize,
    seq_start: *const c_void,
    seq_size: usize,
    nb_seq: c_int,
    is_long_offset: ZSTD_longOffset_e,
) -> usize {
    zstd_decompress_sequences_long_default(
        dctx, dst, max_dst_size, seq_start, seq_size, nb_seq, is_long_offset,
    )
}

/// `ZSTD_totalHistorySize()`
unsafe fn zstd_total_history_size(op: *mut u8, virtual_start: *const u8) -> usize {
    (op as usize) - (virtual_start as usize)
}

/// `ZSTD_OffsetInfo`
#[derive(Clone, Copy)]
struct ZSTD_OffsetInfo {
    long_offset_share: c_uint,
    max_nb_additional_bits: c_uint,
}

/// `ZSTD_getOffsetInfo()`
unsafe fn zstd_get_offset_info(off_table: *const ZSTD_seqSymbol, nb_seq: c_int) -> ZSTD_OffsetInfo {
    let mut info = ZSTD_OffsetInfo {
        long_offset_share: 0,
        max_nb_additional_bits: 0,
    };
    if nb_seq != 0 {
        let ptr = off_table as *const ZSTD_seqSymbol_header;
        let table_log: U32 = (*ptr).tableLog;
        let table = off_table.add(1);
        let max: U32 = 1 << table_log;
        for u in 0..max {
            info.max_nb_additional_bits = max_u32(
                info.max_nb_additional_bits,
                (*table.add(u as usize)).nbAdditionalBits as c_uint,
            );
            if (*table.add(u as usize)).nbAdditionalBits > 22 {
                info.long_offset_share += 1;
            }
        }

        info.long_offset_share <<= OffFSELog - table_log;
    }

    info
}

/// `ZSTD_maxShortOffset()`
unsafe fn zstd_max_short_offset() -> usize {
    if mem_64bits() {
        (-1isize) as usize
    } else {
        let max_offbase: usize = (1usize << (STREAM_ACCUMULATOR_MIN_32 + 1)) - 1;
        let max_offset = max_offbase - ZSTD_REP_NUM;
        max_offset
    }
}

/// `ZSTD_decompressBlock_internal()`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_decompressBlock_internal(
    dctx: *mut ZSTD_DCtx,
    dst: *mut c_void,
    dst_capacity: usize,
    src: *const c_void,
    mut src_size: usize,
    streaming: streaming_operation,
) -> usize {
    /* blockType == blockCompressed */
    let mut ip = src as *const u8;

    if src_size > zstd_block_size_max(dctx) {
        return err_code(ZSTD_error_srcSize_wrong);
    }

    /* Decode literals section */
    {
        let lit_c_size =
            zstd_decode_literals_block(dctx, src, src_size, dst, dst_capacity, streaming);
        if err_is_error(lit_c_size) {
            return lit_c_size;
        }
        ip = ip.add(lit_c_size);
        src_size -= lit_c_size;
    }

    /* Build Decoding Tables */
    {
        let block_size_max = min_usize(dst_capacity, zstd_block_size_max(dctx));
        let total_history_size = zstd_total_history_size(
            zstd_maybe_null_ptr_add(dst as *mut u8, block_size_max as isize),
            (*dctx).virtualStart as *const u8,
        );
        let mut is_long_offset = if mem_32bits() && (total_history_size > zstd_max_short_offset()) {
            ZSTD_longOffset_e::ZSTD_lo_isLongOffset
        } else {
            ZSTD_longOffset_e::ZSTD_lo_isRegularOffset
        };
        let mut use_prefetch_decoder = (*dctx).ddictIsCold;
        let mut nb_seq: c_int = 0;
        let seq_h_size = ZSTD_decodeSeqHeaders(dctx, &mut nb_seq, ip as *const c_void, src_size);
        if err_is_error(seq_h_size) {
            return seq_h_size;
        }
        ip = ip.add(seq_h_size);
        src_size -= seq_h_size;

        if (dst.is_null() || dst_capacity == 0) && nb_seq > 0 {
            return err_code(ZSTD_error_dstSize_tooSmall);
        }
        if mem_64bits()
            && core::mem::size_of::<usize>() == core::mem::size_of::<*const c_void>()
            && ((-1isize) as usize) - (dst as usize) < (1usize << 20)
        {
            return err_code(ZSTD_error_dstSize_tooSmall);
        }

        if is_long_offset != ZSTD_longOffset_e::ZSTD_lo_isRegularOffset
            || (use_prefetch_decoder == 0 && (total_history_size > (1u32 << 24) as usize) && (nb_seq > 8))
        {
            let info = zstd_get_offset_info((*dctx).OFTptr, nb_seq);
            if is_long_offset != ZSTD_longOffset_e::ZSTD_lo_isRegularOffset
                && info.max_nb_additional_bits <= stream_accumulator_min()
            {
                is_long_offset = ZSTD_longOffset_e::ZSTD_lo_isRegularOffset;
            }
            if use_prefetch_decoder == 0 {
                let min_share: U32 = if mem_64bits() { 7 } else { 20 };
                use_prefetch_decoder = (info.long_offset_share >= min_share) as c_int;
            }
        }

        (*dctx).ddictIsCold = 0;

        if use_prefetch_decoder != 0 {
            return zstd_decompress_sequences_long(
                dctx,
                dst,
                dst_capacity,
                ip as *const c_void,
                src_size,
                nb_seq,
                is_long_offset,
            );
        }

        /* else */
        if (*dctx).litBufferLocation == ZSTD_split {
            zstd_decompress_sequences_split_lit_buffer(
                dctx,
                dst,
                dst_capacity,
                ip as *const c_void,
                src_size,
                nb_seq,
                is_long_offset,
            )
        } else {
            zstd_decompress_sequences(
                dctx,
                dst,
                dst_capacity,
                ip as *const c_void,
                src_size,
                nb_seq,
                is_long_offset,
            )
        }
    }
}

/// `ZSTD_checkContinuity()`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_checkContinuity(
    dctx: *mut ZSTD_DCtx,
    dst: *const c_void,
    dst_size: usize,
) {
    if dst != (*dctx).previousDstEnd && dst_size > 0 {
        /* not contiguous */
        (*dctx).dictEnd = (*dctx).previousDstEnd;
        (*dctx).virtualStart = (dst as *const u8).offset(
            -(((*dctx).previousDstEnd as isize) - ((*dctx).prefixStart as isize)),
        ) as *const c_void;
        (*dctx).prefixStart = dst;
        (*dctx).previousDstEnd = dst;
    }
}

/// `ZSTD_decompressBlock_deprecated()`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_decompressBlock_deprecated(
    dctx: *mut ZSTD_DCtx,
    dst: *mut c_void,
    dst_capacity: usize,
    src: *const c_void,
    src_size: usize,
) -> usize {
    (*dctx).isFrameDecompression = 0;
    ZSTD_checkContinuity(dctx, dst as *const c_void, dst_capacity);
    let d_size = ZSTD_decompressBlock_internal(
        dctx,
        dst,
        dst_capacity,
        src,
        src_size,
        streaming_operation::not_streaming,
    );
    if err_is_error(d_size) {
        return d_size;
    }
    (*dctx).previousDstEnd = (dst as *mut u8).add(d_size) as *const c_void;
    d_size
}

/// `ZSTD_decompressBlock()`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_decompressBlock(
    dctx: *mut ZSTD_DCtx,
    dst: *mut c_void,
    dst_capacity: usize,
    src: *const c_void,
    src_size: usize,
) -> usize {
    ZSTD_decompressBlock_deprecated(dctx, dst, dst_capacity, src, src_size)
}
