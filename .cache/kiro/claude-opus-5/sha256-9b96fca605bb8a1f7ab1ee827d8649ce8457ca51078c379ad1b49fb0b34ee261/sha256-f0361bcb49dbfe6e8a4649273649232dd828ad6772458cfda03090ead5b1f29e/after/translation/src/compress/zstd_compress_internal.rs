//! Translation of `compress/zstd_compress_internal.h` — internal type
//! definitions, constants and inline helpers shared across `lib/compress`.
//!
//! Literal, semantics-preserving transliteration. Build configuration:
//! `DYNAMIC_BMI2=0`, no `ZSTD_MULTITHREAD`, `ZSTD_TRACE` = 0,
//! `DEBUGLEVEL 0` (asserts / DEBUGLOG dropped).
//!
//! This header carries no exported symbols: everything is `pub` types,
//! `pub const`s and `pub unsafe fn` for the `MEM_STATIC` / inline functions.
//! Function prototypes for symbols defined in `.c` files elsewhere are
//! declared as `extern "C"` so this module type-checks independently.
#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(unused_parens)]

use core::ffi::{c_char, c_int, c_uint, c_void};
use core::ptr::null_mut;

use crate::common::bits::*;
use crate::common::error_private::*;
use crate::common::fse::*;
use crate::common::huf::*;
use crate::common::mem::*;
use crate::common::pool::POOL_ctx;
use crate::common::xxhash::XXH64_state_t;
use crate::common::zstd_common::ZSTD_isError;
use crate::common::zstd_h::*;
use crate::common::zstd_internal::*;

use crate::compress::zstd_cwksp::ZSTD_cwksp;

/* Forward-declared opaque struct: real definition lands in
 * compress/zstdmt_compress.rs. It is only ever reached through a pointer,
 * and (since ZSTD_MULTITHREAD is undefined in this build) is not referenced
 * by any struct field here. Declared for downstream users. */
pub enum ZSTDMT_CCtx {}

/* ZSTD_traceCtx : from zstd_trace.h. ZSTD_TRACE is 1 in this build (weak
 * symbols are available on GCC/ELF), so ZSTD_CCtx_s::traceCtx IS present.
 * completeness. */
pub type ZSTD_traceCtx = u64;

/*-*************************************
*  Constants
***************************************/
pub const kSearchStrength: c_int = 8;
pub const HASH_READ_SIZE: c_int = 8;
/* For btlazy2 strategy, index ZSTD_DUBT_UNSORTED_MARK==1 means "unsorted". */
pub const ZSTD_DUBT_UNSORTED_MARK: U32 = 1;

/* From compress/zstd_preSplit.h (not yet translated in this crate):
 * #define ZSTD_SLIPBLOCK_WORKSPACESIZE 8208 */
pub const ZSTD_SLIPBLOCK_WORKSPACESIZE: size_t = 8208;

/*-*************************************
*  Context memory management
***************************************/
pub type ZSTD_compressionStage_e = c_uint;
pub const ZSTDcs_created: ZSTD_compressionStage_e = 0;
pub const ZSTDcs_init: ZSTD_compressionStage_e = 1;
pub const ZSTDcs_ongoing: ZSTD_compressionStage_e = 2;
pub const ZSTDcs_ending: ZSTD_compressionStage_e = 3;

pub type ZSTD_cStreamStage = c_uint;
pub const zcss_init: ZSTD_cStreamStage = 0;
pub const zcss_load: ZSTD_cStreamStage = 1;
pub const zcss_flush: ZSTD_cStreamStage = 2;

#[repr(C)]
pub struct ZSTD_prefixDict_s {
    pub dict: *const c_void,
    pub dictSize: size_t,
    pub dictContentType: ZSTD_dictContentType_e,
}
pub type ZSTD_prefixDict = ZSTD_prefixDict_s;

#[repr(C)]
pub struct ZSTD_localDict {
    pub dictBuffer: *mut c_void,
    pub dict: *const c_void,
    pub dictSize: size_t,
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
*  Sequences *
***********************************************/
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SeqDef_s {
    pub offBase: U32, /* offBase == Offset + ZSTD_REP_NUM, or repcode 1,2,3 */
    pub litLength: U16,
    pub mlBase: U16, /* mlBase == matchLength - MINMATCH */
}
pub type SeqDef = SeqDef_s;

/* Controls whether seqStore has a single "long" litLength or matchLength. */
pub type ZSTD_longLengthType_e = c_uint;
pub const ZSTD_llt_none: ZSTD_longLengthType_e = 0; /* no longLengthType */
pub const ZSTD_llt_literalLength: ZSTD_longLengthType_e = 1; /* represents a long literal */
pub const ZSTD_llt_matchLength: ZSTD_longLengthType_e = 2; /* represents a long match */

#[repr(C)]
pub struct SeqStore_t {
    pub sequencesStart: *mut SeqDef,
    pub sequences: *mut SeqDef, /* ptr to end of sequences */
    pub litStart: *mut BYTE,
    pub lit: *mut BYTE, /* ptr to end of literals */
    pub llCode: *mut BYTE,
    pub mlCode: *mut BYTE,
    pub ofCode: *mut BYTE,
    pub maxNbSeq: size_t,
    pub maxNbLit: size_t,

