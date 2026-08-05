//! Faithful translation of the TYPES and MEM_STATIC inline helpers from
//! compress/zstd_compress_internal.h — the shared header for lib/compress.
//!
//! Build config: DYNAMIC_BMI2=0, single-threaded (ZSTD_MULTITHREAD undefined),
//! ZSTD_TRACE=1 (traceCtx is a plain u64, trace hooks are no-ops), LE 64-bit.
//! All helpers are `pub` (crate-internal), NOT no_mangle.
#![allow(
    non_snake_case,
    non_camel_case_types,
    non_upper_case_globals,
    dead_code,
    unused_mut,
    unused_assignments,
    unused_parens
)]

use core::ffi::c_void;

use crate::common::allocations::ZSTD_customMem;
use crate::common::error::err_is_error;
use crate::common::fse::{FSE_CTable, FSE_repeat};
use crate::common::mem::{U16, U32, U64};
use crate::common::pool::POOL_ctx;
use crate::common::xxhash::XXH64_state_t;
use crate::common::zstd_internal::{
    bt_raw, bt_rle, MaxLL, MaxML, MaxOff, MaxSeq, LLFSELog, MLFSELog, OffFSELog, MINMATCH,
    WILDCOPY_OVERLENGTH, ZSTD_blockHeaderSize, ZSTD_MAX_FSE_HEADERS_SIZE, ZSTD_MAX_HUF_HEADER_SIZE,
    ZSTD_no_overlap, ZSTD_REP_NUM,
};

use crate::common::bits::{highbit32 as ZSTD_highbit32, nb_common_bytes as ZSTD_NbCommonBytes};
use crate::common::mem::{
    mem_64bits as MEM_64bits, mem_read16 as MEM_read16, mem_read32 as MEM_read32,
    mem_read_le32 as MEM_readLE32, mem_read_le64 as MEM_readLE64, mem_read_st as MEM_readST,
    mem_write_le24 as MEM_writeLE24,
};
use crate::common::zstd_internal::{zstd_copy16 as ZSTD_copy16, zstd_wildcopy as ZSTD_wildcopy};

pub use crate::zstd_h::{
    ZSTD_bounds, ZSTD_compressionParameters, ZSTD_dictContentType_e, ZSTD_format_e,
    ZSTD_frameParameters, ZSTD_inBuffer, ZSTD_strategy,
};

/*-*************************************
*  Public-API enum types defined locally (values match include/zstd.h)
***************************************/

// ZSTD_ParamSwitch_e (a.k.a. ZSTD_paramSwitch_e, old name)
pub type ZSTD_ParamSwitch_e = u32;
pub const ZSTD_ps_auto: ZSTD_ParamSwitch_e = 0;
pub const ZSTD_ps_enable: ZSTD_ParamSwitch_e = 1;
pub const ZSTD_ps_disable: ZSTD_ParamSwitch_e = 2;
pub type ZSTD_paramSwitch_e = ZSTD_ParamSwitch_e;

// ZSTD_dictAttachPref_e
pub type ZSTD_dictAttachPref_e = u32;
pub const ZSTD_dictDefaultAttach: ZSTD_dictAttachPref_e = 0;
pub const ZSTD_dictForceAttach: ZSTD_dictAttachPref_e = 1;
pub const ZSTD_dictForceCopy: ZSTD_dictAttachPref_e = 2;
pub const ZSTD_dictForceLoad: ZSTD_dictAttachPref_e = 3;

// ZSTD_SequenceFormat_e
pub type ZSTD_SequenceFormat_e = u32;
pub const ZSTD_sf_noBlockDelimiters: ZSTD_SequenceFormat_e = 0;
pub const ZSTD_sf_explicitBlockDelimiters: ZSTD_SequenceFormat_e = 1;

// ZSTD_bufferMode_e (from common/zstd_internal.h)
pub type ZSTD_bufferMode_e = u32;
pub const ZSTD_bm_buffered: ZSTD_bufferMode_e = 0;
pub const ZSTD_bm_stable: ZSTD_bufferMode_e = 1;

// SymbolEncodingType_e (from common/zstd_internal.h)
pub type SymbolEncodingType_e = u32;

// ZSTD_cParameter — only the value we reference from inline helpers
pub type ZSTD_cParameter = u32;
pub const ZSTD_c_strategy: ZSTD_cParameter = 107;

/// ZSTD_Sequence (public API, include/zstd.h)
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ZSTD_Sequence {
    pub offset: u32,
    pub litLength: u32,
    pub matchLength: u32,
    pub rep: u32,
}

/// External sequence producer function pointer (public API).
pub type ZSTD_sequenceProducer_F = Option<
    unsafe extern "C" fn(
        *mut c_void,          // sequenceProducerState
        *mut ZSTD_Sequence,   // outSeqs
        usize,                // outSeqsCapacity
        *const c_void,        // src
        usize,                // srcSize
        *const c_void,        // dict
        usize,                // dictSize
        i32,                  // compressionLevel
        usize,                // windowSize
    ) -> usize,
>;

// ZSTD_threadPool == POOL_ctx (single-threaded stub)
pub type ZSTD_threadPool = POOL_ctx;

// Opaque ZSTD_CDict (defined in zstd_compress.c). Only used through pointers here.
#[repr(C)]
pub struct ZSTD_CDict_s {
    _priv: [u8; 0],
}
pub type ZSTD_CDict = ZSTD_CDict_s;

// Tracing: ZSTD_TraceCtx == unsigned long long. Trace hooks are no-ops.
pub type ZSTD_TraceCtx = u64;

/*-*************************************
*  HUF / FSE table element types & sizes
***************************************/
pub type HUF_CElt = usize; /* size_t; "incomplete type" in C */

// HUF_repeat
pub type HUF_repeat = u32;
pub const HUF_repeat_none: HUF_repeat = 0;
pub const HUF_repeat_check: HUF_repeat = 1;
pub const HUF_repeat_valid: HUF_repeat = 2;

const fn HUF_CTABLE_SIZE_ST(maxSymbolValue: usize) -> usize {
    maxSymbolValue + 2
}
const fn FSE_CTABLE_SIZE_U32(maxTableLog: u32, maxSymbolValue: u32) -> usize {
    (1 + (1u32 << (maxTableLog - 1)) + ((maxSymbolValue + 1) * 2)) as usize
}

/*-*************************************
*  Constants
***************************************/
pub const kSearchStrength: u32 = 8;
pub const HASH_READ_SIZE: usize = 8;
pub const ZSTD_DUBT_UNSORTED_MARK: u32 = 1;

pub const ZSTD_WINDOW_START_INDEX: u32 = 2;
pub const ZSTD_ROW_HASH_CACHE_SIZE: usize = 8;
pub const LDM_BATCH_SIZE: usize = 64;
pub const ZSTD_MAX_NB_BLOCK_SPLITS: usize = 196;

pub const ZSTD_OPT_NUM: usize = crate::common::zstd_internal::ZSTD_OPT_NUM;
pub const ZSTD_OPT_SIZE: usize = ZSTD_OPT_NUM + 3;

// ZSTD_SLIPBLOCK_WORKSPACESIZE (from zstd_preSplit.h)
pub const ZSTD_SLIPBLOCK_WORKSPACESIZE: usize = 8208;
// HUF_WORKSPACE_SIZE (from huf.h)
pub const HUF_WORKSPACE_SIZE: usize = crate::common::huf_common::HUF_WORKSPACE_SIZE;

pub const COMPRESS_SEQUENCES_WORKSPACE_SIZE: usize =
    core::mem::size_of::<u32>() * ((MaxSeq as usize) + 2);
pub const ENTROPY_WORKSPACE_SIZE: usize = HUF_WORKSPACE_SIZE + COMPRESS_SEQUENCES_WORKSPACE_SIZE;
pub const TMP_WORKSPACE_SIZE: usize = if ENTROPY_WORKSPACE_SIZE > ZSTD_SLIPBLOCK_WORKSPACESIZE {
    ENTROPY_WORKSPACE_SIZE
} else {
    ZSTD_SLIPBLOCK_WORKSPACESIZE
};

/*-*************************************
*  Context memory management
***************************************/
// ZSTD_compressionStage_e
pub type ZSTD_compressionStage_e = u32;
pub const ZSTDcs_created: ZSTD_compressionStage_e = 0;
pub const ZSTDcs_init: ZSTD_compressionStage_e = 1;
pub const ZSTDcs_ongoing: ZSTD_compressionStage_e = 2;
pub const ZSTDcs_ending: ZSTD_compressionStage_e = 3;

