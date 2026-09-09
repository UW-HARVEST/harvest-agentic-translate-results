//! Translation of `common/entropy_common.c`.
#![allow(dead_code)]

use super::bits::{ZSTD_countTrailingZeros32, ZSTD_highbit32};
use super::error_private::*;
use super::fse::{FSE_MIN_TABLELOG, FSE_TABLELOG_ABSOLUTE_MAX, FSE_VERSION_NUMBER};
use super::huf::HUF_READ_STATS_WORKSPACE_SIZE_U32;
use super::fse_decompress::FSE_decompress_wksp_bmi2;
use super::huf::HUF_TABLELOG_MAX;
use super::mem::{
    size_t, MEM_readLE32, ZSTD_memcpy, ZSTD_memset, BYTE, S16, U32,
};
use core::ffi::{c_char, c_int, c_uint, c_void};

/*===   Version   ===*/
#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSE_versionNumber() -> c_uint {
    FSE_VERSION_NUMBER
}

/*===   Error Management   ===*/
#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSE_isError(code: size_t) -> c_uint {
    ERR_isError(code)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSE_getErrorName(code: size_t) -> *const c_char {
    ERR_getErrorName(code)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_isError(code: size_t) -> c_uint {
    ERR_isError(code)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_getErrorName(code: size_t) -> *const c_char {
    ERR_getErrorName(code)
}

/*-**************************************************************
*  FSE NCount encoding-decoding
****************************************************************/
pub unsafe fn FSE_readNCount_body(
    normalizedCounter: *mut S16,
    maxSVPtr: *mut c_uint,
    tableLogPtr: *mut c_uint,
    headerBuffer: *const c_void,
    hbSize: size_t,
) -> size_t {
    let istart: *const BYTE = headerBuffer as *const BYTE;
    let iend: *const BYTE = istart.add(hbSize);
    let mut ip: *const BYTE = istart;
    let mut nbBits: c_int;
    let mut remaining: c_int;
    let mut threshold: c_int;
    let mut bitStream: U32;
    let mut bitCount: c_int;
    let mut charnum: c_uint = 0;
    let maxSV1: c_uint = *maxSVPtr + 1;
    let mut previous0: c_int = 0;

    if hbSize < 8 {
        /* This function only works when hbSize >= 8 */
        let mut buffer: [c_char; 8] = [0; 8];
        ZSTD_memcpy(
            buffer.as_mut_ptr() as *mut BYTE,
            headerBuffer as *const BYTE,
            hbSize,
        );
        {
            let countSize: size_t = FSE_readNCount(
                normalizedCounter,
                maxSVPtr,
                tableLogPtr,
                buffer.as_ptr() as *const c_void,
                core::mem::size_of::<[c_char; 8]>() as size_t,
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
        normalizedCounter as *mut BYTE,
        0,
        ((*maxSVPtr + 1) as size_t) * core::mem::size_of::<S16>() as size_t,
    ); /* all symbols not present in NCount have a frequency of 0 */
    bitStream = MEM_readLE32(ip);
    nbBits = ((bitStream & 0xF) + FSE_MIN_TABLELOG) as c_int; /* extract tableLog */
    if nbBits > FSE_TABLELOG_ABSOLUTE_MAX as c_int {
        return ERROR(ZSTD_error_tableLog_tooLarge);
    }
    bitStream >>= 4;
    bitCount = 4;
    *tableLogPtr = nbBits as c_uint;
    remaining = (1 << nbBits) + 1;
    threshold = 1 << nbBits;
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
                charnum += 3 * 12;
                if ip <= iend.sub(7) {
                    ip = ip.add(3);
                } else {
                    bitCount -= (8 * (iend.offset_from(ip) - 7)) as c_int;
                    bitCount &= 31;
                    ip = iend.sub(4);
                }
                bitStream = MEM_readLE32(ip) >> bitCount;
                repeats = (ZSTD_countTrailingZeros32(!bitStream | 0x80000000) >> 1) as c_int;
            }
            charnum += (3 * repeats) as c_uint;
            bitStream >>= 2 * repeats;
            bitCount += 2 * repeats;

            /* Add the final repeat which isn't 0b11. */
            charnum += bitStream & 3;
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

            if (ip <= iend.sub(7)) || (ip.add((bitCount >> 3) as size_t) <= iend.sub(4)) {
                ip = ip.add((bitCount >> 3) as size_t);
                bitCount &= 7;
            } else {
                bitCount -= (8 * (iend.offset_from(ip) - 4)) as c_int;
                bitCount &= 31;
                ip = iend.sub(4);
            }
            bitStream = MEM_readLE32(ip) >> bitCount;
        }
        {
            let max: c_int = (2 * threshold - 1) - remaining;
            let mut count: c_int;

            if (bitStream & (threshold as U32 - 1)) < (max as U32) {
                count = (bitStream & (threshold as U32 - 1)) as c_int;
                bitCount += nbBits - 1;
            } else {
                count = (bitStream & (2 * threshold as U32 - 1)) as c_int;
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
            *normalizedCounter.add(charnum as size_t) = count as S16;
            charnum += 1;
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
                threshold = 1 << (nbBits - 1);
            }
            if charnum >= maxSV1 {
                break;
            }

            if (ip <= iend.sub(7)) || (ip.add((bitCount >> 3) as size_t) <= iend.sub(4)) {
                ip = ip.add((bitCount >> 3) as size_t);
                bitCount &= 7;
            } else {
                bitCount -= (8 * (iend.offset_from(ip) - 4)) as c_int;
                bitCount &= 31;
                ip = iend.sub(4);
            }
            bitStream = MEM_readLE32(ip) >> bitCount;
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
    *maxSVPtr = charnum - 1;

    ip = ip.add(((bitCount + 7) >> 3) as size_t);
    ip.offset_from(istart) as size_t
}

/* Avoids the FORCE_INLINE of the _body() function. */
pub unsafe fn FSE_readNCount_body_default(
    normalizedCounter: *mut S16,
    maxSVPtr: *mut c_uint,
    tableLogPtr: *mut c_uint,
    headerBuffer: *const c_void,
    hbSize: size_t,
) -> size_t {
    FSE_readNCount_body(normalizedCounter, maxSVPtr, tableLogPtr, headerBuffer, hbSize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSE_readNCount_bmi2(
    normalizedCounter: *mut S16,
    maxSVPtr: *mut c_uint,
    tableLogPtr: *mut c_uint,
    headerBuffer: *const c_void,
    hbSize: size_t,
    bmi2: c_int,
) -> size_t {
    let _ = bmi2;
    FSE_readNCount_body_default(normalizedCounter, maxSVPtr, tableLogPtr, headerBuffer, hbSize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSE_readNCount(
    normalizedCounter: *mut S16,
    maxSVPtr: *mut c_uint,
    tableLogPtr: *mut c_uint,
    headerBuffer: *const c_void,
    hbSize: size_t,
) -> size_t {
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
    hwSize: size_t,
    rankStats: *mut U32,
    nbSymbolsPtr: *mut U32,
    tableLogPtr: *mut U32,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    let mut wksp: [U32; HUF_READ_STATS_WORKSPACE_SIZE_U32 as usize] =
        [0; HUF_READ_STATS_WORKSPACE_SIZE_U32 as usize];
    HUF_readStats_wksp(
        huffWeight,
        hwSize,
        rankStats,
        nbSymbolsPtr,
        tableLogPtr,
        src,
        srcSize,
        wksp.as_mut_ptr() as *mut c_void,
        core::mem::size_of::<[U32; HUF_READ_STATS_WORKSPACE_SIZE_U32 as usize]>() as size_t,
        /* flags */ 0,
    )
}

pub unsafe fn HUF_readStats_body(
    huffWeight: *mut BYTE,
    hwSize: size_t,
    rankStats: *mut U32,
    nbSymbolsPtr: *mut U32,
    tableLogPtr: *mut U32,
    src: *const c_void,
    srcSize: size_t,
    workSpace: *mut c_void,
    wkspSize: size_t,
    bmi2: c_int,
) -> size_t {
    let weightTotal: U32;
    let mut ip: *const BYTE = src as *const BYTE;
    let mut iSize: size_t;
    let oSize: size_t;

    if srcSize == 0 {
        return ERROR(ZSTD_error_srcSize_wrong);
    }
    iSize = *ip.add(0) as size_t;
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
            while (n as size_t) < oSize {
                *huffWeight.add(n as size_t) = *ip.add((n / 2) as size_t) >> 4;
                *huffWeight.add((n + 1) as size_t) = *ip.add((n / 2) as size_t) & 15;
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
        rankStats as *mut BYTE,
        0,
        (HUF_TABLELOG_MAX as size_t + 1) * core::mem::size_of::<U32>() as size_t,
    );
    let mut weightTotal_local: U32 = 0;
    {
        let mut n: U32 = 0;
        while (n as size_t) < oSize {
            if *huffWeight.add(n as size_t) as U32 > HUF_TABLELOG_MAX {
                return ERROR(ZSTD_error_corruption_detected);
            }
            *rankStats.add(*huffWeight.add(n as size_t) as size_t) += 1;
            weightTotal_local = weightTotal_local
                .wrapping_add((1u32 << *huffWeight.add(n as size_t)) >> 1);
            n += 1;
        }
    }
    weightTotal = weightTotal_local;
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
            let rest: U32 = total - weightTotal;
            let verif: U32 = 1u32 << ZSTD_highbit32(rest);
            let lastWeight: U32 = ZSTD_highbit32(rest) + 1;
            if verif != rest {
                return ERROR(ZSTD_error_corruption_detected);
            } /* last value must be a clean power of 2 */
            *huffWeight.add(oSize) = lastWeight as BYTE;
            *rankStats.add(lastWeight as size_t) += 1;
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
pub unsafe fn HUF_readStats_body_default(
    huffWeight: *mut BYTE,
    hwSize: size_t,
    rankStats: *mut U32,
    nbSymbolsPtr: *mut U32,
    tableLogPtr: *mut U32,
    src: *const c_void,
    srcSize: size_t,
    workSpace: *mut c_void,
    wkspSize: size_t,
) -> size_t {
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
    hwSize: size_t,
    rankStats: *mut U32,
    nbSymbolsPtr: *mut U32,
    tableLogPtr: *mut U32,
    src: *const c_void,
    srcSize: size_t,
    workSpace: *mut c_void,
    wkspSize: size_t,
    flags: c_int,
) -> size_t {
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