    /* longLengthPos and longLengthType to allow us to represent either a single litLength
     * or matchLength in the seqStore that has a value larger than U16 (if it exists). */
    pub longLengthType: ZSTD_longLengthType_e,
    pub longLengthPos: U32, /* Index of the sequence to apply long length modification to */
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ZSTD_SequenceLength {
    pub litLength: U32,
    pub matchLength: U32,
}

/**
 * Returns the ZSTD_SequenceLength for the given sequences.
 */
pub unsafe fn ZSTD_getSequenceLength(
    seqStore: *const SeqStore_t,
    seq: *const SeqDef,
) -> ZSTD_SequenceLength {
    let mut seqLen = ZSTD_SequenceLength {
        litLength: 0,
        matchLength: 0,
    };
    seqLen.litLength = (*seq).litLength as U32;
    seqLen.matchLength = (*seq).mlBase as U32 + MINMATCH;
    if (*seqStore).longLengthPos == (seq.offset_from((*seqStore).sequencesStart) as U32) {
        if (*seqStore).longLengthType == ZSTD_llt_literalLength {
            seqLen.litLength += 0x10000;
        }
        if (*seqStore).longLengthType == ZSTD_llt_matchLength {
            seqLen.matchLength += 0x10000;
        }
    }
    seqLen
}

extern "C" {
    /* compress & dictBuilder */
    pub fn ZSTD_getSeqStore(ctx: *const ZSTD_CCtx) -> *const SeqStore_t;
    /* compress, dictBuilder, decodeCorpus */
    pub fn ZSTD_seqToCodes(seqStorePtr: *const SeqStore_t) -> c_int;
}

/***********************************************
*  Entropy buffer statistics structs and funcs *
***********************************************/
#[repr(C)]
pub struct ZSTD_hufCTablesMetadata_t {
    pub hType: SymbolEncodingType_e,
    pub hufDesBuffer: [BYTE; ZSTD_MAX_HUF_HEADER_SIZE],
    pub hufDesSize: size_t,
}

#[repr(C)]
pub struct ZSTD_fseCTablesMetadata_t {
    pub llType: SymbolEncodingType_e,
    pub ofType: SymbolEncodingType_e,
    pub mlType: SymbolEncodingType_e,
    pub fseTablesBuffer: [BYTE; ZSTD_MAX_FSE_HEADERS_SIZE],
    pub fseTablesSize: size_t,
    pub lastCountSize: size_t, /* To account for bug in 1.3.4. */
}

#[repr(C)]
pub struct ZSTD_entropyCTablesMetadata_t {
    pub hufMetadata: ZSTD_hufCTablesMetadata_t,
    pub fseMetadata: ZSTD_fseCTablesMetadata_t,
}

extern "C" {
    /** ZSTD_buildBlockEntropyStats() : @return : 0 on success or error code */
    pub fn ZSTD_buildBlockEntropyStats(
        seqStorePtr: *const SeqStore_t,
        prevEntropy: *const ZSTD_entropyCTables_t,
        nextEntropy: *mut ZSTD_entropyCTables_t,
        cctxParams: *const ZSTD_CCtx_params,
        entropyMetadata: *mut ZSTD_entropyCTablesMetadata_t,
        workspace: *mut c_void,
        wkspSize: size_t,
    ) -> size_t;
}

/*********************************
*  Compression internals structs *
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
    pub offset: U32,      /* Offset of sequence */
    pub litLength: U32,   /* Length of literals prior to match */
    pub matchLength: U32, /* Raw length of match */
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct RawSeqStore_t {
    pub seq: *mut rawSeq,     /* The start of the sequences */
    pub pos: size_t,          /* The index in seq where reading stopped. pos <= size. */
    pub posInSequence: size_t, /* The position within the sequence at seq[pos] where reading stopped. */
    pub size: size_t,         /* The number of sequences. <= capacity. */
    pub capacity: size_t,     /* The capacity starting from `seq` pointer */
}

pub const kNullRawSeqStore: RawSeqStore_t = RawSeqStore_t {
    seq: null_mut(),
    pos: 0,
    posInSequence: 0,
    size: 0,
    capacity: 0,
};

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ZSTD_optimal_t {
    pub price: c_int,             /* price from beginning of segment to this position */
    pub off: U32,                 /* offset of previous match */
    pub mlen: U32,                /* length of previous match */
    pub litlen: U32,              /* nb of literals since previous match */
    pub rep: [U32; ZSTD_REP_NUM], /* offset history after previous match */
}

pub type ZSTD_OptPrice_e = c_uint;
pub const zop_dynamic: ZSTD_OptPrice_e = 0;
pub const zop_predef: ZSTD_OptPrice_e = 1;

pub const ZSTD_OPT_SIZE: size_t = (ZSTD_OPT_NUM as size_t) + 3;

#[repr(C)]
pub struct optState_t {
    /* All tables are allocated inside cctx->workspace by ZSTD_resetCCtx_internal() */
    pub litFreq: *mut c_uint,          /* table of literals statistics, of size 256 */
    pub litLengthFreq: *mut c_uint,    /* table of litLength statistics, of size (MaxLL+1) */
    pub matchLengthFreq: *mut c_uint,  /* table of matchLength statistics, of size (MaxML+1) */
    pub offCodeFreq: *mut c_uint,      /* table of offCode statistics, of size (MaxOff+1) */
    pub matchTable: *mut ZSTD_match_t, /* list of found matches, of size ZSTD_OPT_SIZE */
    pub priceTable: *mut ZSTD_optimal_t, /* All positions tracked by optimal parser, of size ZSTD_OPT_SIZE */

    pub litSum: U32,                  /* nb of literals */
    pub litLengthSum: U32,            /* nb of litLength codes */
    pub matchLengthSum: U32,          /* nb of matchLength codes */
    pub offCodeSum: U32,              /* nb of offset codes */
    pub litSumBasePrice: U32,         /* to compare to log2(litfreq) */
    pub litLengthSumBasePrice: U32,   /* to compare to log2(llfreq)  */
    pub matchLengthSumBasePrice: U32, /* to compare to log2(mlfreq)  */
    pub offCodeSumBasePrice: U32,     /* to compare to log2(offreq)  */
    pub priceType: ZSTD_OptPrice_e,   /* prices can be determined dynamically, or follow a pre-defined cost structure */
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
    pub nextSrc: *const BYTE,       /* next block here to continue on current prefix */
    pub base: *const BYTE,          /* All regular indexes relative to this position */
    pub dictBase: *const BYTE,      /* extDict indexes relative to this position */
    pub dictLimit: U32,             /* below that point, need extDict */
    pub lowLimit: U32,              /* below that point, no more valid data */
    pub nbOverflowCorrections: U32, /* Number of times overflow correction has run since ZSTD_window_init(). */
}

pub const ZSTD_WINDOW_START_INDEX: U32 = 2;

pub const ZSTD_ROW_HASH_CACHE_SIZE: usize = 8; /* Size of prefetching hash cache for row-based matchfinder */

#[repr(C)]
pub struct ZSTD_MatchState_t {
    pub window: ZSTD_window_t, /* State for window round buffer management */
    pub loadedDictEnd: U32,    /* index of end of dictionary, within context's referential. */
    pub nextToUpdate: U32,     /* index from which to continue table update */
    pub hashLog3: U32,         /* dispatch table for matches of len==3 */

    pub rowHashLog: U32, /* For row-based matchfinder: Hashlog based on nb of rows in the hashTable. */
    pub tagTable: *mut BYTE, /* For row-based matchFinder: A row-based table containing the hashes and head index. */
    pub hashCache: [U32; ZSTD_ROW_HASH_CACHE_SIZE], /* For row-based matchFinder: a cache of hashes to improve speed */
    pub hashSalt: U64, /* For row-based matchFinder: salts the hash for reuse of tag table */
    pub hashSaltEntropy: U32, /* For row-based matchFinder: collects entropy for salt generation */

    pub hashTable: *mut U32,
    pub hashTable3: *mut U32,
    pub chainTable: *mut U32,

    pub forceNonContiguous: c_int, /* Non-zero if we should force non-contiguous load for the next window update. */

    pub dedicatedDictSearch: c_int, /* Indicates whether this matchState is using the dedicated dictionary search structure. */
    pub opt: optState_t,            /* optimal parser state */
    pub dictMatchState: *const ZSTD_MatchState_t,
    pub cParams: ZSTD_compressionParameters,
    pub ldmSeqStore: *const RawSeqStore_t,

    /* Controls prefetching in some dictMatchState matchfinders. */
    pub prefetchCDictTables: c_int,

    /* When == 0, lazy match finders insert every position.
     * When != 0, lazy match finders only insert positions they search. */
    pub lazySkipping: c_int,
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
    pub split: *const BYTE,
    pub hash: U32,
    pub checksum: U32,
    pub bucket: *mut ldmEntry_t,
}

pub const LDM_BATCH_SIZE: usize = 64;

#[repr(C)]
pub struct ldmState_t {
    pub window: ZSTD_window_t, /* State for the window round buffer management */
    pub hashTable: *mut ldmEntry_t,
    pub loadedDictEnd: U32,
    pub bucketOffsets: *mut BYTE, /* Next position in bucket to insert entry */
    pub splitIndices: [size_t; LDM_BATCH_SIZE],
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
    pub collectSequences: c_int,
    pub seqStart: *mut ZSTD_Sequence,
    pub seqIndex: size_t,
    pub maxSequences: size_t,
}

#[repr(C)]
pub struct ZSTD_CCtx_params_s {
    pub format: ZSTD_format_e,
    pub cParams: ZSTD_compressionParameters,
    pub fParams: ZSTD_frameParameters,

    pub compressionLevel: c_int,
    pub forceWindow: c_int, /* force back-references to respect limit of 1<<wLog, even for dictionary */
    pub targetCBlockSize: size_t, /* Tries to fit compressed block size to be around targetCBlockSize. */
    pub srcSizeHint: c_int, /* User's best guess of source size. */

    pub attachDictPref: ZSTD_dictAttachPref_e,
    pub literalCompressionMode: ZSTD_ParamSwitch_e,

    /* Multithreading: used to pass parameters to mtctx */
    pub nbWorkers: c_int,
    pub jobSize: size_t,
    pub overlapLog: c_int,
    pub rsyncable: c_int,

    /* Long distance matching parameters */
    pub ldmParams: ldmParams_t,

