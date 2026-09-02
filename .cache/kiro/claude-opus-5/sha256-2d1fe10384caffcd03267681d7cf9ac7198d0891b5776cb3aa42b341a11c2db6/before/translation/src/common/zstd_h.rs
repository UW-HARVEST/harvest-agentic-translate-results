//! Translation of the public `zstd.h` API: types, constants, enums and
//! function-like-macro equivalents. NO function bodies live here; other
//! modules implement the functions.

#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]

use super::mem::*;
use core::ffi::{c_char, c_int, c_uint, c_ulonglong, c_void};

// ---------------------------------------------------------------------------
// Opaque / forward context types.
//
// ZSTD_CCtx / ZSTD_DCtx / ZSTD_CDict / ZSTD_DDict / ZSTD_CCtx_params are the
// opaque structs `ZSTD_CCtx_s`, `ZSTD_DCtx_s`, `ZSTD_CDict_s`, `ZSTD_DDict_s`
// and `ZSTD_CCtx_params_s`. Their bodies are defined in the compress /
// decompress internals and are imported from there by their users. We do NOT
// create placeholder structs for them here.
//
// POOL_ctx lives in `super::pool`. ZSTD_threadPool is an alias of it.
use super::pool::POOL_ctx;
pub type ZSTD_threadPool = POOL_ctx;

// ---------------------------------------------------------------------------
// Version
// ---------------------------------------------------------------------------
pub const ZSTD_VERSION_MAJOR: c_uint = 1;
pub const ZSTD_VERSION_MINOR: c_uint = 5;
pub const ZSTD_VERSION_RELEASE: c_uint = 7;
pub const ZSTD_VERSION_NUMBER: c_uint =
    ZSTD_VERSION_MAJOR * 100 * 100 + ZSTD_VERSION_MINOR * 100 + ZSTD_VERSION_RELEASE;

// ZSTD_LIB_VERSION / ZSTD_VERSION_STRING are textual stringification macros;
// the resulting string constant "1.5.7" is provided for convenience.
pub const ZSTD_VERSION_STRING: &str = "1.5.7";

// ---------------------------------------------------------------------------
// Default constant
// ---------------------------------------------------------------------------
pub const ZSTD_CLEVEL_DEFAULT: c_int = 3;

// ---------------------------------------------------------------------------
// Magic numbers / block size
// ---------------------------------------------------------------------------
pub const ZSTD_MAGICNUMBER: c_uint = 0xFD2FB528; /* valid since v0.8.0 */
pub const ZSTD_MAGIC_DICTIONARY: c_uint = 0xEC30A437; /* valid since v0.7.0 */
pub const ZSTD_MAGIC_SKIPPABLE_START: c_uint = 0x184D2A50;
pub const ZSTD_MAGIC_SKIPPABLE_MASK: c_uint = 0xFFFFFFF0;

pub const ZSTD_BLOCKSIZELOG_MAX: c_uint = 17;
pub const ZSTD_BLOCKSIZE_MAX: c_uint = 1 << ZSTD_BLOCKSIZELOG_MAX;

// ---------------------------------------------------------------------------
// Content size sentinels (unsigned long long)
// ---------------------------------------------------------------------------
pub const ZSTD_CONTENTSIZE_UNKNOWN: u64 = 0u64.wrapping_sub(1); /* (0ULL - 1) */
pub const ZSTD_CONTENTSIZE_ERROR: u64 = 0u64.wrapping_sub(2); /* (0ULL - 2) */

// ---------------------------------------------------------------------------
// Compress bound
// ---------------------------------------------------------------------------
// #define ZSTD_MAX_INPUT_SIZE ((sizeof(size_t)==8) ? 0xFF00FF00FF00FF00ULL : 0xFF00FF00U)
pub const ZSTD_MAX_INPUT_SIZE: u64 = if core::mem::size_of::<size_t>() == 8 {
    0xFF00FF00FF00FF00u64
} else {
    0xFF00FF00u64
};

