//! Translation of decompress/zstd_decompress_internal.h
#![allow(
    non_snake_case,
    dead_code,
    non_upper_case_globals,
    non_camel_case_types,
    unused_variables
)]

use crate::huf::*;
use crate::mem::*;
use crate::xxhash::*;
use crate::zstd_h::*;
use crate::zstd_internal::*;

pub static LL_base: [U32; (MaxLL + 1) as usize] = [
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 18, 20, 22, 24, 28, 32, 40, 48, 64,
    0x80, 0x100, 0x200, 0x400, 0x800, 0x1000, 0x2000, 0x4000, 0x8000, 0x10000,
];

pub static OF_base: [U32; (MaxOff + 1) as usize] = [
    0, 1, 1, 5, 0xD, 0x1D, 0x3D, 0x7D, 0xFD, 0x1FD, 0x3FD, 0x7FD, 0xFFD, 0x1FFD, 0x3FFD, 0x7FFD,
    0xFFFD, 0x1FFFD, 0x3FFFD, 0x7FFFD, 0xFFFFD, 0x1FFFFD, 0x3FFFFD, 0x7FFFFD, 0xFFFFFD, 0x1FFFFFD,
    0x3FFFFFD, 0x7FFFFFD, 0xFFFFFFD, 0x1FFFFFFD, 0x3FFFFFFD, 0x7FFFFFFD,
];

pub static OF_bits: [U8; (MaxOff + 1) as usize] = [
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
    26, 27, 28, 29, 30, 31,
];

