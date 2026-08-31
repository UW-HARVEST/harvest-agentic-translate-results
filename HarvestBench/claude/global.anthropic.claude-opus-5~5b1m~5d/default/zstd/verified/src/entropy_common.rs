//! Translation of `common/entropy_common.c`
#![allow(dead_code)]

use core::ffi::{c_char, c_int, c_uint, c_void};

use crate::bits::{ZSTD_countTrailingZeros32, ZSTD_highbit32};
use crate::cmem::*;
use crate::error_private::*;
use crate::fse::{FSE_MIN_TABLELOG, FSE_TABLELOG_ABSOLUTE_MAX, FSE_VERSION_NUMBER};
use crate::fse_decompress::FSE_decompress_wksp_bmi2;
use crate::huf::{HUF_READ_STATS_WORKSPACE_SIZE_U32, HUF_TABLELOG_MAX, HUF_flags_bmi2};

/*===   Version   ===*/
#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSE_versionNumber() -> c_uint {
    FSE_VERSION_NUMBER as c_uint
}

/*===   Error Management   ===*/
#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSE_isError(code: usize) -> c_uint {
    ERR_isError(code)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSE_getErrorName(code: usize) -> *const c_char {
    ERR_getErrorName(code)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_isError(code: usize) -> c_uint {
    ERR_isError(code)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_getErrorName(code: usize) -> *const c_char {
    ERR_getErrorName(code)
}

/*-**************************************************************
*  FSE NCount encoding-decoding
****************************************************************/
pub(crate) unsafe fn FSE_readNCount_body(
    normalizedCounter: *mut i16,
    maxSVPtr: *mut c_uint,
    tableLogPtr: *mut c_uint,
    headerBuffer: *const c_void,
    hbSize: usize,
) -> usize {
    let istart = headerBuffer as *const BYTE;
    let iend = istart.wrapping_add(hbSize);
    let mut ip = istart;
    let mut nbBits: c_int;
    let mut remaining: c_int;
    let mut threshold: c_int;
    let mut bitStream: U32;
    let mut bitCount: c_int;
    let mut charnum: c_uint = 0;
    let maxSV1: c_uint = (*maxSVPtr).wrapping_add(1);
    let mut previous0: c_int = 0;

    if hbSize < 8 {
        /* This function only works when hbSize >= 8 */
        let mut buffer: [c_char; 8] = [0; 8];
        ZSTD_memcpy(buffer.as_mut_ptr() as *mut c_void, headerBuffer, hbSize);
        {
            let countSize = FSE_readNCount(
                normalizedCounter,
                maxSVPtr,
                tableLogPtr,
                buffer.as_ptr() as *const c_void,
                core::mem::size_of::<[c_char; 8]>(),
            );
            if FSE_isError(countSize) != 0 {
                return countSize;
            }
            if countSize > hbSize {
                return ERROR(ZSTD_error_corruption_detected);
            }
            return countSize;
        }
    }

    /* init */
    ZSTD_memset(
        normalizedCounter as *mut c_void,
        0,
        ((*maxSVPtr).wrapping_add(1) as usize) * core::mem::size_of::<i16>(),
    ); /* all symbols not present in NCount have a frequency of 0 */
    bitStream = MEM_readLE32(ip as *const c_void);
    nbBits = ((bitStream & 0xF) as c_int) + FSE_MIN_TABLELOG as c_int; /* extract tableLog */
    if nbBits > FSE_TABLELOG_ABSOLUTE_MAX as c_int {
        return ERROR(ZSTD_error_tableLog_tooLarge);
    }
    bitStream >>= 4;
    bitCount = 4;
    *tableLogPtr = nbBits as c_uint;
    remaining = (1i32 << nbBits) + 1;
    threshold = 1i32 << nbBits;
    nbBits += 1;

    loop {
        if previous0 != 0 {
            /* Count the number of repeats. Each time the
             * 2-bit repeat code is 0b11 there is another
             * repeat.
             * Avoid UB by setting the high bit to 1.
             */
            let mut repeats: c_int =
                (ZSTD_countTrailingZeros32(!bitStream | 0x80000000) >> 1) as c_int;
            while repeats >= 12 {
                charnum = charnum.wrapping_add(3 * 12);
                if ip <= iend.wrapping_sub(7) {
                    ip = ip.wrapping_add(3);
                } else {
                    bitCount -= (8 * (iend.wrapping_sub(7).offset_from(ip))) as c_int;
                    bitCount &= 31;
                    ip = iend.wrapping_sub(4);
                }
                bitStream = MEM_readLE32(ip as *const c_void) >> bitCount;
                repeats = (ZSTD_countTrailingZeros32(!bitStream | 0x80000000) >> 1) as c_int;
            }
            charnum = charnum.wrapping_add((3 * repeats) as c_uint);
            bitStream >>= 2 * repeats;
            bitCount += 2 * repeats;

            /* Add the final repeat which isn't 0b11. */
            charnum = charnum.wrapping_add(bitStream & 3);
            bitCount += 2;

            /* This is an error, but break and return an error
             * at the end, because returning out of a loop makes
             * it harder for the compiler to optimize.
             */
            if charnum >= maxSV1 {
                break;
            }

            /* We don't need to set the normalized count to 0
             * because we already memset the whole buffer to 0.
             */

            if (ip <= iend.wrapping_sub(7))
                || (ip.wrapping_offset((bitCount >> 3) as isize) <= iend.wrapping_sub(4))
            {
                ip = ip.wrapping_offset((bitCount >> 3) as isize);
                bitCount &= 7;
            } else {
                bitCount -= (8 * (iend.wrapping_sub(4).offset_from(ip))) as c_int;
                bitCount &= 31;
                ip = iend.wrapping_sub(4);
            }
            bitStream = MEM_readLE32(ip as *const c_void) >> bitCount;
        }
        {
            let max: c_int = (2 * threshold - 1) - remaining;
            let mut count: c_int;

            if (bitStream & (threshold - 1) as U32) < max as U32 {
                count = (bitStream & (threshold - 1) as U32) as c_int;
                bitCount += nbBits - 1;
            } else {
                count = (bitStream & (2 * threshold - 1) as U32) as c_int;
                if count >= threshold {
                    count -= max;
                }
                bitCount += nbBits;
            }

            count -= 1; /* extra accuracy */
            /* When it matters (small blocks), this is a
             * predictable branch, because we don't use -1.
             */
            if count >= 0 {
                remaining -= count;
            } else {
                remaining += count;
            }
            *normalizedCounter.add(charnum as usize) = count as i16;
            charnum = charnum.wrapping_add(1);
            previous0 = (count == 0) as c_int;

            if remaining < threshold {
                /* This branch can be folded into the
                 * threshold update condition because we
                 * know that threshold > 1.
                 */
                if remaining <= 1 {
                    break;
                }
                nbBits = ZSTD_highbit32(remaining as U32) as c_int + 1;
                threshold = 1i32 << (nbBits - 1);
            }
            if charnum >= maxSV1 {
                break;
            }

            if (ip <= iend.wrapping_sub(7))
                || (ip.wrapping_offset((bitCount >> 3) as isize) <= iend.wrapping_sub(4))
            {
                ip = ip.wrapping_offset((bitCount >> 3) as isize);
                bitCount &= 7;
            } else {
                bitCount -= (8 * (iend.wrapping_sub(4).offset_from(ip))) as c_int;
                bitCount &= 31;
                ip = iend.wrapping_sub(4);
            }
            bitStream = MEM_readLE32(ip as *const c_void) >> bitCount;
        }
    }
    if remaining != 1 {
        return ERROR(ZSTD_error_corruption_detected);
    }
    /* Only possible when there are too many zeros. */
    if charnum > maxSV1 {
        return ERROR(ZSTD_error_maxSymbolValue_tooSmall);
    }
    if bitCount > 32 {
        return ERROR(ZSTD_error_corruption_detected);
    }
    *maxSVPtr = charnum.wrapping_sub(1);

    ip = ip.wrapping_offset(((bitCount + 7) >> 3) as isize);
    (ip as usize).wrapping_sub(istart as usize)
}

/* Avoids the FORCE_INLINE of the _body() function. */
pub(crate) unsafe fn FSE_readNCount_body_default(
    normalizedCounter: *mut i16,
    maxSVPtr: *mut c_uint,
    tableLogPtr: *mut c_uint,
    headerBuffer: *const c_void,
    hbSize: usize,
) -> usize {
    FSE_readNCount_body(
        normalizedCounter,
        maxSVPtr,
        tableLogPtr,
        headerBuffer,
        hbSize,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSE_readNCount_bmi2(
    normalizedCounter: *mut i16,
    maxSVPtr: *mut c_uint,
    tableLogPtr: *mut c_uint,
    headerBuffer: *const c_void,
    hbSize: usize,
    bmi2: c_int,
) -> usize {
    let _ = bmi2;
    FSE_readNCount_body_default(
        normalizedCounter,
        maxSVPtr,
        tableLogPtr,
        headerBuffer,
        hbSize,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSE_readNCount(
    normalizedCounter: *mut i16,
    maxSVPtr: *mut c_uint,
    tableLogPtr: *mut c_uint,
    headerBuffer: *const c_void,
    hbSize: usize,
) -> usize {
    FSE_readNCount_bmi2(
        normalizedCounter,
        maxSVPtr,
        tableLogPtr,
        headerBuffer,
        hbSize,
        /* bmi2 */ 0,
    )
}

/* HUF_readStats() :
    Read compact Huffman tree, saved by HUF_writeCTable().
    `huffWeight` is destination buffer.
    `rankStats` is assumed to be a table of at least HUF_TABLELOG_MAX U32.
    @return : size read from `src` , or an error Code .
    Note : Needed by HUF_readCTable() and HUF_readDTableX?() .
*/
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_readStats(
    huffWeight: *mut BYTE,
    hwSize: usize,
    rankStats: *mut U32,
    nbSymbolsPtr: *mut U32,
    tableLogPtr: *mut U32,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let mut wksp: [U32; HUF_READ_STATS_WORKSPACE_SIZE_U32] = [0; HUF_READ_STATS_WORKSPACE_SIZE_U32];
    HUF_readStats_wksp(
        huffWeight,
        hwSize,
        rankStats,
        nbSymbolsPtr,
        tableLogPtr,
        src,
        srcSize,
        wksp.as_mut_ptr() as *mut c_void,
        core::mem::size_of::<[U32; HUF_READ_STATS_WORKSPACE_SIZE_U32]>(),
        /* flags */ 0,
    )
}

pub(crate) unsafe fn HUF_readStats_body(
    huffWeight: *mut BYTE,
    hwSize: usize,
    rankStats: *mut U32,
    nbSymbolsPtr: *mut U32,
    tableLogPtr: *mut U32,
    src: *const c_void,
    srcSize: usize,
    workSpace: *mut c_void,
    wkspSize: usize,
    bmi2: c_int,
) -> usize {
    let mut weightTotal: U32;
    let mut ip = src as *const BYTE;
    let mut iSize: usize;
    let oSize: usize;

    if srcSize == 0 {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    iSize = *ip.add(0) as usize;
    /* ZSTD_memset(huffWeight, 0, hwSize);   */
    /* is not necessary, even though some analyzer complain ... */

    if iSize >= 128 {
        /* special header */
        oSize = iSize - 127;
        iSize = (oSize + 1) / 2;
        if iSize + 1 > srcSize {
            return ERROR(ZSTD_error_srcSize_wrong);
        }
        if oSize >= hwSize {
            return ERROR(ZSTD_error_corruption_detected);
        }
        ip = ip.add(1);
        {
            let mut n: U32 = 0;
            while (n as usize) < oSize {
                *huffWeight.add(n as usize) = *ip.add((n / 2) as usize) >> 4;
                *huffWeight.add(n as usize + 1) = *ip.add((n / 2) as usize) & 15;
                n += 2;
            }
        }
    } else {
        /* header compressed with FSE (normal case) */
        if iSize + 1 > srcSize {
            return ERROR(ZSTD_error_srcSize_wrong);
        }
        /* max (hwSize-1) values decoded, as last one is implied */
        oSize = FSE_decompress_wksp_bmi2(
            huffWeight as *mut c_void,
            hwSize - 1,
            ip.add(1) as *const c_void,
            iSize,
            6,
            workSpace,
            wkspSize,
            bmi2,
        );
        if FSE_isError(oSize) != 0 {
            return oSize;
        }
    }

    /* collect weight stats */
    ZSTD_memset(
        rankStats as *mut c_void,
        0,
        (HUF_TABLELOG_MAX as usize + 1) * core::mem::size_of::<U32>(),
    );
    weightTotal = 0;
    {
        let mut n: U32 = 0;
        while (n as usize) < oSize {
            if *huffWeight.add(n as usize) as U32 > HUF_TABLELOG_MAX {
                return ERROR(ZSTD_error_corruption_detected);
            }
            *rankStats.add(*huffWeight.add(n as usize) as usize) =
                (*rankStats.add(*huffWeight.add(n as usize) as usize)).wrapping_add(1);
            weightTotal = weightTotal
                .wrapping_add(((1i32 << *huffWeight.add(n as usize)) >> 1) as U32);
            n += 1;
        }
    }
    if weightTotal == 0 {
        return ERROR(ZSTD_error_corruption_detected);
    }

    /* get last non-null symbol weight (implied, total must be 2^n) */
    {
        let tableLog: U32 = ZSTD_highbit32(weightTotal) + 1;
        if tableLog > HUF_TABLELOG_MAX {
            return ERROR(ZSTD_error_corruption_detected);
        }
        *tableLogPtr = tableLog;
        /* determine last weight */
        {
            let total: U32 = 1u32 << tableLog;
            let rest: U32 = total.wrapping_sub(weightTotal);
            let verif: U32 = 1u32 << ZSTD_highbit32(rest);
            let lastWeight: U32 = ZSTD_highbit32(rest) + 1;
            if verif != rest {
                return ERROR(ZSTD_error_corruption_detected);
            } /* last value must be a clean power of 2 */
            *huffWeight.add(oSize) = lastWeight as BYTE;
            *rankStats.add(lastWeight as usize) =
                (*rankStats.add(lastWeight as usize)).wrapping_add(1);
        }
    }

    /* check tree construction validity */
    if (*rankStats.add(1) < 2) || (*rankStats.add(1) & 1) != 0 {
        return ERROR(ZSTD_error_corruption_detected);
    } /* by construction : at least 2 elts of rank 1, must be even */

    /* results */
    *nbSymbolsPtr = (oSize + 1) as U32;
    iSize + 1
}

/* Avoids the FORCE_INLINE of the _body() function. */
pub(crate) unsafe fn HUF_readStats_body_default(
    huffWeight: *mut BYTE,
    hwSize: usize,
    rankStats: *mut U32,
    nbSymbolsPtr: *mut U32,
    tableLogPtr: *mut U32,
    src: *const c_void,
    srcSize: usize,
    workSpace: *mut c_void,
    wkspSize: usize,
) -> usize {
    HUF_readStats_body(
        huffWeight,
        hwSize,
        rankStats,
        nbSymbolsPtr,
        tableLogPtr,
        src,
        srcSize,
        workSpace,
        wkspSize,
        0,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_readStats_wksp(
    huffWeight: *mut BYTE,
    hwSize: usize,
    rankStats: *mut U32,
    nbSymbolsPtr: *mut U32,
    tableLogPtr: *mut U32,
    src: *const c_void,
    srcSize: usize,
    workSpace: *mut c_void,
    wkspSize: usize,
    flags: c_int,
) -> usize {
    let _ = flags;
    HUF_readStats_body_default(
        huffWeight,
        hwSize,
        rankStats,
        nbSymbolsPtr,
        tableLogPtr,
        src,
        srcSize,
        workSpace,
        wkspSize,
    )
}
