//! Translation of compress/zstd_compress_literals.c (+ compress/zstd_compress_literals.h)
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

use crate::error_private::*;
use crate::huf::*;
use crate::mem::*;
use crate::zstd_compress_internal::{ZSTD_hufCTables_t, ZSTD_minGain};
use crate::zstd_h::{ZSTD_lazy, ZSTD_strategy};
use crate::zstd_internal::{
    set_basic, set_compressed, set_repeat, set_rle, LitHufLog, SymbolEncodingType_e, MIN,
    MIN_LITERALS_FOR_4_STREAMS,
};

/* **************************************************************
 *  Literals compression - special cases
 ****************************************************************/

/// `size_t ZSTD_noCompressLiterals (void* dst, size_t dstCapacity, const void* src, size_t srcSize)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_noCompressLiterals(
    dst: *mut core::ffi::c_void,
    dstCapacity: usize,
    src: *const core::ffi::c_void,
    srcSize: usize,
) -> usize {
    let ostart: *mut BYTE = dst as *mut BYTE;
    let flSize: U32 = 1u32
        .wrapping_add((srcSize > 31) as U32)
        .wrapping_add((srcSize > 4095) as U32);

    if srcSize.wrapping_add(flSize as usize) > dstCapacity {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }

    match flSize {
        1 => {
            /* 2 - 1 - 5 */
            *ostart.add(0) = ((set_basic as U32 as usize).wrapping_add(srcSize << 3)) as BYTE;
        }
        2 => {
            /* 2 - 2 - 12 */
            MEM_writeLE16(
                ostart,
                ((set_basic as U32 as usize)
                    .wrapping_add(1 << 2)
                    .wrapping_add(srcSize << 4)) as U16,
            );
        }
        3 => {
            /* 2 - 2 - 20 */
            MEM_writeLE32(
                ostart,
                ((set_basic as U32 as usize)
                    .wrapping_add(3 << 2)
                    .wrapping_add(srcSize << 4)) as U32,
            );
        }
        _ => {
            /* not necessary : flSize is {1,2,3} */
        }
    }

    ZSTD_memcpy(ostart.add(flSize as usize), src as *const BYTE, srcSize);
    srcSize.wrapping_add(flSize as usize)
}

pub unsafe fn allBytesIdentical(src: *const core::ffi::c_void, srcSize: usize) -> core::ffi::c_int {
    {
        let b: BYTE = *(src as *const BYTE).add(0);
        let mut p: usize;
        p = 1;
        while p < srcSize {
            if *(src as *const BYTE).add(p) != b {
                return 0;
            }
            p = p.wrapping_add(1);
        }
        return 1;
    }
}

/// `size_t ZSTD_compressRleLiteralsBlock (void* dst, size_t dstCapacity, const void* src, size_t srcSize)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressRleLiteralsBlock(
    dst: *mut core::ffi::c_void,
    dstCapacity: usize,
    src: *const core::ffi::c_void,
    srcSize: usize,
) -> usize {
    let ostart: *mut BYTE = dst as *mut BYTE;
    let flSize: U32 = 1u32
        .wrapping_add((srcSize > 31) as U32)
        .wrapping_add((srcSize > 4095) as U32);

    match flSize {
        1 => {
            /* 2 - 1 - 5 */
            *ostart.add(0) = ((set_rle as U32 as usize).wrapping_add(srcSize << 3)) as BYTE;
        }
        2 => {
            /* 2 - 2 - 12 */
            MEM_writeLE16(
                ostart,
                ((set_rle as U32 as usize)
                    .wrapping_add(1 << 2)
                    .wrapping_add(srcSize << 4)) as U16,
            );
        }
        3 => {
            /* 2 - 2 - 20 */
            MEM_writeLE32(
                ostart,
                ((set_rle as U32 as usize)
                    .wrapping_add(3 << 2)
                    .wrapping_add(srcSize << 4)) as U32,
            );
        }
        _ => {
            /* not necessary : flSize is {1,2,3} */
        }
    }

    *ostart.add(flSize as usize) = *(src as *const BYTE);
    (flSize as usize).wrapping_add(1)
}

