extern "C" {
    pub type ZSTD_DCtx_s;
    fn ZSTD_createDStream() -> *mut ZSTD_DStream;
    fn ZSTD_freeDStream(zds: *mut ZSTD_DStream) -> size_t;
    fn ZSTD_initDStream(zds: *mut ZSTD_DStream) -> size_t;
    fn ZSTD_decompressStream(
        zds: *mut ZSTD_DStream,
        output: *mut ZSTD_outBuffer,
        input: *mut ZSTD_inBuffer,
    ) -> size_t;
    fn ZSTD_DStreamInSize() -> size_t;
    fn ZSTD_DStreamOutSize() -> size_t;
    fn ZSTD_createDStream_advanced(customMem: ZSTD_customMem) -> *mut ZSTD_DStream;
    fn ZSTD_initDStream_usingDict(
        zds: *mut ZSTD_DStream,
        dict: *const ::core::ffi::c_void,
        dictSize: size_t,
    ) -> size_t;
}
pub type size_t = usize;
pub type ZSTD_DCtx = ZSTD_DCtx_s;
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
pub type ZSTD_DStream = ZSTD_DCtx;
pub type ZBUFF_DCtx = ZSTD_DStream;
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFF_createDCtx() -> *mut ZBUFF_DCtx {
    return ZSTD_createDStream() as *mut ZBUFF_DCtx;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFF_createDCtx_advanced(
    mut customMem: ZSTD_customMem,
) -> *mut ZBUFF_DCtx {
    return ZSTD_createDStream_advanced(customMem) as *mut ZBUFF_DCtx;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFF_freeDCtx(mut zbd: *mut ZBUFF_DCtx) -> size_t {
    return ZSTD_freeDStream(zbd as *mut ZSTD_DStream);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFF_decompressInitDictionary(
    mut zbd: *mut ZBUFF_DCtx,
    mut dict: *const ::core::ffi::c_void,
    mut dictSize: size_t,
) -> size_t {
    return ZSTD_initDStream_usingDict(zbd as *mut ZSTD_DStream, dict, dictSize);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFF_decompressInit(mut zbd: *mut ZBUFF_DCtx) -> size_t {
    return ZSTD_initDStream(zbd as *mut ZSTD_DStream);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFF_decompressContinue(
    mut zbd: *mut ZBUFF_DCtx,
    mut dst: *mut ::core::ffi::c_void,
    mut dstCapacityPtr: *mut size_t,
    mut src: *const ::core::ffi::c_void,
    mut srcSizePtr: *mut size_t,
) -> size_t {
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
    let mut result: size_t = 0;
    outBuff.dst = dst;
    outBuff.pos = 0 as size_t;
    outBuff.size = *dstCapacityPtr;
    inBuff.src = src;
    inBuff.pos = 0 as size_t;
    inBuff.size = *srcSizePtr;
    result = ZSTD_decompressStream(zbd as *mut ZSTD_DStream, &raw mut outBuff, &raw mut inBuff);
    *dstCapacityPtr = outBuff.pos;
    *srcSizePtr = inBuff.pos;
    return result;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFF_recommendedDInSize() -> size_t {
    return ZSTD_DStreamInSize();
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ZBUFF_recommendedDOutSize() -> size_t {
    return ZSTD_DStreamOutSize();
}