// ZSTD_cStreamStage
pub type ZSTD_cStreamStage = u32;
pub const zcss_init: ZSTD_cStreamStage = 0;
pub const zcss_load: ZSTD_cStreamStage = 1;
pub const zcss_flush: ZSTD_cStreamStage = 2;

#[repr(C)]
pub struct ZSTD_prefixDict {
    pub dict: *const c_void,
    pub dictSize: usize,
    pub dictContentType: ZSTD_dictContentType_e,
}

#[repr(C)]
pub struct ZSTD_localDict {
    pub dictBuffer: *mut c_void,
    pub dict: *const c_void,
    pub dictSize: usize,
    pub dictContentType: ZSTD_dictContentType_e,
    pub cdict: *mut ZSTD_CDict,
}

#[repr(C)]
pub struct ZSTD_hufCTables_t {
    pub CTable: [HUF_CElt; HUF_CTABLE_SIZE_ST(255)],
    pub repeatMode: HUF_repeat,
}

#[repr(C)]
pub struct ZSTD_fseCTables_t {
    pub offcodeCTable: [FSE_CTable; FSE_CTABLE_SIZE_U32(OffFSELog, MaxOff)],
    pub matchlengthCTable: [FSE_CTable; FSE_CTABLE_SIZE_U32(MLFSELog, MaxML)],
    pub litlengthCTable: [FSE_CTable; FSE_CTABLE_SIZE_U32(LLFSELog, MaxLL)],
    pub offcode_repeatMode: FSE_repeat,
    pub matchlength_repeatMode: FSE_repeat,
    pub litlength_repeatMode: FSE_repeat,
}

#[repr(C)]
pub struct ZSTD_entropyCTables_t {
    pub huf: ZSTD_hufCTables_t,
    pub fse: ZSTD_fseCTables_t,
}

/***********************************************
*  Sequences
***********************************************/
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SeqDef {
    pub offBase: U32, /* offBase == Offset + ZSTD_REP_NUM, or repcode 1,2,3 */
    pub litLength: U16,
    pub mlBase: U16, /* mlBase == matchLength - MINMATCH */
}

