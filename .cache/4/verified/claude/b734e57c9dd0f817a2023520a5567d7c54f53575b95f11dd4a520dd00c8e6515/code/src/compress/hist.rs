//! Translation of `compress/hist.c` (+ `compress/hist.h`)
#![allow(dead_code)]

use crate::common::error_private::*;
use crate::common::mem::*;
use crate::libc::{ZSTD_memmove, ZSTD_memset};
use core::ffi::c_void;

/* --- advanced histogram functions --- */

pub const HIST_WKSP_SIZE_U32: usize = 1024;
pub const HIST_WKSP_SIZE: usize = HIST_WKSP_SIZE_U32 * core::mem::size_of::<u32>();

/* --- Error management --- */
#[unsafe(no_mangle)]
pub extern "C" fn HIST_isError(code: usize) -> u32 {
    ERR_isError(code)
}

/*-**************************************************************
 *  Histogram functions
 ****************************************************************/
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HIST_add(count: *mut u32, src: *const c_void, srcSize: usize) {
    let mut ip = src as *const BYTE;
    let end = ip.wrapping_add(srcSize);

    while ip < end {
        *count.add(*ip as usize) += 1;
        ip = ip.add(1);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HIST_count_simple(
    count: *mut u32,
    maxSymbolValuePtr: *mut u32,
    src: *const c_void,
    srcSize: usize,
) -> u32 {
    let mut ip = src as *const BYTE;
    let end = ip.wrapping_add(srcSize);
    let mut maxSymbolValue: u32 = *maxSymbolValuePtr;
    let mut largestCount: u32 = 0;

    ZSTD_memset(
        count as *mut c_void,
        0,
        (maxSymbolValue as usize + 1) * core::mem::size_of::<u32>(),
    );
    if srcSize == 0 {
        *maxSymbolValuePtr = 0;
        return 0;
    }

    while ip < end {
        *count.add(*ip as usize) += 1;
        ip = ip.add(1);
    }

    while *count.add(maxSymbolValue as usize) == 0 {
        maxSymbolValue = maxSymbolValue.wrapping_sub(1);
    }
    *maxSymbolValuePtr = maxSymbolValue;

    {
        let mut s: U32 = 0;
        while s <= maxSymbolValue {
            if *count.add(s as usize) > largestCount {
                largestCount = *count.add(s as usize);
            }
            s += 1;
        }
    }

    largestCount
}

type HIST_checkInput_e = i32;
const trustInput: HIST_checkInput_e = 0;
const checkMaxSymbolValue: HIST_checkInput_e = 1;

/* HIST_count_parallel_wksp() :
 * store histogram into 4 intermediate tables, recombined at the end.
 * this design makes better use of OoO cpus,
 * and is noticeably faster when some values are heavily repeated.
 * But it needs some additional workspace for intermediate tables.
 * `workSpace` must be a U32 table of size >= HIST_WKSP_SIZE_U32.
 * @return : largest histogram frequency,
 *           or an error code (notably when histogram's alphabet is larger than *maxSymbolValuePtr) */
unsafe fn HIST_count_parallel_wksp(
    count: *mut u32,
    maxSymbolValuePtr: *mut u32,
    source: *const c_void,
    sourceSize: usize,
    check: HIST_checkInput_e,
    workSpace: *mut U32,
) -> usize {
    let mut ip = source as *const BYTE;
    let iend = ip.wrapping_add(sourceSize);
    let countSize: usize = (*maxSymbolValuePtr as usize + 1) * core::mem::size_of::<u32>();
    let mut max: u32 = 0;
    let Counting1: *mut U32 = workSpace;
    let Counting2: *mut U32 = Counting1.add(256);
    let Counting3: *mut U32 = Counting2.add(256);
    let Counting4: *mut U32 = Counting3.add(256);

    /* safety checks */
    if sourceSize == 0 {
        ZSTD_memset(count as *mut c_void, 0, countSize);
        *maxSymbolValuePtr = 0;
        return 0;
    }
    ZSTD_memset(
        workSpace as *mut c_void,
        0,
        4 * 256 * core::mem::size_of::<u32>(),
    );

    /* by stripes of 16 bytes */
    {
        let mut cached: U32 = MEM_read32(ip as *const c_void);
        ip = ip.add(4);
        while ip < iend.wrapping_sub(15) {
            let mut c: U32 = cached;
            cached = MEM_read32(ip as *const c_void);
            ip = ip.add(4);
            *Counting1.add(c as BYTE as usize) += 1;
            *Counting2.add((c >> 8) as BYTE as usize) += 1;
            *Counting3.add((c >> 16) as BYTE as usize) += 1;
            *Counting4.add((c >> 24) as usize) += 1;
            c = cached;
            cached = MEM_read32(ip as *const c_void);
            ip = ip.add(4);
            *Counting1.add(c as BYTE as usize) += 1;
            *Counting2.add((c >> 8) as BYTE as usize) += 1;
            *Counting3.add((c >> 16) as BYTE as usize) += 1;
            *Counting4.add((c >> 24) as usize) += 1;
            c = cached;
            cached = MEM_read32(ip as *const c_void);
            ip = ip.add(4);
            *Counting1.add(c as BYTE as usize) += 1;
            *Counting2.add((c >> 8) as BYTE as usize) += 1;
            *Counting3.add((c >> 16) as BYTE as usize) += 1;
            *Counting4.add((c >> 24) as usize) += 1;
            c = cached;
            cached = MEM_read32(ip as *const c_void);
            ip = ip.add(4);
            *Counting1.add(c as BYTE as usize) += 1;
            *Counting2.add((c >> 8) as BYTE as usize) += 1;
            *Counting3.add((c >> 16) as BYTE as usize) += 1;
            *Counting4.add((c >> 24) as usize) += 1;
        }
        ip = ip.wrapping_sub(4);
    }

    /* finish last symbols */
    while ip < iend {
        *Counting1.add(*ip as usize) += 1;
        ip = ip.add(1);
    }

    {
        let mut s: U32 = 0;
        while s < 256 {
            *Counting1.add(s as usize) = (*Counting1.add(s as usize))
                .wrapping_add(
                    (*Counting2.add(s as usize))
                        .wrapping_add(*Counting3.add(s as usize))
                        .wrapping_add(*Counting4.add(s as usize)),
                );
            if *Counting1.add(s as usize) > max {
                max = *Counting1.add(s as usize);
            }
            s += 1;
        }
    }

    {
        let mut maxSymbolValue: u32 = 255;
        while *Counting1.add(maxSymbolValue as usize) == 0 {
            maxSymbolValue = maxSymbolValue.wrapping_sub(1);
        }
        if check != 0 && maxSymbolValue > *maxSymbolValuePtr {
            return ERROR(ZSTD_error_maxSymbolValue_tooSmall);
        }
        *maxSymbolValuePtr = maxSymbolValue;
        /* in case count & Counting1 are overlapping */
        ZSTD_memmove(count as *mut c_void, Counting1 as *const c_void, countSize);
    }
    max as usize
}

/* HIST_countFast_wksp() :
 * Same as HIST_countFast(), but using an externally provided scratch buffer.
 * `workSpace` is a writable buffer which must be 4-bytes aligned,
 * `workSpaceSize` must be >= HIST_WKSP_SIZE
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HIST_countFast_wksp(
    count: *mut u32,
    maxSymbolValuePtr: *mut u32,
    source: *const c_void,
    sourceSize: usize,
    workSpace: *mut c_void,
    workSpaceSize: usize,
) -> usize {
    if sourceSize < 1500 {
        /* heuristic threshold */
        return HIST_count_simple(count, maxSymbolValuePtr, source, sourceSize) as usize;
    }
    if (workSpace as usize) & 3 != 0 {
        return ERROR(ZSTD_error_GENERIC);
    } /* must be aligned on 4-bytes boundaries */
    if workSpaceSize < HIST_WKSP_SIZE {
        return ERROR(ZSTD_error_workSpace_tooSmall);
    }
    HIST_count_parallel_wksp(
        count,
        maxSymbolValuePtr,
        source,
        sourceSize,
        trustInput,
        workSpace as *mut U32,
    )
}

