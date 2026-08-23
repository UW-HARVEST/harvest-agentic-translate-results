//! Translation of decompress/zstd_decompress_internal.h — shared types/tables.
#![allow(dead_code)]
use crate::common::allocations::ZSTD_customMem;
use crate::common::xxhash::XXH64_state_t;
use crate::zstd_h::*;
use core::ffi::c_void;

// Base tables (from the header)
pub static LL_base: [u32; 36] = [
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 18, 20, 22, 24, 28, 32, 40, 48, 64,
    0x80, 0x100, 0x200, 0x400, 0x800, 0x1000, 0x2000, 0x4000, 0x8000, 0x10000,
];
pub static OF_base: [u32; 32] = [
    0, 1, 1, 5, 0xD, 0x1D, 0x3D, 0x7D, 0xFD, 0x1FD, 0x3FD, 0x7FD, 0xFFD, 0x1FFD, 0x3FFD, 0x7FFD,
    0xFFFD, 0x1FFFD, 0x3FFFD, 0x7FFFD, 0xFFFFD, 0x1FFFFD, 0x3FFFFD, 0x7FFFFD, 0xFFFFFD, 0x1FFFFFD,
    0x3FFFFFD, 0x7FFFFFD, 0xFFFFFFD, 0x1FFFFFFD, 0x3FFFFFFD, 0x7FFFFFFD,
];
pub static OF_bits: [u8; 32] = [
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
    26, 27, 28, 29, 30, 31,
];
pub static ML_base: [u32; 53] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27,
    28, 29, 30, 31, 32, 33, 34, 35, 37, 39, 41, 43, 47, 51, 59, 67, 83, 99, 0x83, 0x103, 0x203,
    0x403, 0x803, 0x1003, 0x2003, 0x4003, 0x8003, 0x10003,
];

pub const LLFSELog: u32 = 9;
pub const OffFSELog: u32 = 8;
pub const MLFSELog: u32 = 9;
pub const LL_DEFAULTNORMLOG: u32 = 6;
pub const ML_DEFAULTNORMLOG: u32 = 6;
pub const OF_DEFAULTNORMLOG: u32 = 5;

#[inline]
pub const fn seqsymbol_table_size(log: u32) -> usize {
    (1 + (1 << log)) as usize
}

pub const ZSTD_BUILD_FSE_TABLE_WKSP_SIZE: usize =
    2 * (crate::common::zstd_internal::MaxSeq as usize + 1) + (1 << crate::common::zstd_internal::MaxFSELog) + 8;
pub const ZSTD_BUILD_FSE_TABLE_WKSP_SIZE_U32: usize =
    (ZSTD_BUILD_FSE_TABLE_WKSP_SIZE + 3) / 4;
