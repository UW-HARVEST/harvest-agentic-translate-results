//! Rust transliteration of `c_src/src/decompress/zstd_decompress_internal.h`.
//!
//! objects and definitions shared within lib/decompress modules.
//!
//! Build configuration: DYNAMIC_BMI2=0, ZSTD_TRACE=0,
//! ZSTD_LEGACY_SUPPORT=5 (>=1), no ZSTD_MULTITHREAD,
//! no FUZZING_BUILD_MODE_UNSAFE_FOR_PRODUCTION.
#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]

use core::ffi::c_void;

use crate::common::huf::*;
use crate::common::mem::*;
use crate::common::xxhash::XXH64_state_t;
use crate::common::zstd_h::*;
use crate::common::zstd_internal::*;

use super::zstd_ddict::ZSTD_DDict;

/*-*******************************************************
 *  Constants
 *********************************************************/
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

/*-*******************************************************
 *  Decompression types
 *********************************************************/
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ZSTD_seqSymbol_header {
    pub fastMode: U32,
    pub tableLog: U32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ZSTD_seqSymbol {
    pub nextState: U16,
    pub nbAdditionalBits: BYTE,
    pub nbBits: BYTE,
    pub baseValue: U32,
}

/// `#define SEQSYMBOL_TABLE_SIZE(log)   (1 + (1 << (log)))`
#[inline(always)]
pub const fn SEQSYMBOL_TABLE_SIZE(log: u32) -> usize {
    (1 + (1u32 << log)) as usize
}

/// `sizeof(S16) * (MaxSeq + 1) + (1u << MaxFSELog) + sizeof(U64)`
pub const ZSTD_BUILD_FSE_TABLE_WKSP_SIZE: size_t = core::mem::size_of::<S16>() * (MaxSeq as size_t + 1)
    + (1usize << MaxFSELog)
    + core::mem::size_of::<U64>();
/// `(ZSTD_BUILD_FSE_TABLE_WKSP_SIZE + sizeof(U32) - 1) / sizeof(U32)`
pub const ZSTD_BUILD_FSE_TABLE_WKSP_SIZE_U32: size_t =
    (ZSTD_BUILD_FSE_TABLE_WKSP_SIZE + core::mem::size_of::<U32>() - 1) / core::mem::size_of::<U32>();
pub const ZSTD_HUFFDTABLE_CAPACITY_LOG: u32 = 12;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ZSTD_entropyDTables_t {
    /* Note : Space reserved for FSE Tables */
    pub LLTable: [ZSTD_seqSymbol; SEQSYMBOL_TABLE_SIZE(LLFSELog)],
    /* is also used as temporary workspace while building hufTable during DDict creation */
    pub OFTable: [ZSTD_seqSymbol; SEQSYMBOL_TABLE_SIZE(OffFSELog)],
    /* and therefore must be at least HUF_DECOMPRESS_WORKSPACE_SIZE large */
    pub MLTable: [ZSTD_seqSymbol; SEQSYMBOL_TABLE_SIZE(MLFSELog)],
    /* can accommodate HUF_decompress4X */
    pub hufTable: [HUF_DTable; HUF_DTABLE_SIZE(ZSTD_HUFFDTABLE_CAPACITY_LOG)],
    pub rep: [U32; ZSTD_REP_NUM],
    pub workspace: [U32; ZSTD_BUILD_FSE_TABLE_WKSP_SIZE_U32],
}

pub type ZSTD_dStage = core::ffi::c_uint;
pub const ZSTDds_getFrameHeaderSize: ZSTD_dStage = 0;
pub const ZSTDds_decodeFrameHeader: ZSTD_dStage = 1;
pub const ZSTDds_decodeBlockHeader: ZSTD_dStage = 2;
pub const ZSTDds_decompressBlock: ZSTD_dStage = 3;
pub const ZSTDds_decompressLastBlock: ZSTD_dStage = 4;
pub const ZSTDds_checkChecksum: ZSTD_dStage = 5;
pub const ZSTDds_decodeSkippableHeader: ZSTD_dStage = 6;
pub const ZSTDds_skipFrame: ZSTD_dStage = 7;

pub type ZSTD_dStreamStage = core::ffi::c_uint;
pub const zdss_init: ZSTD_dStreamStage = 0;
pub const zdss_loadHeader: ZSTD_dStreamStage = 1;
pub const zdss_read: ZSTD_dStreamStage = 2;
pub const zdss_load: ZSTD_dStreamStage = 3;
pub const zdss_flush: ZSTD_dStreamStage = 4;

pub type ZSTD_dictUses_e = core::ffi::c_int;
pub const ZSTD_use_indefinitely: ZSTD_dictUses_e = -1; /* Use the dictionary indefinitely */
pub const ZSTD_dont_use: ZSTD_dictUses_e = 0; /* Do not use the dictionary (if one exists free it) */
pub const ZSTD_use_once: ZSTD_dictUses_e = 1; /* Use the dictionary once and set to ZSTD_dont_use */

/* Hashset for storing references to multiple ZSTD_DDict within ZSTD_DCtx */
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ZSTD_DDictHashSet {
    pub ddictPtrTable: *const *const ZSTD_DDict,
    pub ddictPtrTableSize: size_t,
    pub ddictPtrCount: size_t,
}

/// `#define ZSTD_DECODER_INTERNAL_BUFFER  (1 << 16)`
pub const ZSTD_DECODER_INTERNAL_BUFFER: size_t = 1 << 16;

pub const ZSTD_LBMIN: size_t = 64;
pub const ZSTD_LBMAX: size_t = 128 << 10;

/// extra buffer, compensates when dst is not large enough to store litBuffer
/// `BOUNDED(ZSTD_LBMIN, ZSTD_DECODER_INTERNAL_BUFFER, ZSTD_LBMAX)`
pub const ZSTD_LITBUFFEREXTRASIZE: size_t = {
    let val = ZSTD_DECODER_INTERNAL_BUFFER;
    // MAX(ZSTD_LBMIN, MIN(val, ZSTD_LBMAX))
    let inner = if val < ZSTD_LBMAX { val } else { ZSTD_LBMAX };
    if ZSTD_LBMIN > inner { ZSTD_LBMIN } else { inner }
};

pub type ZSTD_litLocation_e = core::ffi::c_uint;
pub const ZSTD_not_in_dst: ZSTD_litLocation_e = 0; /* Stored entirely within litExtraBuffer */
pub const ZSTD_in_dst: ZSTD_litLocation_e = 1; /* Stored entirely within dst (in memory after current output write) */
pub const ZSTD_split: ZSTD_litLocation_e = 2; /* Split between litExtraBuffer and dst */

#[repr(C)]
pub struct ZSTD_DCtx_s {
    pub LLTptr: *const ZSTD_seqSymbol,
    pub MLTptr: *const ZSTD_seqSymbol,
    pub OFTptr: *const ZSTD_seqSymbol,
    pub HUFptr: *const HUF_DTable,
    pub entropy: ZSTD_entropyDTables_t,
    /* space needed when building huffman tables */
    pub workspace: [U32; HUF_DECOMPRESS_WORKSPACE_SIZE_U32],
    pub previousDstEnd: *const c_void, /* detect continuity */
    pub prefixStart: *const c_void,    /* start of current segment */
    pub virtualStart: *const c_void,   /* virtual start of previous segment if it was just before current one */
    pub dictEnd: *const c_void,        /* end of previous segment */
    pub expected: size_t,
    pub fParams: ZSTD_FrameHeader,
    pub processedCSize: U64,
    pub decodedSize: U64,
    /* used in ZSTD_decompressContinue(), store blockType between block header decoding and block decompression stages */
    pub bType: blockType_e,
    pub stage: ZSTD_dStage,
    pub litEntropy: U32,
    pub fseEntropy: U32,
    pub xxhState: XXH64_state_t,
    pub headerSize: size_t,
    pub format: ZSTD_format_e,
    /* User specified: if == 1, will ignore checksums in compressed frame. Default == 0 */
    pub forceIgnoreChecksum: ZSTD_forceIgnoreChecksum_e,
    /* if == 1, will validate checksum. Is == 1 if (fParams.checksumFlag == 1) and (forceIgnoreChecksum == 0). */
    pub validateChecksum: U32,
    pub litPtr: *const BYTE,
    pub customMem: crate::common::zstd_internal::ZSTD_customMem,
    pub litSize: size_t,
    pub rleSize: size_t,
    pub staticSize: size_t,
    pub isFrameDecompression: core::ffi::c_int,
    /* DYNAMIC_BMI2 == 0 : `int bmi2;` field omitted */

    /* dictionary */
    pub ddictLocal: *mut ZSTD_DDict,
    /* set by ZSTD_initDStream_usingDDict(), or ZSTD_DCtx_refDDict() */
    pub ddict: *const ZSTD_DDict,
    pub dictID: U32,
    /* if == 1 : dictionary is "new" for working context, and presumed "cold" (not in cpu cache) */
    pub ddictIsCold: core::ffi::c_int,
    pub dictUses: ZSTD_dictUses_e,
    /* Hash set for multiple ddicts */
    pub ddictSet: *mut ZSTD_DDictHashSet,
    /* User specified: if == 1, will allow references to multiple DDicts. Default == 0 (disabled) */
    pub refMultipleDDicts: ZSTD_refMultipleDDicts_e,
    pub disableHufAsm: core::ffi::c_int,
    pub maxBlockSizeParam: core::ffi::c_int,

    /* streaming */
    pub streamStage: ZSTD_dStreamStage,
    pub inBuff: *mut core::ffi::c_char,
    pub inBuffSize: size_t,
    pub inPos: size_t,
    pub maxWindowSize: size_t,
    pub outBuff: *mut core::ffi::c_char,
    pub outBuffSize: size_t,
    pub outStart: size_t,
    pub outEnd: size_t,
    pub lhSize: size_t,
    /* ZSTD_LEGACY_SUPPORT == 5 (>=1) : these fields present */
    pub legacyContext: *mut c_void,
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
    /* literal buffer can be split between storage within dst and within this scratch buffer */
    pub litExtraBuffer: [BYTE; ZSTD_LITBUFFEREXTRASIZE + WILDCOPY_OVERLENGTH as size_t],
    pub headerBuffer: [BYTE; ZSTD_FRAMEHEADERSIZE_MAX],

    pub oversizedDuration: size_t,
    /* FUZZING_BUILD_MODE_UNSAFE_FOR_PRODUCTION not defined : fields omitted */
    /* ZSTD_TRACE == 1 on this platform (GCC/ELF has weak symbols), so the
     * traceCtx field IS compiled in. */
    pub traceCtx: ZSTD_TraceCtx,
}
/* typedef'd to ZSTD_DCtx within "zstd.h" */

pub type ZSTD_TraceCtx = u64;
pub type ZSTD_DCtx = ZSTD_DCtx_s;
pub type ZSTD_DStream = ZSTD_DCtx;

#[inline(always)]
pub unsafe fn ZSTD_DCtx_get_bmi2(dctx: *const ZSTD_DCtx_s) -> core::ffi::c_int {
    /* DYNAMIC_BMI2 == 0 */
    let _ = dctx;
    0
}

/*-*******************************************************
 *  Shared internal functions
 *
 *  ZSTD_loadDEntropy() is defined in zstd_decompress.c.
 *  ZSTD_checkContinuity() is defined in zstd_decompress.c.
 *  ZSTD_buildFSETable() / ZSTD_decodeSeqHeaders() are defined elsewhere.
 *  They are declared (not defined) in the header, so nothing to emit here.
 *********************************************************/
