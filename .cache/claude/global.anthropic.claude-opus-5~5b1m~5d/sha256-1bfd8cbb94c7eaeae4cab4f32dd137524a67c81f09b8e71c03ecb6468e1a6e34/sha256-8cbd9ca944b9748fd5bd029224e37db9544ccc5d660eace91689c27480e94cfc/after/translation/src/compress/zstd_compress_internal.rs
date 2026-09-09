//! Translation of `compress/zstd_compress_internal.h`
#![allow(dead_code)]

use core::ffi::{c_int, c_uint, c_void};

use crate::bits::*;
use crate::bitstream::*;
use crate::cmem::*;
use crate::compress::zstd_cwksp::*;
use crate::error_private::*;
use crate::fse::*;
use crate::huf::*;
use crate::xxhash::XXH64_state_t;
use crate::zstd_h::*;
use crate::zstd_trace::ZSTD_TraceCtx;
use crate::zstd_internal::*;

pub const kSearchStrength: U32 = 8;
pub const HASH_READ_SIZE: usize = 8;
pub const ZSTD_DUBT_UNSORTED_MARK: U32 = 1;

pub const ZSTD_SLIPBLOCK_WORKSPACESIZE: usize = 8208;

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
#[derive(Copy, Clone)]
pub struct ZSTD_prefixDict {
    pub dict: *const c_void,
    pub dictSize: usize,
    pub dictContentType: ZSTD_dictContentType_e,
}

