//! Translation of `decompress/zstd_decompress_internal.h` — the decoder's
//! internal constants and the `ZSTD_DCtx` / `ZSTD_DDict` layouts.
//!
//! The C build has `ZSTD_TRACE == 1` (weak symbols are available on x86-64 ELF)
//! and `ZSTD_LEGACY_SUPPORT == 5`, so the `traceCtx` and `legacyContext` fields
//! are present; `DYNAMIC_BMI2 == 0`, so the `bmi2` field is not.
#![allow(dead_code)]
#![allow(non_upper_case_globals)]

use core::ffi::{c_char, c_int, c_uint, c_void};

use crate::allocations::ZSTD_customMem;
use crate::huf::*;
use crate::mem::*;
use crate::xxhash::XXH64_state_t;
use crate::zstd_internal::*;
use crate::zstd_public::*;

pub static LL_base: [U32; (MaxLL + 1) as usize] = [
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 18, 20, 22, 24, 28, 32, 40, 48, 64,
    0x80, 0x100, 0x200, 0x400, 0x800, 0x1000, 0x2000, 0x4000, 0x8000, 0x10000,
];

pub static OF_base: [U32; (MaxOff + 1) as usize] = [
    0, 1, 1, 5, 0xD, 0x1D, 0x3D, 0x7D, 0xFD, 0x1FD, 0x3FD, 0x7FD, 0xFFD, 0x1FFD, 0x3FFD, 0x7FFD,
    0xFFFD, 0x1FFFD, 0x3FFFD, 0x7FFFD, 0xFFFFD, 0x1FFFFD, 0x3FFFFD, 0x7FFFFD, 0xFFFFFD, 0x1FFFFFD,
    0x3FFFFFD, 0x7FFFFFD, 0xFFFFFFD, 0x1FFFFFFD, 0x3FFFFFFD, 0x7FFFFFFD,
];

pub static OF_bits: [u8; (MaxOff + 1) as usize] = [
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
    26, 27, 28, 29, 30, 31,
];

pub static ML_base: [U32; (MaxML + 1) as usize] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27,
    28, 29, 30, 31, 32, 33, 34, 35, 37, 39, 41, 43, 47, 51, 59, 67, 83, 99, 0x83, 0x103, 0x203,
    0x403, 0x803, 0x1003, 0x2003, 0x4003, 0x8003, 0x10003,
];

/// `ZSTD_seqSymbol_header`
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct ZSTD_seqSymbol_header {
    pub fastMode: U32,
    pub tableLog: U32,
}

/// `ZSTD_seqSymbol`
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct ZSTD_seqSymbol {
    pub nextState: U16,
    pub nbAdditionalBits: u8,
    pub nbBits: u8,
    pub baseValue: U32,
}

/// `SEQSYMBOL_TABLE_SIZE(log)`
pub const fn seqsymbol_table_size(log: u32) -> usize {
    1 + (1usize << log)
}

/// `ZSTD_BUILD_FSE_TABLE_WKSP_SIZE`
pub const ZSTD_BUILD_FSE_TABLE_WKSP_SIZE: usize =
    core::mem::size_of::<S16>() * (MaxSeq as usize + 1) + (1usize << MaxFSELog) + 8;
/// `ZSTD_BUILD_FSE_TABLE_WKSP_SIZE_U32`
pub const ZSTD_BUILD_FSE_TABLE_WKSP_SIZE_U32: usize = (ZSTD_BUILD_FSE_TABLE_WKSP_SIZE + 3) / 4;
pub const ZSTD_HUFFDTABLE_CAPACITY_LOG: u32 = 12;

pub const LL_TABLE_SIZE: usize = seqsymbol_table_size(LLFSELog);
pub const OF_TABLE_SIZE: usize = seqsymbol_table_size(OffFSELog);
pub const ML_TABLE_SIZE: usize = seqsymbol_table_size(MLFSELog);
pub const HUF_TABLE_SIZE: usize = huf_dtable_size(ZSTD_HUFFDTABLE_CAPACITY_LOG);

