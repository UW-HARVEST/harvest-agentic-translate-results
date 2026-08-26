//! Public API types & constants from include/zstd.h
#![allow(non_snake_case, dead_code, non_upper_case_globals, non_camel_case_types)]

use crate::mem::*;

pub const ZSTD_VERSION_MAJOR: u32 = 1;
pub const ZSTD_VERSION_MINOR: u32 = 5;
pub const ZSTD_VERSION_RELEASE: u32 = 7;
pub const ZSTD_VERSION_NUMBER: u32 =
    ZSTD_VERSION_MAJOR * 100 * 100 + ZSTD_VERSION_MINOR * 100 + ZSTD_VERSION_RELEASE;
pub const ZSTD_VERSION_STRING: &str = "1.5.7\0";

pub const ZSTD_CLEVEL_DEFAULT: i32 = 3;

pub const ZSTD_MAGICNUMBER: u32 = 0xFD2FB528;
pub const ZSTD_MAGIC_DICTIONARY: u32 = 0xEC30A437;
pub const ZSTD_MAGIC_SKIPPABLE_START: u32 = 0x184D2A50;
pub const ZSTD_MAGIC_SKIPPABLE_MASK: u32 = 0xFFFFFFF0;

pub const ZSTD_BLOCKSIZELOG_MAX: u32 = 17;
pub const ZSTD_BLOCKSIZE_MAX: u32 = 1 << ZSTD_BLOCKSIZELOG_MAX;

pub const ZSTD_CONTENTSIZE_UNKNOWN: u64 = 0u64.wrapping_sub(1);
pub const ZSTD_CONTENTSIZE_ERROR: u64 = 0u64.wrapping_sub(2);

/* ---- strategies ---- */
pub type ZSTD_strategy = core::ffi::c_int;
pub const ZSTD_fast: ZSTD_strategy = 1;
pub const ZSTD_dfast: ZSTD_strategy = 2;
pub const ZSTD_greedy: ZSTD_strategy = 3;
pub const ZSTD_lazy: ZSTD_strategy = 4;
pub const ZSTD_lazy2: ZSTD_strategy = 5;
pub const ZSTD_btlazy2: ZSTD_strategy = 6;
pub const ZSTD_btopt: ZSTD_strategy = 7;
pub const ZSTD_btultra: ZSTD_strategy = 8;
pub const ZSTD_btultra2: ZSTD_strategy = 9;

/* ---- cParameters ---- */
pub type ZSTD_cParameter = core::ffi::c_int;
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

pub const ZSTD_c_rsyncable: ZSTD_cParameter = ZSTD_c_experimentalParam1;
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
pub const ZSTD_c_searchForExternalRepcodes: ZSTD_cParameter = ZSTD_c_experimentalParam19;

pub const ZSTD_BLOCKSPLITTER_LEVEL_MAX: i32 = 6;

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct ZSTD_bounds {
    pub error: usize,
    pub lowerBound: core::ffi::c_int,
    pub upperBound: core::ffi::c_int,
}

pub type ZSTD_ResetDirective = core::ffi::c_int;
pub const ZSTD_reset_session_only: ZSTD_ResetDirective = 1;
pub const ZSTD_reset_parameters: ZSTD_ResetDirective = 2;
pub const ZSTD_reset_session_and_parameters: ZSTD_ResetDirective = 3;