pub const ZSTD_HUFFDTABLE_CAPACITY_LOG: u32 = 12;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ZSTD_seqSymbol_header {
    pub fastMode: u32,
    pub tableLog: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ZSTD_seqSymbol {
    pub nextState: u16,
    pub nbAdditionalBits: u8,
    pub nbBits: u8,
    pub baseValue: u32,
}

pub const HUF_DTABLE_SIZE_LOG12: usize = 1 + (1 << 12);
pub const HUF_DECOMPRESS_WORKSPACE_SIZE: usize = (2 << 10) + (1 << 9);
pub const HUF_DECOMPRESS_WORKSPACE_SIZE_U32: usize = HUF_DECOMPRESS_WORKSPACE_SIZE / 4;

pub type HUF_DTable = u32;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ZSTD_entropyDTables_t {
    pub LLTable: [ZSTD_seqSymbol; 513],  // SEQSYMBOL_TABLE_SIZE(LLFSELog=9) = 1+512
    pub OFTable: [ZSTD_seqSymbol; 257],  // OffFSELog=8 -> 1+256
    pub MLTable: [ZSTD_seqSymbol; 513],  // MLFSELog=9
    pub hufTable: [HUF_DTable; HUF_DTABLE_SIZE_LOG12],
    pub rep: [u32; 3],
    pub workspace: [u32; ZSTD_BUILD_FSE_TABLE_WKSP_SIZE_U32],
}

// ZSTD_dStage
pub const ZSTDds_getFrameHeaderSize: u32 = 0;
pub const ZSTDds_decodeFrameHeader: u32 = 1;
pub const ZSTDds_decodeBlockHeader: u32 = 2;
pub const ZSTDds_decompressBlock: u32 = 3;
pub const ZSTDds_decompressLastBlock: u32 = 4;
pub const ZSTDds_checkChecksum: u32 = 5;
pub const ZSTDds_decodeSkippableHeader: u32 = 6;
pub const ZSTDds_skipFrame: u32 = 7;
pub type ZSTD_dStage = u32;

// ZSTD_dStreamStage
pub const zdss_init: u32 = 0;
pub const zdss_loadHeader: u32 = 1;
pub const zdss_read: u32 = 2;
pub const zdss_load: u32 = 3;
pub const zdss_flush: u32 = 4;
pub type ZSTD_dStreamStage = u32;

// ZSTD_dictUses_e
pub const ZSTD_use_indefinitely: i32 = -1;
pub const ZSTD_dont_use: i32 = 0;
pub const ZSTD_use_once: i32 = 1;
pub type ZSTD_dictUses_e = i32;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ZSTD_DDictHashSet {
    pub ddictPtrTable: *const *const ZSTD_DDict,
    pub ddictPtrTableSize: usize,
    pub ddictPtrCount: usize,
}

pub const ZSTD_DECODER_INTERNAL_BUFFER: usize = 1 << 16;
pub const ZSTD_LBMIN: usize = 64;
pub const ZSTD_LBMAX: usize = 128 << 10;
pub const ZSTD_LITBUFFEREXTRASIZE: usize = ZSTD_DECODER_INTERNAL_BUFFER; // BOUNDED(64, 65536, 131072) = 65536
pub const WILDCOPY_OVERLENGTH: usize = 32;
pub const ZSTD_FRAMEHEADERSIZE_MAX: usize = 18;

// ZSTD_litLocation_e
pub const ZSTD_not_in_dst: u32 = 0;
pub const ZSTD_in_dst: u32 = 1;
pub const ZSTD_split: u32 = 2;
pub type ZSTD_litLocation_e = u32;

// blockType_e reuse
pub use crate::common::zstd_internal::{bt_compressed, bt_raw, bt_rle, bt_reserved};

pub const ZSTD_REP_NUM: usize = 3;

// ZSTD_FrameHeader (public)
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ZSTD_FrameHeader {
    pub frameContentSize: u64,
    pub windowSize: u64,
    pub blockSizeMax: u32,
    pub frameType: u32, // ZSTD_FrameType_e
    pub headerSize: u32,
    pub dictID: u32,
    pub checksumFlag: u32,
    pub _reserved1: u32,
    pub _reserved2: u32,
}

// ZSTD_FrameType_e
pub const ZSTD_frame: u32 = 0;
pub const ZSTD_skippableFrame: u32 = 1;

// Opaque DDict — real definition lives in zstd_ddict.rs
#[repr(C)]
pub struct ZSTD_DDict_s {
    pub dictBuffer: *mut c_void,
    pub dictContent: *const c_void,
    pub dictSize: usize,
    pub entropy: ZSTD_entropyDTables_t,
    pub dictID: u32,
    pub entropyPresent: u32,
    pub cMem: ZSTD_customMem,
}
pub type ZSTD_DDict = ZSTD_DDict_s;

pub type ZSTD_TraceCtx = u64;

#[repr(C)]
pub struct ZSTD_DCtx_s {
    pub LLTptr: *const ZSTD_seqSymbol,
    pub MLTptr: *const ZSTD_seqSymbol,
    pub OFTptr: *const ZSTD_seqSymbol,
    pub HUFptr: *const HUF_DTable,
    pub entropy: ZSTD_entropyDTables_t,
    pub workspace: [u32; HUF_DECOMPRESS_WORKSPACE_SIZE_U32],
    pub previousDstEnd: *const c_void,
    pub prefixStart: *const c_void,
    pub virtualStart: *const c_void,
    pub dictEnd: *const c_void,
    pub expected: usize,
    pub fParams: ZSTD_FrameHeader,
    pub processedCSize: u64,
    pub decodedSize: u64,
    pub bType: u32, // blockType_e
    pub stage: ZSTD_dStage,
    pub litEntropy: u32,
    pub fseEntropy: u32,
    pub xxhState: XXH64_state_t,
    pub headerSize: usize,
    pub format: u32, // ZSTD_format_e
    pub forceIgnoreChecksum: u32,
    pub validateChecksum: u32,
    pub litPtr: *const u8,
    pub customMem: ZSTD_customMem,
    pub litSize: usize,
    pub rleSize: usize,
    pub staticSize: usize,
    pub isFrameDecompression: i32,
    // DYNAMIC_BMI2=0 -> no bmi2 field

    pub ddictLocal: *mut ZSTD_DDict,
    pub ddict: *const ZSTD_DDict,
    pub dictID: u32,
    pub ddictIsCold: i32,
    pub dictUses: ZSTD_dictUses_e,
    pub ddictSet: *mut ZSTD_DDictHashSet,
    pub refMultipleDDicts: u32, // ZSTD_refMultipleDDicts_e
    pub disableHufAsm: i32,
    pub maxBlockSizeParam: i32,

    pub streamStage: ZSTD_dStreamStage,
    pub inBuff: *mut u8,
    pub inBuffSize: usize,
    pub inPos: usize,
    pub maxWindowSize: usize,
    pub outBuff: *mut u8,
    pub outBuffSize: usize,
    pub outStart: usize,
    pub outEnd: usize,
    pub lhSize: usize,
    // ZSTD_LEGACY_SUPPORT >= 1
    pub legacyContext: *mut c_void,
    pub previousLegacyVersion: u32,
    pub legacyVersion: u32,

    pub hostageByte: u32,
    pub noForwardProgress: i32,
    pub outBufferMode: u32, // ZSTD_bufferMode_e
    pub expectedOutBuffer: ZSTD_outBuffer,

    pub litBuffer: *mut u8,
    pub litBufferEnd: *const u8,
    pub litBufferLocation: ZSTD_litLocation_e,
    pub litExtraBuffer: [u8; ZSTD_LITBUFFEREXTRASIZE + WILDCOPY_OVERLENGTH],
    pub headerBuffer: [u8; ZSTD_FRAMEHEADERSIZE_MAX],

    pub oversizedDuration: usize,

    // ZSTD_TRACE=1
    pub traceCtx: ZSTD_TraceCtx,
}
pub type ZSTD_DCtx = ZSTD_DCtx_s;

#[inline]
pub fn zstd_dctx_get_bmi2(_dctx: *const ZSTD_DCtx) -> i32 {
    0
}

