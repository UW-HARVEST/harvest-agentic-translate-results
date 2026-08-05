//! Public API types from include/zstd.h and zstd_errors.h needed across modules.
#![allow(dead_code)]
use core::ffi::c_void;

pub const ZSTD_CONTENTSIZE_UNKNOWN: u64 = 0u64.wrapping_sub(1);
pub const ZSTD_CONTENTSIZE_ERROR: u64 = 0u64.wrapping_sub(2);

pub const ZSTD_MAGICNUMBER: u32 = 0xFD2FB528;
pub const ZSTD_MAGIC_DICTIONARY: u32 = 0xEC30A437;
pub const ZSTD_MAGIC_SKIPPABLE_START: u32 = 0x184D2A50;
pub const ZSTD_MAGIC_SKIPPABLE_MASK: u32 = 0xFFFFFFF0;

pub const ZSTD_BLOCKSIZELOG_MAX: u32 = 17;
pub const ZSTD_BLOCKSIZE_MAX: usize = 1 << ZSTD_BLOCKSIZELOG_MAX;

// ZSTD_strategy
pub const ZSTD_fast: i32 = 1;
pub const ZSTD_dfast: i32 = 2;
pub const ZSTD_greedy: i32 = 3;
pub const ZSTD_lazy: i32 = 4;
pub const ZSTD_lazy2: i32 = 5;
pub const ZSTD_btlazy2: i32 = 6;
pub const ZSTD_btopt: i32 = 7;
pub const ZSTD_btultra: i32 = 8;
pub const ZSTD_btultra2: i32 = 9;
pub type ZSTD_strategy = i32;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ZSTD_bounds {
    pub error: usize,
    pub lowerBound: i32,
    pub upperBound: i32,
}

// ZSTD_ResetDirective
pub const ZSTD_reset_session_only: u32 = 1;
pub const ZSTD_reset_parameters: u32 = 2;
pub const ZSTD_reset_session_and_parameters: u32 = 3;
pub type ZSTD_ResetDirective = u32;

// ZSTD_EndDirective
pub const ZSTD_e_continue: u32 = 0;
pub const ZSTD_e_flush: u32 = 1;
pub const ZSTD_e_end: u32 = 2;
pub type ZSTD_EndDirective = u32;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ZSTD_inBuffer {
    pub src: *const c_void,
    pub size: usize,
    pub pos: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ZSTD_outBuffer {
    pub dst: *mut c_void,
    pub size: usize,
    pub pos: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ZSTD_frameParameters {
    pub contentSizeFlag: i32,
    pub checksumFlag: i32,
    pub noDictIDFlag: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ZSTD_parameters {
    pub cParams: ZSTD_compressionParameters,
    pub fParams: ZSTD_frameParameters,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ZSTD_compressionParameters {
    pub windowLog: u32,
    pub chainLog: u32,
    pub hashLog: u32,
    pub searchLog: u32,
    pub minMatch: u32,
    pub targetLength: u32,
    pub strategy: ZSTD_strategy,
}

// ZSTD_dictContentType_e
pub const ZSTD_dct_auto: u32 = 0;
pub const ZSTD_dct_rawContent: u32 = 1;
pub const ZSTD_dct_fullDict: u32 = 2;
pub type ZSTD_dictContentType_e = u32;

// ZSTD_dictLoadMethod_e
pub const ZSTD_dlm_byCopy: u32 = 0;
pub const ZSTD_dlm_byRef: u32 = 1;
pub type ZSTD_dictLoadMethod_e = u32;

// ZSTD_format_e
pub const ZSTD_f_zstd1: u32 = 0;
pub const ZSTD_f_zstd1_magicless: u32 = 1;
pub type ZSTD_format_e = u32;

// ZSTD_forceIgnoreChecksum_e
pub const ZSTD_d_validateChecksum: u32 = 0;
pub const ZSTD_d_ignoreChecksum: u32 = 1;
pub type ZSTD_forceIgnoreChecksum_e = u32;

// ZSTD_nextInputType_e
pub const ZSTDnit_frameHeader: u32 = 0;
pub const ZSTDnit_blockHeader: u32 = 1;
pub const ZSTDnit_block: u32 = 2;
pub const ZSTDnit_lastBlock: u32 = 3;
pub const ZSTDnit_checksum: u32 = 4;
pub const ZSTDnit_skippableFrame: u32 = 5;
pub type ZSTD_nextInputType_e = u32;

// ZSTD_ResetDirective already above. ZSTD_refMultipleDDicts_e
pub const ZSTD_rmd_refSingleDDict: u32 = 0;
pub const ZSTD_rmd_refMultipleDDicts: u32 = 1;
pub type ZSTD_refMultipleDDicts_e = u32;