// #define ZSTD_COMPRESSBOUND(srcSize) ...
#[inline(always)]
pub const fn ZSTD_COMPRESSBOUND(srcSize: size_t) -> size_t {
    if (srcSize as u64) >= ZSTD_MAX_INPUT_SIZE {
        0
    } else {
        srcSize
            + (srcSize >> 8)
            + (if srcSize < (128 << 10) {
                ((128 << 10) - srcSize) >> 11 /* margin, from 64 to 0 */
            } else {
                0
            })
    }
}

// ---------------------------------------------------------------------------
// Compression strategies
// ---------------------------------------------------------------------------
pub type ZSTD_strategy = c_uint;
pub const ZSTD_fast: ZSTD_strategy = 1;
pub const ZSTD_dfast: ZSTD_strategy = 2;
pub const ZSTD_greedy: ZSTD_strategy = 3;
pub const ZSTD_lazy: ZSTD_strategy = 4;
pub const ZSTD_lazy2: ZSTD_strategy = 5;
pub const ZSTD_btlazy2: ZSTD_strategy = 6;
pub const ZSTD_btopt: ZSTD_strategy = 7;
pub const ZSTD_btultra: ZSTD_strategy = 8;
pub const ZSTD_btultra2: ZSTD_strategy = 9;

// ---------------------------------------------------------------------------
// ZSTD_cParameter
// ---------------------------------------------------------------------------
pub type ZSTD_cParameter = c_uint;
pub const ZSTD_c_compressionLevel: ZSTD_cParameter = 100;
pub const ZSTD_c_windowLog: ZSTD_cParameter = 101;
pub const ZSTD_c_hashLog: ZSTD_cParameter = 102;
pub const ZSTD_c_chainLog: ZSTD_cParameter = 103;
pub const ZSTD_c_searchLog: ZSTD_cParameter = 104;
pub const ZSTD_c_minMatch: ZSTD_cParameter = 105;
pub const ZSTD_c_targetLength: ZSTD_cParameter = 106;
pub const ZSTD_c_strategy: ZSTD_cParameter = 107;
pub const ZSTD_c_targetCBlockSize: ZSTD_cParameter = 130;
pub const ZSTD_c_enableLongDistanceMatching: ZSTD_cParameter = 160;
pub const ZSTD_c_ldmHashLog: ZSTD_cParameter = 161;
pub const ZSTD_c_ldmMinMatch: ZSTD_cParameter = 162;
pub const ZSTD_c_ldmBucketSizeLog: ZSTD_cParameter = 163;
pub const ZSTD_c_ldmHashRateLog: ZSTD_cParameter = 164;
pub const ZSTD_c_contentSizeFlag: ZSTD_cParameter = 200;
pub const ZSTD_c_checksumFlag: ZSTD_cParameter = 201;
pub const ZSTD_c_dictIDFlag: ZSTD_cParameter = 202;
pub const ZSTD_c_nbWorkers: ZSTD_cParameter = 400;
pub const ZSTD_c_jobSize: ZSTD_cParameter = 401;
pub const ZSTD_c_overlapLog: ZSTD_cParameter = 402;
pub const ZSTD_c_experimentalParam1: ZSTD_cParameter = 500;
pub const ZSTD_c_experimentalParam2: ZSTD_cParameter = 10;
pub const ZSTD_c_experimentalParam3: ZSTD_cParameter = 1000;
pub const ZSTD_c_experimentalParam4: ZSTD_cParameter = 1001;
pub const ZSTD_c_experimentalParam5: ZSTD_cParameter = 1002;
/* was ZSTD_c_experimentalParam6=1003; is now ZSTD_c_targetCBlockSize */
pub const ZSTD_c_experimentalParam7: ZSTD_cParameter = 1004;
pub const ZSTD_c_experimentalParam8: ZSTD_cParameter = 1005;
pub const ZSTD_c_experimentalParam9: ZSTD_cParameter = 1006;
pub const ZSTD_c_experimentalParam10: ZSTD_cParameter = 1007;
pub const ZSTD_c_experimentalParam11: ZSTD_cParameter = 1008;
pub const ZSTD_c_experimentalParam12: ZSTD_cParameter = 1009;
pub const ZSTD_c_experimentalParam13: ZSTD_cParameter = 1010;
pub const ZSTD_c_experimentalParam14: ZSTD_cParameter = 1011;
pub const ZSTD_c_experimentalParam15: ZSTD_cParameter = 1012;
pub const ZSTD_c_experimentalParam16: ZSTD_cParameter = 1013;
pub const ZSTD_c_experimentalParam17: ZSTD_cParameter = 1014;
pub const ZSTD_c_experimentalParam18: ZSTD_cParameter = 1015;
pub const ZSTD_c_experimentalParam19: ZSTD_cParameter = 1016;
pub const ZSTD_c_experimentalParam20: ZSTD_cParameter = 1017;

