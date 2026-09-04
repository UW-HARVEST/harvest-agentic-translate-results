//! Translation of `common/huf.h` (constants and types).
#![allow(dead_code)]

use crate::cmem::*;
use crate::fse::*;

pub const HUF_BLOCKSIZE_MAX: usize = 128 * 1024;
pub const HUF_WORKSPACE_SIZE: usize = (8 << 10) + 512;
pub const HUF_WORKSPACE_SIZE_U64: usize = HUF_WORKSPACE_SIZE / 8;

pub const HUF_TABLELOG_MAX: u32 = 12;
pub const HUF_TABLELOG_DEFAULT: u32 = 11;
pub const HUF_SYMBOLVALUE_MAX: u32 = 255;
pub const HUF_TABLELOG_ABSOLUTEMAX: u32 = 12;

pub const HUF_CTABLEBOUND: usize = 129;

#[inline(always)]
pub const fn HUF_BLOCKBOUND(size: usize) -> usize {
    size + (size >> 8) + 8
}
#[inline(always)]
pub const fn HUF_COMPRESSBOUND(size: usize) -> usize {
    HUF_CTABLEBOUND + HUF_BLOCKBOUND(size)
}

pub type HUF_CElt = usize;

#[inline(always)]
pub const fn HUF_CTABLE_SIZE_ST(maxSymbolValue: usize) -> usize {
    maxSymbolValue + 2
}
#[inline(always)]
pub const fn HUF_CTABLE_SIZE(maxSymbolValue: usize) -> usize {
    HUF_CTABLE_SIZE_ST(maxSymbolValue) * core::mem::size_of::<usize>()
}

pub type HUF_DTable = U32;

#[inline(always)]
pub const fn HUF_DTABLE_SIZE(maxTableLog: u32) -> usize {
    (1 + (1u32 << maxTableLog)) as usize
}

pub type HUF_flags_e = i32;
pub const HUF_flags_bmi2: i32 = 1 << 0;
pub const HUF_flags_optimalDepth: i32 = 1 << 1;
pub const HUF_flags_preferRepeat: i32 = 1 << 2;
pub const HUF_flags_suspectUncompressible: i32 = 1 << 3;
pub const HUF_flags_disableAsm: i32 = 1 << 4;
pub const HUF_flags_disableFast: i32 = 1 << 5;

pub type HUF_repeat = u32;
pub const HUF_repeat_none: HUF_repeat = 0;
pub const HUF_repeat_check: HUF_repeat = 1;
pub const HUF_repeat_valid: HUF_repeat = 2;

pub const HUF_CTABLE_WORKSPACE_SIZE_U32: usize = (4 * (HUF_SYMBOLVALUE_MAX as usize + 1)) + 192;
pub const HUF_CTABLE_WORKSPACE_SIZE: usize = HUF_CTABLE_WORKSPACE_SIZE_U32 * 4;

pub const HUF_READ_STATS_WORKSPACE_SIZE_U32: usize =
    FSE_DECOMPRESS_WKSP_SIZE_U32(6, HUF_TABLELOG_MAX - 1);
pub const HUF_READ_STATS_WORKSPACE_SIZE: usize = HUF_READ_STATS_WORKSPACE_SIZE_U32 * 4;

pub const HUF_DECOMPRESS_WORKSPACE_SIZE: usize = (2 << 10) + (1 << 9);
pub const HUF_DECOMPRESS_WORKSPACE_SIZE_U32: usize = HUF_DECOMPRESS_WORKSPACE_SIZE / 4;

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct HUF_CTableHeader {
    pub tableLog: BYTE,
    pub maxSymbolValue: BYTE,
    pub unused: [BYTE; core::mem::size_of::<usize>() - 2],
}