pub static ML_base: [U32; (MaxML + 1) as usize] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27,
    28, 29, 30, 31, 32, 33, 34, 35, 37, 39, 41, 43, 47, 51, 59, 67, 83, 99, 0x83, 0x103, 0x203,
    0x403, 0x803, 0x1003, 0x2003, 0x4003, 0x8003, 0x10003,
];

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct ZSTD_seqSymbol_header {
    pub fastMode: U32,
    pub tableLog: U32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct ZSTD_seqSymbol {
    pub nextState: U16,
    pub nbAdditionalBits: BYTE,
    pub nbBits: BYTE,
    pub baseValue: U32,
}

#[inline(always)]
pub const fn SEQSYMBOL_TABLE_SIZE(log: u32) -> usize {
    1 + (1usize << log)
}

pub const ZSTD_BUILD_FSE_TABLE_WKSP_SIZE: usize = core::mem::size_of::<S16>()
    * (MaxSeq as usize + 1)
    + (1usize << MaxFSELog)
    + core::mem::size_of::<U64>();
pub const ZSTD_BUILD_FSE_TABLE_WKSP_SIZE_U32: usize =
    (ZSTD_BUILD_FSE_TABLE_WKSP_SIZE + core::mem::size_of::<U32>() - 1)
        / core::mem::size_of::<U32>();
pub const ZSTD_HUFFDTABLE_CAPACITY_LOG: u32 = 12;

pub const LLTABLE_SIZE: usize = SEQSYMBOL_TABLE_SIZE(LLFSELog);
pub const OFTABLE_SIZE: usize = SEQSYMBOL_TABLE_SIZE(OffFSELog);
pub const MLTABLE_SIZE: usize = SEQSYMBOL_TABLE_SIZE(MLFSELog);
pub const HUFTABLE_SIZE: usize = HUF_DTABLE_SIZE(ZSTD_HUFFDTABLE_CAPACITY_LOG);

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ZSTD_entropyDTables_t {
    pub LLTable: [ZSTD_seqSymbol; LLTABLE_SIZE],
    pub OFTable: [ZSTD_seqSymbol; OFTABLE_SIZE],
    pub MLTable: [ZSTD_seqSymbol; MLTABLE_SIZE],
    pub hufTable: [HUF_DTable; HUFTABLE_SIZE],
    pub rep: [U32; ZSTD_REP_NUM],
    pub workspace: [U32; ZSTD_BUILD_FSE_TABLE_WKSP_SIZE_U32],
}

impl Default for ZSTD_entropyDTables_t {
    fn default() -> Self {
        ZSTD_entropyDTables_t {
            LLTable: [ZSTD_seqSymbol::default(); LLTABLE_SIZE],
            OFTable: [ZSTD_seqSymbol::default(); OFTABLE_SIZE],
            MLTable: [ZSTD_seqSymbol::default(); MLTABLE_SIZE],
            hufTable: [0; HUFTABLE_SIZE],
            rep: [0; ZSTD_REP_NUM],
            workspace: [0; ZSTD_BUILD_FSE_TABLE_WKSP_SIZE_U32],
        }
    }
}

pub type ZSTD_dStage = core::ffi::c_int;
pub const ZSTDds_getFrameHeaderSize: ZSTD_dStage = 0;
pub const ZSTDds_decodeFrameHeader: ZSTD_dStage = 1;
pub const ZSTDds_decodeBlockHeader: ZSTD_dStage = 2;
pub const ZSTDds_decompressBlock: ZSTD_dStage = 3;
pub const ZSTDds_decompressLastBlock: ZSTD_dStage = 4;
pub const ZSTDds_checkChecksum: ZSTD_dStage = 5;
pub const ZSTDds_decodeSkippableHeader: ZSTD_dStage = 6;
pub const ZSTDds_skipFrame: ZSTD_dStage = 7;

pub type ZSTD_dStreamStage = core::ffi::c_int;
pub const zdss_init: ZSTD_dStreamStage = 0;
pub const zdss_loadHeader: ZSTD_dStreamStage = 1;
pub const zdss_read: ZSTD_dStreamStage = 2;
pub const zdss_load: ZSTD_dStreamStage = 3;
pub const zdss_flush: ZSTD_dStreamStage = 4;

pub type ZSTD_dictUses_e = core::ffi::c_int;
pub const ZSTD_use_indefinitely: ZSTD_dictUses_e = -1;
pub const ZSTD_dont_use: ZSTD_dictUses_e = 0;
pub const ZSTD_use_once: ZSTD_dictUses_e = 1;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ZSTD_DDictHashSet {
    pub ddictPtrTable: *mut *const ZSTD_DDict,
    pub ddictPtrTableSize: usize,
    pub ddictPtrCount: usize,
}
impl Default for ZSTD_DDictHashSet {
    fn default() -> Self {
        ZSTD_DDictHashSet {
            ddictPtrTable: core::ptr::null_mut(),
            ddictPtrTableSize: 0,
            ddictPtrCount: 0,
        }
    }
}

pub const ZSTD_DECODER_INTERNAL_BUFFER: usize = 1 << 16;
pub const ZSTD_LBMIN: usize = 64;
pub const ZSTD_LBMAX: usize = 128 << 10;
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

pub type ZSTD_litLocation_e = core::ffi::c_int;
pub const ZSTD_not_in_dst: ZSTD_litLocation_e = 0;
pub const ZSTD_in_dst: ZSTD_litLocation_e = 1;
pub const ZSTD_split: ZSTD_litLocation_e = 2;

/// `ZSTD_DDict` is defined in decompress/zstd_ddict.c; declared here so all
/// modules agree on its layout.
#[repr(C)]
pub struct ZSTD_DDict {
    pub dictBuffer: *mut core::ffi::c_void,
    pub dictContent: *const core::ffi::c_void,
    pub dictSize: usize,
    pub entropy: ZSTD_entropyDTables_t,
    pub dictID: U32,
    pub entropyPresent: U32,
    pub cMem: ZSTD_customMem,
}

#[repr(C)]
pub struct ZSTD_DCtx {
    pub LLTptr: *const ZSTD_seqSymbol,
    pub MLTptr: *const ZSTD_seqSymbol,
    pub OFTptr: *const ZSTD_seqSymbol,
    pub HUFptr: *const HUF_DTable,
    pub entropy: ZSTD_entropyDTables_t,
    pub workspace: [U32; HUF_DECOMPRESS_WORKSPACE_SIZE_U32],
    pub previousDstEnd: *const core::ffi::c_void,
    pub prefixStart: *const core::ffi::c_void,
    pub virtualStart: *const core::ffi::c_void,
    pub dictEnd: *const core::ffi::c_void,
    pub expected: usize,
    pub fParams: ZSTD_FrameHeader,
    pub processedCSize: U64,
    pub decodedSize: U64,
    pub bType: blockType_e,
    pub stage: ZSTD_dStage,
    pub litEntropy: U32,
    pub fseEntropy: U32,
    pub xxhState: XXH64_state_t,
    pub headerSize: usize,
    pub format: ZSTD_format_e,
    pub forceIgnoreChecksum: ZSTD_forceIgnoreChecksum_e,
    pub validateChecksum: U32,
    pub litPtr: *const BYTE,
    pub customMem: ZSTD_customMem,
    pub litSize: usize,
    pub rleSize: usize,
    pub staticSize: usize,
    pub isFrameDecompression: core::ffi::c_int,

    /* dictionary */
    pub ddictLocal: *mut ZSTD_DDict,
    pub ddict: *const ZSTD_DDict,
    pub dictID: U32,
    pub ddictIsCold: core::ffi::c_int,
    pub dictUses: ZSTD_dictUses_e,
    pub ddictSet: *mut ZSTD_DDictHashSet,
    pub refMultipleDDicts: ZSTD_refMultipleDDicts_e,
    pub disableHufAsm: core::ffi::c_int,
    pub maxBlockSizeParam: core::ffi::c_int,

    /* streaming */
    pub streamStage: ZSTD_dStreamStage,
    pub inBuff: *mut core::ffi::c_char,
    pub inBuffSize: usize,
    pub inPos: usize,
    pub maxWindowSize: usize,
    pub outBuff: *mut core::ffi::c_char,
    pub outBuffSize: usize,
    pub outStart: usize,
    pub outEnd: usize,
    pub lhSize: usize,
    pub legacyContext: *mut core::ffi::c_void,
    pub previousLegacyVersion: U32,
    pub legacyVersion: U32,
    pub hostageByte: U32,
    pub noForwardProgress: core::ffi::c_int,
    pub outBufferMode: ZSTD_bufferMode_e,
    pub expectedOutBuffer: ZSTD_outBuffer,

    /* workspace */
    pub litBuffer: *mut BYTE,
    pub litBufferEnd: *const BYTE,
    pub litBufferLocation: ZSTD_litLocation_e,
    pub litExtraBuffer: [BYTE; ZSTD_LITBUFFEREXTRASIZE + WILDCOPY_OVERLENGTH as usize],
    pub headerBuffer: [BYTE; ZSTD_FRAMEHEADERSIZE_MAX],

    pub oversizedDuration: usize,

    /* Tracing.
     * `ZSTD_TRACE` is 1 in the reference build (GNUC + ELF + x86_64 => weak
     * symbols are available), so `ZSTD_DCtx_s` carries this field. The
     * `ZSTD_trace_decompress_*` hooks are *undefined* weak symbols in the
     * reference shared library, so they resolve to NULL and every trace code
     * path is dead; `traceCtx` therefore stays 0. The field is reproduced
     * because `sizeof(ZSTD_DCtx)` is observable through ZSTD_estimateDCtxSize(),
     * ZSTD_estimateDStreamSize*(), ZSTD_sizeof_DCtx()/ZSTD_sizeof_DStream() and
     * the `sizeof(ZSTD_DCtx) - ...` arithmetic inside ZSTD_copyDCtx(). */
    pub traceCtx: crate::zstd_compress_internal::ZSTD_TraceCtx,
}

pub type ZSTD_DStream = ZSTD_DCtx;

#[inline(always)]
pub unsafe fn ZSTD_DCtx_get_bmi2(_dctx: *const ZSTD_DCtx) -> core::ffi::c_int {
    0
}
