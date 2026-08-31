//! Shared public types from `zstd.h` that the decoder (and later the encoder)
//! need. Layout must match the C exactly for every type that crosses the ABI.
#![allow(dead_code)]

use core::ffi::{c_int, c_uint, c_void};

use crate::mem::*;

pub const ZSTD_MAGICNUMBER: U32 = 0xFD2FB528;
pub const ZSTD_MAGIC_DICTIONARY: U32 = 0xEC30A437;
pub const ZSTD_MAGIC_SKIPPABLE_START: U32 = 0x184D2A50;
pub const ZSTD_MAGIC_SKIPPABLE_MASK: U32 = 0xFFFFFFF0;

pub const ZSTD_BLOCKSIZELOG_MAX: u32 = 17;
pub const ZSTD_BLOCKSIZE_MAX: usize = 1 << ZSTD_BLOCKSIZELOG_MAX;

pub const ZSTD_FRAMEHEADERSIZE_MAX: usize = 18;
pub const ZSTD_SKIPPABLEHEADERSIZE: usize = 8;

pub const ZSTD_CONTENTSIZE_UNKNOWN: u64 = 0u64.wrapping_sub(1);
pub const ZSTD_CONTENTSIZE_ERROR: u64 = 0u64.wrapping_sub(2);

pub const ZSTD_WINDOWLOG_MAX_32: u32 = 30;
pub const ZSTD_WINDOWLOG_MAX_64: u32 = 31;

/// `ZSTD_WINDOWLOG_MAX`
pub const ZSTD_WINDOWLOG_MAX: u32 = if core::mem::size_of::<usize>() == 4 {
    ZSTD_WINDOWLOG_MAX_32
} else {
    ZSTD_WINDOWLOG_MAX_64
};
pub const ZSTD_WINDOWLOG_MIN: u32 = 10;

/// `ZSTD_FrameType_e`
pub type ZSTD_FrameType_e = c_uint;
pub const ZSTD_frame: ZSTD_FrameType_e = 0;
pub const ZSTD_skippableFrame: ZSTD_FrameType_e = 1;

/// `ZSTD_FrameHeader`
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct ZSTD_FrameHeader {
    pub frameContentSize: u64,
    pub windowSize: u64,
    pub blockSizeMax: c_uint,
    pub frameType: ZSTD_FrameType_e,
    pub headerSize: c_uint,
    pub dictID: c_uint,
    pub checksumFlag: c_uint,
    pub _reserved1: c_uint,
    pub _reserved2: c_uint,
}

/// `ZSTD_dictContentType_e`
pub type ZSTD_dictContentType_e = c_uint;
pub const ZSTD_dct_auto: ZSTD_dictContentType_e = 0;
pub const ZSTD_dct_rawContent: ZSTD_dictContentType_e = 1;
pub const ZSTD_dct_fullDict: ZSTD_dictContentType_e = 2;

/// `ZSTD_dictLoadMethod_e`
pub type ZSTD_dictLoadMethod_e = c_uint;
pub const ZSTD_dlm_byCopy: ZSTD_dictLoadMethod_e = 0;
pub const ZSTD_dlm_byRef: ZSTD_dictLoadMethod_e = 1;

/// `ZSTD_format_e`
pub type ZSTD_format_e = c_uint;
pub const ZSTD_f_zstd1: ZSTD_format_e = 0;
pub const ZSTD_f_zstd1_magicless: ZSTD_format_e = 1;

/// `ZSTD_FRAMEHEADERSIZE_PREFIX(format)`
#[inline(always)]
pub fn zstd_frameheadersize_prefix(format: ZSTD_format_e) -> usize {
    if format == ZSTD_f_zstd1 {
        5
    } else {
        1
    }
}

/// `ZSTD_FRAMEHEADERSIZE_MIN(format)`
#[inline(always)]
pub fn zstd_frameheadersize_min(format: ZSTD_format_e) -> usize {
    if format == ZSTD_f_zstd1 {
        6
    } else {
        2
    }
}

/// `ZSTD_forceIgnoreChecksum_e`
pub type ZSTD_forceIgnoreChecksum_e = c_uint;
pub const ZSTD_d_validateChecksum: ZSTD_forceIgnoreChecksum_e = 0;
pub const ZSTD_d_ignoreChecksum: ZSTD_forceIgnoreChecksum_e = 1;

/// `ZSTD_refMultipleDDicts_e`
pub type ZSTD_refMultipleDDicts_e = c_uint;
pub const ZSTD_rmd_refSingleDDict: ZSTD_refMultipleDDicts_e = 0;
pub const ZSTD_rmd_refMultipleDDicts: ZSTD_refMultipleDDicts_e = 1;

/// `ZSTD_ResetDirective`
pub type ZSTD_ResetDirective = c_uint;
pub const ZSTD_reset_session_only: ZSTD_ResetDirective = 1;
pub const ZSTD_reset_parameters: ZSTD_ResetDirective = 2;
pub const ZSTD_reset_session_and_parameters: ZSTD_ResetDirective = 3;

/// `ZSTD_bounds`
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct ZSTD_bounds {
    pub error: usize,
    pub lowerBound: c_int,
    pub upperBound: c_int,
}

/// `ZSTD_inBuffer`
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ZSTD_inBuffer {
    pub src: *const c_void,
    pub size: usize,
    pub pos: usize,
}

/// `ZSTD_outBuffer`
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ZSTD_outBuffer {
    pub dst: *mut c_void,
    pub size: usize,
    pub pos: usize,
}

impl Default for ZSTD_outBuffer {
    fn default() -> Self {
        ZSTD_outBuffer {
            dst: core::ptr::null_mut(),
            size: 0,
            pos: 0,
        }
    }
}

impl Default for ZSTD_inBuffer {
    fn default() -> Self {
        ZSTD_inBuffer {
            src: core::ptr::null(),
            size: 0,
            pos: 0,
        }
    }
}

/// `ZSTD_dParameter`
pub type ZSTD_dParameter = c_uint;
pub const ZSTD_d_windowLogMax: ZSTD_dParameter = 100;
pub const ZSTD_d_experimentalParam1: ZSTD_dParameter = 1000;
pub const ZSTD_d_experimentalParam2: ZSTD_dParameter = 1001;
pub const ZSTD_d_experimentalParam3: ZSTD_dParameter = 1002;
pub const ZSTD_d_experimentalParam4: ZSTD_dParameter = 1003;
pub const ZSTD_d_experimentalParam5: ZSTD_dParameter = 1004;
pub const ZSTD_d_experimentalParam6: ZSTD_dParameter = 1005;