pub type ZSTD_dParameter = core::ffi::c_int;
pub const ZSTD_d_windowLogMax: ZSTD_dParameter = 100;
pub const ZSTD_d_experimentalParam1: ZSTD_dParameter = 1000;
pub const ZSTD_d_experimentalParam2: ZSTD_dParameter = 1001;
pub const ZSTD_d_experimentalParam3: ZSTD_dParameter = 1002;
pub const ZSTD_d_experimentalParam4: ZSTD_dParameter = 1003;
pub const ZSTD_d_experimentalParam5: ZSTD_dParameter = 1004;
pub const ZSTD_d_experimentalParam6: ZSTD_dParameter = 1005;
pub const ZSTD_d_format: ZSTD_dParameter = ZSTD_d_experimentalParam1;
pub const ZSTD_d_stableOutBuffer: ZSTD_dParameter = ZSTD_d_experimentalParam2;
pub const ZSTD_d_forceIgnoreChecksum: ZSTD_dParameter = ZSTD_d_experimentalParam3;
pub const ZSTD_d_refMultipleDDicts: ZSTD_dParameter = ZSTD_d_experimentalParam4;
pub const ZSTD_d_disableHuffmanAssembly: ZSTD_dParameter = ZSTD_d_experimentalParam5;
pub const ZSTD_d_maxBlockSize: ZSTD_dParameter = ZSTD_d_experimentalParam6;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ZSTD_inBuffer {
    pub src: *const core::ffi::c_void,
    pub size: usize,
    pub pos: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ZSTD_outBuffer {
    pub dst: *mut core::ffi::c_void,
    pub size: usize,
    pub pos: usize,
}

pub type ZSTD_EndDirective = core::ffi::c_int;
pub const ZSTD_e_continue: ZSTD_EndDirective = 0;
pub const ZSTD_e_flush: ZSTD_EndDirective = 1;
pub const ZSTD_e_end: ZSTD_EndDirective = 2;

/* ---- frame header sizes / bounds ---- */
pub const ZSTD_FRAMEHEADERSIZE_MAX: usize = 18;
pub const ZSTD_SKIPPABLEHEADERSIZE: usize = 8;
#[inline(always)]
pub fn ZSTD_FRAMEHEADERSIZE_PREFIX(format: ZSTD_format_e) -> usize {
    if format == ZSTD_f_zstd1 {
        5
    } else {
        1
    }
}
#[inline(always)]
pub fn ZSTD_FRAMEHEADERSIZE_MIN(format: ZSTD_format_e) -> usize {
    if format == ZSTD_f_zstd1 {
        6
    } else {
        2
    }
}

pub const ZSTD_WINDOWLOG_MAX_32: i32 = 30;
pub const ZSTD_WINDOWLOG_MAX_64: i32 = 31;
pub const ZSTD_WINDOWLOG_MAX: i32 = if core::mem::size_of::<usize>() == 4 {
    ZSTD_WINDOWLOG_MAX_32
} else {
    ZSTD_WINDOWLOG_MAX_64
};
pub const ZSTD_WINDOWLOG_MIN: i32 = 10;
pub const ZSTD_HASHLOG_MAX: i32 = if ZSTD_WINDOWLOG_MAX < 30 {
    ZSTD_WINDOWLOG_MAX
} else {
    30
};
pub const ZSTD_HASHLOG_MIN: i32 = 6;
pub const ZSTD_CHAINLOG_MAX_32: i32 = 29;
pub const ZSTD_CHAINLOG_MAX_64: i32 = 30;
pub const ZSTD_CHAINLOG_MAX: i32 = if core::mem::size_of::<usize>() == 4 {
    ZSTD_CHAINLOG_MAX_32
} else {
    ZSTD_CHAINLOG_MAX_64
};
pub const ZSTD_CHAINLOG_MIN: i32 = ZSTD_HASHLOG_MIN;
pub const ZSTD_SEARCHLOG_MAX: i32 = ZSTD_WINDOWLOG_MAX - 1;
pub const ZSTD_SEARCHLOG_MIN: i32 = 1;
pub const ZSTD_MINMATCH_MAX: i32 = 7;
pub const ZSTD_MINMATCH_MIN: i32 = 3;
pub const ZSTD_TARGETLENGTH_MAX: i32 = ZSTD_BLOCKSIZE_MAX as i32;
pub const ZSTD_TARGETLENGTH_MIN: i32 = 0;
pub const ZSTD_STRATEGY_MIN: i32 = ZSTD_fast;
pub const ZSTD_STRATEGY_MAX: i32 = ZSTD_btultra2;
pub const ZSTD_BLOCKSIZE_MAX_MIN: i32 = 1 << 10;
pub const ZSTD_OVERLAPLOG_MIN: i32 = 0;
pub const ZSTD_OVERLAPLOG_MAX: i32 = 9;
pub const ZSTD_WINDOWLOG_LIMIT_DEFAULT: i32 = 27;
pub const ZSTD_LDM_HASHLOG_MIN: i32 = ZSTD_HASHLOG_MIN;
pub const ZSTD_LDM_HASHLOG_MAX: i32 = ZSTD_HASHLOG_MAX;
pub const ZSTD_LDM_MINMATCH_MIN: i32 = 4;
pub const ZSTD_LDM_MINMATCH_MAX: i32 = 4096;
pub const ZSTD_LDM_BUCKETSIZELOG_MIN: i32 = 1;
pub const ZSTD_LDM_BUCKETSIZELOG_MAX: i32 = 8;
pub const ZSTD_LDM_HASHRATELOG_MIN: i32 = 0;
pub const ZSTD_LDM_HASHRATELOG_MAX: i32 = ZSTD_WINDOWLOG_MAX - ZSTD_HASHLOG_MIN;
pub const ZSTD_TARGETCBLOCKSIZE_MIN: i32 = 1340;
pub const ZSTD_TARGETCBLOCKSIZE_MAX: i32 = ZSTD_BLOCKSIZE_MAX as i32;
pub const ZSTD_SRCSIZEHINT_MIN: i32 = 0;
pub const ZSTD_SRCSIZEHINT_MAX: i32 = i32::MAX;

pub const ZSTD_MAX_INPUT_SIZE: u64 = if core::mem::size_of::<usize>() == 8 {
    0xFF00FF00FF00FF00u64
} else {
    0xFF00FF00u64
};

#[repr(C)]
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub struct ZSTD_Sequence {
    pub offset: core::ffi::c_uint,
    pub litLength: core::ffi::c_uint,
    pub matchLength: core::ffi::c_uint,
    pub rep: core::ffi::c_uint,
}

#[repr(C)]
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub struct ZSTD_compressionParameters {
    pub windowLog: core::ffi::c_uint,
    pub chainLog: core::ffi::c_uint,
    pub hashLog: core::ffi::c_uint,
    pub searchLog: core::ffi::c_uint,
    pub minMatch: core::ffi::c_uint,
    pub targetLength: core::ffi::c_uint,
    pub strategy: ZSTD_strategy,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct ZSTD_frameParameters {
    pub contentSizeFlag: core::ffi::c_int,
    pub checksumFlag: core::ffi::c_int,
    pub noDictIDFlag: core::ffi::c_int,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct ZSTD_parameters {
    pub cParams: ZSTD_compressionParameters,
    pub fParams: ZSTD_frameParameters,
}

pub type ZSTD_dictContentType_e = core::ffi::c_int;
pub const ZSTD_dct_auto: ZSTD_dictContentType_e = 0;
pub const ZSTD_dct_rawContent: ZSTD_dictContentType_e = 1;
pub const ZSTD_dct_fullDict: ZSTD_dictContentType_e = 2;

pub type ZSTD_dictLoadMethod_e = core::ffi::c_int;
pub const ZSTD_dlm_byCopy: ZSTD_dictLoadMethod_e = 0;
pub const ZSTD_dlm_byRef: ZSTD_dictLoadMethod_e = 1;

pub type ZSTD_format_e = core::ffi::c_int;
pub const ZSTD_f_zstd1: ZSTD_format_e = 0;
pub const ZSTD_f_zstd1_magicless: ZSTD_format_e = 1;

pub type ZSTD_forceIgnoreChecksum_e = core::ffi::c_int;
pub const ZSTD_d_validateChecksum: ZSTD_forceIgnoreChecksum_e = 0;
pub const ZSTD_d_ignoreChecksum: ZSTD_forceIgnoreChecksum_e = 1;

pub type ZSTD_refMultipleDDicts_e = core::ffi::c_int;
pub const ZSTD_rmd_refSingleDDict: ZSTD_refMultipleDDicts_e = 0;
pub const ZSTD_rmd_refMultipleDDicts: ZSTD_refMultipleDDicts_e = 1;

pub type ZSTD_dictAttachPref_e = core::ffi::c_int;
pub const ZSTD_dictDefaultAttach: ZSTD_dictAttachPref_e = 0;
pub const ZSTD_dictForceAttach: ZSTD_dictAttachPref_e = 1;
pub const ZSTD_dictForceCopy: ZSTD_dictAttachPref_e = 2;
pub const ZSTD_dictForceLoad: ZSTD_dictAttachPref_e = 3;

pub type ZSTD_literalCompressionMode_e = core::ffi::c_int;
pub const ZSTD_lcm_auto: ZSTD_literalCompressionMode_e = 0;
pub const ZSTD_lcm_huffman: ZSTD_literalCompressionMode_e = 1;
pub const ZSTD_lcm_uncompressed: ZSTD_literalCompressionMode_e = 2;

pub type ZSTD_ParamSwitch_e = core::ffi::c_int;
pub const ZSTD_ps_auto: ZSTD_ParamSwitch_e = 0;
pub const ZSTD_ps_enable: ZSTD_ParamSwitch_e = 1;
pub const ZSTD_ps_disable: ZSTD_ParamSwitch_e = 2;

pub type ZSTD_FrameType_e = core::ffi::c_int;
pub const ZSTD_frame: ZSTD_FrameType_e = 0;
pub const ZSTD_skippableFrame: ZSTD_FrameType_e = 1;

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct ZSTD_FrameHeader {
    pub frameContentSize: core::ffi::c_ulonglong,
    pub windowSize: core::ffi::c_ulonglong,
    pub blockSizeMax: core::ffi::c_uint,
    pub frameType: ZSTD_FrameType_e,
    pub headerSize: core::ffi::c_uint,
    pub dictID: core::ffi::c_uint,
    pub checksumFlag: core::ffi::c_uint,
    pub _reserved1: core::ffi::c_uint,
    pub _reserved2: core::ffi::c_uint,
}

pub type ZSTD_SequenceFormat_e = core::ffi::c_int;
pub const ZSTD_sf_noBlockDelimiters: ZSTD_SequenceFormat_e = 0;
pub const ZSTD_sf_explicitBlockDelimiters: ZSTD_SequenceFormat_e = 1;

pub type ZSTD_allocFunction =
    Option<unsafe extern "C" fn(*mut core::ffi::c_void, usize) -> *mut core::ffi::c_void>;
pub type ZSTD_freeFunction =
    Option<unsafe extern "C" fn(*mut core::ffi::c_void, *mut core::ffi::c_void)>;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ZSTD_customMem {
    pub customAlloc: ZSTD_allocFunction,
    pub customFree: ZSTD_freeFunction,
    pub opaque: *mut core::ffi::c_void,
}

impl Default for ZSTD_customMem {
    fn default() -> Self {
        ZSTD_customMem {
            customAlloc: None,
            customFree: None,
            opaque: core::ptr::null_mut(),
        }
    }
}

pub const ZSTD_defaultCMem: ZSTD_customMem = ZSTD_customMem {
    customAlloc: None,
    customFree: None,
    opaque: core::ptr::null_mut(),
};

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct ZSTD_frameProgression {
    pub ingested: core::ffi::c_ulonglong,
    pub consumed: core::ffi::c_ulonglong,
    pub produced: core::ffi::c_ulonglong,
    pub flushed: core::ffi::c_ulonglong,
    pub currentJobID: core::ffi::c_uint,
    pub nbActiveWorkers: core::ffi::c_uint,
}

pub type ZSTD_nextInputType_e = core::ffi::c_int;
pub const ZSTDnit_frameHeader: ZSTD_nextInputType_e = 0;
pub const ZSTDnit_blockHeader: ZSTD_nextInputType_e = 1;
pub const ZSTDnit_block: ZSTD_nextInputType_e = 2;
pub const ZSTDnit_lastBlock: ZSTD_nextInputType_e = 3;
pub const ZSTDnit_checksum: ZSTD_nextInputType_e = 4;
pub const ZSTDnit_skippableFrame: ZSTD_nextInputType_e = 5;

pub const ZSTD_SEQUENCE_PRODUCER_ERROR: usize = usize::MAX;

pub type ZSTD_sequenceProducer_F = Option<
    unsafe extern "C" fn(
        *mut core::ffi::c_void,
        *mut ZSTD_Sequence,
        usize,
        *const core::ffi::c_void,
        usize,
        *const core::ffi::c_void,
        usize,
        core::ffi::c_int,
        usize,
    ) -> usize,
>;

/* ---- custom allocation helpers (allocations.h) ---- */

#[inline]
pub unsafe fn ZSTD_customMalloc(size: usize, customMem: ZSTD_customMem) -> *mut u8 {
    if let Some(f) = customMem.customAlloc {
        return f(customMem.opaque, size) as *mut u8;
    }
    malloc(size) as *mut u8
}

#[inline]
pub unsafe fn ZSTD_customCalloc(size: usize, customMem: ZSTD_customMem) -> *mut u8 {
    if let Some(f) = customMem.customAlloc {
        let ptr = f(customMem.opaque, size) as *mut u8;
        ZSTD_memset(ptr, 0, size);
        return ptr;
    }
    calloc(1, size) as *mut u8
}

#[inline]
pub unsafe fn ZSTD_customFree(ptr: *mut u8, customMem: ZSTD_customMem) {
    if !ptr.is_null() {
        if let Some(f) = customMem.customFree {
            f(customMem.opaque, ptr as *mut core::ffi::c_void);
        } else {
            free(ptr as *mut core::ffi::c_void);
        }
    }
}

/* ---- ZSTD_COMPRESSBOUND macro ---- */
#[inline(always)]
pub fn ZSTD_COMPRESSBOUND(srcSize: usize) -> usize {
    if (srcSize as u64) >= ZSTD_MAX_INPUT_SIZE {
        0
    } else {
        srcSize
            + (srcSize >> 8)
            + (if srcSize < (128 << 10) {
                ((128 << 10) - srcSize) >> 11
            } else {
                0
            })
    }
}

#[inline(always)]
pub fn ZSTD_DECOMPRESSION_MARGIN(originalSize: usize, blockSize: usize) -> usize {
    ZSTD_FRAMEHEADERSIZE_MAX
        + 4
        + (if originalSize == 0 {
            0
        } else {
            3 * ((originalSize + blockSize - 1) / blockSize)
        })
        + blockSize
}
