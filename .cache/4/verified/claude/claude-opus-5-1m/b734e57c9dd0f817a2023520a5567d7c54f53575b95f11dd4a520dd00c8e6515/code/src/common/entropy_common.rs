//! Translation of `common/entropy_common.c`
#![allow(dead_code)]

use super::bits::*;
use super::error_private::*;
use super::fse::*;
use super::huf::*;
use super::mem::*;
use crate::libc::*;
use core::ffi::{c_char, c_int, c_void};

/*===   Version   ===*/
#[unsafe(no_mangle)]
pub extern "C" fn FSE_versionNumber() -> u32 {
    FSE_VERSION_NUMBER
}

/*===   Error Management   ===*/
#[unsafe(no_mangle)]
pub extern "C" fn FSE_isError(code: usize) -> u32 {
    ERR_isError(code)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSE_getErrorName(code: usize) -> *const c_char {
    ERR_getErrorName(code)
}

#[unsafe(no_mangle)]
pub extern "C" fn HUF_isError(code: usize) -> u32 {
    ERR_isError(code)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_getErrorName(code: usize) -> *const c_char {
    ERR_getErrorName(code)
}

/*-**************************************************************
*  FSE NCount encoding-decoding
****************************************************************/
unsafe fn FSE_readNCount_body(
    normalizedCounter: *mut i16,
    maxSVPtr: *mut u32,
    tableLogPtr: *mut u32,
    headerBuffer: *const c_void,
    hbSize: usize,
) -> usize {
    let istart = headerBuffer as *const BYTE;
    let iend = istart.add(hbSize);
    let mut ip = istart;
    let mut nbBits: c_int;
    let mut remaining: c_int;
    let mut threshold: c_int;
    let mut bitStream: U32;
    let mut bitCount: c_int;
    let mut charnum: u32 = 0;
    let maxSV1: u32 = (*maxSVPtr) + 1;
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
                8,
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
        ((*maxSVPtr) as usize + 1) * core::mem::size_of::<i16>(),
    );
    bitStream = MEM_readLE32(ip as *const c_void);
    nbBits = ((bitStream & 0xF) + FSE_MIN_TABLELOG) as c_int;
    if nbBits > FSE_TABLELOG_ABSOLUTE_MAX as c_int {
        return ERROR(ZSTD_error_tableLog_tooLarge);
    }
    bitStream >>= 4;
    bitCount = 4;
    *tableLogPtr = nbBits as u32;
    remaining = (1i32 << nbBits) + 1;
    threshold = 1i32 << nbBits;
    nbBits += 1;

    loop {
        if previous0 != 0 {
            let mut repeats = (ZSTD_countTrailingZeros32(!bitStream | 0x80000000) >> 1) as c_int;
            while repeats >= 12 {
                charnum += 3 * 12;
                if ip <= iend.sub(7) {
                    ip = ip.add(3);
                } else {
                    bitCount -= (8 * (iend.sub(7).offset_from(ip))) as c_int;
                    bitCount &= 31;
                    ip = iend.sub(4);
                }
                bitStream = MEM_readLE32(ip as *const c_void) >> bitCount;
                repeats = (ZSTD_countTrailingZeros32(!bitStream | 0x80000000) >> 1) as c_int;
            }
            charnum += 3 * repeats as u32;
            bitStream >>= 2 * repeats;
            bitCount += 2 * repeats;

            /* Add the final repeat which isn't 0b11. */
            charnum += bitStream & 3;
            bitCount += 2;

            if charnum >= maxSV1 {
                break;
            }

            if (ip <= iend.sub(7)) || (ip.offset((bitCount >> 3) as isize) <= iend.sub(4)) {
                ip = ip.offset((bitCount >> 3) as isize);
                bitCount &= 7;
            } else {
                bitCount -= (8 * (iend.sub(4).offset_from(ip))) as c_int;
                bitCount &= 31;
                ip = iend.sub(4);
            }
            bitStream = MEM_readLE32(ip as *const c_void) >> bitCount;
        }
        {
            let max = (2 * threshold - 1) - remaining;
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
            if count >= 0 {
                remaining -= count;
            } else {
                remaining += count;
            }
            *normalizedCounter.add(charnum as usize) = count as i16;
            charnum += 1;
            previous0 = (count == 0) as c_int;

            if remaining < threshold {
                if remaining <= 1 {
                    break;
                }
                nbBits = ZSTD_highbit32(remaining as U32) as c_int + 1;
                threshold = 1i32 << (nbBits - 1);
            }
            if charnum >= maxSV1 {
                break;
            }

            if (ip <= iend.sub(7)) || (ip.offset((bitCount >> 3) as isize) <= iend.sub(4)) {
                ip = ip.offset((bitCount >> 3) as isize);
                bitCount &= 7;
            } else {
                bitCount -= (8 * (iend.sub(4).offset_from(ip))) as c_int;
                bitCount &= 31;
                ip = iend.sub(4);
            }
            bitStream = MEM_readLE32(ip as *const c_void) >> bitCount;
        }
    }
    if remaining != 1 {
        return ERROR(ZSTD_error_corruption_detected);
    }
    if charnum > maxSV1 {
        return ERROR(ZSTD_error_maxSymbolValue_tooSmall);
    }
    if bitCount > 32 {
        return ERROR(ZSTD_error_corruption_detected);
    }
    *maxSVPtr = charnum - 1;

    ip = ip.offset(((bitCount + 7) >> 3) as isize);
    ip.offset_from(istart) as usize
}

unsafe fn FSE_readNCount_body_default(
    normalizedCounter: *mut i16,
    maxSVPtr: *mut u32,
    tableLogPtr: *mut u32,
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
    maxSVPtr: *mut u32,
    tableLogPtr: *mut u32,
    headerBuffer: *const c_void,
    hbSize: usize,
    _bmi2: c_int,
) -> usize {
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
    maxSVPtr: *mut u32,
    tableLogPtr: *mut u32,
    headerBuffer: *const c_void,
    hbSize: usize,
) -> usize {
    FSE_readNCount_bmi2(
        normalizedCounter,
        maxSVPtr,
        tableLogPtr,
        headerBuffer,
        hbSize,
        0,
    )
}

/* HUF_readStats() */
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
        core::mem::size_of_val(&wksp),
        0,
    )
}