    /* Dedicated dict search algorithm trigger */
    pub enableDedicatedDictSearch: c_int,

    /* Input/output buffer modes */
    pub inBufferMode: ZSTD_bufferMode_e,
    pub outBufferMode: ZSTD_bufferMode_e,

    /* Sequence compression API */
    pub blockDelimiters: ZSTD_SequenceFormat_e,
    pub validateSequences: c_int,

    /* Block splitting */
    pub postBlockSplitter: ZSTD_ParamSwitch_e,
    pub preBlockSplitter_level: c_int,

    /* Adjust the max block size */
    pub maxBlockSize: size_t,

    /* Param for deciding whether to use row-based matchfinder */
    pub useRowMatchFinder: ZSTD_ParamSwitch_e,

    /* Always load a dictionary in ext-dict mode (not prefix mode)? */
    pub deterministicRefPrefix: c_int,

    /* Internal use, for createCCtxParams() and freeCCtxParams() only */
    pub customMem: crate::common::zstd_internal::ZSTD_customMem,

    /* Controls prefetching in some dictMatchState matchfinders */
    pub prefetchCDictTables: ZSTD_ParamSwitch_e,

    /* Controls whether zstd will fall back to an internal matchfinder
     * if the external matchfinder returns an error code. */
    pub enableMatchFinderFallback: c_int,

    /* Parameters for the external sequence producer API. */
    pub extSeqProdState: *mut c_void,
    pub extSeqProdFunc: ZSTD_sequenceProducer_F,

    /* Controls repcode search in external sequence parsing */
    pub searchForExternalRepcodes: ZSTD_ParamSwitch_e,
}

pub const COMPRESS_SEQUENCES_WORKSPACE_SIZE: size_t =
    core::mem::size_of::<c_uint>() * ((MaxSeq as size_t) + 2);
pub const ENTROPY_WORKSPACE_SIZE: size_t =
    HUF_WORKSPACE_SIZE + COMPRESS_SEQUENCES_WORKSPACE_SIZE;
pub const TMP_WORKSPACE_SIZE: size_t = if ENTROPY_WORKSPACE_SIZE > ZSTD_SLIPBLOCK_WORKSPACESIZE {
    ENTROPY_WORKSPACE_SIZE
} else {
    ZSTD_SLIPBLOCK_WORKSPACESIZE
};

pub type ZSTD_buffered_policy_e = c_uint;
pub const ZSTDb_not_buffered: ZSTD_buffered_policy_e = 0;
pub const ZSTDb_buffered: ZSTD_buffered_policy_e = 1;

pub const ZSTD_MAX_NB_BLOCK_SPLITS: usize = 196;

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
    pub cParamsChanged: c_int, /* == 1 if cParams(except wlog) or compression level are changed in requestedParams. */
    pub bmi2: c_int, /* == 1 if the CPU supports BMI2 and 0 otherwise. */
    pub requestedParams: ZSTD_CCtx_params,
    pub appliedParams: ZSTD_CCtx_params,
    pub simpleApiParams: ZSTD_CCtx_params, /* Param storage used by the simple API - not sticky. */
    pub dictID: U32,
    pub dictContentSize: size_t,

    pub workspace: ZSTD_cwksp, /* manages buffer for dynamic allocations */
    pub blockSizeMax: size_t,
    pub pledgedSrcSizePlusOne: core::ffi::c_ulonglong, /* this way, 0 (default) == unknown */
    pub consumedSrcSize: core::ffi::c_ulonglong,
    pub producedCSize: core::ffi::c_ulonglong,
    pub xxhState: XXH64_state_t,
    pub customMem: crate::common::zstd_internal::ZSTD_customMem,
    pub pool: *mut ZSTD_threadPool,
    pub staticSize: size_t,
    pub seqCollector: SeqCollector,
    pub isFirstBlock: c_int,
    pub initialized: c_int,

    pub seqStore: SeqStore_t, /* sequences storage ptrs */
    pub ldmState: ldmState_t, /* long distance matching state */
    pub ldmSequences: *mut rawSeq, /* Storage for the ldm output sequences */
    pub maxNbLdmSequences: size_t,
    pub externSeqStore: RawSeqStore_t, /* Mutable reference to external sequences */
    pub blockState: ZSTD_blockState_t,
    pub tmpWorkspace: *mut c_void, /* used as substitute of stack space - must be aligned for S64 type */
    pub tmpWkspSize: size_t,

    /* Whether we are streaming or not */
    pub bufferedPolicy: ZSTD_buffered_policy_e,

    /* streaming */
    pub inBuff: *mut c_char,
    pub inBuffSize: size_t,
    pub inToCompress: size_t,
    pub inBuffPos: size_t,
    pub inBuffTarget: size_t,
    pub outBuff: *mut c_char,
    pub outBuffSize: size_t,
    pub outBuffContentSize: size_t,
    pub outBuffFlushedSize: size_t,
    pub streamStage: ZSTD_cStreamStage,
    pub frameEnded: U32,

    /* Stable in/out buffer verification */
    pub expectedInBuffer: ZSTD_inBuffer,
    pub stableIn_notConsumed: size_t, /* nb bytes within stable input buffer that are said to be consumed but are not */
    pub expectedOutBufferSize: size_t,

    /* Dictionary */
    pub localDict: ZSTD_localDict,
    pub cdict: *const ZSTD_CDict,
    pub prefixDict: ZSTD_prefixDict, /* single-usage dictionary */

    /* Multi-threading */
    /* (ZSTD_MULTITHREAD undefined: mtctx field omitted) */

    /* Tracing */
    /* ZSTD_TRACE == 1 on this platform (GCC/ELF has weak symbols), so the
     * traceCtx field IS compiled in. */
    pub traceCtx: ZSTD_traceCtx,

    /* Workspace for block splitter */
    pub blockSplitCtx: ZSTD_blockSplitCtx,

