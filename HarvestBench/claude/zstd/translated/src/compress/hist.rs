//! Translation of compress/hist.c — histogram functions.
#![allow(dead_code)]
use crate::common::error::{code, err_is_error, error};
use crate::common::mem::mem_read32;
use core::ffi::c_void;

pub const HIST_WKSP_SIZE_U32: usize = 1024;
pub const HIST_WKSP_SIZE: usize = HIST_WKSP_SIZE_U32 * 4;

#[unsafe(no_mangle)]
pub extern "C" fn HIST_isError(code: usize) -> u32 {
    err_is_error(code)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HIST_add(count: *mut u32, src: *const c_void, srcSize: usize) {
    let mut ip = src as *const u8;
    let end = ip.add(srcSize);
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
    let mut ip = src as *const u8;
    let end = ip.add(srcSize);
    let mut maxSymbolValue = *maxSymbolValuePtr;
    let mut largestCount: u32 = 0;

    core::ptr::write_bytes(count, 0, (maxSymbolValue + 1) as usize);
    if srcSize == 0 {
        *maxSymbolValuePtr = 0;
        return 0;
    }

    while ip < end {
        *count.add(*ip as usize) += 1;
        ip = ip.add(1);
    }

    while *count.add(maxSymbolValue as usize) == 0 {
        maxSymbolValue -= 1;
    }
    *maxSymbolValuePtr = maxSymbolValue;

    for s in 0..=maxSymbolValue {
        if *count.add(s as usize) > largestCount {
            largestCount = *count.add(s as usize);
        }
    }
    largestCount
}

const TRUST_INPUT: i32 = 0;
const CHECK_MAX_SYMBOL_VALUE: i32 = 1;

unsafe fn hist_count_parallel_wksp(
    count: *mut u32,
    maxSymbolValuePtr: *mut u32,
    source: *const c_void,
    sourceSize: usize,
    check: i32,
    workSpace: *mut u32,
) -> usize {
    let mut ip = source as *const u8;
    let iend = ip.add(sourceSize);
    let countSize = (*maxSymbolValuePtr + 1) as usize * 4;
    let mut max: u32 = 0;
    let counting1 = workSpace;
    let counting2 = counting1.add(256);
    let counting3 = counting2.add(256);
    let counting4 = counting3.add(256);

    if sourceSize == 0 {
        core::ptr::write_bytes(count as *mut u8, 0, countSize);
        *maxSymbolValuePtr = 0;
        return 0;
    }
    core::ptr::write_bytes(workSpace, 0, 4 * 256);

    {
        let mut cached = mem_read32(ip as *const c_void);
        ip = ip.add(4);
        while ip < iend.sub(15) {
            let mut c = cached;
            cached = mem_read32(ip as *const c_void);
            ip = ip.add(4);
            *counting1.add((c & 0xFF) as usize) += 1;
            *counting2.add(((c >> 8) & 0xFF) as usize) += 1;
            *counting3.add(((c >> 16) & 0xFF) as usize) += 1;
            *counting4.add((c >> 24) as usize) += 1;
            c = cached;
            cached = mem_read32(ip as *const c_void);
            ip = ip.add(4);
            *counting1.add((c & 0xFF) as usize) += 1;
            *counting2.add(((c >> 8) & 0xFF) as usize) += 1;
            *counting3.add(((c >> 16) & 0xFF) as usize) += 1;
            *counting4.add((c >> 24) as usize) += 1;
            c = cached;
            cached = mem_read32(ip as *const c_void);
            ip = ip.add(4);
            *counting1.add((c & 0xFF) as usize) += 1;
            *counting2.add(((c >> 8) & 0xFF) as usize) += 1;
            *counting3.add(((c >> 16) & 0xFF) as usize) += 1;
            *counting4.add((c >> 24) as usize) += 1;
            c = cached;
            cached = mem_read32(ip as *const c_void);
            ip = ip.add(4);
            *counting1.add((c & 0xFF) as usize) += 1;
            *counting2.add(((c >> 8) & 0xFF) as usize) += 1;
            *counting3.add(((c >> 16) & 0xFF) as usize) += 1;
            *counting4.add((c >> 24) as usize) += 1;
        }
        ip = ip.sub(4);
    }

    while ip < iend {
        *counting1.add(*ip as usize) += 1;
        ip = ip.add(1);
    }

    for s in 0..256usize {
        *counting1.add(s) +=
            *counting2.add(s) + *counting3.add(s) + *counting4.add(s);
        if *counting1.add(s) > max {
            max = *counting1.add(s);
        }
    }

    {
        let mut maxSymbolValue = 255usize;
        while *counting1.add(maxSymbolValue) == 0 {
            maxSymbolValue -= 1;
        }
        if check != 0 && maxSymbolValue as u32 > *maxSymbolValuePtr {
            return error(code::MAXSYMBOLVALUE_TOOSMALL);
        }
        *maxSymbolValuePtr = maxSymbolValue as u32;
        core::ptr::copy(counting1 as *const u8, count as *mut u8, countSize);
    }
    max as usize
}

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
        return HIST_count_simple(count, maxSymbolValuePtr, source, sourceSize) as usize;
    }
    if (workSpace as usize) & 3 != 0 {
        return error(code::GENERIC);
    }
    if workSpaceSize < HIST_WKSP_SIZE {
        return error(code::WORKSPACE_TOOSMALL);
    }
    hist_count_parallel_wksp(
        count,
        maxSymbolValuePtr,
        source,
        sourceSize,
        TRUST_INPUT,
        workSpace as *mut u32,
    )
}

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
        return error(code::GENERIC);
    }
    if workSpaceSize < HIST_WKSP_SIZE {
        return error(code::WORKSPACE_TOOSMALL);
    }
    if *maxSymbolValuePtr < 255 {
        return hist_count_parallel_wksp(
            count,
            maxSymbolValuePtr,
            source,
            sourceSize,
            CHECK_MAX_SYMBOL_VALUE,
            workSpace as *mut u32,
        );
    }
    *maxSymbolValuePtr = 255;
    HIST_countFast_wksp(count, maxSymbolValuePtr, source, sourceSize, workSpace, workSpaceSize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HIST_countFast(
    count: *mut u32,
    maxSymbolValuePtr: *mut u32,
    source: *const c_void,
    sourceSize: usize,
) -> usize {
    let mut tmpCounters = [0u32; HIST_WKSP_SIZE_U32];
    HIST_countFast_wksp(
        count,
        maxSymbolValuePtr,
        source,
        sourceSize,
        tmpCounters.as_mut_ptr() as *mut c_void,
        core::mem::size_of_val(&tmpCounters),
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HIST_count(
    count: *mut u32,
    maxSymbolValuePtr: *mut u32,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    let mut tmpCounters = [0u32; HIST_WKSP_SIZE_U32];
    HIST_count_wksp(
        count,
        maxSymbolValuePtr,
        src,
        srcSize,
        tmpCounters.as_mut_ptr() as *mut c_void,
        core::mem::size_of_val(&tmpCounters),
    )
}