unsafe fn HUF_readStats_body(
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
        oSize = crate::common::fse_decompress::FSE_decompress_wksp_bmi2(
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
            if *huffWeight.add(n as usize) as u32 > HUF_TABLELOG_MAX {
                return ERROR(ZSTD_error_corruption_detected);
            }
            *rankStats.add(*huffWeight.add(n as usize) as usize) += 1;
            weightTotal = weightTotal.wrapping_add((1u32 << *huffWeight.add(n as usize)) >> 1);
            n += 1;
        }
    }
    if weightTotal == 0 {
        return ERROR(ZSTD_error_corruption_detected);
    }

    /* get last non-null symbol weight (implied, total must be 2^n) */
    {
        let tableLog = ZSTD_highbit32(weightTotal) + 1;
        if tableLog > HUF_TABLELOG_MAX {
            return ERROR(ZSTD_error_corruption_detected);
        }
        *tableLogPtr = tableLog;
        {
            let total = 1u32 << tableLog;
            let rest = total - weightTotal;
            let verif = 1u32 << ZSTD_highbit32(rest);
            let lastWeight = ZSTD_highbit32(rest) + 1;
            if verif != rest {
                return ERROR(ZSTD_error_corruption_detected);
            }
            *huffWeight.add(oSize) = lastWeight as BYTE;
            *rankStats.add(lastWeight as usize) += 1;
        }
    }

    /* check tree construction validity */
    if (*rankStats.add(1) < 2) || (*rankStats.add(1) & 1) != 0 {
        return ERROR(ZSTD_error_corruption_detected);
    }

    /* results */
    *nbSymbolsPtr = (oSize + 1) as U32;
    iSize + 1
}

unsafe fn HUF_readStats_body_default(
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
    _flags: c_int,
) -> usize {
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
