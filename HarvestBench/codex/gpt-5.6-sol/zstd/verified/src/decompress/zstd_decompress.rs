use ::libc;
extern "C" {
    pub type ZSTD_DDict_s;
    pub type ZBUFFv07_DCtx_s;
    pub type ZBUFFv06_DCtx_s;
    pub type ZBUFFv05_DCtx_s;
    pub type ZSTD_CCtx_s;
    pub type ZSTD_CCtx_params_s;
    pub type ZSTDv07_DCtx_s;
    pub type ZSTDv06_DCtx_s;
    pub type ZSTDv05_DCtx_s;
    fn malloc(__size: size_t) -> *mut ::core::ffi::c_void;
    fn calloc(__nmemb: size_t, __size: size_t) -> *mut ::core::ffi::c_void;
    fn free(__ptr: *mut ::core::ffi::c_void);
    fn ZSTD_getErrorCode(functionResult: size_t) -> ZSTD_ErrorCode;
    fn ZSTD_freeDDict(ddict: *mut ZSTD_DDict) -> size_t;
    fn ZSTD_getDictID_fromDDict(ddict: *const ZSTD_DDict) -> ::core::ffi::c_uint;
    fn ZSTD_sizeof_DDict(ddict: *const ZSTD_DDict) -> size_t;
    fn ZSTD_createDDict_advanced(
        dict: *const ::core::ffi::c_void,
        dictSize: size_t,
        dictLoadMethod: ZSTD_dictLoadMethod_e,
        dictContentType: ZSTD_dictContentType_e,
        customMem: ZSTD_customMem,
    ) -> *mut ZSTD_DDict;
    fn FSE_readNCount(
        normalizedCounter: *mut ::core::ffi::c_short,
        maxSymbolValuePtr: *mut ::core::ffi::c_uint,
        tableLogPtr: *mut ::core::ffi::c_uint,
        rBuffer: *const ::core::ffi::c_void,
        rBuffSize: size_t,
    ) -> size_t;
    fn HUF_readDTableX2_wksp(
        DTable: *mut HUF_DTable,
        src: *const ::core::ffi::c_void,
        srcSize: size_t,
        workSpace: *mut ::core::ffi::c_void,
        wkspSize: size_t,
        flags: ::core::ffi::c_int,
    ) -> size_t;
    fn ZSTD_XXH64(
        input: *const ::core::ffi::c_void,
        length: size_t,
        seed: XXH64_hash_t,
    ) -> XXH64_hash_t;
    fn ZSTD_XXH64_reset(statePtr: *mut XXH64_state_t, seed: XXH64_hash_t) -> XXH_errorcode;
    fn ZSTD_XXH64_update(
        statePtr: *mut XXH64_state_t,
        input: *const ::core::ffi::c_void,
        length: size_t,
    ) -> XXH_errorcode;
    fn ZSTD_XXH64_digest(statePtr: *const XXH64_state_t) -> XXH64_hash_t;
    fn ZSTD_getcBlockSize(
        src: *const ::core::ffi::c_void,
        srcSize: size_t,
        bpPtr: *mut blockProperties_t,
    ) -> size_t;
    fn ZSTD_checkContinuity(dctx: *mut ZSTD_DCtx, dst: *const ::core::ffi::c_void, dstSize: size_t);
    fn ZSTD_DDict_dictContent(ddict: *const ZSTD_DDict) -> *const ::core::ffi::c_void;
    fn ZSTD_DDict_dictSize(ddict: *const ZSTD_DDict) -> size_t;
    fn ZSTD_copyDDictParameters(dctx: *mut ZSTD_DCtx, ddict: *const ZSTD_DDict);
    fn ZSTD_decompressBlock_internal(
        dctx: *mut ZSTD_DCtx,
        dst: *mut ::core::ffi::c_void,
        dstCapacity: size_t,
        src: *const ::core::ffi::c_void,
        srcSize: size_t,
        streaming: streaming_operation,
    ) -> size_t;
    fn ZSTD_buildFSETable(
        dt: *mut ZSTD_seqSymbol,
        normalizedCounter: *const ::core::ffi::c_short,
        maxSymbolValue: ::core::ffi::c_uint,
        baseValue: *const U32,
        nbAdditionalBits: *const U8,
        tableLog: ::core::ffi::c_uint,
        wksp: *mut ::core::ffi::c_void,
        wkspSize: size_t,
        bmi2: ::core::ffi::c_int,
    );
    fn ZSTDv05_findFrameSizeInfoLegacy(
        src: *const ::core::ffi::c_void,
        srcSize: size_t,
        cSize: *mut size_t,
        dBound: *mut ::core::ffi::c_ulonglong,
    );
    fn ZSTDv05_createDCtx() -> *mut ZSTDv05_DCtx;
    fn ZSTDv05_freeDCtx(dctx: *mut ZSTDv05_DCtx) -> size_t;
    fn ZSTDv05_decompress_usingDict(
        dctx: *mut ZSTDv05_DCtx,
        dst: *mut ::core::ffi::c_void,
        dstCapacity: size_t,
        src: *const ::core::ffi::c_void,
        srcSize: size_t,
        dict: *const ::core::ffi::c_void,
        dictSize: size_t,
    ) -> size_t;
    fn ZSTDv05_getFrameParams(
        params: *mut ZSTDv05_parameters,
        src: *const ::core::ffi::c_void,
        srcSize: size_t,
    ) -> size_t;
    fn ZBUFFv05_createDCtx() -> *mut ZBUFFv05_DCtx;
    fn ZBUFFv05_freeDCtx(dctx: *mut ZBUFFv05_DCtx) -> size_t;
    fn ZBUFFv05_decompressInitDictionary(
        dctx: *mut ZBUFFv05_DCtx,
        dict: *const ::core::ffi::c_void,
        dictSize: size_t,
    ) -> size_t;
    fn ZBUFFv05_decompressContinue(
        dctx: *mut ZBUFFv05_DCtx,
        dst: *mut ::core::ffi::c_void,
        dstCapacityPtr: *mut size_t,
        src: *const ::core::ffi::c_void,
        srcSizePtr: *mut size_t,
    ) -> size_t;
    fn ZSTDv06_findFrameSizeInfoLegacy(
        src: *const ::core::ffi::c_void,
        srcSize: size_t,
        cSize: *mut size_t,
        dBound: *mut ::core::ffi::c_ulonglong,
    );
    fn ZSTDv06_createDCtx() -> *mut ZSTDv06_DCtx;
    fn ZSTDv06_freeDCtx(dctx: *mut ZSTDv06_DCtx) -> size_t;
    fn ZSTDv06_decompress_usingDict(
        dctx: *mut ZSTDv06_DCtx,
        dst: *mut ::core::ffi::c_void,
        dstCapacity: size_t,
        src: *const ::core::ffi::c_void,
        srcSize: size_t,
        dict: *const ::core::ffi::c_void,
        dictSize: size_t,
    ) -> size_t;
    fn ZSTDv06_getFrameParams(
        fparamsPtr: *mut ZSTDv06_frameParams,
        src: *const ::core::ffi::c_void,
        srcSize: size_t,
    ) -> size_t;
    fn ZBUFFv06_createDCtx() -> *mut ZBUFFv06_DCtx;
    fn ZBUFFv06_freeDCtx(dctx: *mut ZBUFFv06_DCtx) -> size_t;
    fn ZBUFFv06_decompressInitDictionary(
        dctx: *mut ZBUFFv06_DCtx,
        dict: *const ::core::ffi::c_void,
        dictSize: size_t,
    ) -> size_t;
    fn ZBUFFv06_decompressContinue(
        dctx: *mut ZBUFFv06_DCtx,
        dst: *mut ::core::ffi::c_void,
        dstCapacityPtr: *mut size_t,
        src: *const ::core::ffi::c_void,
        srcSizePtr: *mut size_t,
    ) -> size_t;
    fn ZSTDv07_findFrameSizeInfoLegacy(
        src: *const ::core::ffi::c_void,
        srcSize: size_t,
        cSize: *mut size_t,
        dBound: *mut ::core::ffi::c_ulonglong,
    );
    fn ZSTDv07_createDCtx() -> *mut ZSTDv07_DCtx;
    fn ZSTDv07_freeDCtx(dctx: *mut ZSTDv07_DCtx) -> size_t;
    fn ZSTDv07_decompress_usingDict(
        dctx: *mut ZSTDv07_DCtx,
        dst: *mut ::core::ffi::c_void,
        dstCapacity: size_t,
        src: *const ::core::ffi::c_void,
        srcSize: size_t,
        dict: *const ::core::ffi::c_void,
        dictSize: size_t,
    ) -> size_t;
    fn ZSTDv07_getFrameParams(
        fparamsPtr: *mut ZSTDv07_frameParams,
        src: *const ::core::ffi::c_void,
        srcSize: size_t,
    ) -> size_t;
    fn ZBUFFv07_createDCtx() -> *mut ZBUFFv07_DCtx;
    fn ZBUFFv07_freeDCtx(dctx: *mut ZBUFFv07_DCtx) -> size_t;
    fn ZBUFFv07_decompressInitDictionary(
        dctx: *mut ZBUFFv07_DCtx,
        dict: *const ::core::ffi::c_void,
        dictSize: size_t,
    ) -> size_t;
    fn ZBUFFv07_decompressContinue(
        dctx: *mut ZBUFFv07_DCtx,
        dst: *mut ::core::ffi::c_void,
        dstCapacityPtr: *mut size_t,
        src: *const ::core::ffi::c_void,
        srcSizePtr: *mut size_t,
    ) -> size_t;
}
pub type size_t = usize;
pub type __uint8_t = u8;
pub type __uint16_t = u16;
pub type __uint32_t = u32;
pub type __uint64_t = u64;
pub type ZSTD_ErrorCode = ::core::ffi::c_uint;
pub const ZSTD_error_maxCode: ZSTD_ErrorCode = 120;
pub const ZSTD_error_externalSequences_invalid: ZSTD_ErrorCode = 107;
pub const ZSTD_error_sequenceProducer_failed: ZSTD_ErrorCode = 106;
pub const ZSTD_error_srcBuffer_wrong: ZSTD_ErrorCode = 105;
pub const ZSTD_error_dstBuffer_wrong: ZSTD_ErrorCode = 104;
pub const ZSTD_error_seekableIO: ZSTD_ErrorCode = 102;
pub const ZSTD_error_frameIndex_tooLarge: ZSTD_ErrorCode = 100;
pub const ZSTD_error_noForwardProgress_inputEmpty: ZSTD_ErrorCode = 82;
pub const ZSTD_error_noForwardProgress_destFull: ZSTD_ErrorCode = 80;
pub const ZSTD_error_dstBuffer_null: ZSTD_ErrorCode = 74;
pub const ZSTD_error_srcSize_wrong: ZSTD_ErrorCode = 72;
pub const ZSTD_error_dstSize_tooSmall: ZSTD_ErrorCode = 70;
pub const ZSTD_error_workSpace_tooSmall: ZSTD_ErrorCode = 66;
pub const ZSTD_error_memory_allocation: ZSTD_ErrorCode = 64;
pub const ZSTD_error_init_missing: ZSTD_ErrorCode = 62;
pub const ZSTD_error_stage_wrong: ZSTD_ErrorCode = 60;
pub const ZSTD_error_stabilityCondition_notRespected: ZSTD_ErrorCode = 50;
pub const ZSTD_error_cannotProduce_uncompressedBlock: ZSTD_ErrorCode = 49;
pub const ZSTD_error_maxSymbolValue_tooSmall: ZSTD_ErrorCode = 48;
pub const ZSTD_error_maxSymbolValue_tooLarge: ZSTD_ErrorCode = 46;
pub const ZSTD_error_tableLog_tooLarge: ZSTD_ErrorCode = 44;
pub const ZSTD_error_parameter_outOfBound: ZSTD_ErrorCode = 42;
pub const ZSTD_error_parameter_combination_unsupported: ZSTD_ErrorCode = 41;
pub const ZSTD_error_parameter_unsupported: ZSTD_ErrorCode = 40;
pub const ZSTD_error_dictionaryCreation_failed: ZSTD_ErrorCode = 34;
pub const ZSTD_error_dictionary_wrong: ZSTD_ErrorCode = 32;
pub const ZSTD_error_dictionary_corrupted: ZSTD_ErrorCode = 30;
pub const ZSTD_error_literals_headerWrong: ZSTD_ErrorCode = 24;
pub const ZSTD_error_checksum_wrong: ZSTD_ErrorCode = 22;
pub const ZSTD_error_corruption_detected: ZSTD_ErrorCode = 20;
pub const ZSTD_error_frameParameter_windowTooLarge: ZSTD_ErrorCode = 16;
pub const ZSTD_error_frameParameter_unsupported: ZSTD_ErrorCode = 14;
pub const ZSTD_error_version_unsupported: ZSTD_ErrorCode = 12;
pub const ZSTD_error_prefix_unknown: ZSTD_ErrorCode = 10;
pub const ZSTD_error_GENERIC: ZSTD_ErrorCode = 1;
pub const ZSTD_error_no_error: ZSTD_ErrorCode = 0;
pub type ZSTD_DCtx = ZSTD_DCtx_s;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ZSTD_DCtx_s {
    pub LLTptr: *const ZSTD_seqSymbol,
    pub MLTptr: *const ZSTD_seqSymbol,
    pub OFTptr: *const ZSTD_seqSymbol,
    pub HUFptr: *const HUF_DTable,
    pub entropy: ZSTD_entropyDTables_t,
    pub workspace: [U32; 640],
    pub previousDstEnd: *const ::core::ffi::c_void,
    pub prefixStart: *const ::core::ffi::c_void,
    pub virtualStart: *const ::core::ffi::c_void,
    pub dictEnd: *const ::core::ffi::c_void,
    pub expected: size_t,
    pub fParams: ZSTD_FrameHeader,
    pub processedCSize: U64,
    pub decodedSize: U64,
    pub bType: blockType_e,
    pub stage: ZSTD_dStage,
    pub litEntropy: U32,
    pub fseEntropy: U32,
    pub xxhState: XXH64_state_t,
    pub headerSize: size_t,
    pub format: ZSTD_format_e,
    pub forceIgnoreChecksum: ZSTD_forceIgnoreChecksum_e,
    pub validateChecksum: U32,
    pub litPtr: *const BYTE,
    pub customMem: ZSTD_customMem,
    pub litSize: size_t,
    pub rleSize: size_t,
    pub staticSize: size_t,
    pub isFrameDecompression: ::core::ffi::c_int,
    pub ddictLocal: *mut ZSTD_DDict,
    pub ddict: *const ZSTD_DDict,
    pub dictID: U32,
    pub ddictIsCold: ::core::ffi::c_int,
    pub dictUses: ZSTD_dictUses_e,
    pub ddictSet: *mut ZSTD_DDictHashSet,
    pub refMultipleDDicts: ZSTD_refMultipleDDicts_e,
    pub disableHufAsm: ::core::ffi::c_int,
    pub maxBlockSizeParam: ::core::ffi::c_int,
    pub streamStage: ZSTD_dStreamStage,
    pub inBuff: *mut ::core::ffi::c_char,
    pub inBuffSize: size_t,
    pub inPos: size_t,
    pub maxWindowSize: size_t,
    pub outBuff: *mut ::core::ffi::c_char,
    pub outBuffSize: size_t,
    pub outStart: size_t,
    pub outEnd: size_t,
    pub lhSize: size_t,
    pub legacyContext: *mut ::core::ffi::c_void,
    pub previousLegacyVersion: U32,
    pub legacyVersion: U32,
    pub hostageByte: U32,
    pub noForwardProgress: ::core::ffi::c_int,
    pub outBufferMode: ZSTD_bufferMode_e,
    pub expectedOutBuffer: ZSTD_outBuffer,
    pub litBuffer: *mut BYTE,
    pub litBufferEnd: *const BYTE,
    pub litBufferLocation: ZSTD_litLocation_e,
    pub litExtraBuffer: [BYTE; 65568],
    pub headerBuffer: [BYTE; 18],
    pub oversizedDuration: size_t,
    pub traceCtx: ZSTD_TraceCtx,
}
pub type ZSTD_TraceCtx = ::core::ffi::c_ulonglong;
pub type BYTE = uint8_t;
pub type uint8_t = __uint8_t;
pub type ZSTD_litLocation_e = ::core::ffi::c_uint;
pub const ZSTD_split: ZSTD_litLocation_e = 2;
pub const ZSTD_in_dst: ZSTD_litLocation_e = 1;
pub const ZSTD_not_in_dst: ZSTD_litLocation_e = 0;
pub type ZSTD_outBuffer = ZSTD_outBuffer_s;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ZSTD_outBuffer_s {
    pub dst: *mut ::core::ffi::c_void,
    pub size: size_t,
    pub pos: size_t,
}
pub type ZSTD_bufferMode_e = ::core::ffi::c_uint;
pub const ZSTD_bm_stable: ZSTD_bufferMode_e = 1;
pub const ZSTD_bm_buffered: ZSTD_bufferMode_e = 0;
pub type U32 = uint32_t;
pub type uint32_t = __uint32_t;
pub type ZSTD_dStreamStage = ::core::ffi::c_uint;
pub const zdss_flush: ZSTD_dStreamStage = 4;
pub const zdss_load: ZSTD_dStreamStage = 3;
pub const zdss_read: ZSTD_dStreamStage = 2;
pub const zdss_loadHeader: ZSTD_dStreamStage = 1;
pub const zdss_init: ZSTD_dStreamStage = 0;
pub type ZSTD_refMultipleDDicts_e = ::core::ffi::c_uint;
pub const ZSTD_rmd_refMultipleDDicts: ZSTD_refMultipleDDicts_e = 1;
pub const ZSTD_rmd_refSingleDDict: ZSTD_refMultipleDDicts_e = 0;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ZSTD_DDictHashSet {
    pub ddictPtrTable: *mut *const ZSTD_DDict,
    pub ddictPtrTableSize: size_t,
    pub ddictPtrCount: size_t,
}
pub type ZSTD_DDict = ZSTD_DDict_s;
pub type ZSTD_dictUses_e = ::core::ffi::c_int;
pub const ZSTD_use_once: ZSTD_dictUses_e = 1;
pub const ZSTD_dont_use: ZSTD_dictUses_e = 0;
pub const ZSTD_use_indefinitely: ZSTD_dictUses_e = -1;
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
pub type ZSTD_forceIgnoreChecksum_e = ::core::ffi::c_uint;
pub const ZSTD_d_ignoreChecksum: ZSTD_forceIgnoreChecksum_e = 1;
pub const ZSTD_d_validateChecksum: ZSTD_forceIgnoreChecksum_e = 0;
pub type ZSTD_format_e = ::core::ffi::c_uint;
pub const ZSTD_f_zstd1_magicless: ZSTD_format_e = 1;
pub const ZSTD_f_zstd1: ZSTD_format_e = 0;
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
pub type uint64_t = __uint64_t;
pub type XXH32_hash_t = uint32_t;
pub type ZSTD_dStage = ::core::ffi::c_uint;
pub const ZSTDds_skipFrame: ZSTD_dStage = 7;
pub const ZSTDds_decodeSkippableHeader: ZSTD_dStage = 6;
pub const ZSTDds_checkChecksum: ZSTD_dStage = 5;
pub const ZSTDds_decompressLastBlock: ZSTD_dStage = 4;
pub const ZSTDds_decompressBlock: ZSTD_dStage = 3;
pub const ZSTDds_decodeBlockHeader: ZSTD_dStage = 2;
pub const ZSTDds_decodeFrameHeader: ZSTD_dStage = 1;
pub const ZSTDds_getFrameHeaderSize: ZSTD_dStage = 0;
pub type blockType_e = ::core::ffi::c_uint;
pub const bt_reserved: blockType_e = 3;
pub const bt_compressed: blockType_e = 2;
pub const bt_rle: blockType_e = 1;
pub const bt_raw: blockType_e = 0;
pub type U64 = uint64_t;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ZSTD_FrameHeader {
    pub frameContentSize: ::core::ffi::c_ulonglong,
    pub windowSize: ::core::ffi::c_ulonglong,
    pub blockSizeMax: ::core::ffi::c_uint,
    pub frameType: ZSTD_FrameType_e,
    pub headerSize: ::core::ffi::c_uint,
    pub dictID: ::core::ffi::c_uint,
    pub checksumFlag: ::core::ffi::c_uint,
    pub _reserved1: ::core::ffi::c_uint,
    pub _reserved2: ::core::ffi::c_uint,
}
pub type ZSTD_FrameType_e = ::core::ffi::c_uint;
pub const ZSTD_skippableFrame: ZSTD_FrameType_e = 1;
pub const ZSTD_frame: ZSTD_FrameType_e = 0;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ZSTD_entropyDTables_t {
    pub LLTable: [ZSTD_seqSymbol; 513],
    pub OFTable: [ZSTD_seqSymbol; 257],
    pub MLTable: [ZSTD_seqSymbol; 513],
    pub hufTable: [HUF_DTable; 4097],
    pub rep: [U32; 3],
    pub workspace: [U32; 157],
}
pub type HUF_DTable = U32;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ZSTD_seqSymbol {
    pub nextState: U16,
    pub nbAdditionalBits: BYTE,
    pub nbBits: BYTE,
    pub baseValue: U32,
}
pub type U16 = uint16_t;
pub type uint16_t = __uint16_t;
pub type ZBUFFv07_DCtx = ZBUFFv07_DCtx_s;
pub type ZBUFFv06_DCtx = ZBUFFv06_DCtx_s;
pub type ZBUFFv05_DCtx = ZBUFFv05_DCtx_s;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ZSTD_Trace {
    pub version: ::core::ffi::c_uint,
    pub streaming: ::core::ffi::c_int,
    pub dictionaryID: ::core::ffi::c_uint,
    pub dictionaryIsCold: ::core::ffi::c_int,
    pub dictionarySize: size_t,
    pub uncompressedSize: size_t,
    pub compressedSize: size_t,
    pub params: *const ZSTD_CCtx_params_s,
    pub cctx: *const ZSTD_CCtx_s,
    pub dctx: *const ZSTD_DCtx_s,
}
pub type unalign32 = U32;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct blockProperties_t {
    pub blockType: blockType_e,
    pub lastBlock: U32,
    pub origSize: U32,
}
pub type XXH_errorcode = ::core::ffi::c_uint;
pub const XXH_ERROR: XXH_errorcode = 1;
pub const XXH_OK: XXH_errorcode = 0;
pub type streaming_operation = ::core::ffi::c_uint;
pub const is_streaming: streaming_operation = 1;
pub const not_streaming: streaming_operation = 0;
pub type unalign64 = U64;
pub type unalign16 = U16;
pub type U8 = uint8_t;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ZSTD_frameSizeInfo {
    pub nbBlocks: size_t,
    pub compressedSize: size_t,
    pub decompressedBound: ::core::ffi::c_ulonglong,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ZSTDv07_frameParams {
    pub frameContentSize: ::core::ffi::c_ulonglong,
    pub windowSize: ::core::ffi::c_uint,
    pub dictID: ::core::ffi::c_uint,
    pub checksumFlag: ::core::ffi::c_uint,
}
pub type ZSTDv06_frameParams = ZSTDv06_frameParams_s;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ZSTDv06_frameParams_s {
    pub frameContentSize: ::core::ffi::c_ulonglong,
    pub windowLog: ::core::ffi::c_uint,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ZSTDv05_parameters {
    pub srcSize: U64,
    pub windowLog: U32,
    pub contentLog: U32,
    pub hashLog: U32,
    pub searchLog: U32,
    pub searchLength: U32,
    pub targetLength: U32,
    pub strategy: ZSTDv05_strategy,
}
pub type ZSTDv05_strategy = ::core::ffi::c_uint;
pub const ZSTDv05_btopt: ZSTDv05_strategy = 6;
pub const ZSTDv05_opt: ZSTDv05_strategy = 5;
pub const ZSTDv05_btlazy2: ZSTDv05_strategy = 4;
pub const ZSTDv05_lazy2: ZSTDv05_strategy = 3;
pub const ZSTDv05_lazy: ZSTDv05_strategy = 2;
pub const ZSTDv05_greedy: ZSTDv05_strategy = 1;
pub const ZSTDv05_fast: ZSTDv05_strategy = 0;
pub type ZSTDv07_DCtx = ZSTDv07_DCtx_s;
pub type ZSTDv06_DCtx = ZSTDv06_DCtx_s;
pub type ZSTDv05_DCtx = ZSTDv05_DCtx_s;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ZSTD_bounds {
    pub error: size_t,
    pub lowerBound: ::core::ffi::c_int,
    pub upperBound: ::core::ffi::c_int,
}
pub type ZSTD_ResetDirective = ::core::ffi::c_uint;
pub const ZSTD_reset_session_and_parameters: ZSTD_ResetDirective = 3;
pub const ZSTD_reset_parameters: ZSTD_ResetDirective = 2;
pub const ZSTD_reset_session_only: ZSTD_ResetDirective = 1;
pub type ZSTD_dParameter = ::core::ffi::c_uint;
pub const ZSTD_d_experimentalParam6: ZSTD_dParameter = 1005;
pub const ZSTD_d_experimentalParam5: ZSTD_dParameter = 1004;
pub const ZSTD_d_experimentalParam4: ZSTD_dParameter = 1003;
pub const ZSTD_d_experimentalParam3: ZSTD_dParameter = 1002;
pub const ZSTD_d_experimentalParam2: ZSTD_dParameter = 1001;
pub const ZSTD_d_experimentalParam1: ZSTD_dParameter = 1000;
pub const ZSTD_d_windowLogMax: ZSTD_dParameter = 100;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ZSTD_inBuffer_s {
    pub src: *const ::core::ffi::c_void,
    pub size: size_t,
    pub pos: size_t,
}
pub type ZSTD_inBuffer = ZSTD_inBuffer_s;
pub type ZSTD_DStream = ZSTD_DCtx;
pub const ZSTDnit_block: ZSTD_nextInputType_e = 2;
pub type ZSTD_nextInputType_e = ::core::ffi::c_uint;
pub const ZSTDnit_skippableFrame: ZSTD_nextInputType_e = 5;
pub const ZSTDnit_checksum: ZSTD_nextInputType_e = 4;
pub const ZSTDnit_lastBlock: ZSTD_nextInputType_e = 3;
pub const ZSTDnit_blockHeader: ZSTD_nextInputType_e = 1;
pub const ZSTDnit_frameHeader: ZSTD_nextInputType_e = 0;
pub type ZSTD_dictContentType_e = ::core::ffi::c_uint;
pub const ZSTD_dct_fullDict: ZSTD_dictContentType_e = 2;
pub const ZSTD_dct_rawContent: ZSTD_dictContentType_e = 1;
pub const ZSTD_dct_auto: ZSTD_dictContentType_e = 0;
pub type ZSTD_dictLoadMethod_e = ::core::ffi::c_uint;
pub const ZSTD_dlm_byRef: ZSTD_dictLoadMethod_e = 1;
pub const ZSTD_dlm_byCopy: ZSTD_dictLoadMethod_e = 0;
pub const ZSTD_MAXWINDOWSIZE_DEFAULT: U32 =
    ((1 as ::core::ffi::c_int as U32) << ZSTD_WINDOWLOG_LIMIT_DEFAULT).wrapping_add(1 as U32);
pub const ZSTD_NO_FORWARD_PROGRESS_MAX: ::core::ffi::c_int = 16 as ::core::ffi::c_int;
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
pub const ZSTD_VERSION_MAJOR: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const ZSTD_VERSION_MINOR: ::core::ffi::c_int = 5 as ::core::ffi::c_int;
pub const ZSTD_VERSION_RELEASE: ::core::ffi::c_int = 7 as ::core::ffi::c_int;
pub const ZSTD_VERSION_NUMBER: ::core::ffi::c_int =
    ZSTD_VERSION_MAJOR * 100 as ::core::ffi::c_int * 100 as ::core::ffi::c_int
        + ZSTD_VERSION_MINOR * 100 as ::core::ffi::c_int
        + ZSTD_VERSION_RELEASE;
pub const ZSTD_MAGICNUMBER: ::core::ffi::c_uint = 0xfd2fb528 as ::core::ffi::c_uint;
pub const ZSTD_MAGIC_DICTIONARY: ::core::ffi::c_uint = 0xec30a437 as ::core::ffi::c_uint;
pub const ZSTD_MAGIC_SKIPPABLE_START: ::core::ffi::c_int = 0x184d2a50 as ::core::ffi::c_int;
pub const ZSTD_MAGIC_SKIPPABLE_MASK: ::core::ffi::c_uint = 0xfffffff0 as ::core::ffi::c_uint;
pub const ZSTD_BLOCKSIZELOG_MAX: ::core::ffi::c_int = 17 as ::core::ffi::c_int;
pub const ZSTD_BLOCKSIZE_MAX: ::core::ffi::c_int =
    (1 as ::core::ffi::c_int) << ZSTD_BLOCKSIZELOG_MAX;
pub const ZSTD_CONTENTSIZE_UNKNOWN: ::core::ffi::c_ulonglong =
    (0 as ::core::ffi::c_ulonglong).wrapping_sub(1 as ::core::ffi::c_ulonglong);
pub const ZSTD_CONTENTSIZE_ERROR: ::core::ffi::c_ulonglong =
    (0 as ::core::ffi::c_ulonglong).wrapping_sub(2 as ::core::ffi::c_ulonglong);
pub const ZSTD_SKIPPABLEHEADERSIZE: ::core::ffi::c_int = 8 as ::core::ffi::c_int;
pub const ZSTD_WINDOWLOG_MAX_32: ::core::ffi::c_int = 30 as ::core::ffi::c_int;
pub const ZSTD_WINDOWLOG_MAX_64: ::core::ffi::c_int = 31 as ::core::ffi::c_int;
pub const ZSTD_BLOCKSIZE_MAX_MIN: ::core::ffi::c_int =
    (1 as ::core::ffi::c_int) << 10 as ::core::ffi::c_int;
pub const ZSTD_WINDOWLOG_LIMIT_DEFAULT: ::core::ffi::c_int = 27 as ::core::ffi::c_int;
static mut ZSTD_defaultCMem: ZSTD_customMem = ZSTD_customMem {
    customAlloc: None,
    customFree: None,
    opaque: NULL,
};
pub const ZSTD_d_stableOutBuffer: ::core::ffi::c_uint = 1001 as ::core::ffi::c_uint;
pub const ZSTD_d_forceIgnoreChecksum: ::core::ffi::c_uint = 1002 as ::core::ffi::c_uint;
pub const ZSTD_d_refMultipleDDicts: ::core::ffi::c_uint = 1003 as ::core::ffi::c_uint;
pub const ZSTD_d_disableHuffmanAssembly: ::core::ffi::c_uint = 1004 as ::core::ffi::c_uint;
pub const ZSTD_d_maxBlockSize: ::core::ffi::c_uint = 1005 as ::core::ffi::c_uint;
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
unsafe extern "C" fn MEM_isLittleEndian() -> ::core::ffi::c_uint {
    return 1 as ::core::ffi::c_uint;
}
#[inline]
unsafe extern "C" fn MEM_read16(mut ptr: *const ::core::ffi::c_void) -> U16 {
    return *(ptr as *const unalign16);
}
#[inline]
unsafe extern "C" fn MEM_read32(mut ptr: *const ::core::ffi::c_void) -> U32 {
    return *(ptr as *const unalign32);
}
#[inline]
unsafe extern "C" fn MEM_read64(mut ptr: *const ::core::ffi::c_void) -> U64 {
    return *(ptr as *const unalign64);
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
unsafe extern "C" fn MEM_swap64(mut in_0: U64) -> U64 {
    return in_0.swap_bytes();
}
#[inline]
unsafe extern "C" fn MEM_readLE16(mut memPtr: *const ::core::ffi::c_void) -> U16 {
    if MEM_isLittleEndian() != 0 {
        return MEM_read16(memPtr);
    } else {
        let mut p: *const BYTE = memPtr as *const BYTE;
        return (*p.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
            + ((*p.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int)
                << 8 as ::core::ffi::c_int)) as U16;
    };
}
#[inline]
unsafe extern "C" fn MEM_readLE32(mut memPtr: *const ::core::ffi::c_void) -> U32 {
    if MEM_isLittleEndian() != 0 {
        return MEM_read32(memPtr);
    } else {
        return MEM_swap32(MEM_read32(memPtr));
    };
}
#[inline]
unsafe extern "C" fn MEM_writeLE32(mut memPtr: *mut ::core::ffi::c_void, mut val32: U32) {
    if MEM_isLittleEndian() != 0 {
        MEM_write32(memPtr, val32);
    } else {
        MEM_write32(memPtr, MEM_swap32(val32));
    };
}
#[inline]
unsafe extern "C" fn MEM_readLE64(mut memPtr: *const ::core::ffi::c_void) -> U64 {
    if MEM_isLittleEndian() != 0 {
        return MEM_read64(memPtr);
    } else {
        return MEM_swap64(MEM_read64(memPtr));
    };
}
#[inline]
unsafe extern "C" fn ZSTD_countLeadingZeros32(mut val: U32) -> ::core::ffi::c_uint {
    return val.leading_zeros() as i32 as ::core::ffi::c_uint;
}
#[inline]
unsafe extern "C" fn ZSTD_highbit32(mut val: U32) -> ::core::ffi::c_uint {
    return (31 as ::core::ffi::c_uint).wrapping_sub(ZSTD_countLeadingZeros32(val));
}
static mut repStartValue: [U32; 3] = [
    1 as ::core::ffi::c_int as U32,
    4 as ::core::ffi::c_int as U32,
    8 as ::core::ffi::c_int as U32,
];
pub const ZSTD_WINDOWLOG_ABSOLUTEMIN: ::core::ffi::c_int = 10 as ::core::ffi::c_int;
static mut ZSTD_fcs_fieldSize: [size_t; 4] = [
    0 as ::core::ffi::c_int as size_t,
    2 as ::core::ffi::c_int as size_t,
    4 as ::core::ffi::c_int as size_t,
    8 as ::core::ffi::c_int as size_t,
];
static mut ZSTD_did_fieldSize: [size_t; 4] = [
    0 as ::core::ffi::c_int as size_t,
    1 as ::core::ffi::c_int as size_t,
    2 as ::core::ffi::c_int as size_t,
    4 as ::core::ffi::c_int as size_t,
];
pub const ZSTD_FRAMEIDSIZE: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const ZSTD_BLOCKHEADERSIZE: ::core::ffi::c_int = 3 as ::core::ffi::c_int;
static mut ZSTD_blockHeaderSize: size_t = ZSTD_BLOCKHEADERSIZE as size_t;
pub const MaxML: ::core::ffi::c_int = 52 as ::core::ffi::c_int;
pub const MaxLL: ::core::ffi::c_int = 35 as ::core::ffi::c_int;
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
pub const WILDCOPY_OVERLENGTH: ::core::ffi::c_int = 32 as ::core::ffi::c_int;
#[inline]
unsafe extern "C" fn ZSTD_limitCopy(
    mut dst: *mut ::core::ffi::c_void,
    mut dstCapacity: size_t,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    let length: size_t = if dstCapacity < srcSize {
        dstCapacity
    } else {
        srcSize
    };
    if length > 0 as size_t {
        ::libc::memcpy(dst, src, length as ::libc::size_t);
    }
    return length;
}
pub const ZSTD_WORKSPACETOOLARGE_FACTOR: ::core::ffi::c_int = 3 as ::core::ffi::c_int;
pub const ZSTD_WORKSPACETOOLARGE_MAXDURATION: ::core::ffi::c_int = 128 as ::core::ffi::c_int;
static mut LL_base: [U32; 36] = [
    0 as ::core::ffi::c_int as U32,
    1 as ::core::ffi::c_int as U32,
    2 as ::core::ffi::c_int as U32,
    3 as ::core::ffi::c_int as U32,
    4 as ::core::ffi::c_int as U32,
    5 as ::core::ffi::c_int as U32,
    6 as ::core::ffi::c_int as U32,
    7 as ::core::ffi::c_int as U32,
    8 as ::core::ffi::c_int as U32,
    9 as ::core::ffi::c_int as U32,
    10 as ::core::ffi::c_int as U32,
    11 as ::core::ffi::c_int as U32,
    12 as ::core::ffi::c_int as U32,
    13 as ::core::ffi::c_int as U32,
    14 as ::core::ffi::c_int as U32,
    15 as ::core::ffi::c_int as U32,
    16 as ::core::ffi::c_int as U32,
    18 as ::core::ffi::c_int as U32,
    20 as ::core::ffi::c_int as U32,
    22 as ::core::ffi::c_int as U32,
    24 as ::core::ffi::c_int as U32,
    28 as ::core::ffi::c_int as U32,
    32 as ::core::ffi::c_int as U32,
    40 as ::core::ffi::c_int as U32,
    48 as ::core::ffi::c_int as U32,
    64 as ::core::ffi::c_int as U32,
    0x80 as ::core::ffi::c_int as U32,
    0x100 as ::core::ffi::c_int as U32,
    0x200 as ::core::ffi::c_int as U32,
    0x400 as ::core::ffi::c_int as U32,
    0x800 as ::core::ffi::c_int as U32,
    0x1000 as ::core::ffi::c_int as U32,
    0x2000 as ::core::ffi::c_int as U32,
    0x4000 as ::core::ffi::c_int as U32,
    0x8000 as ::core::ffi::c_int as U32,
    0x10000 as ::core::ffi::c_int as U32,
];
static mut OF_base: [U32; 32] = [
    0 as ::core::ffi::c_int as U32,
    1 as ::core::ffi::c_int as U32,
    1 as ::core::ffi::c_int as U32,
    5 as ::core::ffi::c_int as U32,
    0xd as ::core::ffi::c_int as U32,
    0x1d as ::core::ffi::c_int as U32,
    0x3d as ::core::ffi::c_int as U32,
    0x7d as ::core::ffi::c_int as U32,
    0xfd as ::core::ffi::c_int as U32,
    0x1fd as ::core::ffi::c_int as U32,
    0x3fd as ::core::ffi::c_int as U32,
    0x7fd as ::core::ffi::c_int as U32,
    0xffd as ::core::ffi::c_int as U32,
    0x1ffd as ::core::ffi::c_int as U32,
    0x3ffd as ::core::ffi::c_int as U32,
    0x7ffd as ::core::ffi::c_int as U32,
    0xfffd as ::core::ffi::c_int as U32,
    0x1fffd as ::core::ffi::c_int as U32,
    0x3fffd as ::core::ffi::c_int as U32,
    0x7fffd as ::core::ffi::c_int as U32,
    0xffffd as ::core::ffi::c_int as U32,
    0x1ffffd as ::core::ffi::c_int as U32,
    0x3ffffd as ::core::ffi::c_int as U32,
    0x7ffffd as ::core::ffi::c_int as U32,
    0xfffffd as ::core::ffi::c_int as U32,
    0x1fffffd as ::core::ffi::c_int as U32,
    0x3fffffd as ::core::ffi::c_int as U32,
    0x7fffffd as ::core::ffi::c_int as U32,
    0xffffffd as ::core::ffi::c_int as U32,
    0x1ffffffd as ::core::ffi::c_int as U32,
    0x3ffffffd as ::core::ffi::c_int as U32,
    0x7ffffffd as ::core::ffi::c_int as U32,
];
static mut OF_bits: [U8; 32] = [
    0 as ::core::ffi::c_int as U8,
    1 as ::core::ffi::c_int as U8,
    2 as ::core::ffi::c_int as U8,
    3 as ::core::ffi::c_int as U8,
    4 as ::core::ffi::c_int as U8,
    5 as ::core::ffi::c_int as U8,
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
    17 as ::core::ffi::c_int as U8,
    18 as ::core::ffi::c_int as U8,
    19 as ::core::ffi::c_int as U8,
    20 as ::core::ffi::c_int as U8,
    21 as ::core::ffi::c_int as U8,
    22 as ::core::ffi::c_int as U8,
    23 as ::core::ffi::c_int as U8,
    24 as ::core::ffi::c_int as U8,
    25 as ::core::ffi::c_int as U8,
    26 as ::core::ffi::c_int as U8,
    27 as ::core::ffi::c_int as U8,
    28 as ::core::ffi::c_int as U8,
    29 as ::core::ffi::c_int as U8,
    30 as ::core::ffi::c_int as U8,
    31 as ::core::ffi::c_int as U8,
];
static mut ML_base: [U32; 53] = [
    3 as ::core::ffi::c_int as U32,
    4 as ::core::ffi::c_int as U32,
    5 as ::core::ffi::c_int as U32,
    6 as ::core::ffi::c_int as U32,
    7 as ::core::ffi::c_int as U32,
    8 as ::core::ffi::c_int as U32,
    9 as ::core::ffi::c_int as U32,
    10 as ::core::ffi::c_int as U32,
    11 as ::core::ffi::c_int as U32,
    12 as ::core::ffi::c_int as U32,
    13 as ::core::ffi::c_int as U32,
    14 as ::core::ffi::c_int as U32,
    15 as ::core::ffi::c_int as U32,
    16 as ::core::ffi::c_int as U32,
    17 as ::core::ffi::c_int as U32,
    18 as ::core::ffi::c_int as U32,
    19 as ::core::ffi::c_int as U32,
    20 as ::core::ffi::c_int as U32,
    21 as ::core::ffi::c_int as U32,
    22 as ::core::ffi::c_int as U32,
    23 as ::core::ffi::c_int as U32,
    24 as ::core::ffi::c_int as U32,
    25 as ::core::ffi::c_int as U32,
    26 as ::core::ffi::c_int as U32,
    27 as ::core::ffi::c_int as U32,
    28 as ::core::ffi::c_int as U32,
    29 as ::core::ffi::c_int as U32,
    30 as ::core::ffi::c_int as U32,
    31 as ::core::ffi::c_int as U32,
    32 as ::core::ffi::c_int as U32,
    33 as ::core::ffi::c_int as U32,
    34 as ::core::ffi::c_int as U32,
    35 as ::core::ffi::c_int as U32,
    37 as ::core::ffi::c_int as U32,
    39 as ::core::ffi::c_int as U32,
    41 as ::core::ffi::c_int as U32,
    43 as ::core::ffi::c_int as U32,
    47 as ::core::ffi::c_int as U32,
    51 as ::core::ffi::c_int as U32,
    59 as ::core::ffi::c_int as U32,
    67 as ::core::ffi::c_int as U32,
    83 as ::core::ffi::c_int as U32,
    99 as ::core::ffi::c_int as U32,
    0x83 as ::core::ffi::c_int as U32,
    0x103 as ::core::ffi::c_int as U32,
    0x203 as ::core::ffi::c_int as U32,
    0x403 as ::core::ffi::c_int as U32,
    0x803 as ::core::ffi::c_int as U32,
    0x1003 as ::core::ffi::c_int as U32,
    0x2003 as ::core::ffi::c_int as U32,
    0x4003 as ::core::ffi::c_int as U32,
    0x8003 as ::core::ffi::c_int as U32,
    0x10003 as ::core::ffi::c_int as U32,
];
pub const ZSTD_HUFFDTABLE_CAPACITY_LOG: ::core::ffi::c_int = 12 as ::core::ffi::c_int;
pub const ZSTDv05_MAGICNUMBER: ::core::ffi::c_uint = 4247762213;
pub const ZSTDv06_MAGICNUMBER: ::core::ffi::c_uint = 4247762214;
pub const ZSTDv07_MAGICNUMBER: ::core::ffi::c_uint = 4247762215;
#[inline]
unsafe extern "C" fn ZSTD_isLegacy(
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> ::core::ffi::c_uint {
    let mut magicNumberLE: U32 = 0;
    if srcSize < 4 as size_t {
        return 0 as ::core::ffi::c_uint;
    }
    magicNumberLE = MEM_readLE32(src);
    match magicNumberLE {
        ZSTDv05_MAGICNUMBER => return 5 as ::core::ffi::c_uint,
        ZSTDv06_MAGICNUMBER => return 6 as ::core::ffi::c_uint,
        ZSTDv07_MAGICNUMBER => return 7 as ::core::ffi::c_uint,
        _ => return 0 as ::core::ffi::c_uint,
    };
}
#[inline]
unsafe extern "C" fn ZSTD_getDecompressedSize_legacy(
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> ::core::ffi::c_ulonglong {
    let version: U32 = ZSTD_isLegacy(src, srcSize) as U32;
    if version < 5 as U32 {
        return 0 as ::core::ffi::c_ulonglong;
    }
    if version == 5 as U32 {
        let mut fParams: ZSTDv05_parameters = ZSTDv05_parameters {
            srcSize: 0,
            windowLog: 0,
            contentLog: 0,
            hashLog: 0,
            searchLog: 0,
            searchLength: 0,
            targetLength: 0,
            strategy: ZSTDv05_fast,
        };
        let frResult: size_t = ZSTDv05_getFrameParams(&raw mut fParams, src, srcSize) as size_t;
        if frResult != 0 as size_t {
            return 0 as ::core::ffi::c_ulonglong;
        }
        return fParams.srcSize as ::core::ffi::c_ulonglong;
    }
    if version == 6 as U32 {
        let mut fParams_0: ZSTDv06_frameParams = ZSTDv06_frameParams {
            frameContentSize: 0,
            windowLog: 0,
        };
        let frResult_0: size_t = ZSTDv06_getFrameParams(&raw mut fParams_0, src, srcSize) as size_t;
        if frResult_0 != 0 as size_t {
            return 0 as ::core::ffi::c_ulonglong;
        }
        return fParams_0.frameContentSize;
    }
    if version == 7 as U32 {
        let mut fParams_1: ZSTDv07_frameParams = ZSTDv07_frameParams {
            frameContentSize: 0,
            windowSize: 0,
            dictID: 0,
            checksumFlag: 0,
        };
        let frResult_1: size_t = ZSTDv07_getFrameParams(&raw mut fParams_1, src, srcSize) as size_t;
        if frResult_1 != 0 as size_t {
            return 0 as ::core::ffi::c_ulonglong;
        }
        return fParams_1.frameContentSize;
    }
    return 0 as ::core::ffi::c_ulonglong;
}
#[inline]
unsafe extern "C" fn ZSTD_decompressLegacy(
    mut dst: *mut ::core::ffi::c_void,
    mut dstCapacity: size_t,
    mut src: *const ::core::ffi::c_void,
    mut compressedSize: size_t,
    mut dict: *const ::core::ffi::c_void,
    mut dictSize: size_t,
) -> size_t {
    let version: U32 = ZSTD_isLegacy(src, compressedSize) as U32;
    let mut x: ::core::ffi::c_char = 0;
    if dst.is_null() {
        dst = &raw mut x as *mut ::core::ffi::c_void;
    }
    if src.is_null() {
        src = &raw mut x as *const ::core::ffi::c_void;
    }
    if dict.is_null() {
        dict = &raw mut x as *const ::core::ffi::c_void;
    }
    match version {
        5 => {
            let mut result: size_t = 0;
            let zd: *mut ZSTDv05_DCtx = ZSTDv05_createDCtx() as *mut ZSTDv05_DCtx;
            if zd.is_null() {
                return -(ZSTD_error_memory_allocation as ::core::ffi::c_int) as size_t;
            }
            result = ZSTDv05_decompress_usingDict(
                zd,
                dst,
                dstCapacity,
                src,
                compressedSize,
                dict,
                dictSize,
            );
            ZSTDv05_freeDCtx(zd);
            return result;
        }
        6 => {
            let mut result_0: size_t = 0;
            let zd_0: *mut ZSTDv06_DCtx = ZSTDv06_createDCtx() as *mut ZSTDv06_DCtx;
            if zd_0.is_null() {
                return -(ZSTD_error_memory_allocation as ::core::ffi::c_int) as size_t;
            }
            result_0 = ZSTDv06_decompress_usingDict(
                zd_0,
                dst,
                dstCapacity,
                src,
                compressedSize,
                dict,
                dictSize,
            );
            ZSTDv06_freeDCtx(zd_0);
            return result_0;
        }
        7 => {
            let mut result_1: size_t = 0;
            let zd_1: *mut ZSTDv07_DCtx = ZSTDv07_createDCtx() as *mut ZSTDv07_DCtx;
            if zd_1.is_null() {
                return -(ZSTD_error_memory_allocation as ::core::ffi::c_int) as size_t;
            }
            result_1 = ZSTDv07_decompress_usingDict(
                zd_1,
                dst,
                dstCapacity,
                src,
                compressedSize,
                dict,
                dictSize,
            );
            ZSTDv07_freeDCtx(zd_1);
            return result_1;
        }
        _ => return -(ZSTD_error_prefix_unknown as ::core::ffi::c_int) as size_t,
    };
}
#[inline]
unsafe extern "C" fn ZSTD_findFrameSizeInfoLegacy(
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> ZSTD_frameSizeInfo {
    let mut frameSizeInfo: ZSTD_frameSizeInfo = ZSTD_frameSizeInfo {
        nbBlocks: 0,
        compressedSize: 0,
        decompressedBound: 0,
    };
    let version: U32 = ZSTD_isLegacy(src, srcSize) as U32;
    match version {
        5 => {
            ZSTDv05_findFrameSizeInfoLegacy(
                src,
                srcSize,
                &raw mut frameSizeInfo.compressedSize,
                &raw mut frameSizeInfo.decompressedBound,
            );
        }
        6 => {
            ZSTDv06_findFrameSizeInfoLegacy(
                src,
                srcSize,
                &raw mut frameSizeInfo.compressedSize,
                &raw mut frameSizeInfo.decompressedBound,
            );
        }
        7 => {
            ZSTDv07_findFrameSizeInfoLegacy(
                src,
                srcSize,
                &raw mut frameSizeInfo.compressedSize,
                &raw mut frameSizeInfo.decompressedBound,
            );
        }
        _ => {
            frameSizeInfo.compressedSize =
                -(ZSTD_error_prefix_unknown as ::core::ffi::c_int) as size_t;
            frameSizeInfo.decompressedBound = ZSTD_CONTENTSIZE_ERROR;
        }
    }
    if ERR_isError(frameSizeInfo.compressedSize) == 0 && frameSizeInfo.compressedSize > srcSize {
        frameSizeInfo.compressedSize = -(ZSTD_error_srcSize_wrong as ::core::ffi::c_int) as size_t;
        frameSizeInfo.decompressedBound = ZSTD_CONTENTSIZE_ERROR;
    }
    if frameSizeInfo.decompressedBound != ZSTD_CONTENTSIZE_ERROR {
        frameSizeInfo.nbBlocks = frameSizeInfo
            .decompressedBound
            .wrapping_div(ZSTD_BLOCKSIZE_MAX as ::core::ffi::c_ulonglong)
            as size_t;
    }
    return frameSizeInfo;
}
#[inline]
unsafe extern "C" fn ZSTD_findFrameCompressedSizeLegacy(
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    let mut frameSizeInfo: ZSTD_frameSizeInfo = ZSTD_findFrameSizeInfoLegacy(src, srcSize);
    return frameSizeInfo.compressedSize;
}
#[inline]
unsafe extern "C" fn ZSTD_freeLegacyStreamContext(
    mut legacyContext: *mut ::core::ffi::c_void,
    mut version: U32,
) -> size_t {
    match version {
        5 => return ZBUFFv05_freeDCtx(legacyContext as *mut ZBUFFv05_DCtx),
        6 => return ZBUFFv06_freeDCtx(legacyContext as *mut ZBUFFv06_DCtx),
        7 => return ZBUFFv07_freeDCtx(legacyContext as *mut ZBUFFv07_DCtx),
        1 | 2 | 3 | _ => {
            return -(ZSTD_error_version_unsupported as ::core::ffi::c_int) as size_t;
        }
    };
}
#[inline]
unsafe extern "C" fn ZSTD_initLegacyStream(
    mut legacyContext: *mut *mut ::core::ffi::c_void,
    mut prevVersion: U32,
    mut newVersion: U32,
    mut dict: *const ::core::ffi::c_void,
    mut dictSize: size_t,
) -> size_t {
    let mut x: ::core::ffi::c_char = 0;
    if dict.is_null() {
        dict = &raw mut x as *const ::core::ffi::c_void;
    }
    if prevVersion != newVersion {
        ZSTD_freeLegacyStreamContext(*legacyContext, prevVersion);
    }
    match newVersion {
        5 => {
            let mut dctx: *mut ZBUFFv05_DCtx = if prevVersion != newVersion {
                ZBUFFv05_createDCtx()
            } else {
                *legacyContext as *mut ZBUFFv05_DCtx
            };
            if dctx.is_null() {
                return -(ZSTD_error_memory_allocation as ::core::ffi::c_int) as size_t;
            }
            ZBUFFv05_decompressInitDictionary(dctx, dict, dictSize);
            *legacyContext = dctx as *mut ::core::ffi::c_void;
            return 0 as size_t;
        }
        6 => {
            let mut dctx_0: *mut ZBUFFv06_DCtx = if prevVersion != newVersion {
                ZBUFFv06_createDCtx()
            } else {
                *legacyContext as *mut ZBUFFv06_DCtx
            };
            if dctx_0.is_null() {
                return -(ZSTD_error_memory_allocation as ::core::ffi::c_int) as size_t;
            }
            ZBUFFv06_decompressInitDictionary(dctx_0, dict, dictSize);
            *legacyContext = dctx_0 as *mut ::core::ffi::c_void;
            return 0 as size_t;
        }
        7 => {
            let mut dctx_1: *mut ZBUFFv07_DCtx = if prevVersion != newVersion {
                ZBUFFv07_createDCtx()
            } else {
                *legacyContext as *mut ZBUFFv07_DCtx
            };
            if dctx_1.is_null() {
                return -(ZSTD_error_memory_allocation as ::core::ffi::c_int) as size_t;
            }
            ZBUFFv07_decompressInitDictionary(dctx_1, dict, dictSize);
            *legacyContext = dctx_1 as *mut ::core::ffi::c_void;
            return 0 as size_t;
        }
        1 | 2 | 3 | _ => return 0 as size_t,
    };
}
#[inline]
unsafe extern "C" fn ZSTD_decompressLegacyStream(
    mut legacyContext: *mut ::core::ffi::c_void,
    mut version: U32,
    mut output: *mut ZSTD_outBuffer,
    mut input: *mut ZSTD_inBuffer,
) -> size_t {
    static mut x: ::core::ffi::c_char = 0;
    if (*output).dst.is_null() {
        (*output).dst = &raw mut x as *mut ::core::ffi::c_void;
    }
    if (*input).src.is_null() {
        (*input).src = &raw mut x as *const ::core::ffi::c_void;
    }
    match version {
        5 => {
            let mut dctx: *mut ZBUFFv05_DCtx = legacyContext as *mut ZBUFFv05_DCtx;
            let mut src: *const ::core::ffi::c_void = ((*input).src as *const ::core::ffi::c_char)
                .offset((*input).pos as isize)
                as *const ::core::ffi::c_void;
            let mut readSize: size_t = (*input).size.wrapping_sub((*input).pos);
            let mut dst: *mut ::core::ffi::c_void = ((*output).dst as *mut ::core::ffi::c_char)
                .offset((*output).pos as isize)
                as *mut ::core::ffi::c_void;
            let mut decodedSize: size_t = (*output).size.wrapping_sub((*output).pos);
            let hintSize: size_t = ZBUFFv05_decompressContinue(
                dctx,
                dst,
                &raw mut decodedSize,
                src,
                &raw mut readSize,
            ) as size_t;
            (*output).pos = ((*output).pos as ::core::ffi::c_ulong)
                .wrapping_add(decodedSize as ::core::ffi::c_ulong)
                as size_t as size_t;
            (*input).pos = ((*input).pos as ::core::ffi::c_ulong)
                .wrapping_add(readSize as ::core::ffi::c_ulong) as size_t
                as size_t;
            return hintSize;
        }
        6 => {
            let mut dctx_0: *mut ZBUFFv06_DCtx = legacyContext as *mut ZBUFFv06_DCtx;
            let mut src_0: *const ::core::ffi::c_void = ((*input).src as *const ::core::ffi::c_char)
                .offset((*input).pos as isize)
                as *const ::core::ffi::c_void;
            let mut readSize_0: size_t = (*input).size.wrapping_sub((*input).pos);
            let mut dst_0: *mut ::core::ffi::c_void = ((*output).dst as *mut ::core::ffi::c_char)
                .offset((*output).pos as isize)
                as *mut ::core::ffi::c_void;
            let mut decodedSize_0: size_t = (*output).size.wrapping_sub((*output).pos);
            let hintSize_0: size_t = ZBUFFv06_decompressContinue(
                dctx_0,
                dst_0,
                &raw mut decodedSize_0,
                src_0,
                &raw mut readSize_0,
            ) as size_t;
            (*output).pos = ((*output).pos as ::core::ffi::c_ulong)
                .wrapping_add(decodedSize_0 as ::core::ffi::c_ulong)
                as size_t as size_t;
            (*input).pos = ((*input).pos as ::core::ffi::c_ulong)
                .wrapping_add(readSize_0 as ::core::ffi::c_ulong)
                as size_t as size_t;
            return hintSize_0;
        }
        7 => {
            let mut dctx_1: *mut ZBUFFv07_DCtx = legacyContext as *mut ZBUFFv07_DCtx;
            let mut src_1: *const ::core::ffi::c_void = ((*input).src as *const ::core::ffi::c_char)
                .offset((*input).pos as isize)
                as *const ::core::ffi::c_void;
            let mut readSize_1: size_t = (*input).size.wrapping_sub((*input).pos);
            let mut dst_1: *mut ::core::ffi::c_void = ((*output).dst as *mut ::core::ffi::c_char)
                .offset((*output).pos as isize)
                as *mut ::core::ffi::c_void;
            let mut decodedSize_1: size_t = (*output).size.wrapping_sub((*output).pos);
            let hintSize_1: size_t = ZBUFFv07_decompressContinue(
                dctx_1,
                dst_1,
                &raw mut decodedSize_1,
                src_1,
                &raw mut readSize_1,
            ) as size_t;
            (*output).pos = ((*output).pos as ::core::ffi::c_ulong)
                .wrapping_add(decodedSize_1 as ::core::ffi::c_ulong)
                as size_t as size_t;
            (*input).pos = ((*input).pos as ::core::ffi::c_ulong)
                .wrapping_add(readSize_1 as ::core::ffi::c_ulong)
                as size_t as size_t;
            return hintSize_1;
        }
        1 | 2 | 3 | _ => {
            return -(ZSTD_error_version_unsupported as ::core::ffi::c_int) as size_t;
        }
    };
}
pub const DDICT_HASHSET_MAX_LOAD_FACTOR_COUNT_MULT: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const DDICT_HASHSET_MAX_LOAD_FACTOR_SIZE_MULT: ::core::ffi::c_int = 3 as ::core::ffi::c_int;
pub const DDICT_HASHSET_TABLE_BASE_SIZE: ::core::ffi::c_int = 64 as ::core::ffi::c_int;
pub const DDICT_HASHSET_RESIZE_FACTOR: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
unsafe extern "C" fn ZSTD_DDictHashSet_getIndex(
    mut hashSet: *const ZSTD_DDictHashSet,
    mut dictID: U32,
) -> size_t {
    let hash: U64 = ZSTD_XXH64(
        &raw mut dictID as *const ::core::ffi::c_void,
        ::core::mem::size_of::<U32>() as size_t,
        0 as XXH64_hash_t,
    ) as U64;
    return hash as size_t & (*hashSet).ddictPtrTableSize.wrapping_sub(1 as size_t);
}
unsafe extern "C" fn ZSTD_DDictHashSet_emplaceDDict(
    mut hashSet: *mut ZSTD_DDictHashSet,
    mut ddict: *const ZSTD_DDict,
) -> size_t {
    let dictID: U32 = ZSTD_getDictID_fromDDict(ddict) as U32;
    let mut idx: size_t = ZSTD_DDictHashSet_getIndex(hashSet, dictID);
    let idxRangeMask: size_t = (*hashSet).ddictPtrTableSize.wrapping_sub(1 as size_t);
    if (*hashSet).ddictPtrCount == (*hashSet).ddictPtrTableSize {
        return -(ZSTD_error_GENERIC as ::core::ffi::c_int) as size_t;
    }
    while !(*(*hashSet).ddictPtrTable.offset(idx as isize)).is_null() {
        if ZSTD_getDictID_fromDDict(*(*hashSet).ddictPtrTable.offset(idx as isize)) as U32 == dictID
        {
            let ref mut fresh1 = *(*hashSet).ddictPtrTable.offset(idx as isize);
            *fresh1 = ddict;
            return 0 as size_t;
        }
        idx = (idx as ::core::ffi::c_ulong & idxRangeMask as ::core::ffi::c_ulong) as size_t;
        idx = idx.wrapping_add(1);
    }
    let ref mut fresh2 = *(*hashSet).ddictPtrTable.offset(idx as isize);
    *fresh2 = ddict;
    (*hashSet).ddictPtrCount = (*hashSet).ddictPtrCount.wrapping_add(1);
    return 0 as size_t;
}
unsafe extern "C" fn ZSTD_DDictHashSet_expand(
    mut hashSet: *mut ZSTD_DDictHashSet,
    mut customMem: ZSTD_customMem,
) -> size_t {
    let mut newTableSize: size_t = (*hashSet)
        .ddictPtrTableSize
        .wrapping_mul(DDICT_HASHSET_RESIZE_FACTOR as size_t);
    let mut newTable: *mut *const ZSTD_DDict = ZSTD_customCalloc(
        (::core::mem::size_of::<*mut ZSTD_DDict>() as size_t).wrapping_mul(newTableSize),
        customMem,
    ) as *mut *const ZSTD_DDict;
    let mut oldTable: *mut *const ZSTD_DDict = (*hashSet).ddictPtrTable;
    let mut oldTableSize: size_t = (*hashSet).ddictPtrTableSize;
    let mut i: size_t = 0;
    if newTable.is_null() {
        return -(ZSTD_error_memory_allocation as ::core::ffi::c_int) as size_t;
    }
    (*hashSet).ddictPtrTable = newTable;
    (*hashSet).ddictPtrTableSize = newTableSize;
    (*hashSet).ddictPtrCount = 0 as size_t;
    i = 0 as size_t;
    while i < oldTableSize {
        if !(*oldTable.offset(i as isize)).is_null() {
            let err_code: size_t =
                ZSTD_DDictHashSet_emplaceDDict(hashSet, *oldTable.offset(i as isize)) as size_t;
            if ERR_isError(err_code) != 0 {
                return err_code;
            }
        }
        i = i.wrapping_add(1);
    }
    ZSTD_customFree(oldTable as *mut ::core::ffi::c_void, customMem);
    return 0 as size_t;
}
unsafe extern "C" fn ZSTD_DDictHashSet_getDDict(
    mut hashSet: *mut ZSTD_DDictHashSet,
    mut dictID: U32,
) -> *const ZSTD_DDict {
    let mut idx: size_t = ZSTD_DDictHashSet_getIndex(hashSet, dictID);
    let idxRangeMask: size_t = (*hashSet).ddictPtrTableSize.wrapping_sub(1 as size_t);
    loop {
        let mut currDictID: size_t =
            ZSTD_getDictID_fromDDict(*(*hashSet).ddictPtrTable.offset(idx as isize)) as size_t;
        if currDictID == dictID as size_t || currDictID == 0 as size_t {
            break;
        }
        idx = (idx as ::core::ffi::c_ulong & idxRangeMask as ::core::ffi::c_ulong) as size_t;
        idx = idx.wrapping_add(1);
    }
    return *(*hashSet).ddictPtrTable.offset(idx as isize);
}
unsafe extern "C" fn ZSTD_createDDictHashSet(
    mut customMem: ZSTD_customMem,
) -> *mut ZSTD_DDictHashSet {
    let mut ret: *mut ZSTD_DDictHashSet = ZSTD_customMalloc(
        ::core::mem::size_of::<ZSTD_DDictHashSet>() as size_t,
        customMem,
    ) as *mut ZSTD_DDictHashSet;
    if ret.is_null() {
        return ::core::ptr::null_mut::<ZSTD_DDictHashSet>();
    }
    (*ret).ddictPtrTable = ZSTD_customCalloc(
        (DDICT_HASHSET_TABLE_BASE_SIZE as size_t)
            .wrapping_mul(::core::mem::size_of::<*mut ZSTD_DDict>() as size_t),
        customMem,
    ) as *mut *const ZSTD_DDict;
    if (*ret).ddictPtrTable.is_null() {
        ZSTD_customFree(ret as *mut ::core::ffi::c_void, customMem);
        return ::core::ptr::null_mut::<ZSTD_DDictHashSet>();
    }
    (*ret).ddictPtrTableSize = DDICT_HASHSET_TABLE_BASE_SIZE as size_t;
    (*ret).ddictPtrCount = 0 as size_t;
    return ret;
}
unsafe extern "C" fn ZSTD_freeDDictHashSet(
    mut hashSet: *mut ZSTD_DDictHashSet,
    mut customMem: ZSTD_customMem,
) {
    if !hashSet.is_null() && !(*hashSet).ddictPtrTable.is_null() {
        ZSTD_customFree(
            (*hashSet).ddictPtrTable as *mut ::core::ffi::c_void,
            customMem,
        );
    }
    if !hashSet.is_null() {
        ZSTD_customFree(hashSet as *mut ::core::ffi::c_void, customMem);
    }
}
unsafe extern "C" fn ZSTD_DDictHashSet_addDDict(
    mut hashSet: *mut ZSTD_DDictHashSet,
    mut ddict: *const ZSTD_DDict,
    mut customMem: ZSTD_customMem,
) -> size_t {
    if (*hashSet)
        .ddictPtrCount
        .wrapping_mul(DDICT_HASHSET_MAX_LOAD_FACTOR_COUNT_MULT as size_t)
        .wrapping_div((*hashSet).ddictPtrTableSize)
        .wrapping_mul(DDICT_HASHSET_MAX_LOAD_FACTOR_SIZE_MULT as size_t)
        != 0 as size_t
    {
        let err_code: size_t = ZSTD_DDictHashSet_expand(hashSet, customMem) as size_t;
        if ERR_isError(err_code) != 0 {
            return err_code;
        }
    }
    let err_code_0: size_t = ZSTD_DDictHashSet_emplaceDDict(hashSet, ddict) as size_t;
    if ERR_isError(err_code_0) != 0 {
        return err_code_0;
    }
    return 0 as size_t;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_sizeof_DCtx(mut dctx: *const ZSTD_DCtx) -> size_t {
    if dctx.is_null() {
        return 0 as size_t;
    }
    return (::core::mem::size_of::<ZSTD_DCtx>() as size_t)
        .wrapping_add(ZSTD_sizeof_DDict((*dctx).ddictLocal))
        .wrapping_add((*dctx).inBuffSize)
        .wrapping_add((*dctx).outBuffSize);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_estimateDCtxSize() -> size_t {
    return ::core::mem::size_of::<ZSTD_DCtx>() as size_t;
}
unsafe extern "C" fn ZSTD_startingInputLength(mut format: ZSTD_format_e) -> size_t {
    let startingInputLength: size_t = (if format as ::core::ffi::c_uint
        == ZSTD_f_zstd1 as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        5 as ::core::ffi::c_int
    } else {
        1 as ::core::ffi::c_int
    }) as size_t;
    return startingInputLength;
}
unsafe extern "C" fn ZSTD_DCtx_resetParameters(mut dctx: *mut ZSTD_DCtx) {
    (*dctx).format = ZSTD_f_zstd1;
    (*dctx).maxWindowSize = ZSTD_MAXWINDOWSIZE_DEFAULT as size_t;
    (*dctx).outBufferMode = ZSTD_bm_buffered;
    (*dctx).forceIgnoreChecksum = ZSTD_d_validateChecksum;
    (*dctx).refMultipleDDicts = ZSTD_rmd_refSingleDDict;
    (*dctx).disableHufAsm = 0 as ::core::ffi::c_int;
    (*dctx).maxBlockSizeParam = 0 as ::core::ffi::c_int;
}
unsafe extern "C" fn ZSTD_initDCtx_internal(mut dctx: *mut ZSTD_DCtx) {
    (*dctx).staticSize = 0 as size_t;
    (*dctx).ddict = ::core::ptr::null::<ZSTD_DDict>();
    (*dctx).ddictLocal = ::core::ptr::null_mut::<ZSTD_DDict>();
    (*dctx).dictEnd = ::core::ptr::null::<::core::ffi::c_void>();
    (*dctx).ddictIsCold = 0 as ::core::ffi::c_int;
    (*dctx).dictUses = ZSTD_dont_use;
    (*dctx).inBuff = ::core::ptr::null_mut::<::core::ffi::c_char>();
    (*dctx).inBuffSize = 0 as size_t;
    (*dctx).outBuffSize = 0 as size_t;
    (*dctx).streamStage = zdss_init;
    (*dctx).legacyContext = NULL;
    (*dctx).previousLegacyVersion = 0 as U32;
    (*dctx).noForwardProgress = 0 as ::core::ffi::c_int;
    (*dctx).oversizedDuration = 0 as size_t;
    (*dctx).isFrameDecompression = 1 as ::core::ffi::c_int;
    (*dctx).ddictSet = ::core::ptr::null_mut::<ZSTD_DDictHashSet>();
    ZSTD_DCtx_resetParameters(dctx);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_initStaticDCtx(
    mut workspace: *mut ::core::ffi::c_void,
    mut workspaceSize: size_t,
) -> *mut ZSTD_DCtx {
    let dctx: *mut ZSTD_DCtx = workspace as *mut ZSTD_DCtx;
    if workspace as size_t & 7 as size_t != 0 {
        return ::core::ptr::null_mut::<ZSTD_DCtx>();
    }
    if workspaceSize < ::core::mem::size_of::<ZSTD_DCtx>() as usize {
        return ::core::ptr::null_mut::<ZSTD_DCtx>();
    }
    ZSTD_initDCtx_internal(dctx);
    (*dctx).staticSize = workspaceSize;
    (*dctx).inBuff = dctx.offset(1 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_char;
    return dctx;
}
unsafe extern "C" fn ZSTD_createDCtx_internal(mut customMem: ZSTD_customMem) -> *mut ZSTD_DCtx {
    if customMem.customAlloc.is_none() as ::core::ffi::c_int
        ^ customMem.customFree.is_none() as ::core::ffi::c_int
        != 0
    {
        return ::core::ptr::null_mut::<ZSTD_DCtx>();
    }
    let dctx: *mut ZSTD_DCtx =
        ZSTD_customMalloc(::core::mem::size_of::<ZSTD_DCtx>() as size_t, customMem)
            as *mut ZSTD_DCtx;
    if dctx.is_null() {
        return ::core::ptr::null_mut::<ZSTD_DCtx>();
    }
    (*dctx).customMem = customMem;
    ZSTD_initDCtx_internal(dctx);
    return dctx;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_createDCtx_advanced(mut customMem: ZSTD_customMem) -> *mut ZSTD_DCtx {
    return ZSTD_createDCtx_internal(customMem);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_createDCtx() -> *mut ZSTD_DCtx {
    return ZSTD_createDCtx_internal(ZSTD_defaultCMem);
}
unsafe extern "C" fn ZSTD_clearDict(mut dctx: *mut ZSTD_DCtx) {
    ZSTD_freeDDict((*dctx).ddictLocal);
    (*dctx).ddictLocal = ::core::ptr::null_mut::<ZSTD_DDict>();
    (*dctx).ddict = ::core::ptr::null::<ZSTD_DDict>();
    (*dctx).dictUses = ZSTD_dont_use;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_freeDCtx(mut dctx: *mut ZSTD_DCtx) -> size_t {
    if dctx.is_null() {
        return 0 as size_t;
    }
    if (*dctx).staticSize != 0 {
        return -(ZSTD_error_memory_allocation as ::core::ffi::c_int) as size_t;
    }
    let cMem: ZSTD_customMem = (*dctx).customMem;
    ZSTD_clearDict(dctx);
    ZSTD_customFree((*dctx).inBuff as *mut ::core::ffi::c_void, cMem);
    (*dctx).inBuff = ::core::ptr::null_mut::<::core::ffi::c_char>();
    if !(*dctx).legacyContext.is_null() {
        ZSTD_freeLegacyStreamContext((*dctx).legacyContext, (*dctx).previousLegacyVersion);
    }
    if !(*dctx).ddictSet.is_null() {
        ZSTD_freeDDictHashSet((*dctx).ddictSet, cMem);
        (*dctx).ddictSet = ::core::ptr::null_mut::<ZSTD_DDictHashSet>();
    }
    ZSTD_customFree(dctx as *mut ::core::ffi::c_void, cMem);
    return 0 as size_t;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_copyDCtx(mut dstDCtx: *mut ZSTD_DCtx, mut srcDCtx: *const ZSTD_DCtx) {
    let toCopy: size_t = (&raw mut (*dstDCtx).inBuff as *mut ::core::ffi::c_char)
        .offset_from(dstDCtx as *mut ::core::ffi::c_char)
        as ::core::ffi::c_long as size_t;
    ::libc::memcpy(
        dstDCtx as *mut ::core::ffi::c_void,
        srcDCtx as *const ::core::ffi::c_void,
        toCopy as ::libc::size_t,
    );
}
unsafe extern "C" fn ZSTD_DCtx_selectFrameDDict(mut dctx: *mut ZSTD_DCtx) {
    if !(*dctx).ddict.is_null() {
        let mut frameDDict: *const ZSTD_DDict =
            ZSTD_DDictHashSet_getDDict((*dctx).ddictSet, (*dctx).fParams.dictID as U32);
        if !frameDDict.is_null() {
            ZSTD_clearDict(dctx);
            (*dctx).dictID = (*dctx).fParams.dictID as U32;
            (*dctx).ddict = frameDDict;
            (*dctx).dictUses = ZSTD_use_indefinitely;
        }
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_isFrame(
    mut buffer: *const ::core::ffi::c_void,
    mut size: size_t,
) -> ::core::ffi::c_uint {
    if size < ZSTD_FRAMEIDSIZE as size_t {
        return 0 as ::core::ffi::c_uint;
    }
    let magic: U32 = MEM_readLE32(buffer) as U32;
    if magic == ZSTD_MAGICNUMBER as U32 {
        return 1 as ::core::ffi::c_uint;
    }
    if magic & ZSTD_MAGIC_SKIPPABLE_MASK as U32 == ZSTD_MAGIC_SKIPPABLE_START as U32 {
        return 1 as ::core::ffi::c_uint;
    }
    if ZSTD_isLegacy(buffer, size) != 0 {
        return 1 as ::core::ffi::c_uint;
    }
    return 0 as ::core::ffi::c_uint;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_isSkippableFrame(
    mut buffer: *const ::core::ffi::c_void,
    mut size: size_t,
) -> ::core::ffi::c_uint {
    if size < ZSTD_FRAMEIDSIZE as size_t {
        return 0 as ::core::ffi::c_uint;
    }
    let magic: U32 = MEM_readLE32(buffer) as U32;
    if magic & ZSTD_MAGIC_SKIPPABLE_MASK as U32 == ZSTD_MAGIC_SKIPPABLE_START as U32 {
        return 1 as ::core::ffi::c_uint;
    }
    return 0 as ::core::ffi::c_uint;
}
unsafe extern "C" fn ZSTD_frameHeaderSize_internal(
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
    mut format: ZSTD_format_e,
) -> size_t {
    let minInputSize: size_t = ZSTD_startingInputLength(format) as size_t;
    if srcSize < minInputSize {
        return -(ZSTD_error_srcSize_wrong as ::core::ffi::c_int) as size_t;
    }
    let fhd: BYTE = *(src as *const BYTE).offset(minInputSize.wrapping_sub(1 as size_t) as isize);
    let dictID: U32 = (fhd as ::core::ffi::c_int & 3 as ::core::ffi::c_int) as U32;
    let singleSegment: U32 =
        (fhd as ::core::ffi::c_int >> 5 as ::core::ffi::c_int & 1 as ::core::ffi::c_int) as U32;
    let fcsId: U32 = (fhd as ::core::ffi::c_int >> 6 as ::core::ffi::c_int) as U32;
    return minInputSize
        .wrapping_add((singleSegment == 0) as ::core::ffi::c_int as size_t)
        .wrapping_add(ZSTD_did_fieldSize[dictID as usize])
        .wrapping_add(ZSTD_fcs_fieldSize[fcsId as usize])
        .wrapping_add((singleSegment != 0 && fcsId == 0) as ::core::ffi::c_int as size_t);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_frameHeaderSize(
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    return ZSTD_frameHeaderSize_internal(src, srcSize, ZSTD_f_zstd1);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_getFrameHeader_advanced(
    mut zfhPtr: *mut ZSTD_FrameHeader,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
    mut format: ZSTD_format_e,
) -> size_t {
    let mut ip: *const BYTE = src as *const BYTE;
    let minInputSize: size_t = ZSTD_startingInputLength(format) as size_t;
    if srcSize > 0 as size_t {
        if src.is_null() {
            return -(ZSTD_error_GENERIC as ::core::ffi::c_int) as size_t;
        }
    }
    if srcSize < minInputSize {
        if srcSize > 0 as size_t
            && format as ::core::ffi::c_uint
                != ZSTD_f_zstd1_magicless as ::core::ffi::c_int as ::core::ffi::c_uint
        {
            let toCopy: size_t = if (4 as size_t) < srcSize {
                4 as size_t
            } else {
                srcSize
            };
            let mut hbuf: [::core::ffi::c_uchar; 4] = [0; 4];
            MEM_writeLE32(
                &raw mut hbuf as *mut ::core::ffi::c_uchar as *mut ::core::ffi::c_void,
                ZSTD_MAGICNUMBER as U32,
            );
            ::libc::memcpy(
                &raw mut hbuf as *mut ::core::ffi::c_uchar as *mut ::core::ffi::c_void,
                src,
                toCopy as ::libc::size_t,
            );
            if MEM_readLE32(
                &raw mut hbuf as *mut ::core::ffi::c_uchar as *const ::core::ffi::c_void,
            ) != ZSTD_MAGICNUMBER as U32
            {
                MEM_writeLE32(
                    &raw mut hbuf as *mut ::core::ffi::c_uchar as *mut ::core::ffi::c_void,
                    ZSTD_MAGIC_SKIPPABLE_START as U32,
                );
                ::libc::memcpy(
                    &raw mut hbuf as *mut ::core::ffi::c_uchar as *mut ::core::ffi::c_void,
                    src,
                    toCopy as ::libc::size_t,
                );
                if MEM_readLE32(
                    &raw mut hbuf as *mut ::core::ffi::c_uchar as *const ::core::ffi::c_void,
                ) & ZSTD_MAGIC_SKIPPABLE_MASK as U32
                    != ZSTD_MAGIC_SKIPPABLE_START as U32
                {
                    return -(ZSTD_error_prefix_unknown as ::core::ffi::c_int) as size_t;
                }
            }
        }
        return minInputSize;
    }
    ::libc::memset(
        zfhPtr as *mut ::core::ffi::c_void,
        0 as ::core::ffi::c_int,
        ::core::mem::size_of::<ZSTD_FrameHeader>() as ::libc::size_t,
    );
    if format as ::core::ffi::c_uint
        != ZSTD_f_zstd1_magicless as ::core::ffi::c_int as ::core::ffi::c_uint
        && MEM_readLE32(src) != ZSTD_MAGICNUMBER as U32
    {
        if MEM_readLE32(src) & ZSTD_MAGIC_SKIPPABLE_MASK as U32 == ZSTD_MAGIC_SKIPPABLE_START as U32
        {
            if srcSize < ZSTD_SKIPPABLEHEADERSIZE as size_t {
                return ZSTD_SKIPPABLEHEADERSIZE as size_t;
            }
            ::libc::memset(
                zfhPtr as *mut ::core::ffi::c_void,
                0 as ::core::ffi::c_int,
                ::core::mem::size_of::<ZSTD_FrameHeader>() as ::libc::size_t,
            );
            (*zfhPtr).frameType = ZSTD_skippableFrame;
            (*zfhPtr).dictID = MEM_readLE32(src).wrapping_sub(ZSTD_MAGIC_SKIPPABLE_START as U32)
                as ::core::ffi::c_uint;
            (*zfhPtr).headerSize = ZSTD_SKIPPABLEHEADERSIZE as ::core::ffi::c_uint;
            (*zfhPtr).frameContentSize = MEM_readLE32(
                (src as *const ::core::ffi::c_char).offset(ZSTD_FRAMEIDSIZE as isize)
                    as *const ::core::ffi::c_void,
            ) as ::core::ffi::c_ulonglong;
            return 0 as size_t;
        }
        return -(ZSTD_error_prefix_unknown as ::core::ffi::c_int) as size_t;
    }
    let fhsize: size_t = ZSTD_frameHeaderSize_internal(src, srcSize, format) as size_t;
    if srcSize < fhsize {
        return fhsize;
    }
    (*zfhPtr).headerSize = fhsize as U32 as ::core::ffi::c_uint;
    let fhdByte: BYTE = *ip.offset(minInputSize.wrapping_sub(1 as size_t) as isize);
    let mut pos: size_t = minInputSize;
    let dictIDSizeCode: U32 = (fhdByte as ::core::ffi::c_int & 3 as ::core::ffi::c_int) as U32;
    let checksumFlag: U32 =
        (fhdByte as ::core::ffi::c_int >> 2 as ::core::ffi::c_int & 1 as ::core::ffi::c_int) as U32;
    let singleSegment: U32 =
        (fhdByte as ::core::ffi::c_int >> 5 as ::core::ffi::c_int & 1 as ::core::ffi::c_int) as U32;
    let fcsID: U32 = (fhdByte as ::core::ffi::c_int >> 6 as ::core::ffi::c_int) as U32;
    let mut windowSize: U64 = 0 as U64;
    let mut dictID: U32 = 0 as U32;
    let mut frameContentSize: U64 = ZSTD_CONTENTSIZE_UNKNOWN as U64;
    if fhdByte as ::core::ffi::c_int & 0x8 as ::core::ffi::c_int != 0 as ::core::ffi::c_int {
        return -(ZSTD_error_frameParameter_unsupported as ::core::ffi::c_int) as size_t;
    }
    if singleSegment == 0 {
        let fresh0 = pos;
        pos = pos.wrapping_add(1);
        let wlByte: BYTE = *ip.offset(fresh0 as isize);
        let windowLog: U32 = ((wlByte as ::core::ffi::c_int >> 3 as ::core::ffi::c_int)
            + ZSTD_WINDOWLOG_ABSOLUTEMIN) as U32;
        if windowLog
            > (if ::core::mem::size_of::<size_t>() as usize == 4 as usize {
                30 as ::core::ffi::c_int
            } else {
                31 as ::core::ffi::c_int
            }) as U32
        {
            return -(ZSTD_error_frameParameter_windowTooLarge as ::core::ffi::c_int) as size_t;
        }
        windowSize = ((1 as ::core::ffi::c_ulonglong) << windowLog) as U64;
        windowSize = (windowSize as ::core::ffi::c_ulong).wrapping_add(
            (windowSize >> 3 as ::core::ffi::c_int)
                .wrapping_mul((wlByte as ::core::ffi::c_int & 7 as ::core::ffi::c_int) as U64)
                as ::core::ffi::c_ulong,
        ) as U64 as U64;
    }
    match dictIDSizeCode {
        1 => {
            dictID = *ip.offset(pos as isize) as U32;
            pos = pos.wrapping_add(1);
        }
        2 => {
            dictID = MEM_readLE16(ip.offset(pos as isize) as *const ::core::ffi::c_void) as U32;
            pos = (pos as ::core::ffi::c_ulong).wrapping_add(2 as ::core::ffi::c_ulong) as size_t
                as size_t;
        }
        3 => {
            dictID = MEM_readLE32(ip.offset(pos as isize) as *const ::core::ffi::c_void);
            pos = (pos as ::core::ffi::c_ulong).wrapping_add(4 as ::core::ffi::c_ulong) as size_t
                as size_t;
        }
        0 | _ => {}
    }
    match fcsID {
        1 => {
            frameContentSize = (MEM_readLE16(ip.offset(pos as isize) as *const ::core::ffi::c_void)
                as ::core::ffi::c_int
                + 256 as ::core::ffi::c_int) as U64;
        }
        2 => {
            frameContentSize =
                MEM_readLE32(ip.offset(pos as isize) as *const ::core::ffi::c_void) as U64;
        }
        3 => {
            frameContentSize = MEM_readLE64(ip.offset(pos as isize) as *const ::core::ffi::c_void);
        }
        0 | _ => {
            if singleSegment != 0 {
                frameContentSize = *ip.offset(pos as isize) as U64;
            }
        }
    }
    if singleSegment != 0 {
        windowSize = frameContentSize;
    }
    (*zfhPtr).frameType = ZSTD_frame;
    (*zfhPtr).frameContentSize = frameContentSize as ::core::ffi::c_ulonglong;
    (*zfhPtr).windowSize = windowSize as ::core::ffi::c_ulonglong;
    (*zfhPtr).blockSizeMax =
        (if windowSize < ((1 as ::core::ffi::c_int) << 17 as ::core::ffi::c_int) as U64 {
            windowSize
        } else {
            ((1 as ::core::ffi::c_int) << 17 as ::core::ffi::c_int) as U64
        }) as ::core::ffi::c_uint;
    (*zfhPtr).dictID = dictID as ::core::ffi::c_uint;
    (*zfhPtr).checksumFlag = checksumFlag as ::core::ffi::c_uint;
    return 0 as size_t;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_getFrameHeader(
    mut zfhPtr: *mut ZSTD_FrameHeader,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    return ZSTD_getFrameHeader_advanced(zfhPtr, src, srcSize, ZSTD_f_zstd1);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_getFrameContentSize(
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> ::core::ffi::c_ulonglong {
    if ZSTD_isLegacy(src, srcSize) != 0 {
        let ret: ::core::ffi::c_ulonglong =
            ZSTD_getDecompressedSize_legacy(src, srcSize) as ::core::ffi::c_ulonglong;
        return if ret == 0 as ::core::ffi::c_ulonglong {
            ZSTD_CONTENTSIZE_UNKNOWN
        } else {
            ret
        };
    }
    let mut zfh: ZSTD_FrameHeader = ZSTD_FrameHeader {
        frameContentSize: 0,
        windowSize: 0,
        blockSizeMax: 0,
        frameType: ZSTD_frame,
        headerSize: 0,
        dictID: 0,
        checksumFlag: 0,
        _reserved1: 0,
        _reserved2: 0,
    };
    if ZSTD_getFrameHeader(&raw mut zfh, src, srcSize) != 0 as size_t {
        return ZSTD_CONTENTSIZE_ERROR;
    }
    if zfh.frameType as ::core::ffi::c_uint
        == ZSTD_skippableFrame as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        return 0 as ::core::ffi::c_ulonglong;
    } else {
        return zfh.frameContentSize;
    };
}
unsafe extern "C" fn readSkippableFrameSize(
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    let skippableHeaderSize: size_t = ZSTD_SKIPPABLEHEADERSIZE as size_t;
    let mut sizeU32: U32 = 0;
    if srcSize < 8 as size_t {
        return -(ZSTD_error_srcSize_wrong as ::core::ffi::c_int) as size_t;
    }
    sizeU32 = MEM_readLE32(
        (src as *const BYTE).offset(ZSTD_FRAMEIDSIZE as isize) as *const ::core::ffi::c_void
    );
    if sizeU32.wrapping_add(8 as U32) < sizeU32 {
        return -(ZSTD_error_frameParameter_unsupported as ::core::ffi::c_int) as size_t;
    }
    let skippableSize: size_t = skippableHeaderSize.wrapping_add(sizeU32 as size_t);
    if skippableSize > srcSize {
        return -(ZSTD_error_srcSize_wrong as ::core::ffi::c_int) as size_t;
    }
    return skippableSize;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_readSkippableFrame(
    mut dst: *mut ::core::ffi::c_void,
    mut dstCapacity: size_t,
    mut magicVariant: *mut ::core::ffi::c_uint,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    if srcSize < 8 as size_t {
        return -(ZSTD_error_srcSize_wrong as ::core::ffi::c_int) as size_t;
    }
    let magicNumber: U32 = MEM_readLE32(src) as U32;
    let mut skippableFrameSize: size_t = readSkippableFrameSize(src, srcSize);
    let mut skippableContentSize: size_t =
        skippableFrameSize.wrapping_sub(ZSTD_SKIPPABLEHEADERSIZE as size_t);
    if ZSTD_isSkippableFrame(src, srcSize) == 0 {
        return -(ZSTD_error_frameParameter_unsupported as ::core::ffi::c_int) as size_t;
    }
    if skippableFrameSize < 8 as size_t || skippableFrameSize > srcSize {
        return -(ZSTD_error_srcSize_wrong as ::core::ffi::c_int) as size_t;
    }
    if skippableContentSize > dstCapacity {
        return -(ZSTD_error_dstSize_tooSmall as ::core::ffi::c_int) as size_t;
    }
    if skippableContentSize > 0 as size_t && !dst.is_null() {
        ::libc::memcpy(
            dst,
            (src as *const BYTE).offset(8 as ::core::ffi::c_int as isize)
                as *const ::core::ffi::c_void,
            skippableContentSize as ::libc::size_t,
        );
    }
    if !magicVariant.is_null() {
        *magicVariant =
            magicNumber.wrapping_sub(ZSTD_MAGIC_SKIPPABLE_START as U32) as ::core::ffi::c_uint;
    }
    return skippableContentSize;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_findDecompressedSize(
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> ::core::ffi::c_ulonglong {
    let mut totalDstSize: ::core::ffi::c_ulonglong = 0 as ::core::ffi::c_ulonglong;
    while srcSize >= ZSTD_startingInputLength(ZSTD_f_zstd1) {
        let magicNumber: U32 = MEM_readLE32(src) as U32;
        if magicNumber & ZSTD_MAGIC_SKIPPABLE_MASK as U32 == ZSTD_MAGIC_SKIPPABLE_START as U32 {
            let skippableSize: size_t = readSkippableFrameSize(src, srcSize) as size_t;
            if ERR_isError(skippableSize) != 0 {
                return ZSTD_CONTENTSIZE_ERROR;
            }
            src = (src as *const BYTE).offset(skippableSize as isize) as *const ::core::ffi::c_void;
            srcSize = (srcSize as ::core::ffi::c_ulong)
                .wrapping_sub(skippableSize as ::core::ffi::c_ulong) as size_t
                as size_t;
        } else {
            let fcs: ::core::ffi::c_ulonglong =
                ZSTD_getFrameContentSize(src, srcSize) as ::core::ffi::c_ulonglong;
            if fcs >= ZSTD_CONTENTSIZE_ERROR {
                return fcs;
            }
            if totalDstSize.wrapping_add(fcs) < totalDstSize {
                return ZSTD_CONTENTSIZE_ERROR;
            }
            totalDstSize = totalDstSize.wrapping_add(fcs);
            let frameSrcSize: size_t = ZSTD_findFrameCompressedSize(src, srcSize) as size_t;
            if ERR_isError(frameSrcSize) != 0 {
                return ZSTD_CONTENTSIZE_ERROR;
            }
            src = (src as *const BYTE).offset(frameSrcSize as isize) as *const ::core::ffi::c_void;
            srcSize = (srcSize as ::core::ffi::c_ulong)
                .wrapping_sub(frameSrcSize as ::core::ffi::c_ulong) as size_t
                as size_t;
        }
    }
    if srcSize != 0 {
        return ZSTD_CONTENTSIZE_ERROR;
    }
    return totalDstSize;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_getDecompressedSize(
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> ::core::ffi::c_ulonglong {
    let ret: ::core::ffi::c_ulonglong =
        ZSTD_getFrameContentSize(src, srcSize) as ::core::ffi::c_ulonglong;
    return if ret >= ZSTD_CONTENTSIZE_ERROR {
        0 as ::core::ffi::c_ulonglong
    } else {
        ret
    };
}
unsafe extern "C" fn ZSTD_decodeFrameHeader(
    mut dctx: *mut ZSTD_DCtx,
    mut src: *const ::core::ffi::c_void,
    mut headerSize: size_t,
) -> size_t {
    let result: size_t =
        ZSTD_getFrameHeader_advanced(&raw mut (*dctx).fParams, src, headerSize, (*dctx).format)
            as size_t;
    if ERR_isError(result) != 0 {
        return result;
    }
    if result > 0 as size_t {
        return -(ZSTD_error_srcSize_wrong as ::core::ffi::c_int) as size_t;
    }
    if (*dctx).refMultipleDDicts as ::core::ffi::c_uint
        == ZSTD_rmd_refMultipleDDicts as ::core::ffi::c_int as ::core::ffi::c_uint
        && !(*dctx).ddictSet.is_null()
    {
        ZSTD_DCtx_selectFrameDDict(dctx);
    }
    if (*dctx).fParams.dictID != 0 && (*dctx).dictID != (*dctx).fParams.dictID as U32 {
        return -(ZSTD_error_dictionary_wrong as ::core::ffi::c_int) as size_t;
    }
    (*dctx).validateChecksum =
        (if (*dctx).fParams.checksumFlag != 0 && (*dctx).forceIgnoreChecksum as u64 == 0 {
            1 as ::core::ffi::c_int
        } else {
            0 as ::core::ffi::c_int
        }) as U32;
    if (*dctx).validateChecksum != 0 {
        ZSTD_XXH64_reset(&raw mut (*dctx).xxhState, 0 as XXH64_hash_t);
    }
    (*dctx).processedCSize = ((*dctx).processedCSize as ::core::ffi::c_ulong)
        .wrapping_add(headerSize as ::core::ffi::c_ulong) as U64
        as U64;
    return 0 as size_t;
}
unsafe extern "C" fn ZSTD_errorFrameSizeInfo(mut ret: size_t) -> ZSTD_frameSizeInfo {
    let mut frameSizeInfo: ZSTD_frameSizeInfo = ZSTD_frameSizeInfo {
        nbBlocks: 0,
        compressedSize: 0,
        decompressedBound: 0,
    };
    frameSizeInfo.compressedSize = ret;
    frameSizeInfo.decompressedBound = ZSTD_CONTENTSIZE_ERROR;
    return frameSizeInfo;
}
unsafe extern "C" fn ZSTD_findFrameSizeInfo(
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
    mut format: ZSTD_format_e,
) -> ZSTD_frameSizeInfo {
    let mut frameSizeInfo: ZSTD_frameSizeInfo = ZSTD_frameSizeInfo {
        nbBlocks: 0,
        compressedSize: 0,
        decompressedBound: 0,
    };
    ::libc::memset(
        &raw mut frameSizeInfo as *mut ::core::ffi::c_void,
        0 as ::core::ffi::c_int,
        ::core::mem::size_of::<ZSTD_frameSizeInfo>() as ::libc::size_t,
    );
    if format as ::core::ffi::c_uint == ZSTD_f_zstd1 as ::core::ffi::c_int as ::core::ffi::c_uint
        && ZSTD_isLegacy(src, srcSize) != 0
    {
        return ZSTD_findFrameSizeInfoLegacy(src, srcSize);
    }
    if format as ::core::ffi::c_uint == ZSTD_f_zstd1 as ::core::ffi::c_int as ::core::ffi::c_uint
        && srcSize >= ZSTD_SKIPPABLEHEADERSIZE as size_t
        && MEM_readLE32(src) & ZSTD_MAGIC_SKIPPABLE_MASK as U32 == ZSTD_MAGIC_SKIPPABLE_START as U32
    {
        frameSizeInfo.compressedSize = readSkippableFrameSize(src, srcSize);
        return frameSizeInfo;
    } else {
        let mut ip: *const BYTE = src as *const BYTE;
        let ipstart: *const BYTE = ip;
        let mut remainingSize: size_t = srcSize;
        let mut nbBlocks: size_t = 0 as size_t;
        let mut zfh: ZSTD_FrameHeader = ZSTD_FrameHeader {
            frameContentSize: 0,
            windowSize: 0,
            blockSizeMax: 0,
            frameType: ZSTD_frame,
            headerSize: 0,
            dictID: 0,
            checksumFlag: 0,
            _reserved1: 0,
            _reserved2: 0,
        };
        let ret: size_t =
            ZSTD_getFrameHeader_advanced(&raw mut zfh, src, srcSize, format) as size_t;
        if ERR_isError(ret) != 0 {
            return ZSTD_errorFrameSizeInfo(ret);
        }
        if ret > 0 as size_t {
            return ZSTD_errorFrameSizeInfo(
                -(ZSTD_error_srcSize_wrong as ::core::ffi::c_int) as size_t,
            );
        }
        ip = ip.offset(zfh.headerSize as isize);
        remainingSize = (remainingSize as ::core::ffi::c_ulong)
            .wrapping_sub(zfh.headerSize as ::core::ffi::c_ulong) as size_t
            as size_t;
        loop {
            let mut blockProperties: blockProperties_t = blockProperties_t {
                blockType: bt_raw,
                lastBlock: 0,
                origSize: 0,
            };
            let cBlockSize: size_t = ZSTD_getcBlockSize(
                ip as *const ::core::ffi::c_void,
                remainingSize,
                &raw mut blockProperties,
            ) as size_t;
            if ERR_isError(cBlockSize) != 0 {
                return ZSTD_errorFrameSizeInfo(cBlockSize);
            }
            if ZSTD_blockHeaderSize.wrapping_add(cBlockSize) > remainingSize {
                return ZSTD_errorFrameSizeInfo(
                    -(ZSTD_error_srcSize_wrong as ::core::ffi::c_int) as size_t,
                );
            }
            ip = ip.offset(ZSTD_blockHeaderSize.wrapping_add(cBlockSize) as isize);
            remainingSize = (remainingSize as ::core::ffi::c_ulong)
                .wrapping_sub(ZSTD_blockHeaderSize.wrapping_add(cBlockSize) as ::core::ffi::c_ulong)
                as size_t as size_t;
            nbBlocks = nbBlocks.wrapping_add(1);
            if blockProperties.lastBlock != 0 {
                break;
            }
        }
        if zfh.checksumFlag != 0 {
            if remainingSize < 4 as size_t {
                return ZSTD_errorFrameSizeInfo(
                    -(ZSTD_error_srcSize_wrong as ::core::ffi::c_int) as size_t,
                );
            }
            ip = ip.offset(4 as ::core::ffi::c_int as isize);
        }
        frameSizeInfo.nbBlocks = nbBlocks;
        frameSizeInfo.compressedSize = ip.offset_from(ipstart) as ::core::ffi::c_long as size_t;
        frameSizeInfo.decompressedBound = if zfh.frameContentSize != ZSTD_CONTENTSIZE_UNKNOWN {
            zfh.frameContentSize
        } else {
            (nbBlocks as ::core::ffi::c_ulonglong)
                .wrapping_mul(zfh.blockSizeMax as ::core::ffi::c_ulonglong)
        };
        return frameSizeInfo;
    };
}
unsafe extern "C" fn ZSTD_findFrameCompressedSize_advanced(
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
    mut format: ZSTD_format_e,
) -> size_t {
    let frameSizeInfo: ZSTD_frameSizeInfo =
        ZSTD_findFrameSizeInfo(src, srcSize, format) as ZSTD_frameSizeInfo;
    return frameSizeInfo.compressedSize;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_findFrameCompressedSize(
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    return ZSTD_findFrameCompressedSize_advanced(src, srcSize, ZSTD_f_zstd1);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_decompressBound(
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> ::core::ffi::c_ulonglong {
    let mut bound: ::core::ffi::c_ulonglong = 0 as ::core::ffi::c_ulonglong;
    while srcSize > 0 as size_t {
        let frameSizeInfo: ZSTD_frameSizeInfo =
            ZSTD_findFrameSizeInfo(src, srcSize, ZSTD_f_zstd1) as ZSTD_frameSizeInfo;
        let compressedSize: size_t = frameSizeInfo.compressedSize;
        let decompressedBound: ::core::ffi::c_ulonglong = frameSizeInfo.decompressedBound;
        if ERR_isError(compressedSize) != 0 || decompressedBound == ZSTD_CONTENTSIZE_ERROR {
            return ZSTD_CONTENTSIZE_ERROR;
        }
        src = (src as *const BYTE).offset(compressedSize as isize) as *const ::core::ffi::c_void;
        srcSize = (srcSize as ::core::ffi::c_ulong)
            .wrapping_sub(compressedSize as ::core::ffi::c_ulong) as size_t
            as size_t;
        bound = bound.wrapping_add(decompressedBound);
    }
    return bound;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_decompressionMargin(
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    let mut margin: size_t = 0 as size_t;
    let mut maxBlockSize: ::core::ffi::c_uint = 0 as ::core::ffi::c_uint;
    while srcSize > 0 as size_t {
        let frameSizeInfo: ZSTD_frameSizeInfo =
            ZSTD_findFrameSizeInfo(src, srcSize, ZSTD_f_zstd1) as ZSTD_frameSizeInfo;
        let compressedSize: size_t = frameSizeInfo.compressedSize;
        let decompressedBound: ::core::ffi::c_ulonglong = frameSizeInfo.decompressedBound;
        let mut zfh: ZSTD_FrameHeader = ZSTD_FrameHeader {
            frameContentSize: 0,
            windowSize: 0,
            blockSizeMax: 0,
            frameType: ZSTD_frame,
            headerSize: 0,
            dictID: 0,
            checksumFlag: 0,
            _reserved1: 0,
            _reserved2: 0,
        };
        let err_code: size_t = ZSTD_getFrameHeader(&raw mut zfh, src, srcSize) as size_t;
        if ERR_isError(err_code) != 0 {
            return err_code;
        }
        if ERR_isError(compressedSize) != 0 || decompressedBound == ZSTD_CONTENTSIZE_ERROR {
            return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
        }
        if zfh.frameType as ::core::ffi::c_uint
            == ZSTD_frame as ::core::ffi::c_int as ::core::ffi::c_uint
        {
            margin = (margin as ::core::ffi::c_ulong)
                .wrapping_add(zfh.headerSize as ::core::ffi::c_ulong) as size_t
                as size_t;
            margin = (margin as ::core::ffi::c_ulong).wrapping_add(
                (if zfh.checksumFlag != 0 {
                    4 as ::core::ffi::c_int
                } else {
                    0 as ::core::ffi::c_int
                }) as ::core::ffi::c_ulong,
            ) as size_t as size_t;
            margin = (margin as ::core::ffi::c_ulong).wrapping_add(
                (3 as size_t).wrapping_mul(frameSizeInfo.nbBlocks) as ::core::ffi::c_ulong,
            ) as size_t as size_t;
            maxBlockSize = if maxBlockSize > zfh.blockSizeMax {
                maxBlockSize
            } else {
                zfh.blockSizeMax
            };
        } else {
            margin = (margin as ::core::ffi::c_ulong)
                .wrapping_add(compressedSize as ::core::ffi::c_ulong) as size_t
                as size_t;
        }
        src = (src as *const BYTE).offset(compressedSize as isize) as *const ::core::ffi::c_void;
        srcSize = (srcSize as ::core::ffi::c_ulong)
            .wrapping_sub(compressedSize as ::core::ffi::c_ulong) as size_t
            as size_t;
    }
    margin = (margin as ::core::ffi::c_ulong).wrapping_add(maxBlockSize as ::core::ffi::c_ulong)
        as size_t as size_t;
    return margin;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_insertBlock(
    mut dctx: *mut ZSTD_DCtx,
    mut blockStart: *const ::core::ffi::c_void,
    mut blockSize: size_t,
) -> size_t {
    ZSTD_checkContinuity(dctx, blockStart, blockSize);
    (*dctx).previousDstEnd = (blockStart as *const ::core::ffi::c_char).offset(blockSize as isize)
        as *const ::core::ffi::c_void;
    return blockSize;
}
unsafe extern "C" fn ZSTD_copyRawBlock(
    mut dst: *mut ::core::ffi::c_void,
    mut dstCapacity: size_t,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    if srcSize > dstCapacity {
        return -(ZSTD_error_dstSize_tooSmall as ::core::ffi::c_int) as size_t;
    }
    if dst.is_null() {
        if srcSize == 0 as size_t {
            return 0 as size_t;
        }
        return -(ZSTD_error_dstBuffer_null as ::core::ffi::c_int) as size_t;
    }
    ::libc::memmove(dst, src, srcSize as ::libc::size_t);
    return srcSize;
}
unsafe extern "C" fn ZSTD_setRleBlock(
    mut dst: *mut ::core::ffi::c_void,
    mut dstCapacity: size_t,
    mut b: BYTE,
    mut regenSize: size_t,
) -> size_t {
    if regenSize > dstCapacity {
        return -(ZSTD_error_dstSize_tooSmall as ::core::ffi::c_int) as size_t;
    }
    if dst.is_null() {
        if regenSize == 0 as size_t {
            return 0 as size_t;
        }
        return -(ZSTD_error_dstBuffer_null as ::core::ffi::c_int) as size_t;
    }
    ::libc::memset(dst, b as ::core::ffi::c_int, regenSize as ::libc::size_t);
    return regenSize;
}
unsafe extern "C" fn ZSTD_DCtx_trace_end(
    mut dctx: *const ZSTD_DCtx,
    mut uncompressedSize: U64,
    mut compressedSize: U64,
    mut streaming: ::core::ffi::c_int,
) {
    let _ = (dctx, uncompressedSize, compressedSize, streaming);
}
unsafe extern "C" fn ZSTD_decompressFrame(
    mut dctx: *mut ZSTD_DCtx,
    mut dst: *mut ::core::ffi::c_void,
    mut dstCapacity: size_t,
    mut srcPtr: *mut *const ::core::ffi::c_void,
    mut srcSizePtr: *mut size_t,
) -> size_t {
    let istart: *const BYTE = *srcPtr as *const BYTE;
    let mut ip: *const BYTE = istart;
    let ostart: *mut BYTE = dst as *mut BYTE;
    let oend: *mut BYTE = if dstCapacity != 0 as size_t {
        ostart.offset(dstCapacity as isize)
    } else {
        ostart
    };
    let mut op: *mut BYTE = ostart;
    let mut remainingSrcSize: size_t = *srcSizePtr;
    if remainingSrcSize
        < ((if (*dctx).format as ::core::ffi::c_uint
            == ZSTD_f_zstd1 as ::core::ffi::c_int as ::core::ffi::c_uint
        {
            6 as ::core::ffi::c_int
        } else {
            2 as ::core::ffi::c_int
        }) as size_t)
            .wrapping_add(ZSTD_blockHeaderSize)
    {
        return -(ZSTD_error_srcSize_wrong as ::core::ffi::c_int) as size_t;
    }
    let frameHeaderSize: size_t = ZSTD_frameHeaderSize_internal(
        ip as *const ::core::ffi::c_void,
        (if (*dctx).format as ::core::ffi::c_uint
            == ZSTD_f_zstd1 as ::core::ffi::c_int as ::core::ffi::c_uint
        {
            5 as ::core::ffi::c_int
        } else {
            1 as ::core::ffi::c_int
        }) as size_t,
        (*dctx).format,
    ) as size_t;
    if ERR_isError(frameHeaderSize) != 0 {
        return frameHeaderSize;
    }
    if remainingSrcSize < frameHeaderSize.wrapping_add(ZSTD_blockHeaderSize) {
        return -(ZSTD_error_srcSize_wrong as ::core::ffi::c_int) as size_t;
    }
    let err_code: size_t =
        ZSTD_decodeFrameHeader(dctx, ip as *const ::core::ffi::c_void, frameHeaderSize) as size_t;
    if ERR_isError(err_code) != 0 {
        return err_code;
    }
    ip = ip.offset(frameHeaderSize as isize);
    remainingSrcSize = (remainingSrcSize as ::core::ffi::c_ulong)
        .wrapping_sub(frameHeaderSize as ::core::ffi::c_ulong) as size_t
        as size_t;
    if (*dctx).maxBlockSizeParam != 0 as ::core::ffi::c_int {
        (*dctx).fParams.blockSizeMax =
            if (*dctx).fParams.blockSizeMax < (*dctx).maxBlockSizeParam as ::core::ffi::c_uint {
                (*dctx).fParams.blockSizeMax
            } else {
                (*dctx).maxBlockSizeParam as ::core::ffi::c_uint
            };
    }
    loop {
        let mut oBlockEnd: *mut BYTE = oend;
        let mut decodedSize: size_t = 0;
        let mut blockProperties: blockProperties_t = blockProperties_t {
            blockType: bt_raw,
            lastBlock: 0,
            origSize: 0,
        };
        let cBlockSize: size_t = ZSTD_getcBlockSize(
            ip as *const ::core::ffi::c_void,
            remainingSrcSize,
            &raw mut blockProperties,
        ) as size_t;
        if ERR_isError(cBlockSize) != 0 {
            return cBlockSize;
        }
        ip = ip.offset(ZSTD_blockHeaderSize as isize);
        remainingSrcSize = (remainingSrcSize as ::core::ffi::c_ulong)
            .wrapping_sub(ZSTD_blockHeaderSize as ::core::ffi::c_ulong)
            as size_t as size_t;
        if cBlockSize > remainingSrcSize {
            return -(ZSTD_error_srcSize_wrong as ::core::ffi::c_int) as size_t;
        }
        if ip >= op as *const BYTE && ip < oBlockEnd as *const BYTE {
            oBlockEnd = op.offset(ip.offset_from(op) as ::core::ffi::c_long as isize);
        }
        match blockProperties.blockType as ::core::ffi::c_uint {
            2 => {
                decodedSize = ZSTD_decompressBlock_internal(
                    dctx,
                    op as *mut ::core::ffi::c_void,
                    oBlockEnd.offset_from(op) as ::core::ffi::c_long as size_t,
                    ip as *const ::core::ffi::c_void,
                    cBlockSize,
                    not_streaming,
                );
            }
            0 => {
                decodedSize = ZSTD_copyRawBlock(
                    op as *mut ::core::ffi::c_void,
                    oend.offset_from(op) as ::core::ffi::c_long as size_t,
                    ip as *const ::core::ffi::c_void,
                    cBlockSize,
                );
            }
            1 => {
                decodedSize = ZSTD_setRleBlock(
                    op as *mut ::core::ffi::c_void,
                    oBlockEnd.offset_from(op) as ::core::ffi::c_long as size_t,
                    *ip,
                    blockProperties.origSize as size_t,
                );
            }
            3 | _ => {
                return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
            }
        }
        let err_code_0: size_t = decodedSize;
        if ERR_isError(err_code_0) != 0 {
            return err_code_0;
        }
        if (*dctx).validateChecksum != 0 {
            ZSTD_XXH64_update(
                &raw mut (*dctx).xxhState,
                op as *const ::core::ffi::c_void,
                decodedSize,
            );
        }
        if decodedSize != 0 {
            op = op.offset(decodedSize as isize);
        }
        ip = ip.offset(cBlockSize as isize);
        remainingSrcSize = (remainingSrcSize as ::core::ffi::c_ulong)
            .wrapping_sub(cBlockSize as ::core::ffi::c_ulong) as size_t
            as size_t;
        if blockProperties.lastBlock != 0 {
            break;
        }
    }
    if (*dctx).fParams.frameContentSize != ZSTD_CONTENTSIZE_UNKNOWN {
        if op.offset_from(ostart) as ::core::ffi::c_long as U64 as ::core::ffi::c_ulonglong
            != (*dctx).fParams.frameContentSize
        {
            return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
        }
    }
    if (*dctx).fParams.checksumFlag != 0 {
        if remainingSrcSize < 4 as size_t {
            return -(ZSTD_error_checksum_wrong as ::core::ffi::c_int) as size_t;
        }
        if (*dctx).forceIgnoreChecksum as u64 == 0 {
            let checkCalc: U32 = ZSTD_XXH64_digest(&raw mut (*dctx).xxhState) as U32;
            let mut checkRead: U32 = 0;
            checkRead = MEM_readLE32(ip as *const ::core::ffi::c_void);
            if checkRead != checkCalc {
                return -(ZSTD_error_checksum_wrong as ::core::ffi::c_int) as size_t;
            }
        }
        ip = ip.offset(4 as ::core::ffi::c_int as isize);
        remainingSrcSize = (remainingSrcSize as ::core::ffi::c_ulong)
            .wrapping_sub(4 as ::core::ffi::c_ulong) as size_t as size_t;
    }
    ZSTD_DCtx_trace_end(
        dctx,
        op.offset_from(ostart) as ::core::ffi::c_long as U64,
        ip.offset_from(istart) as ::core::ffi::c_long as U64,
        0 as ::core::ffi::c_int,
    );
    *srcPtr = ip as *const ::core::ffi::c_void;
    *srcSizePtr = remainingSrcSize;
    return op.offset_from(ostart) as ::core::ffi::c_long as size_t;
}
unsafe extern "C" fn ZSTD_decompressMultiFrame(
    mut dctx: *mut ZSTD_DCtx,
    mut dst: *mut ::core::ffi::c_void,
    mut dstCapacity: size_t,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
    mut dict: *const ::core::ffi::c_void,
    mut dictSize: size_t,
    mut ddict: *const ZSTD_DDict,
) -> size_t {
    let dststart: *mut ::core::ffi::c_void = dst;
    let mut moreThan1Frame: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    if !ddict.is_null() {
        dict = ZSTD_DDict_dictContent(ddict);
        dictSize = ZSTD_DDict_dictSize(ddict);
    }
    while srcSize >= ZSTD_startingInputLength((*dctx).format) {
        if (*dctx).format as ::core::ffi::c_uint
            == ZSTD_f_zstd1 as ::core::ffi::c_int as ::core::ffi::c_uint
            && ZSTD_isLegacy(src, srcSize) != 0
        {
            let mut decodedSize: size_t = 0;
            let frameSize: size_t = ZSTD_findFrameCompressedSizeLegacy(src, srcSize) as size_t;
            if ERR_isError(frameSize) != 0 {
                return frameSize;
            }
            if (*dctx).staticSize != 0 {
                return -(ZSTD_error_memory_allocation as ::core::ffi::c_int) as size_t;
            }
            decodedSize = ZSTD_decompressLegacy(dst, dstCapacity, src, frameSize, dict, dictSize);
            if ERR_isError(decodedSize) != 0 {
                return decodedSize;
            }
            let expectedSize: ::core::ffi::c_ulonglong =
                ZSTD_getFrameContentSize(src, srcSize) as ::core::ffi::c_ulonglong;
            if expectedSize
                == (0 as ::core::ffi::c_ulonglong).wrapping_sub(2 as ::core::ffi::c_ulonglong)
            {
                return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
            }
            if expectedSize != ZSTD_CONTENTSIZE_UNKNOWN {
                if expectedSize != decodedSize as ::core::ffi::c_ulonglong {
                    return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
                }
            }
            dst = (dst as *mut BYTE).offset(decodedSize as isize) as *mut ::core::ffi::c_void;
            dstCapacity = (dstCapacity as ::core::ffi::c_ulong)
                .wrapping_sub(decodedSize as ::core::ffi::c_ulong)
                as size_t as size_t;
            src = (src as *const BYTE).offset(frameSize as isize) as *const ::core::ffi::c_void;
            srcSize = (srcSize as ::core::ffi::c_ulong)
                .wrapping_sub(frameSize as ::core::ffi::c_ulong) as size_t
                as size_t;
        } else {
            if (*dctx).format as ::core::ffi::c_uint
                == ZSTD_f_zstd1 as ::core::ffi::c_int as ::core::ffi::c_uint
                && srcSize >= 4 as size_t
            {
                let magicNumber: U32 = MEM_readLE32(src) as U32;
                if magicNumber & ZSTD_MAGIC_SKIPPABLE_MASK as U32
                    == ZSTD_MAGIC_SKIPPABLE_START as U32
                {
                    let skippableSize: size_t = readSkippableFrameSize(src, srcSize) as size_t;
                    let err_code: size_t = skippableSize;
                    if ERR_isError(err_code) != 0 {
                        return err_code;
                    }
                    src = (src as *const BYTE).offset(skippableSize as isize)
                        as *const ::core::ffi::c_void;
                    srcSize = (srcSize as ::core::ffi::c_ulong)
                        .wrapping_sub(skippableSize as ::core::ffi::c_ulong)
                        as size_t as size_t;
                    continue;
                }
            }
            if !ddict.is_null() {
                let err_code_0: size_t = ZSTD_decompressBegin_usingDDict(dctx, ddict) as size_t;
                if ERR_isError(err_code_0) != 0 {
                    return err_code_0;
                }
            } else {
                let err_code_1: size_t =
                    ZSTD_decompressBegin_usingDict(dctx, dict, dictSize) as size_t;
                if ERR_isError(err_code_1) != 0 {
                    return err_code_1;
                }
            }
            ZSTD_checkContinuity(dctx, dst, dstCapacity);
            let res: size_t =
                ZSTD_decompressFrame(dctx, dst, dstCapacity, &raw mut src, &raw mut srcSize)
                    as size_t;
            if ZSTD_getErrorCode(res) as ::core::ffi::c_uint
                == ZSTD_error_prefix_unknown as ::core::ffi::c_int as ::core::ffi::c_uint
                && moreThan1Frame == 1 as ::core::ffi::c_int
            {
                return -(ZSTD_error_srcSize_wrong as ::core::ffi::c_int) as size_t;
            }
            if ERR_isError(res) != 0 {
                return res;
            }
            if res != 0 as size_t {
                dst = (dst as *mut BYTE).offset(res as isize) as *mut ::core::ffi::c_void;
            }
            dstCapacity = (dstCapacity as ::core::ffi::c_ulong)
                .wrapping_sub(res as ::core::ffi::c_ulong) as size_t
                as size_t;
            moreThan1Frame = 1 as ::core::ffi::c_int;
        }
    }
    if srcSize != 0 {
        return -(ZSTD_error_srcSize_wrong as ::core::ffi::c_int) as size_t;
    }
    return (dst as *mut BYTE).offset_from(dststart as *mut BYTE) as ::core::ffi::c_long as size_t;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_decompress_usingDict(
    mut dctx: *mut ZSTD_DCtx,
    mut dst: *mut ::core::ffi::c_void,
    mut dstCapacity: size_t,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
    mut dict: *const ::core::ffi::c_void,
    mut dictSize: size_t,
) -> size_t {
    return ZSTD_decompressMultiFrame(
        dctx,
        dst,
        dstCapacity,
        src,
        srcSize,
        dict,
        dictSize,
        ::core::ptr::null::<ZSTD_DDict>(),
    );
}
unsafe extern "C" fn ZSTD_getDDict(mut dctx: *mut ZSTD_DCtx) -> *const ZSTD_DDict {
    match (*dctx).dictUses as ::core::ffi::c_int {
        -1 => return (*dctx).ddict,
        1 => {
            (*dctx).dictUses = ZSTD_dont_use;
            return (*dctx).ddict;
        }
        0 | _ => {
            ZSTD_clearDict(dctx);
            return ::core::ptr::null::<ZSTD_DDict>();
        }
    };
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_decompressDCtx(
    mut dctx: *mut ZSTD_DCtx,
    mut dst: *mut ::core::ffi::c_void,
    mut dstCapacity: size_t,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    return ZSTD_decompress_usingDDict(dctx, dst, dstCapacity, src, srcSize, ZSTD_getDDict(dctx));
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_decompress(
    mut dst: *mut ::core::ffi::c_void,
    mut dstCapacity: size_t,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    let mut regenSize: size_t = 0;
    let dctx: *mut ZSTD_DCtx = ZSTD_createDCtx_internal(ZSTD_defaultCMem) as *mut ZSTD_DCtx;
    if dctx.is_null() {
        return -(ZSTD_error_memory_allocation as ::core::ffi::c_int) as size_t;
    }
    regenSize = ZSTD_decompressDCtx(dctx, dst, dstCapacity, src, srcSize);
    ZSTD_freeDCtx(dctx);
    return regenSize;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_nextSrcSizeToDecompress(mut dctx: *mut ZSTD_DCtx) -> size_t {
    return (*dctx).expected;
}
unsafe extern "C" fn ZSTD_nextSrcSizeToDecompressWithInputSize(
    mut dctx: *mut ZSTD_DCtx,
    mut inputSize: size_t,
) -> size_t {
    if !((*dctx).stage as ::core::ffi::c_uint
        == ZSTDds_decompressBlock as ::core::ffi::c_int as ::core::ffi::c_uint
        || (*dctx).stage as ::core::ffi::c_uint
            == ZSTDds_decompressLastBlock as ::core::ffi::c_int as ::core::ffi::c_uint)
    {
        return (*dctx).expected;
    }
    if (*dctx).bType as ::core::ffi::c_uint != bt_raw as ::core::ffi::c_int as ::core::ffi::c_uint {
        return (*dctx).expected;
    }
    return if 1 as size_t
        > (if inputSize < (*dctx).expected {
            inputSize
        } else {
            (*dctx).expected
        }) {
        1 as size_t
    } else if inputSize < (*dctx).expected {
        inputSize
    } else {
        (*dctx).expected
    };
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_nextInputType(mut dctx: *mut ZSTD_DCtx) -> ZSTD_nextInputType_e {
    match (*dctx).stage as ::core::ffi::c_uint {
        2 => return ZSTDnit_blockHeader,
        3 => return ZSTDnit_block,
        4 => return ZSTDnit_lastBlock,
        5 => return ZSTDnit_checksum,
        6 | 7 => return ZSTDnit_skippableFrame,
        0 | 1 | _ => return ZSTDnit_frameHeader,
    };
}
unsafe extern "C" fn ZSTD_isSkipFrame(mut dctx: *mut ZSTD_DCtx) -> ::core::ffi::c_int {
    return ((*dctx).stage as ::core::ffi::c_uint
        == ZSTDds_skipFrame as ::core::ffi::c_int as ::core::ffi::c_uint)
        as ::core::ffi::c_int;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_decompressContinue(
    mut dctx: *mut ZSTD_DCtx,
    mut dst: *mut ::core::ffi::c_void,
    mut dstCapacity: size_t,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    if srcSize != ZSTD_nextSrcSizeToDecompressWithInputSize(dctx, srcSize) {
        return -(ZSTD_error_srcSize_wrong as ::core::ffi::c_int) as size_t;
    }
    ZSTD_checkContinuity(dctx, dst, dstCapacity);
    (*dctx).processedCSize = ((*dctx).processedCSize as ::core::ffi::c_ulong)
        .wrapping_add(srcSize as ::core::ffi::c_ulong) as U64 as U64;
    match (*dctx).stage as ::core::ffi::c_uint {
        0 => {
            if (*dctx).format as ::core::ffi::c_uint
                == ZSTD_f_zstd1 as ::core::ffi::c_int as ::core::ffi::c_uint
            {
                if MEM_readLE32(src) & ZSTD_MAGIC_SKIPPABLE_MASK as U32
                    == ZSTD_MAGIC_SKIPPABLE_START as U32
                {
                    ::libc::memcpy(
                        &raw mut (*dctx).headerBuffer as *mut BYTE as *mut ::core::ffi::c_void,
                        src,
                        srcSize as ::libc::size_t,
                    );
                    (*dctx).expected = (ZSTD_SKIPPABLEHEADERSIZE as size_t).wrapping_sub(srcSize);
                    (*dctx).stage = ZSTDds_decodeSkippableHeader;
                    return 0 as size_t;
                }
            }
            (*dctx).headerSize = ZSTD_frameHeaderSize_internal(src, srcSize, (*dctx).format);
            if ERR_isError((*dctx).headerSize) != 0 {
                return (*dctx).headerSize;
            }
            ::libc::memcpy(
                &raw mut (*dctx).headerBuffer as *mut BYTE as *mut ::core::ffi::c_void,
                src,
                srcSize as ::libc::size_t,
            );
            (*dctx).expected = (*dctx).headerSize.wrapping_sub(srcSize);
            (*dctx).stage = ZSTDds_decodeFrameHeader;
            return 0 as size_t;
        }
        1 => {
            ::libc::memcpy(
                (&raw mut (*dctx).headerBuffer as *mut BYTE)
                    .offset((*dctx).headerSize.wrapping_sub(srcSize) as isize)
                    as *mut ::core::ffi::c_void,
                src,
                srcSize as ::libc::size_t,
            );
            let err_code: size_t = ZSTD_decodeFrameHeader(
                dctx,
                &raw mut (*dctx).headerBuffer as *mut BYTE as *const ::core::ffi::c_void,
                (*dctx).headerSize,
            ) as size_t;
            if ERR_isError(err_code) != 0 {
                return err_code;
            }
            (*dctx).expected = ZSTD_blockHeaderSize;
            (*dctx).stage = ZSTDds_decodeBlockHeader;
            return 0 as size_t;
        }
        2 => {
            let mut bp: blockProperties_t = blockProperties_t {
                blockType: bt_raw,
                lastBlock: 0,
                origSize: 0,
            };
            let cBlockSize: size_t =
                ZSTD_getcBlockSize(src, ZSTD_blockHeaderSize, &raw mut bp) as size_t;
            if ERR_isError(cBlockSize) != 0 {
                return cBlockSize;
            }
            if cBlockSize > (*dctx).fParams.blockSizeMax as size_t {
                return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
            }
            (*dctx).expected = cBlockSize;
            (*dctx).bType = bp.blockType;
            (*dctx).rleSize = bp.origSize as size_t;
            if cBlockSize != 0 {
                (*dctx).stage = (if bp.lastBlock != 0 {
                    ZSTDds_decompressLastBlock as ::core::ffi::c_int
                } else {
                    ZSTDds_decompressBlock as ::core::ffi::c_int
                }) as ZSTD_dStage;
                return 0 as size_t;
            }
            if bp.lastBlock != 0 {
                if (*dctx).fParams.checksumFlag != 0 {
                    (*dctx).expected = 4 as size_t;
                    (*dctx).stage = ZSTDds_checkChecksum;
                } else {
                    (*dctx).expected = 0 as size_t;
                    (*dctx).stage = ZSTDds_getFrameHeaderSize;
                }
            } else {
                (*dctx).expected = ZSTD_blockHeaderSize;
                (*dctx).stage = ZSTDds_decodeBlockHeader;
            }
            return 0 as size_t;
        }
        4 | 3 => {
            let mut rSize: size_t = 0;
            match (*dctx).bType as ::core::ffi::c_uint {
                2 => {
                    rSize = ZSTD_decompressBlock_internal(
                        dctx,
                        dst,
                        dstCapacity,
                        src,
                        srcSize,
                        is_streaming,
                    );
                    (*dctx).expected = 0 as size_t;
                }
                0 => {
                    rSize = ZSTD_copyRawBlock(dst, dstCapacity, src, srcSize);
                    let err_code_0: size_t = rSize;
                    if ERR_isError(err_code_0) != 0 {
                        return err_code_0;
                    }
                    (*dctx).expected = ((*dctx).expected as ::core::ffi::c_ulong)
                        .wrapping_sub(rSize as ::core::ffi::c_ulong)
                        as size_t as size_t;
                }
                1 => {
                    rSize =
                        ZSTD_setRleBlock(dst, dstCapacity, *(src as *const BYTE), (*dctx).rleSize);
                    (*dctx).expected = 0 as size_t;
                }
                3 | _ => {
                    return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
                }
            }
            let err_code_1: size_t = rSize;
            if ERR_isError(err_code_1) != 0 {
                return err_code_1;
            }
            if rSize > (*dctx).fParams.blockSizeMax as size_t {
                return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
            }
            (*dctx).decodedSize = ((*dctx).decodedSize as ::core::ffi::c_ulong)
                .wrapping_add(rSize as ::core::ffi::c_ulong)
                as U64 as U64;
            if (*dctx).validateChecksum != 0 {
                ZSTD_XXH64_update(&raw mut (*dctx).xxhState, dst, rSize);
            }
            (*dctx).previousDstEnd = (dst as *mut ::core::ffi::c_char).offset(rSize as isize)
                as *const ::core::ffi::c_void;
            if (*dctx).expected > 0 as size_t {
                return rSize;
            }
            if (*dctx).stage as ::core::ffi::c_uint
                == ZSTDds_decompressLastBlock as ::core::ffi::c_int as ::core::ffi::c_uint
            {
                if (*dctx).fParams.frameContentSize
                    != (0 as ::core::ffi::c_ulonglong).wrapping_sub(1 as ::core::ffi::c_ulonglong)
                    && (*dctx).decodedSize as ::core::ffi::c_ulonglong
                        != (*dctx).fParams.frameContentSize
                {
                    return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
                }
                if (*dctx).fParams.checksumFlag != 0 {
                    (*dctx).expected = 4 as size_t;
                    (*dctx).stage = ZSTDds_checkChecksum;
                } else {
                    ZSTD_DCtx_trace_end(
                        dctx,
                        (*dctx).decodedSize,
                        (*dctx).processedCSize,
                        1 as ::core::ffi::c_int,
                    );
                    (*dctx).expected = 0 as size_t;
                    (*dctx).stage = ZSTDds_getFrameHeaderSize;
                }
            } else {
                (*dctx).stage = ZSTDds_decodeBlockHeader;
                (*dctx).expected = ZSTD_blockHeaderSize;
            }
            return rSize;
        }
        5 => {
            if (*dctx).validateChecksum != 0 {
                let h32: U32 = ZSTD_XXH64_digest(&raw mut (*dctx).xxhState) as U32;
                let check32: U32 = MEM_readLE32(src) as U32;
                if check32 != h32 {
                    return -(ZSTD_error_checksum_wrong as ::core::ffi::c_int) as size_t;
                }
            }
            ZSTD_DCtx_trace_end(
                dctx,
                (*dctx).decodedSize,
                (*dctx).processedCSize,
                1 as ::core::ffi::c_int,
            );
            (*dctx).expected = 0 as size_t;
            (*dctx).stage = ZSTDds_getFrameHeaderSize;
            return 0 as size_t;
        }
        6 => {
            ::libc::memcpy(
                (&raw mut (*dctx).headerBuffer as *mut BYTE)
                    .offset((8 as size_t).wrapping_sub(srcSize) as isize)
                    as *mut ::core::ffi::c_void,
                src,
                srcSize as ::libc::size_t,
            );
            (*dctx).expected = MEM_readLE32(
                (&raw mut (*dctx).headerBuffer as *mut BYTE).offset(ZSTD_FRAMEIDSIZE as isize)
                    as *const ::core::ffi::c_void,
            ) as size_t;
            (*dctx).stage = ZSTDds_skipFrame;
            return 0 as size_t;
        }
        7 => {
            (*dctx).expected = 0 as size_t;
            (*dctx).stage = ZSTDds_getFrameHeaderSize;
            return 0 as size_t;
        }
        _ => return -(ZSTD_error_GENERIC as ::core::ffi::c_int) as size_t,
    };
}
unsafe extern "C" fn ZSTD_refDictContent(
    mut dctx: *mut ZSTD_DCtx,
    mut dict: *const ::core::ffi::c_void,
    mut dictSize: size_t,
) -> size_t {
    (*dctx).dictEnd = (*dctx).previousDstEnd;
    (*dctx).virtualStart = (dict as *const ::core::ffi::c_char).offset(
        -(((*dctx).previousDstEnd as *const ::core::ffi::c_char)
            .offset_from((*dctx).prefixStart as *const ::core::ffi::c_char)
            as ::core::ffi::c_long as isize),
    ) as *const ::core::ffi::c_void;
    (*dctx).prefixStart = dict;
    (*dctx).previousDstEnd = (dict as *const ::core::ffi::c_char).offset(dictSize as isize)
        as *const ::core::ffi::c_void;
    return 0 as size_t;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_loadDEntropy(
    mut entropy: *mut ZSTD_entropyDTables_t,
    dict: *const ::core::ffi::c_void,
    dictSize: size_t,
) -> size_t {
    let mut dictPtr: *const BYTE = dict as *const BYTE;
    let dictEnd: *const BYTE = dictPtr.offset(dictSize as isize);
    if dictSize <= 8 as size_t {
        return -(ZSTD_error_dictionary_corrupted as ::core::ffi::c_int) as size_t;
    }
    dictPtr = dictPtr.offset(8 as ::core::ffi::c_int as isize);
    let workspace: *mut ::core::ffi::c_void =
        &raw mut (*entropy).LLTable as *mut ::core::ffi::c_void;
    let workspaceSize: size_t = (::core::mem::size_of::<[ZSTD_seqSymbol; 513]>() as size_t)
        .wrapping_add(::core::mem::size_of::<[ZSTD_seqSymbol; 257]>() as size_t)
        .wrapping_add(::core::mem::size_of::<[ZSTD_seqSymbol; 513]>() as size_t);
    let hSize: size_t = HUF_readDTableX2_wksp(
        &raw mut (*entropy).hufTable as *mut HUF_DTable,
        dictPtr as *const ::core::ffi::c_void,
        dictEnd.offset_from(dictPtr) as ::core::ffi::c_long as size_t,
        workspace,
        workspaceSize,
        0 as ::core::ffi::c_int,
    ) as size_t;
    if ERR_isError(hSize) != 0 {
        return -(ZSTD_error_dictionary_corrupted as ::core::ffi::c_int) as size_t;
    }
    dictPtr = dictPtr.offset(hSize as isize);
    let mut offcodeNCount: [::core::ffi::c_short; 32] = [0; 32];
    let mut offcodeMaxValue: ::core::ffi::c_uint = MaxOff as ::core::ffi::c_uint;
    let mut offcodeLog: ::core::ffi::c_uint = 0;
    let offcodeHeaderSize: size_t = FSE_readNCount(
        &raw mut offcodeNCount as *mut ::core::ffi::c_short,
        &raw mut offcodeMaxValue,
        &raw mut offcodeLog,
        dictPtr as *const ::core::ffi::c_void,
        dictEnd.offset_from(dictPtr) as ::core::ffi::c_long as size_t,
    ) as size_t;
    if ERR_isError(offcodeHeaderSize) != 0 {
        return -(ZSTD_error_dictionary_corrupted as ::core::ffi::c_int) as size_t;
    }
    if offcodeMaxValue > 31 as ::core::ffi::c_uint {
        return -(ZSTD_error_dictionary_corrupted as ::core::ffi::c_int) as size_t;
    }
    if offcodeLog > 8 as ::core::ffi::c_uint {
        return -(ZSTD_error_dictionary_corrupted as ::core::ffi::c_int) as size_t;
    }
    ZSTD_buildFSETable(
        &raw mut (*entropy).OFTable as *mut ZSTD_seqSymbol,
        &raw mut offcodeNCount as *mut ::core::ffi::c_short,
        offcodeMaxValue,
        &raw const OF_base as *const U32,
        &raw const OF_bits as *const U8,
        offcodeLog,
        &raw mut (*entropy).workspace as *mut U32 as *mut ::core::ffi::c_void,
        ::core::mem::size_of::<[U32; 157]>() as size_t,
        0 as ::core::ffi::c_int,
    );
    dictPtr = dictPtr.offset(offcodeHeaderSize as isize);
    let mut matchlengthNCount: [::core::ffi::c_short; 53] = [0; 53];
    let mut matchlengthMaxValue: ::core::ffi::c_uint = MaxML as ::core::ffi::c_uint;
    let mut matchlengthLog: ::core::ffi::c_uint = 0;
    let matchlengthHeaderSize: size_t = FSE_readNCount(
        &raw mut matchlengthNCount as *mut ::core::ffi::c_short,
        &raw mut matchlengthMaxValue,
        &raw mut matchlengthLog,
        dictPtr as *const ::core::ffi::c_void,
        dictEnd.offset_from(dictPtr) as ::core::ffi::c_long as size_t,
    ) as size_t;
    if ERR_isError(matchlengthHeaderSize) != 0 {
        return -(ZSTD_error_dictionary_corrupted as ::core::ffi::c_int) as size_t;
    }
    if matchlengthMaxValue > 52 as ::core::ffi::c_uint {
        return -(ZSTD_error_dictionary_corrupted as ::core::ffi::c_int) as size_t;
    }
    if matchlengthLog > 9 as ::core::ffi::c_uint {
        return -(ZSTD_error_dictionary_corrupted as ::core::ffi::c_int) as size_t;
    }
    ZSTD_buildFSETable(
        &raw mut (*entropy).MLTable as *mut ZSTD_seqSymbol,
        &raw mut matchlengthNCount as *mut ::core::ffi::c_short,
        matchlengthMaxValue,
        &raw const ML_base as *const U32,
        &raw const ML_bits as *const U8,
        matchlengthLog,
        &raw mut (*entropy).workspace as *mut U32 as *mut ::core::ffi::c_void,
        ::core::mem::size_of::<[U32; 157]>() as size_t,
        0 as ::core::ffi::c_int,
    );
    dictPtr = dictPtr.offset(matchlengthHeaderSize as isize);
    let mut litlengthNCount: [::core::ffi::c_short; 36] = [0; 36];
    let mut litlengthMaxValue: ::core::ffi::c_uint = MaxLL as ::core::ffi::c_uint;
    let mut litlengthLog: ::core::ffi::c_uint = 0;
    let litlengthHeaderSize: size_t = FSE_readNCount(
        &raw mut litlengthNCount as *mut ::core::ffi::c_short,
        &raw mut litlengthMaxValue,
        &raw mut litlengthLog,
        dictPtr as *const ::core::ffi::c_void,
        dictEnd.offset_from(dictPtr) as ::core::ffi::c_long as size_t,
    ) as size_t;
    if ERR_isError(litlengthHeaderSize) != 0 {
        return -(ZSTD_error_dictionary_corrupted as ::core::ffi::c_int) as size_t;
    }
    if litlengthMaxValue > 35 as ::core::ffi::c_uint {
        return -(ZSTD_error_dictionary_corrupted as ::core::ffi::c_int) as size_t;
    }
    if litlengthLog > 9 as ::core::ffi::c_uint {
        return -(ZSTD_error_dictionary_corrupted as ::core::ffi::c_int) as size_t;
    }
    ZSTD_buildFSETable(
        &raw mut (*entropy).LLTable as *mut ZSTD_seqSymbol,
        &raw mut litlengthNCount as *mut ::core::ffi::c_short,
        litlengthMaxValue,
        &raw const LL_base as *const U32,
        &raw const LL_bits as *const U8,
        litlengthLog,
        &raw mut (*entropy).workspace as *mut U32 as *mut ::core::ffi::c_void,
        ::core::mem::size_of::<[U32; 157]>() as size_t,
        0 as ::core::ffi::c_int,
    );
    dictPtr = dictPtr.offset(litlengthHeaderSize as isize);
    if dictPtr.offset(12 as ::core::ffi::c_int as isize) > dictEnd {
        return -(ZSTD_error_dictionary_corrupted as ::core::ffi::c_int) as size_t;
    }
    let mut i: ::core::ffi::c_int = 0;
    let dictContentSize: size_t = dictEnd
        .offset_from(dictPtr.offset(12 as ::core::ffi::c_int as isize))
        as ::core::ffi::c_long as size_t;
    i = 0 as ::core::ffi::c_int;
    while i < 3 as ::core::ffi::c_int {
        let rep: U32 = MEM_readLE32(dictPtr as *const ::core::ffi::c_void) as U32;
        dictPtr = dictPtr.offset(4 as ::core::ffi::c_int as isize);
        if rep == 0 as U32 || rep as size_t > dictContentSize {
            return -(ZSTD_error_dictionary_corrupted as ::core::ffi::c_int) as size_t;
        }
        (*entropy).rep[i as usize] = rep;
        i += 1;
    }
    return dictPtr.offset_from(dict as *const BYTE) as ::core::ffi::c_long as size_t;
}
unsafe extern "C" fn ZSTD_decompress_insertDictionary(
    mut dctx: *mut ZSTD_DCtx,
    mut dict: *const ::core::ffi::c_void,
    mut dictSize: size_t,
) -> size_t {
    if dictSize < 8 as size_t {
        return ZSTD_refDictContent(dctx, dict, dictSize);
    }
    let magic: U32 = MEM_readLE32(dict) as U32;
    if magic != ZSTD_MAGIC_DICTIONARY as U32 {
        return ZSTD_refDictContent(dctx, dict, dictSize);
    }
    (*dctx).dictID = MEM_readLE32(
        (dict as *const ::core::ffi::c_char).offset(ZSTD_FRAMEIDSIZE as isize)
            as *const ::core::ffi::c_void,
    );
    let eSize: size_t = ZSTD_loadDEntropy(&raw mut (*dctx).entropy, dict, dictSize) as size_t;
    if ERR_isError(eSize) != 0 {
        return -(ZSTD_error_dictionary_corrupted as ::core::ffi::c_int) as size_t;
    }
    dict =
        (dict as *const ::core::ffi::c_char).offset(eSize as isize) as *const ::core::ffi::c_void;
    dictSize = (dictSize as ::core::ffi::c_ulong).wrapping_sub(eSize as ::core::ffi::c_ulong)
        as size_t as size_t;
    (*dctx).fseEntropy = 1 as U32;
    (*dctx).litEntropy = (*dctx).fseEntropy;
    return ZSTD_refDictContent(dctx, dict, dictSize);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_decompressBegin(mut dctx: *mut ZSTD_DCtx) -> size_t {
    (*dctx).traceCtx = 0 as ZSTD_TraceCtx;
    (*dctx).expected = ZSTD_startingInputLength((*dctx).format);
    (*dctx).stage = ZSTDds_getFrameHeaderSize;
    (*dctx).processedCSize = 0 as U64;
    (*dctx).decodedSize = 0 as U64;
    (*dctx).previousDstEnd = ::core::ptr::null::<::core::ffi::c_void>();
    (*dctx).prefixStart = ::core::ptr::null::<::core::ffi::c_void>();
    (*dctx).virtualStart = ::core::ptr::null::<::core::ffi::c_void>();
    (*dctx).dictEnd = ::core::ptr::null::<::core::ffi::c_void>();
    (*dctx).entropy.hufTable[0 as ::core::ffi::c_int as usize] =
        (12 as ::core::ffi::c_int * 0x1000001 as ::core::ffi::c_int) as HUF_DTable;
    (*dctx).fseEntropy = 0 as U32;
    (*dctx).litEntropy = (*dctx).fseEntropy;
    (*dctx).dictID = 0 as U32;
    (*dctx).bType = bt_reserved;
    (*dctx).isFrameDecompression = 1 as ::core::ffi::c_int;
    ::libc::memcpy(
        &raw mut (*dctx).entropy.rep as *mut U32 as *mut ::core::ffi::c_void,
        &raw const repStartValue as *const U32 as *const ::core::ffi::c_void,
        ::core::mem::size_of::<[U32; 3]>() as ::libc::size_t,
    );
    (*dctx).LLTptr = &raw mut (*dctx).entropy.LLTable as *mut ZSTD_seqSymbol;
    (*dctx).MLTptr = &raw mut (*dctx).entropy.MLTable as *mut ZSTD_seqSymbol;
    (*dctx).OFTptr = &raw mut (*dctx).entropy.OFTable as *mut ZSTD_seqSymbol;
    (*dctx).HUFptr = &raw mut (*dctx).entropy.hufTable as *mut HUF_DTable;
    return 0 as size_t;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_decompressBegin_usingDict(
    mut dctx: *mut ZSTD_DCtx,
    mut dict: *const ::core::ffi::c_void,
    mut dictSize: size_t,
) -> size_t {
    let err_code: size_t = ZSTD_decompressBegin(dctx) as size_t;
    if ERR_isError(err_code) != 0 {
        return err_code;
    }
    if !dict.is_null() && dictSize != 0 {
        if ERR_isError(ZSTD_decompress_insertDictionary(dctx, dict, dictSize)) != 0 {
            return -(ZSTD_error_dictionary_corrupted as ::core::ffi::c_int) as size_t;
        }
    }
    return 0 as size_t;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_decompressBegin_usingDDict(
    mut dctx: *mut ZSTD_DCtx,
    mut ddict: *const ZSTD_DDict,
) -> size_t {
    if !ddict.is_null() {
        let dictStart: *const ::core::ffi::c_char =
            ZSTD_DDict_dictContent(ddict) as *const ::core::ffi::c_char;
        let dictSize: size_t = ZSTD_DDict_dictSize(ddict) as size_t;
        let dictEnd: *const ::core::ffi::c_void =
            dictStart.offset(dictSize as isize) as *const ::core::ffi::c_void;
        (*dctx).ddictIsCold = ((*dctx).dictEnd != dictEnd) as ::core::ffi::c_int;
    }
    let err_code: size_t = ZSTD_decompressBegin(dctx) as size_t;
    if ERR_isError(err_code) != 0 {
        return err_code;
    }
    if !ddict.is_null() {
        ZSTD_copyDDictParameters(dctx, ddict);
    }
    return 0 as size_t;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_getDictID_fromDict(
    mut dict: *const ::core::ffi::c_void,
    mut dictSize: size_t,
) -> ::core::ffi::c_uint {
    if dictSize < 8 as size_t {
        return 0 as ::core::ffi::c_uint;
    }
    if MEM_readLE32(dict) != ZSTD_MAGIC_DICTIONARY as U32 {
        return 0 as ::core::ffi::c_uint;
    }
    return MEM_readLE32(
        (dict as *const ::core::ffi::c_char).offset(ZSTD_FRAMEIDSIZE as isize)
            as *const ::core::ffi::c_void,
    ) as ::core::ffi::c_uint;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_getDictID_fromFrame(
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> ::core::ffi::c_uint {
    let mut zfp: ZSTD_FrameHeader = ZSTD_FrameHeader {
        frameContentSize: 0 as ::core::ffi::c_ulonglong,
        windowSize: 0 as ::core::ffi::c_ulonglong,
        blockSizeMax: 0 as ::core::ffi::c_uint,
        frameType: ZSTD_frame,
        headerSize: 0 as ::core::ffi::c_uint,
        dictID: 0 as ::core::ffi::c_uint,
        checksumFlag: 0 as ::core::ffi::c_uint,
        _reserved1: 0 as ::core::ffi::c_uint,
        _reserved2: 0 as ::core::ffi::c_uint,
    };
    let hError: size_t = ZSTD_getFrameHeader(&raw mut zfp, src, srcSize) as size_t;
    if ERR_isError(hError) != 0 {
        return 0 as ::core::ffi::c_uint;
    }
    return zfp.dictID;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_decompress_usingDDict(
    mut dctx: *mut ZSTD_DCtx,
    mut dst: *mut ::core::ffi::c_void,
    mut dstCapacity: size_t,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
    mut ddict: *const ZSTD_DDict,
) -> size_t {
    return ZSTD_decompressMultiFrame(
        dctx,
        dst,
        dstCapacity,
        src,
        srcSize,
        ::core::ptr::null::<::core::ffi::c_void>(),
        0 as size_t,
        ddict,
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_createDStream() -> *mut ZSTD_DStream {
    return ZSTD_createDCtx_internal(ZSTD_defaultCMem) as *mut ZSTD_DStream;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_initStaticDStream(
    mut workspace: *mut ::core::ffi::c_void,
    mut workspaceSize: size_t,
) -> *mut ZSTD_DStream {
    return ZSTD_initStaticDCtx(workspace, workspaceSize) as *mut ZSTD_DStream;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_createDStream_advanced(
    mut customMem: ZSTD_customMem,
) -> *mut ZSTD_DStream {
    return ZSTD_createDCtx_internal(customMem) as *mut ZSTD_DStream;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_freeDStream(mut zds: *mut ZSTD_DStream) -> size_t {
    return ZSTD_freeDCtx(zds as *mut ZSTD_DCtx);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_DStreamInSize() -> size_t {
    return (ZSTD_BLOCKSIZE_MAX as size_t).wrapping_add(ZSTD_blockHeaderSize);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_DStreamOutSize() -> size_t {
    return ZSTD_BLOCKSIZE_MAX as size_t;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_DCtx_loadDictionary_advanced(
    mut dctx: *mut ZSTD_DCtx,
    mut dict: *const ::core::ffi::c_void,
    mut dictSize: size_t,
    mut dictLoadMethod: ZSTD_dictLoadMethod_e,
    mut dictContentType: ZSTD_dictContentType_e,
) -> size_t {
    if (*dctx).streamStage as ::core::ffi::c_uint
        != zdss_init as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        return -(ZSTD_error_stage_wrong as ::core::ffi::c_int) as size_t;
    }
    ZSTD_clearDict(dctx);
    if !dict.is_null() && dictSize != 0 as size_t {
        (*dctx).ddictLocal = ZSTD_createDDict_advanced(
            dict,
            dictSize,
            dictLoadMethod,
            dictContentType,
            (*dctx).customMem,
        );
        if (*dctx).ddictLocal.is_null() {
            return -(ZSTD_error_memory_allocation as ::core::ffi::c_int) as size_t;
        }
        (*dctx).ddict = (*dctx).ddictLocal;
        (*dctx).dictUses = ZSTD_use_indefinitely;
    }
    return 0 as size_t;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_DCtx_loadDictionary_byReference(
    mut dctx: *mut ZSTD_DCtx,
    mut dict: *const ::core::ffi::c_void,
    mut dictSize: size_t,
) -> size_t {
    return ZSTD_DCtx_loadDictionary_advanced(dctx, dict, dictSize, ZSTD_dlm_byRef, ZSTD_dct_auto);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_DCtx_loadDictionary(
    mut dctx: *mut ZSTD_DCtx,
    mut dict: *const ::core::ffi::c_void,
    mut dictSize: size_t,
) -> size_t {
    return ZSTD_DCtx_loadDictionary_advanced(dctx, dict, dictSize, ZSTD_dlm_byCopy, ZSTD_dct_auto);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_DCtx_refPrefix_advanced(
    mut dctx: *mut ZSTD_DCtx,
    mut prefix: *const ::core::ffi::c_void,
    mut prefixSize: size_t,
    mut dictContentType: ZSTD_dictContentType_e,
) -> size_t {
    let err_code: size_t = ZSTD_DCtx_loadDictionary_advanced(
        dctx,
        prefix,
        prefixSize,
        ZSTD_dlm_byRef,
        dictContentType,
    ) as size_t;
    if ERR_isError(err_code) != 0 {
        return err_code;
    }
    (*dctx).dictUses = ZSTD_use_once;
    return 0 as size_t;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_DCtx_refPrefix(
    mut dctx: *mut ZSTD_DCtx,
    mut prefix: *const ::core::ffi::c_void,
    mut prefixSize: size_t,
) -> size_t {
    return ZSTD_DCtx_refPrefix_advanced(dctx, prefix, prefixSize, ZSTD_dct_rawContent);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_initDStream_usingDict(
    mut zds: *mut ZSTD_DStream,
    mut dict: *const ::core::ffi::c_void,
    mut dictSize: size_t,
) -> size_t {
    let err_code: size_t =
        ZSTD_DCtx_reset(zds as *mut ZSTD_DCtx, ZSTD_reset_session_only) as size_t;
    if ERR_isError(err_code) != 0 {
        return err_code;
    }
    let err_code_0: size_t =
        ZSTD_DCtx_loadDictionary(zds as *mut ZSTD_DCtx, dict, dictSize) as size_t;
    if ERR_isError(err_code_0) != 0 {
        return err_code_0;
    }
    return ZSTD_startingInputLength((*zds).format);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_initDStream(mut zds: *mut ZSTD_DStream) -> size_t {
    let err_code: size_t =
        ZSTD_DCtx_reset(zds as *mut ZSTD_DCtx, ZSTD_reset_session_only) as size_t;
    if ERR_isError(err_code) != 0 {
        return err_code;
    }
    let err_code_0: size_t =
        ZSTD_DCtx_refDDict(zds as *mut ZSTD_DCtx, ::core::ptr::null::<ZSTD_DDict>()) as size_t;
    if ERR_isError(err_code_0) != 0 {
        return err_code_0;
    }
    return ZSTD_startingInputLength((*zds).format);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_initDStream_usingDDict(
    mut dctx: *mut ZSTD_DStream,
    mut ddict: *const ZSTD_DDict,
) -> size_t {
    let err_code: size_t =
        ZSTD_DCtx_reset(dctx as *mut ZSTD_DCtx, ZSTD_reset_session_only) as size_t;
    if ERR_isError(err_code) != 0 {
        return err_code;
    }
    let err_code_0: size_t = ZSTD_DCtx_refDDict(dctx as *mut ZSTD_DCtx, ddict) as size_t;
    if ERR_isError(err_code_0) != 0 {
        return err_code_0;
    }
    return ZSTD_startingInputLength((*dctx).format);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_resetDStream(mut dctx: *mut ZSTD_DStream) -> size_t {
    let err_code: size_t =
        ZSTD_DCtx_reset(dctx as *mut ZSTD_DCtx, ZSTD_reset_session_only) as size_t;
    if ERR_isError(err_code) != 0 {
        return err_code;
    }
    return ZSTD_startingInputLength((*dctx).format);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_DCtx_refDDict(
    mut dctx: *mut ZSTD_DCtx,
    mut ddict: *const ZSTD_DDict,
) -> size_t {
    if (*dctx).streamStage as ::core::ffi::c_uint
        != zdss_init as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        return -(ZSTD_error_stage_wrong as ::core::ffi::c_int) as size_t;
    }
    ZSTD_clearDict(dctx);
    if !ddict.is_null() {
        (*dctx).ddict = ddict;
        (*dctx).dictUses = ZSTD_use_indefinitely;
        if (*dctx).refMultipleDDicts as ::core::ffi::c_uint
            == ZSTD_rmd_refMultipleDDicts as ::core::ffi::c_int as ::core::ffi::c_uint
        {
            if (*dctx).ddictSet.is_null() {
                (*dctx).ddictSet = ZSTD_createDDictHashSet((*dctx).customMem);
                if (*dctx).ddictSet.is_null() {
                    return -(ZSTD_error_memory_allocation as ::core::ffi::c_int) as size_t;
                }
            }
            let err_code: size_t =
                ZSTD_DDictHashSet_addDDict((*dctx).ddictSet, ddict, (*dctx).customMem) as size_t;
            if ERR_isError(err_code) != 0 {
                return err_code;
            }
        }
    }
    return 0 as size_t;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_DCtx_setMaxWindowSize(
    mut dctx: *mut ZSTD_DCtx,
    mut maxWindowSize: size_t,
) -> size_t {
    let bounds: ZSTD_bounds = ZSTD_dParam_getBounds(ZSTD_d_windowLogMax) as ZSTD_bounds;
    let min: size_t = (1 as ::core::ffi::c_int as size_t) << bounds.lowerBound;
    let max: size_t = (1 as ::core::ffi::c_int as size_t) << bounds.upperBound;
    if (*dctx).streamStage as ::core::ffi::c_uint
        != zdss_init as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        return -(ZSTD_error_stage_wrong as ::core::ffi::c_int) as size_t;
    }
    if maxWindowSize < min {
        return -(ZSTD_error_parameter_outOfBound as ::core::ffi::c_int) as size_t;
    }
    if maxWindowSize > max {
        return -(ZSTD_error_parameter_outOfBound as ::core::ffi::c_int) as size_t;
    }
    (*dctx).maxWindowSize = maxWindowSize;
    return 0 as size_t;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_DCtx_setFormat(
    mut dctx: *mut ZSTD_DCtx,
    mut format: ZSTD_format_e,
) -> size_t {
    return ZSTD_DCtx_setParameter(
        dctx,
        ZSTD_d_experimentalParam1,
        format as ::core::ffi::c_int,
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_dParam_getBounds(mut dParam: ZSTD_dParameter) -> ZSTD_bounds {
    let mut bounds: ZSTD_bounds = ZSTD_bounds {
        error: 0 as size_t,
        lowerBound: 0 as ::core::ffi::c_int,
        upperBound: 0 as ::core::ffi::c_int,
    };
    match dParam as ::core::ffi::c_uint {
        100 => {
            bounds.lowerBound = ZSTD_WINDOWLOG_ABSOLUTEMIN;
            bounds.upperBound = if ::core::mem::size_of::<size_t>() as usize == 4 as usize {
                ZSTD_WINDOWLOG_MAX_32
            } else {
                ZSTD_WINDOWLOG_MAX_64
            };
            return bounds;
        }
        1000 => {
            bounds.lowerBound = ZSTD_f_zstd1 as ::core::ffi::c_int;
            bounds.upperBound = ZSTD_f_zstd1_magicless as ::core::ffi::c_int;
            return bounds;
        }
        1001 => {
            bounds.lowerBound = ZSTD_bm_buffered as ::core::ffi::c_int;
            bounds.upperBound = ZSTD_bm_stable as ::core::ffi::c_int;
            return bounds;
        }
        1002 => {
            bounds.lowerBound = ZSTD_d_validateChecksum as ::core::ffi::c_int;
            bounds.upperBound = ZSTD_d_ignoreChecksum as ::core::ffi::c_int;
            return bounds;
        }
        1003 => {
            bounds.lowerBound = ZSTD_rmd_refSingleDDict as ::core::ffi::c_int;
            bounds.upperBound = ZSTD_rmd_refMultipleDDicts as ::core::ffi::c_int;
            return bounds;
        }
        1004 => {
            bounds.lowerBound = 0 as ::core::ffi::c_int;
            bounds.upperBound = 1 as ::core::ffi::c_int;
            return bounds;
        }
        1005 => {
            bounds.lowerBound = ZSTD_BLOCKSIZE_MAX_MIN;
            bounds.upperBound = ZSTD_BLOCKSIZE_MAX;
            return bounds;
        }
        _ => {}
    }
    bounds.error = -(ZSTD_error_parameter_unsupported as ::core::ffi::c_int) as size_t;
    return bounds;
}
unsafe extern "C" fn ZSTD_dParam_withinBounds(
    mut dParam: ZSTD_dParameter,
    mut value: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let bounds: ZSTD_bounds = ZSTD_dParam_getBounds(dParam) as ZSTD_bounds;
    if ERR_isError(bounds.error) != 0 {
        return 0 as ::core::ffi::c_int;
    }
    if value < bounds.lowerBound {
        return 0 as ::core::ffi::c_int;
    }
    if value > bounds.upperBound {
        return 0 as ::core::ffi::c_int;
    }
    return 1 as ::core::ffi::c_int;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_DCtx_getParameter(
    mut dctx: *mut ZSTD_DCtx,
    mut param: ZSTD_dParameter,
    mut value: *mut ::core::ffi::c_int,
) -> size_t {
    match param as ::core::ffi::c_uint {
        100 => {
            *value = ZSTD_highbit32((*dctx).maxWindowSize as U32) as ::core::ffi::c_int;
            return 0 as size_t;
        }
        1000 => {
            *value = (*dctx).format as ::core::ffi::c_int;
            return 0 as size_t;
        }
        1001 => {
            *value = (*dctx).outBufferMode as ::core::ffi::c_int;
            return 0 as size_t;
        }
        1002 => {
            *value = (*dctx).forceIgnoreChecksum as ::core::ffi::c_int;
            return 0 as size_t;
        }
        1003 => {
            *value = (*dctx).refMultipleDDicts as ::core::ffi::c_int;
            return 0 as size_t;
        }
        1004 => {
            *value = (*dctx).disableHufAsm;
            return 0 as size_t;
        }
        1005 => {
            *value = (*dctx).maxBlockSizeParam;
            return 0 as size_t;
        }
        _ => {}
    }
    return -(ZSTD_error_parameter_unsupported as ::core::ffi::c_int) as size_t;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_DCtx_setParameter(
    mut dctx: *mut ZSTD_DCtx,
    mut dParam: ZSTD_dParameter,
    mut value: ::core::ffi::c_int,
) -> size_t {
    if (*dctx).streamStage as ::core::ffi::c_uint
        != zdss_init as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        return -(ZSTD_error_stage_wrong as ::core::ffi::c_int) as size_t;
    }
    match dParam as ::core::ffi::c_uint {
        100 => {
            if value == 0 as ::core::ffi::c_int {
                value = ZSTD_WINDOWLOG_LIMIT_DEFAULT;
            }
            if ZSTD_dParam_withinBounds(ZSTD_d_windowLogMax, value) == 0 {
                return -(ZSTD_error_parameter_outOfBound as ::core::ffi::c_int) as size_t;
            }
            (*dctx).maxWindowSize = (1 as ::core::ffi::c_int as size_t) << value;
            return 0 as size_t;
        }
        1000 => {
            if ZSTD_dParam_withinBounds(ZSTD_d_experimentalParam1, value) == 0 {
                return -(ZSTD_error_parameter_outOfBound as ::core::ffi::c_int) as size_t;
            }
            (*dctx).format = value as ZSTD_format_e;
            return 0 as size_t;
        }
        1001 => {
            if ZSTD_dParam_withinBounds(ZSTD_d_experimentalParam2, value) == 0 {
                return -(ZSTD_error_parameter_outOfBound as ::core::ffi::c_int) as size_t;
            }
            (*dctx).outBufferMode = value as ZSTD_bufferMode_e;
            return 0 as size_t;
        }
        1002 => {
            if ZSTD_dParam_withinBounds(ZSTD_d_experimentalParam3, value) == 0 {
                return -(ZSTD_error_parameter_outOfBound as ::core::ffi::c_int) as size_t;
            }
            (*dctx).forceIgnoreChecksum = value as ZSTD_forceIgnoreChecksum_e;
            return 0 as size_t;
        }
        1003 => {
            if ZSTD_dParam_withinBounds(ZSTD_d_experimentalParam4, value) == 0 {
                return -(ZSTD_error_parameter_outOfBound as ::core::ffi::c_int) as size_t;
            }
            if (*dctx).staticSize != 0 as size_t {
                return -(ZSTD_error_parameter_unsupported as ::core::ffi::c_int) as size_t;
            }
            (*dctx).refMultipleDDicts = value as ZSTD_refMultipleDDicts_e;
            return 0 as size_t;
        }
        1004 => {
            if ZSTD_dParam_withinBounds(ZSTD_d_experimentalParam5, value) == 0 {
                return -(ZSTD_error_parameter_outOfBound as ::core::ffi::c_int) as size_t;
            }
            (*dctx).disableHufAsm = (value != 0 as ::core::ffi::c_int) as ::core::ffi::c_int;
            return 0 as size_t;
        }
        1005 => {
            if value != 0 as ::core::ffi::c_int {
                if ZSTD_dParam_withinBounds(ZSTD_d_experimentalParam6, value) == 0 {
                    return -(ZSTD_error_parameter_outOfBound as ::core::ffi::c_int) as size_t;
                }
            }
            (*dctx).maxBlockSizeParam = value;
            return 0 as size_t;
        }
        _ => {}
    }
    return -(ZSTD_error_parameter_unsupported as ::core::ffi::c_int) as size_t;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_DCtx_reset(
    mut dctx: *mut ZSTD_DCtx,
    mut reset: ZSTD_ResetDirective,
) -> size_t {
    if reset as ::core::ffi::c_uint
        == ZSTD_reset_session_only as ::core::ffi::c_int as ::core::ffi::c_uint
        || reset as ::core::ffi::c_uint
            == ZSTD_reset_session_and_parameters as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        (*dctx).streamStage = zdss_init;
        (*dctx).noForwardProgress = 0 as ::core::ffi::c_int;
        (*dctx).isFrameDecompression = 1 as ::core::ffi::c_int;
    }
    if reset as ::core::ffi::c_uint
        == ZSTD_reset_parameters as ::core::ffi::c_int as ::core::ffi::c_uint
        || reset as ::core::ffi::c_uint
            == ZSTD_reset_session_and_parameters as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        if (*dctx).streamStage as ::core::ffi::c_uint
            != zdss_init as ::core::ffi::c_int as ::core::ffi::c_uint
        {
            return -(ZSTD_error_stage_wrong as ::core::ffi::c_int) as size_t;
        }
        ZSTD_clearDict(dctx);
        ZSTD_DCtx_resetParameters(dctx);
    }
    return 0 as size_t;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_sizeof_DStream(mut dctx: *const ZSTD_DStream) -> size_t {
    return ZSTD_sizeof_DCtx(dctx as *const ZSTD_DCtx);
}
unsafe extern "C" fn ZSTD_decodingBufferSize_internal(
    mut windowSize: ::core::ffi::c_ulonglong,
    mut frameContentSize: ::core::ffi::c_ulonglong,
    mut blockSizeMax: size_t,
) -> size_t {
    let blockSize: size_t = if ((if windowSize
        < ((1 as ::core::ffi::c_int) << 17 as ::core::ffi::c_int) as ::core::ffi::c_ulonglong
    {
        windowSize
    } else {
        ((1 as ::core::ffi::c_int) << 17 as ::core::ffi::c_int) as ::core::ffi::c_ulonglong
    }) as size_t)
        < blockSizeMax
    {
        (if windowSize
            < ((1 as ::core::ffi::c_int) << 17 as ::core::ffi::c_int) as ::core::ffi::c_ulonglong
        {
            windowSize
        } else {
            ((1 as ::core::ffi::c_int) << 17 as ::core::ffi::c_int) as ::core::ffi::c_ulonglong
        }) as size_t
    } else {
        blockSizeMax
    };
    let neededRBSize: ::core::ffi::c_ulonglong = windowSize
        .wrapping_add(blockSize.wrapping_mul(2 as size_t) as ::core::ffi::c_ulonglong)
        .wrapping_add((WILDCOPY_OVERLENGTH * 2 as ::core::ffi::c_int) as ::core::ffi::c_ulonglong);
    let neededSize: ::core::ffi::c_ulonglong = if frameContentSize < neededRBSize {
        frameContentSize
    } else {
        neededRBSize
    };
    let minRBSize: size_t = neededSize as size_t;
    if minRBSize as ::core::ffi::c_ulonglong != neededSize {
        return -(ZSTD_error_frameParameter_windowTooLarge as ::core::ffi::c_int) as size_t;
    }
    return minRBSize;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_decodingBufferSize_min(
    mut windowSize: ::core::ffi::c_ulonglong,
    mut frameContentSize: ::core::ffi::c_ulonglong,
) -> size_t {
    return ZSTD_decodingBufferSize_internal(
        windowSize,
        frameContentSize,
        ZSTD_BLOCKSIZE_MAX as size_t,
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_estimateDStreamSize(mut windowSize: size_t) -> size_t {
    let blockSize: size_t =
        if windowSize < ((1 as ::core::ffi::c_int) << 17 as ::core::ffi::c_int) as size_t {
            windowSize
        } else {
            ((1 as ::core::ffi::c_int) << 17 as ::core::ffi::c_int) as size_t
        };
    let inBuffSize: size_t = blockSize;
    let outBuffSize: size_t = ZSTD_decodingBufferSize_min(
        windowSize as ::core::ffi::c_ulonglong,
        ZSTD_CONTENTSIZE_UNKNOWN,
    ) as size_t;
    return ZSTD_estimateDCtxSize()
        .wrapping_add(inBuffSize)
        .wrapping_add(outBuffSize);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_estimateDStreamSize_fromFrame(
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    let windowSizeMax: U32 = (1 as U32)
        << (if ::core::mem::size_of::<size_t>() as usize == 4 as usize {
            ZSTD_WINDOWLOG_MAX_32
        } else {
            ZSTD_WINDOWLOG_MAX_64
        });
    let mut zfh: ZSTD_FrameHeader = ZSTD_FrameHeader {
        frameContentSize: 0,
        windowSize: 0,
        blockSizeMax: 0,
        frameType: ZSTD_frame,
        headerSize: 0,
        dictID: 0,
        checksumFlag: 0,
        _reserved1: 0,
        _reserved2: 0,
    };
    let err: size_t = ZSTD_getFrameHeader(&raw mut zfh, src, srcSize) as size_t;
    if ERR_isError(err) != 0 {
        return err;
    }
    if err > 0 as size_t {
        return -(ZSTD_error_srcSize_wrong as ::core::ffi::c_int) as size_t;
    }
    if zfh.windowSize > windowSizeMax as ::core::ffi::c_ulonglong {
        return -(ZSTD_error_frameParameter_windowTooLarge as ::core::ffi::c_int) as size_t;
    }
    return ZSTD_estimateDStreamSize(zfh.windowSize as size_t);
}
unsafe extern "C" fn ZSTD_DCtx_isOverflow(
    mut zds: *mut ZSTD_DStream,
    neededInBuffSize: size_t,
    neededOutBuffSize: size_t,
) -> ::core::ffi::c_int {
    return ((*zds).inBuffSize.wrapping_add((*zds).outBuffSize)
        >= neededInBuffSize
            .wrapping_add(neededOutBuffSize)
            .wrapping_mul(ZSTD_WORKSPACETOOLARGE_FACTOR as size_t))
        as ::core::ffi::c_int;
}
unsafe extern "C" fn ZSTD_DCtx_updateOversizedDuration(
    mut zds: *mut ZSTD_DStream,
    neededInBuffSize: size_t,
    neededOutBuffSize: size_t,
) {
    if ZSTD_DCtx_isOverflow(zds, neededInBuffSize, neededOutBuffSize) != 0 {
        (*zds).oversizedDuration = (*zds).oversizedDuration.wrapping_add(1);
    } else {
        (*zds).oversizedDuration = 0 as size_t;
    };
}
unsafe extern "C" fn ZSTD_DCtx_isOversizedTooLong(
    mut zds: *mut ZSTD_DStream,
) -> ::core::ffi::c_int {
    return ((*zds).oversizedDuration >= ZSTD_WORKSPACETOOLARGE_MAXDURATION as size_t)
        as ::core::ffi::c_int;
}
unsafe extern "C" fn ZSTD_checkOutBuffer(
    mut zds: *const ZSTD_DStream,
    mut output: *const ZSTD_outBuffer,
) -> size_t {
    let expect: ZSTD_outBuffer = (*zds).expectedOutBuffer;
    if (*zds).outBufferMode as ::core::ffi::c_uint
        != ZSTD_bm_stable as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        return 0 as size_t;
    }
    if (*zds).streamStage as ::core::ffi::c_uint
        == zdss_init as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        return 0 as size_t;
    }
    if expect.dst == (*output).dst && expect.pos == (*output).pos && expect.size == (*output).size {
        return 0 as size_t;
    }
    return -(ZSTD_error_dstBuffer_wrong as ::core::ffi::c_int) as size_t;
}
unsafe extern "C" fn ZSTD_decompressContinueStream(
    mut zds: *mut ZSTD_DStream,
    mut op: *mut *mut ::core::ffi::c_char,
    mut oend: *mut ::core::ffi::c_char,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
) -> size_t {
    let isSkipFrame: ::core::ffi::c_int =
        ZSTD_isSkipFrame(zds as *mut ZSTD_DCtx) as ::core::ffi::c_int;
    if (*zds).outBufferMode as ::core::ffi::c_uint
        == ZSTD_bm_buffered as ::core::ffi::c_int as ::core::ffi::c_uint
    {
        let dstSize: size_t = if isSkipFrame != 0 {
            0 as size_t
        } else {
            (*zds).outBuffSize.wrapping_sub((*zds).outStart)
        };
        let decodedSize: size_t = ZSTD_decompressContinue(
            zds as *mut ZSTD_DCtx,
            (*zds).outBuff.offset((*zds).outStart as isize) as *mut ::core::ffi::c_void,
            dstSize,
            src,
            srcSize,
        ) as size_t;
        let err_code: size_t = decodedSize;
        if ERR_isError(err_code) != 0 {
            return err_code;
        }
        if decodedSize == 0 && isSkipFrame == 0 {
            (*zds).streamStage = zdss_read;
        } else {
            (*zds).outEnd = (*zds).outStart.wrapping_add(decodedSize);
            (*zds).streamStage = zdss_flush;
        }
    } else {
        let dstSize_0: size_t = if isSkipFrame != 0 {
            0 as size_t
        } else {
            oend.offset_from(*op) as ::core::ffi::c_long as size_t
        };
        let decodedSize_0: size_t = ZSTD_decompressContinue(
            zds as *mut ZSTD_DCtx,
            *op as *mut ::core::ffi::c_void,
            dstSize_0,
            src,
            srcSize,
        ) as size_t;
        let err_code_0: size_t = decodedSize_0;
        if ERR_isError(err_code_0) != 0 {
            return err_code_0;
        }
        *op = (*op).offset(decodedSize_0 as isize);
        (*zds).streamStage = zdss_read;
    }
    return 0 as size_t;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_decompressStream(
    mut zds: *mut ZSTD_DStream,
    mut output: *mut ZSTD_outBuffer,
    mut input: *mut ZSTD_inBuffer,
) -> size_t {
    let src: *const ::core::ffi::c_char = (*input).src as *const ::core::ffi::c_char;
    let istart: *const ::core::ffi::c_char = if (*input).pos != 0 as size_t {
        src.offset((*input).pos as isize)
    } else {
        src
    };
    let iend: *const ::core::ffi::c_char = if (*input).size != 0 as size_t {
        src.offset((*input).size as isize)
    } else {
        src
    };
    let mut ip: *const ::core::ffi::c_char = istart;
    let dst: *mut ::core::ffi::c_char = (*output).dst as *mut ::core::ffi::c_char;
    let ostart: *mut ::core::ffi::c_char = if (*output).pos != 0 as size_t {
        dst.offset((*output).pos as isize)
    } else {
        dst
    };
    let oend: *mut ::core::ffi::c_char = if (*output).size != 0 as size_t {
        dst.offset((*output).size as isize)
    } else {
        dst
    };
    let mut op: *mut ::core::ffi::c_char = ostart;
    let mut someMoreWork: U32 = 1 as U32;
    if (*input).pos > (*input).size {
        return -(ZSTD_error_srcSize_wrong as ::core::ffi::c_int) as size_t;
    }
    if (*output).pos > (*output).size {
        return -(ZSTD_error_dstSize_tooSmall as ::core::ffi::c_int) as size_t;
    }
    let err_code: size_t = ZSTD_checkOutBuffer(zds, output) as size_t;
    if ERR_isError(err_code) != 0 {
        return err_code;
    }
    while someMoreWork != 0 {
        let mut current_block_402: u64;
        match (*zds).streamStage as ::core::ffi::c_uint {
            0 => {
                (*zds).streamStage = zdss_loadHeader;
                (*zds).outEnd = 0 as size_t;
                (*zds).outStart = (*zds).outEnd;
                (*zds).inPos = (*zds).outStart;
                (*zds).lhSize = (*zds).inPos;
                (*zds).legacyVersion = 0 as U32;
                (*zds).hostageByte = 0 as U32;
                (*zds).expectedOutBuffer = *output;
                current_block_402 = 1623252117315916725;
            }
            1 => {
                current_block_402 = 1623252117315916725;
            }
            2 => {
                current_block_402 = 7991679940794782184;
            }
            3 => {
                current_block_402 = 18111739650402451604;
            }
            4 => {
                let toFlushSize: size_t = (*zds).outEnd.wrapping_sub((*zds).outStart);
                let flushedSize: size_t = ZSTD_limitCopy(
                    op as *mut ::core::ffi::c_void,
                    oend.offset_from(op) as ::core::ffi::c_long as size_t,
                    (*zds).outBuff.offset((*zds).outStart as isize) as *const ::core::ffi::c_void,
                    toFlushSize,
                ) as size_t;
                op = if !op.is_null() {
                    op.offset(flushedSize as isize)
                } else {
                    op
                };
                (*zds).outStart = ((*zds).outStart as ::core::ffi::c_ulong)
                    .wrapping_add(flushedSize as ::core::ffi::c_ulong)
                    as size_t as size_t;
                if flushedSize == toFlushSize {
                    (*zds).streamStage = zdss_read;
                    if ((*zds).outBuffSize as ::core::ffi::c_ulonglong)
                        < (*zds).fParams.frameContentSize
                        && (*zds)
                            .outStart
                            .wrapping_add((*zds).fParams.blockSizeMax as size_t)
                            > (*zds).outBuffSize
                    {
                        (*zds).outEnd = 0 as size_t;
                        (*zds).outStart = (*zds).outEnd;
                    }
                } else {
                    someMoreWork = 0 as U32;
                }
                current_block_402 = 7792909578691485565;
            }
            _ => return -(ZSTD_error_GENERIC as ::core::ffi::c_int) as size_t,
        }
        match current_block_402 {
            1623252117315916725 => {
                if (*zds).legacyVersion != 0 {
                    if (*zds).staticSize != 0 {
                        return -(ZSTD_error_memory_allocation as ::core::ffi::c_int) as size_t;
                    }
                    let hint: size_t = ZSTD_decompressLegacyStream(
                        (*zds).legacyContext,
                        (*zds).legacyVersion,
                        output,
                        input,
                    ) as size_t;
                    if hint == 0 as size_t {
                        (*zds).streamStage = zdss_init;
                    }
                    return hint;
                }
                let hSize: size_t = ZSTD_getFrameHeader_advanced(
                    &raw mut (*zds).fParams,
                    &raw mut (*zds).headerBuffer as *mut BYTE as *const ::core::ffi::c_void,
                    (*zds).lhSize,
                    (*zds).format,
                ) as size_t;
                if (*zds).refMultipleDDicts as ::core::ffi::c_uint != 0
                    && !(*zds).ddictSet.is_null()
                {
                    ZSTD_DCtx_selectFrameDDict(zds as *mut ZSTD_DCtx);
                }
                if ERR_isError(hSize) != 0 {
                    let legacyVersion: U32 = ZSTD_isLegacy(
                        istart as *const ::core::ffi::c_void,
                        iend.offset_from(istart) as ::core::ffi::c_long as size_t,
                    ) as U32;
                    if legacyVersion != 0 {
                        let ddict: *const ZSTD_DDict =
                            ZSTD_getDDict(zds as *mut ZSTD_DCtx) as *const ZSTD_DDict;
                        let dict: *const ::core::ffi::c_void = if !ddict.is_null() {
                            ZSTD_DDict_dictContent(ddict) as *const ::core::ffi::c_void
                        } else {
                            ::core::ptr::null::<::core::ffi::c_void>()
                        };
                        let dictSize: size_t = if !ddict.is_null() {
                            ZSTD_DDict_dictSize(ddict) as size_t
                        } else {
                            0 as size_t
                        };
                        if (*zds).staticSize != 0 {
                            return -(ZSTD_error_memory_allocation as ::core::ffi::c_int) as size_t;
                        }
                        let err_code_0: size_t = ZSTD_initLegacyStream(
                            &raw mut (*zds).legacyContext,
                            (*zds).previousLegacyVersion,
                            legacyVersion,
                            dict,
                            dictSize,
                        ) as size_t;
                        if ERR_isError(err_code_0) != 0 {
                            return err_code_0;
                        }
                        (*zds).previousLegacyVersion = legacyVersion;
                        (*zds).legacyVersion = (*zds).previousLegacyVersion;
                        let hint_0: size_t = ZSTD_decompressLegacyStream(
                            (*zds).legacyContext,
                            legacyVersion,
                            output,
                            input,
                        ) as size_t;
                        if hint_0 == 0 as size_t {
                            (*zds).streamStage = zdss_init;
                        }
                        return hint_0;
                    }
                    return hSize;
                }
                if hSize != 0 as size_t {
                    let toLoad: size_t = hSize.wrapping_sub((*zds).lhSize);
                    let remainingInput: size_t =
                        iend.offset_from(ip) as ::core::ffi::c_long as size_t;
                    if toLoad > remainingInput {
                        if remainingInput > 0 as size_t {
                            ::libc::memcpy(
                                (&raw mut (*zds).headerBuffer as *mut BYTE)
                                    .offset((*zds).lhSize as isize)
                                    as *mut ::core::ffi::c_void,
                                ip as *const ::core::ffi::c_void,
                                remainingInput as ::libc::size_t,
                            );
                            (*zds).lhSize = ((*zds).lhSize as ::core::ffi::c_ulong)
                                .wrapping_add(remainingInput as ::core::ffi::c_ulong)
                                as size_t as size_t;
                        }
                        (*input).pos = (*input).size;
                        let err_code_1: size_t = ZSTD_getFrameHeader_advanced(
                            &raw mut (*zds).fParams,
                            &raw mut (*zds).headerBuffer as *mut BYTE as *const ::core::ffi::c_void,
                            (*zds).lhSize,
                            (*zds).format,
                        ) as size_t;
                        if ERR_isError(err_code_1) != 0 {
                            return err_code_1;
                        }
                        return (if (if (*zds).format as ::core::ffi::c_uint
                            == ZSTD_f_zstd1 as ::core::ffi::c_int as ::core::ffi::c_uint
                        {
                            6 as ::core::ffi::c_int
                        } else {
                            2 as ::core::ffi::c_int
                        }) as size_t
                            > hSize
                        {
                            (if (*zds).format as ::core::ffi::c_uint
                                == ZSTD_f_zstd1 as ::core::ffi::c_int as ::core::ffi::c_uint
                            {
                                6 as ::core::ffi::c_int
                            } else {
                                2 as ::core::ffi::c_int
                            }) as size_t
                        } else {
                            hSize
                        })
                        .wrapping_sub((*zds).lhSize)
                        .wrapping_add(ZSTD_blockHeaderSize);
                    }
                    ::libc::memcpy(
                        (&raw mut (*zds).headerBuffer as *mut BYTE).offset((*zds).lhSize as isize)
                            as *mut ::core::ffi::c_void,
                        ip as *const ::core::ffi::c_void,
                        toLoad as ::libc::size_t,
                    );
                    (*zds).lhSize = hSize;
                    ip = ip.offset(toLoad as isize);
                    current_block_402 = 7792909578691485565;
                } else {
                    if (*zds).fParams.frameContentSize != ZSTD_CONTENTSIZE_UNKNOWN
                        && (*zds).fParams.frameType as ::core::ffi::c_uint
                            != ZSTD_skippableFrame as ::core::ffi::c_int as ::core::ffi::c_uint
                        && oend.offset_from(op) as ::core::ffi::c_long as size_t as U64
                            as ::core::ffi::c_ulonglong
                            >= (*zds).fParams.frameContentSize
                    {
                        let cSize: size_t = ZSTD_findFrameCompressedSize_advanced(
                            istart as *const ::core::ffi::c_void,
                            iend.offset_from(istart) as ::core::ffi::c_long as size_t,
                            (*zds).format,
                        ) as size_t;
                        if cSize <= iend.offset_from(istart) as ::core::ffi::c_long as size_t {
                            let decompressedSize: size_t = ZSTD_decompress_usingDDict(
                                zds as *mut ZSTD_DCtx,
                                op as *mut ::core::ffi::c_void,
                                oend.offset_from(op) as ::core::ffi::c_long as size_t,
                                istart as *const ::core::ffi::c_void,
                                cSize,
                                ZSTD_getDDict(zds as *mut ZSTD_DCtx),
                            ) as size_t;
                            if ERR_isError(decompressedSize) != 0 {
                                return decompressedSize;
                            }
                            ip = istart.offset(cSize as isize);
                            op = if !op.is_null() {
                                op.offset(decompressedSize as isize)
                            } else {
                                op
                            };
                            (*zds).expected = 0 as size_t;
                            (*zds).streamStage = zdss_init;
                            someMoreWork = 0 as U32;
                            current_block_402 = 7792909578691485565;
                        } else {
                            current_block_402 = 8968043056769084000;
                        }
                    } else {
                        current_block_402 = 8968043056769084000;
                    }
                    match current_block_402 {
                        7792909578691485565 => {}
                        _ => {
                            if (*zds).outBufferMode as ::core::ffi::c_uint
                                == ZSTD_bm_stable as ::core::ffi::c_int as ::core::ffi::c_uint
                                && (*zds).fParams.frameType as ::core::ffi::c_uint
                                    != ZSTD_skippableFrame as ::core::ffi::c_int
                                        as ::core::ffi::c_uint
                                && (*zds).fParams.frameContentSize != ZSTD_CONTENTSIZE_UNKNOWN
                                && (oend.offset_from(op) as ::core::ffi::c_long as size_t as U64
                                    as ::core::ffi::c_ulonglong)
                                    < (*zds).fParams.frameContentSize
                            {
                                return -(ZSTD_error_dstSize_tooSmall as ::core::ffi::c_int)
                                    as size_t;
                            }
                            let err_code_2: size_t = ZSTD_decompressBegin_usingDDict(
                                zds as *mut ZSTD_DCtx,
                                ZSTD_getDDict(zds as *mut ZSTD_DCtx),
                            ) as size_t;
                            if ERR_isError(err_code_2) != 0 {
                                return err_code_2;
                            }
                            if (*zds).format as ::core::ffi::c_uint
                                == ZSTD_f_zstd1 as ::core::ffi::c_int as ::core::ffi::c_uint
                                && MEM_readLE32(
                                    &raw mut (*zds).headerBuffer as *mut BYTE
                                        as *const ::core::ffi::c_void,
                                ) & ZSTD_MAGIC_SKIPPABLE_MASK as U32
                                    == ZSTD_MAGIC_SKIPPABLE_START as U32
                            {
                                (*zds).expected = MEM_readLE32(
                                    (&raw mut (*zds).headerBuffer as *mut BYTE)
                                        .offset(ZSTD_FRAMEIDSIZE as isize)
                                        as *const ::core::ffi::c_void,
                                ) as size_t;
                                (*zds).stage = ZSTDds_skipFrame;
                            } else {
                                let err_code_3: size_t = ZSTD_decodeFrameHeader(
                                    zds as *mut ZSTD_DCtx,
                                    &raw mut (*zds).headerBuffer as *mut BYTE
                                        as *const ::core::ffi::c_void,
                                    (*zds).lhSize,
                                )
                                    as size_t;
                                if ERR_isError(err_code_3) != 0 {
                                    return err_code_3;
                                }
                                (*zds).expected = ZSTD_blockHeaderSize;
                                (*zds).stage = ZSTDds_decodeBlockHeader;
                            }
                            (*zds).fParams.windowSize = if (*zds).fParams.windowSize
                                > ((1 as ::core::ffi::c_uint) << 10 as ::core::ffi::c_int)
                                    as ::core::ffi::c_ulonglong
                            {
                                (*zds).fParams.windowSize
                            } else {
                                ((1 as ::core::ffi::c_uint) << 10 as ::core::ffi::c_int)
                                    as ::core::ffi::c_ulonglong
                            };
                            if (*zds).fParams.windowSize
                                > (*zds).maxWindowSize as ::core::ffi::c_ulonglong
                            {
                                return -(ZSTD_error_frameParameter_windowTooLarge
                                    as ::core::ffi::c_int)
                                    as size_t;
                            }
                            if (*zds).maxBlockSizeParam != 0 as ::core::ffi::c_int {
                                (*zds).fParams.blockSizeMax = if (*zds).fParams.blockSizeMax
                                    < (*zds).maxBlockSizeParam as ::core::ffi::c_uint
                                {
                                    (*zds).fParams.blockSizeMax
                                } else {
                                    (*zds).maxBlockSizeParam as ::core::ffi::c_uint
                                };
                            }
                            let neededInBuffSize: size_t =
                                (if (*zds).fParams.blockSizeMax > 4 as ::core::ffi::c_uint {
                                    (*zds).fParams.blockSizeMax
                                } else {
                                    4 as ::core::ffi::c_uint
                                }) as size_t;
                            let neededOutBuffSize: size_t = if (*zds).outBufferMode
                                as ::core::ffi::c_uint
                                == ZSTD_bm_buffered as ::core::ffi::c_int as ::core::ffi::c_uint
                            {
                                ZSTD_decodingBufferSize_internal(
                                    (*zds).fParams.windowSize,
                                    (*zds).fParams.frameContentSize,
                                    (*zds).fParams.blockSizeMax as size_t,
                                ) as size_t
                            } else {
                                0 as size_t
                            };
                            ZSTD_DCtx_updateOversizedDuration(
                                zds,
                                neededInBuffSize,
                                neededOutBuffSize,
                            );
                            let tooSmall: ::core::ffi::c_int = ((*zds).inBuffSize
                                < neededInBuffSize
                                || (*zds).outBuffSize < neededOutBuffSize)
                                as ::core::ffi::c_int;
                            let tooLarge: ::core::ffi::c_int =
                                ZSTD_DCtx_isOversizedTooLong(zds) as ::core::ffi::c_int;
                            if tooSmall != 0 || tooLarge != 0 {
                                let bufferSize: size_t =
                                    neededInBuffSize.wrapping_add(neededOutBuffSize);
                                if (*zds).staticSize != 0 {
                                    if bufferSize
                                        > (*zds).staticSize.wrapping_sub(::core::mem::size_of::<
                                            ZSTD_DCtx,
                                        >(
                                        )
                                            as size_t)
                                    {
                                        return -(ZSTD_error_memory_allocation as ::core::ffi::c_int)
                                            as size_t;
                                    }
                                } else {
                                    ZSTD_customFree(
                                        (*zds).inBuff as *mut ::core::ffi::c_void,
                                        (*zds).customMem,
                                    );
                                    (*zds).inBuffSize = 0 as size_t;
                                    (*zds).outBuffSize = 0 as size_t;
                                    (*zds).inBuff = ZSTD_customMalloc(bufferSize, (*zds).customMem)
                                        as *mut ::core::ffi::c_char;
                                    if (*zds).inBuff.is_null() {
                                        return -(ZSTD_error_memory_allocation as ::core::ffi::c_int)
                                            as size_t;
                                    }
                                }
                                (*zds).inBuffSize = neededInBuffSize;
                                (*zds).outBuff = (*zds).inBuff.offset((*zds).inBuffSize as isize);
                                (*zds).outBuffSize = neededOutBuffSize;
                            }
                            (*zds).streamStage = zdss_read;
                            current_block_402 = 7991679940794782184;
                        }
                    }
                }
            }
            _ => {}
        }
        match current_block_402 {
            7991679940794782184 => {
                let neededInSize: size_t = ZSTD_nextSrcSizeToDecompressWithInputSize(
                    zds as *mut ZSTD_DCtx,
                    iend.offset_from(ip) as ::core::ffi::c_long as size_t,
                ) as size_t;
                if neededInSize == 0 as size_t {
                    (*zds).streamStage = zdss_init;
                    someMoreWork = 0 as U32;
                    current_block_402 = 7792909578691485565;
                } else if iend.offset_from(ip) as ::core::ffi::c_long as size_t >= neededInSize {
                    let err_code_4: size_t = ZSTD_decompressContinueStream(
                        zds,
                        &raw mut op,
                        oend,
                        ip as *const ::core::ffi::c_void,
                        neededInSize,
                    ) as size_t;
                    if ERR_isError(err_code_4) != 0 {
                        return err_code_4;
                    }
                    ip = ip.offset(neededInSize as isize);
                    current_block_402 = 7792909578691485565;
                } else if ip == iend {
                    someMoreWork = 0 as U32;
                    current_block_402 = 7792909578691485565;
                } else {
                    (*zds).streamStage = zdss_load;
                    current_block_402 = 18111739650402451604;
                }
            }
            _ => {}
        }
        match current_block_402 {
            18111739650402451604 => {
                let neededInSize_0: size_t =
                    ZSTD_nextSrcSizeToDecompress(zds as *mut ZSTD_DCtx) as size_t;
                let toLoad_0: size_t = neededInSize_0.wrapping_sub((*zds).inPos);
                let isSkipFrame: ::core::ffi::c_int =
                    ZSTD_isSkipFrame(zds as *mut ZSTD_DCtx) as ::core::ffi::c_int;
                let mut loadedSize: size_t = 0;
                if isSkipFrame != 0 {
                    loadedSize = if toLoad_0 < iend.offset_from(ip) as ::core::ffi::c_long as size_t
                    {
                        toLoad_0
                    } else {
                        iend.offset_from(ip) as ::core::ffi::c_long as size_t
                    };
                } else {
                    if toLoad_0 > (*zds).inBuffSize.wrapping_sub((*zds).inPos) {
                        return -(ZSTD_error_corruption_detected as ::core::ffi::c_int) as size_t;
                    }
                    loadedSize = ZSTD_limitCopy(
                        (*zds).inBuff.offset((*zds).inPos as isize) as *mut ::core::ffi::c_void,
                        toLoad_0,
                        ip as *const ::core::ffi::c_void,
                        iend.offset_from(ip) as ::core::ffi::c_long as size_t,
                    );
                }
                if loadedSize != 0 as size_t {
                    ip = ip.offset(loadedSize as isize);
                    (*zds).inPos = ((*zds).inPos as ::core::ffi::c_ulong)
                        .wrapping_add(loadedSize as ::core::ffi::c_ulong)
                        as size_t as size_t;
                }
                if loadedSize < toLoad_0 {
                    someMoreWork = 0 as U32;
                } else {
                    (*zds).inPos = 0 as size_t;
                    let err_code_5: size_t = ZSTD_decompressContinueStream(
                        zds,
                        &raw mut op,
                        oend,
                        (*zds).inBuff as *const ::core::ffi::c_void,
                        neededInSize_0,
                    ) as size_t;
                    if ERR_isError(err_code_5) != 0 {
                        return err_code_5;
                    }
                }
            }
            _ => {}
        }
    }
    (*input).pos =
        ip.offset_from((*input).src as *const ::core::ffi::c_char) as ::core::ffi::c_long as size_t;
    (*output).pos =
        op.offset_from((*output).dst as *mut ::core::ffi::c_char) as ::core::ffi::c_long as size_t;
    (*zds).expectedOutBuffer = *output;
    if ip == istart && op == ostart {
        (*zds).noForwardProgress += 1;
        if (*zds).noForwardProgress >= ZSTD_NO_FORWARD_PROGRESS_MAX {
            if op == oend {
                return -(ZSTD_error_noForwardProgress_destFull as ::core::ffi::c_int) as size_t;
            }
            if ip == iend {
                return -(ZSTD_error_noForwardProgress_inputEmpty as ::core::ffi::c_int) as size_t;
            }
        }
    } else {
        (*zds).noForwardProgress = 0 as ::core::ffi::c_int;
    }
    let mut nextSrcSizeHint: size_t = ZSTD_nextSrcSizeToDecompress(zds as *mut ZSTD_DCtx);
    if nextSrcSizeHint == 0 {
        if (*zds).outEnd == (*zds).outStart {
            if (*zds).hostageByte != 0 {
                if (*input).pos >= (*input).size {
                    (*zds).streamStage = zdss_read;
                    return 1 as size_t;
                }
                (*input).pos = (*input).pos.wrapping_add(1);
            }
            return 0 as size_t;
        }
        if (*zds).hostageByte == 0 {
            (*input).pos = (*input).pos.wrapping_sub(1);
            (*zds).hostageByte = 1 as U32;
        }
        return 1 as size_t;
    }
    nextSrcSizeHint =
        (nextSrcSizeHint as ::core::ffi::c_ulong).wrapping_add(ZSTD_blockHeaderSize.wrapping_mul(
            (ZSTD_nextInputType(zds as *mut ZSTD_DCtx) as ::core::ffi::c_uint
                == ZSTDnit_block as ::core::ffi::c_int as ::core::ffi::c_uint)
                as ::core::ffi::c_int as size_t,
        ) as ::core::ffi::c_ulong) as size_t as size_t;
    nextSrcSizeHint = (nextSrcSizeHint as ::core::ffi::c_ulong)
        .wrapping_sub((*zds).inPos as ::core::ffi::c_ulong) as size_t
        as size_t;
    return nextSrcSizeHint;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZSTD_decompressStream_simpleArgs(
    mut dctx: *mut ZSTD_DCtx,
    mut dst: *mut ::core::ffi::c_void,
    mut dstCapacity: size_t,
    mut dstPos: *mut size_t,
    mut src: *const ::core::ffi::c_void,
    mut srcSize: size_t,
    mut srcPos: *mut size_t,
) -> size_t {
    let mut output: ZSTD_outBuffer = ZSTD_outBuffer {
        dst: ::core::ptr::null_mut::<::core::ffi::c_void>(),
        size: 0,
        pos: 0,
    };
    let mut input: ZSTD_inBuffer = ZSTD_inBuffer {
        src: ::core::ptr::null::<::core::ffi::c_void>(),
        size: 0,
        pos: 0,
    };
    output.dst = dst;
    output.size = dstCapacity;
    output.pos = *dstPos;
    input.src = src;
    input.size = srcSize;
    input.pos = *srcPos;
    let cErr: size_t =
        ZSTD_decompressStream(dctx as *mut ZSTD_DStream, &raw mut output, &raw mut input) as size_t;
    *dstPos = output.pos;
    *srcPos = input.pos;
    return cErr;
}