/// `ZSTD_entropyDTables_t`
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ZSTD_entropyDTables_t {
    pub LLTable: [ZSTD_seqSymbol; LL_TABLE_SIZE],
    pub OFTable: [ZSTD_seqSymbol; OF_TABLE_SIZE],
    pub MLTable: [ZSTD_seqSymbol; ML_TABLE_SIZE],
    pub hufTable: [HUF_DTable; HUF_TABLE_SIZE],
    pub rep: [U32; ZSTD_REP_NUM],
    pub workspace: [U32; ZSTD_BUILD_FSE_TABLE_WKSP_SIZE_U32],
}

impl Default for ZSTD_entropyDTables_t {
    fn default() -> Self {
        ZSTD_entropyDTables_t {
            LLTable: [ZSTD_seqSymbol::default(); LL_TABLE_SIZE],
            OFTable: [ZSTD_seqSymbol::default(); OF_TABLE_SIZE],
            MLTable: [ZSTD_seqSymbol::default(); ML_TABLE_SIZE],
            hufTable: [0; HUF_TABLE_SIZE],
            rep: [0; ZSTD_REP_NUM],
            workspace: [0; ZSTD_BUILD_FSE_TABLE_WKSP_SIZE_U32],
        }
    }
}

/// `ZSTD_dStage`
pub type ZSTD_dStage = c_uint;
pub const ZSTDds_getFrameHeaderSize: ZSTD_dStage = 0;
pub const ZSTDds_decodeFrameHeader: ZSTD_dStage = 1;
pub const ZSTDds_decodeBlockHeader: ZSTD_dStage = 2;
pub const ZSTDds_decompressBlock: ZSTD_dStage = 3;
pub const ZSTDds_decompressLastBlock: ZSTD_dStage = 4;
pub const ZSTDds_checkChecksum: ZSTD_dStage = 5;
pub const ZSTDds_decodeSkippableHeader: ZSTD_dStage = 6;
pub const ZSTDds_skipFrame: ZSTD_dStage = 7;

/// `ZSTD_dStreamStage`
pub type ZSTD_dStreamStage = c_uint;
pub const zdss_init: ZSTD_dStreamStage = 0;
pub const zdss_loadHeader: ZSTD_dStreamStage = 1;
pub const zdss_read: ZSTD_dStreamStage = 2;
pub const zdss_load: ZSTD_dStreamStage = 3;
pub const zdss_flush: ZSTD_dStreamStage = 4;

/// `ZSTD_dictUses_e`
pub type ZSTD_dictUses_e = c_int;
pub const ZSTD_use_indefinitely: ZSTD_dictUses_e = -1;
pub const ZSTD_dont_use: ZSTD_dictUses_e = 0;
pub const ZSTD_use_once: ZSTD_dictUses_e = 1;

/// `ZSTD_DDictHashSet`
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ZSTD_DDictHashSet {
    pub ddictPtrTable: *mut *const ZSTD_DDict,
    pub ddictPtrTableSize: usize,
    pub ddictPtrCount: usize,
}

pub const ZSTD_DECODER_INTERNAL_BUFFER: usize = 1 << 16;
pub const ZSTD_LBMIN: usize = 64;
pub const ZSTD_LBMAX: usize = 128 << 10;

/// `ZSTD_LITBUFFEREXTRASIZE` == `BOUNDED(ZSTD_LBMIN, ZSTD_DECODER_INTERNAL_BUFFER, ZSTD_LBMAX)`
pub const ZSTD_LITBUFFEREXTRASIZE: usize = {
    let v = if ZSTD_DECODER_INTERNAL_BUFFER < ZSTD_LBMAX {
        ZSTD_DECODER_INTERNAL_BUFFER
    } else {
        ZSTD_LBMAX
    };
    if ZSTD_LBMIN > v {
        ZSTD_LBMIN
    } else {
        v
    }
};