    /* Buffer for output from external sequence producer */
    pub extSeqBuf: *mut ZSTD_Sequence,
    pub extSeqBufCapacity: size_t,
}

pub type ZSTD_dictTableLoadMethod_e = c_uint;
pub const ZSTD_dtlm_fast: ZSTD_dictTableLoadMethod_e = 0;
pub const ZSTD_dtlm_full: ZSTD_dictTableLoadMethod_e = 1;

pub type ZSTD_tableFillPurpose_e = c_uint;
pub const ZSTD_tfp_forCCtx: ZSTD_tableFillPurpose_e = 0;
pub const ZSTD_tfp_forCDict: ZSTD_tableFillPurpose_e = 1;

pub type ZSTD_dictMode_e = c_uint;
pub const ZSTD_noDict: ZSTD_dictMode_e = 0;
pub const ZSTD_extDict: ZSTD_dictMode_e = 1;
pub const ZSTD_dictMatchState: ZSTD_dictMode_e = 2;
pub const ZSTD_dedicatedDictSearch: ZSTD_dictMode_e = 3;

pub type ZSTD_CParamMode_e = c_uint;
pub const ZSTD_cpm_noAttachDict: ZSTD_CParamMode_e = 0;
pub const ZSTD_cpm_attachDict: ZSTD_CParamMode_e = 1;
pub const ZSTD_cpm_createCDict: ZSTD_CParamMode_e = 2;
pub const ZSTD_cpm_unknown: ZSTD_CParamMode_e = 3;

pub type ZSTD_BlockCompressor_f = Option<
    unsafe extern "C" fn(
        bs: *mut ZSTD_MatchState_t,
        seqStore: *mut SeqStore_t,
        rep: *mut U32, /* U32 rep[ZSTD_REP_NUM] */
        src: *const c_void,
        srcSize: size_t,
    ) -> size_t,
>;

extern "C" {
    pub fn ZSTD_selectBlockCompressor(
        strat: ZSTD_strategy,
        rowMatchfinderMode: ZSTD_ParamSwitch_e,
        dictMode: ZSTD_dictMode_e,
    ) -> ZSTD_BlockCompressor_f;
}

/* opaque struct definitions (real fields live in the corresponding .c/.rs) */
pub type ZSTD_CCtx = ZSTD_CCtx_s;
pub type ZSTD_CDict = ZSTD_CDict_s;
pub type ZSTD_CCtx_params = ZSTD_CCtx_params_s;
pub type ZSTD_CStream = ZSTD_CCtx;

/* ZSTD_CDict_s is defined in zstd_compress.c; only reached through a pointer
 * from this header, so an opaque type is sufficient here. */
pub enum ZSTD_CDict_s {}

pub unsafe fn ZSTD_LLcode(litLength: U32) -> U32 {
    static LL_Code: [BYTE; 64] = [
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

/* ZSTD_MLcode() :
 * note : mlBase = matchLength - MINMATCH; */
pub unsafe fn ZSTD_MLcode(mlBase: U32) -> U32 {
    static ML_Code: [BYTE; 128] = [
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

extern "C" {
    pub fn ZSTD_cParam_getBounds(cParam: ZSTD_cParameter) -> ZSTD_bounds;
}

/* ZSTD_cParam_withinBounds:
 * @return 1 if value is within cParam bounds, 0 otherwise */
pub unsafe fn ZSTD_cParam_withinBounds(cParam: ZSTD_cParameter, value: c_int) -> c_int {
    let bounds: ZSTD_bounds = ZSTD_cParam_getBounds(cParam);
    if ZSTD_isError(bounds.error) != 0 {
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

/* ZSTD_selectAddr:
 * @return index >= lowLimit ? candidate : backup */
pub unsafe fn ZSTD_selectAddr(
    index: U32,
    lowLimit: U32,
    candidate: *const BYTE,
    backup: *const BYTE,
) -> *const BYTE {
    /* inline-asm variant is x86_64 only; take the portable body */
    if index >= lowLimit {
        candidate
    } else {
        backup
    }
}

/* ZSTD_noCompressBlock() :
 * Writes uncompressed block to dst buffer from given src.
 * Returns the size of the block */
pub unsafe fn ZSTD_noCompressBlock(
    dst: *mut c_void,
    dstCapacity: size_t,
    src: *const c_void,
    srcSize: size_t,
    lastBlock: U32,
) -> size_t {
    let cBlockHeader24: U32 =
        lastBlock + (((bt_raw as U32) << 1)) + ((srcSize << 3) as U32);
    if srcSize + ZSTD_blockHeaderSize > dstCapacity {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    MEM_writeLE24(dst as *mut u8, cBlockHeader24);
    ZSTD_memcpy(
        (dst as *mut u8).wrapping_add(ZSTD_blockHeaderSize),
        src as *const u8,
        srcSize,
    );
    ZSTD_blockHeaderSize + srcSize
}

pub unsafe fn ZSTD_rleCompressBlock(
    dst: *mut c_void,
    dstCapacity: size_t,
    src: BYTE,
    srcSize: size_t,
    lastBlock: U32,
) -> size_t {
    let op: *mut BYTE = dst as *mut BYTE;
    let cBlockHeader: U32 =
        lastBlock + (((bt_rle as U32) << 1)) + ((srcSize << 3) as U32);
    if dstCapacity < 4 {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    MEM_writeLE24(op, cBlockHeader);
    *op.wrapping_add(3) = src;
    4
}

/* ZSTD_minGain() :
 * minimum compression required
 * to generate a compress block or a compressed literals section. */
pub unsafe fn ZSTD_minGain(srcSize: size_t, strat: ZSTD_strategy) -> size_t {
    let minlog: U32 = if strat >= ZSTD_btultra {
        (strat as U32) - 1
    } else {
        6
    };
    (srcSize >> minlog) + 2
}

pub unsafe fn ZSTD_literalsCompressionIsDisabled(cctxParams: *const ZSTD_CCtx_params) -> c_int {
    match (*cctxParams).literalCompressionMode {
        ZSTD_ps_enable => 0,
        ZSTD_ps_disable => 1,
        _ => {
            /* default / ZSTD_ps_auto (fallthrough from default) */
            (((*cctxParams).cParams.strategy == ZSTD_fast)
                && ((*cctxParams).cParams.targetLength > 0)) as c_int
        }
    }
}

/* ZSTD_safecopyLiterals() :
 *  memcpy() function that won't read beyond more than WILDCOPY_OVERLENGTH bytes past ilimit_w. */
pub unsafe fn ZSTD_safecopyLiterals(
    mut op: *mut BYTE,
    mut ip: *const BYTE,
    iend: *const BYTE,
    ilimit_w: *const BYTE,
) {
    if ip <= ilimit_w {
        ZSTD_wildcopy(
            op,
            ip,
            ilimit_w.offset_from(ip) as isize,
            ZSTD_no_overlap,
        );
        op = op.wrapping_offset(ilimit_w.offset_from(ip));
        ip = ilimit_w;
    }
    while ip < iend {
        *op = *ip;
        op = op.wrapping_add(1);
        ip = ip.wrapping_add(1);
    }
}

/* offBase sum-type helpers (macros in C) */
#[inline(always)]
pub fn REPCODE_TO_OFFBASE(r: U32) -> U32 {
    r /* accepts IDs 1,2,3 */
}
#[inline(always)]
pub fn OFFSET_TO_OFFBASE(o: U32) -> U32 {
    o + (ZSTD_REP_NUM as U32)
}
#[inline(always)]
pub fn OFFBASE_IS_OFFSET(o: U32) -> bool {
    o > (ZSTD_REP_NUM as U32)
}
#[inline(always)]
pub fn OFFBASE_IS_REPCODE(o: U32) -> bool {
    1 <= o && o <= (ZSTD_REP_NUM as U32)
}
#[inline(always)]
pub fn OFFBASE_TO_OFFSET(o: U32) -> U32 {
    o - (ZSTD_REP_NUM as U32)
}
#[inline(always)]
pub fn OFFBASE_TO_REPCODE(o: U32) -> U32 {
    o /* returns ID 1,2,3 */
}
pub const REPCODE1_TO_OFFBASE: U32 = 1;
pub const REPCODE2_TO_OFFBASE: U32 = 2;
pub const REPCODE3_TO_OFFBASE: U32 = 3;

/* ZSTD_storeSeqOnly() :
 *  Store a sequence (litlen, litPtr, offBase and matchLength) into SeqStore_t. */
pub unsafe fn ZSTD_storeSeqOnly(
    seqStorePtr: *mut SeqStore_t,
    litLength: size_t,
    offBase: U32,
    matchLength: size_t,
) {
    /* literal Length */
    if litLength > 0xFFFF {
        (*seqStorePtr).longLengthType = ZSTD_llt_literalLength;
        (*seqStorePtr).longLengthPos =
            (*seqStorePtr).sequences.offset_from((*seqStorePtr).sequencesStart) as U32;
    }
    (*(*seqStorePtr).sequences.wrapping_add(0)).litLength = litLength as U16;

    /* match offset */
    (*(*seqStorePtr).sequences.wrapping_add(0)).offBase = offBase;

    /* match Length */
    {
        let mlBase: size_t = matchLength - (MINMATCH as size_t);
        if mlBase > 0xFFFF {
            (*seqStorePtr).longLengthType = ZSTD_llt_matchLength;
            (*seqStorePtr).longLengthPos =
                (*seqStorePtr).sequences.offset_from((*seqStorePtr).sequencesStart) as U32;
        }
        (*(*seqStorePtr).sequences.wrapping_add(0)).mlBase = mlBase as U16;
    }

    (*seqStorePtr).sequences = (*seqStorePtr).sequences.wrapping_add(1);
}

/* ZSTD_storeSeq() :
 *  Store a sequence (litlen, litPtr, offBase and matchLength) into SeqStore_t. */
pub unsafe fn ZSTD_storeSeq(
    seqStorePtr: *mut SeqStore_t,
    litLength: size_t,
    literals: *const BYTE,
    litLimit: *const BYTE,
    offBase: U32,
    matchLength: size_t,
) {
    let litLimit_w: *const BYTE = litLimit.wrapping_offset(-(WILDCOPY_OVERLENGTH));
    let litEnd: *const BYTE = literals.wrapping_add(litLength);
    /* copy Literals */
    if litEnd <= litLimit_w {
        /* Common case we can use wildcopy.
         * First copy 16 bytes, because literals are likely short. */
        ZSTD_copy16((*seqStorePtr).lit, literals);
        if litLength > 16 {
            ZSTD_wildcopy(
                (*seqStorePtr).lit.wrapping_add(16),
                literals.wrapping_add(16),
                (litLength as isize) - 16,
                ZSTD_no_overlap,
            );
        }
    } else {
        ZSTD_safecopyLiterals((*seqStorePtr).lit, literals, litEnd, litLimit_w);
    }
    (*seqStorePtr).lit = (*seqStorePtr).lit.wrapping_add(litLength);

    ZSTD_storeSeqOnly(seqStorePtr, litLength, offBase, matchLength);
}

/* ZSTD_updateRep() :
 * updates in-place @rep (array of repeat offsets) */
pub unsafe fn ZSTD_updateRep(rep: *mut U32, offBase: U32, ll0: U32) {
    if OFFBASE_IS_OFFSET(offBase) {
        /* full offset */
        *rep.wrapping_add(2) = *rep.wrapping_add(1);
        *rep.wrapping_add(1) = *rep.wrapping_add(0);
        *rep.wrapping_add(0) = OFFBASE_TO_OFFSET(offBase);
    } else {
        /* repcode */
        let repCode: U32 = OFFBASE_TO_REPCODE(offBase) - 1 + ll0;
        if repCode > 0 {
            /* note : if repCode==0, no change */
            let currentOffset: U32 = if repCode == (ZSTD_REP_NUM as U32) {
                *rep.wrapping_add(0) - 1
            } else {
                *rep.wrapping_add(repCode as usize)
            };
            *rep.wrapping_add(2) = if repCode >= 2 {
                *rep.wrapping_add(1)
            } else {
                *rep.wrapping_add(2)
            };
            *rep.wrapping_add(1) = *rep.wrapping_add(0);
            *rep.wrapping_add(0) = currentOffset;
        } else {
            /* repCode == 0 : nothing to do */
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct repcodes_s {
    pub rep: [U32; 3],
}
pub type Repcodes_t = repcodes_s;

pub unsafe fn ZSTD_newRep(rep: *const U32, offBase: U32, ll0: U32) -> Repcodes_t {
    let mut newReps = Repcodes_t { rep: [0; 3] };
    ZSTD_memcpy(
        &mut newReps as *mut Repcodes_t as *mut u8,
        rep as *const u8,
        core::mem::size_of::<Repcodes_t>() as size_t,
    );
    ZSTD_updateRep(newReps.rep.as_mut_ptr(), offBase, ll0);
    newReps
}

/*-*************************************
*  Match length counter
***************************************/
pub unsafe fn ZSTD_count(pIn: *const BYTE, pMatch: *const BYTE, pInLimit: *const BYTE) -> size_t {
    let pStart: *const BYTE = pIn;
    let mut pIn = pIn;
    let mut pMatch = pMatch;
    let pInLoopLimit: *const BYTE =
        pInLimit.wrapping_sub(core::mem::size_of::<size_t>() - 1);

    if pIn < pInLoopLimit {
        {
            let diff: size_t = MEM_readST(pMatch) ^ MEM_readST(pIn);
            if diff != 0 {
                return ZSTD_NbCommonBytes(diff) as size_t;
            }
        }
        pIn = pIn.wrapping_add(core::mem::size_of::<size_t>());
        pMatch = pMatch.wrapping_add(core::mem::size_of::<size_t>());
        while pIn < pInLoopLimit {
            let diff: size_t = MEM_readST(pMatch) ^ MEM_readST(pIn);
            if diff == 0 {
                pIn = pIn.wrapping_add(core::mem::size_of::<size_t>());
                pMatch = pMatch.wrapping_add(core::mem::size_of::<size_t>());
                continue;
            }
            pIn = pIn.wrapping_add(ZSTD_NbCommonBytes(diff) as usize);
            return pIn.offset_from(pStart) as size_t;
        }
    }
    if MEM_64bits() != 0
        && (pIn < pInLimit.wrapping_sub(3))
        && (MEM_read32(pMatch) == MEM_read32(pIn))
    {
        pIn = pIn.wrapping_add(4);
        pMatch = pMatch.wrapping_add(4);
    }
    if (pIn < pInLimit.wrapping_sub(1)) && (MEM_read16(pMatch) == MEM_read16(pIn)) {
        pIn = pIn.wrapping_add(2);
        pMatch = pMatch.wrapping_add(2);
    }
    if (pIn < pInLimit) && (*pMatch == *pIn) {
        pIn = pIn.wrapping_add(1);
    }
    pIn.offset_from(pStart) as size_t
}

/** ZSTD_count_2segments() :
 *  can count match length with `ip` & `match` in 2 different segments. */
pub unsafe fn ZSTD_count_2segments(
    ip: *const BYTE,
    r#match: *const BYTE,
    iEnd: *const BYTE,
    mEnd: *const BYTE,
    iStart: *const BYTE,
) -> size_t {
    let vEnd: *const BYTE = MIN(
        ip.wrapping_offset(mEnd.offset_from(r#match)),
        iEnd,
    );
    let matchLength: size_t = ZSTD_count(ip, r#match, vEnd);
    if r#match.wrapping_add(matchLength) != mEnd {
        return matchLength;
    }
    matchLength + ZSTD_count(ip.wrapping_add(matchLength), iStart, iEnd)
}

/*-*************************************
 *  Hashes
 ***************************************/
pub static prime3bytes: U32 = 506832829;
pub unsafe fn ZSTD_hash3(u: U32, h: U32, s: U32) -> U32 {
    ((((u << (32 - 24)).wrapping_mul(prime3bytes)) ^ s) >> (32 - h))
}
pub unsafe fn ZSTD_hash3Ptr(ptr: *const c_void, h: U32) -> size_t {
    ZSTD_hash3(MEM_readLE32(ptr as *const u8), h, 0) as size_t
}
pub unsafe fn ZSTD_hash3PtrS(ptr: *const c_void, h: U32, s: U32) -> size_t {
    ZSTD_hash3(MEM_readLE32(ptr as *const u8), h, s) as size_t
}

pub static prime4bytes: U32 = 2654435761;
pub unsafe fn ZSTD_hash4(u: U32, h: U32, s: U32) -> U32 {
    ((u.wrapping_mul(prime4bytes)) ^ s) >> (32 - h)
}
pub unsafe fn ZSTD_hash4Ptr(ptr: *const c_void, h: U32) -> size_t {
    ZSTD_hash4(MEM_readLE32(ptr as *const u8), h, 0) as size_t
}
pub unsafe fn ZSTD_hash4PtrS(ptr: *const c_void, h: U32, s: U32) -> size_t {
    ZSTD_hash4(MEM_readLE32(ptr as *const u8), h, s) as size_t
}

pub static prime5bytes: U64 = 889523592379;
pub unsafe fn ZSTD_hash5(u: U64, h: U32, s: U64) -> size_t {
    (((((u << (64 - 40)).wrapping_mul(prime5bytes)) ^ s) >> (64 - h)) as size_t)
}
pub unsafe fn ZSTD_hash5Ptr(p: *const c_void, h: U32) -> size_t {
    ZSTD_hash5(MEM_readLE64(p as *const u8), h, 0)
}
pub unsafe fn ZSTD_hash5PtrS(p: *const c_void, h: U32, s: U64) -> size_t {
    ZSTD_hash5(MEM_readLE64(p as *const u8), h, s)
}

pub static prime6bytes: U64 = 227718039650203;
pub unsafe fn ZSTD_hash6(u: U64, h: U32, s: U64) -> size_t {
    (((((u << (64 - 48)).wrapping_mul(prime6bytes)) ^ s) >> (64 - h)) as size_t)
}
pub unsafe fn ZSTD_hash6Ptr(p: *const c_void, h: U32) -> size_t {
    ZSTD_hash6(MEM_readLE64(p as *const u8), h, 0)
}
pub unsafe fn ZSTD_hash6PtrS(p: *const c_void, h: U32, s: U64) -> size_t {
    ZSTD_hash6(MEM_readLE64(p as *const u8), h, s)
}

pub static prime7bytes: U64 = 58295818150454627;
pub unsafe fn ZSTD_hash7(u: U64, h: U32, s: U64) -> size_t {
    (((((u << (64 - 56)).wrapping_mul(prime7bytes)) ^ s) >> (64 - h)) as size_t)
}
pub unsafe fn ZSTD_hash7Ptr(p: *const c_void, h: U32) -> size_t {
    ZSTD_hash7(MEM_readLE64(p as *const u8), h, 0)
}
pub unsafe fn ZSTD_hash7PtrS(p: *const c_void, h: U32, s: U64) -> size_t {
    ZSTD_hash7(MEM_readLE64(p as *const u8), h, s)
}

pub static prime8bytes: U64 = 0xCF1BBCDCB7A56463;
pub unsafe fn ZSTD_hash8(u: U64, h: U32, s: U64) -> size_t {
    ((((u.wrapping_mul(prime8bytes)) ^ s) >> (64 - h)) as size_t)
}
pub unsafe fn ZSTD_hash8Ptr(p: *const c_void, h: U32) -> size_t {
    ZSTD_hash8(MEM_readLE64(p as *const u8), h, 0)
}
pub unsafe fn ZSTD_hash8PtrS(p: *const c_void, h: U32, s: U64) -> size_t {
    ZSTD_hash8(MEM_readLE64(p as *const u8), h, s)
}

pub unsafe fn ZSTD_hashPtr(p: *const c_void, hBits: U32, mls: U32) -> size_t {
    match mls {
        5 => ZSTD_hash5Ptr(p, hBits),
        6 => ZSTD_hash6Ptr(p, hBits),
        7 => ZSTD_hash7Ptr(p, hBits),
        8 => ZSTD_hash8Ptr(p, hBits),
        /* default and case 4 */
        _ => ZSTD_hash4Ptr(p, hBits),
    }
}

pub unsafe fn ZSTD_hashPtrSalted(p: *const c_void, hBits: U32, mls: U32, hashSalt: U64) -> size_t {
    match mls {
        5 => ZSTD_hash5PtrS(p, hBits, hashSalt),
        6 => ZSTD_hash6PtrS(p, hBits, hashSalt),
        7 => ZSTD_hash7PtrS(p, hBits, hashSalt),
        8 => ZSTD_hash8PtrS(p, hBits, hashSalt),
        /* default and case 4 */
        _ => ZSTD_hash4PtrS(p, hBits, hashSalt as U32),
    }
}

/** ZSTD_ipow() :
 * Return base^exponent. */
pub unsafe fn ZSTD_ipow(base: U64, exponent: U64) -> U64 {
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

/** ZSTD_rollingHash_append() : */
pub unsafe fn ZSTD_rollingHash_append(hash: U64, buf: *const c_void, size: size_t) -> U64 {
    let istart: *const BYTE = buf as *const BYTE;
    let mut hash = hash;
    let mut pos: size_t = 0;
    while pos < size {
        hash = hash.wrapping_mul(prime8bytes);
        hash = hash.wrapping_add((*istart.wrapping_add(pos)) as U64 + ZSTD_ROLL_HASH_CHAR_OFFSET);
        pos += 1;
    }
    hash
}

/** ZSTD_rollingHash_compute() : */
pub unsafe fn ZSTD_rollingHash_compute(buf: *const c_void, size: size_t) -> U64 {
    ZSTD_rollingHash_append(0, buf, size)
}

/** ZSTD_rollingHash_primePower() : */
pub unsafe fn ZSTD_rollingHash_primePower(length: U32) -> U64 {
    ZSTD_ipow(prime8bytes, (length - 1) as U64)
}

/** ZSTD_rollingHash_rotate() : */
pub unsafe fn ZSTD_rollingHash_rotate(hash: U64, toRemove: BYTE, toAdd: BYTE, primePower: U64) -> U64 {
    let mut hash = hash;
    hash = hash.wrapping_sub(
        ((toRemove as U64) + ZSTD_ROLL_HASH_CHAR_OFFSET).wrapping_mul(primePower),
    );
    hash = hash.wrapping_mul(prime8bytes);
    hash = hash.wrapping_add((toAdd as U64) + ZSTD_ROLL_HASH_CHAR_OFFSET);
    hash
}

/*-*************************************
*  Round buffer management
***************************************/
/* Max @current value allowed. */
pub const ZSTD_CURRENT_MAX: U32 = if MEM_64bits_const() {
    3500u32 * (1 << 20)
} else {
    2000u32 * (1 << 20)
};
/* Maximum chunk size before overflow correction needs to be called again */
pub const ZSTD_CHUNKSIZE_MAX: U32 = (u32::MAX) - ZSTD_CURRENT_MAX;

/* helper for const-context 64-bit detection (mirrors MEM_64bits()) */
pub const fn MEM_64bits_const() -> bool {
    core::mem::size_of::<size_t>() == 8
}

/**
 * ZSTD_window_clear(): Clears the window containing the history. */
pub unsafe fn ZSTD_window_clear(window: *mut ZSTD_window_t) {
    let endT: size_t = (*window).nextSrc.offset_from((*window).base) as size_t;
    let end: U32 = endT as U32;

    (*window).lowLimit = end;
    (*window).dictLimit = end;
}

pub unsafe fn ZSTD_window_isEmpty(window: ZSTD_window_t) -> U32 {
    ((window.dictLimit == ZSTD_WINDOW_START_INDEX)
        && (window.lowLimit == ZSTD_WINDOW_START_INDEX)
        && ((window.nextSrc.offset_from(window.base) as U32) == ZSTD_WINDOW_START_INDEX)) as U32
}

/**
 * ZSTD_window_hasExtDict(): Returns non-zero if the window has a non-empty extDict. */
pub unsafe fn ZSTD_window_hasExtDict(window: ZSTD_window_t) -> U32 {
    (window.lowLimit < window.dictLimit) as U32
}

/**
 * ZSTD_matchState_dictMode(): */
pub unsafe fn ZSTD_matchState_dictMode(ms: *const ZSTD_MatchState_t) -> ZSTD_dictMode_e {
    if ZSTD_window_hasExtDict((*ms).window) != 0 {
        ZSTD_extDict
    } else if (*ms).dictMatchState != null_mut() {
        if (*(*ms).dictMatchState).dedicatedDictSearch != 0 {
            ZSTD_dedicatedDictSearch
        } else {
            ZSTD_dictMatchState
        }
    } else {
        ZSTD_noDict
    }
}

pub const ZSTD_WINDOW_OVERFLOW_CORRECT_FREQUENTLY: c_int = 0;

/**
 * ZSTD_window_canOverflowCorrect(): */
pub unsafe fn ZSTD_window_canOverflowCorrect(
    window: ZSTD_window_t,
    cycleLog: U32,
    maxDist: U32,
    loadedDictEnd: U32,
    src: *const c_void,
) -> U32 {
    let cycleSize: U32 = 1u32 << cycleLog;
    let curr: U32 = (src as *const BYTE).offset_from(window.base) as U32;
    let minIndexToOverflowCorrect: U32 =
        cycleSize + MAX(maxDist, cycleSize) + ZSTD_WINDOW_START_INDEX;

    let adjustment: U32 = window.nbOverflowCorrections + 1;
    let adjustedIndex: U32 = MAX(
        minIndexToOverflowCorrect.wrapping_mul(adjustment),
        minIndexToOverflowCorrect,
    );
    let indexLargeEnough: U32 = (curr > adjustedIndex) as U32;

    let dictionaryInvalidated: U32 = (curr > maxDist + loadedDictEnd) as U32;

    indexLargeEnough & dictionaryInvalidated
}

/**
 * ZSTD_window_needOverflowCorrection(): */
pub unsafe fn ZSTD_window_needOverflowCorrection(
    window: ZSTD_window_t,
    cycleLog: U32,
    maxDist: U32,
    loadedDictEnd: U32,
    src: *const c_void,
    srcEnd: *const c_void,
) -> U32 {
    let curr: U32 = (srcEnd as *const BYTE).offset_from(window.base) as U32;
    if ZSTD_WINDOW_OVERFLOW_CORRECT_FREQUENTLY != 0 {
        if ZSTD_window_canOverflowCorrect(window, cycleLog, maxDist, loadedDictEnd, src) != 0 {
            return 1;
        }
    }
    (curr > ZSTD_CURRENT_MAX) as U32
}

/**
 * ZSTD_window_correctOverflow(): Reduces the indices to protect from index overflow. */
pub unsafe fn ZSTD_window_correctOverflow(
    window: *mut ZSTD_window_t,
    cycleLog: U32,
    maxDist: U32,
    src: *const c_void,
) -> U32 {
    let cycleSize: U32 = 1u32 << cycleLog;
    let cycleMask: U32 = cycleSize - 1;
    let curr: U32 = (src as *const BYTE).offset_from((*window).base) as U32;
    let currentCycle: U32 = curr & cycleMask;
    /* Ensure newCurrent - maxDist >= ZSTD_WINDOW_START_INDEX. */
    let currentCycleCorrection: U32 = if currentCycle < ZSTD_WINDOW_START_INDEX {
        MAX(cycleSize, ZSTD_WINDOW_START_INDEX)
    } else {
        0
    };
    let newCurrent: U32 = currentCycle + currentCycleCorrection + MAX(maxDist, cycleSize);
    let correction: U32 = curr - newCurrent;

    (*window).base = (*window).base.wrapping_offset(correction as isize);
    (*window).dictBase = (*window).dictBase.wrapping_offset(correction as isize);
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

    (*window).nbOverflowCorrections += 1;

    correction
}

/**
 * ZSTD_window_enforceMaxDist(): */
pub unsafe fn ZSTD_window_enforceMaxDist(
    window: *mut ZSTD_window_t,
    blockEnd: *const c_void,
    maxDist: U32,
    loadedDictEndPtr: *mut U32,
    dictMatchStatePtr: *mut *const ZSTD_MatchState_t,
) {
    let blockEndIdx: U32 = (blockEnd as *const BYTE).offset_from((*window).base) as U32;
    let loadedDictEnd: U32 = if loadedDictEndPtr != null_mut() {
        *loadedDictEndPtr
    } else {
        0
    };

    if blockEndIdx > maxDist + loadedDictEnd {
        let newLowLimit: U32 = blockEndIdx - maxDist;
        if (*window).lowLimit < newLowLimit {
            (*window).lowLimit = newLowLimit;
        }
        if (*window).dictLimit < (*window).lowLimit {
            (*window).dictLimit = (*window).lowLimit;
        }
        /* On reaching window size, dictionaries are invalidated */
        if loadedDictEndPtr != null_mut() {
            *loadedDictEndPtr = 0;
        }
        if dictMatchStatePtr != null_mut() {
            *dictMatchStatePtr = null_mut();
        }
    }
}

/* Similar to ZSTD_window_enforceMaxDist(), but only invalidates dictionary
 * when input progresses beyond window size. */
pub unsafe fn ZSTD_checkDictValidity(
    window: *const ZSTD_window_t,
    blockEnd: *const c_void,
    maxDist: U32,
    loadedDictEndPtr: *mut U32,
    dictMatchStatePtr: *mut *const ZSTD_MatchState_t,
) {
    {
        let blockEndIdx: U32 = (blockEnd as *const BYTE).offset_from((*window).base) as U32;
        let loadedDictEnd: U32 = *loadedDictEndPtr;

        if blockEndIdx > loadedDictEnd + maxDist || loadedDictEnd != (*window).dictLimit {
            *loadedDictEndPtr = 0;
            *dictMatchStatePtr = null_mut();
        } else {
            if *loadedDictEndPtr != 0 {
                /* dictionary considered valid for current block */
            }
        }
    }
}

pub unsafe fn ZSTD_window_init(window: *mut ZSTD_window_t) {
    ZSTD_memset(
        window as *mut u8,
        0,
        core::mem::size_of::<ZSTD_window_t>() as size_t,
    );
    (*window).base = c" ".as_ptr() as *const BYTE;
    (*window).dictBase = c" ".as_ptr() as *const BYTE;
    (*window).dictLimit = ZSTD_WINDOW_START_INDEX; /* start from >0, so that 1st position is valid */
    (*window).lowLimit = ZSTD_WINDOW_START_INDEX; /* it ensures first and later CCtx usages compress the same */
    (*window).nextSrc = (*window).base.wrapping_offset(ZSTD_WINDOW_START_INDEX as isize); /* see issue #1241 */
    (*window).nbOverflowCorrections = 0;
}

/**
 * ZSTD_window_update(): Updates the window by appending [src, src + srcSize) to the window. */
pub unsafe fn ZSTD_window_update(
    window: *mut ZSTD_window_t,
    src: *const c_void,
    srcSize: size_t,
    forceNonContiguous: c_int,
) -> U32 {
    let ip: *const BYTE = src as *const BYTE;
    let mut contiguous: U32 = 1;
    if srcSize == 0 {
        return contiguous;
    }
    /* Check if blocks follow each other */
    if src != (*window).nextSrc as *const c_void || forceNonContiguous != 0 {
        /* not contiguous */
        let distanceFromBase: size_t = (*window).nextSrc.offset_from((*window).base) as size_t;
        (*window).lowLimit = (*window).dictLimit;
        (*window).dictLimit = distanceFromBase as U32;
        (*window).dictBase = (*window).base;
        (*window).base = ip.wrapping_sub(distanceFromBase);
        /* ms->nextToUpdate = window->dictLimit; */
        if (*window).dictLimit - (*window).lowLimit < (HASH_READ_SIZE as U32) {
            (*window).lowLimit = (*window).dictLimit;
        } /* too small extDict */
        contiguous = 0;
    }
    (*window).nextSrc = ip.wrapping_add(srcSize);
    /* if input and dictionary overlap : reduce dictionary (area presumed modified by input) */
    if (ip.wrapping_add(srcSize) > (*window).dictBase.wrapping_offset((*window).lowLimit as isize))
        && (ip < (*window).dictBase.wrapping_offset((*window).dictLimit as isize))
    {
        let highInputIdx: size_t =
            (ip.wrapping_add(srcSize)).offset_from((*window).dictBase) as size_t;
        let lowLimitMax: U32 = if highInputIdx > ((*window).dictLimit as size_t) {
            (*window).dictLimit
        } else {
            highInputIdx as U32
        };
        (*window).lowLimit = lowLimitMax;
    }
    contiguous
}

/**
 * Returns the lowest allowed match index. It may either be in the ext-dict or the prefix. */
pub unsafe fn ZSTD_getLowestMatchIndex(
    ms: *const ZSTD_MatchState_t,
    curr: U32,
    windowLog: c_uint,
) -> U32 {
    let maxDistance: U32 = 1u32 << windowLog;
    let lowestValid: U32 = (*ms).window.lowLimit;
    let withinWindow: U32 = if curr - lowestValid > maxDistance {
        curr - maxDistance
    } else {
        lowestValid
    };
    let isDictionary: U32 = ((*ms).loadedDictEnd != 0) as U32;
    let matchLowest: U32 = if isDictionary != 0 {
        lowestValid
    } else {
        withinWindow
    };
    matchLowest
}

/**
 * Returns the lowest allowed match index in the prefix. */
pub unsafe fn ZSTD_getLowestPrefixIndex(
    ms: *const ZSTD_MatchState_t,
    curr: U32,
    windowLog: c_uint,
) -> U32 {
    let maxDistance: U32 = 1u32 << windowLog;
    let lowestValid: U32 = (*ms).window.dictLimit;
    let withinWindow: U32 = if curr - lowestValid > maxDistance {
        curr - maxDistance
    } else {
        lowestValid
    };
    let isDictionary: U32 = ((*ms).loadedDictEnd != 0) as U32;
    let matchLowest: U32 = if isDictionary != 0 {
        lowestValid
    } else {
        withinWindow
    };
    matchLowest
}

/* index_safety_check: intentional underflow : ensure repIndex isn't overlapping dict + prefix */
pub unsafe fn ZSTD_index_overlap_check(prefixLowestIndex: U32, repIndex: U32) -> c_int {
    ((prefixLowestIndex.wrapping_sub(1)).wrapping_sub(repIndex) >= 3) as c_int
}

/* Short Cache */
pub const ZSTD_SHORT_CACHE_TAG_BITS: U32 = 8;
pub const ZSTD_SHORT_CACHE_TAG_MASK: U32 = (1u32 << ZSTD_SHORT_CACHE_TAG_BITS) - 1;

/* Helper function for ZSTD_fillHashTable and ZSTD_fillDoubleHashTable. */
pub unsafe fn ZSTD_writeTaggedIndex(hashTable: *mut U32, hashAndTag: size_t, index: U32) {
    let hash: size_t = hashAndTag >> ZSTD_SHORT_CACHE_TAG_BITS;
    let tag: U32 = (hashAndTag as U32) & ZSTD_SHORT_CACHE_TAG_MASK;
    *hashTable.wrapping_add(hash) = (index << ZSTD_SHORT_CACHE_TAG_BITS) | tag;
}

/* Helper function for short cache matchfinders. */
pub unsafe fn ZSTD_comparePackedTags(packedTag1: size_t, packedTag2: size_t) -> c_int {
    let tag1: U32 = (packedTag1 as U32) & ZSTD_SHORT_CACHE_TAG_MASK;
    let tag2: U32 = (packedTag2 as U32) & ZSTD_SHORT_CACHE_TAG_MASK;
    (tag1 == tag2) as c_int
}

/* ===============================================================
 * Shared internal declarations
 * =============================================================== */

extern "C" {
    pub fn ZSTD_loadCEntropy(
        bs: *mut ZSTD_compressedBlockState_t,
        workspace: *mut c_void,
        dict: *const c_void,
        dictSize: size_t,
    ) -> size_t;

    pub fn ZSTD_reset_compressedBlockState(bs: *mut ZSTD_compressedBlockState_t);
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ZSTD_SequencePosition {
    pub idx: U32,           /* Index in array of ZSTD_Sequence */
    pub posInSequence: U32, /* Position within sequence at idx */
    pub posInSrc: size_t,   /* Number of bytes given by sequences provided so far */
}

extern "C" {
    /* for benchmark */
    pub fn ZSTD_convertBlockSequences(
        cctx: *mut ZSTD_CCtx,
        inSeqs: *const ZSTD_Sequence,
        nbSequences: size_t,
        repcodeResolution: c_int,
    ) -> size_t;
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct BlockSummary {
    pub nbSequences: size_t,
    pub blockSize: size_t,
    pub litSize: size_t,
}

extern "C" {
    pub fn ZSTD_get1BlockSummary(seqs: *const ZSTD_Sequence, nbSeqs: size_t) -> BlockSummary;

    /* Private declarations */
    pub fn ZSTD_getCParamsFromCCtxParams(
        CCtxParams: *const ZSTD_CCtx_params,
        srcSizeHint: U64,
        dictSize: size_t,
        mode: ZSTD_CParamMode_e,
    ) -> ZSTD_compressionParameters;

    pub fn ZSTD_initCStream_internal(
        zcs: *mut ZSTD_CStream,
        dict: *const c_void,
        dictSize: size_t,
        cdict: *const ZSTD_CDict,
        params: *const ZSTD_CCtx_params,
        pledgedSrcSize: core::ffi::c_ulonglong,
    ) -> size_t;

    pub fn ZSTD_resetSeqStore(ssPtr: *mut SeqStore_t);

    pub fn ZSTD_getCParamsFromCDict(cdict: *const ZSTD_CDict) -> ZSTD_compressionParameters;

    pub fn ZSTD_compressBegin_advanced_internal(
        cctx: *mut ZSTD_CCtx,
        dict: *const c_void,
        dictSize: size_t,
        dictContentType: ZSTD_dictContentType_e,
        dtlm: ZSTD_dictTableLoadMethod_e,
        cdict: *const ZSTD_CDict,
        params: *const ZSTD_CCtx_params,
        pledgedSrcSize: core::ffi::c_ulonglong,
    ) -> size_t;

    pub fn ZSTD_compress_advanced_internal(
        cctx: *mut ZSTD_CCtx,
        dst: *mut c_void,
        dstCapacity: size_t,
        src: *const c_void,
        srcSize: size_t,
        dict: *const c_void,
        dictSize: size_t,
        params: *const ZSTD_CCtx_params,
    ) -> size_t;

    pub fn ZSTD_writeLastEmptyBlock(dst: *mut c_void, dstCapacity: size_t) -> size_t;

    pub fn ZSTD_referenceExternalSequences(
        cctx: *mut ZSTD_CCtx,
        seq: *mut rawSeq,
        nbSeq: size_t,
    );

    pub fn ZSTD_cycleLog(hashLog: U32, strat: ZSTD_strategy) -> U32;

    pub fn ZSTD_CCtx_trace(cctx: *mut ZSTD_CCtx, extraCSize: size_t);
}

/* Returns 1 if an external sequence producer is registered, otherwise returns 0. */
pub unsafe fn ZSTD_hasExtSeqProd(params: *const ZSTD_CCtx_params) -> c_int {
    ((*params).extSeqProdFunc.is_some()) as c_int
}

extern "C" {
    /* Deprecated definitions that are still used internally. */
    pub fn ZSTD_compressBegin_usingCDict_deprecated(
        cctx: *mut ZSTD_CCtx,
        cdict: *const ZSTD_CDict,
    ) -> size_t;

    pub fn ZSTD_compressContinue_public(
        cctx: *mut ZSTD_CCtx,
        dst: *mut c_void,
        dstCapacity: size_t,
        src: *const c_void,
        srcSize: size_t,
    ) -> size_t;

    pub fn ZSTD_compressEnd_public(
        cctx: *mut ZSTD_CCtx,
        dst: *mut c_void,
        dstCapacity: size_t,
        src: *const c_void,
        srcSize: size_t,
    ) -> size_t;

    pub fn ZSTD_compressBlock_deprecated(
        cctx: *mut ZSTD_CCtx,
        dst: *mut c_void,
        dstCapacity: size_t,
        src: *const c_void,
        srcSize: size_t,
    ) -> size_t;
}
