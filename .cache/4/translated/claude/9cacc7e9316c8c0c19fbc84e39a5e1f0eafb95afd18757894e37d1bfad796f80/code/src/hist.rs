//! Translation of compress/hist.c (+ compress/hist.h)
//!
//! hist : Histogram functions
//! part of Finite State Entropy project
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
use crate::mem::*;

/* --- advanced histogram functions --- */

/* #define HIST_WKSP_SIZE_U32 1024 */
pub const HIST_WKSP_SIZE_U32: usize = 1024;
/* #define HIST_WKSP_SIZE (HIST_WKSP_SIZE_U32 * sizeof(unsigned)) */
pub const HIST_WKSP_SIZE: usize = HIST_WKSP_SIZE_U32 * core::mem::size_of::<core::ffi::c_uint>();

/* --- Error management --- */

/// `unsigned HIST_isError(size_t code) { return ERR_isError(code); }`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HIST_isError(code: usize) -> core::ffi::c_uint {
    ERR_isError(code)
}

/*-**************************************************************
 *  Histogram functions
 ****************************************************************/

/// `void HIST_add(unsigned* count, const void* src, size_t srcSize)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HIST_add(
    count: *mut core::ffi::c_uint,
    src: *const core::ffi::c_void,
    srcSize: usize,
) {
    let mut ip: *const BYTE = src as *const BYTE;
    let end: *const BYTE = ip.wrapping_add(srcSize);

    while ip < end {
        let s = *ip;
        ip = ip.wrapping_add(1);
        *count.add(s as usize) = (*count.add(s as usize)).wrapping_add(1);
    }
}