impl Default for ZSTD_prefixDict {
    fn default() -> Self {
        ZSTD_prefixDict {
            dict: core::ptr::null(),
            dictSize: 0,
            dictContentType: 0,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct ZSTD_localDict {
    pub dictBuffer: *mut c_void,
    pub dict: *const c_void,
    pub dictSize: usize,
    pub dictContentType: ZSTD_dictContentType_e,
    pub cdict: *mut ZSTD_CDict,
}

impl Default for ZSTD_localDict {
    fn default() -> Self {
        ZSTD_localDict {
            dictBuffer: core::ptr::null_mut(),
            dict: core::ptr::null(),
            dictSize: 0,
            dictContentType: 0,
            cdict: core::ptr::null_mut(),
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct ZSTD_hufCTables_t {
    pub CTable: [HUF_CElt; HUF_CTABLE_SIZE_ST(255)],
    pub repeatMode: HUF_repeat,
}

impl Default for ZSTD_hufCTables_t {
    fn default() -> Self {
        ZSTD_hufCTables_t {
            CTable: [0; HUF_CTABLE_SIZE_ST(255)],
            repeatMode: 0,
        }
    }
}

pub const OFFCODE_CTABLE_SIZE_U32: usize = FSE_CTABLE_SIZE_U32(OffFSELog, MaxOff);
pub const MATCHLENGTH_CTABLE_SIZE_U32: usize = FSE_CTABLE_SIZE_U32(MLFSELog, MaxML);
pub const LITLENGTH_CTABLE_SIZE_U32: usize = FSE_CTABLE_SIZE_U32(LLFSELog, MaxLL);

#[repr(C)]
#[derive(Copy, Clone)]
pub struct ZSTD_fseCTables_t {
    pub offcodeCTable: [FSE_CTable; OFFCODE_CTABLE_SIZE_U32],
    pub matchlengthCTable: [FSE_CTable; MATCHLENGTH_CTABLE_SIZE_U32],
    pub litlengthCTable: [FSE_CTable; LITLENGTH_CTABLE_SIZE_U32],
    pub offcode_repeatMode: FSE_repeat,
    pub matchlength_repeatMode: FSE_repeat,
    pub litlength_repeatMode: FSE_repeat,
}

impl Default for ZSTD_fseCTables_t {
    fn default() -> Self {
        ZSTD_fseCTables_t {
            offcodeCTable: [0; OFFCODE_CTABLE_SIZE_U32],
            matchlengthCTable: [0; MATCHLENGTH_CTABLE_SIZE_U32],
            litlengthCTable: [0; LITLENGTH_CTABLE_SIZE_U32],
            offcode_repeatMode: 0,
            matchlength_repeatMode: 0,
            litlength_repeatMode: 0,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct ZSTD_entropyCTables_t {
    pub huf: ZSTD_hufCTables_t,
    pub fse: ZSTD_fseCTables_t,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct SeqDef {
    pub offBase: U32,
    pub litLength: U16,
    pub mlBase: U16,
}

pub type ZSTD_longLengthType_e = c_uint;
pub const ZSTD_llt_none: ZSTD_longLengthType_e = 0;
pub const ZSTD_llt_literalLength: ZSTD_longLengthType_e = 1;
pub const ZSTD_llt_matchLength: ZSTD_longLengthType_e = 2;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct SeqStore_t {
    pub sequencesStart: *mut SeqDef,
    pub sequences: *mut SeqDef,
    pub litStart: *mut BYTE,
    pub lit: *mut BYTE,
    pub llCode: *mut BYTE,
    pub mlCode: *mut BYTE,
    pub ofCode: *mut BYTE,
    pub maxNbSeq: usize,
    pub maxNbLit: usize,
    pub longLengthType: ZSTD_longLengthType_e,
    pub longLengthPos: U32,
}

impl Default for SeqStore_t {
    fn default() -> Self {
        SeqStore_t {
            sequencesStart: core::ptr::null_mut(),
            sequences: core::ptr::null_mut(),
            litStart: core::ptr::null_mut(),
            lit: core::ptr::null_mut(),
            llCode: core::ptr::null_mut(),
            mlCode: core::ptr::null_mut(),
            ofCode: core::ptr::null_mut(),
            maxNbSeq: 0,
            maxNbLit: 0,
            longLengthType: 0,
            longLengthPos: 0,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct ZSTD_SequenceLength {
    pub litLength: U32,
    pub matchLength: U32,
}

#[inline(always)]
pub unsafe fn ZSTD_getSequenceLength(
    seqStore: *const SeqStore_t,
    seq: *const SeqDef,
) -> ZSTD_SequenceLength {
    let mut seqLen = ZSTD_SequenceLength::default();
    seqLen.litLength = (*seq).litLength as U32;
    seqLen.matchLength = (*seq).mlBase as U32 + MINMATCH as U32;
    if (*seqStore).longLengthPos
        == (seq.offset_from((*seqStore).sequencesStart as *const SeqDef)) as U32
    {
        if (*seqStore).longLengthType == ZSTD_llt_literalLength {
            seqLen.litLength += 0x10000;
        }
        if (*seqStore).longLengthType == ZSTD_llt_matchLength {
            seqLen.matchLength += 0x10000;
        }
    }
    seqLen
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct ZSTD_hufCTablesMetadata_t {
    pub hType: SymbolEncodingType_e,
    pub hufDesBuffer: [BYTE; ZSTD_MAX_HUF_HEADER_SIZE],
    pub hufDesSize: usize,
}

impl Default for ZSTD_hufCTablesMetadata_t {
    fn default() -> Self {
        ZSTD_hufCTablesMetadata_t {
            hType: 0,
            hufDesBuffer: [0; ZSTD_MAX_HUF_HEADER_SIZE],
            hufDesSize: 0,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct ZSTD_fseCTablesMetadata_t {
    pub llType: SymbolEncodingType_e,
    pub ofType: SymbolEncodingType_e,
    pub mlType: SymbolEncodingType_e,
    pub fseTablesBuffer: [BYTE; ZSTD_MAX_FSE_HEADERS_SIZE],
    pub fseTablesSize: usize,
    pub lastCountSize: usize,
}

impl Default for ZSTD_fseCTablesMetadata_t {
    fn default() -> Self {
        ZSTD_fseCTablesMetadata_t {
            llType: 0,
            ofType: 0,
            mlType: 0,
            fseTablesBuffer: [0; ZSTD_MAX_FSE_HEADERS_SIZE],
            fseTablesSize: 0,
            lastCountSize: 0,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct ZSTD_entropyCTablesMetadata_t {
    pub hufMetadata: ZSTD_hufCTablesMetadata_t,
    pub fseMetadata: ZSTD_fseCTablesMetadata_t,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct ZSTD_match_t {
    pub off: U32,
    pub len: U32,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct rawSeq {
    pub offset: U32,
    pub litLength: U32,
    pub matchLength: U32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct RawSeqStore_t {
    pub seq: *mut rawSeq,
    pub pos: usize,
    pub posInSequence: usize,
    pub size: usize,
    pub capacity: usize,
}

impl Default for RawSeqStore_t {
    fn default() -> Self {
        RawSeqStore_t {
            seq: core::ptr::null_mut(),
            pos: 0,
            posInSequence: 0,
            size: 0,
            capacity: 0,
        }
    }
}

pub const kNullRawSeqStore: RawSeqStore_t = RawSeqStore_t {
    seq: core::ptr::null_mut(),
    pos: 0,
    posInSequence: 0,
    size: 0,
    capacity: 0,
};

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct ZSTD_optimal_t {
    pub price: c_int,
    pub off: U32,
    pub mlen: U32,
    pub litlen: U32,
    pub rep: [U32; ZSTD_REP_NUM],
}

pub type ZSTD_OptPrice_e = c_uint;
pub const zop_dynamic: ZSTD_OptPrice_e = 0;
pub const zop_predef: ZSTD_OptPrice_e = 1;

pub const ZSTD_OPT_SIZE: usize = ZSTD_OPT_NUM + 3;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct optState_t {
    pub litFreq: *mut c_uint,
    pub litLengthFreq: *mut c_uint,
    pub matchLengthFreq: *mut c_uint,
    pub offCodeFreq: *mut c_uint,
    pub matchTable: *mut ZSTD_match_t,
    pub priceTable: *mut ZSTD_optimal_t,
    pub litSum: U32,
    pub litLengthSum: U32,
    pub matchLengthSum: U32,
    pub offCodeSum: U32,
    pub litSumBasePrice: U32,
    pub litLengthSumBasePrice: U32,
    pub matchLengthSumBasePrice: U32,
    pub offCodeSumBasePrice: U32,
    pub priceType: ZSTD_OptPrice_e,
    pub symbolCosts: *const ZSTD_entropyCTables_t,
    pub literalCompressionMode: ZSTD_ParamSwitch_e,
}

impl Default for optState_t {
    fn default() -> Self {
        optState_t {
            litFreq: core::ptr::null_mut(),
            litLengthFreq: core::ptr::null_mut(),
            matchLengthFreq: core::ptr::null_mut(),
            offCodeFreq: core::ptr::null_mut(),
            matchTable: core::ptr::null_mut(),
            priceTable: core::ptr::null_mut(),
            litSum: 0,
            litLengthSum: 0,
            matchLengthSum: 0,
            offCodeSum: 0,
            litSumBasePrice: 0,
            litLengthSumBasePrice: 0,
            matchLengthSumBasePrice: 0,
            offCodeSumBasePrice: 0,
            priceType: 0,
            symbolCosts: core::ptr::null(),
            literalCompressionMode: 0,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct ZSTD_compressedBlockState_t {
    pub entropy: ZSTD_entropyCTables_t,
    pub rep: [U32; ZSTD_REP_NUM],
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct ZSTD_window_t {
    pub nextSrc: *const BYTE,
    pub base: *const BYTE,
    pub dictBase: *const BYTE,
    pub dictLimit: U32,
    pub lowLimit: U32,
    pub nbOverflowCorrections: U32,
}

impl Default for ZSTD_window_t {
    fn default() -> Self {
        ZSTD_window_t {
            nextSrc: core::ptr::null(),
            base: core::ptr::null(),
            dictBase: core::ptr::null(),
            dictLimit: 0,
            lowLimit: 0,
            nbOverflowCorrections: 0,
        }
    }
}

pub const ZSTD_WINDOW_START_INDEX: U32 = 2;
pub const ZSTD_ROW_HASH_CACHE_SIZE: usize = 8;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct ZSTD_MatchState_t {
    pub window: ZSTD_window_t,
    pub loadedDictEnd: U32,
    pub nextToUpdate: U32,
    pub hashLog3: U32,
    pub rowHashLog: U32,
    pub tagTable: *mut BYTE,
    pub hashCache: [U32; ZSTD_ROW_HASH_CACHE_SIZE],
    pub hashSalt: U64,
    pub hashSaltEntropy: U32,
    pub hashTable: *mut U32,
    pub hashTable3: *mut U32,
    pub chainTable: *mut U32,
    pub forceNonContiguous: c_int,
    pub dedicatedDictSearch: c_int,
    pub opt: optState_t,
    pub dictMatchState: *const ZSTD_MatchState_t,
    pub cParams: ZSTD_compressionParameters,
    pub ldmSeqStore: *const RawSeqStore_t,
    pub prefetchCDictTables: c_int,
    pub lazySkipping: c_int,
}

impl Default for ZSTD_MatchState_t {
    fn default() -> Self {
        ZSTD_MatchState_t {
            window: ZSTD_window_t::default(),
            loadedDictEnd: 0,
            nextToUpdate: 0,
            hashLog3: 0,
            rowHashLog: 0,
            tagTable: core::ptr::null_mut(),
            hashCache: [0; ZSTD_ROW_HASH_CACHE_SIZE],
            hashSalt: 0,
            hashSaltEntropy: 0,
            hashTable: core::ptr::null_mut(),
            hashTable3: core::ptr::null_mut(),
            chainTable: core::ptr::null_mut(),
            forceNonContiguous: 0,
            dedicatedDictSearch: 0,
            opt: optState_t::default(),
            dictMatchState: core::ptr::null(),
            cParams: ZSTD_compressionParameters::default(),
            ldmSeqStore: core::ptr::null(),
            prefetchCDictTables: 0,
            lazySkipping: 0,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct ZSTD_blockState_t {
    pub prevCBlock: *mut ZSTD_compressedBlockState_t,
    pub nextCBlock: *mut ZSTD_compressedBlockState_t,
    pub matchState: ZSTD_MatchState_t,
}

impl Default for ZSTD_blockState_t {
    fn default() -> Self {
        ZSTD_blockState_t {
            prevCBlock: core::ptr::null_mut(),
            nextCBlock: core::ptr::null_mut(),
            matchState: ZSTD_MatchState_t::default(),
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct ldmEntry_t {
    pub offset: U32,
    pub checksum: U32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct ldmMatchCandidate_t {
    pub split: *const BYTE,
    pub hash: U32,
    pub checksum: U32,
    pub bucket: *mut ldmEntry_t,
}

impl Default for ldmMatchCandidate_t {
    fn default() -> Self {
        ldmMatchCandidate_t {
            split: core::ptr::null(),
            hash: 0,
            checksum: 0,
            bucket: core::ptr::null_mut(),
        }
    }
}

pub const LDM_BATCH_SIZE: usize = 64;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct ldmState_t {
    pub window: ZSTD_window_t,
    pub hashTable: *mut ldmEntry_t,
    pub loadedDictEnd: U32,
    pub bucketOffsets: *mut BYTE,
    pub splitIndices: [usize; LDM_BATCH_SIZE],
    pub matchCandidates: [ldmMatchCandidate_t; LDM_BATCH_SIZE],
}

impl Default for ldmState_t {
    fn default() -> Self {
        ldmState_t {
            window: ZSTD_window_t::default(),
            hashTable: core::ptr::null_mut(),
            loadedDictEnd: 0,
            bucketOffsets: core::ptr::null_mut(),
            splitIndices: [0; LDM_BATCH_SIZE],
            matchCandidates: [ldmMatchCandidate_t::default(); LDM_BATCH_SIZE],
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct ldmParams_t {
    pub enableLdm: ZSTD_ParamSwitch_e,
    pub hashLog: U32,
    pub bucketSizeLog: U32,
    pub minMatchLength: U32,
    pub hashRateLog: U32,
    pub windowLog: U32,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct SeqCollector {
    pub collectSequences: c_int,
    pub seqStart: *mut ZSTD_Sequence,
    pub seqIndex: usize,
    pub maxSequences: usize,
}

impl Default for SeqCollector {
    fn default() -> Self {
        SeqCollector {
            collectSequences: 0,
            seqStart: core::ptr::null_mut(),
            seqIndex: 0,
            maxSequences: 0,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct ZSTD_CCtx_params {
    pub format: ZSTD_format_e,
    pub cParams: ZSTD_compressionParameters,
    pub fParams: ZSTD_frameParameters,

    pub compressionLevel: c_int,
    pub forceWindow: c_int,
    pub targetCBlockSize: usize,
    pub srcSizeHint: c_int,

    pub attachDictPref: ZSTD_dictAttachPref_e,
    pub literalCompressionMode: ZSTD_ParamSwitch_e,

    pub nbWorkers: c_int,
    pub jobSize: usize,
    pub overlapLog: c_int,
    pub rsyncable: c_int,

    pub ldmParams: ldmParams_t,

    pub enableDedicatedDictSearch: c_int,

    pub inBufferMode: ZSTD_bufferMode_e,
    pub outBufferMode: ZSTD_bufferMode_e,

    pub blockDelimiters: ZSTD_SequenceFormat_e,
    pub validateSequences: c_int,

    pub postBlockSplitter: ZSTD_ParamSwitch_e,
    pub preBlockSplitter_level: c_int,

    pub maxBlockSize: usize,

    pub useRowMatchFinder: ZSTD_ParamSwitch_e,

    pub deterministicRefPrefix: c_int,

    pub customMem: ZSTD_customMem,

    pub prefetchCDictTables: ZSTD_ParamSwitch_e,

    pub enableMatchFinderFallback: c_int,

    pub extSeqProdState: *mut c_void,
    pub extSeqProdFunc: ZSTD_sequenceProducer_F,

    pub searchForExternalRepcodes: ZSTD_ParamSwitch_e,
}

impl Default for ZSTD_CCtx_params {
    fn default() -> Self {
        ZSTD_CCtx_params {
            format: 0,
            cParams: ZSTD_compressionParameters::default(),
            fParams: ZSTD_frameParameters::default(),
            compressionLevel: 0,
            forceWindow: 0,
            targetCBlockSize: 0,
            srcSizeHint: 0,
            attachDictPref: 0,
            literalCompressionMode: 0,
            nbWorkers: 0,
            jobSize: 0,
            overlapLog: 0,
            rsyncable: 0,
            ldmParams: ldmParams_t::default(),
            enableDedicatedDictSearch: 0,
            inBufferMode: 0,
            outBufferMode: 0,
            blockDelimiters: 0,
            validateSequences: 0,
            postBlockSplitter: 0,
            preBlockSplitter_level: 0,
            maxBlockSize: 0,
            useRowMatchFinder: 0,
            deterministicRefPrefix: 0,
            customMem: ZSTD_customMem::default(),
            prefetchCDictTables: 0,
            enableMatchFinderFallback: 0,
            extSeqProdState: core::ptr::null_mut(),
            extSeqProdFunc: None,
            searchForExternalRepcodes: 0,
        }
    }
}

pub const COMPRESS_SEQUENCES_WORKSPACE_SIZE: usize = 4 * (MaxSeq as usize + 2);
pub const ENTROPY_WORKSPACE_SIZE: usize = HUF_WORKSPACE_SIZE + COMPRESS_SEQUENCES_WORKSPACE_SIZE;
pub const TMP_WORKSPACE_SIZE: usize = if ENTROPY_WORKSPACE_SIZE > ZSTD_SLIPBLOCK_WORKSPACESIZE {
    ENTROPY_WORKSPACE_SIZE
} else {
    ZSTD_SLIPBLOCK_WORKSPACESIZE
};

pub type ZSTD_buffered_policy_e = c_uint;
pub const ZSTDb_not_buffered: ZSTD_buffered_policy_e = 0;
pub const ZSTDb_buffered: ZSTD_buffered_policy_e = 1;

pub const ZSTD_MAX_NB_BLOCK_SPLITS: usize = 196;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct ZSTD_blockSplitCtx {
    pub fullSeqStoreChunk: SeqStore_t,
    pub firstHalfSeqStore: SeqStore_t,
    pub secondHalfSeqStore: SeqStore_t,
    pub currSeqStore: SeqStore_t,
    pub nextSeqStore: SeqStore_t,
    pub partitions: [U32; ZSTD_MAX_NB_BLOCK_SPLITS],
    pub entropyMetadata: ZSTD_entropyCTablesMetadata_t,
}

impl Default for ZSTD_blockSplitCtx {
    fn default() -> Self {
        ZSTD_blockSplitCtx {
            fullSeqStoreChunk: SeqStore_t::default(),
            firstHalfSeqStore: SeqStore_t::default(),
            secondHalfSeqStore: SeqStore_t::default(),
            currSeqStore: SeqStore_t::default(),
            nextSeqStore: SeqStore_t::default(),
            partitions: [0; ZSTD_MAX_NB_BLOCK_SPLITS],
            entropyMetadata: ZSTD_entropyCTablesMetadata_t::default(),
        }
    }
}

/// `struct ZSTD_CCtx_s`
#[repr(C)]
pub struct ZSTD_CCtx {
    pub stage: ZSTD_compressionStage_e,
    pub cParamsChanged: c_int,
    pub bmi2: c_int,
    pub requestedParams: ZSTD_CCtx_params,
    pub appliedParams: ZSTD_CCtx_params,
    pub simpleApiParams: ZSTD_CCtx_params,
    pub dictID: U32,
    pub dictContentSize: usize,

    pub workspace: ZSTD_cwksp,
    pub blockSizeMax: usize,
    pub pledgedSrcSizePlusOne: u64,
    pub consumedSrcSize: u64,
    pub producedCSize: u64,
    pub xxhState: XXH64_state_t,
    pub customMem: ZSTD_customMem,
    pub pool: *mut crate::pool::POOL_ctx,
    pub staticSize: usize,
    pub seqCollector: SeqCollector,
    pub isFirstBlock: c_int,
    pub initialized: c_int,

    pub seqStore: SeqStore_t,
    pub ldmState: ldmState_t,
    pub ldmSequences: *mut rawSeq,
    pub maxNbLdmSequences: usize,
    pub externSeqStore: RawSeqStore_t,
    pub blockState: ZSTD_blockState_t,
    pub tmpWorkspace: *mut c_void,
    pub tmpWkspSize: usize,

    pub bufferedPolicy: ZSTD_buffered_policy_e,

    pub inBuff: *mut i8,
    pub inBuffSize: usize,
    pub inToCompress: usize,
    pub inBuffPos: usize,
    pub inBuffTarget: usize,
    pub outBuff: *mut i8,
    pub outBuffSize: usize,
    pub outBuffContentSize: usize,
    pub outBuffFlushedSize: usize,
    pub streamStage: ZSTD_cStreamStage,
    pub frameEnded: U32,

    pub expectedInBuffer: ZSTD_inBuffer,
    pub stableIn_notConsumed: usize,
    pub expectedOutBufferSize: usize,

    pub localDict: ZSTD_localDict,
    pub cdict: *const ZSTD_CDict,
    pub prefixDict: ZSTD_prefixDict,

    /* Tracing (ZSTD_TRACE == 1 on this platform: GCC/ELF/x86-64) */
    pub traceCtx: ZSTD_TraceCtx,

    pub blockSplitCtx: ZSTD_blockSplitCtx,

    pub extSeqBuf: *mut ZSTD_Sequence,
    pub extSeqBufCapacity: usize,
}

pub type ZSTD_CStream = ZSTD_CCtx;

/// `struct ZSTD_CDict_s` (defined in zstd_compress.c)
#[repr(C)]
pub struct ZSTD_CDict {
    pub dictContent: *const c_void,
    pub dictContentSize: usize,
    pub dictContentType: ZSTD_dictContentType_e,
    pub entropyWorkspace: *mut U32,
    pub workspace: ZSTD_cwksp,
    pub matchState: ZSTD_MatchState_t,
    pub cBlockState: ZSTD_compressedBlockState_t,
    pub customMem: ZSTD_customMem,
    pub dictID: U32,
    pub compressionLevel: c_int,
    pub useRowMatchFinder: ZSTD_ParamSwitch_e,
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
        *mut ZSTD_MatchState_t,
        *mut SeqStore_t,
        *mut U32,
        *const c_void,
        usize,
    ) -> usize,
>;

static LL_Code: [BYTE; 64] = [
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 16, 17, 17, 18, 18, 19, 19, 20, 20,
    20, 20, 21, 21, 21, 21, 22, 22, 22, 22, 22, 22, 22, 22, 23, 23, 23, 23, 23, 23, 23, 23, 24, 24,
    24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24, 24,
];
const LL_deltaCode: U32 = 19;

#[inline(always)]
pub fn ZSTD_LLcode(litLength: U32) -> U32 {
    if litLength > 63 {
        ZSTD_highbit32(litLength) + LL_deltaCode
    } else {
        LL_Code[litLength as usize] as U32
    }
}

static ML_Code: [BYTE; 128] = [
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
    26, 27, 28, 29, 30, 31, 32, 32, 33, 33, 34, 34, 35, 35, 36, 36, 36, 36, 37, 37, 37, 37, 38, 38,
    38, 38, 38, 38, 38, 38, 39, 39, 39, 39, 39, 39, 39, 39, 40, 40, 40, 40, 40, 40, 40, 40, 40, 40,
    40, 40, 40, 40, 40, 40, 41, 41, 41, 41, 41, 41, 41, 41, 41, 41, 41, 41, 41, 41, 41, 41, 42, 42,
    42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42, 42,
    42, 42, 42, 42, 42, 42,
];
const ML_deltaCode: U32 = 36;

#[inline(always)]
pub fn ZSTD_MLcode(mlBase: U32) -> U32 {
    if mlBase > 127 {
        ZSTD_highbit32(mlBase) + ML_deltaCode
    } else {
        ML_Code[mlBase as usize] as U32
    }
}

/* NOTE: `ZSTD_cParam_withinBounds()` from zstd_compress_internal.h lives in
 * `compress/zstd_compress.rs` (its only non-assert user) to avoid a circular
 * module reference. */

#[inline(always)]
pub unsafe fn ZSTD_selectAddr(
    index: U32,
    lowLimit: U32,
    candidate: *const BYTE,
    backup: *const BYTE,
) -> *const BYTE {
    if index >= lowLimit {
        candidate
    } else {
        backup
    }
}

#[inline(always)]
pub unsafe fn ZSTD_noCompressBlock(
    dst: *mut c_void,
    dstCapacity: usize,
    src: *const c_void,
    srcSize: usize,
    lastBlock: U32,
) -> usize {
    let cBlockHeader24: U32 = lastBlock
        .wrapping_add((bt_raw as U32) << 1)
        .wrapping_add((srcSize << 3) as U32);
    if srcSize + ZSTD_blockHeaderSize > dstCapacity {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    MEM_writeLE24(dst, cBlockHeader24);
    ZSTD_memcpy(
        (dst as *mut BYTE).add(ZSTD_blockHeaderSize) as *mut c_void,
        src,
        srcSize,
    );
    ZSTD_blockHeaderSize + srcSize
}

#[inline(always)]
pub unsafe fn ZSTD_rleCompressBlock(
    dst: *mut c_void,
    dstCapacity: usize,
    src: BYTE,
    srcSize: usize,
    lastBlock: U32,
) -> usize {
    let op = dst as *mut BYTE;
    let cBlockHeader: U32 = lastBlock
        .wrapping_add((bt_rle as U32) << 1)
        .wrapping_add((srcSize << 3) as U32);
    if dstCapacity < 4 {
        return ERROR(ZSTD_error_dstSize_tooSmall);
    }
    MEM_writeLE24(op as *mut c_void, cBlockHeader);
    *op.add(3) = src;
    4
}

#[inline(always)]
pub fn ZSTD_minGain(srcSize: usize, strat: ZSTD_strategy) -> usize {
    let minlog: U32 = if strat >= ZSTD_btultra {
        strat - 1
    } else {
        6
    };
    (srcSize >> minlog) + 2
}

#[inline(always)]
pub unsafe fn ZSTD_literalsCompressionIsDisabled(cctxParams: *const ZSTD_CCtx_params) -> c_int {
    match (*cctxParams).literalCompressionMode {
        ZSTD_ps_enable => 0,
        ZSTD_ps_disable => 1,
        _ => {
            (((*cctxParams).cParams.strategy == ZSTD_fast)
                && ((*cctxParams).cParams.targetLength > 0)) as c_int
        }
    }
}

pub unsafe fn ZSTD_safecopyLiterals(
    mut op: *mut BYTE,
    mut ip: *const BYTE,
    iend: *const BYTE,
    ilimit_w: *const BYTE,
) {
    if ip <= ilimit_w {
        ZSTD_wildcopy(
            op as *mut c_void,
            ip as *const c_void,
            ilimit_w.offset_from(ip),
            ZSTD_no_overlap,
        );
        op = op.offset(ilimit_w.offset_from(ip));
        ip = ilimit_w;
    }
    while ip < iend {
        *op = *ip;
        op = op.add(1);
        ip = ip.add(1);
    }
}

pub const REPCODE1_TO_OFFBASE: U32 = 1;
pub const REPCODE2_TO_OFFBASE: U32 = 2;
pub const REPCODE3_TO_OFFBASE: U32 = 3;

#[inline(always)]
pub const fn REPCODE_TO_OFFBASE(r: U32) -> U32 {
    r
}
#[inline(always)]
pub const fn OFFSET_TO_OFFBASE(o: U32) -> U32 {
    o + ZSTD_REP_NUM as U32
}
#[inline(always)]
pub const fn OFFBASE_IS_OFFSET(o: U32) -> bool {
    o > ZSTD_REP_NUM as U32
}
#[inline(always)]
pub const fn OFFBASE_IS_REPCODE(o: U32) -> bool {
    1 <= o && o <= ZSTD_REP_NUM as U32
}
#[inline(always)]
pub const fn OFFBASE_TO_OFFSET(o: U32) -> U32 {
    o - ZSTD_REP_NUM as U32
}
#[inline(always)]
pub const fn OFFBASE_TO_REPCODE(o: U32) -> U32 {
    o
}

#[inline]
pub unsafe fn ZSTD_storeSeqOnly(
    seqStorePtr: *mut SeqStore_t,
    litLength: usize,
    offBase: U32,
    matchLength: usize,
) {
    /* literal Length */
    if litLength > 0xFFFF {
        (*seqStorePtr).longLengthType = ZSTD_llt_literalLength;
        (*seqStorePtr).longLengthPos =
            (*seqStorePtr).sequences.offset_from((*seqStorePtr).sequencesStart) as U32;
    }
    (*(*seqStorePtr).sequences).litLength = litLength as U16;

    /* match offset */
    (*(*seqStorePtr).sequences).offBase = offBase;

    /* match Length */
    {
        let mlBase = matchLength - MINMATCH;
        if mlBase > 0xFFFF {
            (*seqStorePtr).longLengthType = ZSTD_llt_matchLength;
            (*seqStorePtr).longLengthPos =
                (*seqStorePtr).sequences.offset_from((*seqStorePtr).sequencesStart) as U32;
        }
        (*(*seqStorePtr).sequences).mlBase = mlBase as U16;
    }

    (*seqStorePtr).sequences = (*seqStorePtr).sequences.add(1);
}

#[inline]
pub unsafe fn ZSTD_storeSeq(
    seqStorePtr: *mut SeqStore_t,
    litLength: usize,
    literals: *const BYTE,
    litLimit: *const BYTE,
    offBase: U32,
    matchLength: usize,
) {
    let litLimit_w = litLimit.sub(WILDCOPY_OVERLENGTH);
    let litEnd = literals.add(litLength);

    if litEnd <= litLimit_w {
        ZSTD_copy16(
            (*seqStorePtr).lit as *mut c_void,
            literals as *const c_void,
        );
        if litLength > 16 {
            ZSTD_wildcopy(
                (*seqStorePtr).lit.add(16) as *mut c_void,
                literals.add(16) as *const c_void,
                litLength as isize - 16,
                ZSTD_no_overlap,
            );
        }
    } else {
        ZSTD_safecopyLiterals((*seqStorePtr).lit, literals, litEnd, litLimit_w);
    }
    (*seqStorePtr).lit = (*seqStorePtr).lit.add(litLength);

    ZSTD_storeSeqOnly(seqStorePtr, litLength, offBase, matchLength);
}

#[inline(always)]
pub unsafe fn ZSTD_updateRep(rep: *mut U32, offBase: U32, ll0: U32) {
    if OFFBASE_IS_OFFSET(offBase) {
        *rep.add(2) = *rep.add(1);
        *rep.add(1) = *rep.add(0);
        *rep.add(0) = OFFBASE_TO_OFFSET(offBase);
    } else {
        let repCode: U32 = OFFBASE_TO_REPCODE(offBase) - 1 + ll0;
        if repCode > 0 {
            let currentOffset: U32 = if repCode == ZSTD_REP_NUM as U32 {
                (*rep.add(0)).wrapping_sub(1)
            } else {
                *rep.add(repCode as usize)
            };
            *rep.add(2) = if repCode >= 2 { *rep.add(1) } else { *rep.add(2) };
            *rep.add(1) = *rep.add(0);
            *rep.add(0) = currentOffset;
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct Repcodes_t {
    pub rep: [U32; 3],
}

#[inline(always)]
pub unsafe fn ZSTD_newRep(rep: *const U32, offBase: U32, ll0: U32) -> Repcodes_t {
    let mut newReps = Repcodes_t::default();
    ZSTD_memcpy(
        &mut newReps as *mut Repcodes_t as *mut c_void,
        rep as *const c_void,
        core::mem::size_of::<Repcodes_t>(),
    );
    ZSTD_updateRep(newReps.rep.as_mut_ptr(), offBase, ll0);
    newReps
}

/* ===== Match length counter ===== */

#[inline(always)]
pub unsafe fn ZSTD_count(
    mut pIn: *const BYTE,
    mut pMatch: *const BYTE,
    pInLimit: *const BYTE,
) -> usize {
    let pStart = pIn;
    let pInLoopLimit = pInLimit.sub(core::mem::size_of::<usize>() - 1);

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
        && (pIn < pInLimit.sub(3))
        && (MEM_read32(pMatch as *const c_void) == MEM_read32(pIn as *const c_void))
    {
        pIn = pIn.add(4);
        pMatch = pMatch.add(4);
    }
    if (pIn < pInLimit.sub(1))
        && (MEM_read16(pMatch as *const c_void) == MEM_read16(pIn as *const c_void))
    {
        pIn = pIn.add(2);
        pMatch = pMatch.add(2);
    }
    if (pIn < pInLimit) && (*pMatch == *pIn) {
        pIn = pIn.add(1);
    }
    pIn.offset_from(pStart) as usize
}

#[inline(always)]
pub unsafe fn ZSTD_count_2segments(
    ip: *const BYTE,
    match_: *const BYTE,
    iEnd: *const BYTE,
    mEnd: *const BYTE,
    iStart: *const BYTE,
) -> usize {
    let a = ip.offset(mEnd.offset_from(match_));
    let vEnd = if a < iEnd { a } else { iEnd };
    let matchLength = ZSTD_count(ip, match_, vEnd);
    if match_.add(matchLength) != mEnd {
        return matchLength;
    }
    matchLength + ZSTD_count(ip.add(matchLength), iStart, iEnd)
}

/* ===== Hashes ===== */

pub const prime3bytes: U32 = 506832829;
pub const prime4bytes: U32 = 2654435761;
pub const prime5bytes: U64 = 889523592379;
pub const prime6bytes: U64 = 227718039650203;
pub const prime7bytes: U64 = 58295818150454627;
pub const prime8bytes: U64 = 0xCF1BBCDCB7A56463;

#[inline(always)]
pub fn ZSTD_hash3(u: U32, h: U32, s: U32) -> U32 {
    (((u << (32 - 24)).wrapping_mul(prime3bytes)) ^ s) >> (32 - h)
}
#[inline(always)]
pub unsafe fn ZSTD_hash3Ptr(ptr: *const c_void, h: U32) -> usize {
    ZSTD_hash3(MEM_readLE32(ptr), h, 0) as usize
}
#[inline(always)]
pub unsafe fn ZSTD_hash3PtrS(ptr: *const c_void, h: U32, s: U32) -> usize {
    ZSTD_hash3(MEM_readLE32(ptr), h, s) as usize
}

#[inline(always)]
pub fn ZSTD_hash4(u: U32, h: U32, s: U32) -> U32 {
    ((u.wrapping_mul(prime4bytes)) ^ s) >> (32 - h)
}
#[inline(always)]
pub unsafe fn ZSTD_hash4Ptr(ptr: *const c_void, h: U32) -> usize {
    ZSTD_hash4(MEM_readLE32(ptr), h, 0) as usize
}
#[inline(always)]
pub unsafe fn ZSTD_hash4PtrS(ptr: *const c_void, h: U32, s: U32) -> usize {
    ZSTD_hash4(MEM_readLE32(ptr), h, s) as usize
}

#[inline(always)]
pub fn ZSTD_hash5(u: U64, h: U32, s: U64) -> usize {
    ((((u << (64 - 40)).wrapping_mul(prime5bytes)) ^ s) >> (64 - h)) as usize
}
#[inline(always)]
pub unsafe fn ZSTD_hash5Ptr(p: *const c_void, h: U32) -> usize {
    ZSTD_hash5(MEM_readLE64(p), h, 0)
}
#[inline(always)]
pub unsafe fn ZSTD_hash5PtrS(p: *const c_void, h: U32, s: U64) -> usize {
    ZSTD_hash5(MEM_readLE64(p), h, s)
}

#[inline(always)]
pub fn ZSTD_hash6(u: U64, h: U32, s: U64) -> usize {
    ((((u << (64 - 48)).wrapping_mul(prime6bytes)) ^ s) >> (64 - h)) as usize
}
#[inline(always)]
pub unsafe fn ZSTD_hash6Ptr(p: *const c_void, h: U32) -> usize {
    ZSTD_hash6(MEM_readLE64(p), h, 0)
}
#[inline(always)]
pub unsafe fn ZSTD_hash6PtrS(p: *const c_void, h: U32, s: U64) -> usize {
    ZSTD_hash6(MEM_readLE64(p), h, s)
}

#[inline(always)]
pub fn ZSTD_hash7(u: U64, h: U32, s: U64) -> usize {
    ((((u << (64 - 56)).wrapping_mul(prime7bytes)) ^ s) >> (64 - h)) as usize
}
#[inline(always)]
pub unsafe fn ZSTD_hash7Ptr(p: *const c_void, h: U32) -> usize {
    ZSTD_hash7(MEM_readLE64(p), h, 0)
}
#[inline(always)]
pub unsafe fn ZSTD_hash7PtrS(p: *const c_void, h: U32, s: U64) -> usize {
    ZSTD_hash7(MEM_readLE64(p), h, s)
}

#[inline(always)]
pub fn ZSTD_hash8(u: U64, h: U32, s: U64) -> usize {
    (((u.wrapping_mul(prime8bytes)) ^ s) >> (64 - h)) as usize
}
#[inline(always)]
pub unsafe fn ZSTD_hash8Ptr(p: *const c_void, h: U32) -> usize {
    ZSTD_hash8(MEM_readLE64(p), h, 0)
}
#[inline(always)]
pub unsafe fn ZSTD_hash8PtrS(p: *const c_void, h: U32, s: U64) -> usize {
    ZSTD_hash8(MEM_readLE64(p), h, s)
}

#[inline(always)]
pub unsafe fn ZSTD_hashPtr(p: *const c_void, hBits: U32, mls: U32) -> usize {
    match mls {
        5 => ZSTD_hash5Ptr(p, hBits),
        6 => ZSTD_hash6Ptr(p, hBits),
        7 => ZSTD_hash7Ptr(p, hBits),
        8 => ZSTD_hash8Ptr(p, hBits),
        _ => ZSTD_hash4Ptr(p, hBits),
    }
}

#[inline(always)]
pub unsafe fn ZSTD_hashPtrSalted(p: *const c_void, hBits: U32, mls: U32, hashSalt: U64) -> usize {
    match mls {
        5 => ZSTD_hash5PtrS(p, hBits, hashSalt),
        6 => ZSTD_hash6PtrS(p, hBits, hashSalt),
        7 => ZSTD_hash7PtrS(p, hBits, hashSalt),
        8 => ZSTD_hash8PtrS(p, hBits, hashSalt),
        _ => ZSTD_hash4PtrS(p, hBits, hashSalt as U32),
    }
}

pub fn ZSTD_ipow(mut base: U64, mut exponent: U64) -> U64 {
    let mut power: U64 = 1;
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

pub unsafe fn ZSTD_rollingHash_append(mut hash: U64, buf: *const c_void, size: usize) -> U64 {
    let istart = buf as *const BYTE;
    let mut pos = 0usize;
    while pos < size {
        hash = hash.wrapping_mul(prime8bytes);
        hash = hash.wrapping_add((*istart.add(pos) as U64).wrapping_add(ZSTD_ROLL_HASH_CHAR_OFFSET));
        pos += 1;
    }
    hash
}

#[inline(always)]
pub unsafe fn ZSTD_rollingHash_compute(buf: *const c_void, size: usize) -> U64 {
    ZSTD_rollingHash_append(0, buf, size)
}

#[inline(always)]
pub fn ZSTD_rollingHash_primePower(length: U32) -> U64 {
    ZSTD_ipow(prime8bytes, (length - 1) as U64)
}

#[inline(always)]
pub fn ZSTD_rollingHash_rotate(mut hash: U64, toRemove: BYTE, toAdd: BYTE, primePower: U64) -> U64 {
    hash = hash.wrapping_sub(
        ((toRemove as U64).wrapping_add(ZSTD_ROLL_HASH_CHAR_OFFSET)).wrapping_mul(primePower),
    );
    hash = hash.wrapping_mul(prime8bytes);
    hash = hash.wrapping_add((toAdd as U64).wrapping_add(ZSTD_ROLL_HASH_CHAR_OFFSET));
    hash
}

/* ===== Round buffer management ===== */

#[inline(always)]
pub fn ZSTD_CURRENT_MAX() -> U32 {
    if MEM_64bits() != 0 {
        3500u32 * (1 << 20)
    } else {
        2000u32 * (1 << 20)
    }
}

#[inline(always)]
pub fn ZSTD_CHUNKSIZE_MAX() -> U32 {
    (u32::MAX).wrapping_sub(ZSTD_CURRENT_MAX())
}

#[inline(always)]
pub unsafe fn ZSTD_window_clear(window: *mut ZSTD_window_t) {
    let endT = (*window).nextSrc.offset_from((*window).base) as usize;
    let end = endT as U32;
    (*window).lowLimit = end;
    (*window).dictLimit = end;
}

#[inline(always)]
pub unsafe fn ZSTD_window_isEmpty(window: ZSTD_window_t) -> U32 {
    (window.dictLimit == ZSTD_WINDOW_START_INDEX
        && window.lowLimit == ZSTD_WINDOW_START_INDEX
        && (window.nextSrc.offset_from(window.base) as U32) == ZSTD_WINDOW_START_INDEX)
        as U32
}

#[inline(always)]
pub fn ZSTD_window_hasExtDict(window: ZSTD_window_t) -> U32 {
    (window.lowLimit < window.dictLimit) as U32
}

#[inline(always)]
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

#[inline(always)]
pub unsafe fn ZSTD_window_canOverflowCorrect(
    window: ZSTD_window_t,
    cycleLog: U32,
    maxDist: U32,
    loadedDictEnd: U32,
    src: *const c_void,
) -> U32 {
    let cycleSize: U32 = 1u32 << cycleLog;
    let curr: U32 = (src as *const BYTE).offset_from(window.base) as U32;
    let minIndexToOverflowCorrect: U32 = cycleSize
        .wrapping_add(if maxDist > cycleSize { maxDist } else { cycleSize })
        .wrapping_add(ZSTD_WINDOW_START_INDEX);

    let adjustment: U32 = window.nbOverflowCorrections.wrapping_add(1);
    let prod = minIndexToOverflowCorrect.wrapping_mul(adjustment);
    let adjustedIndex: U32 = if prod > minIndexToOverflowCorrect {
        prod
    } else {
        minIndexToOverflowCorrect
    };
    let indexLargeEnough: U32 = (curr > adjustedIndex) as U32;
    let dictionaryInvalidated: U32 = (curr > maxDist.wrapping_add(loadedDictEnd)) as U32;
    ((indexLargeEnough != 0) && (dictionaryInvalidated != 0)) as U32
}

#[inline(always)]
pub unsafe fn ZSTD_window_needOverflowCorrection(
    window: ZSTD_window_t,
    _cycleLog: U32,
    _maxDist: U32,
    _loadedDictEnd: U32,
    _src: *const c_void,
    srcEnd: *const c_void,
) -> U32 {
    let curr: U32 = (srcEnd as *const BYTE).offset_from(window.base) as U32;
    (curr > ZSTD_CURRENT_MAX()) as U32
}

#[inline(always)]
pub unsafe fn ZSTD_window_correctOverflow(
    window: *mut ZSTD_window_t,
    cycleLog: U32,
    maxDist: U32,
    src: *const c_void,
) -> U32 {
    let cycleSize: U32 = 1u32 << cycleLog;
    let cycleMask: U32 = cycleSize.wrapping_sub(1);
    let curr: U32 = ((src as *const BYTE as usize).wrapping_sub((*window).base as usize)) as U32;
    let currentCycle: U32 = curr & cycleMask;
    let currentCycleCorrection: U32 = if currentCycle < ZSTD_WINDOW_START_INDEX {
        if cycleSize > ZSTD_WINDOW_START_INDEX {
            cycleSize
        } else {
            ZSTD_WINDOW_START_INDEX
        }
    } else {
        0
    };
    let newCurrent: U32 = currentCycle
        .wrapping_add(currentCycleCorrection)
        .wrapping_add(if maxDist > cycleSize { maxDist } else { cycleSize });
    let correction: U32 = curr.wrapping_sub(newCurrent);

    (*window).base = ((*window).base as usize).wrapping_add(correction as usize) as *const BYTE;
    (*window).dictBase =
        ((*window).dictBase as usize).wrapping_add(correction as usize) as *const BYTE;
    if (*window).lowLimit < correction.wrapping_add(ZSTD_WINDOW_START_INDEX) {
        (*window).lowLimit = ZSTD_WINDOW_START_INDEX;
    } else {
        (*window).lowLimit -= correction;
    }
    if (*window).dictLimit < correction.wrapping_add(ZSTD_WINDOW_START_INDEX) {
        (*window).dictLimit = ZSTD_WINDOW_START_INDEX;
    } else {
        (*window).dictLimit -= correction;
    }

    (*window).nbOverflowCorrections += 1;
    correction
}

#[inline(always)]
pub unsafe fn ZSTD_window_enforceMaxDist(
    window: *mut ZSTD_window_t,
    blockEnd: *const c_void,
    maxDist: U32,
    loadedDictEndPtr: *mut U32,
    dictMatchStatePtr: *mut *const ZSTD_MatchState_t,
) {
    let blockEndIdx: U32 = (blockEnd as *const BYTE).offset_from((*window).base) as U32;
    let loadedDictEnd: U32 = if !loadedDictEndPtr.is_null() {
        *loadedDictEndPtr
    } else {
        0
    };

    if blockEndIdx > maxDist.wrapping_add(loadedDictEnd) {
        let newLowLimit: U32 = blockEndIdx - maxDist;
        if (*window).lowLimit < newLowLimit {
            (*window).lowLimit = newLowLimit;
        }
        if (*window).dictLimit < (*window).lowLimit {
            (*window).dictLimit = (*window).lowLimit;
        }
        if !loadedDictEndPtr.is_null() {
            *loadedDictEndPtr = 0;
        }
        if !dictMatchStatePtr.is_null() {
            *dictMatchStatePtr = core::ptr::null();
        }
    }
}

#[inline(always)]
pub unsafe fn ZSTD_checkDictValidity(
    window: *const ZSTD_window_t,
    blockEnd: *const c_void,
    maxDist: U32,
    loadedDictEndPtr: *mut U32,
    dictMatchStatePtr: *mut *const ZSTD_MatchState_t,
) {
    let blockEndIdx: U32 = (blockEnd as *const BYTE).offset_from((*window).base) as U32;
    let loadedDictEnd: U32 = *loadedDictEndPtr;

    if blockEndIdx > loadedDictEnd.wrapping_add(maxDist) || loadedDictEnd != (*window).dictLimit {
        *loadedDictEndPtr = 0;
        *dictMatchStatePtr = core::ptr::null();
    }
}

static ZSTD_WINDOW_INIT_BASE: [u8; 2] = [b' ', 0];

#[inline(always)]
pub unsafe fn ZSTD_window_init(window: *mut ZSTD_window_t) {
    ZSTD_memset(
        window as *mut c_void,
        0,
        core::mem::size_of::<ZSTD_window_t>(),
    );
    (*window).base = ZSTD_WINDOW_INIT_BASE.as_ptr();
    (*window).dictBase = ZSTD_WINDOW_INIT_BASE.as_ptr();
    (*window).dictLimit = ZSTD_WINDOW_START_INDEX;
    (*window).lowLimit = ZSTD_WINDOW_START_INDEX;
    (*window).nextSrc = (*window).base.add(ZSTD_WINDOW_START_INDEX as usize);
    (*window).nbOverflowCorrections = 0;
}

#[inline(always)]
pub unsafe fn ZSTD_window_update(
    window: *mut ZSTD_window_t,
    src: *const c_void,
    srcSize: usize,
    forceNonContiguous: c_int,
) -> U32 {
    let ip = src as *const BYTE;
    let mut contiguous: U32 = 1;
    if srcSize == 0 {
        return contiguous;
    }
    if src as *const BYTE != (*window).nextSrc || forceNonContiguous != 0 {
        let distanceFromBase =
            ((*window).nextSrc as usize).wrapping_sub((*window).base as usize);
        (*window).lowLimit = (*window).dictLimit;
        (*window).dictLimit = distanceFromBase as U32;
        (*window).dictBase = (*window).base;
        (*window).base = (ip as usize).wrapping_sub(distanceFromBase) as *const BYTE;
        if (*window).dictLimit.wrapping_sub((*window).lowLimit) < HASH_READ_SIZE as U32 {
            (*window).lowLimit = (*window).dictLimit;
        }
        contiguous = 0;
    }
    (*window).nextSrc = ip.add(srcSize);
    if ((ip as usize).wrapping_add(srcSize)
        > ((*window).dictBase as usize).wrapping_add((*window).lowLimit as usize))
        && ((ip as usize)
            < ((*window).dictBase as usize).wrapping_add((*window).dictLimit as usize))
    {
        let highInputIdx =
            ((ip as usize).wrapping_add(srcSize)).wrapping_sub((*window).dictBase as usize);
        let lowLimitMax: U32 = if highInputIdx > (*window).dictLimit as usize {
            (*window).dictLimit
        } else {
            highInputIdx as U32
        };
        (*window).lowLimit = lowLimitMax;
    }
    contiguous
}

#[inline(always)]
pub unsafe fn ZSTD_getLowestMatchIndex(
    ms: *const ZSTD_MatchState_t,
    curr: U32,
    windowLog: c_uint,
) -> U32 {
    let maxDistance: U32 = 1u32 << windowLog;
    let lowestValid: U32 = (*ms).window.lowLimit;
    let withinWindow: U32 = if curr.wrapping_sub(lowestValid) > maxDistance {
        curr.wrapping_sub(maxDistance)
    } else {
        lowestValid
    };
    let isDictionary: U32 = ((*ms).loadedDictEnd != 0) as U32;
    if isDictionary != 0 {
        lowestValid
    } else {
        withinWindow
    }
}

#[inline(always)]
pub unsafe fn ZSTD_getLowestPrefixIndex(
    ms: *const ZSTD_MatchState_t,
    curr: U32,
    windowLog: c_uint,
) -> U32 {
    let maxDistance: U32 = 1u32 << windowLog;
    let lowestValid: U32 = (*ms).window.dictLimit;
    let withinWindow: U32 = if curr.wrapping_sub(lowestValid) > maxDistance {
        curr.wrapping_sub(maxDistance)
    } else {
        lowestValid
    };
    let isDictionary: U32 = ((*ms).loadedDictEnd != 0) as U32;
    if isDictionary != 0 {
        lowestValid
    } else {
        withinWindow
    }
}

#[inline(always)]
pub fn ZSTD_index_overlap_check(prefixLowestIndex: U32, repIndex: U32) -> c_int {
    ((prefixLowestIndex.wrapping_sub(1).wrapping_sub(repIndex)) >= 3) as c_int
}

/* ===== Short Cache ===== */

pub const ZSTD_SHORT_CACHE_TAG_BITS: u32 = 8;
pub const ZSTD_SHORT_CACHE_TAG_MASK: usize = (1usize << ZSTD_SHORT_CACHE_TAG_BITS) - 1;

#[inline(always)]
pub unsafe fn ZSTD_writeTaggedIndex(hashTable: *mut U32, hashAndTag: usize, index: U32) {
    let hash = hashAndTag >> ZSTD_SHORT_CACHE_TAG_BITS;
    let tag = (hashAndTag & ZSTD_SHORT_CACHE_TAG_MASK) as U32;
    *hashTable.add(hash) = (index << ZSTD_SHORT_CACHE_TAG_BITS) | tag;
}

#[inline(always)]
pub fn ZSTD_comparePackedTags(packedTag1: usize, packedTag2: usize) -> c_int {
    let tag1 = (packedTag1 & ZSTD_SHORT_CACHE_TAG_MASK) as U32;
    let tag2 = (packedTag2 & ZSTD_SHORT_CACHE_TAG_MASK) as U32;
    (tag1 == tag2) as c_int
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct ZSTD_SequencePosition {
    pub idx: U32,
    pub posInSequence: U32,
    pub posInSrc: usize,
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct BlockSummary {
    pub nbSequences: usize,
    pub blockSize: usize,
    pub litSize: usize,
}

#[inline(always)]
pub unsafe fn ZSTD_hasExtSeqProd(params: *const ZSTD_CCtx_params) -> c_int {
    (*params).extSeqProdFunc.is_some() as c_int
}