// Experimental aliases defined via `#define` later in the header.
pub const ZSTD_c_format: ZSTD_cParameter = ZSTD_c_experimentalParam2;
pub const ZSTD_c_forceMaxWindow: ZSTD_cParameter = ZSTD_c_experimentalParam3;
pub const ZSTD_c_forceAttachDict: ZSTD_cParameter = ZSTD_c_experimentalParam4;
pub const ZSTD_c_literalCompressionMode: ZSTD_cParameter = ZSTD_c_experimentalParam5;
pub const ZSTD_c_srcSizeHint: ZSTD_cParameter = ZSTD_c_experimentalParam7;
pub const ZSTD_c_enableDedicatedDictSearch: ZSTD_cParameter = ZSTD_c_experimentalParam8;
pub const ZSTD_c_stableInBuffer: ZSTD_cParameter = ZSTD_c_experimentalParam9;
pub const ZSTD_c_stableOutBuffer: ZSTD_cParameter = ZSTD_c_experimentalParam10;
pub const ZSTD_c_blockDelimiters: ZSTD_cParameter = ZSTD_c_experimentalParam11;
pub const ZSTD_c_validateSequences: ZSTD_cParameter = ZSTD_c_experimentalParam12;
pub const ZSTD_c_blockSplitterLevel: ZSTD_cParameter = ZSTD_c_experimentalParam20;
pub const ZSTD_c_splitAfterSequences: ZSTD_cParameter = ZSTD_c_experimentalParam13;
pub const ZSTD_c_useRowMatchFinder: ZSTD_cParameter = ZSTD_c_experimentalParam14;
pub const ZSTD_c_deterministicRefPrefix: ZSTD_cParameter = ZSTD_c_experimentalParam15;
pub const ZSTD_c_prefetchCDictTables: ZSTD_cParameter = ZSTD_c_experimentalParam16;
pub const ZSTD_c_enableSeqProducerFallback: ZSTD_cParameter = ZSTD_c_experimentalParam17;
pub const ZSTD_c_maxBlockSize: ZSTD_cParameter = ZSTD_c_experimentalParam18;
pub const ZSTD_c_repcodeResolution: ZSTD_cParameter = ZSTD_c_experimentalParam19;
pub const ZSTD_c_searchForExternalRepcodes: ZSTD_cParameter = ZSTD_c_experimentalParam19; /* older name */

// ---------------------------------------------------------------------------
// ZSTD_bounds
// ---------------------------------------------------------------------------
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ZSTD_bounds {
    pub error: size_t,
    pub lowerBound: c_int,
    pub upperBound: c_int,
}

// ---------------------------------------------------------------------------
// ZSTD_ResetDirective
// ---------------------------------------------------------------------------
pub type ZSTD_ResetDirective = c_uint;
pub const ZSTD_reset_session_only: ZSTD_ResetDirective = 1;
pub const ZSTD_reset_parameters: ZSTD_ResetDirective = 2;
pub const ZSTD_reset_session_and_parameters: ZSTD_ResetDirective = 3;

