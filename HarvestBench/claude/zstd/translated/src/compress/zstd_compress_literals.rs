//! Translation of compress/zstd_compress_literals.c

use core::ffi::c_void;

use crate::common::error;
use crate::common::huf_common::{
    HUF_flags_bmi2, HUF_flags_optimalDepth, HUF_flags_preferRepeat, HUF_flags_suspectUncompressible,
    HUF_SYMBOLVALUE_MAX,
};
use crate::common::mem::{mem_write_le16, mem_write_le24, mem_write_le32};
use crate::common::zstd_internal::{
    set_basic, set_compressed, set_repeat, set_rle, LitHufLog, MIN_LITERALS_FOR_4_STREAMS,
};
use crate::compress::huf_compress::{HUF_compress1X_repeat, HUF_compress4X_repeat};
use crate::compress::zstd_compress_internal::{
    ZSTD_minGain, ZSTD_hufCTables_t, HUF_CElt, HUF_repeat, HUF_repeat_check, HUF_repeat_none,
    HUF_repeat_valid,
};
use crate::zstd_h::{ZSTD_btultra, ZSTD_lazy, ZSTD_strategy};

/* HUF_OPTIMAL_DEPTH_THRESHOLD == ZSTD_btultra */
const HUF_OPTIMAL_DEPTH_THRESHOLD: ZSTD_strategy = ZSTD_btultra;

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
    let ostart = dst as *mut u8;
    let flSize: u32 = 1 + (srcSize > 31) as u32 + (srcSize > 4095) as u32;

    if srcSize + flSize as usize > dstCapacity {
        return error::error(error::code::DSTSIZE_TOOSMALL);
    }

    match flSize {
        1 => {
            /* 2 - 1 - 5 */
            *ostart.add(0) = ((set_basic) as usize + (srcSize << 3)) as u8;
        }
        2 => {
            /* 2 - 2 - 12 */
            mem_write_le16(
                ostart as *mut c_void,
                ((set_basic) as usize + (1 << 2) + (srcSize << 4)) as u16,
            );
        }
        3 => {
            /* 2 - 2 - 20 */
            mem_write_le32(
                ostart as *mut c_void,
                ((set_basic) as usize + (3 << 2) + (srcSize << 4)) as u32,
            );
        }
        _ => {
            /* not necessary : flSize is {1,2,3} */
            debug_assert!(false);
        }
    }

    core::ptr::copy_nonoverlapping(src as *const u8, ostart.add(flSize as usize), srcSize);
    srcSize + flSize as usize
}