/// `unsigned HIST_count_simple(unsigned* count, unsigned* maxSymbolValuePtr,
///                             const void* src, size_t srcSize)`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HIST_count_simple(
    count: *mut core::ffi::c_uint,
    maxSymbolValuePtr: *mut core::ffi::c_uint,
    src: *const core::ffi::c_void,
    srcSize: usize,
) -> core::ffi::c_uint {
    let mut ip: *const BYTE = src as *const BYTE;
    let end: *const BYTE = ip.wrapping_add(srcSize);
    let mut maxSymbolValue: core::ffi::c_uint = *maxSymbolValuePtr;
    let mut largestCount: core::ffi::c_uint = 0;

    ZSTD_memset(
        count as *mut u8,
        0,
        (maxSymbolValue.wrapping_add(1) as usize)
            .wrapping_mul(core::mem::size_of::<core::ffi::c_uint>()),
    );
    if srcSize == 0 {
        *maxSymbolValuePtr = 0;
        return 0;
    }

    while ip < end {
        let s = *ip;
        ip = ip.wrapping_add(1);
        *count.add(s as usize) = (*count.add(s as usize)).wrapping_add(1);
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

/* typedef enum { trustInput, checkMaxSymbolValue } HIST_checkInput_e; */
pub type HIST_checkInput_e = core::ffi::c_int;
pub const trustInput: HIST_checkInput_e = 0;
pub const checkMaxSymbolValue: HIST_checkInput_e = 1;

/* HIST_count_parallel_wksp() :
 * store histogram into 4 intermediate tables, recombined at the end.
 * this design makes better use of OoO cpus,
 * and is noticeably faster when some values are heavily repeated.
 * But it needs some additional workspace for intermediate tables.
 * `workSpace` must be a U32 table of size >= HIST_WKSP_SIZE_U32.
 * @return : largest histogram frequency,
 *           or an error code (notably when histogram's alphabet is larger than *maxSymbolValuePtr) */
pub unsafe fn HIST_count_parallel_wksp(
    count: *mut core::ffi::c_uint,
    maxSymbolValuePtr: *mut core::ffi::c_uint,
    source: *const core::ffi::c_void,
    sourceSize: usize,
    check: HIST_checkInput_e,
    workSpace: *mut U32,
) -> usize {
    let mut ip: *const BYTE = source as *const BYTE;
    let iend: *const BYTE = ip.wrapping_add(sourceSize);
    let countSize: usize = ((*maxSymbolValuePtr).wrapping_add(1) as usize)
        .wrapping_mul(core::mem::size_of::<core::ffi::c_uint>());
    let mut max: core::ffi::c_uint = 0;
    let Counting1: *mut U32 = workSpace;
    let Counting2: *mut U32 = Counting1.wrapping_add(256);
    let Counting3: *mut U32 = Counting2.wrapping_add(256);
    let Counting4: *mut U32 = Counting3.wrapping_add(256);

    /* safety checks */
    if sourceSize == 0 {
        ZSTD_memset(count as *mut u8, 0, countSize);
        *maxSymbolValuePtr = 0;
        return 0;
    }
    ZSTD_memset(
        workSpace as *mut u8,
        0,
        4 * 256 * core::mem::size_of::<core::ffi::c_uint>(),
    );

    /* by stripes of 16 bytes */
    {
        let mut cached: U32 = MEM_read32(ip);
        ip = ip.wrapping_add(4);
        while ip < iend.wrapping_sub(15) {
            let mut c: U32 = cached;
            cached = MEM_read32(ip);
            ip = ip.wrapping_add(4);
            *Counting1.add((c as BYTE) as usize) =
                (*Counting1.add((c as BYTE) as usize)).wrapping_add(1);
            *Counting2.add(((c >> 8) as BYTE) as usize) =
                (*Counting2.add(((c >> 8) as BYTE) as usize)).wrapping_add(1);
            *Counting3.add(((c >> 16) as BYTE) as usize) =
                (*Counting3.add(((c >> 16) as BYTE) as usize)).wrapping_add(1);
            *Counting4.add((c >> 24) as usize) =
                (*Counting4.add((c >> 24) as usize)).wrapping_add(1);
            c = cached;
            cached = MEM_read32(ip);
            ip = ip.wrapping_add(4);
            *Counting1.add((c as BYTE) as usize) =
                (*Counting1.add((c as BYTE) as usize)).wrapping_add(1);
            *Counting2.add(((c >> 8) as BYTE) as usize) =
                (*Counting2.add(((c >> 8) as BYTE) as usize)).wrapping_add(1);
            *Counting3.add(((c >> 16) as BYTE) as usize) =
                (*Counting3.add(((c >> 16) as BYTE) as usize)).wrapping_add(1);
            *Counting4.add((c >> 24) as usize) =
                (*Counting4.add((c >> 24) as usize)).wrapping_add(1);
            c = cached;
            cached = MEM_read32(ip);
            ip = ip.wrapping_add(4);
            *Counting1.add((c as BYTE) as usize) =
                (*Counting1.add((c as BYTE) as usize)).wrapping_add(1);
            *Counting2.add(((c >> 8) as BYTE) as usize) =
                (*Counting2.add(((c >> 8) as BYTE) as usize)).wrapping_add(1);
            *Counting3.add(((c >> 16) as BYTE) as usize) =
                (*Counting3.add(((c >> 16) as BYTE) as usize)).wrapping_add(1);
            *Counting4.add((c >> 24) as usize) =
                (*Counting4.add((c >> 24) as usize)).wrapping_add(1);
            c = cached;
            cached = MEM_read32(ip);
            ip = ip.wrapping_add(4);
            *Counting1.add((c as BYTE) as usize) =
                (*Counting1.add((c as BYTE) as usize)).wrapping_add(1);
            *Counting2.add(((c >> 8) as BYTE) as usize) =
                (*Counting2.add(((c >> 8) as BYTE) as usize)).wrapping_add(1);
            *Counting3.add(((c >> 16) as BYTE) as usize) =
                (*Counting3.add(((c >> 16) as BYTE) as usize)).wrapping_add(1);
            *Counting4.add((c >> 24) as usize) =
                (*Counting4.add((c >> 24) as usize)).wrapping_add(1);
        }
        ip = ip.wrapping_sub(4);
    }

    /* finish last symbols */
    while ip < iend {
        let s = *ip;
        ip = ip.wrapping_add(1);
        *Counting1.add(s as usize) = (*Counting1.add(s as usize)).wrapping_add(1);
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
            s = s.wrapping_add(1);
        }
    }

    {
        let mut maxSymbolValue: core::ffi::c_uint = 255;
        while *Counting1.add(maxSymbolValue as usize) == 0 {
            maxSymbolValue = maxSymbolValue.wrapping_sub(1);
        }
        if check != 0 && maxSymbolValue > *maxSymbolValuePtr {
            return ERROR(ZSTD_error_maxSymbolValue_tooSmall);
        }
        *maxSymbolValuePtr = maxSymbolValue;
        /* in case count & Counting1 are overlapping */
        ZSTD_memmove(count as *mut u8, Counting1 as *const u8, countSize);
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
    count: *mut core::ffi::c_uint,
    maxSymbolValuePtr: *mut core::ffi::c_uint,
    source: *const core::ffi::c_void,
    sourceSize: usize,
    workSpace: *mut core::ffi::c_void,
    workSpaceSize: usize,
) -> usize {
    if sourceSize < 1500 {
        /* heuristic threshold */
        return HIST_count_simple(count, maxSymbolValuePtr, source, sourceSize) as usize;
    }
    if ((workSpace as usize) & 3) != 0 {
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
 * Same as HIST_count(), but using an externally provided scratch buffer.
 * `workSpace` size must be table of >= HIST_WKSP_SIZE_U32 unsigned */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn HIST_count_wksp(
    count: *mut core::ffi::c_uint,
    maxSymbolValuePtr: *mut core::ffi::c_uint,
    source: *const core::ffi::c_void,
    sourceSize: usize,
    workSpace: *mut core::ffi::c_void,
    workSpaceSize: usize,
) -> usize {
    if ((workSpace as usize) & 3) != 0 {
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
    count: *mut core::ffi::c_uint,
    maxSymbolValuePtr: *mut core::ffi::c_uint,
    source: *const core::ffi::c_void,
    sourceSize: usize,
) -> usize {
    let mut tmpCounters =
        core::mem::MaybeUninit::<[core::ffi::c_uint; HIST_WKSP_SIZE_U32]>::uninit();
    HIST_countFast_wksp(
        count,
        maxSymbolValuePtr,
        source,
        sourceSize,
        tmpCounters.as_mut_ptr() as *mut core::ffi::c_void,
        core::mem::size_of::<[core::ffi::c_uint; HIST_WKSP_SIZE_U32]>(),
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HIST_count(
    count: *mut core::ffi::c_uint,
    maxSymbolValuePtr: *mut core::ffi::c_uint,
    src: *const core::ffi::c_void,
    srcSize: usize,
) -> usize {
    let mut tmpCounters =
        core::mem::MaybeUninit::<[core::ffi::c_uint; HIST_WKSP_SIZE_U32]>::uninit();
    HIST_count_wksp(
        count,
        maxSymbolValuePtr,
        src,
        srcSize,
        tmpCounters.as_mut_ptr() as *mut core::ffi::c_void,
        core::mem::size_of::<[core::ffi::c_uint; HIST_WKSP_SIZE_U32]>(),
    )
}
