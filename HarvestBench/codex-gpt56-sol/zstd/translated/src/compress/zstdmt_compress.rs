use ::c2rust_bitfields;
use ::libc;
extern "C" {
    pub type ZSTD_CDict_s;
    pub type POOL_ctx_s;
    fn malloc(__size: size_t) -> *mut ::core::ffi::c_void;
    fn calloc(__nmemb: size_t, __size: size_t) -> *mut ::core::ffi::c_void;
    fn free(__ptr: *mut ::core::ffi::c_void);
    fn ZSTD_compressBound(srcSize: size_t) -> size_t;
    fn ZSTD_freeCCtx(cctx: *mut ZSTD_CCtx) -> size_t;
    fn ZSTD_freeCDict(CDict: *mut ZSTD_CDict) -> size_t;
    fn ZSTD_sizeof_CCtx(cctx: *const ZSTD_CCtx) -> size_t;
    fn ZSTD_sizeof_CDict(cdict: *const ZSTD_CDict) -> size_t;
    fn ZSTD_createCCtx_advanced(customMem: ZSTD_customMem) -> *mut ZSTD_CCtx;
    fn ZSTD_createCDict_advanced(
        dict: *const ::core::ffi::c_void,
        dictSize: size_t,
        dictLoadMethod: ZSTD_dictLoadMethod_e,
        dictContentType: ZSTD_dictContentType_e,
        cParams: ZSTD_compressionParameters,
        customMem: ZSTD_customMem,
    ) -> *mut ZSTD_CDict;
    fn ZSTD_CCtxParams_setParameter(
        params: *mut ZSTD_CCtx_params,
        param: ZSTD_cParameter,
        value: ::core::ffi::c_int,
    ) -> size_t;
    fn POOL_free(ctx: *mut POOL_ctx);
    fn POOL_resize(ctx: *mut POOL_ctx, numThreads: size_t) -> ::core::ffi::c_int;
    fn POOL_sizeof(ctx: *const POOL_ctx) -> size_t;
    fn POOL_tryAdd(
        ctx: *mut POOL_ctx,
        function: POOL_function,
        opaque: *mut ::core::ffi::c_void,
    ) -> ::core::ffi::c_int;
    fn ZSTD_XXH64_reset(statePtr: *mut XXH64_state_t, seed: XXH64_hash_t) -> XXH_errorcode;
    fn ZSTD_XXH64_update(
        statePtr: *mut XXH64_state_t,
        input: *const ::core::ffi::c_void,
        length: size_t,
    ) -> XXH_errorcode;
    fn ZSTD_XXH64_digest(statePtr: *const XXH64_state_t) -> XXH64_hash_t;
    fn ZSTD_invalidateRepCodes(cctx: *mut ZSTD_CCtx);
    fn ZSTD_getCParamsFromCCtxParams(
        CCtxParams: *const ZSTD_CCtx_params,
        srcSizeHint: U64,
        dictSize: size_t,
        mode: ZSTD_CParamMode_e,
    ) -> ZSTD_compressionParameters;
    fn ZSTD_compressBegin_advanced_internal(
        cctx: *mut ZSTD_CCtx,
        dict: *const ::core::ffi::c_void,
        dictSize: size_t,
        dictContentType: ZSTD_dictContentType_e,
        dtlm: ZSTD_dictTableLoadMethod_e,
        cdict: *const ZSTD_CDict,
        params: *const ZSTD_CCtx_params,
        pledgedSrcSize: ::core::ffi::c_ulonglong,
    ) -> size_t;
    fn ZSTD_writeLastEmptyBlock(dst: *mut ::core::ffi::c_void, dstCapacity: size_t) -> size_t;
    fn ZSTD_referenceExternalSequences(cctx: *mut ZSTD_CCtx, seq: *mut rawSeq, nbSeq: size_t);
    fn ZSTD_cycleLog(hashLog: U32, strat: ZSTD_strategy) -> U32;
    fn ZSTD_CCtx_trace(cctx: *mut ZSTD_CCtx, extraCSize: size_t);
    fn ZSTD_compressContinue_public(
        cctx: *mut ZSTD_CCtx,
        dst: *mut ::core::ffi::c_void,
        dstCapacity: size_t,
        src: *const ::core::ffi::c_void,
        srcSize: size_t,
    ) -> size_t;
    fn ZSTD_compressEnd_public(
        cctx: *mut ZSTD_CCtx,
        dst: *mut ::core::ffi::c_void,
        dstCapacity: size_t,
        src: *const ::core::ffi::c_void,
        srcSize: size_t,
    ) -> size_t;
    fn ZSTD_ldm_fillHashTable(
        state: *mut ldmState_t,
        ip: *const BYTE,
        iend: *const BYTE,
        params: *const ldmParams_t,
    );
    fn ZSTD_ldm_generateSequences(
        ldms: *mut ldmState_t,
        sequences: *mut RawSeqStore_t,
        params: *const ldmParams_t,
        src: *const ::core::ffi::c_void,
        srcSize: size_t,
    ) -> size_t;
    fn ZSTD_ldm_getMaxNbSeq(params: ldmParams_t, maxChunkSize: size_t) -> size_t;
    fn ZSTD_ldm_adjustParameters(
        params: *mut ldmParams_t,
        cParams: *const ZSTD_compressionParameters,
    );
}
pub type size_t = usize;
pub type __uint8_t = u8;
pub type __uint16_t = u16;
pub type __uint32_t = u32;
pub type __uint64_t = u64;
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
pub type ZSTD_cParameter = ::core::ffi::c_uint;
pub const ZSTD_c_experimentalParam20: ZSTD_cParameter = 1017;
pub const ZSTD_c_experimentalParam19: ZSTD_cParameter = 1016;
pub const ZSTD_c_experimentalParam18: ZSTD_cParameter = 1015;
pub const ZSTD_c_experimentalParam17: ZSTD_cParameter = 1014;
pub const ZSTD_c_experimentalParam16: ZSTD_cParameter = 1013;
pub const ZSTD_c_experimentalParam15: ZSTD_cParameter = 1012;
pub const ZSTD_c_experimentalParam14: ZSTD_cParameter = 1011;
pub const ZSTD_c_experimentalParam13: ZSTD_cParameter = 1010;
pub const ZSTD_c_experimentalParam12: ZSTD_cParameter = 1009;
pub const ZSTD_c_experimentalParam11: ZSTD_cParameter = 1008;
pub const ZSTD_c_experimentalParam10: ZSTD_cParameter = 1007;
pub const ZSTD_c_experimentalParam9: ZSTD_cParameter = 1006;
pub const ZSTD_c_experimentalParam8: ZSTD_cParameter = 1005;
pub const ZSTD_c_experimentalParam7: ZSTD_cParameter = 1004;
pub const ZSTD_c_experimentalParam5: ZSTD_cParameter = 1002;
pub const ZSTD_c_experimentalParam4: ZSTD_cParameter = 1001;
pub const ZSTD_c_experimentalParam3: ZSTD_cParameter = 1000;
pub const ZSTD_c_experimentalParam2: ZSTD_cParameter = 10;
pub const ZSTD_c_experimentalParam1: ZSTD_cParameter = 500;
pub const ZSTD_c_overlapLog: ZSTD_cParameter = 402;
pub const ZSTD_c_jobSize: ZSTD_cParameter = 401;
pub const ZSTD_c_nbWorkers: ZSTD_cParameter = 400;
pub const ZSTD_c_dictIDFlag: ZSTD_cParameter = 202;
pub const ZSTD_c_checksumFlag: ZSTD_cParameter = 201;
pub const ZSTD_c_contentSizeFlag: ZSTD_cParameter = 200;
pub const ZSTD_c_ldmHashRateLog: ZSTD_cParameter = 164;
pub const ZSTD_c_ldmBucketSizeLog: ZSTD_cParameter = 163;
pub const ZSTD_c_ldmMinMatch: ZSTD_cParameter = 162;
pub const ZSTD_c_ldmHashLog: ZSTD_cParameter = 161;
pub const ZSTD_c_enableLongDistanceMatching: ZSTD_cParameter = 160;
pub const ZSTD_c_targetCBlockSize: ZSTD_cParameter = 130;
pub const ZSTD_c_strategy: ZSTD_cParameter = 107;
pub const ZSTD_c_targetLength: ZSTD_cParameter = 106;
pub const ZSTD_c_minMatch: ZSTD_cParameter = 105;
pub const ZSTD_c_searchLog: ZSTD_cParameter = 104;
pub const ZSTD_c_chainLog: ZSTD_cParameter = 103;
pub const ZSTD_c_hashLog: ZSTD_cParameter = 102;
pub const ZSTD_c_windowLog: ZSTD_cParameter = 101;
pub const ZSTD_c_compressionLevel: ZSTD_cParameter = 100;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ZSTD_outBuffer_s {
    pub dst: *mut ::core::ffi::c_void,
    pub size: size_t,
    pub pos: size_t,
}
pub type ZSTD_outBuffer = ZSTD_outBuffer_s;
pub type ZSTD_EndDirective = ::core::ffi::c_uint;
pub const ZSTD_e_end: ZSTD_EndDirective = 2;
pub const ZSTD_e_flush: ZSTD_EndDirective = 1;
pub const ZSTD_e_continue: ZSTD_EndDirective = 0;
pub type ZSTD_dictLoadMethod_e = ::core::ffi::c_uint;
pub const ZSTD_dlm_byRef: ZSTD_dictLoadMethod_e = 1;
pub const ZSTD_dlm_byCopy: ZSTD_dictLoadMethod_e = 0;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ZSTD_frameProgression {
    pub ingested: ::core::ffi::c_ulonglong,
    pub consumed: ::core::ffi::c_ulonglong,
    pub produced: ::core::ffi::c_ulonglong,
    pub flushed: ::core::ffi::c_ulonglong,
    pub currentJobID: ::core::ffi::c_uint,
    pub nbActiveWorkers: ::core::ffi::c_uint,
}
pub type unalign32 = U32;
pub type POOL_ctx = POOL_ctx_s;
pub type POOL_function = Option<unsafe extern "C" fn(*mut ::core::ffi::c_void) -> ()>;
pub type ZSTD_pthread_mutex_t = ::core::ffi::c_int;
pub type ZSTD_pthread_cond_t = ::core::ffi::c_int;
pub type XXH_errorcode = ::core::ffi::c_uint;
pub const XXH_ERROR: XXH_errorcode = 1;
pub const XXH_OK: XXH_errorcode = 0;
pub type ZSTD_dictTableLoadMethod_e = ::core::ffi::c_uint;
pub const ZSTD_dtlm_full: ZSTD_dictTableLoadMethod_e = 1;
pub const ZSTD_dtlm_fast: ZSTD_dictTableLoadMethod_e = 0;
pub type ZSTD_CParamMode_e = ::core::ffi::c_uint;
pub const ZSTD_cpm_unknown: ZSTD_CParamMode_e = 3;
pub const ZSTD_cpm_createCDict: ZSTD_CParamMode_e = 2;
pub const ZSTD_cpm_attachDict: ZSTD_CParamMode_e = 1;
pub const ZSTD_cpm_noAttachDict: ZSTD_CParamMode_e = 0;
#[derive(Copy, Clone, BitfieldStruct)]
#[repr(C)]
pub struct ZSTDMT_CCtx_s {
    pub factory: *mut POOL_ctx,
    pub jobs: *mut ZSTDMT_jobDescription,
    pub bufPool: *mut ZSTDMT_bufferPool,
    pub cctxPool: *mut ZSTDMT_CCtxPool,
    pub seqPool: *mut ZSTDMT_seqPool,
    pub params: ZSTD_CCtx_params,
    pub targetSectionSize: size_t,
    pub targetPrefixSize: size_t,
    pub jobReady: ::core::ffi::c_int,
    pub inBuff: InBuff_t,
    pub roundBuff: RoundBuff_t,
    pub serial: SerialState,
    pub rsync: RSyncState_t,
    pub jobIDMask: ::core::ffi::c_uint,
    pub doneJobID: ::core::ffi::c_uint,
    pub nextJobID: ::core::ffi::c_uint,
    pub frameEnded: ::core::ffi::c_uint,
    pub allJobsCompleted: ::core::ffi::c_uint,
    pub frameContentSize: ::core::ffi::c_ulonglong,
    pub consumed: ::core::ffi::c_ulonglong,
    pub produced: ::core::ffi::c_ulonglong,
    pub cMem: ZSTD_customMem,
    pub cdictLocal: *mut ZSTD_CDict,
    pub cdict: *const ZSTD_CDict,
    #[bitfield(name = "providedFactory", ty = "::core::ffi::c_uint", bits = "0..=0")]
    pub providedFactory: [u8; 1],
    #[bitfield(padding)]
    pub c2rust_padding: [u8; 7],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct RSyncState_t {
    pub hash: U64,
    pub hitMask: U64,
    pub primePower: U64,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct SerialState {
    pub mutex: ZSTD_pthread_mutex_t,
    pub cond: ZSTD_pthread_cond_t,
    pub params: ZSTD_CCtx_params,
    pub ldmState: ldmState_t,
    pub xxhState: XXH64_state_t,
    pub nextJobID: ::core::ffi::c_uint,
    pub ldmWindowMutex: ZSTD_pthread_mutex_t,
    pub ldmWindowCond: ZSTD_pthread_cond_t,
    pub ldmWindow: ZSTD_window_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct RoundBuff_t {
    pub buffer: *mut BYTE,
    pub capacity: size_t,
    pub pos: size_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct InBuff_t {
    pub prefix: Range,
    pub buffer: Buffer,
    pub filled: size_t,
}
pub type Buffer = buffer_s;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct buffer_s {
    pub start: *mut ::core::ffi::c_void,
    pub capacity: size_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct Range {
    pub start: *const ::core::ffi::c_void,
    pub size: size_t,
}
pub type ZSTDMT_seqPool = ZSTDMT_bufferPool;
pub type ZSTDMT_bufferPool = ZSTDMT_bufferPool_s;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ZSTDMT_bufferPool_s {
    pub poolMutex: ZSTD_pthread_mutex_t,
    pub bufferSize: size_t,
    pub totalBuffers: ::core::ffi::c_uint,
    pub nbBuffers: ::core::ffi::c_uint,
    pub cMem: ZSTD_customMem,
    pub buffers: *mut Buffer,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ZSTDMT_CCtxPool {
    pub poolMutex: ZSTD_pthread_mutex_t,
    pub totalCCtx: ::core::ffi::c_int,
    pub availCCtx: ::core::ffi::c_int,
    pub cMem: ZSTD_customMem,
    pub cctxs: *mut *mut ZSTD_CCtx,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ZSTDMT_jobDescription {
    pub consumed: size_t,
    pub cSize: size_t,
    pub job_mutex: ZSTD_pthread_mutex_t,
    pub job_cond: ZSTD_pthread_cond_t,
    pub cctxPool: *mut ZSTDMT_CCtxPool,
    pub bufPool: *mut ZSTDMT_bufferPool,
    pub seqPool: *mut ZSTDMT_seqPool,
    pub serial: *mut SerialState,
    pub dstBuff: Buffer,
    pub prefix: Range,
    pub src: Range,
    pub jobID: ::core::ffi::c_uint,
    pub firstJob: ::core::ffi::c_uint,
    pub lastJob: ::core::ffi::c_uint,
    pub params: ZSTD_CCtx_params,
    pub cdict: *const ZSTD_CDict,
    pub fullFrameSize: ::core::ffi::c_ulonglong,
    pub dstFlushed: size_t,
    pub frameChecksumNeeded: ::core::ffi::c_uint,
}
pub type ZSTDMT_CCtx = ZSTDMT_CCtx_s;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct SyncPoint {
    pub toLoad: size_t,
    pub flush: ::core::ffi::c_int,
}
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
pub const ZSTD_BLOCKSIZELOG_MAX: ::core::ffi::c_int = 17 as ::core::ffi::c_int;
pub const ZSTD_BLOCKSIZE_MAX: ::core::ffi::c_int =
    (1 as ::core::ffi::c_int) << ZSTD_BLOCKSIZELOG_MAX;
pub const ZSTD_CONTENTSIZE_UNKNOWN: ::core::ffi::c_ulonglong =
    (0 as ::core::ffi::c_ulonglong).wrapping_sub(1 as ::core::ffi::c_ulonglong);
#[inline]
unsafe extern "C" fn ZSTD_customMalloc(
    mut size: size_t,
    mut customMem: ZSTD_customMem,
) -> *mut ::core::ffi::c_void {
    if customMem.customAlloc.is_some() {
        return customMem.customAlloc.expect("non-null function pointer")(customMem.opaque, size);
    }
    return malloc(size);
}
#[inline]
unsafe extern "C" fn ZSTD_customCalloc(
    mut size: size_t,
    mut customMem: ZSTD_customMem,
) -> *mut ::core::ffi::c_void {
    if customMem.customAlloc.is_some() {
        let ptr: *mut ::core::ffi::c_void =
            customMem.customAlloc.expect("non-null function pointer")(customMem.opaque, size)
                as *mut ::core::ffi::c_void;
        ::libc::memset(ptr, 0 as ::core::ffi::c_int, size as ::libc::size_t);
        return ptr;
    }
    return calloc(1 as size_t, size);
}
#[inline]
unsafe extern "C" fn ZSTD_customFree(
    mut ptr: *mut ::core::ffi::c_void,
    mut customMem: ZSTD_customMem,
) {
    if !ptr.is_null() {
        if customMem.customFree.is_some() {
            customMem.customFree.expect("non-null function pointer")(customMem.opaque, ptr);
        } else {
            free(ptr);
        }
    }
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
unsafe extern "C" fn MEM_write32(mut memPtr: *mut ::core::ffi::c_void, mut value: U32) {
    *(memPtr as *mut unalign32) = value as unalign32;
}
#[inline]
unsafe extern "C" fn MEM_swap32(mut in_0: U32) -> U32 {
    return in_0.swap_bytes();
}
#[inline]
unsafe extern "C" fn MEM_writeLE32(mut memPtr: *mut ::core::ffi::c_void, mut val32: U32) {
    if MEM_isLittleEndian() != 0 {
        MEM_write32(memPtr, val32);
    } else {
        MEM_write32(memPtr, MEM_swap32(val32));
    };
}
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
unsafe extern "C" fn ZSTD_countLeadingZeros32(mut val: U32) -> ::core::ffi::c_uint {
    return val.leading_zeros() as i32 as ::core::ffi::c_uint;
}
#[inline]
unsafe extern "C" fn ZSTD_highbit32(mut val: U32) -> ::core::ffi::c_uint {
    return (31 as ::core::ffi::c_uint).wrapping_sub(ZSTD_countLeadingZeros32(val));
}
pub const HASH_READ_SIZE: ::core::ffi::c_int = 8 as ::core::ffi::c_int;
static mut kNullRawSeqStore: RawSeqStore_t = RawSeqStore_t {
    seq: ::core::ptr::null::<rawSeq>() as *mut rawSeq,
    pos: 0 as size_t,
    posInSequence: 0 as size_t,
    size: 0 as size_t,
    capacity: 0 as size_t,
};
pub const ZSTD_WINDOW_START_INDEX: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
static mut prime8bytes: U64 = 0xcf1bbcdcb7a56463 as U64;
unsafe extern "C" fn ZSTD_ipow(mut base: U64, mut exponent: U64) -> U64 {
    let mut power: U64 = 1 as U64;
    while exponent != 0 {
        if exponent & 1 as U64 != 0 {
            power = (power as ::core::ffi::c_ulong).wrapping_mul(base as ::core::ffi::c_ulong)
                as U64 as U64;
        }
        exponent >>= 1 as ::core::ffi::c_int;
        base =
            (base as ::core::ffi::c_ulong).wrapping_mul(base as ::core::ffi::c_ulong) as U64 as U64;
    }
    return power;
}
pub const ZSTD_ROLL_HASH_CHAR_OFFSET: ::core::ffi::c_int = 10 as ::core::ffi::c_int;
unsafe extern "C" fn ZSTD_rollingHash_append(
    mut hash: U64,
    mut buf: *const ::core::ffi::c_void,
    mut size: size_t,
) -> U64 {
    let mut istart: *const BYTE = buf as *const BYTE;
    let mut pos: size_t = 0;
    pos = 0 as size_t;
    while pos < size {
        hash = (hash as ::core::ffi::c_ulong).wrapping_mul(prime8bytes as ::core::ffi::c_ulong)
            as U64 as U64;
        hash = (hash as ::core::ffi::c_ulong).wrapping_add(
            (*istart.offset(pos as isize) as ::core::ffi::c_int + ZSTD_ROLL_HASH_CHAR_OFFSET)
                as ::core::ffi::c_ulong,
        ) as U64 as U64;
        pos = pos.wrapping_add(1);
    }
    return hash;
}
#[inline]
unsafe extern "C" fn ZSTD_rollingHash_compute(
    mut buf: *const ::core::ffi::c_void,
    mut size: size_t,
) -> U64 {
    return ZSTD_rollingHash_append(0 as U64, buf, size);
}
#[inline]
unsafe extern "C" fn ZSTD_rollingHash_primePower(mut length: U32) -> U64 {
    return ZSTD_ipow(prime8bytes, length.wrapping_sub(1 as U32) as U64);
}
#[inline]
unsafe extern "C" fn ZSTD_rollingHash_rotate(
    mut hash: U64,
    mut toRemove: BYTE,
    mut toAdd: BYTE,
    mut primePower: U64,
) -> U64 {
    hash = (hash as ::core::ffi::c_ulong).wrapping_sub(
        ((toRemove as ::core::ffi::c_int + ZSTD_ROLL_HASH_CHAR_OFFSET) as U64)
            .wrapping_mul(primePower) as ::core::ffi::c_ulong,
    ) as U64 as U64;
    hash = (hash as ::core::ffi::c_ulong).wrapping_mul(prime8bytes as ::core::ffi::c_ulong) as U64
        as U64;
    hash = (hash as ::core::ffi::c_ulong).wrapping_add(
        (toAdd as ::core::ffi::c_int + ZSTD_ROLL_HASH_CHAR_OFFSET) as ::core::ffi::c_ulong,
    ) as U64 as U64;
    return hash;
}
#[inline]
unsafe extern "C" fn ZSTD_window_clear(mut window: *mut ZSTD_window_t) {
    let endT: size_t =
        (*window).nextSrc.offset_from((*window).base) as ::core::ffi::c_long as size_t;
    let end: U32 = endT as U32;
    (*window).lowLimit = end;
    (*window).dictLimit = end;
}
#[inline]
unsafe extern "C" fn ZSTD_window_init(mut window: *mut ZSTD_window_t) {
    ::libc::memset(
        window as *mut ::core::ffi::c_void,
        0 as ::core::ffi::c_int,
        ::core::mem::size_of::<ZSTD_window_t>() as ::libc::size_t,
    );
    (*window).base = b" \0" as *const u8 as *const ::core::ffi::c_char as *const BYTE;
    (*window).dictBase = b" \0" as *const u8 as *const ::core::ffi::c_char as *const BYTE;
    (*window).dictLimit = ZSTD_WINDOW_START_INDEX as U32;
    (*window).lowLimit = ZSTD_WINDOW_START_INDEX as U32;
    (*window).nextSrc = (*window).base.offset(ZSTD_WINDOW_START_INDEX as isize);
    (*window).nbOverflowCorrections = 0 as U32;
}
#[inline]
unsafe extern "C" fn ZSTD_window_update(
    mut window: *mut ZSTD_window_t,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
    mut forceNonContiguous: ::core::ffi::c_int,
) -> U32 {
    let ip: *const BYTE = src as *const BYTE;
    let mut contiguous: U32 = 1 as U32;
    if srcSize == 0 as size_t {
        return contiguous;
    }
    if src != (*window).nextSrc as *const ::core::ffi::c_void || forceNonContiguous != 0 {
        let distanceFromBase: size_t =
            (*window).nextSrc.offset_from((*window).base) as ::core::ffi::c_long as size_t;
        (*window).lowLimit = (*window).dictLimit;
        (*window).dictLimit = distanceFromBase as U32;
        (*window).dictBase = (*window).base;
        (*window).base = ip.offset(-(distanceFromBase as isize));
        if (*window).dictLimit.wrapping_sub((*window).lowLimit) < HASH_READ_SIZE as U32 {
            (*window).lowLimit = (*window).dictLimit;
        }
        contiguous = 0 as U32;
    }
    (*window).nextSrc = ip.offset(srcSize as isize);
    if (ip.offset(srcSize as isize) > (*window).dictBase.offset((*window).lowLimit as isize))
        as ::core::ffi::c_int
        & (ip < (*window).dictBase.offset((*window).dictLimit as isize)) as ::core::ffi::c_int
        != 0
    {
        let highInputIdx: size_t = ip.offset(srcSize as isize).offset_from((*window).dictBase)
            as ::core::ffi::c_long as size_t;
        let lowLimitMax: U32 = if highInputIdx > (*window).dictLimit as size_t {
            (*window).dictLimit
        } else {
            highInputIdx as U32
        };
        (*window).lowLimit = lowLimitMax;
    }
    return contiguous;
}
pub const ZSTDMT_JOBSIZE_MIN: ::core::ffi::c_int =
    512 as ::core::ffi::c_int * ((1 as ::core::ffi::c_int) << 10 as ::core::ffi::c_int);
static mut g_nullBuffer: Buffer = buffer_s {
    start: NULL,
    capacity: 0 as size_t,
};
unsafe extern "C" fn ZSTDMT_freeBufferPool(mut bufPool: *mut ZSTDMT_bufferPool) {
    if bufPool.is_null() {
        return;
    }
    if !(*bufPool).buffers.is_null() {
        let mut u: ::core::ffi::c_uint = 0;
        u = 0 as ::core::ffi::c_uint;
        while u < (*bufPool).totalBuffers {
            ZSTD_customFree(
                (*(*bufPool).buffers.offset(u as isize)).start,
                (*bufPool).cMem,
            );
            u = u.wrapping_add(1);
        }
        ZSTD_customFree(
            (*bufPool).buffers as *mut ::core::ffi::c_void,
            (*bufPool).cMem,
        );
    }
    &raw mut (*bufPool).poolMutex;
    ZSTD_customFree(bufPool as *mut ::core::ffi::c_void, (*bufPool).cMem);
}
unsafe extern "C" fn ZSTDMT_createBufferPool(
    mut maxNbBuffers: ::core::ffi::c_uint,
    mut cMem: ZSTD_customMem,
) -> *mut ZSTDMT_bufferPool {
    let bufPool: *mut ZSTDMT_bufferPool =
        ZSTD_customCalloc(::core::mem::size_of::<ZSTDMT_bufferPool>() as size_t, cMem)
            as *mut ZSTDMT_bufferPool;
    if bufPool.is_null() {
        return ::core::ptr::null_mut::<ZSTDMT_bufferPool>();
    }
    &raw mut (*bufPool).poolMutex;
    if 0 as ::core::ffi::c_int != 0 {
        ZSTD_customFree(bufPool as *mut ::core::ffi::c_void, cMem);
        return ::core::ptr::null_mut::<ZSTDMT_bufferPool>();
    }
    (*bufPool).buffers = ZSTD_customCalloc(
        (maxNbBuffers as size_t).wrapping_mul(::core::mem::size_of::<Buffer>() as size_t),
        cMem,
    ) as *mut Buffer;
    if (*bufPool).buffers.is_null() {
        ZSTDMT_freeBufferPool(bufPool);
        return ::core::ptr::null_mut::<ZSTDMT_bufferPool>();
    }
    (*bufPool).bufferSize = (64 as ::core::ffi::c_int
        * ((1 as ::core::ffi::c_int) << 10 as ::core::ffi::c_int))
        as size_t;
    (*bufPool).totalBuffers = maxNbBuffers;
    (*bufPool).nbBuffers = 0 as ::core::ffi::c_uint;
    (*bufPool).cMem = cMem;
    return bufPool;
}
unsafe extern "C" fn ZSTDMT_sizeof_bufferPool(mut bufPool: *mut ZSTDMT_bufferPool) -> size_t {
    let poolSize: size_t = ::core::mem::size_of::<ZSTDMT_bufferPool>() as size_t;
    let arraySize: size_t = ((*bufPool).totalBuffers as size_t)
        .wrapping_mul(::core::mem::size_of::<Buffer>() as size_t);
    let mut u: ::core::ffi::c_uint = 0;
    let mut totalBufferSize: size_t = 0 as size_t;
    &raw mut (*bufPool).poolMutex;
    u = 0 as ::core::ffi::c_uint;
    while u < (*bufPool).totalBuffers {
        totalBufferSize = (totalBufferSize as ::core::ffi::c_ulong)
            .wrapping_add((*(*bufPool).buffers.offset(u as isize)).capacity as ::core::ffi::c_ulong)
            as size_t as size_t;
        u = u.wrapping_add(1);
    }
    &raw mut (*bufPool).poolMutex;
    return poolSize
        .wrapping_add(arraySize)
        .wrapping_add(totalBufferSize);
}
unsafe extern "C" fn ZSTDMT_setBufferSize(bufPool: *mut ZSTDMT_bufferPool, bSize: size_t) {
    &raw mut (*bufPool).poolMutex;
    (*bufPool).bufferSize = bSize;
    &raw mut (*bufPool).poolMutex;
}
unsafe extern "C" fn ZSTDMT_expandBufferPool(
    mut srcBufPool: *mut ZSTDMT_bufferPool,
    mut maxNbBuffers: ::core::ffi::c_uint,
) -> *mut ZSTDMT_bufferPool {
    if srcBufPool.is_null() {
        return ::core::ptr::null_mut::<ZSTDMT_bufferPool>();
    }
    if (*srcBufPool).totalBuffers >= maxNbBuffers {
        return srcBufPool;
    }
    let cMem: ZSTD_customMem = (*srcBufPool).cMem;
    let bSize: size_t = (*srcBufPool).bufferSize;
    let mut newBufPool: *mut ZSTDMT_bufferPool = ::core::ptr::null_mut::<ZSTDMT_bufferPool>();
    ZSTDMT_freeBufferPool(srcBufPool);
    newBufPool = ZSTDMT_createBufferPool(maxNbBuffers, cMem);
    if newBufPool.is_null() {
        return newBufPool;
    }
    ZSTDMT_setBufferSize(newBufPool, bSize);
    return newBufPool;
}
unsafe extern "C" fn ZSTDMT_getBuffer(mut bufPool: *mut ZSTDMT_bufferPool) -> Buffer {
    let bSize: size_t = (*bufPool).bufferSize;
    &raw mut (*bufPool).poolMutex;
    if (*bufPool).nbBuffers != 0 {
        (*bufPool).nbBuffers = (*bufPool).nbBuffers.wrapping_sub(1);
        let buf: Buffer = *(*bufPool).buffers.offset((*bufPool).nbBuffers as isize);
        let availBufferSize: size_t = buf.capacity;
        *(*bufPool).buffers.offset((*bufPool).nbBuffers as isize) = g_nullBuffer;
        if (availBufferSize >= bSize) as ::core::ffi::c_int
            & (availBufferSize >> 3 as ::core::ffi::c_int <= bSize) as ::core::ffi::c_int
            != 0
        {
            &raw mut (*bufPool).poolMutex;
            return buf;
        }
        ZSTD_customFree(buf.start, (*bufPool).cMem);
    }
    &raw mut (*bufPool).poolMutex;
    let mut buffer: Buffer = Buffer {
        start: ::core::ptr::null_mut::<::core::ffi::c_void>(),
        capacity: 0,
    };
    let start: *mut ::core::ffi::c_void =
        ZSTD_customMalloc(bSize, (*bufPool).cMem) as *mut ::core::ffi::c_void;
    buffer.start = start;
    buffer.capacity = if start.is_null() { 0 as size_t } else { bSize };
    start.is_null();
    return buffer;
}
unsafe extern "C" fn ZSTDMT_releaseBuffer(mut bufPool: *mut ZSTDMT_bufferPool, mut buf: Buffer) {
    if buf.start.is_null() {
        return;
    }
    &raw mut (*bufPool).poolMutex;
    if (*bufPool).nbBuffers < (*bufPool).totalBuffers {
        let fresh0 = (*bufPool).nbBuffers;
        (*bufPool).nbBuffers = (*bufPool).nbBuffers.wrapping_add(1);
        *(*bufPool).buffers.offset(fresh0 as isize) = buf;
        &raw mut (*bufPool).poolMutex;
        return;
    }
    &raw mut (*bufPool).poolMutex;
    ZSTD_customFree(buf.start, (*bufPool).cMem);
}
unsafe extern "C" fn ZSTDMT_sizeof_seqPool(mut seqPool: *mut ZSTDMT_seqPool) -> size_t {
    return ZSTDMT_sizeof_bufferPool(seqPool as *mut ZSTDMT_bufferPool);
}
unsafe extern "C" fn bufferToSeq(mut buffer: Buffer) -> RawSeqStore_t {
    let mut seq: RawSeqStore_t = kNullRawSeqStore;
    seq.seq = buffer.start as *mut rawSeq;
    seq.capacity = buffer
        .capacity
        .wrapping_div(::core::mem::size_of::<rawSeq>() as size_t);
    return seq;
}
unsafe extern "C" fn seqToBuffer(mut seq: RawSeqStore_t) -> Buffer {
    let mut buffer: Buffer = Buffer {
        start: ::core::ptr::null_mut::<::core::ffi::c_void>(),
        capacity: 0,
    };
    buffer.start = seq.seq as *mut ::core::ffi::c_void;
    buffer.capacity = seq
        .capacity
        .wrapping_mul(::core::mem::size_of::<rawSeq>() as size_t);
    return buffer;
}
unsafe extern "C" fn ZSTDMT_getSeq(mut seqPool: *mut ZSTDMT_seqPool) -> RawSeqStore_t {
    if (*seqPool).bufferSize == 0 as size_t {
        return kNullRawSeqStore;
    }
    return bufferToSeq(ZSTDMT_getBuffer(seqPool as *mut ZSTDMT_bufferPool));
}
unsafe extern "C" fn ZSTDMT_releaseSeq(mut seqPool: *mut ZSTDMT_seqPool, mut seq: RawSeqStore_t) {
    ZSTDMT_releaseBuffer(seqPool as *mut ZSTDMT_bufferPool, seqToBuffer(seq));
}
unsafe extern "C" fn ZSTDMT_setNbSeq(seqPool: *mut ZSTDMT_seqPool, nbSeq: size_t) {
    ZSTDMT_setBufferSize(
        seqPool as *mut ZSTDMT_bufferPool,
        nbSeq.wrapping_mul(::core::mem::size_of::<rawSeq>() as size_t),
    );
}
unsafe extern "C" fn ZSTDMT_freeSeqPool(mut seqPool: *mut ZSTDMT_seqPool) {
    ZSTDMT_freeBufferPool(seqPool as *mut ZSTDMT_bufferPool);
}
unsafe extern "C" fn ZSTDMT_expandSeqPool(
    mut pool: *mut ZSTDMT_seqPool,
    mut nbWorkers: U32,
) -> *mut ZSTDMT_seqPool {
    return ZSTDMT_expandBufferPool(
        pool as *mut ZSTDMT_bufferPool,
        nbWorkers as ::core::ffi::c_uint,
    ) as *mut ZSTDMT_seqPool;
}
unsafe extern "C" fn ZSTDMT_freeCCtxPool(mut pool: *mut ZSTDMT_CCtxPool) {
    if pool.is_null() {
        return;
    }
    &raw mut (*pool).poolMutex;
    if !(*pool).cctxs.is_null() {
        let mut cid: ::core::ffi::c_int = 0;
        cid = 0 as ::core::ffi::c_int;
        while cid < (*pool).totalCCtx {
            ZSTD_freeCCtx(*(*pool).cctxs.offset(cid as isize));
            cid += 1;
        }
        ZSTD_customFree((*pool).cctxs as *mut ::core::ffi::c_void, (*pool).cMem);
    }
    ZSTD_customFree(pool as *mut ::core::ffi::c_void, (*pool).cMem);
}
unsafe extern "C" fn ZSTDMT_createCCtxPool(
    mut nbWorkers: ::core::ffi::c_int,
    mut cMem: ZSTD_customMem,
) -> *mut ZSTDMT_CCtxPool {
    let cctxPool: *mut ZSTDMT_CCtxPool =
        ZSTD_customCalloc(::core::mem::size_of::<ZSTDMT_CCtxPool>() as size_t, cMem)
            as *mut ZSTDMT_CCtxPool;
    if cctxPool.is_null() {
        return ::core::ptr::null_mut::<ZSTDMT_CCtxPool>();
    }
    &raw mut (*cctxPool).poolMutex;
    if 0 as ::core::ffi::c_int != 0 {
        ZSTD_customFree(cctxPool as *mut ::core::ffi::c_void, cMem);
        return ::core::ptr::null_mut::<ZSTDMT_CCtxPool>();
    }
    (*cctxPool).totalCCtx = nbWorkers;
    (*cctxPool).cctxs = ZSTD_customCalloc(
        (nbWorkers as size_t).wrapping_mul(::core::mem::size_of::<*mut ZSTD_CCtx>() as size_t),
        cMem,
    ) as *mut *mut ZSTD_CCtx;
    if (*cctxPool).cctxs.is_null() {
        ZSTDMT_freeCCtxPool(cctxPool);
        return ::core::ptr::null_mut::<ZSTDMT_CCtxPool>();
    }
    (*cctxPool).cMem = cMem;
    let ref mut fresh1 = *(*cctxPool).cctxs.offset(0 as ::core::ffi::c_int as isize);
    *fresh1 = ZSTD_createCCtx_advanced(cMem);
    if (*(*cctxPool).cctxs.offset(0 as ::core::ffi::c_int as isize)).is_null() {
        ZSTDMT_freeCCtxPool(cctxPool);
        return ::core::ptr::null_mut::<ZSTDMT_CCtxPool>();
    }
    (*cctxPool).availCCtx = 1 as ::core::ffi::c_int;
    return cctxPool;
}
unsafe extern "C" fn ZSTDMT_expandCCtxPool(
    mut srcPool: *mut ZSTDMT_CCtxPool,
    mut nbWorkers: ::core::ffi::c_int,
) -> *mut ZSTDMT_CCtxPool {
    if srcPool.is_null() {
        return ::core::ptr::null_mut::<ZSTDMT_CCtxPool>();
    }
    if nbWorkers <= (*srcPool).totalCCtx {
        return srcPool;
    }
    let cMem: ZSTD_customMem = (*srcPool).cMem;
    ZSTDMT_freeCCtxPool(srcPool);
    return ZSTDMT_createCCtxPool(nbWorkers, cMem);
}
unsafe extern "C" fn ZSTDMT_sizeof_CCtxPool(mut cctxPool: *mut ZSTDMT_CCtxPool) -> size_t {
    &raw mut (*cctxPool).poolMutex;
    let nbWorkers: ::core::ffi::c_uint = (*cctxPool).totalCCtx as ::core::ffi::c_uint;
    let poolSize: size_t = ::core::mem::size_of::<ZSTDMT_CCtxPool>() as size_t;
    let arraySize: size_t = ((*cctxPool).totalCCtx as size_t)
        .wrapping_mul(::core::mem::size_of::<*mut ZSTD_CCtx>() as size_t);
    let mut totalCCtxSize: size_t = 0 as size_t;
    let mut u: ::core::ffi::c_uint = 0;
    u = 0 as ::core::ffi::c_uint;
    while u < nbWorkers {
        totalCCtxSize = (totalCCtxSize as ::core::ffi::c_ulong)
            .wrapping_add(
                ZSTD_sizeof_CCtx(*(*cctxPool).cctxs.offset(u as isize)) as ::core::ffi::c_ulong
            ) as size_t as size_t;
        u = u.wrapping_add(1);
    }
    &raw mut (*cctxPool).poolMutex;
    return poolSize.wrapping_add(arraySize).wrapping_add(totalCCtxSize);
}
unsafe extern "C" fn ZSTDMT_getCCtx(mut cctxPool: *mut ZSTDMT_CCtxPool) -> *mut ZSTD_CCtx {
    &raw mut (*cctxPool).poolMutex;
    if (*cctxPool).availCCtx != 0 {
        (*cctxPool).availCCtx -= 1;
        let cctx: *mut ZSTD_CCtx = *(*cctxPool).cctxs.offset((*cctxPool).availCCtx as isize);
        &raw mut (*cctxPool).poolMutex;
        return cctx;
    }
    &raw mut (*cctxPool).poolMutex;
    return ZSTD_createCCtx_advanced((*cctxPool).cMem);
}
unsafe extern "C" fn ZSTDMT_releaseCCtx(mut pool: *mut ZSTDMT_CCtxPool, mut cctx: *mut ZSTD_CCtx) {
    if cctx.is_null() {
        return;
    }
    &raw mut (*pool).poolMutex;
    if (*pool).availCCtx < (*pool).totalCCtx {
        let fresh10 = (*pool).availCCtx;
        (*pool).availCCtx = (*pool).availCCtx + 1;
        let ref mut fresh11 = *(*pool).cctxs.offset(fresh10 as isize);
        *fresh11 = cctx;
    } else {
        ZSTD_freeCCtx(cctx);
    }
    &raw mut (*pool).poolMutex;
}
unsafe extern "C" fn ZSTDMT_serialState_reset(
    mut serialState: *mut SerialState,
    mut seqPool: *mut ZSTDMT_seqPool,
    mut params: ZSTD_CCtx_params,
    mut jobSize: size_t,
    mut dict: *const ::core::ffi::c_void,
    dictSize: size_t,
    mut dictContentType: ZSTD_dictContentType_e,
) -> ::core::ffi::c_int {
    if params.ldmParams.enableLdm as ::core::ffi::c_uint
        == ZSTD_ps_enable as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        ZSTD_ldm_adjustParameters(&raw mut params.ldmParams, &raw mut params.cParams);
    } else {
        ::libc::memset(
            &raw mut params.ldmParams as *mut ::core::ffi::c_void,
            0 as ::core::ffi::c_int,
            ::core::mem::size_of::<ldmParams_t>() as ::libc::size_t,
        );
    }
    (*serialState).nextJobID = 0 as ::core::ffi::c_uint;
    if params.fParams.checksumFlag != 0 {
        ZSTD_XXH64_reset(&raw mut (*serialState).xxhState, 0 as XXH64_hash_t);
    }
    if params.ldmParams.enableLdm as ::core::ffi::c_uint
        == ZSTD_ps_enable as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        let mut cMem: ZSTD_customMem = params.customMem;
        let hashLog: ::core::ffi::c_uint = params.ldmParams.hashLog as ::core::ffi::c_uint;
        let hashSize: size_t = ((1 as ::core::ffi::c_int as size_t) << hashLog)
            .wrapping_mul(::core::mem::size_of::<ldmEntry_t>() as size_t);
        let bucketLog: ::core::ffi::c_uint = (params.ldmParams.hashLog as ::core::ffi::c_uint)
            .wrapping_sub(params.ldmParams.bucketSizeLog as ::core::ffi::c_uint);
        let prevBucketLog: ::core::ffi::c_uint = ((*serialState).params.ldmParams.hashLog
            as ::core::ffi::c_uint)
            .wrapping_sub((*serialState).params.ldmParams.bucketSizeLog as ::core::ffi::c_uint);
        let numBuckets: size_t = (1 as ::core::ffi::c_int as size_t) << bucketLog;
        ZSTDMT_setNbSeq(seqPool, ZSTD_ldm_getMaxNbSeq(params.ldmParams, jobSize));
        ZSTD_window_init(&raw mut (*serialState).ldmState.window);
        if (*serialState).ldmState.hashTable.is_null()
            || (*serialState).params.ldmParams.hashLog < hashLog as U32
        {
            ZSTD_customFree(
                (*serialState).ldmState.hashTable as *mut ::core::ffi::c_void,
                cMem,
            );
            (*serialState).ldmState.hashTable =
                ZSTD_customMalloc(hashSize, cMem) as *mut ldmEntry_t;
        }
        if (*serialState).ldmState.bucketOffsets.is_null() || prevBucketLog < bucketLog {
            ZSTD_customFree(
                (*serialState).ldmState.bucketOffsets as *mut ::core::ffi::c_void,
                cMem,
            );
            (*serialState).ldmState.bucketOffsets =
                ZSTD_customMalloc(numBuckets, cMem) as *mut BYTE;
        }
        if (*serialState).ldmState.hashTable.is_null()
            || (*serialState).ldmState.bucketOffsets.is_null()
        {
            return 1 as ::core::ffi::c_int;
        }
        ::libc::memset(
            (*serialState).ldmState.hashTable as *mut ::core::ffi::c_void,
            0 as ::core::ffi::c_int,
            hashSize as ::libc::size_t,
        );
        ::libc::memset(
            (*serialState).ldmState.bucketOffsets as *mut ::core::ffi::c_void,
            0 as ::core::ffi::c_int,
            numBuckets as ::libc::size_t,
        );
        (*serialState).ldmState.loadedDictEnd = 0 as U32;
        if dictSize > 0 as size_t {
            if dictContentType as ::core::ffi::c_uint
                == ZSTD_dct_rawContent as ::core::ffi::c_int as ::core::ffi::c_uint
            {
                let dictEnd: *const BYTE = (dict as *const BYTE).offset(dictSize as isize);
                ZSTD_window_update(
                    &raw mut (*serialState).ldmState.window,
                    dict,
                    dictSize,
                    0 as ::core::ffi::c_int,
                );
                ZSTD_ldm_fillHashTable(
                    &raw mut (*serialState).ldmState,
                    dict as *const BYTE,
                    dictEnd,
                    &raw mut params.ldmParams,
                );
                (*serialState).ldmState.loadedDictEnd = if params.forceWindow != 0 {
                    0 as U32
                } else {
                    dictEnd.offset_from((*serialState).ldmState.window.base) as ::core::ffi::c_long
                        as U32
                };
            }
        }
        (*serialState).ldmWindow = (*serialState).ldmState.window;
    }
    (*serialState).params = params;
    (*serialState).params.jobSize = jobSize as U32 as size_t;
    return 0 as ::core::ffi::c_int;
}
unsafe extern "C" fn ZSTDMT_serialState_free(mut serialState: *mut SerialState) {
    let mut cMem: ZSTD_customMem = (*serialState).params.customMem;
    &raw mut (*serialState).mutex;
    &raw mut (*serialState).cond;
    &raw mut (*serialState).ldmWindowMutex;
    &raw mut (*serialState).ldmWindowCond;
    ZSTD_customFree(
        (*serialState).ldmState.hashTable as *mut ::core::ffi::c_void,
        cMem,
    );
    ZSTD_customFree(
        (*serialState).ldmState.bucketOffsets as *mut ::core::ffi::c_void,
        cMem,
    );
}
unsafe extern "C" fn ZSTDMT_serialState_genSequences(
    mut serialState: *mut SerialState,
    mut seqStore: *mut RawSeqStore_t,
    mut src: Range,
    mut jobID: ::core::ffi::c_uint,
) {
    &raw mut (*serialState).mutex;
    while (*serialState).nextJobID < jobID {
        &raw mut (*serialState).cond;
        &raw mut (*serialState).mutex;
    }
    if (*serialState).nextJobID == jobID {
        if (*serialState).params.ldmParams.enableLdm as ::core::ffi::c_uint
            == ZSTD_ps_enable as ::core::ffi::c_int as ::core::ffi::c_uint
        {
            let mut error: size_t = 0;
            ZSTD_window_update(
                &raw mut (*serialState).ldmState.window,
                src.start,
                src.size,
                0 as ::core::ffi::c_int,
            );
            error = ZSTD_ldm_generateSequences(
                &raw mut (*serialState).ldmState,
                seqStore,
                &raw mut (*serialState).params.ldmParams,
                src.start,
                src.size,
            );
            &raw mut (*serialState).ldmWindowMutex;
            (*serialState).ldmWindow = (*serialState).ldmState.window;
            &raw mut (*serialState).ldmWindowCond;
            &raw mut (*serialState).ldmWindowMutex;
        }
        if (*serialState).params.fParams.checksumFlag != 0 && src.size > 0 as size_t {
            ZSTD_XXH64_update(&raw mut (*serialState).xxhState, src.start, src.size);
        }
    }
    (*serialState).nextJobID = (*serialState).nextJobID.wrapping_add(1);
    &raw mut (*serialState).cond;
    &raw mut (*serialState).mutex;
}
unsafe extern "C" fn ZSTDMT_serialState_applySequences(
    mut serialState: *const SerialState,
    mut jobCCtx: *mut ZSTD_CCtx,
    mut seqStore: *const RawSeqStore_t,
) {
    if (*seqStore).size > 0 as size_t {
        ZSTD_referenceExternalSequences(jobCCtx, (*seqStore).seq, (*seqStore).size);
    }
}
unsafe extern "C" fn ZSTDMT_serialState_ensureFinished(
    mut serialState: *mut SerialState,
    mut jobID: ::core::ffi::c_uint,
    mut cSize: size_t,
) {
    &raw mut (*serialState).mutex;
    if (*serialState).nextJobID <= jobID {
        (*serialState).nextJobID = jobID.wrapping_add(1 as ::core::ffi::c_uint);
        &raw mut (*serialState).cond;
        &raw mut (*serialState).ldmWindowMutex;
        ZSTD_window_clear(&raw mut (*serialState).ldmWindow);
        &raw mut (*serialState).ldmWindowCond;
        &raw mut (*serialState).ldmWindowMutex;
    }
    &raw mut (*serialState).mutex;
}
static mut kNullRange: Range = Range {
    start: ::core::ptr::null::<::core::ffi::c_void>(),
    size: 0 as size_t,
};
unsafe extern "C" fn ZSTDMT_compressionJob(mut jobDescription: *mut ::core::ffi::c_void) {
    let mut current_block: u64;
    let job: *mut ZSTDMT_jobDescription = jobDescription as *mut ZSTDMT_jobDescription;
    let mut jobParams: ZSTD_CCtx_params = (*job).params;
    let cctx: *mut ZSTD_CCtx = ZSTDMT_getCCtx((*job).cctxPool) as *mut ZSTD_CCtx;
    let mut rawSeqStore: RawSeqStore_t = ZSTDMT_getSeq((*job).seqPool);
    let mut dstBuff: Buffer = (*job).dstBuff;
    let mut lastCBlockSize: size_t = 0 as size_t;
    if cctx.is_null() {
        &raw mut (*job).job_mutex;
        (*job).cSize = -(ZSTD_error_memory_allocation as ::core::ffi::c_int) as size_t;
        &raw mut (*job).job_mutex;
    } else {
        if dstBuff.start.is_null() {
            dstBuff = ZSTDMT_getBuffer((*job).bufPool);
            if dstBuff.start.is_null() {
                &raw mut (*job).job_mutex;
                (*job).cSize = -(ZSTD_error_memory_allocation as ::core::ffi::c_int) as size_t;
                &raw mut (*job).job_mutex;
                current_block = 8293844872593298595;
            } else {
                (*job).dstBuff = dstBuff;
                current_block = 7976072742316086414;
            }
        } else {
            current_block = 7976072742316086414;
        }
        match current_block {
            8293844872593298595 => {}
            _ => {
                if jobParams.ldmParams.enableLdm as ::core::ffi::c_uint
                    == ZSTD_ps_enable as ::core::ffi::c_int as ::core::ffi::c_uint
                    && rawSeqStore.seq.is_null()
                {
                    &raw mut (*job).job_mutex;
                    (*job).cSize = -(ZSTD_error_memory_allocation as ::core::ffi::c_int) as size_t;
                    &raw mut (*job).job_mutex;
                } else {
                    if (*job).jobID != 0 as ::core::ffi::c_uint {
                        jobParams.fParams.checksumFlag = 0 as ::core::ffi::c_int;
                    }
                    jobParams.ldmParams.enableLdm = ZSTD_ps_disable;
                    jobParams.nbWorkers = 0 as ::core::ffi::c_int;
                    ZSTDMT_serialState_genSequences(
                        (*job).serial,
                        &raw mut rawSeqStore,
                        (*job).src,
                        (*job).jobID,
                    );
                    if !(*job).cdict.is_null() {
                        let initError: size_t = ZSTD_compressBegin_advanced_internal(
                            cctx,
                            ::core::ptr::null::<::core::ffi::c_void>(),
                            0 as size_t,
                            ZSTD_dct_auto,
                            ZSTD_dtlm_fast,
                            (*job).cdict,
                            &raw mut jobParams,
                            (*job).fullFrameSize,
                        ) as size_t;
                        if ERR_isError(initError) != 0 {
                            &raw mut (*job).job_mutex;
                            (*job).cSize = initError;
                            &raw mut (*job).job_mutex;
                            current_block = 8293844872593298595;
                        } else {
                            current_block = 16738040538446813684;
                        }
                    } else {
                        let pledgedSrcSize: U64 = (if (*job).firstJob != 0 {
                            (*job).fullFrameSize
                        } else {
                            (*job).src.size as ::core::ffi::c_ulonglong
                        }) as U64;
                        let forceWindowError: size_t = ZSTD_CCtxParams_setParameter(
                            &raw mut jobParams,
                            ZSTD_c_experimentalParam3,
                            ((*job).firstJob == 0) as ::core::ffi::c_int,
                        ) as size_t;
                        if ERR_isError(forceWindowError) != 0 {
                            &raw mut (*job).job_mutex;
                            (*job).cSize = forceWindowError;
                            &raw mut (*job).job_mutex;
                            current_block = 8293844872593298595;
                        } else {
                            if (*job).firstJob == 0 {
                                let err: size_t = ZSTD_CCtxParams_setParameter(
                                    &raw mut jobParams,
                                    ZSTD_c_experimentalParam15,
                                    0 as ::core::ffi::c_int,
                                ) as size_t;
                                if ERR_isError(err) != 0 {
                                    &raw mut (*job).job_mutex;
                                    (*job).cSize = err;
                                    &raw mut (*job).job_mutex;
                                    current_block = 8293844872593298595;
                                } else {
                                    current_block = 2543120759711851213;
                                }
                            } else {
                                current_block = 2543120759711851213;
                            }
                            match current_block {
                                8293844872593298595 => {}
                                _ => {
                                    let initError_0: size_t = ZSTD_compressBegin_advanced_internal(
                                        cctx,
                                        (*job).prefix.start,
                                        (*job).prefix.size,
                                        ZSTD_dct_rawContent,
                                        ZSTD_dtlm_fast,
                                        ::core::ptr::null::<ZSTD_CDict>(),
                                        &raw mut jobParams,
                                        pledgedSrcSize as ::core::ffi::c_ulonglong,
                                    )
                                        as size_t;
                                    if ERR_isError(initError_0) != 0 {
                                        &raw mut (*job).job_mutex;
                                        (*job).cSize = initError_0;
                                        &raw mut (*job).job_mutex;
                                        current_block = 8293844872593298595;
                                    } else {
                                        current_block = 16738040538446813684;
                                    }
                                }
                            }
                        }
                    }
                    match current_block {
                        8293844872593298595 => {}
                        _ => {
                            ZSTDMT_serialState_applySequences(
                                (*job).serial,
                                cctx,
                                &raw mut rawSeqStore,
                            );
                            if (*job).firstJob == 0 {
                                let hSize: size_t = ZSTD_compressContinue_public(
                                    cctx,
                                    dstBuff.start,
                                    dstBuff.capacity,
                                    (*job).src.start,
                                    0 as size_t,
                                ) as size_t;
                                if ERR_isError(hSize) != 0 {
                                    &raw mut (*job).job_mutex;
                                    (*job).cSize = hSize;
                                    &raw mut (*job).job_mutex;
                                    current_block = 8293844872593298595;
                                } else {
                                    ZSTD_invalidateRepCodes(cctx);
                                    current_block = 6560072651652764009;
                                }
                            } else {
                                current_block = 6560072651652764009;
                            }
                            match current_block {
                                8293844872593298595 => {}
                                _ => {
                                    let chunkSize: size_t =
                                        (4 as ::core::ffi::c_int * ZSTD_BLOCKSIZE_MAX) as size_t;
                                    let nbChunks: ::core::ffi::c_int = (*job)
                                        .src
                                        .size
                                        .wrapping_add(chunkSize.wrapping_sub(1 as size_t))
                                        .wrapping_div(chunkSize)
                                        as ::core::ffi::c_int;
                                    let mut ip: *const BYTE = (*job).src.start as *const BYTE;
                                    let ostart: *mut BYTE = dstBuff.start as *mut BYTE;
                                    let mut op: *mut BYTE = ostart;
                                    let mut oend: *mut BYTE = op.offset(dstBuff.capacity as isize);
                                    let mut chunkNb: ::core::ffi::c_int = 0;
                                    ::core::mem::size_of::<size_t>() as usize
                                        > ::core::mem::size_of::<::core::ffi::c_int>() as usize;
                                    chunkNb = 1 as ::core::ffi::c_int;
                                    loop {
                                        if !(chunkNb < nbChunks) {
                                            current_block = 993425571616822999;
                                            break;
                                        }
                                        let cSize: size_t = ZSTD_compressContinue_public(
                                            cctx,
                                            op as *mut ::core::ffi::c_void,
                                            oend.offset_from(op) as ::core::ffi::c_long as size_t,
                                            ip as *const ::core::ffi::c_void,
                                            chunkSize,
                                        )
                                            as size_t;
                                        if ERR_isError(cSize) != 0 {
                                            &raw mut (*job).job_mutex;
                                            (*job).cSize = cSize;
                                            &raw mut (*job).job_mutex;
                                            current_block = 8293844872593298595;
                                            break;
                                        } else {
                                            ip = ip.offset(chunkSize as isize);
                                            op = op.offset(cSize as isize);
                                            &raw mut (*job).job_mutex;
                                            (*job).cSize = ((*job).cSize as ::core::ffi::c_ulong)
                                                .wrapping_add(cSize as ::core::ffi::c_ulong)
                                                as size_t
                                                as size_t;
                                            (*job).consumed =
                                                chunkSize.wrapping_mul(chunkNb as size_t);
                                            &raw mut (*job).job_cond;
                                            &raw mut (*job).job_mutex;
                                            chunkNb += 1;
                                        }
                                    }
                                    match current_block {
                                        8293844872593298595 => {}
                                        _ => {
                                            if (nbChunks > 0 as ::core::ffi::c_int)
                                                as ::core::ffi::c_int
                                                as ::core::ffi::c_uint
                                                | (*job).lastJob
                                                != 0
                                            {
                                                let lastBlockSize1: size_t = (*job).src.size
                                                    & chunkSize.wrapping_sub(1 as size_t);
                                                let lastBlockSize: size_t = if (lastBlockSize1
                                                    == 0 as size_t)
                                                    as ::core::ffi::c_int
                                                    & ((*job).src.size >= chunkSize)
                                                        as ::core::ffi::c_int
                                                    != 0
                                                {
                                                    chunkSize
                                                } else {
                                                    lastBlockSize1
                                                };
                                                let cSize_0: size_t = if (*job).lastJob != 0 {
                                                    ZSTD_compressEnd_public(
                                                        cctx,
                                                        op as *mut ::core::ffi::c_void,
                                                        oend.offset_from(op) as ::core::ffi::c_long
                                                            as size_t,
                                                        ip as *const ::core::ffi::c_void,
                                                        lastBlockSize,
                                                    )
                                                        as size_t
                                                } else {
                                                    ZSTD_compressContinue_public(
                                                        cctx,
                                                        op as *mut ::core::ffi::c_void,
                                                        oend.offset_from(op) as ::core::ffi::c_long
                                                            as size_t,
                                                        ip as *const ::core::ffi::c_void,
                                                        lastBlockSize,
                                                    )
                                                        as size_t
                                                };
                                                if ERR_isError(cSize_0) != 0 {
                                                    &raw mut (*job).job_mutex;
                                                    (*job).cSize = cSize_0;
                                                    &raw mut (*job).job_mutex;
                                                    current_block = 8293844872593298595;
                                                } else {
                                                    lastCBlockSize = cSize_0;
                                                    current_block = 200744462051969938;
                                                }
                                            } else {
                                                current_block = 200744462051969938;
                                            }
                                            match current_block {
                                                8293844872593298595 => {}
                                                _ => {
                                                    (*job).firstJob == 0;
                                                    ZSTD_CCtx_trace(cctx, 0 as size_t);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    ZSTDMT_serialState_ensureFinished((*job).serial, (*job).jobID, (*job).cSize);
    (*job).prefix.size > 0 as size_t;
    ZSTDMT_releaseSeq((*job).seqPool, rawSeqStore);
    ZSTDMT_releaseCCtx((*job).cctxPool, cctx);
    &raw mut (*job).job_mutex;
    ERR_isError((*job).cSize) != 0;
    (*job).cSize = ((*job).cSize as ::core::ffi::c_ulong)
        .wrapping_add(lastCBlockSize as ::core::ffi::c_ulong) as size_t
        as size_t;
    (*job).consumed = (*job).src.size;
    &raw mut (*job).job_cond;
    &raw mut (*job).job_mutex;
}
pub const RSYNC_LENGTH: ::core::ffi::c_int = 32 as ::core::ffi::c_int;
pub const RSYNC_MIN_BLOCK_LOG: ::core::ffi::c_int = ZSTD_BLOCKSIZELOG_MAX;
pub const RSYNC_MIN_BLOCK_SIZE: ::core::ffi::c_int =
    (1 as ::core::ffi::c_int) << RSYNC_MIN_BLOCK_LOG;
unsafe extern "C" fn ZSTDMT_freeJobsTable(
    mut jobTable: *mut ZSTDMT_jobDescription,
    mut nbJobs: U32,
    mut cMem: ZSTD_customMem,
) {
    let mut jobNb: U32 = 0;
    if jobTable.is_null() {
        return;
    }
    jobNb = 0 as U32;
    while jobNb < nbJobs {
        &raw mut (*jobTable.offset(jobNb as isize)).job_mutex;
        &raw mut (*jobTable.offset(jobNb as isize)).job_cond;
        jobNb = jobNb.wrapping_add(1);
    }
    ZSTD_customFree(jobTable as *mut ::core::ffi::c_void, cMem);
}
unsafe extern "C" fn ZSTDMT_createJobsTable(
    mut nbJobsPtr: *mut U32,
    mut cMem: ZSTD_customMem,
) -> *mut ZSTDMT_jobDescription {
    let nbJobsLog2: U32 = (ZSTD_highbit32(*nbJobsPtr) as U32).wrapping_add(1 as U32);
    let nbJobs: U32 = ((1 as ::core::ffi::c_int) << nbJobsLog2) as U32;
    let mut jobNb: U32 = 0;
    let jobTable: *mut ZSTDMT_jobDescription = ZSTD_customCalloc(
        (nbJobs as size_t).wrapping_mul(::core::mem::size_of::<ZSTDMT_jobDescription>() as size_t),
        cMem,
    ) as *mut ZSTDMT_jobDescription;
    let mut initError: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    if jobTable.is_null() {
        return ::core::ptr::null_mut::<ZSTDMT_jobDescription>();
    }
    *nbJobsPtr = nbJobs;
    jobNb = 0 as U32;
    while jobNb < nbJobs {
        &raw mut (*jobTable.offset(jobNb as isize)).job_mutex;
        initError |= 0 as ::core::ffi::c_int;
        &raw mut (*jobTable.offset(jobNb as isize)).job_cond;
        initError |= 0 as ::core::ffi::c_int;
        jobNb = jobNb.wrapping_add(1);
    }
    if initError != 0 as ::core::ffi::c_int {
        ZSTDMT_freeJobsTable(jobTable, nbJobs, cMem);
        return ::core::ptr::null_mut::<ZSTDMT_jobDescription>();
    }
    return jobTable;
}
unsafe extern "C" fn ZSTDMT_expandJobsTable(
    mut mtctx: *mut ZSTDMT_CCtx,
    mut nbWorkers: U32,
) -> size_t {
    let mut nbJobs: U32 = nbWorkers.wrapping_add(2 as U32);
    if nbJobs > ((*mtctx).jobIDMask as U32).wrapping_add(1 as U32) {
        ZSTDMT_freeJobsTable(
            (*mtctx).jobs,
            ((*mtctx).jobIDMask as U32).wrapping_add(1 as U32),
            (*mtctx).cMem,
        );
        (*mtctx).jobIDMask = 0 as ::core::ffi::c_uint;
        (*mtctx).jobs = ZSTDMT_createJobsTable(&raw mut nbJobs, (*mtctx).cMem);
        if (*mtctx).jobs.is_null() {
            return -(ZSTD_error_memory_allocation as ::core::ffi::c_int) as size_t;
        }
        (*mtctx).jobIDMask = nbJobs.wrapping_sub(1 as U32) as ::core::ffi::c_uint;
    }
    return 0 as size_t;
}
unsafe extern "C" fn ZSTDMT_CCtxParam_setNbWorkers(
    mut params: *mut ZSTD_CCtx_params,
    mut nbWorkers: ::core::ffi::c_uint,
) -> size_t {
    return ZSTD_CCtxParams_setParameter(params, ZSTD_c_nbWorkers, nbWorkers as ::core::ffi::c_int);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDMT_createCCtx_advanced(
    mut nbWorkers: ::core::ffi::c_uint,
    mut cMem: ZSTD_customMem,
    mut pool: *mut ZSTD_threadPool,
) -> *mut ZSTDMT_CCtx {
    return ::core::ptr::null_mut::<ZSTDMT_CCtx>();
}
unsafe extern "C" fn ZSTDMT_releaseAllJobResources(mut mtctx: *mut ZSTDMT_CCtx) {
    let mut jobID: ::core::ffi::c_uint = 0;
    jobID = 0 as ::core::ffi::c_uint;
    while jobID <= (*mtctx).jobIDMask {
        let mutex: ZSTD_pthread_mutex_t = (*(*mtctx).jobs.offset(jobID as isize)).job_mutex;
        let cond: ZSTD_pthread_cond_t = (*(*mtctx).jobs.offset(jobID as isize)).job_cond;
        ZSTDMT_releaseBuffer(
            (*mtctx).bufPool,
            (*(*mtctx).jobs.offset(jobID as isize)).dstBuff,
        );
        ::libc::memset(
            (*mtctx).jobs.offset(jobID as isize) as *mut ZSTDMT_jobDescription
                as *mut ::core::ffi::c_void,
            0 as ::core::ffi::c_int,
            ::core::mem::size_of::<ZSTDMT_jobDescription>() as ::libc::size_t,
        );
        (*(*mtctx).jobs.offset(jobID as isize)).job_mutex = mutex;
        (*(*mtctx).jobs.offset(jobID as isize)).job_cond = cond;
        jobID = jobID.wrapping_add(1);
    }
    (*mtctx).inBuff.buffer = g_nullBuffer;
    (*mtctx).inBuff.filled = 0 as size_t;
    (*mtctx).allJobsCompleted = 1 as ::core::ffi::c_uint;
}
unsafe extern "C" fn ZSTDMT_waitForAllJobsCompleted(mut mtctx: *mut ZSTDMT_CCtx) {
    while (*mtctx).doneJobID < (*mtctx).nextJobID {
        let jobID: ::core::ffi::c_uint = (*mtctx).doneJobID & (*mtctx).jobIDMask;
        &raw mut (*(*mtctx).jobs.offset(jobID as isize)).job_mutex;
        while (*(*mtctx).jobs.offset(jobID as isize)).consumed
            < (*(*mtctx).jobs.offset(jobID as isize)).src.size
        {
            &raw mut (*(*mtctx).jobs.offset(jobID as isize)).job_cond;
            &raw mut (*(*mtctx).jobs.offset(jobID as isize)).job_mutex;
        }
        &raw mut (*(*mtctx).jobs.offset(jobID as isize)).job_mutex;
        (*mtctx).doneJobID = (*mtctx).doneJobID.wrapping_add(1);
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDMT_freeCCtx(mut mtctx: *mut ZSTDMT_CCtx) -> size_t {
    if mtctx.is_null() {
        return 0 as size_t;
    }
    if (*mtctx).providedFactory() == 0 {
        POOL_free((*mtctx).factory);
    }
    ZSTDMT_releaseAllJobResources(mtctx);
    ZSTDMT_freeJobsTable(
        (*mtctx).jobs,
        ((*mtctx).jobIDMask as U32).wrapping_add(1 as U32),
        (*mtctx).cMem,
    );
    ZSTDMT_freeBufferPool((*mtctx).bufPool);
    ZSTDMT_freeCCtxPool((*mtctx).cctxPool);
    ZSTDMT_freeSeqPool((*mtctx).seqPool);
    ZSTDMT_serialState_free(&raw mut (*mtctx).serial);
    ZSTD_freeCDict((*mtctx).cdictLocal);
    if !(*mtctx).roundBuff.buffer.is_null() {
        ZSTD_customFree(
            (*mtctx).roundBuff.buffer as *mut ::core::ffi::c_void,
            (*mtctx).cMem,
        );
    }
    ZSTD_customFree(mtctx as *mut ::core::ffi::c_void, (*mtctx).cMem);
    return 0 as size_t;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDMT_sizeof_CCtx(mut mtctx: *mut ZSTDMT_CCtx) -> size_t {
    if mtctx.is_null() {
        return 0 as size_t;
    }
    return (::core::mem::size_of::<ZSTDMT_CCtx>() as size_t)
        .wrapping_add(POOL_sizeof((*mtctx).factory))
        .wrapping_add(ZSTDMT_sizeof_bufferPool((*mtctx).bufPool))
        .wrapping_add(
            ((*mtctx).jobIDMask.wrapping_add(1 as ::core::ffi::c_uint) as size_t)
                .wrapping_mul(::core::mem::size_of::<ZSTDMT_jobDescription>() as size_t),
        )
        .wrapping_add(ZSTDMT_sizeof_CCtxPool((*mtctx).cctxPool))
        .wrapping_add(ZSTDMT_sizeof_seqPool((*mtctx).seqPool))
        .wrapping_add(ZSTD_sizeof_CDict((*mtctx).cdictLocal))
        .wrapping_add((*mtctx).roundBuff.capacity);
}
unsafe extern "C" fn ZSTDMT_resize(
    mut mtctx: *mut ZSTDMT_CCtx,
    mut nbWorkers: ::core::ffi::c_uint,
) -> size_t {
    if POOL_resize((*mtctx).factory, nbWorkers as size_t) != 0 {
        return -(ZSTD_error_memory_allocation as ::core::ffi::c_int) as size_t;
    }
    let err_code: size_t = ZSTDMT_expandJobsTable(mtctx, nbWorkers as U32) as size_t;
    if ERR_isError(err_code) != 0 {
        return err_code;
    }
    (*mtctx).bufPool = ZSTDMT_expandBufferPool(
        (*mtctx).bufPool,
        (2 as ::core::ffi::c_uint)
            .wrapping_mul(nbWorkers)
            .wrapping_add(3 as ::core::ffi::c_uint),
    );
    if (*mtctx).bufPool.is_null() {
        return -(ZSTD_error_memory_allocation as ::core::ffi::c_int) as size_t;
    }
    (*mtctx).cctxPool = ZSTDMT_expandCCtxPool((*mtctx).cctxPool, nbWorkers as ::core::ffi::c_int);
    if (*mtctx).cctxPool.is_null() {
        return -(ZSTD_error_memory_allocation as ::core::ffi::c_int) as size_t;
    }
    (*mtctx).seqPool = ZSTDMT_expandSeqPool((*mtctx).seqPool, nbWorkers as U32);
    if (*mtctx).seqPool.is_null() {
        return -(ZSTD_error_memory_allocation as ::core::ffi::c_int) as size_t;
    }
    ZSTDMT_CCtxParam_setNbWorkers(&raw mut (*mtctx).params, nbWorkers);
    return 0 as size_t;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDMT_updateCParams_whileCompressing(
    mut mtctx: *mut ZSTDMT_CCtx,
    mut cctxParams: *const ZSTD_CCtx_params,
) {
    let saved_wlog: U32 = (*mtctx).params.cParams.windowLog as U32;
    let compressionLevel: ::core::ffi::c_int = (*cctxParams).compressionLevel;
    (*mtctx).params.compressionLevel = compressionLevel;
    let mut cParams: ZSTD_compressionParameters = ZSTD_getCParamsFromCCtxParams(
        cctxParams,
        ZSTD_CONTENTSIZE_UNKNOWN as U64,
        0 as size_t,
        ZSTD_cpm_noAttachDict,
    );
    cParams.windowLog = saved_wlog as ::core::ffi::c_uint;
    (*mtctx).params.cParams = cParams;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDMT_getFrameProgression(
    mut mtctx: *mut ZSTDMT_CCtx,
) -> ZSTD_frameProgression {
    let mut fps: ZSTD_frameProgression = ZSTD_frameProgression {
        ingested: 0,
        consumed: 0,
        produced: 0,
        flushed: 0,
        currentJobID: 0,
        nbActiveWorkers: 0,
    };
    fps.ingested = (*mtctx)
        .consumed
        .wrapping_add((*mtctx).inBuff.filled as ::core::ffi::c_ulonglong);
    fps.consumed = (*mtctx).consumed;
    fps.flushed = (*mtctx).produced;
    fps.produced = fps.flushed;
    fps.currentJobID = (*mtctx).nextJobID;
    fps.nbActiveWorkers = 0 as ::core::ffi::c_uint;
    let mut jobNb: ::core::ffi::c_uint = 0;
    let mut lastJobNb: ::core::ffi::c_uint = (*mtctx)
        .nextJobID
        .wrapping_add((*mtctx).jobReady as ::core::ffi::c_uint);
    jobNb = (*mtctx).doneJobID;
    while jobNb < lastJobNb {
        let wJobID: ::core::ffi::c_uint = jobNb & (*mtctx).jobIDMask;
        let mut jobPtr: *mut ZSTDMT_jobDescription =
            (*mtctx).jobs.offset(wJobID as isize) as *mut ZSTDMT_jobDescription;
        &raw mut (*jobPtr).job_mutex;
        let cResult: size_t = (*jobPtr).cSize;
        let produced: size_t = if ERR_isError(cResult) != 0 {
            0 as size_t
        } else {
            cResult
        };
        let flushed: size_t = if ERR_isError(cResult) != 0 {
            0 as size_t
        } else {
            (*jobPtr).dstFlushed
        };
        fps.ingested = fps
            .ingested
            .wrapping_add((*jobPtr).src.size as ::core::ffi::c_ulonglong);
        fps.consumed = fps
            .consumed
            .wrapping_add((*jobPtr).consumed as ::core::ffi::c_ulonglong);
        fps.produced = fps
            .produced
            .wrapping_add(produced as ::core::ffi::c_ulonglong);
        fps.flushed = fps
            .flushed
            .wrapping_add(flushed as ::core::ffi::c_ulonglong);
        fps.nbActiveWorkers = fps.nbActiveWorkers.wrapping_add(
            ((*jobPtr).consumed < (*jobPtr).src.size) as ::core::ffi::c_int as ::core::ffi::c_uint,
        );
        &raw mut (*(*mtctx).jobs.offset(wJobID as isize)).job_mutex;
        jobNb = jobNb.wrapping_add(1);
    }
    return fps;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDMT_toFlushNow(mut mtctx: *mut ZSTDMT_CCtx) -> size_t {
    let mut toFlush: size_t = 0;
    let jobID: ::core::ffi::c_uint = (*mtctx).doneJobID;
    if jobID == (*mtctx).nextJobID {
        return 0 as size_t;
    }
    let wJobID: ::core::ffi::c_uint = jobID & (*mtctx).jobIDMask;
    let jobPtr: *mut ZSTDMT_jobDescription =
        (*mtctx).jobs.offset(wJobID as isize) as *mut ZSTDMT_jobDescription;
    &raw mut (*jobPtr).job_mutex;
    let cResult: size_t = (*jobPtr).cSize;
    let produced: size_t = if ERR_isError(cResult) != 0 {
        0 as size_t
    } else {
        cResult
    };
    let flushed: size_t = if ERR_isError(cResult) != 0 {
        0 as size_t
    } else {
        (*jobPtr).dstFlushed
    };
    toFlush = produced.wrapping_sub(flushed);
    toFlush == 0 as size_t;
    &raw mut (*(*mtctx).jobs.offset(wJobID as isize)).job_mutex;
    return toFlush;
}
unsafe extern "C" fn ZSTDMT_computeTargetJobLog(
    mut params: *const ZSTD_CCtx_params,
) -> ::core::ffi::c_uint {
    let mut jobLog: ::core::ffi::c_uint = 0;
    if (*params).ldmParams.enableLdm as ::core::ffi::c_uint
        == ZSTD_ps_enable as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        jobLog = (if 21 as U32
            > ZSTD_cycleLog(
                (*params).cParams.chainLog as U32,
                (*params).cParams.strategy,
            )
            .wrapping_add(3 as U32)
        {
            21 as U32
        } else {
            ZSTD_cycleLog(
                (*params).cParams.chainLog as U32,
                (*params).cParams.strategy,
            )
            .wrapping_add(3 as U32)
        }) as ::core::ffi::c_uint;
    } else {
        jobLog = if 20 as ::core::ffi::c_uint
            > (*params)
                .cParams
                .windowLog
                .wrapping_add(2 as ::core::ffi::c_uint)
        {
            20 as ::core::ffi::c_uint
        } else {
            (*params)
                .cParams
                .windowLog
                .wrapping_add(2 as ::core::ffi::c_uint)
        };
    }
    return if jobLog
        < (if MEM_32bits() != 0 {
            29 as ::core::ffi::c_int
        } else {
            30 as ::core::ffi::c_int
        }) as ::core::ffi::c_uint
    {
        jobLog
    } else {
        (if MEM_32bits() != 0 {
            29 as ::core::ffi::c_int
        } else {
            30 as ::core::ffi::c_int
        }) as ::core::ffi::c_uint
    };
}
unsafe extern "C" fn ZSTDMT_overlapLog_default(mut strat: ZSTD_strategy) -> ::core::ffi::c_int {
    match strat as ::core::ffi::c_uint {
        9 => return 9 as ::core::ffi::c_int,
        8 | 7 => return 8 as ::core::ffi::c_int,
        6 | 5 => return 7 as ::core::ffi::c_int,
        4 | 3 | 2 | 1 | _ => {}
    }
    return 6 as ::core::ffi::c_int;
}
unsafe extern "C" fn ZSTDMT_overlapLog(
    mut ovlog: ::core::ffi::c_int,
    mut strat: ZSTD_strategy,
) -> ::core::ffi::c_int {
    if ovlog == 0 as ::core::ffi::c_int {
        return ZSTDMT_overlapLog_default(strat);
    }
    return ovlog;
}
unsafe extern "C" fn ZSTDMT_computeOverlapSize(mut params: *const ZSTD_CCtx_params) -> size_t {
    let overlapRLog: ::core::ffi::c_int = 9 as ::core::ffi::c_int
        - ZSTDMT_overlapLog((*params).overlapLog, (*params).cParams.strategy) as ::core::ffi::c_int;
    let mut ovLog: ::core::ffi::c_int = (if overlapRLog >= 8 as ::core::ffi::c_int {
        0 as ::core::ffi::c_uint
    } else {
        (*params)
            .cParams
            .windowLog
            .wrapping_sub(overlapRLog as ::core::ffi::c_uint)
    }) as ::core::ffi::c_int;
    if (*params).ldmParams.enableLdm as ::core::ffi::c_uint
        == ZSTD_ps_enable as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        ovLog = (if (*params).cParams.windowLog
            < ZSTDMT_computeTargetJobLog(params).wrapping_sub(2 as ::core::ffi::c_uint)
        {
            (*params).cParams.windowLog
        } else {
            ZSTDMT_computeTargetJobLog(params).wrapping_sub(2 as ::core::ffi::c_uint)
        })
        .wrapping_sub(overlapRLog as ::core::ffi::c_uint) as ::core::ffi::c_int;
    }
    return if ovLog == 0 as ::core::ffi::c_int {
        0 as size_t
    } else {
        (1 as ::core::ffi::c_int as size_t) << ovLog
    };
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDMT_initCStream_internal(
    mut mtctx: *mut ZSTDMT_CCtx,
    mut dict: *const ::core::ffi::c_void,
    mut dictSize: size_t,
    mut dictContentType: ZSTD_dictContentType_e,
    mut cdict: *const ZSTD_CDict,
    mut params: ZSTD_CCtx_params,
    mut pledgedSrcSize: ::core::ffi::c_ulonglong,
) -> size_t {
    if params.nbWorkers != (*mtctx).params.nbWorkers {
        let err_code: size_t =
            ZSTDMT_resize(mtctx, params.nbWorkers as ::core::ffi::c_uint) as size_t;
        if ERR_isError(err_code) != 0 {
            return err_code;
        }
    }
    if params.jobSize != 0 as size_t && params.jobSize < ZSTDMT_JOBSIZE_MIN as size_t {
        params.jobSize = ZSTDMT_JOBSIZE_MIN as size_t;
    }
    if params.jobSize
        > (if MEM_32bits() != 0 {
            512 as ::core::ffi::c_int * ((1 as ::core::ffi::c_int) << 20 as ::core::ffi::c_int)
        } else {
            1024 as ::core::ffi::c_int * ((1 as ::core::ffi::c_int) << 20 as ::core::ffi::c_int)
        }) as size_t
    {
        params.jobSize = (if MEM_32bits() != 0 {
            512 as ::core::ffi::c_int * ((1 as ::core::ffi::c_int) << 20 as ::core::ffi::c_int)
        } else {
            1024 as ::core::ffi::c_int * ((1 as ::core::ffi::c_int) << 20 as ::core::ffi::c_int)
        }) as size_t;
    }
    if (*mtctx).allJobsCompleted == 0 as ::core::ffi::c_uint {
        ZSTDMT_waitForAllJobsCompleted(mtctx);
        ZSTDMT_releaseAllJobResources(mtctx);
        (*mtctx).allJobsCompleted = 1 as ::core::ffi::c_uint;
    }
    (*mtctx).params = params;
    (*mtctx).frameContentSize = pledgedSrcSize;
    ZSTD_freeCDict((*mtctx).cdictLocal);
    if !dict.is_null() {
        (*mtctx).cdictLocal = ZSTD_createCDict_advanced(
            dict,
            dictSize,
            ZSTD_dlm_byCopy,
            dictContentType,
            params.cParams,
            (*mtctx).cMem,
        );
        (*mtctx).cdict = (*mtctx).cdictLocal;
        if (*mtctx).cdictLocal.is_null() {
            return -(ZSTD_error_memory_allocation as ::core::ffi::c_int) as size_t;
        }
    } else {
        (*mtctx).cdictLocal = ::core::ptr::null_mut::<ZSTD_CDict>();
        (*mtctx).cdict = cdict;
    }
    (*mtctx).targetPrefixSize = ZSTDMT_computeOverlapSize(&raw mut params);
    (*mtctx).targetSectionSize = params.jobSize;
    if (*mtctx).targetSectionSize == 0 as size_t {
        (*mtctx).targetSectionSize = ((1 as ::core::ffi::c_ulonglong)
            << ZSTDMT_computeTargetJobLog(&raw mut params))
            as size_t;
    }
    if params.rsyncable != 0 {
        let jobSizeKB: U32 = ((*mtctx).targetSectionSize >> 10 as ::core::ffi::c_int) as U32;
        let rsyncBits: U32 = (ZSTD_highbit32(jobSizeKB) as U32).wrapping_add(10 as U32);
        (*mtctx).rsync.hash = 0 as U64;
        (*mtctx).rsync.hitMask = ((1 as ::core::ffi::c_ulonglong) << rsyncBits)
            .wrapping_sub(1 as ::core::ffi::c_ulonglong) as U64;
        (*mtctx).rsync.primePower = ZSTD_rollingHash_primePower(RSYNC_LENGTH as U32);
    }
    if (*mtctx).targetSectionSize < (*mtctx).targetPrefixSize {
        (*mtctx).targetSectionSize = (*mtctx).targetPrefixSize;
    }
    ZSTDMT_setBufferSize(
        (*mtctx).bufPool,
        ZSTD_compressBound((*mtctx).targetSectionSize),
    );
    let windowSize: size_t = (if (*mtctx).params.ldmParams.enableLdm as ::core::ffi::c_uint
        == ZSTD_ps_enable as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        (1 as ::core::ffi::c_uint) << (*mtctx).params.cParams.windowLog
    } else {
        0 as ::core::ffi::c_uint
    }) as size_t;
    let nbSlackBuffers: size_t = (2 as ::core::ffi::c_int
        + ((*mtctx).targetPrefixSize > 0 as size_t) as ::core::ffi::c_int)
        as size_t;
    let slackSize: size_t = (*mtctx).targetSectionSize.wrapping_mul(nbSlackBuffers);
    let nbWorkers: size_t = (if (*mtctx).params.nbWorkers > 1 as ::core::ffi::c_int {
        (*mtctx).params.nbWorkers
    } else {
        1 as ::core::ffi::c_int
    }) as size_t;
    let sectionsSize: size_t = (*mtctx).targetSectionSize.wrapping_mul(nbWorkers);
    let capacity: size_t = (if windowSize > sectionsSize {
        windowSize
    } else {
        sectionsSize
    })
    .wrapping_add(slackSize);
    if (*mtctx).roundBuff.capacity < capacity {
        if !(*mtctx).roundBuff.buffer.is_null() {
            ZSTD_customFree(
                (*mtctx).roundBuff.buffer as *mut ::core::ffi::c_void,
                (*mtctx).cMem,
            );
        }
        (*mtctx).roundBuff.buffer = ZSTD_customMalloc(capacity, (*mtctx).cMem) as *mut BYTE;
        if (*mtctx).roundBuff.buffer.is_null() {
            (*mtctx).roundBuff.capacity = 0 as size_t;
            return -(ZSTD_error_memory_allocation as ::core::ffi::c_int) as size_t;
        }
        (*mtctx).roundBuff.capacity = capacity;
    }
    (*mtctx).roundBuff.pos = 0 as size_t;
    (*mtctx).inBuff.buffer = g_nullBuffer;
    (*mtctx).inBuff.filled = 0 as size_t;
    (*mtctx).inBuff.prefix = kNullRange;
    (*mtctx).doneJobID = 0 as ::core::ffi::c_uint;
    (*mtctx).nextJobID = 0 as ::core::ffi::c_uint;
    (*mtctx).frameEnded = 0 as ::core::ffi::c_uint;
    (*mtctx).allJobsCompleted = 0 as ::core::ffi::c_uint;
    (*mtctx).consumed = 0 as ::core::ffi::c_ulonglong;
    (*mtctx).produced = 0 as ::core::ffi::c_ulonglong;
    ZSTD_freeCDict((*mtctx).cdictLocal);
    (*mtctx).cdictLocal = ::core::ptr::null_mut::<ZSTD_CDict>();
    (*mtctx).cdict = ::core::ptr::null::<ZSTD_CDict>();
    if !dict.is_null() {
        if dictContentType as ::core::ffi::c_uint
            == ZSTD_dct_rawContent as ::core::ffi::c_int as ::core::ffi::c_uint
        {
            (*mtctx).inBuff.prefix.start = dict as *const BYTE as *const ::core::ffi::c_void;
            (*mtctx).inBuff.prefix.size = dictSize;
        } else {
            (*mtctx).cdictLocal = ZSTD_createCDict_advanced(
                dict,
                dictSize,
                ZSTD_dlm_byRef,
                dictContentType,
                params.cParams,
                (*mtctx).cMem,
            );
            (*mtctx).cdict = (*mtctx).cdictLocal;
            if (*mtctx).cdictLocal.is_null() {
                return -(ZSTD_error_memory_allocation as ::core::ffi::c_int) as size_t;
            }
        }
    } else {
        (*mtctx).cdict = cdict;
    }
    if ZSTDMT_serialState_reset(
        &raw mut (*mtctx).serial,
        (*mtctx).seqPool,
        params,
        (*mtctx).targetSectionSize,
        dict,
        dictSize,
        dictContentType,
    ) != 0
    {
        return -(ZSTD_error_memory_allocation as ::core::ffi::c_int) as size_t;
    }
    return 0 as size_t;
}
unsafe extern "C" fn ZSTDMT_writeLastEmptyBlock(mut job: *mut ZSTDMT_jobDescription) {
    (*job).dstBuff = ZSTDMT_getBuffer((*job).bufPool);
    if (*job).dstBuff.start.is_null() {
        (*job).cSize = -(ZSTD_error_memory_allocation as ::core::ffi::c_int) as size_t;
        return;
    }
    (*job).src = kNullRange;
    (*job).cSize = ZSTD_writeLastEmptyBlock((*job).dstBuff.start, (*job).dstBuff.capacity);
}
unsafe extern "C" fn ZSTDMT_createCompressionJob(
    mut mtctx: *mut ZSTDMT_CCtx,
    mut srcSize: size_t,
    mut endOp: ZSTD_EndDirective,
) -> size_t {
    let jobID: ::core::ffi::c_uint = (*mtctx).nextJobID & (*mtctx).jobIDMask;
    let endFrame: ::core::ffi::c_int = (endOp as ::core::ffi::c_uint
        == ZSTD_e_end as ::core::ffi::c_int as ::core::ffi::c_uint)
        as ::core::ffi::c_int;
    if (*mtctx).nextJobID > (*mtctx).doneJobID.wrapping_add((*mtctx).jobIDMask) {
        return 0 as size_t;
    }
    if (*mtctx).jobReady == 0 {
        let mut src: *const BYTE = (*mtctx).inBuff.buffer.start as *const BYTE;
        let ref mut fresh4 = (*(*mtctx).jobs.offset(jobID as isize)).src.start;
        *fresh4 = src as *const ::core::ffi::c_void;
        (*(*mtctx).jobs.offset(jobID as isize)).src.size = srcSize;
        (*(*mtctx).jobs.offset(jobID as isize)).prefix = (*mtctx).inBuff.prefix;
        (*(*mtctx).jobs.offset(jobID as isize)).consumed = 0 as size_t;
        (*(*mtctx).jobs.offset(jobID as isize)).cSize = 0 as size_t;
        (*(*mtctx).jobs.offset(jobID as isize)).params = (*mtctx).params;
        let ref mut fresh5 = (*(*mtctx).jobs.offset(jobID as isize)).cdict;
        *fresh5 = if (*mtctx).nextJobID == 0 as ::core::ffi::c_uint {
            (*mtctx).cdict
        } else {
            ::core::ptr::null::<ZSTD_CDict>()
        };
        (*(*mtctx).jobs.offset(jobID as isize)).fullFrameSize = (*mtctx).frameContentSize;
        (*(*mtctx).jobs.offset(jobID as isize)).dstBuff = g_nullBuffer;
        let ref mut fresh6 = (*(*mtctx).jobs.offset(jobID as isize)).cctxPool;
        *fresh6 = (*mtctx).cctxPool;
        let ref mut fresh7 = (*(*mtctx).jobs.offset(jobID as isize)).bufPool;
        *fresh7 = (*mtctx).bufPool;
        let ref mut fresh8 = (*(*mtctx).jobs.offset(jobID as isize)).seqPool;
        *fresh8 = (*mtctx).seqPool;
        let ref mut fresh9 = (*(*mtctx).jobs.offset(jobID as isize)).serial;
        *fresh9 = &raw mut (*mtctx).serial;
        (*(*mtctx).jobs.offset(jobID as isize)).jobID = (*mtctx).nextJobID;
        (*(*mtctx).jobs.offset(jobID as isize)).firstJob =
            ((*mtctx).nextJobID == 0 as ::core::ffi::c_uint) as ::core::ffi::c_int
                as ::core::ffi::c_uint;
        (*(*mtctx).jobs.offset(jobID as isize)).lastJob = endFrame as ::core::ffi::c_uint;
        (*(*mtctx).jobs.offset(jobID as isize)).frameChecksumNeeded =
            ((*mtctx).params.fParams.checksumFlag != 0
                && endFrame != 0
                && (*mtctx).nextJobID > 0 as ::core::ffi::c_uint) as ::core::ffi::c_int
                as ::core::ffi::c_uint;
        (*(*mtctx).jobs.offset(jobID as isize)).dstFlushed = 0 as size_t;
        (*mtctx).roundBuff.pos = ((*mtctx).roundBuff.pos as ::core::ffi::c_ulong)
            .wrapping_add(srcSize as ::core::ffi::c_ulong)
            as size_t as size_t;
        (*mtctx).inBuff.buffer = g_nullBuffer;
        (*mtctx).inBuff.filled = 0 as size_t;
        if endFrame == 0 {
            let newPrefixSize: size_t = if srcSize < (*mtctx).targetPrefixSize {
                srcSize
            } else {
                (*mtctx).targetPrefixSize
            };
            (*mtctx).inBuff.prefix.start = src
                .offset(srcSize as isize)
                .offset(-(newPrefixSize as isize))
                as *const ::core::ffi::c_void;
            (*mtctx).inBuff.prefix.size = newPrefixSize;
        } else {
            (*mtctx).inBuff.prefix = kNullRange;
            (*mtctx).frameEnded = endFrame as ::core::ffi::c_uint;
            if (*mtctx).nextJobID == 0 as ::core::ffi::c_uint {
                (*mtctx).params.fParams.checksumFlag = 0 as ::core::ffi::c_int;
            }
        }
        if srcSize == 0 as size_t && (*mtctx).nextJobID > 0 as ::core::ffi::c_uint {
            ZSTDMT_writeLastEmptyBlock((*mtctx).jobs.offset(jobID as isize));
            (*mtctx).nextJobID = (*mtctx).nextJobID.wrapping_add(1);
            return 0 as size_t;
        }
    }
    if POOL_tryAdd(
        (*mtctx).factory,
        Some(ZSTDMT_compressionJob as unsafe extern "C" fn(*mut ::core::ffi::c_void) -> ()),
        (*mtctx).jobs.offset(jobID as isize) as *mut ZSTDMT_jobDescription
            as *mut ::core::ffi::c_void,
    ) != 0
    {
        (*mtctx).nextJobID = (*mtctx).nextJobID.wrapping_add(1);
        (*mtctx).jobReady = 0 as ::core::ffi::c_int;
    } else {
        (*mtctx).jobReady = 1 as ::core::ffi::c_int;
    }
    return 0 as size_t;
}
unsafe extern "C" fn ZSTDMT_flushProduced(
    mut mtctx: *mut ZSTDMT_CCtx,
    mut output: *mut ZSTD_outBuffer,
    mut blockToFlush: ::core::ffi::c_uint,
    mut end: ZSTD_EndDirective,
) -> size_t {
    let wJobID: ::core::ffi::c_uint = (*mtctx).doneJobID & (*mtctx).jobIDMask;
    &raw mut (*(*mtctx).jobs.offset(wJobID as isize)).job_mutex;
    if blockToFlush != 0 && (*mtctx).doneJobID < (*mtctx).nextJobID {
        while (*(*mtctx).jobs.offset(wJobID as isize)).dstFlushed
            == (*(*mtctx).jobs.offset(wJobID as isize)).cSize
        {
            if (*(*mtctx).jobs.offset(wJobID as isize)).consumed
                == (*(*mtctx).jobs.offset(wJobID as isize)).src.size
            {
                break;
            }
            &raw mut (*(*mtctx).jobs.offset(wJobID as isize)).job_cond;
            &raw mut (*(*mtctx).jobs.offset(wJobID as isize)).job_mutex;
        }
    }
    let mut cSize: size_t = (*(*mtctx).jobs.offset(wJobID as isize)).cSize;
    let srcConsumed: size_t = (*(*mtctx).jobs.offset(wJobID as isize)).consumed;
    let srcSize: size_t = (*(*mtctx).jobs.offset(wJobID as isize)).src.size;
    &raw mut (*(*mtctx).jobs.offset(wJobID as isize)).job_mutex;
    if ERR_isError(cSize) != 0 {
        ZSTDMT_waitForAllJobsCompleted(mtctx);
        ZSTDMT_releaseAllJobResources(mtctx);
        return cSize;
    }
    if srcConsumed == srcSize && (*(*mtctx).jobs.offset(wJobID as isize)).frameChecksumNeeded != 0 {
        let checksum: U32 = ZSTD_XXH64_digest(&raw mut (*mtctx).serial.xxhState) as U32;
        MEM_writeLE32(
            ((*(*mtctx).jobs.offset(wJobID as isize)).dstBuff.start as *mut ::core::ffi::c_char)
                .offset((*(*mtctx).jobs.offset(wJobID as isize)).cSize as isize)
                as *mut ::core::ffi::c_void,
            checksum,
        );
        cSize = (cSize as ::core::ffi::c_ulong).wrapping_add(4 as ::core::ffi::c_ulong) as size_t
            as size_t;
        let ref mut fresh2 = (*(*mtctx).jobs.offset(wJobID as isize)).cSize;
        *fresh2 = (*fresh2 as ::core::ffi::c_ulong).wrapping_add(4 as ::core::ffi::c_ulong)
            as size_t as size_t;
        (*(*mtctx).jobs.offset(wJobID as isize)).frameChecksumNeeded = 0 as ::core::ffi::c_uint;
    }
    if cSize > 0 as size_t {
        let toFlush: size_t = if cSize
            .wrapping_sub((*(*mtctx).jobs.offset(wJobID as isize)).dstFlushed)
            < (*output).size.wrapping_sub((*output).pos)
        {
            cSize.wrapping_sub((*(*mtctx).jobs.offset(wJobID as isize)).dstFlushed)
        } else {
            (*output).size.wrapping_sub((*output).pos)
        };
        if toFlush > 0 as size_t {
            ::libc::memcpy(
                ((*output).dst as *mut ::core::ffi::c_char).offset((*output).pos as isize)
                    as *mut ::core::ffi::c_void,
                ((*(*mtctx).jobs.offset(wJobID as isize)).dstBuff.start
                    as *const ::core::ffi::c_char)
                    .offset((*(*mtctx).jobs.offset(wJobID as isize)).dstFlushed as isize)
                    as *const ::core::ffi::c_void,
                toFlush as ::libc::size_t,
            );
        }
        (*output).pos = ((*output).pos as ::core::ffi::c_ulong)
            .wrapping_add(toFlush as ::core::ffi::c_ulong) as size_t
            as size_t;
        let ref mut fresh3 = (*(*mtctx).jobs.offset(wJobID as isize)).dstFlushed;
        *fresh3 = (*fresh3 as ::core::ffi::c_ulong).wrapping_add(toFlush as ::core::ffi::c_ulong)
            as size_t as size_t;
        if srcConsumed == srcSize && (*(*mtctx).jobs.offset(wJobID as isize)).dstFlushed == cSize {
            ZSTDMT_releaseBuffer(
                (*mtctx).bufPool,
                (*(*mtctx).jobs.offset(wJobID as isize)).dstBuff,
            );
            (*(*mtctx).jobs.offset(wJobID as isize)).dstBuff = g_nullBuffer;
            (*(*mtctx).jobs.offset(wJobID as isize)).cSize = 0 as size_t;
            (*mtctx).consumed = (*mtctx)
                .consumed
                .wrapping_add(srcSize as ::core::ffi::c_ulonglong);
            (*mtctx).produced = (*mtctx)
                .produced
                .wrapping_add(cSize as ::core::ffi::c_ulonglong);
            (*mtctx).doneJobID = (*mtctx).doneJobID.wrapping_add(1);
        }
    }
    if cSize > (*(*mtctx).jobs.offset(wJobID as isize)).dstFlushed {
        return cSize.wrapping_sub((*(*mtctx).jobs.offset(wJobID as isize)).dstFlushed);
    }
    if srcSize > srcConsumed {
        return 1 as size_t;
    }
    if (*mtctx).doneJobID < (*mtctx).nextJobID {
        return 1 as size_t;
    }
    if (*mtctx).jobReady != 0 {
        return 1 as size_t;
    }
    if (*mtctx).inBuff.filled > 0 as size_t {
        return 1 as size_t;
    }
    (*mtctx).allJobsCompleted = (*mtctx).frameEnded;
    if end as ::core::ffi::c_uint == ZSTD_e_end as ::core::ffi::c_int as ::core::ffi::c_uint {
        return ((*mtctx).frameEnded == 0) as ::core::ffi::c_int as size_t;
    }
    return 0 as size_t;
}
unsafe extern "C" fn ZSTDMT_getInputDataInUse(mut mtctx: *mut ZSTDMT_CCtx) -> Range {
    let firstJobID: ::core::ffi::c_uint = (*mtctx).doneJobID;
    let lastJobID: ::core::ffi::c_uint = (*mtctx).nextJobID;
    let mut jobID: ::core::ffi::c_uint = 0;
    let mut roundBuffCapacity: size_t = (*mtctx).roundBuff.capacity;
    let mut nbJobs1stRoundMin: size_t = roundBuffCapacity.wrapping_div((*mtctx).targetSectionSize);
    if (lastJobID as size_t) < nbJobs1stRoundMin {
        return kNullRange;
    }
    jobID = firstJobID;
    while jobID < lastJobID {
        let wJobID: ::core::ffi::c_uint = jobID & (*mtctx).jobIDMask;
        let mut consumed: size_t = 0;
        &raw mut (*(*mtctx).jobs.offset(wJobID as isize)).job_mutex;
        consumed = (*(*mtctx).jobs.offset(wJobID as isize)).consumed;
        &raw mut (*(*mtctx).jobs.offset(wJobID as isize)).job_mutex;
        if consumed < (*(*mtctx).jobs.offset(wJobID as isize)).src.size {
            let mut range: Range = (*(*mtctx).jobs.offset(wJobID as isize)).prefix;
            if range.size == 0 as size_t {
                range = (*(*mtctx).jobs.offset(wJobID as isize)).src;
            }
            return range;
        }
        jobID = jobID.wrapping_add(1);
    }
    return kNullRange;
}
unsafe extern "C" fn ZSTDMT_isOverlapped(
    mut buffer: Buffer,
    mut range: Range,
) -> ::core::ffi::c_int {
    let bufferStart: *const BYTE = buffer.start as *const BYTE;
    let rangeStart: *const BYTE = range.start as *const BYTE;
    if rangeStart.is_null() || bufferStart.is_null() {
        return 0 as ::core::ffi::c_int;
    }
    let bufferEnd: *const BYTE = bufferStart.offset(buffer.capacity as isize);
    let rangeEnd: *const BYTE = rangeStart.offset(range.size as isize);
    if bufferStart == bufferEnd || rangeStart == rangeEnd {
        return 0 as ::core::ffi::c_int;
    }
    return (bufferStart < rangeEnd && rangeStart < bufferEnd) as ::core::ffi::c_int;
}
unsafe extern "C" fn ZSTDMT_doesOverlapWindow(
    mut buffer: Buffer,
    mut window: ZSTD_window_t,
) -> ::core::ffi::c_int {
    let mut extDict: Range = Range {
        start: ::core::ptr::null::<::core::ffi::c_void>(),
        size: 0,
    };
    let mut prefix: Range = Range {
        start: ::core::ptr::null::<::core::ffi::c_void>(),
        size: 0,
    };
    extDict.start = window.dictBase.offset(window.lowLimit as isize) as *const ::core::ffi::c_void;
    extDict.size = window.dictLimit.wrapping_sub(window.lowLimit) as size_t;
    prefix.start = window.base.offset(window.dictLimit as isize) as *const ::core::ffi::c_void;
    prefix.size = window
        .nextSrc
        .offset_from(window.base.offset(window.dictLimit as isize))
        as ::core::ffi::c_long as size_t;
    return (ZSTDMT_isOverlapped(buffer, extDict) != 0 || ZSTDMT_isOverlapped(buffer, prefix) != 0)
        as ::core::ffi::c_int;
}
unsafe extern "C" fn ZSTDMT_waitForLdmComplete(mut mtctx: *mut ZSTDMT_CCtx, mut buffer: Buffer) {
    if (*mtctx).params.ldmParams.enableLdm as ::core::ffi::c_uint
        == ZSTD_ps_enable as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        let mut mutex: *mut ZSTD_pthread_mutex_t = &raw mut (*mtctx).serial.ldmWindowMutex;
        while ZSTDMT_doesOverlapWindow(buffer, (*mtctx).serial.ldmWindow) != 0 {
            &raw mut (*mtctx).serial.ldmWindowCond;
        }
    }
}
unsafe extern "C" fn ZSTDMT_tryGetInputRange(mut mtctx: *mut ZSTDMT_CCtx) -> ::core::ffi::c_int {
    let inUse: Range = ZSTDMT_getInputDataInUse(mtctx) as Range;
    let spaceLeft: size_t = (*mtctx)
        .roundBuff
        .capacity
        .wrapping_sub((*mtctx).roundBuff.pos);
    let spaceNeeded: size_t = (*mtctx).targetSectionSize;
    let mut buffer: Buffer = Buffer {
        start: ::core::ptr::null_mut::<::core::ffi::c_void>(),
        capacity: 0,
    };
    if spaceLeft < spaceNeeded {
        let start: *mut BYTE = (*mtctx).roundBuff.buffer;
        let prefixSize: size_t = (*mtctx).inBuff.prefix.size;
        buffer.start = start as *mut ::core::ffi::c_void;
        buffer.capacity = prefixSize;
        if ZSTDMT_isOverlapped(buffer, inUse) != 0 {
            return 0 as ::core::ffi::c_int;
        }
        ZSTDMT_waitForLdmComplete(mtctx, buffer);
        ::libc::memmove(
            start as *mut ::core::ffi::c_void,
            (*mtctx).inBuff.prefix.start,
            prefixSize as ::libc::size_t,
        );
        (*mtctx).inBuff.prefix.start = start as *const ::core::ffi::c_void;
        (*mtctx).roundBuff.pos = prefixSize;
    }
    buffer.start = (*mtctx)
        .roundBuff
        .buffer
        .offset((*mtctx).roundBuff.pos as isize) as *mut ::core::ffi::c_void;
    buffer.capacity = spaceNeeded;
    if ZSTDMT_isOverlapped(buffer, inUse) != 0 {
        return 0 as ::core::ffi::c_int;
    }
    ZSTDMT_waitForLdmComplete(mtctx, buffer);
    (*mtctx).inBuff.buffer = buffer;
    (*mtctx).inBuff.filled = 0 as size_t;
    return 1 as ::core::ffi::c_int;
}
unsafe extern "C" fn findSynchronizationPoint(
    mut mtctx: *const ZSTDMT_CCtx,
    input: ZSTD_inBuffer,
) -> SyncPoint {
    let istart: *const BYTE = (input.src as *const BYTE).offset(input.pos as isize);
    let primePower: U64 = (*mtctx).rsync.primePower;
    let hitMask: U64 = (*mtctx).rsync.hitMask;
    let mut syncPoint: SyncPoint = SyncPoint {
        toLoad: 0,
        flush: 0,
    };
    let mut hash: U64 = 0;
    let mut prev: *const BYTE = ::core::ptr::null::<BYTE>();
    let mut pos: size_t = 0;
    syncPoint.toLoad = if input.size.wrapping_sub(input.pos)
        < (*mtctx)
            .targetSectionSize
            .wrapping_sub((*mtctx).inBuff.filled)
    {
        input.size.wrapping_sub(input.pos)
    } else {
        (*mtctx)
            .targetSectionSize
            .wrapping_sub((*mtctx).inBuff.filled)
    };
    syncPoint.flush = 0 as ::core::ffi::c_int;
    if (*mtctx).params.rsyncable == 0 {
        return syncPoint;
    }
    if (*mtctx)
        .inBuff
        .filled
        .wrapping_add(input.size)
        .wrapping_sub(input.pos)
        < RSYNC_MIN_BLOCK_SIZE as size_t
    {
        return syncPoint;
    }
    if (*mtctx).inBuff.filled.wrapping_add(syncPoint.toLoad) < RSYNC_LENGTH as size_t {
        return syncPoint;
    }
    if (*mtctx).inBuff.filled < RSYNC_MIN_BLOCK_SIZE as size_t {
        pos = (RSYNC_MIN_BLOCK_SIZE as size_t).wrapping_sub((*mtctx).inBuff.filled);
        if pos >= RSYNC_LENGTH as size_t {
            prev = istart.offset(pos as isize).offset(-(RSYNC_LENGTH as isize));
            hash = ZSTD_rollingHash_compute(
                prev as *const ::core::ffi::c_void,
                RSYNC_LENGTH as size_t,
            );
        } else {
            prev = ((*mtctx).inBuff.buffer.start as *const BYTE)
                .offset((*mtctx).inBuff.filled as isize)
                .offset(-(RSYNC_LENGTH as isize));
            hash = ZSTD_rollingHash_compute(
                prev.offset(pos as isize) as *const ::core::ffi::c_void,
                (RSYNC_LENGTH as size_t).wrapping_sub(pos),
            );
            hash = ZSTD_rollingHash_append(hash, istart as *const ::core::ffi::c_void, pos);
        }
    } else {
        pos = 0 as size_t;
        prev = ((*mtctx).inBuff.buffer.start as *const BYTE)
            .offset((*mtctx).inBuff.filled as isize)
            .offset(-(RSYNC_LENGTH as isize));
        hash = ZSTD_rollingHash_compute(prev as *const ::core::ffi::c_void, RSYNC_LENGTH as size_t);
        if hash & hitMask == hitMask {
            syncPoint.toLoad = 0 as size_t;
            syncPoint.flush = 1 as ::core::ffi::c_int;
            return syncPoint;
        }
    }
    while pos < syncPoint.toLoad {
        let toRemove: BYTE = (if pos < RSYNC_LENGTH as size_t {
            *prev.offset(pos as isize) as ::core::ffi::c_int
        } else {
            *istart.offset(pos.wrapping_sub(RSYNC_LENGTH as size_t) as isize) as ::core::ffi::c_int
        }) as BYTE;
        hash = ZSTD_rollingHash_rotate(hash, toRemove, *istart.offset(pos as isize), primePower);
        if hash & hitMask == hitMask {
            syncPoint.toLoad = pos.wrapping_add(1 as size_t);
            syncPoint.flush = 1 as ::core::ffi::c_int;
            pos = pos.wrapping_add(1);
            break;
        } else {
            pos = pos.wrapping_add(1);
        }
    }
    return syncPoint;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDMT_nextInputSizeHint(mut mtctx: *const ZSTDMT_CCtx) -> size_t {
    let mut hintInSize: size_t = (*mtctx)
        .targetSectionSize
        .wrapping_sub((*mtctx).inBuff.filled);
    if hintInSize == 0 as size_t {
        hintInSize = (*mtctx).targetSectionSize;
    }
    return hintInSize;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTDMT_compressStream_generic(
    mut mtctx: *mut ZSTDMT_CCtx,
    mut output: *mut ZSTD_outBuffer,
    mut input: *mut ZSTD_inBuffer,
    mut endOp: ZSTD_EndDirective,
) -> size_t {
    let mut forwardInputProgress: ::core::ffi::c_uint = 0 as ::core::ffi::c_uint;
    if (*mtctx).frameEnded != 0
        && endOp as ::core::ffi::c_uint
            == ZSTD_e_continue as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        return -(ZSTD_error_stage_wrong as ::core::ffi::c_int) as size_t;
    }
    if (*mtctx).jobReady == 0 && (*input).size > (*input).pos {
        if (*mtctx).inBuff.buffer.start.is_null() {
            ZSTDMT_tryGetInputRange(mtctx) == 0;
        }
        if !(*mtctx).inBuff.buffer.start.is_null() {
            let syncPoint: SyncPoint = findSynchronizationPoint(mtctx, *input) as SyncPoint;
            if syncPoint.flush != 0
                && endOp as ::core::ffi::c_uint
                    == ZSTD_e_continue as ::core::ffi::c_int as ::core::ffi::c_uint
            {
                endOp = ZSTD_e_flush;
            }
            ::libc::memcpy(
                ((*mtctx).inBuff.buffer.start as *mut ::core::ffi::c_char)
                    .offset((*mtctx).inBuff.filled as isize)
                    as *mut ::core::ffi::c_void,
                ((*input).src as *const ::core::ffi::c_char).offset((*input).pos as isize)
                    as *const ::core::ffi::c_void,
                syncPoint.toLoad as ::libc::size_t,
            );
            (*input).pos = ((*input).pos as ::core::ffi::c_ulong)
                .wrapping_add(syncPoint.toLoad as ::core::ffi::c_ulong)
                as size_t as size_t;
            (*mtctx).inBuff.filled = ((*mtctx).inBuff.filled as ::core::ffi::c_ulong)
                .wrapping_add(syncPoint.toLoad as ::core::ffi::c_ulong)
                as size_t as size_t;
            forwardInputProgress =
                (syncPoint.toLoad > 0 as size_t) as ::core::ffi::c_int as ::core::ffi::c_uint;
        }
    }
    if (*input).pos < (*input).size
        && endOp as ::core::ffi::c_uint == ZSTD_e_end as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        endOp = ZSTD_e_flush;
    }
    if (*mtctx).jobReady != 0
        || (*mtctx).inBuff.filled >= (*mtctx).targetSectionSize
        || endOp as ::core::ffi::c_uint
            != ZSTD_e_continue as ::core::ffi::c_int as ::core::ffi::c_uint
            && (*mtctx).inBuff.filled > 0 as size_t
        || endOp as ::core::ffi::c_uint == ZSTD_e_end as ::core::ffi::c_int as ::core::ffi::c_uint
            && (*mtctx).frameEnded == 0
    {
        let jobSize: size_t = (*mtctx).inBuff.filled;
        let err_code: size_t = ZSTDMT_createCompressionJob(mtctx, jobSize, endOp) as size_t;
        if ERR_isError(err_code) != 0 {
            return err_code;
        }
    }
    let remainingToFlush: size_t = ZSTDMT_flushProduced(
        mtctx,
        output,
        (forwardInputProgress == 0) as ::core::ffi::c_int as ::core::ffi::c_uint,
        endOp,
    ) as size_t;
    if (*input).pos < (*input).size {
        return if remainingToFlush > 1 as size_t {
            remainingToFlush
        } else {
            1 as size_t
        };
    }
    return remainingToFlush;
}