// ZSTD_longLengthType_e
pub type ZSTD_longLengthType_e = u32;
pub const ZSTD_llt_none: ZSTD_longLengthType_e = 0;
pub const ZSTD_llt_literalLength: ZSTD_longLengthType_e = 1;
pub const ZSTD_llt_matchLength: ZSTD_longLengthType_e = 2;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SeqStore_t {
    pub sequencesStart: *mut SeqDef,
    pub sequences: *mut SeqDef, /* ptr to end of sequences */
    pub litStart: *mut u8,
    pub lit: *mut u8, /* ptr to end of literals */
    pub llCode: *mut u8,
    pub mlCode: *mut u8,
    pub ofCode: *mut u8,
    pub maxNbSeq: usize,
    pub maxNbLit: usize,
    pub longLengthType: ZSTD_longLengthType_e,
    pub longLengthPos: U32, /* Index of the sequence to apply long length modification to */
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ZSTD_SequenceLength {
    pub litLength: U32,
    pub matchLength: U32,
}

/***********************************************
*  Entropy buffer statistics structs
***********************************************/
#[repr(C)]
pub struct ZSTD_hufCTablesMetadata_t {
    pub hType: SymbolEncodingType_e,
    pub hufDesBuffer: [u8; ZSTD_MAX_HUF_HEADER_SIZE],
    pub hufDesSize: usize,
}

#[repr(C)]
pub struct ZSTD_fseCTablesMetadata_t {
    pub llType: SymbolEncodingType_e,
    pub ofType: SymbolEncodingType_e,
    pub mlType: SymbolEncodingType_e,
    pub fseTablesBuffer: [u8; ZSTD_MAX_FSE_HEADERS_SIZE],
    pub fseTablesSize: usize,
    pub lastCountSize: usize,
}

#[repr(C)]
pub struct ZSTD_entropyCTablesMetadata_t {
    pub hufMetadata: ZSTD_hufCTablesMetadata_t,
    pub fseMetadata: ZSTD_fseCTablesMetadata_t,
}

/*********************************
*  Compression internals structs
*********************************/
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ZSTD_match_t {
    pub off: U32, /* Offset sumtype code for the match, using ZSTD_storeSeq() format */
    pub len: U32, /* Raw length of match */
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct rawSeq {
    pub offset: U32,
    pub litLength: U32,
    pub matchLength: U32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct RawSeqStore_t {
    pub seq: *mut rawSeq,
    pub pos: usize,
    pub posInSequence: usize,
    pub size: usize,
    pub capacity: usize,
}

pub const kNullRawSeqStore: RawSeqStore_t = RawSeqStore_t {
    seq: core::ptr::null_mut(),
    pos: 0,
    posInSequence: 0,
    size: 0,
    capacity: 0,
};

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ZSTD_optimal_t {
    pub price: i32,               /* price from beginning of segment to this position */
    pub off: U32,                 /* offset of previous match */
    pub mlen: U32,                /* length of previous match */
    pub litlen: U32,              /* nb of literals since previous match */
    pub rep: [U32; ZSTD_REP_NUM], /* offset history after previous match */
}

// ZSTD_OptPrice_e
pub type ZSTD_OptPrice_e = u32;
pub const zop_dynamic: ZSTD_OptPrice_e = 0;
pub const zop_predef: ZSTD_OptPrice_e = 1;

#[repr(C)]
pub struct optState_t {
    pub litFreq: *mut u32,         /* table of literals statistics, of size 256 */
    pub litLengthFreq: *mut u32,   /* table of litLength statistics, of size (MaxLL+1) */
    pub matchLengthFreq: *mut u32, /* table of matchLength statistics, of size (MaxML+1) */
    pub offCodeFreq: *mut u32,     /* table of offCode statistics, of size (MaxOff+1) */
    pub matchTable: *mut ZSTD_match_t, /* list of found matches, of size ZSTD_OPT_SIZE */
    pub priceTable: *mut ZSTD_optimal_t, /* All positions tracked by optimal parser */

    pub litSum: U32,                  /* nb of literals */
    pub litLengthSum: U32,            /* nb of litLength codes */
    pub matchLengthSum: U32,          /* nb of matchLength codes */
    pub offCodeSum: U32,              /* nb of offset codes */
    pub litSumBasePrice: U32,         /* to compare to log2(litfreq) */
    pub litLengthSumBasePrice: U32,   /* to compare to log2(llfreq) */
    pub matchLengthSumBasePrice: U32, /* to compare to log2(mlfreq) */
    pub offCodeSumBasePrice: U32,     /* to compare to log2(offreq) */
    pub priceType: ZSTD_OptPrice_e,   /* prices can be dynamic or predefined */
    pub symbolCosts: *const ZSTD_entropyCTables_t, /* pre-calculated dictionary statistics */
    pub literalCompressionMode: ZSTD_ParamSwitch_e,
}

#[repr(C)]
pub struct ZSTD_compressedBlockState_t {
    pub entropy: ZSTD_entropyCTables_t,
    pub rep: [U32; ZSTD_REP_NUM],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ZSTD_window_t {
    pub nextSrc: *const u8, /* next block here to continue on current prefix */
    pub base: *const u8,    /* All regular indexes relative to this position */
    pub dictBase: *const u8, /* extDict indexes relative to this position */
    pub dictLimit: U32,     /* below that point, need extDict */
    pub lowLimit: U32,      /* below that point, no more valid data */
    pub nbOverflowCorrections: U32, /* Number of overflow corrections since window_init() */
}

#[repr(C)]
pub struct ZSTD_MatchState_t {
    pub window: ZSTD_window_t, /* State for window round buffer management */
    pub loadedDictEnd: U32,    /* index of end of dictionary, within context's referential */
    pub nextToUpdate: U32,     /* index from which to continue table update */
    pub hashLog3: U32,         /* dispatch table for matches of len==3 */

    pub rowHashLog: U32,       /* row-based matchfinder: Hashlog based on nb of rows */
    pub tagTable: *mut u8,     /* row-based matchFinder: hashes and head index */
    pub hashCache: [U32; ZSTD_ROW_HASH_CACHE_SIZE], /* cache of hashes to improve speed */
    pub hashSalt: U64,         /* salts the hash for reuse of tag table */
    pub hashSaltEntropy: U32,  /* collects entropy for salt generation */

    pub hashTable: *mut U32,
    pub hashTable3: *mut U32,
    pub chainTable: *mut U32,

    pub forceNonContiguous: i32, /* force non-contiguous load for the next window update */

    pub dedicatedDictSearch: i32, /* using the dedicated dictionary search structure */
    pub opt: optState_t,          /* optimal parser state */
    pub dictMatchState: *const ZSTD_MatchState_t,
    pub cParams: ZSTD_compressionParameters,
    pub ldmSeqStore: *const RawSeqStore_t,

    pub prefetchCDictTables: i32, /* controls prefetching in some dictMatchState matchfinders */

    pub lazySkipping: i32, /* lazy match finders only insert positions they search when != 0 */
}

#[repr(C)]
pub struct ZSTD_blockState_t {
    pub prevCBlock: *mut ZSTD_compressedBlockState_t,
    pub nextCBlock: *mut ZSTD_compressedBlockState_t,
    pub matchState: ZSTD_MatchState_t,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ldmEntry_t {
    pub offset: U32,
    pub checksum: U32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ldmMatchCandidate_t {
    pub split: *const u8,
    pub hash: U32,
    pub checksum: U32,
    pub bucket: *mut ldmEntry_t,
}

#[repr(C)]
pub struct ldmState_t {
    pub window: ZSTD_window_t, /* State for the window round buffer management */
    pub hashTable: *mut ldmEntry_t,
    pub loadedDictEnd: U32,
    pub bucketOffsets: *mut u8, /* Next position in bucket to insert entry */
    pub splitIndices: [usize; LDM_BATCH_SIZE],
    pub matchCandidates: [ldmMatchCandidate_t; LDM_BATCH_SIZE],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ldmParams_t {
    pub enableLdm: ZSTD_ParamSwitch_e, /* ZSTD_ps_enable to enable LDM. ZSTD_ps_auto by default */
    pub hashLog: U32,                  /* Log size of hashTable */
    pub bucketSizeLog: U32,            /* Log bucket size for collision resolution, at most 8 */
    pub minMatchLength: U32,           /* Minimum match length */
    pub hashRateLog: U32,              /* Log number of entries to skip */
    pub windowLog: U32,                /* Window log for the LDM */
}

#[repr(C)]
pub struct SeqCollector {
    pub collectSequences: i32,
    pub seqStart: *mut ZSTD_Sequence,
    pub seqIndex: usize,
    pub maxSequences: usize,
}

#[repr(C)]
pub struct ZSTD_CCtx_params_s {
    pub format: ZSTD_format_e,
    pub cParams: ZSTD_compressionParameters,
    pub fParams: ZSTD_frameParameters,

    pub compressionLevel: i32,
    pub forceWindow: i32, /* force back-references to respect limit of 1<<wLog */
    pub targetCBlockSize: usize, /* Tries to fit compressed block size around targetCBlockSize */
    pub srcSizeHint: i32, /* User's best guess of source size */

    pub attachDictPref: ZSTD_dictAttachPref_e,
    pub literalCompressionMode: ZSTD_ParamSwitch_e,

    /* Multithreading: used to pass parameters to mtctx */
    pub nbWorkers: i32,
    pub jobSize: usize,
    pub overlapLog: i32,
    pub rsyncable: i32,

    /* Long distance matching parameters */
    pub ldmParams: ldmParams_t,

    /* Dedicated dict search algorithm trigger */
    pub enableDedicatedDictSearch: i32,

    /* Input/output buffer modes */
    pub inBufferMode: ZSTD_bufferMode_e,
    pub outBufferMode: ZSTD_bufferMode_e,

    /* Sequence compression API */
    pub blockDelimiters: ZSTD_SequenceFormat_e,
    pub validateSequences: i32,

    /* Block splitting */
    pub postBlockSplitter: ZSTD_ParamSwitch_e,
    pub preBlockSplitter_level: i32,

    /* Adjust the max block size */
    pub maxBlockSize: usize,

    /* Param for deciding whether to use row-based matchfinder */
    pub useRowMatchFinder: ZSTD_ParamSwitch_e,

    /* Always load a dictionary in ext-dict mode (not prefix mode)? */
    pub deterministicRefPrefix: i32,

    /* Internal use, for createCCtxParams() and freeCCtxParams() only */
    pub customMem: ZSTD_customMem,

    /* Controls prefetching in some dictMatchState matchfinders */
    pub prefetchCDictTables: ZSTD_ParamSwitch_e,

    /* Controls whether zstd will fall back to an internal matchfinder */
    pub enableMatchFinderFallback: i32,

    /* Parameters for the external sequence producer API */
    pub extSeqProdState: *mut c_void,
    pub extSeqProdFunc: ZSTD_sequenceProducer_F,

    /* Controls repcode search in external sequence parsing */
    pub searchForExternalRepcodes: ZSTD_ParamSwitch_e,
}
pub type ZSTD_CCtx_params = ZSTD_CCtx_params_s;

// ZSTD_buffered_policy_e
pub type ZSTD_buffered_policy_e = u32;
pub const ZSTDb_not_buffered: ZSTD_buffered_policy_e = 0;
pub const ZSTDb_buffered: ZSTD_buffered_policy_e = 1;

#[repr(C)]
pub struct ZSTD_blockSplitCtx {
    pub fullSeqStoreChunk: SeqStore_t,
    pub firstHalfSeqStore: SeqStore_t,
    pub secondHalfSeqStore: SeqStore_t,
    pub currSeqStore: SeqStore_t,
    pub nextSeqStore: SeqStore_t,

    pub partitions: [U32; ZSTD_MAX_NB_BLOCK_SPLITS],
    pub entropyMetadata: ZSTD_entropyCTablesMetadata_t,
}

#[repr(C)]
pub struct ZSTD_CCtx_s {
    pub stage: ZSTD_compressionStage_e,
    pub cParamsChanged: i32, /* == 1 if cParams or compression level changed in requestedParams */
    pub bmi2: i32,           /* == 1 if the CPU supports BMI2 */
    pub requestedParams: ZSTD_CCtx_params,
    pub appliedParams: ZSTD_CCtx_params,
    pub simpleApiParams: ZSTD_CCtx_params, /* Param storage used by the simple API - not sticky */
    pub dictID: U32,
    pub dictContentSize: usize,

    pub workspace: crate::compress::zstd_cwksp::ZSTD_cwksp, /* manages buffer for dynamic allocations */
    pub blockSizeMax: usize,
    pub pledgedSrcSizePlusOne: core::ffi::c_ulonglong, /* 0 (default) == unknown */
    pub consumedSrcSize: core::ffi::c_ulonglong,
    pub producedCSize: core::ffi::c_ulonglong,
    pub xxhState: XXH64_state_t,
    pub customMem: ZSTD_customMem,
    pub pool: *mut ZSTD_threadPool,
    pub staticSize: usize,
    pub seqCollector: SeqCollector,
    pub isFirstBlock: i32,
    pub initialized: i32,

    pub seqStore: SeqStore_t, /* sequences storage ptrs */
    pub ldmState: ldmState_t, /* long distance matching state */
    pub ldmSequences: *mut rawSeq, /* Storage for the ldm output sequences */
    pub maxNbLdmSequences: usize,
    pub externSeqStore: RawSeqStore_t, /* Mutable reference to external sequences */
    pub blockState: ZSTD_blockState_t,
    pub tmpWorkspace: *mut c_void, /* used as substitute of stack space */
    pub tmpWkspSize: usize,

    /* Whether we are streaming or not */
    pub bufferedPolicy: ZSTD_buffered_policy_e,

    /* streaming */
    pub inBuff: *mut core::ffi::c_char,
    pub inBuffSize: usize,
    pub inToCompress: usize,
    pub inBuffPos: usize,
    pub inBuffTarget: usize,
    pub outBuff: *mut core::ffi::c_char,
    pub outBuffSize: usize,
    pub outBuffContentSize: usize,
    pub outBuffFlushedSize: usize,
    pub streamStage: ZSTD_cStreamStage,
    pub frameEnded: U32,

    /* Stable in/out buffer verification */
    pub expectedInBuffer: ZSTD_inBuffer,
    pub stableIn_notConsumed: usize, /* nb bytes within stable input buffer said consumed but not */
    pub expectedOutBufferSize: usize,

    /* Dictionary */
    pub localDict: ZSTD_localDict,
    pub cdict: *const ZSTD_CDict,
    pub prefixDict: ZSTD_prefixDict, /* single-usage dictionary */

    /* Multi-threading: ZSTD_MULTITHREAD not defined, so mtctx omitted. */

    /* Tracing (ZSTD_TRACE=1): traceCtx is a plain u64 field. */
    pub traceCtx: ZSTD_TraceCtx,

    /* Workspace for block splitter */
    pub blockSplitCtx: ZSTD_blockSplitCtx,

    /* Buffer for output from external sequence producer */
    pub extSeqBuf: *mut ZSTD_Sequence,
    pub extSeqBufCapacity: usize,
}
pub type ZSTD_CCtx = ZSTD_CCtx_s;
pub type ZSTD_CStream = ZSTD_CCtx;

// ZSTD_dictTableLoadMethod_e
pub type ZSTD_dictTableLoadMethod_e = u32;
pub const ZSTD_dtlm_fast: ZSTD_dictTableLoadMethod_e = 0;
pub const ZSTD_dtlm_full: ZSTD_dictTableLoadMethod_e = 1;

// ZSTD_tableFillPurpose_e
pub type ZSTD_tableFillPurpose_e = u32;
pub const ZSTD_tfp_forCCtx: ZSTD_tableFillPurpose_e = 0;
pub const ZSTD_tfp_forCDict: ZSTD_tableFillPurpose_e = 1;

// ZSTD_dictMode_e
pub type ZSTD_dictMode_e = u32;
pub const ZSTD_noDict: ZSTD_dictMode_e = 0;
pub const ZSTD_extDict: ZSTD_dictMode_e = 1;
pub const ZSTD_dictMatchState: ZSTD_dictMode_e = 2;
pub const ZSTD_dedicatedDictSearch: ZSTD_dictMode_e = 3;

// ZSTD_CParamMode_e
pub type ZSTD_CParamMode_e = u32;
pub const ZSTD_cpm_noAttachDict: ZSTD_CParamMode_e = 0;
pub const ZSTD_cpm_attachDict: ZSTD_CParamMode_e = 1;
pub const ZSTD_cpm_createCDict: ZSTD_CParamMode_e = 2;
pub const ZSTD_cpm_unknown: ZSTD_CParamMode_e = 3;

pub type ZSTD_BlockCompressor_f = Option<
    unsafe extern "C" fn(
        *mut ZSTD_MatchState_t,
        *mut SeqStore_t,
        *mut U32, /* rep[ZSTD_REP_NUM] */
        *const c_void,
        usize,
    ) -> usize,
>;

extern "C" {
    /* Defined in zstd_compress.c (public API). */
    pub fn ZSTD_cParam_getBounds(cParam: ZSTD_cParameter) -> ZSTD_bounds;
}

/*-*************************************
*  offBase sum-type helpers (macros in C)
***************************************/
#[inline]
pub fn REPCODE_TO_OFFBASE(r: U32) -> U32 {
    debug_assert!(r >= 1);
    debug_assert!(r <= ZSTD_REP_NUM as U32);
    r
}
pub const REPCODE1_TO_OFFBASE: U32 = 1;
pub const REPCODE2_TO_OFFBASE: U32 = 2;
pub const REPCODE3_TO_OFFBASE: U32 = 3;
#[inline]
pub fn OFFSET_TO_OFFBASE(o: U32) -> U32 {
    debug_assert!(o > 0);
    o + ZSTD_REP_NUM as U32
}
#[inline]
pub fn OFFBASE_IS_OFFSET(o: U32) -> bool {
    o > ZSTD_REP_NUM as U32
}
#[inline]
pub fn OFFBASE_IS_REPCODE(o: U32) -> bool {
    1 <= o && o <= ZSTD_REP_NUM as U32
}
#[inline]
pub fn OFFBASE_TO_OFFSET(o: U32) -> U32 {
    debug_assert!(OFFBASE_IS_OFFSET(o));
    o - ZSTD_REP_NUM as U32
}
#[inline]
pub fn OFFBASE_TO_REPCODE(o: U32) -> U32 {
    debug_assert!(OFFBASE_IS_REPCODE(o));
    o
}

/// Returns the ZSTD_SequenceLength for the given sequences. Handles decoding of
/// long sequences and adds MINMATCH back to matchLength.
#[inline]
pub unsafe fn ZSTD_getSequenceLength(
    seqStore: *const SeqStore_t,
    seq: *const SeqDef,
) -> ZSTD_SequenceLength {
    let mut seqLen = ZSTD_SequenceLength {
        litLength: (*seq).litLength as U32,
        matchLength: (*seq).mlBase as U32 + MINMATCH,
    };
    if (*seqStore).longLengthPos == seq.offset_from((*seqStore).sequencesStart) as U32 {
        if (*seqStore).longLengthType == ZSTD_llt_literalLength {
            seqLen.litLength += 0x10000;
        }
        if (*seqStore).longLengthType == ZSTD_llt_matchLength {
            seqLen.matchLength += 0x10000;
        }
    }
    seqLen
}

#[inline]
pub fn ZSTD_LLcode(litLength: U32) -> U32 {
    static LL_Code: [u8; 64] = [
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 16, 17, 17, 18, 18, 19, 19, 20,
        20, 20, 20, 21, 21, 21, 21, 22, 22, 22, 22, 22, 22, 22, 22, 23, 23, 23, 23, 23, 23, 23, 23,
        24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24,
    ];
    const LL_deltaCode: U32 = 19;
    if litLength > 63 {
        ZSTD_highbit32(litLength) + LL_deltaCode
    } else {
        LL_Code[litLength as usize] as U32
    }
}

#[inline]
pub fn ZSTD_MLcode(mlBase: U32) -> U32 {
    static ML_Code: [u8; 128] = [
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
        25, 26, 27, 28, 29, 30, 31, 32, 32, 33, 33, 34, 34, 35, 35, 36, 36, 36, 36, 37, 37, 37, 37,
        38, 38, 38, 38, 38, 38, 38, 38, 39, 39, 39, 39, 39, 39, 39, 39, 40, 40, 40, 40, 40, 40, 40,
        40, 40, 40, 40, 40, 40, 40, 40, 40, 41, 41, 41, 41, 41, 41, 41, 41, 41, 41, 41, 41, 41, 41,
        41, 41, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42,
        42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42,
    ];
    const ML_deltaCode: U32 = 36;
    if mlBase > 127 {
        ZSTD_highbit32(mlBase) + ML_deltaCode
    } else {
        ML_Code[mlBase as usize] as U32
    }
}

/// @return 1 if value is within cParam bounds, 0 otherwise
#[inline]
pub unsafe fn ZSTD_cParam_withinBounds(cParam: ZSTD_cParameter, value: i32) -> i32 {
    let bounds = ZSTD_cParam_getBounds(cParam);
    if err_is_error(bounds.error) != 0 {
        return 0;
    }
    if value < bounds.lowerBound {
        return 0;
    }
    if value > bounds.upperBound {
        return 0;
    }
    1
}

/// @return index >= lowLimit ? candidate : backup
#[inline]
pub fn ZSTD_selectAddr(
    index: U32,
    lowLimit: U32,
    candidate: *const u8,
    backup: *const u8,
) -> *const u8 {
    if index >= lowLimit {
        candidate
    } else {
        backup
    }
}

/// Writes uncompressed block to dst buffer from given src. Returns block size.
#[inline]
pub unsafe fn ZSTD_noCompressBlock(
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
    lastBlock: U32,
) -> usize {
    let cBlockHeader24: U32 = lastBlock + ((bt_raw) << 1) + ((srcSize << 3) as U32);
    if srcSize + ZSTD_blockHeaderSize > dstCapacity {
        return crate::common::error::error(crate::common::error::code::DSTSIZE_TOOSMALL);
    }
    MEM_writeLE24(dst, cBlockHeader24);
    core::ptr::copy_nonoverlapping(
        src as *const u8,
        (dst as *mut u8).add(ZSTD_blockHeaderSize),
        srcSize,
    );
    ZSTD_blockHeaderSize + srcSize
}

#[inline]
pub unsafe fn ZSTD_rleCompressBlock(
    dst: *mut c_void,
    dstCapacity: usize,
    src: u8,
    srcSize: usize,
    lastBlock: U32,
) -> usize {
    let op = dst as *mut u8;
    let cBlockHeader: U32 = lastBlock + ((bt_rle) << 1) + ((srcSize << 3) as U32);
    if dstCapacity < 4 {
        return crate::common::error::error(crate::common::error::code::DSTSIZE_TOOSMALL);
    }
    MEM_writeLE24(op as *mut c_void, cBlockHeader);
    *op.add(3) = src;
    4
}

/// Minimum compression required to generate a compress block or compressed
/// literals section.
#[inline]
pub unsafe fn ZSTD_minGain(srcSize: usize, strat: ZSTD_strategy) -> usize {
    let minlog: U32 = if strat >= crate::zstd_h::ZSTD_btultra {
        (strat as U32) - 1
    } else {
        6
    };
    const _: () = assert!(crate::zstd_h::ZSTD_btultra == 8);
    debug_assert!(ZSTD_cParam_withinBounds(ZSTD_c_strategy, strat as i32) != 0);
    (srcSize >> minlog) + 2
}

#[inline]
pub unsafe fn ZSTD_literalsCompressionIsDisabled(cctxParams: *const ZSTD_CCtx_params) -> i32 {
    let mode = (*cctxParams).literalCompressionMode;
    if mode == ZSTD_ps_enable {
        0
    } else if mode == ZSTD_ps_disable {
        1
    } else {
        // ZSTD_ps_auto (default; impossible values are pre-validated away)
        (((*cctxParams).cParams.strategy == crate::zstd_h::ZSTD_fast)
            && ((*cctxParams).cParams.targetLength > 0)) as i32
    }
}

/// memcpy() that won't read beyond WILDCOPY_OVERLENGTH bytes past ilimit_w.
pub unsafe fn ZSTD_safecopyLiterals(
    mut op: *mut u8,
    mut ip: *const u8,
    iend: *const u8,
    ilimit_w: *const u8,
) {
    debug_assert!(iend > ilimit_w);
    if ip <= ilimit_w {
        ZSTD_wildcopy(
            op as *mut c_void,
            ip as *const c_void,
            (ilimit_w as isize) - (ip as isize),
            ZSTD_no_overlap,
        );
        op = op.offset((ilimit_w as isize) - (ip as isize));
        ip = ilimit_w;
    }
    while ip < iend {
        *op = *ip;
        op = op.add(1);
        ip = ip.add(1);
    }
}

/// Store a sequence (litlen, offBase, matchLength) into SeqStore_t.
/// Literals themselves are not copied.
#[inline]
pub unsafe fn ZSTD_storeSeqOnly(
    seqStorePtr: *mut SeqStore_t,
    litLength: usize,
    offBase: U32,
    matchLength: usize,
) {
    debug_assert!(
        ((*seqStorePtr).sequences).offset_from((*seqStorePtr).sequencesStart) < (*seqStorePtr).maxNbSeq as isize
    );

    /* literal Length */
    debug_assert!(litLength <= crate::zstd_h::ZSTD_BLOCKSIZE_MAX);
    if litLength > 0xFFFF {
        debug_assert!((*seqStorePtr).longLengthType == ZSTD_llt_none);
        (*seqStorePtr).longLengthType = ZSTD_llt_literalLength;
        (*seqStorePtr).longLengthPos =
            ((*seqStorePtr).sequences).offset_from((*seqStorePtr).sequencesStart) as U32;
    }
    (*(*seqStorePtr).sequences).litLength = litLength as U16;

    /* match offset */
    (*(*seqStorePtr).sequences).offBase = offBase;

    /* match Length */
    debug_assert!(matchLength <= crate::zstd_h::ZSTD_BLOCKSIZE_MAX);
    debug_assert!(matchLength >= MINMATCH as usize);
    {
        let mlBase = matchLength - MINMATCH as usize;
        if mlBase > 0xFFFF {
            debug_assert!((*seqStorePtr).longLengthType == ZSTD_llt_none);
            (*seqStorePtr).longLengthType = ZSTD_llt_matchLength;
            (*seqStorePtr).longLengthPos =
                ((*seqStorePtr).sequences).offset_from((*seqStorePtr).sequencesStart) as U32;
        }
        (*(*seqStorePtr).sequences).mlBase = mlBase as U16;
    }

    (*seqStorePtr).sequences = ((*seqStorePtr).sequences).add(1);
}

/// Store a sequence (litlen, litPtr, offBase and matchLength) into SeqStore_t.
/// Allowed to over-read literals up to litLimit.
#[inline]
pub unsafe fn ZSTD_storeSeq(
    seqStorePtr: *mut SeqStore_t,
    litLength: usize,
    literals: *const u8,
    litLimit: *const u8,
    offBase: U32,
    matchLength: usize,
) {
    let litLimit_w = litLimit.offset(-(WILDCOPY_OVERLENGTH as isize));
    let litEnd = literals.add(litLength);
    debug_assert!(
        ((*seqStorePtr).sequences).offset_from((*seqStorePtr).sequencesStart) < (*seqStorePtr).maxNbSeq as isize
    );
    /* copy Literals */
    debug_assert!((*seqStorePtr).maxNbLit <= 128 * (1 << 10));
    debug_assert!(
        ((*seqStorePtr).lit).add(litLength) <= ((*seqStorePtr).litStart).add((*seqStorePtr).maxNbLit)
    );
    debug_assert!(literals.add(litLength) <= litLimit);
    if litEnd <= litLimit_w {
        /* Common case we can use wildcopy. First copy 16 bytes. */
        ZSTD_copy16((*seqStorePtr).lit as *mut c_void, literals as *const c_void);
        if litLength > 16 {
            ZSTD_wildcopy(
                ((*seqStorePtr).lit).add(16) as *mut c_void,
                literals.add(16) as *const c_void,
                (litLength as isize) - 16,
                ZSTD_no_overlap,
            );
        }
    } else {
        ZSTD_safecopyLiterals((*seqStorePtr).lit, literals, litEnd, litLimit_w);
    }
    (*seqStorePtr).lit = ((*seqStorePtr).lit).add(litLength);

    ZSTD_storeSeqOnly(seqStorePtr, litLength, offBase, matchLength);
}

/// updates in-place @rep (array of repeat offsets)
#[inline]
pub unsafe fn ZSTD_updateRep(rep: *mut U32, offBase: U32, ll0: U32) {
    if OFFBASE_IS_OFFSET(offBase) {
        /* full offset */
        *rep.add(2) = *rep.add(1);
        *rep.add(1) = *rep.add(0);
        *rep.add(0) = OFFBASE_TO_OFFSET(offBase);
    } else {
        /* repcode */
        let repCode = OFFBASE_TO_REPCODE(offBase) - 1 + ll0;
        if repCode > 0 {
            let currentOffset = if repCode == ZSTD_REP_NUM as U32 {
                *rep.add(0) - 1
            } else {
                *rep.add(repCode as usize)
            };
            *rep.add(2) = if repCode >= 2 { *rep.add(1) } else { *rep.add(2) };
            *rep.add(1) = *rep.add(0);
            *rep.add(0) = currentOffset;
        } else {
            /* repCode == 0, nothing to do */
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Repcodes_t {
    pub rep: [U32; 3],
}

#[inline]
pub unsafe fn ZSTD_newRep(rep: *const U32, offBase: U32, ll0: U32) -> Repcodes_t {
    let mut newReps = Repcodes_t { rep: [0; 3] };
    core::ptr::copy_nonoverlapping(rep, newReps.rep.as_mut_ptr(), 3);
    ZSTD_updateRep(newReps.rep.as_mut_ptr(), offBase, ll0);
    newReps
}

/*-*************************************
*  Match length counter
***************************************/
#[inline]
pub unsafe fn ZSTD_count(pIn: *const u8, pMatch: *const u8, pInLimit: *const u8) -> usize {
    let pStart = pIn;
    let mut pIn = pIn;
    let mut pMatch = pMatch;
    let pInLoopLimit = pInLimit.offset(-((core::mem::size_of::<usize>() - 1) as isize));

    if pIn < pInLoopLimit {
        {
            let diff = MEM_readST(pMatch as *const c_void) ^ MEM_readST(pIn as *const c_void);
            if diff != 0 {
                return ZSTD_NbCommonBytes(diff) as usize;
            }
        }
        pIn = pIn.add(core::mem::size_of::<usize>());
        pMatch = pMatch.add(core::mem::size_of::<usize>());
        while pIn < pInLoopLimit {
            let diff = MEM_readST(pMatch as *const c_void) ^ MEM_readST(pIn as *const c_void);
            if diff == 0 {
                pIn = pIn.add(core::mem::size_of::<usize>());
                pMatch = pMatch.add(core::mem::size_of::<usize>());
                continue;
            }
            pIn = pIn.add(ZSTD_NbCommonBytes(diff) as usize);
            return pIn.offset_from(pStart) as usize;
        }
    }
    if MEM_64bits() != 0
        && pIn < pInLimit.offset(-3)
        && MEM_read32(pMatch as *const c_void) == MEM_read32(pIn as *const c_void)
    {
        pIn = pIn.add(4);
        pMatch = pMatch.add(4);
    }
    if pIn < pInLimit.offset(-1)
        && MEM_read16(pMatch as *const c_void) == MEM_read16(pIn as *const c_void)
    {
        pIn = pIn.add(2);
        pMatch = pMatch.add(2);
    }
    if pIn < pInLimit && *pMatch == *pIn {
        pIn = pIn.add(1);
    }
    pIn.offset_from(pStart) as usize
}

/// Can count match length with `ip` & `match` in 2 different segments.
#[inline]
pub unsafe fn ZSTD_count_2segments(
    ip: *const u8,
    r#match: *const u8,
    iEnd: *const u8,
    mEnd: *const u8,
    iStart: *const u8,
) -> usize {
    let ip_plus = ip.offset(mEnd.offset_from(r#match));
    let vEnd = if ip_plus < iEnd { ip_plus } else { iEnd };
    let matchLength = ZSTD_count(ip, r#match, vEnd);
    if r#match.add(matchLength) != mEnd {
        return matchLength;
    }
    matchLength + ZSTD_count(ip.add(matchLength), iStart, iEnd)
}

/*-*************************************
 *  Hashes
 ***************************************/
const prime3bytes: U32 = 506832829;
#[inline]
fn ZSTD_hash3(u: U32, h: U32, s: U32) -> U32 {
    debug_assert!(h <= 32);
    (((u << (32 - 24)).wrapping_mul(prime3bytes)) ^ s) >> (32 - h)
}
#[inline]
pub unsafe fn ZSTD_hash3Ptr(ptr: *const c_void, h: U32) -> usize {
    ZSTD_hash3(MEM_readLE32(ptr), h, 0) as usize
}
#[inline]
pub unsafe fn ZSTD_hash3PtrS(ptr: *const c_void, h: U32, s: U32) -> usize {
    ZSTD_hash3(MEM_readLE32(ptr), h, s) as usize
}

const prime4bytes: U32 = 2654435761;
#[inline]
fn ZSTD_hash4(u: U32, h: U32, s: U32) -> U32 {
    debug_assert!(h <= 32);
    ((u.wrapping_mul(prime4bytes)) ^ s) >> (32 - h)
}
#[inline]
pub unsafe fn ZSTD_hash4Ptr(ptr: *const c_void, h: U32) -> usize {
    ZSTD_hash4(MEM_readLE32(ptr), h, 0) as usize
}
#[inline]
pub unsafe fn ZSTD_hash4PtrS(ptr: *const c_void, h: U32, s: U32) -> usize {
    ZSTD_hash4(MEM_readLE32(ptr), h, s) as usize
}

const prime5bytes: U64 = 889523592379;
#[inline]
fn ZSTD_hash5(u: U64, h: U32, s: U64) -> usize {
    debug_assert!(h <= 64);
    ((((u << (64 - 40)).wrapping_mul(prime5bytes)) ^ s) >> (64 - h)) as usize
}
#[inline]
pub unsafe fn ZSTD_hash5Ptr(p: *const c_void, h: U32) -> usize {
    ZSTD_hash5(MEM_readLE64(p), h, 0)
}
#[inline]
pub unsafe fn ZSTD_hash5PtrS(p: *const c_void, h: U32, s: U64) -> usize {
    ZSTD_hash5(MEM_readLE64(p), h, s)
}

const prime6bytes: U64 = 227718039650203;
#[inline]
fn ZSTD_hash6(u: U64, h: U32, s: U64) -> usize {
    debug_assert!(h <= 64);
    ((((u << (64 - 48)).wrapping_mul(prime6bytes)) ^ s) >> (64 - h)) as usize
}
#[inline]
pub unsafe fn ZSTD_hash6Ptr(p: *const c_void, h: U32) -> usize {
    ZSTD_hash6(MEM_readLE64(p), h, 0)
}
#[inline]
pub unsafe fn ZSTD_hash6PtrS(p: *const c_void, h: U32, s: U64) -> usize {
    ZSTD_hash6(MEM_readLE64(p), h, s)
}

const prime7bytes: U64 = 58295818150454627;
#[inline]
fn ZSTD_hash7(u: U64, h: U32, s: U64) -> usize {
    debug_assert!(h <= 64);
    ((((u << (64 - 56)).wrapping_mul(prime7bytes)) ^ s) >> (64 - h)) as usize
}
#[inline]
pub unsafe fn ZSTD_hash7Ptr(p: *const c_void, h: U32) -> usize {
    ZSTD_hash7(MEM_readLE64(p), h, 0)
}
#[inline]
pub unsafe fn ZSTD_hash7PtrS(p: *const c_void, h: U32, s: U64) -> usize {
    ZSTD_hash7(MEM_readLE64(p), h, s)
}

const prime8bytes: U64 = 0xCF1BBCDCB7A56463;
#[inline]
fn ZSTD_hash8(u: U64, h: U32, s: U64) -> usize {
    debug_assert!(h <= 64);
    (((u.wrapping_mul(prime8bytes)) ^ s) >> (64 - h)) as usize
}
#[inline]
pub unsafe fn ZSTD_hash8Ptr(p: *const c_void, h: U32) -> usize {
    ZSTD_hash8(MEM_readLE64(p), h, 0)
}
#[inline]
pub unsafe fn ZSTD_hash8PtrS(p: *const c_void, h: U32, s: U64) -> usize {
    ZSTD_hash8(MEM_readLE64(p), h, s)
}

#[inline]
pub unsafe fn ZSTD_hashPtr(p: *const c_void, hBits: U32, mls: U32) -> usize {
    debug_assert!(hBits <= 32);
    match mls {
        5 => ZSTD_hash5Ptr(p, hBits),
        6 => ZSTD_hash6Ptr(p, hBits),
        7 => ZSTD_hash7Ptr(p, hBits),
        8 => ZSTD_hash8Ptr(p, hBits),
        _ => ZSTD_hash4Ptr(p, hBits), /* default and case 4 */
    }
}

#[inline]
pub unsafe fn ZSTD_hashPtrSalted(p: *const c_void, hBits: U32, mls: U32, hashSalt: U64) -> usize {
    debug_assert!(hBits <= 32);
    match mls {
        5 => ZSTD_hash5PtrS(p, hBits, hashSalt),
        6 => ZSTD_hash6PtrS(p, hBits, hashSalt),
        7 => ZSTD_hash7PtrS(p, hBits, hashSalt),
        8 => ZSTD_hash8PtrS(p, hBits, hashSalt),
        _ => ZSTD_hash4PtrS(p, hBits, hashSalt as U32), /* default and case 4 */
    }
}

/// Return base^exponent.
fn ZSTD_ipow(base: U64, exponent: U64) -> U64 {
    let mut power: U64 = 1;
    let mut base = base;
    let mut exponent = exponent;
    while exponent != 0 {
        if exponent & 1 != 0 {
            power = power.wrapping_mul(base);
        }
        exponent >>= 1;
        base = base.wrapping_mul(base);
    }
    power
}

pub const ZSTD_ROLL_HASH_CHAR_OFFSET: U64 = 10;

/// Add the buffer to the hash value.
unsafe fn ZSTD_rollingHash_append(mut hash: U64, buf: *const c_void, size: usize) -> U64 {
    let istart = buf as *const u8;
    let mut pos = 0usize;
    while pos < size {
        hash = hash.wrapping_mul(prime8bytes);
        hash = hash.wrapping_add(*istart.add(pos) as U64 + ZSTD_ROLL_HASH_CHAR_OFFSET);
        pos += 1;
    }
    hash
}

/// Compute the rolling hash value of the buffer.
#[inline]
pub unsafe fn ZSTD_rollingHash_compute(buf: *const c_void, size: usize) -> U64 {
    ZSTD_rollingHash_append(0, buf, size)
}

/// Compute the primePower to be passed to ZSTD_rollingHash_rotate().
#[inline]
pub fn ZSTD_rollingHash_primePower(length: U32) -> U64 {
    ZSTD_ipow(prime8bytes, (length - 1) as U64)
}

/// Rotate the rolling hash by one byte.
#[inline]
pub fn ZSTD_rollingHash_rotate(mut hash: U64, toRemove: u8, toAdd: u8, primePower: U64) -> U64 {
    hash = hash.wrapping_sub(
        (toRemove as U64 + ZSTD_ROLL_HASH_CHAR_OFFSET).wrapping_mul(primePower),
    );
    hash = hash.wrapping_mul(prime8bytes);
    hash = hash.wrapping_add(toAdd as U64 + ZSTD_ROLL_HASH_CHAR_OFFSET);
    hash
}

/*-*************************************
*  Round buffer management
***************************************/
/* Max @current value allowed. 64-bit: 3500 MB. */
#[inline]
pub fn ZSTD_CURRENT_MAX() -> U32 {
    if MEM_64bits() != 0 {
        3500u32 * (1 << 20)
    } else {
        2000u32 * (1 << 20)
    }
}
/* Maximum chunk size before overflow correction needs to be called again */
#[inline]
pub fn ZSTD_CHUNKSIZE_MAX() -> U32 {
    (u32::MAX) - ZSTD_CURRENT_MAX()
}

/* FUZZING_BUILD_MODE not defined -> 0 */
pub const ZSTD_WINDOW_OVERFLOW_CORRECT_FREQUENTLY: i32 = 0;

/// Clears the window containing the history by setting it to empty.
#[inline]
pub unsafe fn ZSTD_window_clear(window: *mut ZSTD_window_t) {
    let endT = (*window).nextSrc.offset_from((*window).base) as usize;
    let end = endT as U32;
    (*window).lowLimit = end;
    (*window).dictLimit = end;
}

#[inline]
pub unsafe fn ZSTD_window_isEmpty(window: ZSTD_window_t) -> U32 {
    (window.dictLimit == ZSTD_WINDOW_START_INDEX
        && window.lowLimit == ZSTD_WINDOW_START_INDEX
        && (window.nextSrc.offset_from(window.base) as U32) == ZSTD_WINDOW_START_INDEX) as U32
}

/// Returns non-zero if the window has a non-empty extDict.
#[inline]
pub fn ZSTD_window_hasExtDict(window: ZSTD_window_t) -> U32 {
    (window.lowLimit < window.dictLimit) as U32
}

/// Inspects the matchState and figures out what dictMode should be passed to
/// the compressor.
#[inline]
pub unsafe fn ZSTD_matchState_dictMode(ms: *const ZSTD_MatchState_t) -> ZSTD_dictMode_e {
    if ZSTD_window_hasExtDict((*ms).window) != 0 {
        ZSTD_extDict
    } else if !(*ms).dictMatchState.is_null() {
        if (*(*ms).dictMatchState).dedicatedDictSearch != 0 {
            ZSTD_dedicatedDictSearch
        } else {
            ZSTD_dictMatchState
        }
    } else {
        ZSTD_noDict
    }
}

/// Returns non-zero if the indices are large enough for overflow correction
/// to work correctly without impacting compression ratio.
#[inline]
pub unsafe fn ZSTD_window_canOverflowCorrect(
    window: ZSTD_window_t,
    cycleLog: U32,
    maxDist: U32,
    loadedDictEnd: U32,
    src: *const c_void,
) -> U32 {
    let cycleSize = 1u32 << cycleLog;
    let curr = (src as *const u8).offset_from(window.base) as U32;
    let minIndexToOverflowCorrect = cycleSize
        .wrapping_add(if maxDist > cycleSize { maxDist } else { cycleSize })
        .wrapping_add(ZSTD_WINDOW_START_INDEX);

    let adjustment = window.nbOverflowCorrections.wrapping_add(1);
    let a = minIndexToOverflowCorrect.wrapping_mul(adjustment);
    let adjustedIndex = if a > minIndexToOverflowCorrect {
        a
    } else {
        minIndexToOverflowCorrect
    };
    let indexLargeEnough = (curr > adjustedIndex) as U32;

    let dictionaryInvalidated = (curr > maxDist.wrapping_add(loadedDictEnd)) as U32;

    indexLargeEnough & dictionaryInvalidated
}

/// Returns non-zero if the indices are getting too large and need overflow
/// protection.
#[inline]
pub unsafe fn ZSTD_window_needOverflowCorrection(
    window: ZSTD_window_t,
    cycleLog: U32,
    maxDist: U32,
    loadedDictEnd: U32,
    src: *const c_void,
    srcEnd: *const c_void,
) -> U32 {
    let curr = (srcEnd as *const u8).offset_from(window.base) as U32;
    if ZSTD_WINDOW_OVERFLOW_CORRECT_FREQUENTLY != 0 {
        if ZSTD_window_canOverflowCorrect(window, cycleLog, maxDist, loadedDictEnd, src) != 0 {
            return 1;
        }
    }
    (curr > ZSTD_CURRENT_MAX()) as U32
}

/// Reduces the indices to protect from index overflow.
#[inline]
pub unsafe fn ZSTD_window_correctOverflow(
    window: *mut ZSTD_window_t,
    cycleLog: U32,
    maxDist: U32,
    src: *const c_void,
) -> U32 {
    let cycleSize = 1u32 << cycleLog;
    let cycleMask = cycleSize - 1;
    let curr = (src as *const u8).offset_from((*window).base) as U32;
    let currentCycle = curr & cycleMask;
    /* Ensure newCurrent - maxDist >= ZSTD_WINDOW_START_INDEX. */
    let currentCycleCorrection = if currentCycle < ZSTD_WINDOW_START_INDEX {
        if cycleSize > ZSTD_WINDOW_START_INDEX {
            cycleSize
        } else {
            ZSTD_WINDOW_START_INDEX
        }
    } else {
        0
    };
    let newCurrent = currentCycle
        .wrapping_add(currentCycleCorrection)
        .wrapping_add(if maxDist > cycleSize { maxDist } else { cycleSize });
    let correction = curr - newCurrent;
    debug_assert!((maxDist & (maxDist - 1)) == 0);
    debug_assert!((curr & cycleMask) == (newCurrent & cycleMask));
    debug_assert!(curr > newCurrent);
    if ZSTD_WINDOW_OVERFLOW_CORRECT_FREQUENTLY == 0 {
        debug_assert!(correction > 1 << 28);
    }

    (*window).base = (*window).base.add(correction as usize);
    (*window).dictBase = (*window).dictBase.add(correction as usize);
    if (*window).lowLimit < correction + ZSTD_WINDOW_START_INDEX {
        (*window).lowLimit = ZSTD_WINDOW_START_INDEX;
    } else {
        (*window).lowLimit -= correction;
    }
    if (*window).dictLimit < correction + ZSTD_WINDOW_START_INDEX {
        (*window).dictLimit = ZSTD_WINDOW_START_INDEX;
    } else {
        (*window).dictLimit -= correction;
    }

    debug_assert!(newCurrent >= maxDist);
    debug_assert!(newCurrent - maxDist >= ZSTD_WINDOW_START_INDEX);
    debug_assert!((*window).lowLimit <= newCurrent);
    debug_assert!((*window).dictLimit <= newCurrent);

    (*window).nbOverflowCorrections += 1;
    correction
}

/// Updates lowLimit so that (srcEnd - base) - lowLimit == maxDist + loadedDictEnd.
#[inline]
pub unsafe fn ZSTD_window_enforceMaxDist(
    window: *mut ZSTD_window_t,
    blockEnd: *const c_void,
    maxDist: U32,
    loadedDictEndPtr: *mut U32,
    dictMatchStatePtr: *mut *const ZSTD_MatchState_t,
) {
    let blockEndIdx = (blockEnd as *const u8).offset_from((*window).base) as U32;
    let loadedDictEnd = if !loadedDictEndPtr.is_null() {
        *loadedDictEndPtr
    } else {
        0
    };

    if blockEndIdx > maxDist + loadedDictEnd {
        let newLowLimit = blockEndIdx - maxDist;
        if (*window).lowLimit < newLowLimit {
            (*window).lowLimit = newLowLimit;
        }
        if (*window).dictLimit < (*window).lowLimit {
            (*window).dictLimit = (*window).lowLimit;
        }
        /* On reaching window size, dictionaries are invalidated */
        if !loadedDictEndPtr.is_null() {
            *loadedDictEndPtr = 0;
        }
        if !dictMatchStatePtr.is_null() {
            *dictMatchStatePtr = core::ptr::null();
        }
    }
}

/// Similar to enforceMaxDist, but only invalidates dictionary when input
/// progresses beyond window size.
#[inline]
pub unsafe fn ZSTD_checkDictValidity(
    window: *const ZSTD_window_t,
    blockEnd: *const c_void,
    maxDist: U32,
    loadedDictEndPtr: *mut U32,
    dictMatchStatePtr: *mut *const ZSTD_MatchState_t,
) {
    debug_assert!(!loadedDictEndPtr.is_null());
    debug_assert!(!dictMatchStatePtr.is_null());
    {
        let blockEndIdx = (blockEnd as *const u8).offset_from((*window).base) as U32;
        let loadedDictEnd = *loadedDictEndPtr;
        debug_assert!(blockEndIdx >= loadedDictEnd);

        if blockEndIdx > loadedDictEnd + maxDist || loadedDictEnd != (*window).dictLimit {
            *loadedDictEndPtr = 0;
            *dictMatchStatePtr = core::ptr::null();
        } else {
            if *loadedDictEndPtr != 0 {
                /* dictionary considered valid for current block */
            }
        }
    }
}

#[inline]
pub unsafe fn ZSTD_window_init(window: *mut ZSTD_window_t) {
    core::ptr::write_bytes(window as *mut u8, 0, core::mem::size_of::<ZSTD_window_t>());
    (*window).base = b" ".as_ptr();
    (*window).dictBase = b" ".as_ptr();
    (*window).dictLimit = ZSTD_WINDOW_START_INDEX;
    (*window).lowLimit = ZSTD_WINDOW_START_INDEX;
    (*window).nextSrc = (*window).base.add(ZSTD_WINDOW_START_INDEX as usize);
    (*window).nbOverflowCorrections = 0;
}

/// Updates the window by appending [src, src + srcSize) to the window.
/// Returns non-zero if the segment is contiguous.
#[inline]
pub unsafe fn ZSTD_window_update(
    window: *mut ZSTD_window_t,
    src: *const c_void,
    srcSize: usize,
    forceNonContiguous: i32,
) -> U32 {
    let ip = src as *const u8;
    let mut contiguous: U32 = 1;
    if srcSize == 0 {
        return contiguous;
    }
    debug_assert!(!(*window).base.is_null());
    debug_assert!(!(*window).dictBase.is_null());
    /* Check if blocks follow each other */
    if src != (*window).nextSrc as *const c_void || forceNonContiguous != 0 {
        /* not contiguous */
        let distanceFromBase = (*window).nextSrc.offset_from((*window).base) as usize;
        (*window).lowLimit = (*window).dictLimit;
        debug_assert!(distanceFromBase == (distanceFromBase as U32) as usize);
        (*window).dictLimit = distanceFromBase as U32;
        (*window).dictBase = (*window).base;
        (*window).base = ip.offset(-(distanceFromBase as isize));
        if (*window).dictLimit - (*window).lowLimit < HASH_READ_SIZE as U32 {
            (*window).lowLimit = (*window).dictLimit;
        }
        contiguous = 0;
    }
    (*window).nextSrc = ip.add(srcSize);
    /* if input and dictionary overlap : reduce dictionary */
    if (ip.add(srcSize) > (*window).dictBase.add((*window).lowLimit as usize))
        && (ip < (*window).dictBase.add((*window).dictLimit as usize))
    {
        let highInputIdx = (ip.add(srcSize)).offset_from((*window).dictBase) as usize;
        let lowLimitMax = if highInputIdx > (*window).dictLimit as usize {
            (*window).dictLimit
        } else {
            highInputIdx as U32
        };
        debug_assert!(highInputIdx < u32::MAX as usize);
        (*window).lowLimit = lowLimitMax;
    }
    contiguous
}

/// Returns the lowest allowed match index. It may be in the ext-dict or prefix.
#[inline]
pub unsafe fn ZSTD_getLowestMatchIndex(
    ms: *const ZSTD_MatchState_t,
    curr: U32,
    windowLog: u32,
) -> U32 {
    let maxDistance = 1u32 << windowLog;
    let lowestValid = (*ms).window.lowLimit;
    let withinWindow = if curr - lowestValid > maxDistance {
        curr - maxDistance
    } else {
        lowestValid
    };
    let isDictionary = ((*ms).loadedDictEnd != 0) as U32;
    let matchLowest = if isDictionary != 0 {
        lowestValid
    } else {
        withinWindow
    };
    matchLowest
}

/// Returns the lowest allowed match index in the prefix.
#[inline]
pub unsafe fn ZSTD_getLowestPrefixIndex(
    ms: *const ZSTD_MatchState_t,
    curr: U32,
    windowLog: u32,
) -> U32 {
    let maxDistance = 1u32 << windowLog;
    let lowestValid = (*ms).window.dictLimit;
    let withinWindow = if curr - lowestValid > maxDistance {
        curr - maxDistance
    } else {
        lowestValid
    };
    let isDictionary = ((*ms).loadedDictEnd != 0) as U32;
    let matchLowest = if isDictionary != 0 {
        lowestValid
    } else {
        withinWindow
    };
    matchLowest
}

/// intentional underflow : ensure repIndex isn't overlapping dict + prefix
#[inline]
pub fn ZSTD_index_overlap_check(prefixLowestIndex: U32, repIndex: U32) -> i32 {
    ((prefixLowestIndex.wrapping_sub(1)).wrapping_sub(repIndex) >= 3) as i32
}

/* Short Cache */
pub const ZSTD_SHORT_CACHE_TAG_BITS: U32 = 8;
pub const ZSTD_SHORT_CACHE_TAG_MASK: U32 = (1u32 << ZSTD_SHORT_CACHE_TAG_BITS) - 1;

/// Unpacks hashAndTag into (hash, tag), then packs (index, tag) into hashTable[hash].
#[inline]
pub unsafe fn ZSTD_writeTaggedIndex(hashTable: *mut U32, hashAndTag: usize, index: U32) {
    let hash = hashAndTag >> ZSTD_SHORT_CACHE_TAG_BITS;
    let tag = (hashAndTag as U32) & ZSTD_SHORT_CACHE_TAG_MASK;
    debug_assert!(index >> (32 - ZSTD_SHORT_CACHE_TAG_BITS) == 0);
    *hashTable.add(hash) = (index << ZSTD_SHORT_CACHE_TAG_BITS) | tag;
}

/// Unpacks tag1 and tag2 from lower bits and checks if the tags match.
#[inline]
pub fn ZSTD_comparePackedTags(packedTag1: usize, packedTag2: usize) -> i32 {
    let tag1 = (packedTag1 as U32) & ZSTD_SHORT_CACHE_TAG_MASK;
    let tag2 = (packedTag2 as U32) & ZSTD_SHORT_CACHE_TAG_MASK;
    (tag1 == tag2) as i32
}

/// SequencePosition (shared internal, used across module boundaries).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ZSTD_SequencePosition {
    pub idx: U32,           /* Index in array of ZSTD_Sequence */
    pub posInSequence: U32, /* Position within sequence at idx */
    pub posInSrc: usize,    /* Number of bytes given by sequences provided so far */
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct BlockSummary {
    pub nbSequences: usize,
    pub blockSize: usize,
    pub litSize: usize,
}

/// Returns 1 if an external sequence producer is registered, otherwise 0.
#[inline]
pub unsafe fn ZSTD_hasExtSeqProd(params: *const ZSTD_CCtx_params) -> i32 {
    (*params).extSeqProdFunc.is_some() as i32
}





