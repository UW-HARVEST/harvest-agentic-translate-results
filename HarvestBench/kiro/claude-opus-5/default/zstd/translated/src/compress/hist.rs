//! Translation of `compress/hist.c` and `compress/hist.h`.
//!
//! hist : Histogram functions
//! part of Finite State Entropy project

use core::ffi::{c_uint, c_void};

use crate::common::error_private::{
    ERR_isError, ERROR, ZSTD_error_GENERIC, ZSTD_error_maxSymbolValue_tooSmall,
    ZSTD_error_workSpace_tooSmall,
};
use crate::common::mem::{size_t, MEM_read32, ZSTD_memmove, ZSTD_memset, BYTE, U32};

/* --- advanced histogram functions --- */

pub const HIST_WKSP_SIZE_U32: usize = 1024;
pub const HIST_WKSP_SIZE: usize = HIST_WKSP_SIZE_U32 * core::mem::size_of::<c_uint>();

/* --- Error management --- */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HIST_isError(code: size_t) -> c_uint {
    ERR_isError(code)
}

/*-**************************************************************
 *  Histogram functions
 ****************************************************************/
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HIST_add(count: *mut c_uint, src: *const c_void, srcSize: size_t) {
    let mut ip = src as *const BYTE;
    let end = ip.wrapping_add(srcSize);

    while ip < end {
        let idx = *ip as usize;
        ip = ip.add(1);
        *count.add(idx) = (*count.add(idx)).wrapping_add(1);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HIST_count_simple(
    count: *mut c_uint,
    maxSymbolValuePtr: *mut c_uint,
    src: *const c_void,
    srcSize: size_t,
) -> c_uint {
    let mut ip = src as *const BYTE;
    let end = ip.wrapping_add(srcSize);
    let mut maxSymbolValue = *maxSymbolValuePtr;
    let mut largestCount: c_uint = 0;

    ZSTD_memset(
        count as *mut u8,
        0,
        (maxSymbolValue as usize + 1) * core::mem::size_of::<c_uint>(),
    );
    if srcSize == 0 {
        *maxSymbolValuePtr = 0;
        return 0;
    }

    while ip < end {
        let idx = *ip as usize;
        ip = ip.add(1);
        *count.add(idx) = (*count.add(idx)).wrapping_add(1);
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
            s = s.wrapping_add(1);
        }
    }

    largestCount
}

pub type HIST_checkInput_e = c_uint;
pub const trustInput: HIST_checkInput_e = 0;
pub const checkMaxSymbolValue: HIST_checkInput_e = 1;

/* HIST_count_parallel_wksp() :
 * store histogram into 4 intermediate tables, recombined at the end.
 * `workSpace` must be a U32 table of size >= HIST_WKSP_SIZE_U32.
 * @return : largest histogram frequency,
 *           or an error code */
pub unsafe fn HIST_count_parallel_wksp(
    count: *mut c_uint,
    maxSymbolValuePtr: *mut c_uint,
    source: *const c_void,
    sourceSize: size_t,
    check: HIST_checkInput_e,
    workSpace: *mut U32,
) -> size_t {
    let mut ip = source as *const BYTE;
    let iend = ip.wrapping_add(sourceSize);
    let countSize: size_t = (*maxSymbolValuePtr as usize + 1) * core::mem::size_of::<c_uint>();
    let mut max: c_uint = 0;
    let Counting1: *mut U32 = workSpace;
    let Counting2: *mut U32 = Counting1.add(256);
    let Counting3: *mut U32 = Counting2.add(256);
    let Counting4: *mut U32 = Counting3.add(256);

    /* safety checks */
    if sourceSize == 0 {
        ZSTD_memset(count as *mut u8, 0, countSize);
        *maxSymbolValuePtr = 0;
        return 0;
    }
    ZSTD_memset(
        workSpace as *mut u8,
        0,
        4 * 256 * core::mem::size_of::<c_uint>(),
    );

    /* by stripes of 16 bytes */
    {
        let mut cached: U32 = MEM_read32(ip);
        ip = ip.add(4);
        while ip < iend.wrapping_sub(15) {
            let mut c: U32 = cached;
            cached = MEM_read32(ip);
            ip = ip.add(4);
            *Counting1.add((c as BYTE) as usize) = (*Counting1.add((c as BYTE) as usize)).wrapping_add(1);
            *Counting2.add(((c >> 8) as BYTE) as usize) = (*Counting2.add(((c >> 8) as BYTE) as usize)).wrapping_add(1);
            *Counting3.add(((c >> 16) as BYTE) as usize) = (*Counting3.add(((c >> 16) as BYTE) as usize)).wrapping_add(1);
            *Counting4.add((c >> 24) as usize) = (*Counting4.add((c >> 24) as usize)).wrapping_add(1);
            c = cached;
            cached = MEM_read32(ip);
            ip = ip.add(4);
            *Counting1.add((c as BYTE) as usize) = (*Counting1.add((c as BYTE) as usize)).wrapping_add(1);
            *Counting2.add(((c >> 8) as BYTE) as usize) = (*Counting2.add(((c >> 8) as BYTE) as usize)).wrapping_add(1);
            *Counting3.add(((c >> 16) as BYTE) as usize) = (*Counting3.add(((c >> 16) as BYTE) as usize)).wrapping_add(1);
            *Counting4.add((c >> 24) as usize) = (*Counting4.add((c >> 24) as usize)).wrapping_add(1);
            c = cached;
            cached = MEM_read32(ip);
            ip = ip.add(4);
            *Counting1.add((c as BYTE) as usize) = (*Counting1.add((c as BYTE) as usize)).wrapping_add(1);
            *Counting2.add(((c >> 8) as BYTE) as usize) = (*Counting2.add(((c >> 8) as BYTE) as usize)).wrapping_add(1);
            *Counting3.add(((c >> 16) as BYTE) as usize) = (*Counting3.add(((c >> 16) as BYTE) as usize)).wrapping_add(1);
            *Counting4.add((c >> 24) as usize) = (*Counting4.add((c >> 24) as usize)).wrapping_add(1);
            c = cached;
            cached = MEM_read32(ip);
            ip = ip.add(4);
            *Counting1.add((c as BYTE) as usize) = (*Counting1.add((c as BYTE) as usize)).wrapping_add(1);
            *Counting2.add(((c >> 8) as BYTE) as usize) = (*Counting2.add(((c >> 8) as BYTE) as usize)).wrapping_add(1);
            *Counting3.add(((c >> 16) as BYTE) as usize) = (*Counting3.add(((c >> 16) as BYTE) as usize)).wrapping_add(1);
            *Counting4.add((c >> 24) as usize) = (*Counting4.add((c >> 24) as usize)).wrapping_add(1);
        }
        ip = ip.wrapping_sub(4);
    }

    /* finish last symbols */
    while ip < iend {
        let idx = *ip as usize;
        ip = ip.add(1);
        *Counting1.add(idx) = (*Counting1.add(idx)).wrapping_add(1);
    }

    {
        let mut s: U32 = 0;
        while s < 256 {
            let su = s as usize;
            *Counting1.add(su) = (*Counting1.add(su))
                .wrapping_add((*Counting2.add(su)).wrapping_add(*Counting3.add(su)).wrapping_add(*Counting4.add(su)));
            if *Counting1.add(su) > max {
                max = *Counting1.add(su);
            }
            s = s.wrapping_add(1);
        }
    }

    {
        let mut maxSymbolValue: c_uint = 255;
        while *Counting1.add(maxSymbolValue as usize) == 0 {
            maxSymbolValue = maxSymbolValue.wrapping_sub(1);
        }
        if check != 0 && maxSymbolValue > *maxSymbolValuePtr {
            return ERROR(ZSTD_error_maxSymbolValue_tooSmall);
        }
        *maxSymbolValuePtr = maxSymbolValue;
        ZSTD_memmove(count as *mut u8, Counting1 as *const u8, countSize); /* in case count & Counting1 are overlapping */
    }
    max as size_t
}

/* HIST_countFast_wksp() :
 * Same as HIST_countFast(), but using an externally provided scratch buffer. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HIST_countFast_wksp(
    count: *mut c_uint,
    maxSymbolValuePtr: *mut c_uint,
    source: *const c_void,
    sourceSize: size_t,
    workSpace: *mut c_void,
    workSpaceSize: size_t,
) -> size_t {
    if sourceSize < 1500 {
        /* heuristic threshold */
        return HIST_count_simple(count, maxSymbolValuePtr, source, sourceSize) as size_t;
    }
    if (workSpace as size_t) & 3 != 0 {
        return ERROR(ZSTD_error_GENERIC); /* must be aligned on 4-bytes boundaries */
    }
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
 * Same as HIST_count(), but using an externally provided scratch buffer. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HIST_count_wksp(
    count: *mut c_uint,
    maxSymbolValuePtr: *mut c_uint,
    source: *const c_void,
    sourceSize: size_t,
    workSpace: *mut c_void,
    workSpaceSize: size_t,
) -> size_t {
    if (workSpace as size_t) & 3 != 0 {
        return ERROR(ZSTD_error_GENERIC); /* must be aligned on 4-bytes boundaries */
    }
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
    count: *mut c_uint,
    maxSymbolValuePtr: *mut c_uint,
    source: *const c_void,
    sourceSize: size_t,
) -> size_t {
    let mut tmpCounters: [c_uint; HIST_WKSP_SIZE_U32] = [0; HIST_WKSP_SIZE_U32];
    HIST_countFast_wksp(
        count,
        maxSymbolValuePtr,
        source,
        sourceSize,
        tmpCounters.as_mut_ptr() as *mut c_void,
        core::mem::size_of::<[c_uint; HIST_WKSP_SIZE_U32]>(),
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HIST_count(
    count: *mut c_uint,
    maxSymbolValuePtr: *mut c_uint,
    src: *const c_void,
    srcSize: size_t,
) -> size_t {
    let mut tmpCounters: [c_uint; HIST_WKSP_SIZE_U32] = [0; HIST_WKSP_SIZE_U32];
    HIST_count_wksp(
        count,
        maxSymbolValuePtr,
        src,
        srcSize,
        tmpCounters.as_mut_ptr() as *mut c_void,
        core::mem::size_of::<[c_uint; HIST_WKSP_SIZE_U32]>(),
    )
}
