//! Translation of `common/huf.h` — constants and table layout definitions.
#![allow(dead_code)]

use crate::fse::fse_decompress_wksp_size_u32;
use crate::mem::*;

pub const HUF_BLOCKSIZE_MAX: usize = 128 * 1024;
pub const HUF_WORKSPACE_SIZE: usize = (8 << 10) + 512;
pub const HUF_WORKSPACE_SIZE_U64: usize = HUF_WORKSPACE_SIZE / 8;

pub const HUF_TABLELOG_MAX: U32 = 12;
pub const HUF_TABLELOG_DEFAULT: U32 = 11;
pub const HUF_SYMBOLVALUE_MAX: U32 = 255;
pub const HUF_TABLELOG_ABSOLUTEMAX: U32 = 12;

pub const HUF_CTABLEBOUND: usize = 129;

/// `HUF_BLOCKBOUND(size)`
#[inline(always)]
pub const fn huf_blockbound(size: usize) -> usize {
    size + (size >> 8) + 8
}

/// `HUF_COMPRESSBOUND(size)`
#[inline(always)]
pub const fn huf_compressbound(size: usize) -> usize {
    HUF_CTABLEBOUND + huf_blockbound(size)
}

/// `HUF_CElt` — `size_t` in C.
pub type HUF_CElt = usize;

/// `HUF_CTABLE_SIZE_ST(maxSymbolValue)`
#[inline(always)]
pub const fn huf_ctable_size_st(max_symbol_value: usize) -> usize {
    max_symbol_value + 2
}

/// `HUF_CTABLE_SIZE(maxSymbolValue)`
#[inline(always)]
pub const fn huf_ctable_size(max_symbol_value: usize) -> usize {
    huf_ctable_size_st(max_symbol_value) * core::mem::size_of::<usize>()
}

/// `HUF_DTable`
pub type HUF_DTable = U32;

/// `HUF_DTABLE_SIZE(maxTableLog)`
#[inline(always)]
pub const fn huf_dtable_size(max_table_log: u32) -> usize {
    (1 + (1u32 << max_table_log)) as usize
}

/// `HUF_flags_e`
pub const HUF_flags_bmi2: i32 = 1 << 0;
pub const HUF_flags_optimalDepth: i32 = 1 << 1;
pub const HUF_flags_preferRepeat: i32 = 1 << 2;
pub const HUF_flags_suspectUncompressible: i32 = 1 << 3;
pub const HUF_flags_disableAsm: i32 = 1 << 4;
pub const HUF_flags_disableFast: i32 = 1 << 5;

pub const HUF_CTABLE_WORKSPACE_SIZE_U32: usize = (4 * (HUF_SYMBOLVALUE_MAX as usize + 1)) + 192;
pub const HUF_CTABLE_WORKSPACE_SIZE: usize =
    HUF_CTABLE_WORKSPACE_SIZE_U32 * core::mem::size_of::<u32>();

pub const HUF_READ_STATS_WORKSPACE_SIZE_U32: usize =
    fse_decompress_wksp_size_u32(6, HUF_TABLELOG_MAX - 1);
pub const HUF_READ_STATS_WORKSPACE_SIZE: usize =
    HUF_READ_STATS_WORKSPACE_SIZE_U32 * core::mem::size_of::<u32>();

pub const HUF_DECOMPRESS_WORKSPACE_SIZE: usize = (2 << 10) + (1 << 9);
pub const HUF_DECOMPRESS_WORKSPACE_SIZE_U32: usize =
    HUF_DECOMPRESS_WORKSPACE_SIZE / core::mem::size_of::<u32>();

/// `HUF_repeat`
pub const HUF_repeat_none: u32 = 0;
pub const HUF_repeat_check: u32 = 1;
pub const HUF_repeat_valid: u32 = 2;
