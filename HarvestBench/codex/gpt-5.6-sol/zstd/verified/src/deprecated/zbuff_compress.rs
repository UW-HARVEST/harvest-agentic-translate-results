extern "C" {
    pub type ZSTD_CCtx_s;
    fn ZSTD_CCtx_setParameter(
        cctx: *mut ZSTD_CCtx,
        param: ZSTD_cParameter,
        value: ::core::ffi::c_int,
    ) -> size_t;
    fn ZSTD_CCtx_setPledgedSrcSize(
        cctx: *mut ZSTD_CCtx,
        pledgedSrcSize: ::core::ffi::c_ulonglong,
    ) -> size_t;
    fn ZSTD_CCtx_reset(cctx: *mut ZSTD_CCtx, reset: ZSTD_ResetDirective) -> size_t;
    fn ZSTD_createCStream() -> *mut ZSTD_CStream;
    fn ZSTD_freeCStream(zcs: *mut ZSTD_CStream) -> size_t;
    fn ZSTD_CStreamInSize() -> size_t;
    fn ZSTD_CStreamOutSize() -> size_t;
    fn ZSTD_initCStream(zcs: *mut ZSTD_CStream, compressionLevel: ::core::ffi::c_int) -> size_t;
    fn ZSTD_compressStream(
        zcs: *mut ZSTD_CStream,
        output: *mut ZSTD_outBuffer,
        input: *mut ZSTD_inBuffer,
    ) -> size_t;
    fn ZSTD_flushStream(zcs: *mut ZSTD_CStream, output: *mut ZSTD_outBuffer) -> size_t;
    fn ZSTD_endStream(zcs: *mut ZSTD_CStream, output: *mut ZSTD_outBuffer) -> size_t;
    fn ZSTD_CCtx_loadDictionary(
        cctx: *mut ZSTD_CCtx,
        dict: *const ::core::ffi::c_void,
        dictSize: size_t,
    ) -> size_t;
    fn ZSTD_createCStream_advanced(customMem: ZSTD_customMem) -> *mut ZSTD_CStream;
    fn ZSTD_checkCParams(params: ZSTD_compressionParameters) -> size_t;
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
pub type ZSTD_CCtx = ZSTD_CCtx_s;
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
pub type ZSTD_ResetDirective = ::core::ffi::c_uint;
pub const ZSTD_reset_session_and_parameters: ZSTD_ResetDirective = 3;
pub const ZSTD_reset_parameters: ZSTD_ResetDirective = 2;
pub const ZSTD_reset_session_only: ZSTD_ResetDirective = 1;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ZSTD_inBuffer_s {
    pub src: *const ::core::ffi::c_void,
    pub size: size_t,
    pub pos: size_t,
}
pub type ZSTD_inBuffer = ZSTD_inBuffer_s;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ZSTD_outBuffer_s {
    pub dst: *mut ::core::ffi::c_void,
    pub size: size_t,
    pub pos: size_t,
}
pub type ZSTD_outBuffer = ZSTD_outBuffer_s;
pub type ZSTD_CStream = ZSTD_CCtx;
pub type ZBUFF_CCtx = ZSTD_CStream;
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
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ZSTD_frameParameters {
    pub contentSizeFlag: ::core::ffi::c_int,
    pub checksumFlag: ::core::ffi::c_int,
    pub noDictIDFlag: ::core::ffi::c_int,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ZSTD_parameters {
    pub cParams: ZSTD_compressionParameters,
    pub fParams: ZSTD_frameParameters,
}
pub type ZSTD_allocFunction =
    Option<unsafe extern "C" fn(*mut ::core::ffi::c_void, size_t) -> *mut ::core::ffi::c_void>;
pub type ZSTD_freeFunction =
    Option<unsafe extern "C" fn(*mut ::core::ffi::c_void, *mut ::core::ffi::c_void) -> ()>;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct ZSTD_customMem {
    pub customAlloc: ZSTD_allocFunction,
    pub customFree: ZSTD_freeFunction,
    pub opaque: *mut ::core::ffi::c_void,
}
pub const ZSTD_CONTENTSIZE_UNKNOWN: ::core::ffi::c_ulonglong =
    (0 as ::core::ffi::c_ulonglong).wrapping_sub(1 as ::core::ffi::c_ulonglong);
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFF_createCCtx() -> *mut ZBUFF_CCtx {
    return ZSTD_createCStream() as *mut ZBUFF_CCtx;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFF_createCCtx_advanced(
    mut customMem: ZSTD_customMem,
) -> *mut ZBUFF_CCtx {
    return ZSTD_createCStream_advanced(customMem) as *mut ZBUFF_CCtx;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFF_freeCCtx(mut zbc: *mut ZBUFF_CCtx) -> size_t {
    return ZSTD_freeCStream(zbc as *mut ZSTD_CStream);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFF_compressInit_advanced(
    mut zbc: *mut ZBUFF_CCtx,
    mut dict: *const ::core::ffi::c_void,
    mut dictSize: size_t,
    mut params: ZSTD_parameters,
    mut pledgedSrcSize: ::core::ffi::c_ulonglong,
) -> size_t {
    if pledgedSrcSize == 0 as ::core::ffi::c_ulonglong {
        pledgedSrcSize = ZSTD_CONTENTSIZE_UNKNOWN;
    }
    let err_code: size_t =
        ZSTD_CCtx_reset(zbc as *mut ZSTD_CCtx, ZSTD_reset_session_only) as size_t;
    if ERR_isError(err_code) != 0 {
        return err_code;
    }
    let err_code_0: size_t =
        ZSTD_CCtx_setPledgedSrcSize(zbc as *mut ZSTD_CCtx, pledgedSrcSize) as size_t;
    if ERR_isError(err_code_0) != 0 {
        return err_code_0;
    }
    let err_code_1: size_t = ZSTD_checkCParams(params.cParams) as size_t;
    if ERR_isError(err_code_1) != 0 {
        return err_code_1;
    }
    let err_code_2: size_t = ZSTD_CCtx_setParameter(
        zbc as *mut ZSTD_CCtx,
        ZSTD_c_windowLog,
        params.cParams.windowLog as ::core::ffi::c_int,
    ) as size_t;
    if ERR_isError(err_code_2) != 0 {
        return err_code_2;
    }
    let err_code_3: size_t = ZSTD_CCtx_setParameter(
        zbc as *mut ZSTD_CCtx,
        ZSTD_c_hashLog,
        params.cParams.hashLog as ::core::ffi::c_int,
    ) as size_t;
    if ERR_isError(err_code_3) != 0 {
        return err_code_3;
    }
    let err_code_4: size_t = ZSTD_CCtx_setParameter(
        zbc as *mut ZSTD_CCtx,
        ZSTD_c_chainLog,
        params.cParams.chainLog as ::core::ffi::c_int,
    ) as size_t;
    if ERR_isError(err_code_4) != 0 {
        return err_code_4;
    }
    let err_code_5: size_t = ZSTD_CCtx_setParameter(
        zbc as *mut ZSTD_CCtx,
        ZSTD_c_searchLog,
        params.cParams.searchLog as ::core::ffi::c_int,
    ) as size_t;
    if ERR_isError(err_code_5) != 0 {
        return err_code_5;
    }
    let err_code_6: size_t = ZSTD_CCtx_setParameter(
        zbc as *mut ZSTD_CCtx,
        ZSTD_c_minMatch,
        params.cParams.minMatch as ::core::ffi::c_int,
    ) as size_t;
    if ERR_isError(err_code_6) != 0 {
        return err_code_6;
    }
    let err_code_7: size_t = ZSTD_CCtx_setParameter(
        zbc as *mut ZSTD_CCtx,
        ZSTD_c_targetLength,
        params.cParams.targetLength as ::core::ffi::c_int,
    ) as size_t;
    if ERR_isError(err_code_7) != 0 {
        return err_code_7;
    }
    let err_code_8: size_t = ZSTD_CCtx_setParameter(
        zbc as *mut ZSTD_CCtx,
        ZSTD_c_strategy,
        params.cParams.strategy as ::core::ffi::c_int,
    ) as size_t;
    if ERR_isError(err_code_8) != 0 {
        return err_code_8;
    }
    let err_code_9: size_t = ZSTD_CCtx_setParameter(
        zbc as *mut ZSTD_CCtx,
        ZSTD_c_contentSizeFlag,
        params.fParams.contentSizeFlag,
    ) as size_t;
    if ERR_isError(err_code_9) != 0 {
        return err_code_9;
    }
    let err_code_10: size_t = ZSTD_CCtx_setParameter(
        zbc as *mut ZSTD_CCtx,
        ZSTD_c_checksumFlag,
        params.fParams.checksumFlag,
    ) as size_t;
    if ERR_isError(err_code_10) != 0 {
        return err_code_10;
    }
    let err_code_11: size_t = ZSTD_CCtx_setParameter(
        zbc as *mut ZSTD_CCtx,
        ZSTD_c_dictIDFlag,
        params.fParams.noDictIDFlag,
    ) as size_t;
    if ERR_isError(err_code_11) != 0 {
        return err_code_11;
    }
    let err_code_12: size_t =
        ZSTD_CCtx_loadDictionary(zbc as *mut ZSTD_CCtx, dict, dictSize) as size_t;
    if ERR_isError(err_code_12) != 0 {
        return err_code_12;
    }
    return 0 as size_t;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFF_compressInitDictionary(
    mut zbc: *mut ZBUFF_CCtx,
    mut dict: *const ::core::ffi::c_void,
    mut dictSize: size_t,
    mut compressionLevel: ::core::ffi::c_int,
) -> size_t {
    let err_code: size_t =
        ZSTD_CCtx_reset(zbc as *mut ZSTD_CCtx, ZSTD_reset_session_only) as size_t;
    if ERR_isError(err_code) != 0 {
        return err_code;
    }
    let err_code_0: size_t = ZSTD_CCtx_setParameter(
        zbc as *mut ZSTD_CCtx,
        ZSTD_c_compressionLevel,
        compressionLevel,
    ) as size_t;
    if ERR_isError(err_code_0) != 0 {
        return err_code_0;
    }
    let err_code_1: size_t =
        ZSTD_CCtx_loadDictionary(zbc as *mut ZSTD_CCtx, dict, dictSize) as size_t;
    if ERR_isError(err_code_1) != 0 {
        return err_code_1;
    }
    return 0 as size_t;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFF_compressInit(
    mut zbc: *mut ZBUFF_CCtx,
    mut compressionLevel: ::core::ffi::c_int,
) -> size_t {
    return ZSTD_initCStream(zbc as *mut ZSTD_CStream, compressionLevel);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFF_compressContinue(
    mut zbc: *mut ZBUFF_CCtx,
    mut dst: *mut ::core::ffi::c_void,
    mut dstCapacityPtr: *mut size_t,
    mut src: *const ::core::ffi::c_void,
    mut srcSizePtr: *mut size_t,
) -> size_t {
    let mut result: size_t = 0;
    let mut outBuff: ZSTD_outBuffer = ZSTD_outBuffer {
        dst: ::core::ptr::null_mut::<::core::ffi::c_void>(),
        size: 0,
        pos: 0,
    };
    let mut inBuff: ZSTD_inBuffer = ZSTD_inBuffer {
        src: ::core::ptr::null::<::core::ffi::c_void>(),
        size: 0,
        pos: 0,
    };
    outBuff.dst = dst;
    outBuff.pos = 0 as size_t;
    outBuff.size = *dstCapacityPtr;
    inBuff.src = src;
    inBuff.pos = 0 as size_t;
    inBuff.size = *srcSizePtr;
    result = ZSTD_compressStream(zbc as *mut ZSTD_CStream, &raw mut outBuff, &raw mut inBuff);
    *dstCapacityPtr = outBuff.pos;
    *srcSizePtr = inBuff.pos;
    return result;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFF_compressFlush(
    mut zbc: *mut ZBUFF_CCtx,
    mut dst: *mut ::core::ffi::c_void,
    mut dstCapacityPtr: *mut size_t,
) -> size_t {
    let mut result: size_t = 0;
    let mut outBuff: ZSTD_outBuffer = ZSTD_outBuffer {
        dst: ::core::ptr::null_mut::<::core::ffi::c_void>(),
        size: 0,
        pos: 0,
    };
    outBuff.dst = dst;
    outBuff.pos = 0 as size_t;
    outBuff.size = *dstCapacityPtr;
    result = ZSTD_flushStream(zbc as *mut ZSTD_CStream, &raw mut outBuff);
    *dstCapacityPtr = outBuff.pos;
    return result;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFF_compressEnd(
    mut zbc: *mut ZBUFF_CCtx,
    mut dst: *mut ::core::ffi::c_void,
    mut dstCapacityPtr: *mut size_t,
) -> size_t {
    let mut result: size_t = 0;
    let mut outBuff: ZSTD_outBuffer = ZSTD_outBuffer {
        dst: ::core::ptr::null_mut::<::core::ffi::c_void>(),
        size: 0,
        pos: 0,
    };
    outBuff.dst = dst;
    outBuff.pos = 0 as size_t;
    outBuff.size = *dstCapacityPtr;
    result = ZSTD_endStream(zbc as *mut ZSTD_CStream, &raw mut outBuff);
    *dstCapacityPtr = outBuff.pos;
    return result;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFF_recommendedCInSize() -> size_t {
    return ZSTD_CStreamInSize();
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFF_recommendedCOutSize() -> size_t {
    return ZSTD_CStreamOutSize();
}