/// `ZSTD_litLocation_e`
pub type ZSTD_litLocation_e = c_uint;
pub const ZSTD_not_in_dst: ZSTD_litLocation_e = 0;
pub const ZSTD_in_dst: ZSTD_litLocation_e = 1;
pub const ZSTD_split: ZSTD_litLocation_e = 2;

/// `struct ZSTD_DDict_s`
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ZSTD_DDict {
    pub dictBuffer: *mut c_void,
    pub dictContent: *const c_void,
    pub dictSize: usize,
    pub entropy: ZSTD_entropyDTables_t,
    pub dictID: U32,
    pub entropyPresent: U32,
    pub cMem: ZSTD_customMem,
}

/// `struct ZSTD_DCtx_s`
#[repr(C)]
pub struct ZSTD_DCtx {
    pub LLTptr: *const ZSTD_seqSymbol,
    pub MLTptr: *const ZSTD_seqSymbol,
    pub OFTptr: *const ZSTD_seqSymbol,
    pub HUFptr: *const HUF_DTable,
    pub entropy: ZSTD_entropyDTables_t,
    pub workspace: [U32; HUF_DECOMPRESS_WORKSPACE_SIZE_U32],
    pub previousDstEnd: *const c_void,
    pub prefixStart: *const c_void,
    pub virtualStart: *const c_void,
    pub dictEnd: *const c_void,
    pub expected: usize,
    pub fParams: ZSTD_FrameHeader,
    pub processedCSize: U64,
    pub decodedSize: U64,
    pub bType: c_uint,
    pub stage: ZSTD_dStage,
    pub litEntropy: U32,
    pub fseEntropy: U32,
    pub xxhState: XXH64_state_t,
    pub headerSize: usize,
    pub format: ZSTD_format_e,
    pub forceIgnoreChecksum: ZSTD_forceIgnoreChecksum_e,
    pub validateChecksum: U32,
    pub litPtr: *const u8,
    pub customMem: ZSTD_customMem,
    pub litSize: usize,
    pub rleSize: usize,
    pub staticSize: usize,
    pub isFrameDecompression: c_int,

    /* dictionary */
    pub ddictLocal: *mut ZSTD_DDict,
    pub ddict: *const ZSTD_DDict,
    pub dictID: U32,
    pub ddictIsCold: c_int,
    pub dictUses: ZSTD_dictUses_e,
    pub ddictSet: *mut ZSTD_DDictHashSet,
    pub refMultipleDDicts: ZSTD_refMultipleDDicts_e,
    pub disableHufAsm: c_int,
    pub maxBlockSizeParam: c_int,

    /* streaming */
    pub streamStage: ZSTD_dStreamStage,
    pub inBuff: *mut c_char,
    pub inBuffSize: usize,
    pub inPos: usize,
    pub maxWindowSize: usize,
    pub outBuff: *mut c_char,
    pub outBuffSize: usize,
    pub outStart: usize,
    pub outEnd: usize,
    pub lhSize: usize,
    /* ZSTD_LEGACY_SUPPORT >= 1 */
    pub legacyContext: *mut c_void,
    pub previousLegacyVersion: U32,
    pub legacyVersion: U32,
    pub hostageByte: U32,
    pub noForwardProgress: c_int,
    pub outBufferMode: c_uint,
    pub expectedOutBuffer: ZSTD_outBuffer,

    /* workspace */
    pub litBuffer: *mut u8,
    pub litBufferEnd: *const u8,
    pub litBufferLocation: ZSTD_litLocation_e,
    pub litExtraBuffer: [u8; ZSTD_LITBUFFEREXTRASIZE + WILDCOPY_OVERLENGTH],
    pub headerBuffer: [u8; ZSTD_FRAMEHEADERSIZE_MAX],

    pub oversizedDuration: usize,

    /* ZSTD_TRACE == 1 */
    pub traceCtx: u64,
}

/// `ZSTD_DCtx_get_bmi2()` — `DYNAMIC_BMI2 == 0`, so always 0.
#[inline(always)]
pub fn zstd_dctx_get_bmi2(_dctx: *const ZSTD_DCtx) -> c_int {
    0
}
