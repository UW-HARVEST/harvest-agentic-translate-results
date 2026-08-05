//! HUF portions of entropy_common.c: version/error + HUF_readStats.
#![allow(dead_code)]
use super::bits::highbit32;
use super::error::{code, err_is_error, error};
use super::fse::{fse_decompress_wksp_size_u32, FSE_decompress_wksp_bmi2};
use core::ffi::{c_char, c_void};

pub const HUF_TABLELOG_MAX: u32 = 12;
pub const HUF_TABLELOG_DEFAULT: u32 = 11;
pub const HUF_SYMBOLVALUE_MAX: u32 = 255;
pub const HUF_TABLELOG_ABSOLUTEMAX: u32 = 12;
pub const HUF_BLOCKSIZE_MAX: usize = 128 * 1024;
pub const HUF_WORKSPACE_SIZE: usize = (8 << 10) + 512;
pub const HUF_WORKSPACE_SIZE_U64: usize = HUF_WORKSPACE_SIZE / 8;

// HUF_flags_e
pub const HUF_flags_bmi2: i32 = 1 << 0;
pub const HUF_flags_optimalDepth: i32 = 1 << 1;
pub const HUF_flags_preferRepeat: i32 = 1 << 2;
pub const HUF_flags_suspectUncompressible: i32 = 1 << 3;
pub const HUF_flags_disableAsm: i32 = 1 << 4;
pub const HUF_flags_disableFast: i32 = 1 << 5;

pub fn huf_read_stats_workspace_size_u32() -> usize {
    fse_decompress_wksp_size_u32(6, HUF_TABLELOG_MAX - 1)
}

#[unsafe(no_mangle)]
pub extern "C" fn HUF_isError(code: usize) -> u32 {
    err_is_error(code)
}

#[unsafe(no_mangle)]
pub extern "C" fn HUF_getErrorName(code: usize) -> *const c_char {
    super::error::err_get_error_name(code)
}

unsafe fn huf_read_stats_body(
    huffWeight: *mut u8,
    hwSize: usize,
    rankStats: *mut u32,
    nbSymbolsPtr: *mut u32,
    tableLogPtr: *mut u32,
    src: *const c_void,
    srcSize: usize,
    workSpace: *mut c_void,
    wkspSize: usize,
    _bmi2: i32,
) -> usize {
    let mut weightTotal: u32;
    let ip = src as *const u8;
    let mut iSize: usize;
    let oSize: usize;

    if srcSize == 0 {
        return error(code::SRCSIZE_WRONG);
    }
    iSize = *ip.add(0) as usize;

    if iSize >= 128 {
        oSize = iSize - 127;
        iSize = (oSize + 1) / 2;
        if iSize + 1 > srcSize {
            return error(code::SRCSIZE_WRONG);
        }
        if oSize >= hwSize {
            return error(code::CORRUPTION_DETECTED);
        }
        let ip1 = ip.add(1);
        let mut n = 0usize;
        while n < oSize {
            *huffWeight.add(n) = *ip1.add(n / 2) >> 4;
            *huffWeight.add(n + 1) = *ip1.add(n / 2) & 15;
            n += 2;
        }
    } else {
        if iSize + 1 > srcSize {
            return error(code::SRCSIZE_WRONG);
        }
        oSize = FSE_decompress_wksp_bmi2(
            huffWeight as *mut c_void,
            hwSize - 1,
            ip.add(1) as *const c_void,
            iSize,
            6,
            workSpace,
            wkspSize,
            _bmi2,
        );
        if err_is_error(oSize) != 0 {
            return oSize;
        }
    }

    core::ptr::write_bytes(rankStats, 0, (HUF_TABLELOG_MAX + 1) as usize);
    weightTotal = 0;
    {
        let mut n = 0usize;
        while n < oSize {
            let w = *huffWeight.add(n) as u32;
            if w > HUF_TABLELOG_MAX {
                return error(code::CORRUPTION_DETECTED);
            }
            *rankStats.add(w as usize) += 1;
            weightTotal += (1u32 << w) >> 1;
            n += 1;
        }
    }
    if weightTotal == 0 {
        return error(code::CORRUPTION_DETECTED);
    }

    {
        let tableLog = highbit32(weightTotal) + 1;
        if tableLog > HUF_TABLELOG_MAX {
            return error(code::CORRUPTION_DETECTED);
        }
        *tableLogPtr = tableLog;
        {
            let total = 1u32 << tableLog;
            let rest = total - weightTotal;
            let verif = 1u32 << highbit32(rest);
            let lastWeight = highbit32(rest) + 1;
            if verif != rest {
                return error(code::CORRUPTION_DETECTED);
            }
            *huffWeight.add(oSize) = lastWeight as u8;
            *rankStats.add(lastWeight as usize) += 1;
        }
    }

    if (*rankStats.add(1) < 2) || (*rankStats.add(1) & 1) != 0 {
        return error(code::CORRUPTION_DETECTED);
    }

    *nbSymbolsPtr = (oSize + 1) as u32;
    iSize + 1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_readStats_wksp(
    huffWeight: *mut u8,
    hwSize: usize,
    rankStats: *mut u32,
    nbSymbolsPtr: *mut u32,
    tableLogPtr: *mut u32,
    src: *const c_void,
    srcSize: usize,
    workSpace: *mut c_void,
    wkspSize: usize,
    flags: i32,
) -> usize {
    let _ = flags;
    huf_read_stats_body(
        huffWeight, hwSize, rankStats, nbSymbolsPtr, tableLogPtr, src, srcSize, workSpace,
        wkspSize, 0,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn HUF_readStats(
    huffWeight: *mut u8,
    hwSize: usize,
    rankStats: *mut u32,
    nbSymbolsPtr: *mut u32,
    tableLogPtr: *mut u32,
    src: *const c_void,
    srcSize: usize,
) -> usize {
    // HUF_READ_STATS_WORKSPACE_SIZE_U32 stack workspace
    let mut wksp = [0u32; 512]; // FSE_DECOMPRESS_WKSP_SIZE_U32(6,11) fits well within
    let n = huf_read_stats_workspace_size_u32();
    debug_assert!(n <= wksp.len());
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