// ---------------------------------------------------------------------------
// ZSTD_dParameter
// ---------------------------------------------------------------------------
pub type ZSTD_dParameter = c_uint;
pub const ZSTD_d_windowLogMax: ZSTD_dParameter = 100;
pub const ZSTD_d_experimentalParam1: ZSTD_dParameter = 1000;
pub const ZSTD_d_experimentalParam2: ZSTD_dParameter = 1001;
pub const ZSTD_d_experimentalParam3: ZSTD_dParameter = 1002;
pub const ZSTD_d_experimentalParam4: ZSTD_dParameter = 1003;
pub const ZSTD_d_experimentalParam5: ZSTD_dParameter = 1004;
pub const ZSTD_d_experimentalParam6: ZSTD_dParameter = 1005;

// Experimental aliases defined via `#define` later in the header.
pub const ZSTD_d_format: ZSTD_dParameter = ZSTD_d_experimentalParam1;
pub const ZSTD_d_stableOutBuffer: ZSTD_dParameter = ZSTD_d_experimentalParam2;
pub const ZSTD_d_forceIgnoreChecksum: ZSTD_dParameter = ZSTD_d_experimentalParam3;
pub const ZSTD_d_refMultipleDDicts: ZSTD_dParameter = ZSTD_d_experimentalParam4;
pub const ZSTD_d_disableHuffmanAssembly: ZSTD_dParameter = ZSTD_d_experimentalParam5;
pub const ZSTD_d_maxBlockSize: ZSTD_dParameter = ZSTD_d_experimentalParam6;

