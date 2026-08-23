//! Translation of common/entropy_common.c
//!
//! Common functions of New Generation Entropy library.
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

use crate::bits::*;
use crate::error_private::*;
use crate::fse::*;
use crate::huf::*;
use crate::mem::*;

extern "C" {
    /* defined in common/fse_decompress.c -> crate::fse_decompress */
    fn FSE_decompress_wksp_bmi2(
        dst: *mut core::ffi::c_void,
        dstCapacity: usize,
        cSrc: *const core::ffi::c_void,
        cSrcSize: usize,
        maxLog: core::ffi::c_uint,
        workSpace: *mut core::ffi::c_void,
        wkspSize: usize,
        bmi2: core::ffi::c_int,
    ) -> usize;
}

/*===   Version   ===*/
#[unsafe(no_mangle)]
pub extern "C" fn FSE_versionNumber() -> core::ffi::c_uint {
    FSE_VERSION_NUMBER
}

/*===   Error Management   ===*/
#[unsafe(no_mangle)]
pub extern "C" fn FSE_isError(code: usize) -> core::ffi::c_uint {
    ERR_isError(code)
}

#[unsafe(no_mangle)]
pub extern "C" fn FSE_getErrorName(code: usize) -> *const core::ffi::c_char {
    ERR_getErrorName(code)
}

#[unsafe(no_mangle)]
pub extern "C" fn HUF_isError(code: usize) -> core::ffi::c_uint {
    ERR_isError(code)
}

#[unsafe(no_mangle)]
pub extern "C" fn HUF_getErrorName(code: usize) -> *const core::ffi::c_char {
    ERR_getErrorName(code)
}

