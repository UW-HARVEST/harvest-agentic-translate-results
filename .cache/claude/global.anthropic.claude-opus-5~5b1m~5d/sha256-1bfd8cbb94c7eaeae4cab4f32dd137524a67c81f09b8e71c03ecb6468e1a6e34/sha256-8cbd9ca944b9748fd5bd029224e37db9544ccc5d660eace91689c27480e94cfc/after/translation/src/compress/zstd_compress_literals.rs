//! Translation of `compress/zstd_compress_literals.c`
#![allow(dead_code)]

use core::ffi::{c_int, c_uint, c_void};

use crate::cmem::*;
use crate::compress::zstd_compress_internal::*;
use crate::error_private::*;
use crate::huf::*;
use crate::zstd_h::*;
use crate::zstd_internal::*;

/// `common/huf.h`: `#define HUF_OPTIMAL_DEPTH_THRESHOLD ZSTD_btultra`
const HUF_OPTIMAL_DEPTH_THRESHOLD: ZSTD_strategy = ZSTD_btultra;

/* **************************************************************
*  Debug Traces
****************************************************************/
/* `showHexa()` is only compiled for DEBUGLEVEL >= 2; DEBUGLEVEL is 0 here. */

/* **************************************************************
*  Literals compression - special cases
****************************************************************/
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_noCompressLiterals(
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let ostart = dst as *mut BYTE;
    let flSize: U32 = 1 + (srcSize > 31) as U32 + (srcSize > 4095) as U32;

    if srcSize + flSize as usize > dstCapacity {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }

    match flSize {
        1 => {
            /* 2 - 1 - 5 */
            *ostart = ((set_basic as U32 as usize).wrapping_add(srcSize << 3)) as BYTE;
        }
        2 => {
            /* 2 - 2 - 12 */
            MEM_writeLE16(
                ostart as *mut c_void,
                ((set_basic as U32 as usize)
                    .wrapping_add(1 << 2)
                    .wrapping_add(srcSize << 4)) as U16,
            );
        }
        3 => {
            /* 2 - 2 - 20 */
            MEM_writeLE32(
                ostart as *mut c_void,
                ((set_basic as U32 as usize)
                    .wrapping_add(3 << 2)
                    .wrapping_add(srcSize << 4)) as U32,
            );
        }
        _ => { /* not necessary : flSize is {1,2,3} */ }
    }

    ZSTD_memcpy(ostart.add(flSize as usize) as *mut c_void, src, srcSize);
    srcSize + flSize as usize
}