unsafe fn allBytesIdentical(src: *const c_void, srcSize: usize) -> i32 {
    debug_assert!(srcSize >= 1);
    debug_assert!(!src.is_null());
    {
        let b = *(src as *const u8);
        let mut p: usize = 1;
        while p < srcSize {
            if *(src as *const u8).add(p) != b {
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
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let ostart = dst as *mut u8;
    let flSize: u32 = 1 + (srcSize > 31) as u32 + (srcSize > 4095) as u32;

    debug_assert!(dstCapacity >= 4);
    let _ = dstCapacity;
    debug_assert!(allBytesIdentical(src, srcSize) != 0);

    match flSize {
        1 => {
            /* 2 - 1 - 5 */
            *ostart.add(0) = ((set_rle) as usize + (srcSize << 3)) as u8;
        }
        2 => {
            /* 2 - 2 - 12 */
            mem_write_le16(
                ostart as *mut c_void,
                ((set_rle) as usize + (1 << 2) + (srcSize << 4)) as u16,
            );
        }
        3 => {
            /* 2 - 2 - 20 */
            mem_write_le32(
                ostart as *mut c_void,
                ((set_rle) as usize + (3 << 2) + (srcSize << 4)) as u32,
            );
        }
        _ => {
            /* not necessary : flSize is {1,2,3} */
            debug_assert!(false);
        }
    }

    *ostart.add(flSize as usize) = *(src as *const u8);
    flSize as usize + 1
}

/* ZSTD_minLiteralsToCompress() :
 * returns minimal amount of literals
 * for literal compression to even be attempted.
 * Minimum is made tighter as compression strategy increases.
 */
unsafe fn ZSTD_minLiteralsToCompress(strategy: ZSTD_strategy, huf_repeat: HUF_repeat) -> usize {
    debug_assert!((strategy as i32) >= 0);
    debug_assert!((strategy as i32) <= 9);
    /* btultra2 : min 8 bytes;
     * then 2x larger for each successive compression strategy
     * max threshold 64 bytes */
    {
        let shift: i32 = core::cmp::min(9 - (strategy as i32), 3);
        let mintc: usize = if huf_repeat == HUF_repeat_valid {
            6
        } else {
            (8usize) << shift
        };
        mintc
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
    disableLiteralCompression: i32,
    suspectUncompressible: i32,
    bmi2: i32,
) -> usize {
    let lhSize: usize = 3 + (srcSize >= 1024) as usize + (srcSize >= 16384) as usize;
    let ostart = dst as *mut u8;
    let mut singleStream: u32 = (srcSize < 256) as u32;
    let mut hType: u32 = set_compressed;
    let cLitSize: usize;

    /* Prepare nextEntropy assuming reusing the existing table */
    core::ptr::copy_nonoverlapping(prevHuf, nextHuf, 1);

    if disableLiteralCompression != 0 {
        return ZSTD_noCompressLiterals(dst, dstCapacity, src, srcSize);
    }

    /* if too small, don't even attempt compression (speed opt) */
    if srcSize < ZSTD_minLiteralsToCompress(strategy, (*prevHuf).repeatMode) {
        return ZSTD_noCompressLiterals(dst, dstCapacity, src, srcSize);
    }

    if dstCapacity < lhSize + 1 {
        return error::error(error::code::DSTSIZE_TOOSMALL);
    }
    {
        let mut repeat: HUF_repeat = (*prevHuf).repeatMode;
        let flags: i32 = 0
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

        if repeat == HUF_repeat_valid && lhSize == 3 {
            singleStream = 1;
        }
        let huf_compress: unsafe extern "C" fn(
            *mut c_void,
            usize,
            *const c_void,
            usize,
            u32,
            u32,
            *mut c_void,
            usize,
            *mut HUF_CElt,
            *mut HUF_repeat,
            i32,
        ) -> usize = if singleStream != 0 {
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
        if (cLitSize == 0) || (cLitSize >= srcSize - minGain) || error::err_is_error(cLitSize) != 0
        {
            core::ptr::copy_nonoverlapping(prevHuf, nextHuf, 1);
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
            core::ptr::copy_nonoverlapping(prevHuf, nextHuf, 1);
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
            if singleStream == 0 {
                debug_assert!(srcSize >= MIN_LITERALS_FOR_4_STREAMS);
            }
            {
                let lhc: u32 = hType
                    + (((singleStream == 0) as u32) << 2)
                    + ((srcSize as u32) << 4)
                    + ((cLitSize as u32) << 14);
                mem_write_le24(ostart as *mut c_void, lhc);
            }
        }
        4 => {
            /* 2 - 2 - 14 - 14 */
            debug_assert!(srcSize >= MIN_LITERALS_FOR_4_STREAMS);
            {
                let lhc: u32 =
                    hType + (2 << 2) + ((srcSize as u32) << 4) + ((cLitSize as u32) << 18);
                mem_write_le32(ostart as *mut c_void, lhc);
            }
        }
        5 => {
            /* 2 - 2 - 18 - 18 */
            debug_assert!(srcSize >= MIN_LITERALS_FOR_4_STREAMS);
            {
                let lhc: u32 =
                    hType + (3 << 2) + ((srcSize as u32) << 4) + ((cLitSize as u32) << 22);
                mem_write_le32(ostart as *mut c_void, lhc);
                *ostart.add(4) = (cLitSize >> 10) as u8;
            }
        }
        _ => {
            /* not possible : lhSize is {3,4,5} */
            debug_assert!(false);
        }
    }
    lhSize + cLitSize
}