/*-**************************************************************
*  FSE NCount encoding-decoding
****************************************************************/
/* FORCE_INLINE_TEMPLATE */
pub unsafe fn FSE_readNCount_body(
    normalizedCounter: *mut i16,
    maxSVPtr: *mut core::ffi::c_uint,
    tableLogPtr: *mut core::ffi::c_uint,
    headerBuffer: *const core::ffi::c_void,
    hbSize: usize,
) -> usize {
    let istart: *const BYTE = headerBuffer as *const BYTE;
    let iend: *const BYTE = istart.add(hbSize);
    let mut ip: *const BYTE = istart;
    let mut nbBits: core::ffi::c_int;
    let mut remaining: core::ffi::c_int;
    let mut threshold: core::ffi::c_int;
    let mut bitStream: U32;
    let mut bitCount: core::ffi::c_int;
    let mut charnum: core::ffi::c_uint = 0;
    let maxSV1: core::ffi::c_uint = (*maxSVPtr).wrapping_add(1);
    let mut previous0: core::ffi::c_int = 0;

    if hbSize < 8 {
        /* This function only works when hbSize >= 8 */
        let mut buffer: [core::ffi::c_char; 8] = [0; 8];
        ZSTD_memcpy(
            buffer.as_mut_ptr() as *mut u8,
            headerBuffer as *const u8,
            hbSize,
        );
        {
            let countSize: usize = FSE_readNCount(
                normalizedCounter,
                maxSVPtr,
                tableLogPtr,
                buffer.as_ptr() as *const core::ffi::c_void,
                core::mem::size_of::<[core::ffi::c_char; 8]>(),
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
        normalizedCounter as *mut u8,
        0,
        ((*maxSVPtr).wrapping_add(1) as usize) * core::mem::size_of::<i16>(),
    ); /* all symbols not present in NCount have a frequency of 0 */
    bitStream = MEM_readLE32(ip);
    nbBits = ((bitStream & 0xF).wrapping_add(FSE_MIN_TABLELOG)) as core::ffi::c_int; /* extract tableLog */
    if nbBits > FSE_TABLELOG_ABSOLUTE_MAX as core::ffi::c_int {
        return ERROR(ZSTD_error_tableLog_tooLarge);
    }
    bitStream >>= 4;
    bitCount = 4;
    *tableLogPtr = nbBits as core::ffi::c_uint;
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
            let mut repeats: core::ffi::c_int =
                (ZSTD_countTrailingZeros32(!bitStream | 0x80000000) >> 1) as core::ffi::c_int;
            while repeats >= 12 {
                charnum = charnum.wrapping_add(3 * 12);
                if ip as isize <= iend as isize - 7 {
                    ip = ip.offset(3);
                } else {
                    bitCount = bitCount.wrapping_sub(
                        (8isize).wrapping_mul(iend as isize - 7 - ip as isize)
                            as core::ffi::c_int,
                    );
                    bitCount &= 31;
                    ip = iend.offset(-4);
                }
                bitStream = MEM_readLE32(ip) >> (bitCount as U32);
                repeats =
                    (ZSTD_countTrailingZeros32(!bitStream | 0x80000000) >> 1) as core::ffi::c_int;
            }
            charnum = charnum.wrapping_add((3 * repeats) as core::ffi::c_uint);
            bitStream >>= (2 * repeats) as U32;
            bitCount = bitCount.wrapping_add(2 * repeats);

            /* Add the final repeat which isn't 0b11. */
            charnum = charnum.wrapping_add(bitStream & 3);
            bitCount = bitCount.wrapping_add(2);

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

            if (ip as isize <= iend as isize - 7)
                || (ip as isize + (bitCount >> 3) as isize <= iend as isize - 4)
            {
                ip = ip.offset((bitCount >> 3) as isize);
                bitCount &= 7;
            } else {
                bitCount = bitCount.wrapping_sub(
                    (8isize).wrapping_mul(iend as isize - 4 - ip as isize) as core::ffi::c_int,
                );
                bitCount &= 31;
                ip = iend.offset(-4);
            }
            bitStream = MEM_readLE32(ip) >> (bitCount as U32);
        }
        {
            let max: core::ffi::c_int = (2 * threshold - 1) - remaining;
            let mut count: core::ffi::c_int;

            if (bitStream & (threshold as U32).wrapping_sub(1)) < (max as U32) {
                count = (bitStream & (threshold as U32).wrapping_sub(1)) as core::ffi::c_int;
                bitCount = bitCount.wrapping_add(nbBits - 1);
            } else {
                count = (bitStream & ((2 * threshold) as U32).wrapping_sub(1)) as core::ffi::c_int;
                if count >= threshold {
                    count -= max;
                }
                bitCount = bitCount.wrapping_add(nbBits);
            }

            count -= 1; /* extra accuracy */
            /* When it matters (small blocks), this is a
             * predictable branch, because we don't use -1.
             */
            if count >= 0 {
                remaining = remaining.wrapping_sub(count);
            } else {
                remaining = remaining.wrapping_add(count);
            }
            *normalizedCounter.add(charnum as usize) = count as i16;
            charnum = charnum.wrapping_add(1);
            previous0 = (count == 0) as core::ffi::c_int;

            if remaining < threshold {
                /* This branch can be folded into the
                 * threshold update condition because we
                 * know that threshold > 1.
                 */
                if remaining <= 1 {
                    break;
                }
                nbBits = ZSTD_highbit32(remaining as U32) as core::ffi::c_int + 1;
                threshold = 1 << (nbBits - 1);
            }
            if charnum >= maxSV1 {
                break;
            }

            if (ip as isize <= iend as isize - 7)
                || (ip as isize + (bitCount >> 3) as isize <= iend as isize - 4)
            {
                ip = ip.offset((bitCount >> 3) as isize);
                bitCount &= 7;
            } else {
                bitCount = bitCount.wrapping_sub(
                    (8isize).wrapping_mul(iend as isize - 4 - ip as isize) as core::ffi::c_int,
                );
                bitCount &= 31;
                ip = iend.offset(-4);
            }
            bitStream = MEM_readLE32(ip) >> (bitCount as U32);
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

    ip = ip.offset(((bitCount + 7) >> 3) as isize);
    (ip as usize).wrapping_sub(istart as usize)
}

/* Avoids the FORCE_INLINE of the _body() function. */
pub unsafe fn FSE_readNCount_body_default(
    normalizedCounter: *mut i16,
    maxSVPtr: *mut core::ffi::c_uint,
    tableLogPtr: *mut core::ffi::c_uint,
    headerBuffer: *const core::ffi::c_void,
    hbSize: usize,
) -> usize {
    FSE_readNCount_body(normalizedCounter, maxSVPtr, tableLogPtr, headerBuffer, hbSize)
}

/* DYNAMIC_BMI2 == 0 : FSE_readNCount_body_bmi2() is not compiled in. */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSE_readNCount_bmi2(
    normalizedCounter: *mut i16,
    maxSVPtr: *mut core::ffi::c_uint,
    tableLogPtr: *mut core::ffi::c_uint,
    headerBuffer: *const core::ffi::c_void,
    hbSize: usize,
    bmi2: core::ffi::c_int,
) -> usize {
    let _ = bmi2;
    FSE_readNCount_body_default(normalizedCounter, maxSVPtr, tableLogPtr, headerBuffer, hbSize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn FSE_readNCount(
    normalizedCounter: *mut i16,
    maxSVPtr: *mut core::ffi::c_uint,
    tableLogPtr: *mut core::ffi::c_uint,
    headerBuffer: *const core::ffi::c_void,
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
    src: *const core::ffi::c_void,
    srcSize: usize,
) -> usize {
    let mut wksp = core::mem::MaybeUninit::<[U32; HUF_READ_STATS_WORKSPACE_SIZE_U32]>::uninit();
    HUF_readStats_wksp(
        huffWeight,
        hwSize,
        rankStats,
        nbSymbolsPtr,
        tableLogPtr,
        src,
        srcSize,
        wksp.as_mut_ptr() as *mut core::ffi::c_void,
        core::mem::size_of::<[U32; HUF_READ_STATS_WORKSPACE_SIZE_U32]>(),
        /* flags */ 0,
    )
}

/* FORCE_INLINE_TEMPLATE */
pub unsafe fn HUF_readStats_body(
    huffWeight: *mut BYTE,
    hwSize: usize,
    rankStats: *mut U32,
    nbSymbolsPtr: *mut U32,
    tableLogPtr: *mut U32,
    src: *const core::ffi::c_void,
    srcSize: usize,
    workSpace: *mut core::ffi::c_void,
    wkspSize: usize,
    bmi2: core::ffi::c_int,
) -> usize {
    let mut weightTotal: U32;
    let mut ip: *const BYTE = src as *const BYTE;
    let mut iSize: usize;
    let mut oSize: usize;

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
        ip = ip.offset(1);
        {
            let mut n: U32;
            n = 0;
            while (n as usize) < oSize {
                *huffWeight.add(n as usize) = *ip.add((n / 2) as usize) >> 4;
                *huffWeight.add(n.wrapping_add(1) as usize) = *ip.add((n / 2) as usize) & 15;
                n = n.wrapping_add(2);
            }
        }
    } else {
        /* header compressed with FSE (normal case) */
        if iSize + 1 > srcSize {
            return ERROR(ZSTD_error_srcSize_wrong);
        }
        /* max (hwSize-1) values decoded, as last one is implied */
        oSize = FSE_decompress_wksp_bmi2(
            huffWeight as *mut core::ffi::c_void,
            hwSize.wrapping_sub(1),
            ip.offset(1) as *const core::ffi::c_void,
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
        rankStats as *mut u8,
        0,
        (HUF_TABLELOG_MAX as usize + 1) * core::mem::size_of::<U32>(),
    );
    weightTotal = 0;
    {
        let mut n: U32;
        n = 0;
        while (n as usize) < oSize {
            if (*huffWeight.add(n as usize) as core::ffi::c_int)
                > HUF_TABLELOG_MAX as core::ffi::c_int
            {
                return ERROR(ZSTD_error_corruption_detected);
            }
            *rankStats.add(*huffWeight.add(n as usize) as usize) =
                (*rankStats.add(*huffWeight.add(n as usize) as usize)).wrapping_add(1);
            weightTotal = weightTotal.wrapping_add(
                ((1 << *huffWeight.add(n as usize)) >> 1) as U32,
            );
            n = n.wrapping_add(1);
        }
    }
    if weightTotal == 0 {
        return ERROR(ZSTD_error_corruption_detected);
    }

    /* get last non-null symbol weight (implied, total must be 2^n) */
    {
        let tableLog: U32 = ZSTD_highbit32(weightTotal).wrapping_add(1);
        if tableLog > HUF_TABLELOG_MAX {
            return ERROR(ZSTD_error_corruption_detected);
        }
        *tableLogPtr = tableLog;
        /* determine last weight */
        {
            let total: U32 = 1u32 << tableLog;
            let rest: U32 = total.wrapping_sub(weightTotal);
            let verif: U32 = 1u32 << ZSTD_highbit32(rest);
            let lastWeight: U32 = ZSTD_highbit32(rest).wrapping_add(1);
            if verif != rest {
                return ERROR(ZSTD_error_corruption_detected);
            } /* last value must be a clean power of 2 */
            *huffWeight.add(oSize) = lastWeight as BYTE;
            *rankStats.add(lastWeight as usize) =
                (*rankStats.add(lastWeight as usize)).wrapping_add(1);
        }
    }

    /* check tree construction validity */
    if (*rankStats.add(1) < 2) || ((*rankStats.add(1) & 1) != 0) {
        return ERROR(ZSTD_error_corruption_detected);
    } /* by construction : at least 2 elts of rank 1, must be even */

    /* results */
    *nbSymbolsPtr = (oSize + 1) as U32;
    iSize + 1
}

/* Avoids the FORCE_INLINE of the _body() function. */
pub unsafe fn HUF_readStats_body_default(
    huffWeight: *mut BYTE,
    hwSize: usize,
    rankStats: *mut U32,
    nbSymbolsPtr: *mut U32,
    tableLogPtr: *mut U32,
    src: *const core::ffi::c_void,
    srcSize: usize,
    workSpace: *mut core::ffi::c_void,
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

/* DYNAMIC_BMI2 == 0 : HUF_readStats_body_bmi2() is not compiled in. */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_readStats_wksp(
    huffWeight: *mut BYTE,
    hwSize: usize,
    rankStats: *mut U32,
    nbSymbolsPtr: *mut U32,
    tableLogPtr: *mut U32,
    src: *const core::ffi::c_void,
    srcSize: usize,
    workSpace: *mut core::ffi::c_void,
    wkspSize: usize,
    flags: core::ffi::c_int,
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
