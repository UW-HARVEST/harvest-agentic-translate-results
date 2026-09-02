//! Translation of `compress/zstd_compress_literals.c` (literals compression).
#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]

use crate::common::error_private::*;
use crate::common::huf::*;
use crate::common::mem::*;
use crate::common::zstd_h::*;
use crate::common::zstd_internal::*;
use crate::compress::huf_compress::*;
use crate::compress::zstd_compress_internal::*;

use core::ffi::{c_int, c_uint, c_void};

/* **************************************************************
*  Literals compression - special cases
****************************************************************/
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_noCompressLiterals(
    dst: *mut c_void,
    dstCapacity: size_t,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    let ostart: *mut BYTE = dst as *mut BYTE;
    let flSize: U32 = 1 + (srcSize > 31) as U32 + (srcSize > 4095) as U32;

    if srcSize + flSize as size_t > dstCapacity {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }

    match flSize {
        1 => {
            /* 2 - 1 - 5 */
            *ostart.add(0) = ((set_basic as U32) + ((srcSize as U32) << 3)) as BYTE;
        }
        2 => {
            /* 2 - 2 - 12 */
            MEM_writeLE16(
                ostart,
                ((set_basic as U32) + (1 << 2) + ((srcSize as U32) << 4)) as U16,
            );
        }
        3 => {
            /* 2 - 2 - 20 */
            MEM_writeLE32(
                ostart,
                (set_basic as U32) + (3 << 2) + ((srcSize as U32) << 4),
            );
        }
        _ => {
            /* not necessary : flSize is {1,2,3} */
        }
    }

    ZSTD_memcpy(ostart.add(flSize as usize), src as *const BYTE, srcSize);
    srcSize + flSize as size_t
}

unsafe fn allBytesIdentical(src: *const c_void, srcSize: size_t) -> c_int {
    {
        let b: BYTE = *(src as *const BYTE).add(0);
        let mut p: size_t = 1;
        while p < srcSize {
            if *(src as *const BYTE).add(p) != b {
                return 0;
            }
            p += 1;
        }
        1
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressRleLiteralsBlock(
    dst: *mut c_void,
    dstCapacity: size_t,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    let ostart: *mut BYTE = dst as *mut BYTE;
    let flSize: U32 = 1 + (srcSize > 31) as U32 + (srcSize > 4095) as U32;

    let _ = dstCapacity;

    match flSize {
        1 => {
            /* 2 - 1 - 5 */
            *ostart.add(0) = ((set_rle as U32) + ((srcSize as U32) << 3)) as BYTE;
        }
        2 => {
            /* 2 - 2 - 12 */
            MEM_writeLE16(
                ostart,
                ((set_rle as U32) + (1 << 2) + ((srcSize as U32) << 4)) as U16,
            );
        }
        3 => {
            /* 2 - 2 - 20 */
            MEM_writeLE32(
                ostart,
                (set_rle as U32) + (3 << 2) + ((srcSize as U32) << 4),
            );
        }
        _ => {
            /* not necessary : flSize is {1,2,3} */
        }
    }

    *ostart.add(flSize as usize) = *(src as *const BYTE);
    flSize as size_t + 1
}

/* ZSTD_minLiteralsToCompress() :
 * returns minimal amount of literals
 * for literal compression to even be attempted.
 * Minimum is made tighter as compression strategy increases.
 */
unsafe fn ZSTD_minLiteralsToCompress(strategy: ZSTD_strategy, huf_repeat: HUF_repeat) -> size_t {
    /* btultra2 : min 8 bytes;
     * then 2x larger for each successive compression strategy
     * max threshold 64 bytes */
    {
        let shift: c_int = MIN(9 - (strategy as c_int), 3);
        let mintc: size_t = if huf_repeat == HUF_repeat_valid {
            6
        } else {
            (8 as size_t) << shift
        };
        mintc
    }
}

type huf_compress_f = unsafe extern "C" fn(
    *mut c_void,
    size_t,
    *const c_void,
    size_t,
    c_uint,
    c_uint,
    *mut c_void,
    size_t,
    *mut HUF_CElt,
    *mut HUF_repeat,
    c_int,
) -> size_t;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressLiterals(
    dst: *mut c_void,
    dstCapacity: size_t,
    src: *const c_void,
    srcSize: size_t,
    entropyWorkspace: *mut c_void,
    entropyWorkspaceSize: size_t,
    prevHuf: *const ZSTD_hufCTables_t,
    nextHuf: *mut ZSTD_hufCTables_t,
    strategy: ZSTD_strategy,
    disableLiteralCompression: c_int,
    suspectUncompressible: c_int,
    bmi2: c_int,
) -> size_t {
    let lhSize: size_t =
        3 + (srcSize >= (1 << 10)) as size_t + (srcSize >= (16 << 10)) as size_t;
    let ostart: *mut BYTE = dst as *mut BYTE;
    let mut singleStream: U32 = (srcSize < 256) as U32;
    let mut hType: SymbolEncodingType_e = set_compressed;
    let cLitSize: size_t;

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

    if dstCapacity < lhSize + 1 {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    {
        let mut repeat: HUF_repeat = (*prevHuf).repeatMode;
        let flags: c_int = 0
            | (if bmi2 != 0 { HUF_flags_bmi2 as c_int } else { 0 })
            | (if strategy < ZSTD_lazy && srcSize <= 1024 {
                HUF_flags_preferRepeat as c_int
            } else {
                0
            })
            | (if strategy >= HUF_OPTIMAL_DEPTH_THRESHOLD {
                HUF_flags_optimalDepth as c_int
            } else {
                0
            })
            | (if suspectUncompressible != 0 {
                HUF_flags_suspectUncompressible as c_int
            } else {
                0
            });

        let huf_compress: huf_compress_f;
        if repeat == HUF_repeat_valid && lhSize == 3 {
            singleStream = 1;
        }
        huf_compress = if singleStream != 0 {
            HUF_compress1X_repeat
        } else {
            HUF_compress4X_repeat
        };
        cLitSize = huf_compress(
            ostart.add(lhSize) as *mut c_void,
            dstCapacity - lhSize,
            src,
            srcSize,
            HUF_SYMBOLVALUE_MAX,
            LitHufLog as c_uint,
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
        let minGain: size_t = ZSTD_minGain(srcSize, strategy);
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
            let lhc: U32 = hType
                + (((singleStream == 0) as U32) << 2)
                + ((srcSize as U32) << 4)
                + ((cLitSize as U32) << 14);
            MEM_writeLE24(ostart, lhc);
        }
        4 => {
            /* 2 - 2 - 14 - 14 */
            let lhc: U32 =
                hType + (2 << 2) + ((srcSize as U32) << 4) + ((cLitSize as U32) << 18);
            MEM_writeLE32(ostart, lhc);
        }
        5 => {
            /* 2 - 2 - 18 - 18 */
            let lhc: U32 =
                hType + (3 << 2) + ((srcSize as U32) << 4) + ((cLitSize as U32) << 22);
            MEM_writeLE32(ostart, lhc);
            *ostart.add(4) = (cLitSize >> 10) as BYTE;
        }
        _ => {
            /* not possible : lhSize is {3,4,5} */
        }
    }
    lhSize + cLitSize
}