pub(crate) unsafe fn allBytesIdentical(src: *const c_void, srcSize: usize) -> c_int {
    {
        let b: BYTE = *(src as *const BYTE);
        let mut p: usize = 1;
        while p < srcSize {
            if *(src as *const BYTE).add(p) != b {
                return 0;
            }
            p += 1;
        }
        return 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressRleLiteralsBlock(
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let ostart = dst as *mut BYTE;
    let flSize: U32 = 1 + (srcSize > 31) as U32 + (srcSize > 4095) as U32;

    match flSize {
        1 => {
            /* 2 - 1 - 5 */
            *ostart = ((set_rle as U32 as usize).wrapping_add(srcSize << 3)) as BYTE;
        }
        2 => {
            /* 2 - 2 - 12 */
            MEM_writeLE16(
                ostart as *mut c_void,
                ((set_rle as U32 as usize)
                    .wrapping_add(1 << 2)
                    .wrapping_add(srcSize << 4)) as U16,
            );
        }
        3 => {
            /* 2 - 2 - 20 */
            MEM_writeLE32(
                ostart as *mut c_void,
                ((set_rle as U32 as usize)
                    .wrapping_add(3 << 2)
                    .wrapping_add(srcSize << 4)) as U32,
            );
        }
        _ => { /* not necessary : flSize is {1,2,3} */ }
    }

    *ostart.add(flSize as usize) = *(src as *const BYTE);
    flSize as usize + 1
}

/* ZSTD_minLiteralsToCompress() :
 * returns minimal amount of literals
 * for literal compression to even be attempted.
 * Minimum is made tighter as compression strategy increases.
 */
pub(crate) unsafe fn ZSTD_minLiteralsToCompress(
    strategy: ZSTD_strategy,
    huf_repeat: HUF_repeat,
) -> usize {
    /* btultra2 : min 8 bytes;
     * then 2x larger for each successive compression strategy
     * max threshold 64 bytes */
    {
        let a: c_int = 9 - strategy as c_int;
        let shift: c_int = if a < 3 { a } else { 3 };
        let mintc: usize = if huf_repeat == HUF_repeat_valid {
            6
        } else {
            8usize << shift
        };
        return mintc;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressLiterals(
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
    entropyWorkspace: *mut c_void,
    entropyWorkspaceSize: usize,
    prevHuf: *const ZSTD_hufCTables_t,
    nextHuf: *mut ZSTD_hufCTables_t,
    strategy: ZSTD_strategy,
    disableLiteralCompression: c_int,
    suspectUncompressible: c_int,
    bmi2: c_int,
) -> usize {
    let lhSize: usize =
        3 + (srcSize >= (1 * (1 << 10))) as usize + (srcSize >= (16 * (1 << 10))) as usize;
    let ostart = dst as *mut BYTE;
    let mut singleStream: U32 = (srcSize < 256) as U32;
    let mut hType: SymbolEncodingType_e = set_compressed;
    let mut cLitSize: usize = 0;

    /* Prepare nextEntropy assuming reusing the existing table */
    ZSTD_memcpy(
        nextHuf as *mut c_void,
        prevHuf as *const c_void,
        core::mem::size_of::<ZSTD_hufCTables_t>(),
    );

    if disableLiteralCompression != 0 {
        return ZSTD_noCompressLiterals(dst, dstCapacity, src, srcSize);
    }

    /* if too small, don't even attempt compression (speed opt) */
    if srcSize < ZSTD_minLiteralsToCompress(strategy, (*prevHuf).repeatMode) {
        return ZSTD_noCompressLiterals(dst, dstCapacity, src, srcSize);
    }

    if dstCapacity < lhSize + 1 {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    {
        let mut repeat: HUF_repeat = (*prevHuf).repeatMode;
        let flags: c_int = 0
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
            *mut c_void,
            usize,
            *const c_void,
            usize,
            c_uint,
            c_uint,
            *mut c_void,
            usize,
            *mut HUF_CElt,
            *mut HUF_repeat,
            c_int,
        ) -> usize;
        let huf_compress: huf_compress_f;
        if repeat == HUF_repeat_valid && lhSize == 3 {
            singleStream = 1;
        }
        huf_compress = if singleStream != 0 {
            crate::compress::huf_compress::HUF_compress1X_repeat
        } else {
            crate::compress::huf_compress::HUF_compress4X_repeat
        };
        cLitSize = huf_compress(
            ostart.add(lhSize) as *mut c_void,
            dstCapacity - lhSize,
            src,
            srcSize,
            HUF_SYMBOLVALUE_MAX,
            LitHufLog,
            entropyWorkspace,
            entropyWorkspaceSize,
            (*nextHuf).CTable.as_mut_ptr() as *mut HUF_CElt,
            &mut repeat,
            flags,
        );
        if repeat != HUF_repeat_none {
            /* reused the existing table */
            hType = set_repeat;
        }
    }

    {
        let minGain: usize = ZSTD_minGain(srcSize, strategy);
        if (cLitSize == 0) || (cLitSize >= srcSize.wrapping_sub(minGain)) || ERR_isError(cLitSize) != 0
        {
            ZSTD_memcpy(
                nextHuf as *mut c_void,
                prevHuf as *const c_void,
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
        if (srcSize >= 8) || allBytesIdentical(src, srcSize) != 0 {
            ZSTD_memcpy(
                nextHuf as *mut c_void,
                prevHuf as *const c_void,
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
            let lhc: U32 = hType
                .wrapping_add(((singleStream == 0) as U32) << 2)
                .wrapping_add((srcSize as U32) << 4)
                .wrapping_add((cLitSize as U32) << 14);
            MEM_writeLE24(ostart as *mut c_void, lhc);
        }
        4 => {
            /* 2 - 2 - 14 - 14 */
            let lhc: U32 = hType
                .wrapping_add(2 << 2)
                .wrapping_add((srcSize as U32) << 4)
                .wrapping_add((cLitSize as U32) << 18);
            MEM_writeLE32(ostart as *mut c_void, lhc);
        }
        5 => {
            /* 2 - 2 - 18 - 18 */
            let lhc: U32 = hType
                .wrapping_add(3 << 2)
                .wrapping_add((srcSize as U32) << 4)
                .wrapping_add((cLitSize as U32) << 22);
            MEM_writeLE32(ostart as *mut c_void, lhc);
            *ostart.add(4) = (cLitSize >> 10) as BYTE;
        }
        _ => { /* not possible : lhSize is {3,4,5} */ }
    }
    lhSize + cLitSize
}