/* ZSTD_minLiteralsToCompress() :
 * returns minimal amount of literals
 * for literal compression to even be attempted.
 * Minimum is made tighter as compression strategy increases.
 */
pub fn ZSTD_minLiteralsToCompress(strategy: ZSTD_strategy, huf_repeat: HUF_repeat) -> usize {
    /* btultra2 : min 8 bytes;
     * then 2x larger for each successive compression strategy
     * max threshold 64 bytes */
    {
        let shift: core::ffi::c_int = MIN(9 - (strategy as core::ffi::c_int), 3);
        let mintc: usize = if huf_repeat == HUF_repeat_valid {
            6
        } else {
            8usize << shift
        };
        return mintc;
    }
}

/// ```c
/// size_t ZSTD_compressLiterals (void* dst, size_t dstCapacity,
///                         const void* src, size_t srcSize,
///                               void* entropyWorkspace, size_t entropyWorkspaceSize,
///                         const ZSTD_hufCTables_t* prevHuf,
///                               ZSTD_hufCTables_t* nextHuf,
///                               ZSTD_strategy strategy, int disableLiteralCompression,
///                               int suspectUncompressible,
///                               int bmi2);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressLiterals(
    dst: *mut core::ffi::c_void,
    dstCapacity: usize,
    src: *const core::ffi::c_void,
    srcSize: usize,
    entropyWorkspace: *mut core::ffi::c_void,
    entropyWorkspaceSize: usize,
    prevHuf: *const ZSTD_hufCTables_t,
    nextHuf: *mut ZSTD_hufCTables_t,
    strategy: ZSTD_strategy,
    disableLiteralCompression: core::ffi::c_int,
    suspectUncompressible: core::ffi::c_int,
    bmi2: core::ffi::c_int,
) -> usize {
    let lhSize: usize = 3usize
        .wrapping_add((srcSize >= 1 * 1024) as usize)
        .wrapping_add((srcSize >= 16 * 1024) as usize);
    let ostart: *mut BYTE = dst as *mut BYTE;
    let mut singleStream: U32 = (srcSize < 256) as U32;
    let mut hType: SymbolEncodingType_e = set_compressed;
    let cLitSize: usize;

    /* Prepare nextEntropy assuming reusing the existing table */
    ZSTD_memcpy(
        nextHuf as *mut BYTE,
        prevHuf as *const BYTE,
        core::mem::size_of::<ZSTD_hufCTables_t>(),
    );

    if disableLiteralCompression != 0 {
        return ZSTD_noCompressLiterals(dst, dstCapacity, src, srcSize);
    }

    /* if too small, don't even attempt compression (speed opt) */
    if srcSize < ZSTD_minLiteralsToCompress(strategy, (*prevHuf).repeatMode) {
        return ZSTD_noCompressLiterals(dst, dstCapacity, src, srcSize);
    }

    if dstCapacity < lhSize.wrapping_add(1) {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    {
        let mut repeat: HUF_repeat = (*prevHuf).repeatMode;
        let flags: core::ffi::c_int = 0
            | (if bmi2 != 0 { HUF_flags_bmi2 } else { 0 })
            | (if strategy < ZSTD_lazy && srcSize <= 1024 {
                HUF_flags_preferRepeat
            } else {
                0
            })
            | (if strategy >= HUF_OPTIMAL_DEPTH_THRESHOLD {
                HUF_flags_optimalDepth
            } else {
                0
            })
            | (if suspectUncompressible != 0 {
                HUF_flags_suspectUncompressible
            } else {
                0
            });

        type huf_compress_f = unsafe extern "C" fn(
            *mut core::ffi::c_void,
            usize,
            *const core::ffi::c_void,
            usize,
            core::ffi::c_uint,
            core::ffi::c_uint,
            *mut core::ffi::c_void,
            usize,
            *mut HUF_CElt,
            *mut HUF_repeat,
            core::ffi::c_int,
        ) -> usize;
        let huf_compress: huf_compress_f;
        if repeat == HUF_repeat_valid && lhSize == 3 {
            singleStream = 1;
        }
        huf_compress = if singleStream != 0 {
            crate::huf_compress::HUF_compress1X_repeat as huf_compress_f
        } else {
            crate::huf_compress::HUF_compress4X_repeat as huf_compress_f
        };
        cLitSize = huf_compress(
            ostart.add(lhSize) as *mut core::ffi::c_void,
            dstCapacity.wrapping_sub(lhSize),
            src,
            srcSize,
            HUF_SYMBOLVALUE_MAX,
            LitHufLog,
            entropyWorkspace,
            entropyWorkspaceSize,
            core::ptr::addr_of_mut!((*nextHuf).CTable) as *mut HUF_CElt,
            &mut repeat as *mut HUF_repeat,
            flags,
        );
        if repeat != HUF_repeat_none {
            /* reused the existing table */
            hType = set_repeat;
        }
    }

    {
        let minGain: usize = ZSTD_minGain(srcSize, strategy);
        if (cLitSize == 0)
            || (cLitSize >= srcSize.wrapping_sub(minGain))
            || (ERR_isError(cLitSize) != 0)
        {
            ZSTD_memcpy(
                nextHuf as *mut BYTE,
                prevHuf as *const BYTE,
                core::mem::size_of::<ZSTD_hufCTables_t>(),
            );
            return ZSTD_noCompressLiterals(dst, dstCapacity, src, srcSize);
        }
    }
    if cLitSize == 1 {
        /* A return value of 1 signals that the alphabet consists of a single symbol.
         * However, in some rare circumstances, it could be the compressed size (a single byte).
         * For that outcome to have a chance to happen, it's necessary that `srcSize < 8`.
         * (it's also necessary to not generate statistics).
         * Therefore, in such a case, actively check that all bytes are identical. */
        if (srcSize >= 8) || (allBytesIdentical(src, srcSize) != 0) {
            ZSTD_memcpy(
                nextHuf as *mut BYTE,
                prevHuf as *const BYTE,
                core::mem::size_of::<ZSTD_hufCTables_t>(),
            );
            return ZSTD_compressRleLiteralsBlock(dst, dstCapacity, src, srcSize);
        }
    }

    if hType == set_compressed {
        /* using a newly constructed table */
        (*nextHuf).repeatMode = HUF_repeat_check;
    }

    /* Build header */
    match lhSize {
        3 => {
            /* 2 - 2 - 10 - 10 */
            {
                let lhc: U32 = (hType as U32)
                    .wrapping_add(((singleStream == 0) as U32) << 2)
                    .wrapping_add((srcSize as U32) << 4)
                    .wrapping_add((cLitSize as U32) << 14);
                MEM_writeLE24(ostart, lhc);
            }
        }
        4 => {
            /* 2 - 2 - 14 - 14 */
            {
                let lhc: U32 = (hType as U32)
                    .wrapping_add(2 << 2)
                    .wrapping_add((srcSize as U32) << 4)
                    .wrapping_add((cLitSize as U32) << 18);
                MEM_writeLE32(ostart, lhc);
            }
        }
        5 => {
            /* 2 - 2 - 18 - 18 */
            {
                let lhc: U32 = (hType as U32)
                    .wrapping_add(3 << 2)
                    .wrapping_add((srcSize as U32) << 4)
                    .wrapping_add((cLitSize as U32) << 22);
                MEM_writeLE32(ostart, lhc);
                *ostart.add(4) = (cLitSize >> 10) as BYTE;
            }
        }
        _ => {
            /* not possible : lhSize is {3,4,5} */
        }
    }
    lhSize.wrapping_add(cLitSize)
}
