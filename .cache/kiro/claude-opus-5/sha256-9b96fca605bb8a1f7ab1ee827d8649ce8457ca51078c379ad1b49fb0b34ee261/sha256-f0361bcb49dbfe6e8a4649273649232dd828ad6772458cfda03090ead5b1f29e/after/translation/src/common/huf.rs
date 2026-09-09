//! Translation of `common/huf.h` (types & constants).
#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]

use super::fse::*;
use super::mem::*;
use core::ffi::c_uint;

pub const HUF_BLOCKSIZE_MAX: size_t = 128 * 1024;

pub const HUF_WORKSPACE_SIZE: size_t = (8 << 10) + 512;
pub const HUF_WORKSPACE_SIZE_U64: size_t =
    HUF_WORKSPACE_SIZE / core::mem::size_of::<U64>();

pub const HUF_TABLELOG_MAX: u32 = 12;
pub const HUF_TABLELOG_DEFAULT: u32 = 11;
pub const HUF_SYMBOLVALUE_MAX: u32 = 255;
pub const HUF_TABLELOG_ABSOLUTEMAX: u32 = 12;

pub const HUF_CTABLEBOUND: size_t = 129;

#[inline(always)]
pub const fn HUF_BLOCKBOUND(size: size_t) -> size_t {
    size + (size >> 8) + 8
}

#[inline(always)]
pub const fn HUF_COMPRESSBOUND(size: size_t) -> size_t {
    HUF_CTABLEBOUND + HUF_BLOCKBOUND(size)
}

/// `typedef size_t HUF_CElt;`
pub type HUF_CElt = size_t;

#[inline(always)]
pub const fn HUF_CTABLE_SIZE_ST(maxSymbolValue: u32) -> size_t {
    (maxSymbolValue + 2) as size_t
}

#[inline(always)]
pub const fn HUF_CTABLE_SIZE(maxSymbolValue: u32) -> size_t {
    HUF_CTABLE_SIZE_ST(maxSymbolValue) * core::mem::size_of::<size_t>()
}

/// `typedef U32 HUF_DTable;`
pub type HUF_DTable = U32;

#[inline(always)]
pub const fn HUF_DTABLE_SIZE(maxTableLog: u32) -> size_t {
    (1 + (1u32 << maxTableLog)) as size_t
}

/* ---- flags ---- */

pub type HUF_flags_e = c_uint;
pub const HUF_flags_bmi2: HUF_flags_e = 1 << 0;
pub const HUF_flags_optimalDepth: HUF_flags_e = 1 << 1;
pub const HUF_flags_preferRepeat: HUF_flags_e = 1 << 2;
pub const HUF_flags_suspectUncompressible: HUF_flags_e = 1 << 3;
pub const HUF_flags_disableAsm: HUF_flags_e = 1 << 4;
pub const HUF_flags_disableFast: HUF_flags_e = 1 << 5;

pub type HUF_repeat = c_uint;
pub const HUF_repeat_none: HUF_repeat = 0;
pub const HUF_repeat_check: HUF_repeat = 1;
pub const HUF_repeat_valid: HUF_repeat = 2;

pub const HUF_CTABLE_WORKSPACE_SIZE_U32: size_t =
    ((4 * (HUF_SYMBOLVALUE_MAX + 1)) + 192) as size_t;
pub const HUF_CTABLE_WORKSPACE_SIZE: size_t =
    HUF_CTABLE_WORKSPACE_SIZE_U32 * core::mem::size_of::<c_uint>();

pub const HUF_READ_STATS_WORKSPACE_SIZE_U32: size_t =
    FSE_DECOMPRESS_WKSP_SIZE_U32(6, HUF_TABLELOG_MAX - 1);
pub const HUF_READ_STATS_WORKSPACE_SIZE: size_t =
    HUF_READ_STATS_WORKSPACE_SIZE_U32 * core::mem::size_of::<c_uint>();

pub const HUF_DECOMPRESS_WORKSPACE_SIZE: size_t = (2 << 10) + (1 << 9);
pub const HUF_DECOMPRESS_WORKSPACE_SIZE_U32: size_t =
    HUF_DECOMPRESS_WORKSPACE_SIZE / core::mem::size_of::<U32>();

/// `ZSTD_btultra` == 8
pub const HUF_OPTIMAL_DEPTH_THRESHOLD: c_uint = 8;

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct HUF_CTableHeader {
    pub tableLog: BYTE,
    pub maxSymbolValue: BYTE,
    pub unused: [BYTE; core::mem::size_of::<size_t>() - 2],
}