// ---------------------------------------------------------------------------
// Streaming buffers
// ---------------------------------------------------------------------------
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ZSTD_inBuffer {
    pub src: *const c_void, /**< start of input buffer */
    pub size: size_t,       /**< size of input buffer */
    pub pos: size_t,        /* position where reading stopped. Will be updated. */
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ZSTD_outBuffer {
    pub dst: *mut c_void, /**< start of output buffer */
    pub size: size_t,     /**< size of output buffer */
    pub pos: size_t,      /* position where writing stopped. Will be updated. */
}

// ---------------------------------------------------------------------------
// ZSTD_EndDirective
// ---------------------------------------------------------------------------
pub type ZSTD_EndDirective = c_uint;
pub const ZSTD_e_continue: ZSTD_EndDirective = 0;
pub const ZSTD_e_flush: ZSTD_EndDirective = 1;
pub const ZSTD_e_end: ZSTD_EndDirective = 2;

// ---------------------------------------------------------------------------
// Static-linking-only constants (advanced parameter bounds)
// ---------------------------------------------------------------------------
// #define ZSTD_FRAMEHEADERSIZE_PREFIX(format) ((format) == ZSTD_f_zstd1 ? 5 : 1)
#[inline(always)]
pub const fn ZSTD_FRAMEHEADERSIZE_PREFIX(format: ZSTD_format_e) -> size_t {
    if format == ZSTD_f_zstd1 {
        5
    } else {
        1
    }
}
// #define ZSTD_FRAMEHEADERSIZE_MIN(format) ((format) == ZSTD_f_zstd1 ? 6 : 2)
#[inline(always)]
pub const fn ZSTD_FRAMEHEADERSIZE_MIN(format: ZSTD_format_e) -> size_t {
    if format == ZSTD_f_zstd1 {
        6
    } else {
        2
    }
}
pub const ZSTD_FRAMEHEADERSIZE_MAX: size_t = 18; /* can be useful for static allocation */
pub const ZSTD_SKIPPABLEHEADERSIZE: size_t = 8;

// #define ZSTD_WINDOWLOG_MAX ((int)(sizeof(size_t) == 4 ? ZSTD_WINDOWLOG_MAX_32 : ZSTD_WINDOWLOG_MAX_64))
pub const ZSTD_WINDOWLOG_MAX_32: c_int = 30;
pub const ZSTD_WINDOWLOG_MAX_64: c_int = 31;
pub const ZSTD_WINDOWLOG_MAX: c_int = if core::mem::size_of::<size_t>() == 4 {
    ZSTD_WINDOWLOG_MAX_32
} else {
    ZSTD_WINDOWLOG_MAX_64
};
pub const ZSTD_WINDOWLOG_MIN: c_int = 10;
pub const ZSTD_HASHLOG_MAX: c_int = if ZSTD_WINDOWLOG_MAX < 30 {
    ZSTD_WINDOWLOG_MAX
} else {
    30
};
pub const ZSTD_HASHLOG_MIN: c_int = 6;
pub const ZSTD_CHAINLOG_MAX_32: c_int = 29;
pub const ZSTD_CHAINLOG_MAX_64: c_int = 30;
pub const ZSTD_CHAINLOG_MAX: c_int = if core::mem::size_of::<size_t>() == 4 {
    ZSTD_CHAINLOG_MAX_32
} else {
    ZSTD_CHAINLOG_MAX_64
};
pub const ZSTD_CHAINLOG_MIN: c_int = ZSTD_HASHLOG_MIN;
pub const ZSTD_SEARCHLOG_MAX: c_int = ZSTD_WINDOWLOG_MAX - 1;
pub const ZSTD_SEARCHLOG_MIN: c_int = 1;
pub const ZSTD_MINMATCH_MAX: c_int = 7; /* only for ZSTD_fast */
pub const ZSTD_MINMATCH_MIN: c_int = 3; /* only for ZSTD_btopt+ */
pub const ZSTD_TARGETLENGTH_MAX: c_int = ZSTD_BLOCKSIZE_MAX as c_int;
pub const ZSTD_TARGETLENGTH_MIN: c_int = 0;
pub const ZSTD_STRATEGY_MIN: c_int = ZSTD_fast as c_int;
pub const ZSTD_STRATEGY_MAX: c_int = ZSTD_btultra2 as c_int;
pub const ZSTD_BLOCKSIZE_MAX_MIN: c_int = 1 << 10; /* The minimum valid max blocksize. */

pub const ZSTD_OVERLAPLOG_MIN: c_int = 0;
pub const ZSTD_OVERLAPLOG_MAX: c_int = 9;

pub const ZSTD_WINDOWLOG_LIMIT_DEFAULT: c_int = 27;

pub const ZSTD_LDM_HASHLOG_MIN: c_int = ZSTD_HASHLOG_MIN;
pub const ZSTD_LDM_HASHLOG_MAX: c_int = ZSTD_HASHLOG_MAX;
pub const ZSTD_LDM_MINMATCH_MIN: c_int = 4;
pub const ZSTD_LDM_MINMATCH_MAX: c_int = 4096;
pub const ZSTD_LDM_BUCKETSIZELOG_MIN: c_int = 1;
pub const ZSTD_LDM_BUCKETSIZELOG_MAX: c_int = 8;
pub const ZSTD_LDM_HASHRATELOG_MIN: c_int = 0;
pub const ZSTD_LDM_HASHRATELOG_MAX: c_int = ZSTD_WINDOWLOG_MAX - ZSTD_HASHLOG_MIN;

/* Advanced parameter bounds */
pub const ZSTD_TARGETCBLOCKSIZE_MIN: c_int = 1340;
pub const ZSTD_TARGETCBLOCKSIZE_MAX: c_int = ZSTD_BLOCKSIZE_MAX as c_int;
pub const ZSTD_SRCSIZEHINT_MIN: c_int = 0;
pub const ZSTD_SRCSIZEHINT_MAX: c_int = c_int::MAX; /* INT_MAX */

// ---------------------------------------------------------------------------
// Advanced types
// ---------------------------------------------------------------------------
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ZSTD_Sequence {
    pub offset: c_uint,
    pub litLength: c_uint,
    pub matchLength: c_uint,
    pub rep: c_uint,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ZSTD_compressionParameters {
    pub windowLog: c_uint,
    pub chainLog: c_uint,
    pub hashLog: c_uint,
    pub searchLog: c_uint,
    pub minMatch: c_uint,
    pub targetLength: c_uint,
    pub strategy: ZSTD_strategy,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ZSTD_frameParameters {
    pub contentSizeFlag: c_int,
    pub checksumFlag: c_int,
    pub noDictIDFlag: c_int,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ZSTD_parameters {
    pub cParams: ZSTD_compressionParameters,
    pub fParams: ZSTD_frameParameters,
}

pub type ZSTD_dictContentType_e = c_uint;
pub const ZSTD_dct_auto: ZSTD_dictContentType_e = 0;
pub const ZSTD_dct_rawContent: ZSTD_dictContentType_e = 1;
pub const ZSTD_dct_fullDict: ZSTD_dictContentType_e = 2;

pub type ZSTD_dictLoadMethod_e = c_uint;
pub const ZSTD_dlm_byCopy: ZSTD_dictLoadMethod_e = 0;
pub const ZSTD_dlm_byRef: ZSTD_dictLoadMethod_e = 1;

pub type ZSTD_format_e = c_uint;
pub const ZSTD_f_zstd1: ZSTD_format_e = 0;
pub const ZSTD_f_zstd1_magicless: ZSTD_format_e = 1;

pub type ZSTD_forceIgnoreChecksum_e = c_uint;
pub const ZSTD_d_validateChecksum: ZSTD_forceIgnoreChecksum_e = 0;
pub const ZSTD_d_ignoreChecksum: ZSTD_forceIgnoreChecksum_e = 1;

pub type ZSTD_refMultipleDDicts_e = c_uint;
pub const ZSTD_rmd_refSingleDDict: ZSTD_refMultipleDDicts_e = 0;
pub const ZSTD_rmd_refMultipleDDicts: ZSTD_refMultipleDDicts_e = 1;

pub type ZSTD_dictAttachPref_e = c_uint;
pub const ZSTD_dictDefaultAttach: ZSTD_dictAttachPref_e = 0;
pub const ZSTD_dictForceAttach: ZSTD_dictAttachPref_e = 1;
pub const ZSTD_dictForceCopy: ZSTD_dictAttachPref_e = 2;
pub const ZSTD_dictForceLoad: ZSTD_dictAttachPref_e = 3;

pub type ZSTD_literalCompressionMode_e = c_uint;
pub const ZSTD_lcm_auto: ZSTD_literalCompressionMode_e = 0;
pub const ZSTD_lcm_huffman: ZSTD_literalCompressionMode_e = 1;
pub const ZSTD_lcm_uncompressed: ZSTD_literalCompressionMode_e = 2;

pub type ZSTD_ParamSwitch_e = c_uint;
pub const ZSTD_ps_auto: ZSTD_ParamSwitch_e = 0;
pub const ZSTD_ps_enable: ZSTD_ParamSwitch_e = 1;
pub const ZSTD_ps_disable: ZSTD_ParamSwitch_e = 2;
/* #define ZSTD_paramSwitch_e ZSTD_ParamSwitch_e  (old name) */
pub type ZSTD_paramSwitch_e = ZSTD_ParamSwitch_e;

// ---------------------------------------------------------------------------
// Frame header / frame type
// ---------------------------------------------------------------------------
pub type ZSTD_FrameType_e = c_uint;
pub const ZSTD_frame: ZSTD_FrameType_e = 0;
pub const ZSTD_skippableFrame: ZSTD_FrameType_e = 1;
/* #define ZSTD_frameType_e ZSTD_FrameType_e  (old name) */
pub type ZSTD_frameType_e = ZSTD_FrameType_e;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ZSTD_FrameHeader {
    pub frameContentSize: c_ulonglong,
    pub windowSize: c_ulonglong,
    pub blockSizeMax: c_uint,
    pub frameType: ZSTD_FrameType_e,
    pub headerSize: c_uint,
    pub dictID: c_uint,
    pub checksumFlag: c_uint,
    pub _reserved1: c_uint,
    pub _reserved2: c_uint,
}
/* #define ZSTD_frameHeader ZSTD_FrameHeader  (old name) */
pub type ZSTD_frameHeader = ZSTD_FrameHeader;

// #define ZSTD_DECOMPRESSION_MARGIN(originalSize, blockSize) ...
#[inline(always)]
pub const fn ZSTD_DECOMPRESSION_MARGIN(originalSize: size_t, blockSize: size_t) -> size_t {
    ZSTD_FRAMEHEADERSIZE_MAX /* Frame header */
        + 4 /* checksum */
        + (if originalSize == 0 {
            0
        } else {
            3 * ((originalSize + blockSize - 1) / blockSize) /* 3 bytes per block */
        })
        + blockSize /* One block of margin */
}

pub type ZSTD_SequenceFormat_e = c_uint;
pub const ZSTD_sf_noBlockDelimiters: ZSTD_SequenceFormat_e = 0;
pub const ZSTD_sf_explicitBlockDelimiters: ZSTD_SequenceFormat_e = 1;
/* #define ZSTD_sequenceFormat_e ZSTD_SequenceFormat_e  (old name) */
pub type ZSTD_sequenceFormat_e = ZSTD_SequenceFormat_e;

pub const ZSTD_BLOCKSPLITTER_LEVEL_MAX: c_int = 6;

// ---------------------------------------------------------------------------
// Custom memory allocation
// ---------------------------------------------------------------------------
pub type ZSTD_allocFunction = Option<unsafe extern "C" fn(opaque: *mut c_void, size: size_t) -> *mut c_void>;
pub type ZSTD_freeFunction = Option<unsafe extern "C" fn(opaque: *mut c_void, address: *mut c_void)>;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ZSTD_customMem {
    pub customAlloc: ZSTD_allocFunction,
    pub customFree: ZSTD_freeFunction,
    pub opaque: *mut c_void,
}

// ---------------------------------------------------------------------------
// Frame progression
// ---------------------------------------------------------------------------
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ZSTD_frameProgression {
    pub ingested: c_ulonglong,
    pub consumed: c_ulonglong,
    pub produced: c_ulonglong,
    pub flushed: c_ulonglong,
    pub currentJobID: c_uint,
    pub nbActiveWorkers: c_uint,
}

// ---------------------------------------------------------------------------
// External sequence producer
// ---------------------------------------------------------------------------
pub type ZSTD_sequenceProducer_F = Option<
    unsafe extern "C" fn(
        sequenceProducerState: *mut c_void,
        outSeqs: *mut ZSTD_Sequence,
        outSeqsCapacity: size_t,
        src: *const c_void,
        srcSize: size_t,
        dict: *const c_void,
        dictSize: size_t,
        compressionLevel: c_int,
        windowSize: size_t,
    ) -> size_t,
>;

pub const ZSTD_SEQUENCE_PRODUCER_ERROR: size_t = (0isize as size_t).wrapping_sub(1); /* (size_t)(-1) */

// ---------------------------------------------------------------------------
// Buffer-less / raw block API: next input type
// ---------------------------------------------------------------------------
pub type ZSTD_nextInputType_e = c_uint;
pub const ZSTDnit_frameHeader: ZSTD_nextInputType_e = 0;
pub const ZSTDnit_blockHeader: ZSTD_nextInputType_e = 1;
pub const ZSTDnit_block: ZSTD_nextInputType_e = 2;
pub const ZSTDnit_lastBlock: ZSTD_nextInputType_e = 3;
pub const ZSTDnit_checksum: ZSTD_nextInputType_e = 4;
pub const ZSTDnit_skippableFrame: ZSTD_nextInputType_e = 5;
