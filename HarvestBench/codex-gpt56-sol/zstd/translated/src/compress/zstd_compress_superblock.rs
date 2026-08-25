use ::libc;
extern "C" {
    pub type ZSTD_CDict_s;
    pub type POOL_ctx_s;
    fn HIST_count_wksp(
        count: *mut ::core::ffi::c_uint,
        maxSymbolValuePtr: *mut ::core::ffi::c_uint,
        src: *const ::core::ffi::c_void,
        srcSize: size_t,
        workSpace: *mut ::core::ffi::c_void,
        workSpaceSize: size_t,
    ) -> size_t;
    fn HIST_countFast_wksp(
        count: *mut ::core::ffi::c_uint,
        maxSymbolValuePtr: *mut ::core::ffi::c_uint,
        src: *const ::core::ffi::c_void,
        srcSize: size_t,
        workSpace: *mut ::core::ffi::c_void,
        workSpaceSize: size_t,
    ) -> size_t;
    fn HUF_compress4X_usingCTable(
        dst: *mut ::core::ffi::c_void,
        dstSize: size_t,
        src: *const ::core::ffi::c_void,
        srcSize: size_t,
        CTable: *const HUF_CElt,
        flags: ::core::ffi::c_int,
    ) -> size_t;
    fn HUF_estimateCompressedSize(
        CTable: *const HUF_CElt,
        count: *const ::core::ffi::c_uint,
        maxSymbolValue: ::core::ffi::c_uint,
    ) -> size_t;
    fn HUF_compress1X_usingCTable(
        dst: *mut ::core::ffi::c_void,
        dstSize: size_t,
        src: *const ::core::ffi::c_void,
        srcSize: size_t,
        CTable: *const HUF_CElt,
        flags: ::core::ffi::c_int,
    ) -> size_t;
    fn ZSTD_encodeSequences(
        dst: *mut ::core::ffi::c_void,
        dstCapacity: size_t,
        CTable_MatchLength: *const FSE_CTable,
        mlCodeTable: *const BYTE,
        CTable_OffsetBits: *const FSE_CTable,
        ofCodeTable: *const BYTE,
        CTable_LitLength: *const FSE_CTable,
        llCodeTable: *const BYTE,
        sequences: *const SeqDef,
        nbSeq: size_t,
        longOffsets: ::core::ffi::c_int,
        bmi2: ::core::ffi::c_int,
    ) -> size_t;
    fn ZSTD_fseBitCost(
        ctable: *const FSE_CTable,
        count: *const ::core::ffi::c_uint,
        max: ::core::ffi::c_uint,
    ) -> size_t;
    fn ZSTD_crossEntropyCost(
        norm: *const ::core::ffi::c_short,
        accuracyLog: ::core::ffi::c_uint,
        count: *const ::core::ffi::c_uint,
        max: ::core::ffi::c_uint,
    ) -> size_t;
    fn ZSTD_buildBlockEntropyStats(
        seqStorePtr: *const SeqStore_t,
        prevEntropy: *const ZSTD_entropyCTables_t,
        nextEntropy: *mut ZSTD_entropyCTables_t,
        cctxParams: *const ZSTD_CCtx_params,
        entropyMetadata: *mut ZSTD_entropyCTablesMetadata_t,
        workspace: *mut ::core::ffi::c_void,
        wkspSize: size_t,
    ) -> size_t;
    fn ZSTD_noCompressLiterals(
        dst: *mut ::core::ffi::c_void,
        dstCapacity: size_t,
        src: *const ::core::ffi::c_void,
        srcSize: size_t,
    ) -> size_t;
    fn ZSTD_compressRleLiteralsBlock(
        dst: *mut ::core::ffi::c_void,
        dstCapacity: size_t,
        src: *const ::core::ffi::c_void,
        srcSize: size_t,
    ) -> size_t;
}
pub type size_t = usize;
pub type C2RustUnnamed = ::core::ffi::c_uint;
pub const ZSTD_error_maxCode: C2RustUnnamed = 120;
pub const ZSTD_error_externalSequences_invalid: C2RustUnnamed = 107;
pub const ZSTD_error_sequenceProducer_failed: C2RustUnnamed = 106;
pub const ZSTD_error_srcBuffer_wrong: C2RustUnnamed = 105;
pub const ZSTD_error_dstBuffer_wrong: C2RustUnnamed = 104;
pub const ZSTD_error_seekableIO: C2RustUnnamed = 102;
pub const ZSTD_error_frameIndex_tooLarge: C2RustUnnamed = 100;
pub const ZSTD_error_noForwardProgress_inputEmpty: C2RustUnnamed = 82;
pub const ZSTD_error_noForwardProgress_destFull: C2RustUnnamed = 80;
pub const ZSTD_error_dstBuffer_null: C2RustUnnamed = 74;
pub const ZSTD_error_srcSize_wrong: C2RustUnnamed = 72;
pub const ZSTD_error_dstSize_tooSmall: C2RustUnnamed = 70;
pub const ZSTD_error_workSpace_tooSmall: C2RustUnnamed = 66;
pub const ZSTD_error_memory_allocation: C2RustUnnamed = 64;
pub const ZSTD_error_init_missing: C2RustUnnamed = 62;
pub const ZSTD_error_stage_wrong: C2RustUnnamed = 60;
pub const ZSTD_error_stabilityCondition_notRespected: C2RustUnnamed = 50;
pub const ZSTD_error_cannotProduce_uncompressedBlock: C2RustUnnamed = 49;
pub const ZSTD_error_maxSymbolValue_tooSmall: C2RustUnnamed = 48;
pub const ZSTD_error_maxSymbolValue_tooLarge: C2RustUnnamed = 46;
pub const ZSTD_error_tableLog_tooLarge: C2RustUnnamed = 44;
pub const ZSTD_error_parameter_outOfBound: C2RustUnnamed = 42;
pub const ZSTD_error_parameter_combination_unsupported: C2RustUnnamed = 41;
pub const ZSTD_error_parameter_unsupported: C2RustUnnamed = 40;
pub const ZSTD_error_dictionaryCreation_failed: C2RustUnnamed = 34;
pub const ZSTD_error_dictionary_wrong: C2RustUnnamed = 32;
pub const ZSTD_error_dictionary_corrupted: C2RustUnnamed = 30;
pub const ZSTD_error_literals_headerWrong: C2RustUnnamed = 24;
pub const ZSTD_error_checksum_wrong: C2RustUnnamed = 22;
pub const ZSTD_error_corruption_detected: C2RustUnnamed = 20;
pub const ZSTD_error_frameParameter_windowTooLarge: C2RustUnnamed = 16;
pub const ZSTD_error_frameParameter_unsupported: C2RustUnnamed = 14;
pub const ZSTD_error_version_unsupported: C2RustUnnamed = 12;
pub const ZSTD_error_prefix_unknown: C2RustUnnamed = 10;
pub const ZSTD_error_GENERIC: C2RustUnnamed = 1;
pub const ZSTD_error_no_error: C2RustUnnamed = 0;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ZSTD_CCtx_s {
    pub stage: ZSTD_compressionStage_e,
    pub cParamsChanged: ::core::ffi::c_int,
    pub bmi2: ::core::ffi::c_int,
    pub requestedParams: ZSTD_CCtx_params,
    pub appliedParams: ZSTD_CCtx_params,
    pub simpleApiParams: ZSTD_CCtx_params,
    pub dictID: U32,
    pub dictContentSize: size_t,
    pub workspace: ZSTD_cwksp,
    pub blockSizeMax: size_t,
    pub pledgedSrcSizePlusOne: ::core::ffi::c_ulonglong,
    pub consumedSrcSize: ::core::ffi::c_ulonglong,
    pub producedCSize: ::core::ffi::c_ulonglong,
    pub xxhState: XXH64_state_t,
    pub customMem: ZSTD_customMem,
    pub pool: *mut ZSTD_threadPool,
    pub staticSize: size_t,
    pub seqCollector: SeqCollector,
    pub isFirstBlock: ::core::ffi::c_int,
    pub initialized: ::core::ffi::c_int,
    pub seqStore: SeqStore_t,
    pub ldmState: ldmState_t,
    pub ldmSequences: *mut rawSeq,
    pub maxNbLdmSequences: size_t,
    pub externSeqStore: RawSeqStore_t,
    pub blockState: ZSTD_blockState_t,
    pub tmpWorkspace: *mut ::core::ffi::c_void,
    pub tmpWkspSize: size_t,
    pub bufferedPolicy: ZSTD_buffered_policy_e,
    pub inBuff: *mut ::core::ffi::c_char,
    pub inBuffSize: size_t,
    pub inToCompress: size_t,
    pub inBuffPos: size_t,
    pub inBuffTarget: size_t,
    pub outBuff: *mut ::core::ffi::c_char,
    pub outBuffSize: size_t,
    pub outBuffContentSize: size_t,
    pub outBuffFlushedSize: size_t,
    pub streamStage: ZSTD_cStreamStage,
    pub frameEnded: U32,
    pub expectedInBuffer: ZSTD_inBuffer,
    pub stableIn_notConsumed: size_t,
    pub expectedOutBufferSize: size_t,
    pub localDict: ZSTD_localDict,
    pub cdict: *const ZSTD_CDict,
    pub prefixDict: ZSTD_prefixDict,
    pub traceCtx: ZSTD_TraceCtx,
    pub blockSplitCtx: ZSTD_blockSplitCtx,
    pub extSeqBuf: *mut ZSTD_Sequence,
    pub extSeqBufCapacity: size_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ZSTD_Sequence {
    pub offset: ::core::ffi::c_uint,
    pub litLength: ::core::ffi::c_uint,
    pub matchLength: ::core::ffi::c_uint,
    pub rep: ::core::ffi::c_uint,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ZSTD_blockSplitCtx {
    pub fullSeqStoreChunk: SeqStore_t,
    pub firstHalfSeqStore: SeqStore_t,
    pub secondHalfSeqStore: SeqStore_t,
    pub currSeqStore: SeqStore_t,
    pub nextSeqStore: SeqStore_t,
    pub partitions: [U32; 196],
    pub entropyMetadata: ZSTD_entropyCTablesMetadata_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ZSTD_entropyCTablesMetadata_t {
    pub hufMetadata: ZSTD_hufCTablesMetadata_t,
    pub fseMetadata: ZSTD_fseCTablesMetadata_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ZSTD_fseCTablesMetadata_t {
    pub llType: SymbolEncodingType_e,
    pub ofType: SymbolEncodingType_e,
    pub mlType: SymbolEncodingType_e,
    pub fseTablesBuffer: [BYTE; 133],
    pub fseTablesSize: size_t,
    pub lastCountSize: size_t,
}
pub type BYTE = uint8_t;
pub type uint8_t = __uint8_t;
pub type __uint8_t = u8;
pub type SymbolEncodingType_e = ::core::ffi::c_uint;
pub const set_repeat: SymbolEncodingType_e = 3;
pub const set_compressed: SymbolEncodingType_e = 2;
pub const set_rle: SymbolEncodingType_e = 1;
pub const set_basic: SymbolEncodingType_e = 0;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ZSTD_hufCTablesMetadata_t {
    pub hType: SymbolEncodingType_e,
    pub hufDesBuffer: [BYTE; 128],
    pub hufDesSize: size_t,
}
pub type U32 = uint32_t;
pub type uint32_t = __uint32_t;
pub type __uint32_t = u32;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct SeqStore_t {
    pub sequencesStart: *mut SeqDef,
    pub sequences: *mut SeqDef,
    pub litStart: *mut BYTE,
    pub lit: *mut BYTE,
    pub llCode: *mut BYTE,
    pub mlCode: *mut BYTE,
    pub ofCode: *mut BYTE,
    pub maxNbSeq: size_t,
    pub maxNbLit: size_t,
    pub longLengthType: ZSTD_longLengthType_e,
    pub longLengthPos: U32,
}
pub type ZSTD_longLengthType_e = ::core::ffi::c_uint;
pub const ZSTD_llt_matchLength: ZSTD_longLengthType_e = 2;
pub const ZSTD_llt_literalLength: ZSTD_longLengthType_e = 1;
pub const ZSTD_llt_none: ZSTD_longLengthType_e = 0;
pub type SeqDef = SeqDef_s;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct SeqDef_s {
    pub offBase: U32,
    pub litLength: U16,
    pub mlBase: U16,
}
pub type U16 = uint16_t;
pub type uint16_t = __uint16_t;
pub type __uint16_t = u16;
pub type ZSTD_TraceCtx = ::core::ffi::c_ulonglong;
pub type ZSTD_prefixDict = ZSTD_prefixDict_s;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ZSTD_prefixDict_s {
    pub dict: *const ::core::ffi::c_void,
    pub dictSize: size_t,
    pub dictContentType: ZSTD_dictContentType_e,
}
pub type ZSTD_dictContentType_e = ::core::ffi::c_uint;
pub const ZSTD_dct_fullDict: ZSTD_dictContentType_e = 2;
pub const ZSTD_dct_rawContent: ZSTD_dictContentType_e = 1;
pub const ZSTD_dct_auto: ZSTD_dictContentType_e = 0;
pub type ZSTD_CDict = ZSTD_CDict_s;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ZSTD_localDict {
    pub dictBuffer: *mut ::core::ffi::c_void,
    pub dict: *const ::core::ffi::c_void,
    pub dictSize: size_t,
    pub dictContentType: ZSTD_dictContentType_e,
    pub cdict: *mut ZSTD_CDict,
}
pub type ZSTD_inBuffer = ZSTD_inBuffer_s;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ZSTD_inBuffer_s {
    pub src: *const ::core::ffi::c_void,
    pub size: size_t,
    pub pos: size_t,
}
pub type ZSTD_cStreamStage = ::core::ffi::c_uint;
pub const zcss_flush: ZSTD_cStreamStage = 2;
pub const zcss_load: ZSTD_cStreamStage = 1;
pub const zcss_init: ZSTD_cStreamStage = 0;
pub type ZSTD_buffered_policy_e = ::core::ffi::c_uint;
pub const ZSTDb_buffered: ZSTD_buffered_policy_e = 1;
pub const ZSTDb_not_buffered: ZSTD_buffered_policy_e = 0;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ZSTD_blockState_t {
    pub prevCBlock: *mut ZSTD_compressedBlockState_t,
    pub nextCBlock: *mut ZSTD_compressedBlockState_t,
    pub matchState: ZSTD_MatchState_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ZSTD_MatchState_t {
    pub window: ZSTD_window_t,
    pub loadedDictEnd: U32,
    pub nextToUpdate: U32,
    pub hashLog3: U32,
    pub rowHashLog: U32,
    pub tagTable: *mut BYTE,
    pub hashCache: [U32; 8],
    pub hashSalt: U64,
    pub hashSaltEntropy: U32,
    pub hashTable: *mut U32,
    pub hashTable3: *mut U32,
    pub chainTable: *mut U32,
    pub forceNonContiguous: ::core::ffi::c_int,
    pub dedicatedDictSearch: ::core::ffi::c_int,
    pub opt: optState_t,
    pub dictMatchState: *const ZSTD_MatchState_t,
    pub cParams: ZSTD_compressionParameters,
    pub ldmSeqStore: *const RawSeqStore_t,
    pub prefetchCDictTables: ::core::ffi::c_int,
    pub lazySkipping: ::core::ffi::c_int,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct RawSeqStore_t {
    pub seq: *mut rawSeq,
    pub pos: size_t,
    pub posInSequence: size_t,
    pub size: size_t,
    pub capacity: size_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct rawSeq {
    pub offset: U32,
    pub litLength: U32,
    pub matchLength: U32,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ZSTD_compressionParameters {
    pub windowLog: ::core::ffi::c_uint,
    pub chainLog: ::core::ffi::c_uint,
    pub hashLog: ::core::ffi::c_uint,
    pub searchLog: ::core::ffi::c_uint,
    pub minMatch: ::core::ffi::c_uint,
    pub targetLength: ::core::ffi::c_uint,
    pub strategy: ZSTD_strategy,
}
pub type ZSTD_strategy = ::core::ffi::c_uint;
pub const ZSTD_btultra2: ZSTD_strategy = 9;
pub const ZSTD_btultra: ZSTD_strategy = 8;
pub const ZSTD_btopt: ZSTD_strategy = 7;
pub const ZSTD_btlazy2: ZSTD_strategy = 6;
pub const ZSTD_lazy2: ZSTD_strategy = 5;
pub const ZSTD_lazy: ZSTD_strategy = 4;
pub const ZSTD_greedy: ZSTD_strategy = 3;
pub const ZSTD_dfast: ZSTD_strategy = 2;
pub const ZSTD_fast: ZSTD_strategy = 1;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct optState_t {
    pub litFreq: *mut ::core::ffi::c_uint,
    pub litLengthFreq: *mut ::core::ffi::c_uint,
    pub matchLengthFreq: *mut ::core::ffi::c_uint,
    pub offCodeFreq: *mut ::core::ffi::c_uint,
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
pub type ZSTD_ParamSwitch_e = ::core::ffi::c_uint;
pub const ZSTD_ps_disable: ZSTD_ParamSwitch_e = 2;
pub const ZSTD_ps_enable: ZSTD_ParamSwitch_e = 1;
pub const ZSTD_ps_auto: ZSTD_ParamSwitch_e = 0;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ZSTD_entropyCTables_t {
    pub huf: ZSTD_hufCTables_t,
    pub fse: ZSTD_fseCTables_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ZSTD_fseCTables_t {
    pub offcodeCTable: [FSE_CTable; 193],
    pub matchlengthCTable: [FSE_CTable; 363],
    pub litlengthCTable: [FSE_CTable; 329],
    pub offcode_repeatMode: FSE_repeat,
    pub matchlength_repeatMode: FSE_repeat,
    pub litlength_repeatMode: FSE_repeat,
}
pub type FSE_repeat = ::core::ffi::c_uint;
pub const FSE_repeat_valid: FSE_repeat = 2;
pub const FSE_repeat_check: FSE_repeat = 1;
pub const FSE_repeat_none: FSE_repeat = 0;
pub type FSE_CTable = ::core::ffi::c_uint;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ZSTD_hufCTables_t {
    pub CTable: [HUF_CElt; 257],
    pub repeatMode: HUF_repeat,
}
pub type HUF_repeat = ::core::ffi::c_uint;
pub const HUF_repeat_valid: HUF_repeat = 2;
pub const HUF_repeat_check: HUF_repeat = 1;
pub const HUF_repeat_none: HUF_repeat = 0;
pub type HUF_CElt = size_t;
pub type ZSTD_OptPrice_e = ::core::ffi::c_uint;
pub const zop_predef: ZSTD_OptPrice_e = 1;
pub const zop_dynamic: ZSTD_OptPrice_e = 0;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ZSTD_optimal_t {
    pub price: ::core::ffi::c_int,
    pub off: U32,
    pub mlen: U32,
    pub litlen: U32,
    pub rep: [U32; 3],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ZSTD_match_t {
    pub off: U32,
    pub len: U32,
}
pub type U64 = uint64_t;
pub type uint64_t = __uint64_t;
pub type __uint64_t = u64;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ZSTD_window_t {
    pub nextSrc: *const BYTE,
    pub base: *const BYTE,
    pub dictBase: *const BYTE,
    pub dictLimit: U32,
    pub lowLimit: U32,
    pub nbOverflowCorrections: U32,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ZSTD_compressedBlockState_t {
    pub entropy: ZSTD_entropyCTables_t,
    pub rep: [U32; 3],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ldmState_t {
    pub window: ZSTD_window_t,
    pub hashTable: *mut ldmEntry_t,
    pub loadedDictEnd: U32,
    pub bucketOffsets: *mut BYTE,
    pub splitIndices: [size_t; 64],
    pub matchCandidates: [ldmMatchCandidate_t; 64],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ldmMatchCandidate_t {
    pub split: *const BYTE,
    pub hash: U32,
    pub checksum: U32,
    pub bucket: *mut ldmEntry_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ldmEntry_t {
    pub offset: U32,
    pub checksum: U32,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct SeqCollector {
    pub collectSequences: ::core::ffi::c_int,
    pub seqStart: *mut ZSTD_Sequence,
    pub seqIndex: size_t,
    pub maxSequences: size_t,
}
pub type ZSTD_threadPool = POOL_ctx_s;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ZSTD_customMem {
    pub customAlloc: ZSTD_allocFunction,
    pub customFree: ZSTD_freeFunction,
    pub opaque: *mut ::core::ffi::c_void,
}
pub type ZSTD_freeFunction =
    Option<unsafe extern "C" fn(*mut ::core::ffi::c_void, *mut ::core::ffi::c_void) -> ()>;
pub type ZSTD_allocFunction =
    Option<unsafe extern "C" fn(*mut ::core::ffi::c_void, size_t) -> *mut ::core::ffi::c_void>;
pub type XXH64_state_t = XXH64_state_s;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct XXH64_state_s {
    pub total_len: XXH64_hash_t,
    pub v: [XXH64_hash_t; 4],
    pub mem64: [XXH64_hash_t; 4],
    pub memsize: XXH32_hash_t,
    pub reserved32: XXH32_hash_t,
    pub reserved64: XXH64_hash_t,
}
pub type XXH64_hash_t = uint64_t;
pub type XXH32_hash_t = uint32_t;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ZSTD_cwksp {
    pub workspace: *mut ::core::ffi::c_void,
    pub workspaceEnd: *mut ::core::ffi::c_void,
    pub objectEnd: *mut ::core::ffi::c_void,
    pub tableEnd: *mut ::core::ffi::c_void,
    pub tableValidEnd: *mut ::core::ffi::c_void,
    pub allocStart: *mut ::core::ffi::c_void,
    pub initOnceStart: *mut ::core::ffi::c_void,
    pub allocFailed: BYTE,
    pub workspaceOversizedDuration: ::core::ffi::c_int,
    pub phase: ZSTD_cwksp_alloc_phase_e,
    pub isStatic: ZSTD_cwksp_static_alloc_e,
}
pub type ZSTD_cwksp_static_alloc_e = ::core::ffi::c_uint;
pub const ZSTD_cwksp_static_alloc: ZSTD_cwksp_static_alloc_e = 1;
pub const ZSTD_cwksp_dynamic_alloc: ZSTD_cwksp_static_alloc_e = 0;
pub type ZSTD_cwksp_alloc_phase_e = ::core::ffi::c_uint;
pub const ZSTD_cwksp_alloc_buffers: ZSTD_cwksp_alloc_phase_e = 3;
pub const ZSTD_cwksp_alloc_aligned: ZSTD_cwksp_alloc_phase_e = 2;
pub const ZSTD_cwksp_alloc_aligned_init_once: ZSTD_cwksp_alloc_phase_e = 1;
pub const ZSTD_cwksp_alloc_objects: ZSTD_cwksp_alloc_phase_e = 0;
pub type ZSTD_CCtx_params = ZSTD_CCtx_params_s;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ZSTD_CCtx_params_s {
    pub format: ZSTD_format_e,
    pub cParams: ZSTD_compressionParameters,
    pub fParams: ZSTD_frameParameters,
    pub compressionLevel: ::core::ffi::c_int,
    pub forceWindow: ::core::ffi::c_int,
    pub targetCBlockSize: size_t,
    pub srcSizeHint: ::core::ffi::c_int,
    pub attachDictPref: ZSTD_dictAttachPref_e,
    pub literalCompressionMode: ZSTD_ParamSwitch_e,
    pub nbWorkers: ::core::ffi::c_int,
    pub jobSize: size_t,
    pub overlapLog: ::core::ffi::c_int,
    pub rsyncable: ::core::ffi::c_int,
    pub ldmParams: ldmParams_t,
    pub enableDedicatedDictSearch: ::core::ffi::c_int,
    pub inBufferMode: ZSTD_bufferMode_e,
    pub outBufferMode: ZSTD_bufferMode_e,
    pub blockDelimiters: ZSTD_SequenceFormat_e,
    pub validateSequences: ::core::ffi::c_int,
    pub postBlockSplitter: ZSTD_ParamSwitch_e,
    pub preBlockSplitter_level: ::core::ffi::c_int,
    pub maxBlockSize: size_t,
    pub useRowMatchFinder: ZSTD_ParamSwitch_e,
    pub deterministicRefPrefix: ::core::ffi::c_int,
    pub customMem: ZSTD_customMem,
    pub prefetchCDictTables: ZSTD_ParamSwitch_e,
    pub enableMatchFinderFallback: ::core::ffi::c_int,
    pub extSeqProdState: *mut ::core::ffi::c_void,
    pub extSeqProdFunc: ZSTD_sequenceProducer_F,
    pub searchForExternalRepcodes: ZSTD_ParamSwitch_e,
}
pub type ZSTD_sequenceProducer_F = Option<
    unsafe extern "C" fn(
        *mut ::core::ffi::c_void,
        *mut ZSTD_Sequence,
        size_t,
        *const ::core::ffi::c_void,
        size_t,
        *const ::core::ffi::c_void,
        size_t,
        ::core::ffi::c_int,
        size_t,
    ) -> size_t,
>;
pub type ZSTD_SequenceFormat_e = ::core::ffi::c_uint;
pub const ZSTD_sf_explicitBlockDelimiters: ZSTD_SequenceFormat_e = 1;
pub const ZSTD_sf_noBlockDelimiters: ZSTD_SequenceFormat_e = 0;
pub type ZSTD_bufferMode_e = ::core::ffi::c_uint;
pub const ZSTD_bm_stable: ZSTD_bufferMode_e = 1;
pub const ZSTD_bm_buffered: ZSTD_bufferMode_e = 0;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ldmParams_t {
    pub enableLdm: ZSTD_ParamSwitch_e,
    pub hashLog: U32,
    pub bucketSizeLog: U32,
    pub minMatchLength: U32,
    pub hashRateLog: U32,
    pub windowLog: U32,
}
pub type ZSTD_dictAttachPref_e = ::core::ffi::c_uint;
pub const ZSTD_dictForceLoad: ZSTD_dictAttachPref_e = 3;
pub const ZSTD_dictForceCopy: ZSTD_dictAttachPref_e = 2;
pub const ZSTD_dictForceAttach: ZSTD_dictAttachPref_e = 1;
pub const ZSTD_dictDefaultAttach: ZSTD_dictAttachPref_e = 0;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ZSTD_frameParameters {
    pub contentSizeFlag: ::core::ffi::c_int,
    pub checksumFlag: ::core::ffi::c_int,
    pub noDictIDFlag: ::core::ffi::c_int,
}
pub type ZSTD_format_e = ::core::ffi::c_uint;
pub const ZSTD_f_zstd1_magicless: ZSTD_format_e = 1;
pub const ZSTD_f_zstd1: ZSTD_format_e = 0;
pub type ZSTD_compressionStage_e = ::core::ffi::c_uint;
pub const ZSTDcs_ending: ZSTD_compressionStage_e = 3;
pub const ZSTDcs_ongoing: ZSTD_compressionStage_e = 2;
pub const ZSTDcs_init: ZSTD_compressionStage_e = 1;
pub const ZSTDcs_created: ZSTD_compressionStage_e = 0;
pub type ZSTD_CCtx = ZSTD_CCtx_s;
pub type Repcodes_t = repcodes_s;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct repcodes_s {
    pub rep: [U32; 3],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ZSTD_SequenceLength {
    pub litLength: U32,
    pub matchLength: U32,
}
pub const bt_raw: C2RustUnnamed_1 = 0;
pub type unalign16 = U16;
pub const bt_compressed: C2RustUnnamed_1 = 2;
pub type unalign32 = U32;
pub const HUF_flags_bmi2: C2RustUnnamed_0 = 1;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct EstimatedBlockSize {
    pub estLitSize: size_t,
    pub estBlockSize: size_t,
}
pub type S16 = int16_t;
pub type int16_t = __int16_t;
pub type __int16_t = i16;
pub type U8 = uint8_t;
pub type C2RustUnnamed_0 = ::core::ffi::c_uint;
pub const HUF_flags_disableFast: C2RustUnnamed_0 = 32;
pub const HUF_flags_disableAsm: C2RustUnnamed_0 = 16;
pub const HUF_flags_suspectUncompressible: C2RustUnnamed_0 = 8;
pub const HUF_flags_preferRepeat: C2RustUnnamed_0 = 4;
pub const HUF_flags_optimalDepth: C2RustUnnamed_0 = 2;
pub type C2RustUnnamed_1 = ::core::ffi::c_uint;
pub const bt_reserved: C2RustUnnamed_1 = 3;
pub const bt_rle: C2RustUnnamed_1 = 1;
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
pub const ZSTD_TARGETCBLOCKSIZE_MIN: ::core::ffi::c_int = 1340 as ::core::ffi::c_int;
pub const ZSTD_REP_NUM: ::core::ffi::c_int = 3 as ::core::ffi::c_int;
pub const ZSTD_BLOCKHEADERSIZE: ::core::ffi::c_int = 3 as ::core::ffi::c_int;
static mut ZSTD_blockHeaderSize: size_t = ZSTD_BLOCKHEADERSIZE as size_t;
pub const LONGNBSEQ: ::core::ffi::c_int = 0x7f00 as ::core::ffi::c_int;
pub const MINMATCH: ::core::ffi::c_int = 3 as ::core::ffi::c_int;
pub const MaxML: ::core::ffi::c_int = 52 as ::core::ffi::c_int;
pub const MaxLL: ::core::ffi::c_int = 35 as ::core::ffi::c_int;
pub const DefaultMaxOff: ::core::ffi::c_int = 28 as ::core::ffi::c_int;
pub const MaxOff: ::core::ffi::c_int = 31 as ::core::ffi::c_int;
static mut LL_bits: [U8; 36] = [
    0 as ::core::ffi::c_int as U8,
    0 as ::core::ffi::c_int as U8,
    0 as ::core::ffi::c_int as U8,
    0 as ::core::ffi::c_int as U8,
    0 as ::core::ffi::c_int as U8,
    0 as ::core::ffi::c_int as U8,
    0 as ::core::ffi::c_int as U8,
    0 as ::core::ffi::c_int as U8,
    0 as ::core::ffi::c_int as U8,
    0 as ::core::ffi::c_int as U8,
    0 as ::core::ffi::c_int as U8,
    0 as ::core::ffi::c_int as U8,
    0 as ::core::ffi::c_int as U8,
    0 as ::core::ffi::c_int as U8,
    0 as ::core::ffi::c_int as U8,
    0 as ::core::ffi::c_int as U8,
    1 as ::core::ffi::c_int as U8,
    1 as ::core::ffi::c_int as U8,
    1 as ::core::ffi::c_int as U8,
    1 as ::core::ffi::c_int as U8,
    2 as ::core::ffi::c_int as U8,
    2 as ::core::ffi::c_int as U8,
    3 as ::core::ffi::c_int as U8,
    3 as ::core::ffi::c_int as U8,
    4 as ::core::ffi::c_int as U8,
    6 as ::core::ffi::c_int as U8,
    7 as ::core::ffi::c_int as U8,
    8 as ::core::ffi::c_int as U8,
    9 as ::core::ffi::c_int as U8,
    10 as ::core::ffi::c_int as U8,
    11 as ::core::ffi::c_int as U8,
    12 as ::core::ffi::c_int as U8,
    13 as ::core::ffi::c_int as U8,
    14 as ::core::ffi::c_int as U8,
    15 as ::core::ffi::c_int as U8,
    16 as ::core::ffi::c_int as U8,
];
static mut LL_defaultNorm: [S16; 36] = [
    4 as ::core::ffi::c_int as S16,
    3 as ::core::ffi::c_int as S16,
    2 as ::core::ffi::c_int as S16,
    2 as ::core::ffi::c_int as S16,
    2 as ::core::ffi::c_int as S16,
    2 as ::core::ffi::c_int as S16,
    2 as ::core::ffi::c_int as S16,
    2 as ::core::ffi::c_int as S16,
    2 as ::core::ffi::c_int as S16,
    2 as ::core::ffi::c_int as S16,
    2 as ::core::ffi::c_int as S16,
    2 as ::core::ffi::c_int as S16,
    2 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    2 as ::core::ffi::c_int as S16,
    2 as ::core::ffi::c_int as S16,
    2 as ::core::ffi::c_int as S16,
    2 as ::core::ffi::c_int as S16,
    2 as ::core::ffi::c_int as S16,
    2 as ::core::ffi::c_int as S16,
    2 as ::core::ffi::c_int as S16,
    2 as ::core::ffi::c_int as S16,
    2 as ::core::ffi::c_int as S16,
    3 as ::core::ffi::c_int as S16,
    2 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    -(1 as ::core::ffi::c_int) as S16,
    -(1 as ::core::ffi::c_int) as S16,
    -(1 as ::core::ffi::c_int) as S16,
    -(1 as ::core::ffi::c_int) as S16,
];
pub const LL_DEFAULTNORMLOG: ::core::ffi::c_int = 6 as ::core::ffi::c_int;
static mut LL_defaultNormLog: U32 = LL_DEFAULTNORMLOG as U32;
static mut ML_bits: [U8; 53] = [
    0 as ::core::ffi::c_int as U8,
    0 as ::core::ffi::c_int as U8,
    0 as ::core::ffi::c_int as U8,
    0 as ::core::ffi::c_int as U8,
    0 as ::core::ffi::c_int as U8,
    0 as ::core::ffi::c_int as U8,
    0 as ::core::ffi::c_int as U8,
    0 as ::core::ffi::c_int as U8,
    0 as ::core::ffi::c_int as U8,
    0 as ::core::ffi::c_int as U8,
    0 as ::core::ffi::c_int as U8,
    0 as ::core::ffi::c_int as U8,
    0 as ::core::ffi::c_int as U8,
    0 as ::core::ffi::c_int as U8,
    0 as ::core::ffi::c_int as U8,
    0 as ::core::ffi::c_int as U8,
    0 as ::core::ffi::c_int as U8,
    0 as ::core::ffi::c_int as U8,
    0 as ::core::ffi::c_int as U8,
    0 as ::core::ffi::c_int as U8,
    0 as ::core::ffi::c_int as U8,
    0 as ::core::ffi::c_int as U8,
    0 as ::core::ffi::c_int as U8,
    0 as ::core::ffi::c_int as U8,
    0 as ::core::ffi::c_int as U8,
    0 as ::core::ffi::c_int as U8,
    0 as ::core::ffi::c_int as U8,
    0 as ::core::ffi::c_int as U8,
    0 as ::core::ffi::c_int as U8,
    0 as ::core::ffi::c_int as U8,
    0 as ::core::ffi::c_int as U8,
    0 as ::core::ffi::c_int as U8,
    1 as ::core::ffi::c_int as U8,
    1 as ::core::ffi::c_int as U8,
    1 as ::core::ffi::c_int as U8,
    1 as ::core::ffi::c_int as U8,
    2 as ::core::ffi::c_int as U8,
    2 as ::core::ffi::c_int as U8,
    3 as ::core::ffi::c_int as U8,
    3 as ::core::ffi::c_int as U8,
    4 as ::core::ffi::c_int as U8,
    4 as ::core::ffi::c_int as U8,
    5 as ::core::ffi::c_int as U8,
    7 as ::core::ffi::c_int as U8,
    8 as ::core::ffi::c_int as U8,
    9 as ::core::ffi::c_int as U8,
    10 as ::core::ffi::c_int as U8,
    11 as ::core::ffi::c_int as U8,
    12 as ::core::ffi::c_int as U8,
    13 as ::core::ffi::c_int as U8,
    14 as ::core::ffi::c_int as U8,
    15 as ::core::ffi::c_int as U8,
    16 as ::core::ffi::c_int as U8,
];
static mut ML_defaultNorm: [S16; 53] = [
    1 as ::core::ffi::c_int as S16,
    4 as ::core::ffi::c_int as S16,
    3 as ::core::ffi::c_int as S16,
    2 as ::core::ffi::c_int as S16,
    2 as ::core::ffi::c_int as S16,
    2 as ::core::ffi::c_int as S16,
    2 as ::core::ffi::c_int as S16,
    2 as ::core::ffi::c_int as S16,
    2 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    -(1 as ::core::ffi::c_int) as S16,
    -(1 as ::core::ffi::c_int) as S16,
    -(1 as ::core::ffi::c_int) as S16,
    -(1 as ::core::ffi::c_int) as S16,
    -(1 as ::core::ffi::c_int) as S16,
    -(1 as ::core::ffi::c_int) as S16,
    -(1 as ::core::ffi::c_int) as S16,
];
pub const ML_DEFAULTNORMLOG: ::core::ffi::c_int = 6 as ::core::ffi::c_int;
static mut ML_defaultNormLog: U32 = ML_DEFAULTNORMLOG as U32;
static mut OF_defaultNorm: [S16; 29] = [
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    2 as ::core::ffi::c_int as S16,
    2 as ::core::ffi::c_int as S16,
    2 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    1 as ::core::ffi::c_int as S16,
    -(1 as ::core::ffi::c_int) as S16,
    -(1 as ::core::ffi::c_int) as S16,
    -(1 as ::core::ffi::c_int) as S16,
    -(1 as ::core::ffi::c_int) as S16,
    -(1 as ::core::ffi::c_int) as S16,
];
pub const OF_DEFAULTNORMLOG: ::core::ffi::c_int = 5 as ::core::ffi::c_int;
static mut OF_defaultNormLog: U32 = OF_DEFAULTNORMLOG as U32;
unsafe extern "C" fn ERR_isError(mut code: size_t) -> ::core::ffi::c_uint {
    return (code > -(ZSTD_error_maxCode as ::core::ffi::c_int) as size_t) as ::core::ffi::c_int
        as ::core::ffi::c_uint;
}
#[inline]
unsafe extern "C" fn _force_has_format_string(
    mut format: *const ::core::ffi::c_char,
    mut args: ...
) {
}
#[inline]
unsafe extern "C" fn MEM_32bits() -> ::core::ffi::c_uint {
    return (::core::mem::size_of::<size_t>() as usize == 4 as usize) as ::core::ffi::c_int
        as ::core::ffi::c_uint;
}
#[inline]
unsafe extern "C" fn MEM_isLittleEndian() -> ::core::ffi::c_uint {
    return 1 as ::core::ffi::c_uint;
}
#[inline]
unsafe extern "C" fn MEM_write16(mut memPtr: *mut ::core::ffi::c_void, mut value: U16) {
    *(memPtr as *mut unalign16) = value as unalign16;
}
#[inline]
unsafe extern "C" fn MEM_write32(mut memPtr: *mut ::core::ffi::c_void, mut value: U32) {
    *(memPtr as *mut unalign32) = value as unalign32;
}
#[inline]
unsafe extern "C" fn MEM_swap32(mut in_0: U32) -> U32 {
    return in_0.swap_bytes();
}
#[inline]
unsafe extern "C" fn MEM_writeLE16(mut memPtr: *mut ::core::ffi::c_void, mut val: U16) {
    if MEM_isLittleEndian() != 0 {
        MEM_write16(memPtr, val);
    } else {
        let mut p: *mut BYTE = memPtr as *mut BYTE;
        *p.offset(0 as ::core::ffi::c_int as isize) = val as BYTE;
        *p.offset(1 as ::core::ffi::c_int as isize) =
            (val as ::core::ffi::c_int >> 8 as ::core::ffi::c_int) as BYTE;
    };
}
#[inline]
unsafe extern "C" fn MEM_writeLE24(mut memPtr: *mut ::core::ffi::c_void, mut val: U32) {
    MEM_writeLE16(memPtr, val as U16);
    *(memPtr as *mut BYTE).offset(2 as ::core::ffi::c_int as isize) =
        (val >> 16 as ::core::ffi::c_int) as BYTE;
}
#[inline]
unsafe extern "C" fn MEM_writeLE32(mut memPtr: *mut ::core::ffi::c_void, mut val32: U32) {
    if MEM_isLittleEndian() != 0 {
        MEM_write32(memPtr, val32);
    } else {
        MEM_write32(memPtr, MEM_swap32(val32));
    };
}
pub const STREAM_ACCUMULATOR_MIN_32: ::core::ffi::c_int = 25 as ::core::ffi::c_int;
pub const STREAM_ACCUMULATOR_MIN_64: ::core::ffi::c_int = 57 as ::core::ffi::c_int;
unsafe extern "C" fn ZSTD_compressSubBlock_literal(
    mut hufTable: *const HUF_CElt,
    mut hufMetadata: *const ZSTD_hufCTablesMetadata_t,
    mut literals: *const BYTE,
    mut litSize: size_t,
    mut dst: *mut ::core::ffi::c_void,
    mut dstSize: size_t,
    bmi2: ::core::ffi::c_int,
    mut writeEntropy: ::core::ffi::c_int,
    mut entropyWritten: *mut ::core::ffi::c_int,
) -> size_t {
    let header: size_t = (if writeEntropy != 0 {
        200 as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    }) as size_t;
    let lhSize: size_t =
        (3 as ::core::ffi::c_int
            + (litSize
                >= ((1 as ::core::ffi::c_int
                    * ((1 as ::core::ffi::c_int) << 10 as ::core::ffi::c_int))
                    as size_t)
                    .wrapping_sub(header)) as ::core::ffi::c_int
            + (litSize
                >= ((16 as ::core::ffi::c_int
                    * ((1 as ::core::ffi::c_int) << 10 as ::core::ffi::c_int))
                    as size_t)
                    .wrapping_sub(header)) as ::core::ffi::c_int) as size_t;
    let ostart: *mut BYTE = dst as *mut BYTE;
    let oend: *mut BYTE = ostart.offset(dstSize as isize);
    let mut op: *mut BYTE = ostart.offset(lhSize as isize);
    let singleStream: U32 = (lhSize == 3 as size_t) as ::core::ffi::c_int as U32;
    let mut hType: SymbolEncodingType_e = (if writeEntropy != 0 {
        (*hufMetadata).hType as ::core::ffi::c_uint
    } else {
        set_repeat as ::core::ffi::c_int as ::core::ffi::c_uint
    }) as SymbolEncodingType_e;
    let mut cLitSize: size_t = 0 as size_t;
    *entropyWritten = 0 as ::core::ffi::c_int;
    if litSize == 0 as size_t
        || (*hufMetadata).hType as ::core::ffi::c_uint
            == set_basic as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        return ZSTD_noCompressLiterals(
            dst,
            dstSize,
            literals as *const ::core::ffi::c_void,
            litSize,
        );
    } else if (*hufMetadata).hType as ::core::ffi::c_uint
        == set_rle as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        return ZSTD_compressRleLiteralsBlock(
            dst,
            dstSize,
            literals as *const ::core::ffi::c_void,
            litSize,
        );
    }
    if writeEntropy != 0
        && (*hufMetadata).hType as ::core::ffi::c_uint
            == set_compressed as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        ::libc::memcpy(
            op as *mut ::core::ffi::c_void,
            &raw const (*hufMetadata).hufDesBuffer as *const BYTE as *const ::core::ffi::c_void,
            (*hufMetadata).hufDesSize as ::libc::size_t,
        );
        op = op.offset((*hufMetadata).hufDesSize as isize);
        cLitSize = (cLitSize as ::core::ffi::c_ulong)
            .wrapping_add((*hufMetadata).hufDesSize as ::core::ffi::c_ulong)
            as size_t as size_t;
    }
    let flags: ::core::ffi::c_int = if bmi2 != 0 {
        HUF_flags_bmi2 as ::core::ffi::c_int as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    };
    let cSize: size_t = if singleStream != 0 {
        HUF_compress1X_usingCTable(
            op as *mut ::core::ffi::c_void,
            oend.offset_from(op) as ::core::ffi::c_long as size_t,
            literals as *const ::core::ffi::c_void,
            litSize,
            hufTable,
            flags,
        ) as size_t
    } else {
        HUF_compress4X_usingCTable(
            op as *mut ::core::ffi::c_void,
            oend.offset_from(op) as ::core::ffi::c_long as size_t,
            literals as *const ::core::ffi::c_void,
            litSize,
            hufTable,
            flags,
        ) as size_t
    };
    op = op.offset(cSize as isize);
    cLitSize = (cLitSize as ::core::ffi::c_ulong).wrapping_add(cSize as ::core::ffi::c_ulong)
        as size_t as size_t;
    if cSize == 0 as size_t || ERR_isError(cSize) != 0 {
        return 0 as size_t;
    }
    if writeEntropy == 0 && cLitSize >= litSize {
        return ZSTD_noCompressLiterals(
            dst,
            dstSize,
            literals as *const ::core::ffi::c_void,
            litSize,
        );
    }
    if lhSize
        < (3 as ::core::ffi::c_int
            + (cLitSize
                >= (1 as ::core::ffi::c_int
                    * ((1 as ::core::ffi::c_int) << 10 as ::core::ffi::c_int))
                    as size_t) as ::core::ffi::c_int
            + (cLitSize
                >= (16 as ::core::ffi::c_int
                    * ((1 as ::core::ffi::c_int) << 10 as ::core::ffi::c_int))
                    as size_t) as ::core::ffi::c_int) as size_t
    {
        return ZSTD_noCompressLiterals(
            dst,
            dstSize,
            literals as *const ::core::ffi::c_void,
            litSize,
        );
    }
    match lhSize {
        3 => {
            let lhc: U32 = (hType as U32)
                .wrapping_add(
                    ((singleStream == 0) as ::core::ffi::c_int as U32) << 2 as ::core::ffi::c_int,
                )
                .wrapping_add((litSize as U32) << 4 as ::core::ffi::c_int)
                .wrapping_add((cLitSize as U32) << 14 as ::core::ffi::c_int);
            MEM_writeLE24(ostart as *mut ::core::ffi::c_void, lhc);
        }
        4 => {
            let lhc_0: U32 = (hType as U32)
                .wrapping_add(((2 as ::core::ffi::c_int) << 2 as ::core::ffi::c_int) as U32)
                .wrapping_add((litSize as U32) << 4 as ::core::ffi::c_int)
                .wrapping_add((cLitSize as U32) << 18 as ::core::ffi::c_int);
            MEM_writeLE32(ostart as *mut ::core::ffi::c_void, lhc_0);
        }
        5 => {
            let lhc_1: U32 = (hType as U32)
                .wrapping_add(((3 as ::core::ffi::c_int) << 2 as ::core::ffi::c_int) as U32)
                .wrapping_add((litSize as U32) << 4 as ::core::ffi::c_int)
                .wrapping_add((cLitSize as U32) << 22 as ::core::ffi::c_int);
            MEM_writeLE32(ostart as *mut ::core::ffi::c_void, lhc_1);
            *ostart.offset(4 as ::core::ffi::c_int as isize) =
                (cLitSize >> 10 as ::core::ffi::c_int) as BYTE;
        }
        _ => {}
    }
    *entropyWritten = 1 as ::core::ffi::c_int;
    return op.offset_from(ostart) as ::core::ffi::c_long as size_t;
}
unsafe extern "C" fn ZSTD_seqDecompressedSize(
    mut seqStore: *const SeqStore_t,
    mut sequences: *const SeqDef,
    mut nbSeqs: size_t,
    mut litSize: size_t,
    mut lastSubBlock: ::core::ffi::c_int,
) -> size_t {
    let mut matchLengthSum: size_t = 0 as size_t;
    let mut litLengthSum: size_t = 0 as size_t;
    let mut n: size_t = 0;
    n = 0 as size_t;
    while n < nbSeqs {
        let seqLen: ZSTD_SequenceLength =
            ZSTD_getSequenceLength(seqStore, sequences.offset(n as isize)) as ZSTD_SequenceLength;
        litLengthSum = (litLengthSum as ::core::ffi::c_ulong)
            .wrapping_add(seqLen.litLength as ::core::ffi::c_ulong) as size_t
            as size_t;
        matchLengthSum = (matchLengthSum as ::core::ffi::c_ulong)
            .wrapping_add(seqLen.matchLength as ::core::ffi::c_ulong)
            as size_t as size_t;
        n = n.wrapping_add(1);
    }
    lastSubBlock == 0;
    return matchLengthSum.wrapping_add(litSize);
}
unsafe extern "C" fn ZSTD_compressSubBlock_sequences(
    mut fseTables: *const ZSTD_fseCTables_t,
    mut fseMetadata: *const ZSTD_fseCTablesMetadata_t,
    mut sequences: *const SeqDef,
    mut nbSeq: size_t,
    mut llCode: *const BYTE,
    mut mlCode: *const BYTE,
    mut ofCode: *const BYTE,
    mut cctxParams: *const ZSTD_CCtx_params,
    mut dst: *mut ::core::ffi::c_void,
    mut dstCapacity: size_t,
    bmi2: ::core::ffi::c_int,
    mut writeEntropy: ::core::ffi::c_int,
    mut entropyWritten: *mut ::core::ffi::c_int,
) -> size_t {
    let longOffsets: ::core::ffi::c_int = ((*cctxParams).cParams.windowLog as U32
        > (if MEM_32bits() != 0 {
            STREAM_ACCUMULATOR_MIN_32
        } else {
            STREAM_ACCUMULATOR_MIN_64
        }) as U32) as ::core::ffi::c_int;
    let ostart: *mut BYTE = dst as *mut BYTE;
    let oend: *mut BYTE = ostart.offset(dstCapacity as isize);
    let mut op: *mut BYTE = ostart;
    let mut seqHead: *mut BYTE = ::core::ptr::null_mut::<BYTE>();
    *entropyWritten = 0 as ::core::ffi::c_int;
    if (oend.offset_from(op) as ::core::ffi::c_long)
        < (3 as ::core::ffi::c_int + 1 as ::core::ffi::c_int) as ::core::ffi::c_long
    {
        return -(ZSTD_error_dstSize_tooSmall as ::core::ffi::c_int) as size_t;
    }
    if nbSeq < 128 as size_t {
        let fresh0 = op;
        op = op.offset(1);
        *fresh0 = nbSeq as BYTE;
    } else if nbSeq < LONGNBSEQ as size_t {
        *op.offset(0 as ::core::ffi::c_int as isize) =
            (nbSeq >> 8 as ::core::ffi::c_int).wrapping_add(0x80 as size_t) as BYTE;
        *op.offset(1 as ::core::ffi::c_int as isize) = nbSeq as BYTE;
        op = op.offset(2 as ::core::ffi::c_int as isize);
    } else {
        *op.offset(0 as ::core::ffi::c_int as isize) = 0xff as BYTE;
        MEM_writeLE16(
            op.offset(1 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_void,
            nbSeq.wrapping_sub(LONGNBSEQ as size_t) as U16,
        );
        op = op.offset(3 as ::core::ffi::c_int as isize);
    }
    if nbSeq == 0 as size_t {
        return op.offset_from(ostart) as ::core::ffi::c_long as size_t;
    }
    let fresh1 = op;
    op = op.offset(1);
    seqHead = fresh1;
    if writeEntropy != 0 {
        let LLtype: U32 = (*fseMetadata).llType as U32;
        let Offtype: U32 = (*fseMetadata).ofType as U32;
        let MLtype: U32 = (*fseMetadata).mlType as U32;
        *seqHead = (LLtype << 6 as ::core::ffi::c_int)
            .wrapping_add(Offtype << 4 as ::core::ffi::c_int)
            .wrapping_add(MLtype << 2 as ::core::ffi::c_int) as BYTE;
        ::libc::memcpy(
            op as *mut ::core::ffi::c_void,
            &raw const (*fseMetadata).fseTablesBuffer as *const BYTE as *const ::core::ffi::c_void,
            (*fseMetadata).fseTablesSize as ::libc::size_t,
        );
        op = op.offset((*fseMetadata).fseTablesSize as isize);
    } else {
        let repeat: U32 = set_repeat as ::core::ffi::c_int as U32;
        *seqHead = (repeat << 6 as ::core::ffi::c_int)
            .wrapping_add(repeat << 4 as ::core::ffi::c_int)
            .wrapping_add(repeat << 2 as ::core::ffi::c_int) as BYTE;
    }
    let bitstreamSize: size_t = ZSTD_encodeSequences(
        op as *mut ::core::ffi::c_void,
        oend.offset_from(op) as ::core::ffi::c_long as size_t,
        &raw const (*fseTables).matchlengthCTable as *const FSE_CTable,
        mlCode,
        &raw const (*fseTables).offcodeCTable as *const FSE_CTable,
        ofCode,
        &raw const (*fseTables).litlengthCTable as *const FSE_CTable,
        llCode,
        sequences,
        nbSeq,
        longOffsets,
        bmi2,
    ) as size_t;
    let err_code: size_t = bitstreamSize;
    if ERR_isError(err_code) != 0 {
        return err_code;
    }
    op = op.offset(bitstreamSize as isize);
    if writeEntropy != 0
        && (*fseMetadata).lastCountSize != 0
        && (*fseMetadata).lastCountSize.wrapping_add(bitstreamSize) < 4 as size_t
    {
        return 0 as size_t;
    }
    if (op.offset_from(seqHead) as ::core::ffi::c_long) < 4 as ::core::ffi::c_long {
        return 0 as size_t;
    }
    *entropyWritten = 1 as ::core::ffi::c_int;
    return op.offset_from(ostart) as ::core::ffi::c_long as size_t;
}
unsafe extern "C" fn ZSTD_compressSubBlock(
    mut entropy: *const ZSTD_entropyCTables_t,
    mut entropyMetadata: *const ZSTD_entropyCTablesMetadata_t,
    mut sequences: *const SeqDef,
    mut nbSeq: size_t,
    mut literals: *const BYTE,
    mut litSize: size_t,
    mut llCode: *const BYTE,
    mut mlCode: *const BYTE,
    mut ofCode: *const BYTE,
    mut cctxParams: *const ZSTD_CCtx_params,
    mut dst: *mut ::core::ffi::c_void,
    mut dstCapacity: size_t,
    bmi2: ::core::ffi::c_int,
    mut writeLitEntropy: ::core::ffi::c_int,
    mut writeSeqEntropy: ::core::ffi::c_int,
    mut litEntropyWritten: *mut ::core::ffi::c_int,
    mut seqEntropyWritten: *mut ::core::ffi::c_int,
    mut lastBlock: U32,
) -> size_t {
    let ostart: *mut BYTE = dst as *mut BYTE;
    let oend: *mut BYTE = ostart.offset(dstCapacity as isize);
    let mut op: *mut BYTE = ostart.offset(ZSTD_blockHeaderSize as isize);
    let mut cLitSize: size_t = ZSTD_compressSubBlock_literal(
        &raw const (*entropy).huf.CTable as *const HUF_CElt,
        &raw const (*entropyMetadata).hufMetadata,
        literals,
        litSize,
        op as *mut ::core::ffi::c_void,
        oend.offset_from(op) as ::core::ffi::c_long as size_t,
        bmi2,
        writeLitEntropy,
        litEntropyWritten,
    );
    let err_code: size_t = cLitSize;
    if ERR_isError(err_code) != 0 {
        return err_code;
    }
    if cLitSize == 0 as size_t {
        return 0 as size_t;
    }
    op = op.offset(cLitSize as isize);
    let mut cSeqSize: size_t = ZSTD_compressSubBlock_sequences(
        &raw const (*entropy).fse,
        &raw const (*entropyMetadata).fseMetadata,
        sequences,
        nbSeq,
        llCode,
        mlCode,
        ofCode,
        cctxParams,
        op as *mut ::core::ffi::c_void,
        oend.offset_from(op) as ::core::ffi::c_long as size_t,
        bmi2,
        writeSeqEntropy,
        seqEntropyWritten,
    );
    let err_code_0: size_t = cSeqSize;
    if ERR_isError(err_code_0) != 0 {
        return err_code_0;
    }
    if cSeqSize == 0 as size_t {
        return 0 as size_t;
    }
    op = op.offset(cSeqSize as isize);
    let mut cSize: size_t = (op.offset_from(ostart) as ::core::ffi::c_long as size_t)
        .wrapping_sub(ZSTD_blockHeaderSize);
    let cBlockHeader24: U32 = lastBlock
        .wrapping_add((bt_compressed as ::core::ffi::c_int as U32) << 1 as ::core::ffi::c_int)
        .wrapping_add((cSize << 3 as ::core::ffi::c_int) as U32);
    MEM_writeLE24(ostart as *mut ::core::ffi::c_void, cBlockHeader24);
    return op.offset_from(ostart) as ::core::ffi::c_long as size_t;
}
unsafe extern "C" fn ZSTD_estimateSubBlockSize_literal(
    mut literals: *const BYTE,
    mut litSize: size_t,
    mut huf: *const ZSTD_hufCTables_t,
    mut hufMetadata: *const ZSTD_hufCTablesMetadata_t,
    mut workspace: *mut ::core::ffi::c_void,
    mut wkspSize: size_t,
    mut writeEntropy: ::core::ffi::c_int,
) -> size_t {
    let countWksp: *mut ::core::ffi::c_uint = workspace as *mut ::core::ffi::c_uint;
    let mut maxSymbolValue: ::core::ffi::c_uint = 255 as ::core::ffi::c_uint;
    let mut literalSectionHeaderSize: size_t = 3 as size_t;
    if (*hufMetadata).hType as ::core::ffi::c_uint
        == set_basic as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        return litSize;
    } else if (*hufMetadata).hType as ::core::ffi::c_uint
        == set_rle as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        return 1 as size_t;
    } else if (*hufMetadata).hType as ::core::ffi::c_uint
        == set_compressed as ::core::ffi::c_int as ::core::ffi::c_uint
        || (*hufMetadata).hType as ::core::ffi::c_uint
            == set_repeat as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        let largest: size_t = HIST_count_wksp(
            countWksp,
            &raw mut maxSymbolValue,
            literals as *const ::core::ffi::c_void,
            litSize,
            workspace,
            wkspSize,
        ) as size_t;
        if ERR_isError(largest) != 0 {
            return litSize;
        }
        let mut cLitSizeEstimate: size_t = HUF_estimateCompressedSize(
            &raw const (*huf).CTable as *const HUF_CElt,
            countWksp,
            maxSymbolValue,
        );
        if writeEntropy != 0 {
            cLitSizeEstimate = (cLitSizeEstimate as ::core::ffi::c_ulong)
                .wrapping_add((*hufMetadata).hufDesSize as ::core::ffi::c_ulong)
                as size_t as size_t;
        }
        return cLitSizeEstimate.wrapping_add(literalSectionHeaderSize);
    }
    return 0 as size_t;
}
unsafe extern "C" fn ZSTD_estimateSubBlockSize_symbolType(
    mut type_0: SymbolEncodingType_e,
    mut codeTable: *const BYTE,
    mut maxCode: ::core::ffi::c_uint,
    mut nbSeq: size_t,
    mut fseCTable: *const FSE_CTable,
    mut additionalBits: *const U8,
    mut defaultNorm: *const ::core::ffi::c_short,
    mut defaultNormLog: U32,
    mut defaultMax: U32,
    mut workspace: *mut ::core::ffi::c_void,
    mut wkspSize: size_t,
) -> size_t {
    let countWksp: *mut ::core::ffi::c_uint = workspace as *mut ::core::ffi::c_uint;
    let mut ctp: *const BYTE = codeTable;
    let ctStart: *const BYTE = ctp;
    let ctEnd: *const BYTE = ctStart.offset(nbSeq as isize);
    let mut cSymbolTypeSizeEstimateInBits: size_t = 0 as size_t;
    let mut max: ::core::ffi::c_uint = maxCode;
    HIST_countFast_wksp(
        countWksp,
        &raw mut max,
        codeTable as *const ::core::ffi::c_void,
        nbSeq,
        workspace,
        wkspSize,
    );
    if type_0 as ::core::ffi::c_uint == set_basic as ::core::ffi::c_int as ::core::ffi::c_uint {
        cSymbolTypeSizeEstimateInBits = if max as U32 <= defaultMax {
            ZSTD_crossEntropyCost(
                defaultNorm,
                defaultNormLog as ::core::ffi::c_uint,
                countWksp,
                max,
            )
        } else {
            -(ZSTD_error_GENERIC as ::core::ffi::c_int) as size_t
        };
    } else if type_0 as ::core::ffi::c_uint == set_rle as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        cSymbolTypeSizeEstimateInBits = 0 as size_t;
    } else if type_0 as ::core::ffi::c_uint
        == set_compressed as ::core::ffi::c_int as ::core::ffi::c_uint
        || type_0 as ::core::ffi::c_uint == set_repeat as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        cSymbolTypeSizeEstimateInBits = ZSTD_fseBitCost(fseCTable, countWksp, max);
    }
    if ERR_isError(cSymbolTypeSizeEstimateInBits) != 0 {
        return nbSeq.wrapping_mul(10 as size_t);
    }
    while ctp < ctEnd {
        if !additionalBits.is_null() {
            cSymbolTypeSizeEstimateInBits = (cSymbolTypeSizeEstimateInBits as ::core::ffi::c_ulong)
                .wrapping_add(*additionalBits.offset(*ctp as isize) as ::core::ffi::c_ulong)
                as size_t as size_t;
        } else {
            cSymbolTypeSizeEstimateInBits = (cSymbolTypeSizeEstimateInBits as ::core::ffi::c_ulong)
                .wrapping_add(*ctp as ::core::ffi::c_ulong)
                as size_t as size_t;
        }
        ctp = ctp.offset(1);
    }
    return cSymbolTypeSizeEstimateInBits.wrapping_div(8 as size_t);
}
unsafe extern "C" fn ZSTD_estimateSubBlockSize_sequences(
    mut ofCodeTable: *const BYTE,
    mut llCodeTable: *const BYTE,
    mut mlCodeTable: *const BYTE,
    mut nbSeq: size_t,
    mut fseTables: *const ZSTD_fseCTables_t,
    mut fseMetadata: *const ZSTD_fseCTablesMetadata_t,
    mut workspace: *mut ::core::ffi::c_void,
    mut wkspSize: size_t,
    mut writeEntropy: ::core::ffi::c_int,
) -> size_t {
    let sequencesSectionHeaderSize: size_t = 3 as size_t;
    let mut cSeqSizeEstimate: size_t = 0 as size_t;
    if nbSeq == 0 as size_t {
        return sequencesSectionHeaderSize;
    }
    cSeqSizeEstimate = (cSeqSizeEstimate as ::core::ffi::c_ulong).wrapping_add(
        ZSTD_estimateSubBlockSize_symbolType(
            (*fseMetadata).ofType,
            ofCodeTable,
            MaxOff as ::core::ffi::c_uint,
            nbSeq,
            &raw const (*fseTables).offcodeCTable as *const FSE_CTable,
            ::core::ptr::null::<U8>(),
            &raw const OF_defaultNorm as *const ::core::ffi::c_short,
            OF_defaultNormLog,
            DefaultMaxOff as U32,
            workspace,
            wkspSize,
        ) as ::core::ffi::c_ulong,
    ) as size_t as size_t;
    cSeqSizeEstimate = (cSeqSizeEstimate as ::core::ffi::c_ulong).wrapping_add(
        ZSTD_estimateSubBlockSize_symbolType(
            (*fseMetadata).llType,
            llCodeTable,
            MaxLL as ::core::ffi::c_uint,
            nbSeq,
            &raw const (*fseTables).litlengthCTable as *const FSE_CTable,
            &raw const LL_bits as *const U8,
            &raw const LL_defaultNorm as *const ::core::ffi::c_short,
            LL_defaultNormLog,
            MaxLL as U32,
            workspace,
            wkspSize,
        ) as ::core::ffi::c_ulong,
    ) as size_t as size_t;
    cSeqSizeEstimate = (cSeqSizeEstimate as ::core::ffi::c_ulong).wrapping_add(
        ZSTD_estimateSubBlockSize_symbolType(
            (*fseMetadata).mlType,
            mlCodeTable,
            MaxML as ::core::ffi::c_uint,
            nbSeq,
            &raw const (*fseTables).matchlengthCTable as *const FSE_CTable,
            &raw const ML_bits as *const U8,
            &raw const ML_defaultNorm as *const ::core::ffi::c_short,
            ML_defaultNormLog,
            MaxML as U32,
            workspace,
            wkspSize,
        ) as ::core::ffi::c_ulong,
    ) as size_t as size_t;
    if writeEntropy != 0 {
        cSeqSizeEstimate = (cSeqSizeEstimate as ::core::ffi::c_ulong)
            .wrapping_add((*fseMetadata).fseTablesSize as ::core::ffi::c_ulong)
            as size_t as size_t;
    }
    return cSeqSizeEstimate.wrapping_add(sequencesSectionHeaderSize);
}
unsafe extern "C" fn ZSTD_estimateSubBlockSize(
    mut literals: *const BYTE,
    mut litSize: size_t,
    mut ofCodeTable: *const BYTE,
    mut llCodeTable: *const BYTE,
    mut mlCodeTable: *const BYTE,
    mut nbSeq: size_t,
    mut entropy: *const ZSTD_entropyCTables_t,
    mut entropyMetadata: *const ZSTD_entropyCTablesMetadata_t,
    mut workspace: *mut ::core::ffi::c_void,
    mut wkspSize: size_t,
    mut writeLitEntropy: ::core::ffi::c_int,
    mut writeSeqEntropy: ::core::ffi::c_int,
) -> EstimatedBlockSize {
    let mut ebs: EstimatedBlockSize = EstimatedBlockSize {
        estLitSize: 0,
        estBlockSize: 0,
    };
    ebs.estLitSize = ZSTD_estimateSubBlockSize_literal(
        literals,
        litSize,
        &raw const (*entropy).huf,
        &raw const (*entropyMetadata).hufMetadata,
        workspace,
        wkspSize,
        writeLitEntropy,
    );
    ebs.estBlockSize = ZSTD_estimateSubBlockSize_sequences(
        ofCodeTable,
        llCodeTable,
        mlCodeTable,
        nbSeq,
        &raw const (*entropy).fse,
        &raw const (*entropyMetadata).fseMetadata,
        workspace,
        wkspSize,
        writeSeqEntropy,
    );
    ebs.estBlockSize = (ebs.estBlockSize as ::core::ffi::c_ulong)
        .wrapping_add(ebs.estLitSize.wrapping_add(ZSTD_blockHeaderSize) as ::core::ffi::c_ulong)
        as size_t as size_t;
    return ebs;
}
unsafe extern "C" fn ZSTD_needSequenceEntropyTables(
    mut fseMetadata: *const ZSTD_fseCTablesMetadata_t,
) -> ::core::ffi::c_int {
    if (*fseMetadata).llType as ::core::ffi::c_uint
        == set_compressed as ::core::ffi::c_int as ::core::ffi::c_uint
        || (*fseMetadata).llType as ::core::ffi::c_uint
            == set_rle as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        return 1 as ::core::ffi::c_int;
    }
    if (*fseMetadata).mlType as ::core::ffi::c_uint
        == set_compressed as ::core::ffi::c_int as ::core::ffi::c_uint
        || (*fseMetadata).mlType as ::core::ffi::c_uint
            == set_rle as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        return 1 as ::core::ffi::c_int;
    }
    if (*fseMetadata).ofType as ::core::ffi::c_uint
        == set_compressed as ::core::ffi::c_int as ::core::ffi::c_uint
        || (*fseMetadata).ofType as ::core::ffi::c_uint
            == set_rle as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        return 1 as ::core::ffi::c_int;
    }
    return 0 as ::core::ffi::c_int;
}
unsafe extern "C" fn countLiterals(
    mut seqStore: *const SeqStore_t,
    mut sp: *const SeqDef,
    mut seqCount: size_t,
) -> size_t {
    let mut n: size_t = 0;
    let mut total: size_t = 0 as size_t;
    n = 0 as size_t;
    while n < seqCount {
        total = (total as ::core::ffi::c_ulong).wrapping_add(
            ZSTD_getSequenceLength(seqStore, sp.offset(n as isize)).litLength
                as ::core::ffi::c_ulong,
        ) as size_t as size_t;
        n = n.wrapping_add(1);
    }
    return total;
}
pub const BYTESCALE: ::core::ffi::c_int = 256 as ::core::ffi::c_int;
unsafe extern "C" fn sizeBlockSequences(
    mut sp: *const SeqDef,
    mut nbSeqs: size_t,
    mut targetBudget: size_t,
    mut avgLitCost: size_t,
    mut avgSeqCost: size_t,
    mut firstSubBlock: ::core::ffi::c_int,
) -> size_t {
    let mut n: size_t = 0;
    let mut budget: size_t = 0 as size_t;
    let mut inSize: size_t = 0 as size_t;
    let headerSize: size_t = (firstSubBlock as size_t)
        .wrapping_mul(120 as size_t)
        .wrapping_mul(BYTESCALE as size_t);
    budget = (budget as ::core::ffi::c_ulong).wrapping_add(headerSize as ::core::ffi::c_ulong)
        as size_t as size_t;
    budget = (budget as ::core::ffi::c_ulong).wrapping_add(
        ((*sp.offset(0 as ::core::ffi::c_int as isize)).litLength as size_t)
            .wrapping_mul(avgLitCost)
            .wrapping_add(avgSeqCost) as ::core::ffi::c_ulong,
    ) as size_t as size_t;
    if budget > targetBudget {
        return 1 as size_t;
    }
    inSize = ((*sp.offset(0 as ::core::ffi::c_int as isize)).litLength as ::core::ffi::c_int
        + ((*sp.offset(0 as ::core::ffi::c_int as isize)).mlBase as ::core::ffi::c_int + MINMATCH))
        as size_t;
    n = 1 as size_t;
    while n < nbSeqs {
        let mut currentCost: size_t = ((*sp.offset(n as isize)).litLength as size_t)
            .wrapping_mul(avgLitCost)
            .wrapping_add(avgSeqCost);
        budget = (budget as ::core::ffi::c_ulong).wrapping_add(currentCost as ::core::ffi::c_ulong)
            as size_t as size_t;
        inSize = (inSize as ::core::ffi::c_ulong).wrapping_add(
            ((*sp.offset(n as isize)).litLength as ::core::ffi::c_int
                + ((*sp.offset(n as isize)).mlBase as ::core::ffi::c_int + MINMATCH))
                as ::core::ffi::c_ulong,
        ) as size_t as size_t;
        if budget > targetBudget && budget < inSize.wrapping_mul(BYTESCALE as size_t) {
            break;
        }
        n = n.wrapping_add(1);
    }
    return n;
}
unsafe extern "C" fn ZSTD_compressSubBlock_multi(
    mut seqStorePtr: *const SeqStore_t,
    mut prevCBlock: *const ZSTD_compressedBlockState_t,
    mut nextCBlock: *mut ZSTD_compressedBlockState_t,
    mut entropyMetadata: *const ZSTD_entropyCTablesMetadata_t,
    mut cctxParams: *const ZSTD_CCtx_params,
    mut dst: *mut ::core::ffi::c_void,
    mut dstCapacity: size_t,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
    bmi2: ::core::ffi::c_int,
    mut lastBlock: U32,
    mut workspace: *mut ::core::ffi::c_void,
    mut wkspSize: size_t,
) -> size_t {
    let sstart: *const SeqDef = (*seqStorePtr).sequencesStart;
    let send: *const SeqDef = (*seqStorePtr).sequences;
    let mut sp: *const SeqDef = sstart;
    let nbSeqs: size_t = send.offset_from(sstart) as ::core::ffi::c_long as size_t;
    let lstart: *const BYTE = (*seqStorePtr).litStart;
    let lend: *const BYTE = (*seqStorePtr).lit;
    let mut lp: *const BYTE = lstart;
    let nbLiterals: size_t = lend.offset_from(lstart) as ::core::ffi::c_long as size_t;
    let mut ip: *const BYTE = src as *const BYTE;
    let iend: *const BYTE = ip.offset(srcSize as isize);
    let ostart: *mut BYTE = dst as *mut BYTE;
    let oend: *mut BYTE = ostart.offset(dstCapacity as isize);
    let mut op: *mut BYTE = ostart;
    let mut llCodePtr: *const BYTE = (*seqStorePtr).llCode;
    let mut mlCodePtr: *const BYTE = (*seqStorePtr).mlCode;
    let mut ofCodePtr: *const BYTE = (*seqStorePtr).ofCode;
    let minTarget: size_t = ZSTD_TARGETCBLOCKSIZE_MIN as size_t;
    let targetCBlockSize: size_t = if minTarget > (*cctxParams).targetCBlockSize {
        minTarget
    } else {
        (*cctxParams).targetCBlockSize
    };
    let mut writeLitEntropy: ::core::ffi::c_int = ((*entropyMetadata).hufMetadata.hType
        as ::core::ffi::c_uint
        == set_compressed as ::core::ffi::c_int as ::core::ffi::c_uint)
        as ::core::ffi::c_int;
    let mut writeSeqEntropy: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
    if nbSeqs > 0 as size_t {
        let ebs: EstimatedBlockSize = ZSTD_estimateSubBlockSize(
            lp,
            nbLiterals,
            ofCodePtr,
            llCodePtr,
            mlCodePtr,
            nbSeqs,
            &raw mut (*nextCBlock).entropy,
            entropyMetadata,
            workspace,
            wkspSize,
            writeLitEntropy,
            writeSeqEntropy,
        ) as EstimatedBlockSize;
        let avgLitCost: size_t = if nbLiterals != 0 {
            ebs.estLitSize
                .wrapping_mul(BYTESCALE as size_t)
                .wrapping_div(nbLiterals)
        } else {
            BYTESCALE as size_t
        };
        let avgSeqCost: size_t = ebs
            .estBlockSize
            .wrapping_sub(ebs.estLitSize)
            .wrapping_mul(BYTESCALE as size_t)
            .wrapping_div(nbSeqs);
        let nbSubBlocks: size_t = if ebs
            .estBlockSize
            .wrapping_add(targetCBlockSize.wrapping_div(2 as size_t))
            .wrapping_div(targetCBlockSize)
            > 1 as size_t
        {
            ebs.estBlockSize
                .wrapping_add(targetCBlockSize.wrapping_div(2 as size_t))
                .wrapping_div(targetCBlockSize)
        } else {
            1 as size_t
        };
        let mut n: size_t = 0;
        let mut avgBlockBudget: size_t = 0;
        let mut blockBudgetSupp: size_t = 0 as size_t;
        avgBlockBudget = ebs
            .estBlockSize
            .wrapping_mul(BYTESCALE as size_t)
            .wrapping_div(nbSubBlocks);
        if ebs.estBlockSize > srcSize {
            return 0 as size_t;
        }
        n = 0 as size_t;
        while n < nbSubBlocks.wrapping_sub(1 as size_t) {
            let seqCount: size_t = sizeBlockSequences(
                sp,
                send.offset_from(sp) as ::core::ffi::c_long as size_t,
                avgBlockBudget.wrapping_add(blockBudgetSupp),
                avgLitCost,
                avgSeqCost,
                (n == 0 as size_t) as ::core::ffi::c_int,
            ) as size_t;
            if sp.offset(seqCount as isize) == send {
                break;
            }
            let mut litEntropyWritten: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
            let mut seqEntropyWritten: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
            let mut litSize: size_t = countLiterals(seqStorePtr, sp, seqCount);
            let decompressedSize: size_t = ZSTD_seqDecompressedSize(
                seqStorePtr,
                sp,
                seqCount,
                litSize,
                0 as ::core::ffi::c_int,
            ) as size_t;
            let cSize: size_t = ZSTD_compressSubBlock(
                &raw mut (*nextCBlock).entropy,
                entropyMetadata,
                sp,
                seqCount,
                lp,
                litSize,
                llCodePtr,
                mlCodePtr,
                ofCodePtr,
                cctxParams,
                op as *mut ::core::ffi::c_void,
                oend.offset_from(op) as ::core::ffi::c_long as size_t,
                bmi2,
                writeLitEntropy,
                writeSeqEntropy,
                &raw mut litEntropyWritten,
                &raw mut seqEntropyWritten,
                0 as U32,
            ) as size_t;
            let err_code: size_t = cSize;
            if ERR_isError(err_code) != 0 {
                return err_code;
            }
            if cSize > 0 as size_t && cSize < decompressedSize {
                ip = ip.offset(decompressedSize as isize);
                lp = lp.offset(litSize as isize);
                op = op.offset(cSize as isize);
                llCodePtr = llCodePtr.offset(seqCount as isize);
                mlCodePtr = mlCodePtr.offset(seqCount as isize);
                ofCodePtr = ofCodePtr.offset(seqCount as isize);
                if litEntropyWritten != 0 {
                    writeLitEntropy = 0 as ::core::ffi::c_int;
                }
                if seqEntropyWritten != 0 {
                    writeSeqEntropy = 0 as ::core::ffi::c_int;
                }
                sp = sp.offset(seqCount as isize);
                blockBudgetSupp = 0 as size_t;
            }
            n = n.wrapping_add(1);
        }
    }
    let mut litEntropyWritten_0: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut seqEntropyWritten_0: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut litSize_0: size_t = lend.offset_from(lp) as ::core::ffi::c_long as size_t;
    let mut seqCount_0: size_t = send.offset_from(sp) as ::core::ffi::c_long as size_t;
    let decompressedSize_0: size_t = ZSTD_seqDecompressedSize(
        seqStorePtr,
        sp,
        seqCount_0,
        litSize_0,
        1 as ::core::ffi::c_int,
    ) as size_t;
    let cSize_0: size_t = ZSTD_compressSubBlock(
        &raw mut (*nextCBlock).entropy,
        entropyMetadata,
        sp,
        seqCount_0,
        lp,
        litSize_0,
        llCodePtr,
        mlCodePtr,
        ofCodePtr,
        cctxParams,
        op as *mut ::core::ffi::c_void,
        oend.offset_from(op) as ::core::ffi::c_long as size_t,
        bmi2,
        writeLitEntropy,
        writeSeqEntropy,
        &raw mut litEntropyWritten_0,
        &raw mut seqEntropyWritten_0,
        lastBlock,
    ) as size_t;
    let err_code_0: size_t = cSize_0;
    if ERR_isError(err_code_0) != 0 {
        return err_code_0;
    }
    if cSize_0 > 0 as size_t && cSize_0 < decompressedSize_0 {
        ip = ip.offset(decompressedSize_0 as isize);
        lp = lp.offset(litSize_0 as isize);
        op = op.offset(cSize_0 as isize);
        llCodePtr = llCodePtr.offset(seqCount_0 as isize);
        mlCodePtr = mlCodePtr.offset(seqCount_0 as isize);
        ofCodePtr = ofCodePtr.offset(seqCount_0 as isize);
        if litEntropyWritten_0 != 0 {
            writeLitEntropy = 0 as ::core::ffi::c_int;
        }
        if seqEntropyWritten_0 != 0 {
            writeSeqEntropy = 0 as ::core::ffi::c_int;
        }
        sp = sp.offset(seqCount_0 as isize);
    }
    if writeLitEntropy != 0 {
        ::libc::memcpy(
            &raw mut (*nextCBlock).entropy.huf as *mut ::core::ffi::c_void,
            &raw const (*prevCBlock).entropy.huf as *const ::core::ffi::c_void,
            ::core::mem::size_of::<ZSTD_hufCTables_t>() as ::libc::size_t,
        );
    }
    if writeSeqEntropy != 0
        && ZSTD_needSequenceEntropyTables(&raw const (*entropyMetadata).fseMetadata) != 0
    {
        return 0 as size_t;
    }
    if ip < iend {
        let rSize: size_t = iend.offset_from(ip) as ::core::ffi::c_long as size_t;
        let cSize_1: size_t = ZSTD_noCompressBlock(
            op as *mut ::core::ffi::c_void,
            oend.offset_from(op) as ::core::ffi::c_long as size_t,
            ip as *const ::core::ffi::c_void,
            rSize,
            lastBlock,
        ) as size_t;
        let err_code_1: size_t = cSize_1;
        if ERR_isError(err_code_1) != 0 {
            return err_code_1;
        }
        op = op.offset(cSize_1 as isize);
        if sp < send {
            let mut seq: *const SeqDef = ::core::ptr::null::<SeqDef>();
            let mut rep: Repcodes_t = Repcodes_t { rep: [0; 3] };
            ::libc::memcpy(
                &raw mut rep as *mut ::core::ffi::c_void,
                &raw const (*prevCBlock).rep as *const U32 as *const ::core::ffi::c_void,
                ::core::mem::size_of::<Repcodes_t>() as ::libc::size_t,
            );
            seq = sstart;
            while seq < sp {
                ZSTD_updateRep(
                    &raw mut rep.rep as *mut U32,
                    (*seq).offBase,
                    (ZSTD_getSequenceLength(seqStorePtr, seq).litLength == 0 as U32)
                        as ::core::ffi::c_int as U32,
                );
                seq = seq.offset(1);
            }
            ::libc::memcpy(
                &raw mut (*nextCBlock).rep as *mut U32 as *mut ::core::ffi::c_void,
                &raw mut rep as *const ::core::ffi::c_void,
                ::core::mem::size_of::<Repcodes_t>() as ::libc::size_t,
            );
        }
    }
    return op.offset_from(ostart) as ::core::ffi::c_long as size_t;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_compressSuperBlock(
    mut zc: *mut ZSTD_CCtx,
    mut dst: *mut ::core::ffi::c_void,
    mut dstCapacity: size_t,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
    mut lastBlock: ::core::ffi::c_uint,
) -> size_t {
    let mut entropyMetadata: ZSTD_entropyCTablesMetadata_t = ZSTD_entropyCTablesMetadata_t {
        hufMetadata: ZSTD_hufCTablesMetadata_t {
            hType: set_basic,
            hufDesBuffer: [0; 128],
            hufDesSize: 0,
        },
        fseMetadata: ZSTD_fseCTablesMetadata_t {
            llType: set_basic,
            ofType: set_basic,
            mlType: set_basic,
            fseTablesBuffer: [0; 133],
            fseTablesSize: 0,
            lastCountSize: 0,
        },
    };
    let err_code: size_t = ZSTD_buildBlockEntropyStats(
        &raw mut (*zc).seqStore,
        &raw mut (*(*zc).blockState.prevCBlock).entropy,
        &raw mut (*(*zc).blockState.nextCBlock).entropy,
        &raw mut (*zc).appliedParams,
        &raw mut entropyMetadata,
        (*zc).tmpWorkspace,
        (*zc).tmpWkspSize,
    ) as size_t;
    if ERR_isError(err_code) != 0 {
        return err_code;
    }
    return ZSTD_compressSubBlock_multi(
        &raw mut (*zc).seqStore,
        (*zc).blockState.prevCBlock,
        (*zc).blockState.nextCBlock,
        &raw mut entropyMetadata,
        &raw mut (*zc).appliedParams,
        dst,
        dstCapacity,
        src,
        srcSize,
        (*zc).bmi2,
        lastBlock as U32,
        (*zc).tmpWorkspace,
        (*zc).tmpWkspSize,
    );
}
#[inline]
unsafe extern "C" fn ZSTD_getSequenceLength(
    mut seqStore: *const SeqStore_t,
    mut seq: *const SeqDef,
) -> ZSTD_SequenceLength {
    let mut seqLen: ZSTD_SequenceLength = ZSTD_SequenceLength {
        litLength: 0,
        matchLength: 0,
    };
    seqLen.litLength = (*seq).litLength as U32;
    seqLen.matchLength = ((*seq).mlBase as ::core::ffi::c_int + MINMATCH) as U32;
    if (*seqStore).longLengthPos
        == seq.offset_from((*seqStore).sequencesStart) as ::core::ffi::c_long as U32
    {
        if (*seqStore).longLengthType as ::core::ffi::c_uint
            == ZSTD_llt_literalLength as ::core::ffi::c_int as ::core::ffi::c_uint
        {
            seqLen.litLength = (seqLen.litLength as ::core::ffi::c_uint)
                .wrapping_add(0x10000 as ::core::ffi::c_int as ::core::ffi::c_uint)
                as U32 as U32;
        }
        if (*seqStore).longLengthType as ::core::ffi::c_uint
            == ZSTD_llt_matchLength as ::core::ffi::c_int as ::core::ffi::c_uint
        {
            seqLen.matchLength = (seqLen.matchLength as ::core::ffi::c_uint)
                .wrapping_add(0x10000 as ::core::ffi::c_int as ::core::ffi::c_uint)
                as U32 as U32;
        }
    }
    return seqLen;
}
#[inline]
unsafe extern "C" fn ZSTD_noCompressBlock(
    mut dst: *mut ::core::ffi::c_void,
    mut dstCapacity: size_t,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
    mut lastBlock: U32,
) -> size_t {
    let cBlockHeader24: U32 = lastBlock
        .wrapping_add((bt_raw as ::core::ffi::c_int as U32) << 1 as ::core::ffi::c_int)
        .wrapping_add((srcSize << 3 as ::core::ffi::c_int) as U32);
    if srcSize.wrapping_add(ZSTD_blockHeaderSize) > dstCapacity {
        return -(ZSTD_error_dstSize_tooSmall as ::core::ffi::c_int) as size_t;
    }
    MEM_writeLE24(dst, cBlockHeader24);
    ::libc::memcpy(
        (dst as *mut BYTE).offset(ZSTD_blockHeaderSize as isize) as *mut ::core::ffi::c_void,
        src,
        srcSize as ::libc::size_t,
    );
    return ZSTD_blockHeaderSize.wrapping_add(srcSize);
}
#[inline]
unsafe extern "C" fn ZSTD_updateRep(mut rep: *mut U32, offBase: U32, ll0: U32) {
    if offBase > ZSTD_REP_NUM as U32 {
        *rep.offset(2 as ::core::ffi::c_int as isize) =
            *rep.offset(1 as ::core::ffi::c_int as isize);
        *rep.offset(1 as ::core::ffi::c_int as isize) =
            *rep.offset(0 as ::core::ffi::c_int as isize);
        *rep.offset(0 as ::core::ffi::c_int as isize) = (offBase as ::core::ffi::c_uint)
            .wrapping_sub(ZSTD_REP_NUM as ::core::ffi::c_uint)
            as U32;
    } else {
        let repCode: U32 = offBase.wrapping_sub(1 as U32).wrapping_add(ll0);
        if repCode > 0 as U32 {
            let currentOffset: U32 = if repCode == ZSTD_REP_NUM as U32 {
                (*rep.offset(0 as ::core::ffi::c_int as isize)).wrapping_sub(1 as U32)
            } else {
                *rep.offset(repCode as isize)
            };
            *rep.offset(2 as ::core::ffi::c_int as isize) = if repCode >= 2 as U32 {
                *rep.offset(1 as ::core::ffi::c_int as isize)
            } else {
                *rep.offset(2 as ::core::ffi::c_int as isize)
            };
            *rep.offset(1 as ::core::ffi::c_int as isize) =
                *rep.offset(0 as ::core::ffi::c_int as isize);
            *rep.offset(0 as ::core::ffi::c_int as isize) = currentOffset;
        }
    };
}