/* HIST_count_wksp() :
 * Same as HIST_count(), but using an externally provided scratch buffer.
 * `workSpace` size must be table of >= HIST_WKSP_SIZE_U32 unsigned */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HIST_count_wksp(
    count: *mut u32,
    maxSymbolValuePtr: *mut u32,
    source: *const c_void,
    sourceSize: usize,
    workSpace: *mut c_void,
    workSpaceSize: usize,
) -> usize {
    if (workSpace as usize) & 3 != 0 {
        return ERROR(ZSTD_error_GENERIC);
    } /* must be aligned on 4-bytes boundaries */
    if workSpaceSize < HIST_WKSP_SIZE {
        return ERROR(ZSTD_error_workSpace_tooSmall);
    }
    if *maxSymbolValuePtr < 255 {
        return HIST_count_parallel_wksp(
            count,
            maxSymbolValuePtr,
            source,
            sourceSize,
            checkMaxSymbolValue,
            workSpace as *mut U32,
        );
    }
    *maxSymbolValuePtr = 255;
    HIST_countFast_wksp(
        count,
        maxSymbolValuePtr,
        source,
        sourceSize,
        workSpace,
        workSpaceSize,
    )
}

/* fast variant (unsafe : won't check if src contains values beyond count[] limit) */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HIST_countFast(
    count: *mut u32,
    maxSymbolValuePtr: *mut u32,
    source: *const c_void,
    sourceSize: usize,
) -> usize {
    let mut tmpCounters: [u32; HIST_WKSP_SIZE_U32] = [0; HIST_WKSP_SIZE_U32];
    HIST_countFast_wksp(
        count,
        maxSymbolValuePtr,
        source,
        sourceSize,
        tmpCounters.as_mut_ptr() as *mut c_void,
        core::mem::size_of::<[u32; HIST_WKSP_SIZE_U32]>(),
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HIST_count(
    count: *mut u32,
    maxSymbolValuePtr: *mut u32,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let mut tmpCounters: [u32; HIST_WKSP_SIZE_U32] = [0; HIST_WKSP_SIZE_U32];
    HIST_count_wksp(
        count,
        maxSymbolValuePtr,
        src,
        srcSize,
        tmpCounters.as_mut_ptr() as *mut c_void,
        core::mem::size_of::<[u32; HIST_WKSP_SIZE_U32]>(),
    )
}
