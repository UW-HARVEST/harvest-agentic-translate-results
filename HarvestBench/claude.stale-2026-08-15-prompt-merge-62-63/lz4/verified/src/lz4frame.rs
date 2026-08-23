// Translation of lz4frame.c (LZ4 Frame v1.10.0). Target: x86_64 little-endian. LZ4F_HEAPMODE=0.
#![allow(non_snake_case, non_upper_case_globals, non_camel_case_types)]

use core::ffi::{c_char, c_int, c_uint, c_void};
use core::ptr;

use crate::lz4::{
    LZ4_attach_dictionary, LZ4_compress_fast_continue, LZ4_compress_fast_extState_fastReset,
    LZ4_decompress_safe_usingDict, LZ4_initStream, LZ4_loadDict, LZ4_loadDictSlow,
    LZ4_resetStream_fast, LZ4_saveDict, LZ4_sizeofState, LZ4_stream_t,
};
use crate::lz4hc::{
    LZ4_attach_HC_dictionary, LZ4_compress_HC_continue, LZ4_compress_HC_extStateHC_fastReset,
    LZ4_favorDecompressionSpeed, LZ4_initStreamHC, LZ4_loadDictHC, LZ4_resetStreamHC_fast,
    LZ4_saveDictHC, LZ4_setCompressionLevel, LZ4_sizeofStateHC, LZ4_streamHC_t,
};
use crate::xxhash::{xxh32, xxh32_digest, xxh32_reset, xxh32_update, XXH32_state_t};

pub type LZ4F_errorCode_t = usize;

const LZ4HC_CLEVEL_MIN: c_int = 2;
const LZ4HC_CLEVEL_MAX: c_int = 12;
const LZ4HC_CLEVEL_DEFAULT: c_int = 9;

const LZ4F_VERSION: c_uint = 100;
const LZ4F_HEADER_SIZE_MAX: usize = 19;
const LZ4F_MAGICNUMBER: u32 = 0x184D2204;
const LZ4F_MAGIC_SKIPPABLE_START: u32 = 0x184D2A50;
const LZ4F_MIN_SIZE_TO_KNOW_HEADER_LENGTH: usize = 5;
const LZ4F_BLOCKUNCOMPRESSED_FLAG: u32 = 0x80000000;

const minFHSize: usize = 7;
const maxFHSize: usize = LZ4F_HEADER_SIZE_MAX;
const BHSize: usize = 4;
const BFSize: usize = 4;

const _1BIT: u32 = 0x01;
const _2BITS: u32 = 0x03;
const _3BITS: u32 = 0x07;
const _4BITS: u32 = 0x0F;

pub type LZ4F_blockSizeID_t = c_int;
pub const LZ4F_default: c_int = 0;
pub const LZ4F_max64KB: c_int = 4;
pub const LZ4F_max256KB: c_int = 5;
pub const LZ4F_max1MB: c_int = 6;
pub const LZ4F_max4MB: c_int = 7;

const LZ4F_blockLinked: c_int = 0;
const LZ4F_blockIndependent: c_int = 1;
const LZ4F_noContentChecksum: c_int = 0;
const LZ4F_contentChecksumEnabled: c_int = 1;
const LZ4F_noBlockChecksum: c_int = 0;
const LZ4F_blockChecksumEnabled: c_int = 1;
const LZ4F_frame: c_int = 0;
const LZ4F_skippableFrame: c_int = 1;
const LZ4F_BLOCKSIZEID_DEFAULT: c_int = LZ4F_max64KB;

#[allow(dead_code)]
mod err {
    pub const OK_NoError: usize = 0;
    pub const ERROR_GENERIC: usize = 1;
    pub const ERROR_maxBlockSize_invalid: usize = 2;
    pub const ERROR_blockMode_invalid: usize = 3;
    pub const ERROR_parameter_invalid: usize = 4;
    pub const ERROR_compressionLevel_invalid: usize = 5;
    pub const ERROR_headerVersion_wrong: usize = 6;
    pub const ERROR_blockChecksum_invalid: usize = 7;
    pub const ERROR_reservedFlag_set: usize = 8;
    pub const ERROR_allocation_failed: usize = 9;
    pub const ERROR_srcSize_tooLarge: usize = 10;
    pub const ERROR_dstMaxSize_tooSmall: usize = 11;
    pub const ERROR_frameHeader_incomplete: usize = 12;
    pub const ERROR_frameType_unknown: usize = 13;
    pub const ERROR_frameSize_wrong: usize = 14;
    pub const ERROR_srcPtr_wrong: usize = 15;
    pub const ERROR_decompressionFailed: usize = 16;
    pub const ERROR_headerChecksum_invalid: usize = 17;
    pub const ERROR_contentChecksum_invalid: usize = 18;
    pub const ERROR_frameDecoding_alreadyStarted: usize = 19;
    pub const ERROR_compressionState_uninitialized: usize = 20;
    pub const ERROR_parameter_null: usize = 21;
    pub const ERROR_io_write: usize = 22;
    pub const ERROR_io_read: usize = 23;
    pub const ERROR_maxCode: usize = 24;
}

static LZ4F_errorStrings: [&[u8]; 25] = [
    b"OK_NoError\0",
    b"ERROR_GENERIC\0",
    b"ERROR_maxBlockSize_invalid\0",
    b"ERROR_blockMode_invalid\0",
    b"ERROR_parameter_invalid\0",
    b"ERROR_compressionLevel_invalid\0",
    b"ERROR_headerVersion_wrong\0",
    b"ERROR_blockChecksum_invalid\0",
    b"ERROR_reservedFlag_set\0",
    b"ERROR_allocation_failed\0",
    b"ERROR_srcSize_tooLarge\0",
    b"ERROR_dstMaxSize_tooSmall\0",
    b"ERROR_frameHeader_incomplete\0",
    b"ERROR_frameType_unknown\0",
    b"ERROR_frameSize_wrong\0",
    b"ERROR_srcPtr_wrong\0",
    b"ERROR_decompressionFailed\0",
    b"ERROR_headerChecksum_invalid\0",
    b"ERROR_contentChecksum_invalid\0",
    b"ERROR_frameDecoding_alreadyStarted\0",
    b"ERROR_compressionState_uninitialized\0",
    b"ERROR_parameter_null\0",
    b"ERROR_io_write\0",
    b"ERROR_io_read\0",
    b"ERROR_maxCode\0",
];

#[inline]
fn returnErrorCode(code: usize) -> LZ4F_errorCode_t {
    (0usize).wrapping_sub(code)
}

macro_rules! RETURN_ERROR {
    ($e:ident) => {
        return returnErrorCode(err::$e)
    };
}
macro_rules! RETURN_ERROR_IF {
    ($c:expr, $e:ident) => {
        if $c {
            return returnErrorCode(err::$e);
        }
    };
}
macro_rules! FORWARD_IF_ERROR {
    ($r:expr) => {
        if LZ4F_isError_internal($r) {
            return $r;
        }
    };
}

#[inline]
fn MIN(a: usize, b: usize) -> usize {
    if a < b { a } else { b }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct LZ4F_frameInfo_t {
    pub blockSizeID: LZ4F_blockSizeID_t,
    pub blockMode: c_int,
    pub contentChecksumFlag: c_int,
    pub frameType: c_int,
    pub contentSize: u64,
    pub dictID: c_uint,
    pub blockChecksumFlag: c_int,
}
impl LZ4F_frameInfo_t {
    fn init() -> Self {
        LZ4F_frameInfo_t {
            blockSizeID: LZ4F_max64KB,
            blockMode: LZ4F_blockLinked,
            contentChecksumFlag: LZ4F_noContentChecksum,
            frameType: LZ4F_frame,
            contentSize: 0,
            dictID: 0,
            blockChecksumFlag: LZ4F_noBlockChecksum,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct LZ4F_preferences_t {
    pub frameInfo: LZ4F_frameInfo_t,
    pub compressionLevel: c_int,
    pub autoFlush: c_uint,
    pub favorDecSpeed: c_uint,
    pub reserved: [c_uint; 3],
}
impl LZ4F_preferences_t {
    fn init() -> Self {
        LZ4F_preferences_t {
            frameInfo: LZ4F_frameInfo_t::init(),
            compressionLevel: 0,
            autoFlush: 0,
            favorDecSpeed: 0,
            reserved: [0, 0, 0],
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct LZ4F_compressOptions_t {
    pub stableSrc: c_uint,
    pub reserved: [c_uint; 3],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct LZ4F_decompressOptions_t {
    pub stableDst: c_uint,
    pub skipChecksums: c_uint,
    pub reserved1: c_uint,
    pub reserved0: c_uint,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct LZ4F_CustomMem {
    pub customAlloc: Option<unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void>,
    pub customCalloc: Option<unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void>,
    pub customFree: Option<unsafe extern "C" fn(*mut c_void, *mut c_void)>,
    pub opaqueState: *mut c_void,
}

const LZ4F_defaultCMem: LZ4F_CustomMem = LZ4F_CustomMem {
    customAlloc: None,
    customCalloc: None,
    customFree: None,
    opaqueState: ptr::null_mut(),
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum LZ4F_BlockCompressMode_e {
    LZ4B_COMPRESSED,
    LZ4B_UNCOMPRESSED,
}
use LZ4F_BlockCompressMode_e::*;

const ctxFast: u16 = 1;

#[repr(C)]
pub struct LZ4F_cctx {
    cmem: LZ4F_CustomMem,
    prefs: LZ4F_preferences_t,
    version: u32,
    cStage: u32,
    cdict: *const LZ4F_CDict,
    maxBlockSize: usize,
    maxBufferSize: usize,
    tmpBuff: *mut u8,
    tmpIn: *mut u8,
    tmpInSize: usize,
    totalInSize: u64,
    xxh: XXH32_state_t,
    lz4CtxPtr: *mut c_void,
    lz4CtxAlloc: u16,
    lz4CtxType: u16,
    blockCompressMode: LZ4F_BlockCompressMode_e,
}

#[repr(C)]
pub struct LZ4F_CDict {
    cmem: LZ4F_CustomMem,
    dictContent: *mut c_void,
    fastCtx: *mut LZ4_stream_t,
    HCCtx: *mut LZ4_streamHC_t,
}

/* ===== memory ===== */
unsafe fn LZ4F_calloc(s: usize, cmem: LZ4F_CustomMem) -> *mut c_void {
    if let Some(cc) = cmem.customCalloc {
        return cc(cmem.opaqueState, s);
    }
    if cmem.customAlloc.is_none() {
        return crate::c_calloc(s) as *mut c_void;
    }
    let p = (cmem.customAlloc.unwrap())(cmem.opaqueState, s);
    if !p.is_null() {
        ptr::write_bytes(p as *mut u8, 0, s);
    }
    p
}
unsafe fn LZ4F_malloc(s: usize, cmem: LZ4F_CustomMem) -> *mut c_void {
    if let Some(a) = cmem.customAlloc {
        return a(cmem.opaqueState, s);
    }
    crate::c_malloc(s) as *mut c_void
}
unsafe fn LZ4F_free(p: *mut c_void, cmem: LZ4F_CustomMem) {
    if p.is_null() {
        return;
    }
    if let Some(f) = cmem.customFree {
        f(cmem.opaqueState, p);
        return;
    }
    crate::c_free(p as *mut u8);
}

/* ===== LE read/write ===== */
#[inline]
unsafe fn LZ4F_readLE32(src: *const u8) -> u32 {
    (*src.add(0) as u32)
        | ((*src.add(1) as u32) << 8)
        | ((*src.add(2) as u32) << 16)
        | ((*src.add(3) as u32) << 24)
}
#[inline]
unsafe fn LZ4F_writeLE32(dst: *mut u8, v: u32) {
    *dst.add(0) = v as u8;
    *dst.add(1) = (v >> 8) as u8;
    *dst.add(2) = (v >> 16) as u8;
    *dst.add(3) = (v >> 24) as u8;
}
#[inline]
unsafe fn LZ4F_readLE64(src: *const u8) -> u64 {
    (*src.add(0) as u64)
        | ((*src.add(1) as u64) << 8)
        | ((*src.add(2) as u64) << 16)
        | ((*src.add(3) as u64) << 24)
        | ((*src.add(4) as u64) << 32)
        | ((*src.add(5) as u64) << 40)
        | ((*src.add(6) as u64) << 48)
        | ((*src.add(7) as u64) << 56)
}
#[inline]
unsafe fn LZ4F_writeLE64(dst: *mut u8, v: u64) {
    *dst.add(0) = v as u8;
    *dst.add(1) = (v >> 8) as u8;
    *dst.add(2) = (v >> 16) as u8;
    *dst.add(3) = (v >> 24) as u8;
    *dst.add(4) = (v >> 32) as u8;
    *dst.add(5) = (v >> 40) as u8;
    *dst.add(6) = (v >> 48) as u8;
    *dst.add(7) = (v >> 56) as u8;
}

/* ===== error mgmt ===== */
#[inline]
fn LZ4F_isError_internal(code: LZ4F_errorCode_t) -> bool {
    code > (0usize).wrapping_sub(err::ERROR_maxCode)
}

#[unsafe(no_mangle)]
pub extern "C" fn LZ4F_isError(code: LZ4F_errorCode_t) -> c_uint {
    LZ4F_isError_internal(code) as c_uint
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_getErrorName(code: LZ4F_errorCode_t) -> *const c_char {
    static codeError: &[u8] = b"Unspecified error code\0";
    if LZ4F_isError_internal(code) {
        let idx = (0usize).wrapping_sub(code);
        return LZ4F_errorStrings[idx].as_ptr() as *const c_char;
    }
    codeError.as_ptr() as *const c_char
}

#[unsafe(no_mangle)]
pub extern "C" fn LZ4F_getErrorCode(functionResult: usize) -> c_int {
    if !LZ4F_isError_internal(functionResult) {
        return err::OK_NoError as c_int;
    }
    ((0usize).wrapping_sub(functionResult)) as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn LZ4F_getVersion() -> c_uint {
    LZ4F_VERSION
}
#[unsafe(no_mangle)]
pub extern "C" fn LZ4F_compressionLevel_max() -> c_int {
    LZ4HC_CLEVEL_MAX
}

#[unsafe(no_mangle)]
pub extern "C" fn LZ4F_getBlockSize(mut blockSizeID: LZ4F_blockSizeID_t) -> usize {
    static blockSizes: [usize; 4] = [64 * 1024, 256 * 1024, 1024 * 1024, 4 * 1024 * 1024];
    if blockSizeID == 0 {
        blockSizeID = LZ4F_BLOCKSIZEID_DEFAULT;
    }
    if blockSizeID < LZ4F_max64KB || blockSizeID > LZ4F_max4MB {
        return returnErrorCode(err::ERROR_maxBlockSize_invalid);
    }
    let idx = (blockSizeID - LZ4F_max64KB) as usize;
    blockSizes[idx]
}

#[inline]
unsafe fn LZ4F_headerChecksum(header: *const u8, length: usize) -> u8 {
    (xxh32(header, length, 0) >> 8) as u8
}

fn LZ4F_optimalBSID(requestedBSID: LZ4F_blockSizeID_t, srcSize: usize) -> LZ4F_blockSizeID_t {
    let mut proposedBSID = LZ4F_max64KB;
    let mut maxBlockSize: usize = 64 * 1024;
    // A C enum whose enumerators are all non-negative has `unsigned int` as its
    // compatible type on this target, so `requestedBSID > proposedBSID` is an
    // UNSIGNED comparison (an out-of-range negative value arriving across the
    // FFI boundary compares as a huge positive number).
    while (requestedBSID as u32) > (proposedBSID as u32) {
        if srcSize <= maxBlockSize {
            return proposedBSID;
        }
        proposedBSID += 1;
        maxBlockSize <<= 2;
    }
    requestedBSID
}

unsafe fn LZ4F_compressBound_internal(
    srcSize: usize,
    preferencesPtr: *const LZ4F_preferences_t,
    alreadyBuffered: usize,
) -> usize {
    let mut prefsNull = LZ4F_preferences_t::init();
    prefsNull.frameInfo.contentChecksumFlag = LZ4F_contentChecksumEnabled;
    prefsNull.frameInfo.blockChecksumFlag = LZ4F_blockChecksumEnabled;
    let prefsPtr = if preferencesPtr.is_null() {
        &prefsNull as *const LZ4F_preferences_t
    } else {
        preferencesPtr
    };
    let prefs = &*prefsPtr;
    let flush = prefs.autoFlush | (srcSize == 0) as u32;
    let blockSize = LZ4F_getBlockSize(prefs.frameInfo.blockSizeID);
    // NOTE: every arithmetic operation below is deliberately wrapping, and
    // `nbFullBlocks`/`nbBlocks` are 32-bit, because that is exactly what the C
    // does: `blockSize` is `(size_t)-2` when `blockSizeID` is invalid (the C
    // never error-checks `LZ4F_getBlockSize` here) and the C declares
    // `unsigned const nbFullBlocks = (unsigned)(maxSrcSize / blockSize);`
    // (lz4frame.c:390-402), so both `size_t` wraparound and the truncation to
    // `unsigned` are observable through `LZ4F_compressBound()`.
    let maxBuffered = blockSize.wrapping_sub(1);
    let bufferedSize = MIN(alreadyBuffered, maxBuffered);
    let maxSrcSize = srcSize.wrapping_add(bufferedSize);
    let nbFullBlocks: u32 = (maxSrcSize / blockSize) as u32;
    let partialBlockSize = maxSrcSize & blockSize.wrapping_sub(1);
    let lastBlockSize = if flush != 0 { partialBlockSize } else { 0 };
    let nbBlocks: u32 = nbFullBlocks.wrapping_add((lastBlockSize > 0) as u32);
    // The enum fields are `unsigned int` in C (all enumerators are
    // non-negative), so they ZERO-extend to `size_t`, they do not sign-extend.
    let blockCRCSize = BFSize.wrapping_mul(prefs.frameInfo.blockChecksumFlag as u32 as usize);
    let frameEnd = BHSize
        .wrapping_add((prefs.frameInfo.contentChecksumFlag as u32 as usize).wrapping_mul(BFSize));
    BHSize
        .wrapping_add(blockCRCSize)
        .wrapping_mul(nbBlocks as usize)
        .wrapping_add(blockSize.wrapping_mul(nbFullBlocks as usize))
        .wrapping_add(lastBlockSize)
        .wrapping_add(frameEnd)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_compressFrameBound(
    srcSize: usize,
    preferencesPtr: *const LZ4F_preferences_t,
) -> usize {
    let mut prefs: LZ4F_preferences_t = if !preferencesPtr.is_null() {
        *preferencesPtr
    } else {
        core::mem::zeroed()
    };
    prefs.autoFlush = 1;
    maxFHSize + LZ4F_compressBound_internal(srcSize, &prefs, 0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_compressFrame_usingCDict(
    cctx: *mut LZ4F_cctx,
    dstBuffer: *mut c_void,
    dstCapacity: usize,
    srcBuffer: *const c_void,
    srcSize: usize,
    cdict: *const LZ4F_CDict,
    preferencesPtr: *const LZ4F_preferences_t,
) -> usize {
    let mut prefs: LZ4F_preferences_t = if !preferencesPtr.is_null() {
        *preferencesPtr
    } else {
        core::mem::zeroed()
    };
    let mut options: LZ4F_compressOptions_t = core::mem::zeroed();
    let dstStart = dstBuffer as *mut u8;
    let mut dstPtr = dstStart;
    let dstEnd = dstStart.wrapping_add(dstCapacity);

    if prefs.frameInfo.contentSize != 0 {
        prefs.frameInfo.contentSize = srcSize as u64;
    }
    prefs.frameInfo.blockSizeID = LZ4F_optimalBSID(prefs.frameInfo.blockSizeID, srcSize);
    prefs.autoFlush = 1;
    if srcSize <= LZ4F_getBlockSize(prefs.frameInfo.blockSizeID) {
        prefs.frameInfo.blockMode = LZ4F_blockIndependent;
    }
    options.stableSrc = 1;

    RETURN_ERROR_IF!(dstCapacity < LZ4F_compressFrameBound(srcSize, &prefs), ERROR_dstMaxSize_tooSmall);

    {
        let headerSize = LZ4F_compressBegin_usingCDict(cctx, dstBuffer, dstCapacity, cdict, &prefs);
        FORWARD_IF_ERROR!(headerSize);
        dstPtr = dstPtr.add(headerSize);
    }
    {
        let cSize = LZ4F_compressUpdate(cctx, dstPtr as *mut c_void, dstEnd as usize - dstPtr as usize, srcBuffer, srcSize, &options);
        FORWARD_IF_ERROR!(cSize);
        dstPtr = dstPtr.add(cSize);
    }
    {
        let tailSize = LZ4F_compressEnd(cctx, dstPtr as *mut c_void, dstEnd as usize - dstPtr as usize, &options);
        FORWARD_IF_ERROR!(tailSize);
        dstPtr = dstPtr.add(tailSize);
    }
    dstPtr as usize - dstStart as usize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_compressFrame(
    dstBuffer: *mut c_void,
    dstCapacity: usize,
    srcBuffer: *const c_void,
    srcSize: usize,
    preferencesPtr: *const LZ4F_preferences_t,
) -> usize {
    let mut cctx: LZ4F_cctx = core::mem::zeroed();
    let mut lz4ctx: LZ4_stream_t = core::mem::zeroed();
    let cctxPtr = &mut cctx as *mut LZ4F_cctx;

    cctx.version = LZ4F_VERSION;
    cctx.maxBufferSize = 5 * 1024 * 1024;
    if preferencesPtr.is_null() || (*preferencesPtr).compressionLevel < LZ4HC_CLEVEL_MIN {
        LZ4_initStream(&mut lz4ctx as *mut LZ4_stream_t as *mut c_void, core::mem::size_of::<LZ4_stream_t>());
        cctx.lz4CtxPtr = &mut lz4ctx as *mut LZ4_stream_t as *mut c_void;
        cctx.lz4CtxAlloc = 1;
        cctx.lz4CtxType = ctxFast;
    }

    let result = LZ4F_compressFrame_usingCDict(cctxPtr, dstBuffer, dstCapacity, srcBuffer, srcSize, ptr::null(), preferencesPtr);

    if !preferencesPtr.is_null() && (*preferencesPtr).compressionLevel >= LZ4HC_CLEVEL_MIN {
        LZ4F_free(cctx.lz4CtxPtr, cctx.cmem);
    }
    result
}

/* ===== CDict ===== */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_createCDict_advanced(
    cmem: LZ4F_CustomMem,
    dictBuffer: *const c_void,
    mut dictSize: usize,
) -> *mut LZ4F_CDict {
    let mut dictStart = dictBuffer as *const c_char;
    let cdict = LZ4F_malloc(core::mem::size_of::<LZ4F_CDict>(), cmem) as *mut LZ4F_CDict;
    if cdict.is_null() {
        return ptr::null_mut();
    }
    (*cdict).cmem = cmem;
    if dictSize > 64 * 1024 {
        dictStart = dictStart.add(dictSize - 64 * 1024);
        dictSize = 64 * 1024;
    }
    (*cdict).dictContent = LZ4F_malloc(dictSize, cmem);
    (*cdict).fastCtx = LZ4F_malloc(core::mem::size_of::<LZ4_stream_t>(), cmem) as *mut LZ4_stream_t;
    (*cdict).HCCtx = LZ4F_malloc(core::mem::size_of::<LZ4_streamHC_t>(), cmem) as *mut LZ4_streamHC_t;
    if (*cdict).dictContent.is_null() || (*cdict).fastCtx.is_null() || (*cdict).HCCtx.is_null() {
        LZ4F_freeCDict(cdict);
        return ptr::null_mut();
    }
    ptr::copy_nonoverlapping(dictStart as *const u8, (*cdict).dictContent as *mut u8, dictSize);
    LZ4_initStream((*cdict).fastCtx as *mut c_void, core::mem::size_of::<LZ4_stream_t>());
    LZ4_loadDictSlow((*cdict).fastCtx, (*cdict).dictContent as *const c_char, dictSize as c_int);
    LZ4_initStreamHC((*cdict).HCCtx as *mut c_void, core::mem::size_of::<LZ4_streamHC_t>());
    LZ4_setCompressionLevel((*cdict).HCCtx, LZ4HC_CLEVEL_DEFAULT);
    LZ4_loadDictHC((*cdict).HCCtx, (*cdict).dictContent as *const c_char, dictSize as c_int);
    cdict
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_createCDict(dictBuffer: *const c_void, dictSize: usize) -> *mut LZ4F_CDict {
    LZ4F_createCDict_advanced(LZ4F_defaultCMem, dictBuffer, dictSize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_freeCDict(cdict: *mut LZ4F_CDict) {
    if cdict.is_null() {
        return;
    }
    let cmem = (*cdict).cmem;
    LZ4F_free((*cdict).dictContent, cmem);
    LZ4F_free((*cdict).fastCtx as *mut c_void, cmem);
    LZ4F_free((*cdict).HCCtx as *mut c_void, cmem);
    LZ4F_free(cdict as *mut c_void, cmem);
}

/* ===== compression context ===== */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_createCompressionContext_advanced(
    customMem: LZ4F_CustomMem,
    version: c_uint,
) -> *mut LZ4F_cctx {
    let cctxPtr = LZ4F_calloc(core::mem::size_of::<LZ4F_cctx>(), customMem) as *mut LZ4F_cctx;
    if cctxPtr.is_null() {
        return ptr::null_mut();
    }
    (*cctxPtr).cmem = customMem;
    (*cctxPtr).version = version;
    (*cctxPtr).cStage = 0;
    cctxPtr
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_createCompressionContext(
    LZ4F_compressionContextPtr: *mut *mut LZ4F_cctx,
    version: c_uint,
) -> LZ4F_errorCode_t {
    RETURN_ERROR_IF!(LZ4F_compressionContextPtr.is_null(), ERROR_parameter_null);
    *LZ4F_compressionContextPtr = LZ4F_createCompressionContext_advanced(LZ4F_defaultCMem, version);
    RETURN_ERROR_IF!((*LZ4F_compressionContextPtr).is_null(), ERROR_allocation_failed);
    err::OK_NoError
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_freeCompressionContext(cctxPtr: *mut LZ4F_cctx) -> LZ4F_errorCode_t {
    if !cctxPtr.is_null() {
        let cmem = (*cctxPtr).cmem;
        LZ4F_free((*cctxPtr).lz4CtxPtr, cmem);
        LZ4F_free((*cctxPtr).tmpBuff as *mut c_void, cmem);
        LZ4F_free(cctxPtr as *mut c_void, cmem);
    }
    err::OK_NoError
}

unsafe fn LZ4F_initStream_internal(ctx: *mut c_void, cdict: *const LZ4F_CDict, level: c_int, blockMode: c_int) {
    if level < LZ4HC_CLEVEL_MIN {
        if !cdict.is_null() || blockMode == LZ4F_blockLinked {
            LZ4_resetStream_fast(ctx as *mut LZ4_stream_t);
            if !cdict.is_null() {
                LZ4_attach_dictionary(ctx as *mut LZ4_stream_t, (*cdict).fastCtx);
            }
        }
    } else {
        LZ4_resetStreamHC_fast(ctx as *mut LZ4_streamHC_t, level);
        if !cdict.is_null() {
            LZ4_attach_HC_dictionary(ctx as *mut LZ4_streamHC_t, (*cdict).HCCtx);
        }
    }
}

fn ctxTypeID_to_size(ctxTypeID: c_int) -> c_int {
    match ctxTypeID {
        1 => LZ4_sizeofState(),
        2 => LZ4_sizeofStateHC(),
        _ => 0,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_compressBegin_internal(
    cctx: *mut LZ4F_cctx,
    dstBuffer: *mut c_void,
    dstCapacity: usize,
    dictBuffer: *const c_void,
    dictSize: usize,
    cdict: *const LZ4F_CDict,
    mut preferencesPtr: *const LZ4F_preferences_t,
) -> usize {
    let prefNull = LZ4F_preferences_t::init();
    let dstStart = dstBuffer as *mut u8;
    let mut dstPtr = dstStart;
    let c = &mut *cctx;

    RETURN_ERROR_IF!(dstCapacity < maxFHSize, ERROR_dstMaxSize_tooSmall);
    if preferencesPtr.is_null() {
        preferencesPtr = &prefNull;
    }
    c.prefs = *preferencesPtr;

    {
        let ctxTypeID: u16 = if c.prefs.compressionLevel < LZ4HC_CLEVEL_MIN { 1 } else { 2 };
        let requiredSize = ctxTypeID_to_size(ctxTypeID as c_int);
        let allocatedSize = ctxTypeID_to_size(c.lz4CtxAlloc as c_int);
        if allocatedSize < requiredSize {
            LZ4F_free(c.lz4CtxPtr, c.cmem);
            if c.prefs.compressionLevel < LZ4HC_CLEVEL_MIN {
                c.lz4CtxPtr = LZ4F_malloc(core::mem::size_of::<LZ4_stream_t>(), c.cmem);
                if !c.lz4CtxPtr.is_null() {
                    LZ4_initStream(c.lz4CtxPtr, core::mem::size_of::<LZ4_stream_t>());
                }
            } else {
                c.lz4CtxPtr = LZ4F_malloc(core::mem::size_of::<LZ4_streamHC_t>(), c.cmem);
                if !c.lz4CtxPtr.is_null() {
                    LZ4_initStreamHC(c.lz4CtxPtr, core::mem::size_of::<LZ4_streamHC_t>());
                }
            }
            RETURN_ERROR_IF!(c.lz4CtxPtr.is_null(), ERROR_allocation_failed);
            c.lz4CtxAlloc = ctxTypeID;
            c.lz4CtxType = ctxTypeID;
        } else if c.lz4CtxType != ctxTypeID {
            if c.prefs.compressionLevel < LZ4HC_CLEVEL_MIN {
                LZ4_initStream(c.lz4CtxPtr, core::mem::size_of::<LZ4_stream_t>());
            } else {
                LZ4_initStreamHC(c.lz4CtxPtr, core::mem::size_of::<LZ4_streamHC_t>());
                LZ4_setCompressionLevel(c.lz4CtxPtr as *mut LZ4_streamHC_t, c.prefs.compressionLevel);
            }
            c.lz4CtxType = ctxTypeID;
        }
    }

    if c.prefs.frameInfo.blockSizeID == 0 {
        c.prefs.frameInfo.blockSizeID = LZ4F_BLOCKSIZEID_DEFAULT;
    }
    c.maxBlockSize = LZ4F_getBlockSize(c.prefs.frameInfo.blockSizeID);

    {
        let requiredBuffSize = if (*preferencesPtr).autoFlush != 0 {
            if c.prefs.frameInfo.blockMode == LZ4F_blockLinked { 64 * 1024 } else { 0 }
        } else {
            c.maxBlockSize + if c.prefs.frameInfo.blockMode == LZ4F_blockLinked { 128 * 1024 } else { 0 }
        };
        if c.maxBufferSize < requiredBuffSize {
            c.maxBufferSize = 0;
            LZ4F_free(c.tmpBuff as *mut c_void, c.cmem);
            c.tmpBuff = LZ4F_malloc(requiredBuffSize, c.cmem) as *mut u8;
            RETURN_ERROR_IF!(c.tmpBuff.is_null(), ERROR_allocation_failed);
            c.maxBufferSize = requiredBuffSize;
        }
    }
    c.tmpIn = c.tmpBuff;
    c.tmpInSize = 0;
    xxh32_reset(&mut c.xxh, 0);

    c.cdict = cdict;
    if c.prefs.frameInfo.blockMode == LZ4F_blockLinked {
        LZ4F_initStream_internal(c.lz4CtxPtr, cdict, c.prefs.compressionLevel, LZ4F_blockLinked);
    }
    if (*preferencesPtr).compressionLevel >= LZ4HC_CLEVEL_MIN {
        LZ4_favorDecompressionSpeed(c.lz4CtxPtr as *mut LZ4_streamHC_t, (*preferencesPtr).favorDecSpeed as c_int);
    }
    if !dictBuffer.is_null() {
        RETURN_ERROR_IF!(dictSize > c_int::MAX as usize, ERROR_parameter_invalid);
        if c.lz4CtxType == ctxFast {
            LZ4_loadDict(c.lz4CtxPtr as *mut LZ4_stream_t, dictBuffer as *const c_char, dictSize as c_int);
        } else {
            LZ4_loadDictHC(c.lz4CtxPtr as *mut LZ4_streamHC_t, dictBuffer as *const c_char, dictSize as c_int);
        }
    }

    LZ4F_writeLE32(dstPtr, LZ4F_MAGICNUMBER);
    dstPtr = dstPtr.add(4);
    {
        let headerStart = dstPtr;
        *dstPtr = (((1 & _2BITS) << 6)
            + (((c.prefs.frameInfo.blockMode as u32) & _1BIT) << 5)
            + (((c.prefs.frameInfo.blockChecksumFlag as u32) & _1BIT) << 4)
            + (((c.prefs.frameInfo.contentSize > 0) as u32) << 3)
            + (((c.prefs.frameInfo.contentChecksumFlag as u32) & _1BIT) << 2)
            + ((c.prefs.frameInfo.dictID > 0) as u32)) as u8;
        dstPtr = dstPtr.add(1);
        *dstPtr = (((c.prefs.frameInfo.blockSizeID as u32) & _3BITS) << 4) as u8;
        dstPtr = dstPtr.add(1);
        if c.prefs.frameInfo.contentSize != 0 {
            LZ4F_writeLE64(dstPtr, c.prefs.frameInfo.contentSize);
            dstPtr = dstPtr.add(8);
            c.totalInSize = 0;
        }
        if c.prefs.frameInfo.dictID != 0 {
            LZ4F_writeLE32(dstPtr, c.prefs.frameInfo.dictID);
            dstPtr = dstPtr.add(4);
        }
        *dstPtr = LZ4F_headerChecksum(headerStart, dstPtr as usize - headerStart as usize);
        dstPtr = dstPtr.add(1);
    }

    c.cStage = 1;
    dstPtr as usize - dstStart as usize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_compressBegin(
    cctx: *mut LZ4F_cctx,
    dstBuffer: *mut c_void,
    dstCapacity: usize,
    preferencesPtr: *const LZ4F_preferences_t,
) -> usize {
    LZ4F_compressBegin_internal(cctx, dstBuffer, dstCapacity, ptr::null(), 0, ptr::null(), preferencesPtr)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_compressBegin_usingDictOnce(
    cctx: *mut LZ4F_cctx,
    dstBuffer: *mut c_void,
    dstCapacity: usize,
    dict: *const c_void,
    dictSize: usize,
    preferencesPtr: *const LZ4F_preferences_t,
) -> usize {
    LZ4F_compressBegin_internal(cctx, dstBuffer, dstCapacity, dict, dictSize, ptr::null(), preferencesPtr)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_compressBegin_usingDict(
    cctx: *mut LZ4F_cctx,
    dstBuffer: *mut c_void,
    dstCapacity: usize,
    dict: *const c_void,
    dictSize: usize,
    preferencesPtr: *const LZ4F_preferences_t,
) -> usize {
    LZ4F_compressBegin_usingDictOnce(cctx, dstBuffer, dstCapacity, dict, dictSize, preferencesPtr)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_compressBegin_usingCDict(
    cctx: *mut LZ4F_cctx,
    dstBuffer: *mut c_void,
    dstCapacity: usize,
    cdict: *const LZ4F_CDict,
    preferencesPtr: *const LZ4F_preferences_t,
) -> usize {
    LZ4F_compressBegin_internal(cctx, dstBuffer, dstCapacity, ptr::null(), 0, cdict, preferencesPtr)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_compressBound(srcSize: usize, preferencesPtr: *const LZ4F_preferences_t) -> usize {
    if !preferencesPtr.is_null() && (*preferencesPtr).autoFlush != 0 {
        return LZ4F_compressBound_internal(srcSize, preferencesPtr, 0);
    }
    LZ4F_compressBound_internal(srcSize, preferencesPtr, usize::MAX)
}

type compressFunc_t =
    unsafe fn(*mut c_void, *const c_char, *mut c_char, c_int, c_int, c_int, *const LZ4F_CDict) -> c_int;

unsafe fn LZ4F_makeBlock(
    dst: *mut c_void,
    src: *const c_void,
    srcSize: usize,
    compress: compressFunc_t,
    lz4ctx: *mut c_void,
    level: c_int,
    cdict: *const LZ4F_CDict,
    crcFlag: c_int,
) -> usize {
    let cSizePtr = dst as *mut u8;
    let mut cSize = compress(
        lz4ctx,
        src as *const c_char,
        cSizePtr.add(BHSize) as *mut c_char,
        srcSize as c_int,
        (srcSize - 1) as c_int,
        level,
        cdict,
    ) as u32;

    if cSize == 0 || cSize >= srcSize as u32 {
        cSize = srcSize as u32;
        LZ4F_writeLE32(cSizePtr, cSize | LZ4F_BLOCKUNCOMPRESSED_FLAG);
        ptr::copy_nonoverlapping(src as *const u8, cSizePtr.add(BHSize), srcSize);
    } else {
        LZ4F_writeLE32(cSizePtr, cSize);
    }
    if crcFlag != 0 {
        let crc32 = xxh32(cSizePtr.add(BHSize), cSize as usize, 0);
        LZ4F_writeLE32(cSizePtr.add(BHSize + cSize as usize), crc32);
    }
    // C: `BHSize + cSize + ((U32)crcFlag)*BFSize` — an explicit U32 cast, so an
    // out-of-range flag zero-extends (it is NOT masked to 0/1).
    BHSize + cSize as usize + (crcFlag as u32 as usize) * BFSize
}

unsafe fn LZ4F_compressBlock(ctx: *mut c_void, src: *const c_char, dst: *mut c_char, srcSize: c_int, dstCapacity: c_int, level: c_int, cdict: *const LZ4F_CDict) -> c_int {
    let acceleration = if level < 0 { -level + 1 } else { 1 };
    LZ4F_initStream_internal(ctx, cdict, level, LZ4F_blockIndependent);
    if !cdict.is_null() {
        LZ4_compress_fast_continue(ctx as *mut LZ4_stream_t, src, dst, srcSize, dstCapacity, acceleration)
    } else {
        LZ4_compress_fast_extState_fastReset(ctx, src, dst, srcSize, dstCapacity, acceleration)
    }
}
unsafe fn LZ4F_compressBlock_continue(ctx: *mut c_void, src: *const c_char, dst: *mut c_char, srcSize: c_int, dstCapacity: c_int, level: c_int, _cdict: *const LZ4F_CDict) -> c_int {
    let acceleration = if level < 0 { -level + 1 } else { 1 };
    LZ4_compress_fast_continue(ctx as *mut LZ4_stream_t, src, dst, srcSize, dstCapacity, acceleration)
}
unsafe fn LZ4F_compressBlockHC(ctx: *mut c_void, src: *const c_char, dst: *mut c_char, srcSize: c_int, dstCapacity: c_int, level: c_int, cdict: *const LZ4F_CDict) -> c_int {
    LZ4F_initStream_internal(ctx, cdict, level, LZ4F_blockIndependent);
    if !cdict.is_null() {
        LZ4_compress_HC_continue(ctx as *mut LZ4_streamHC_t, src, dst, srcSize, dstCapacity)
    } else {
        LZ4_compress_HC_extStateHC_fastReset(ctx, src, dst, srcSize, dstCapacity, level)
    }
}
unsafe fn LZ4F_compressBlockHC_continue(ctx: *mut c_void, src: *const c_char, dst: *mut c_char, srcSize: c_int, dstCapacity: c_int, _level: c_int, _cdict: *const LZ4F_CDict) -> c_int {
    LZ4_compress_HC_continue(ctx as *mut LZ4_streamHC_t, src, dst, srcSize, dstCapacity)
}
unsafe fn LZ4F_doNotCompressBlock(_ctx: *mut c_void, _src: *const c_char, _dst: *mut c_char, _srcSize: c_int, _dstCapacity: c_int, _level: c_int, _cdict: *const LZ4F_CDict) -> c_int {
    0
}

fn LZ4F_selectCompression(blockMode: c_int, level: c_int, compressMode: LZ4F_BlockCompressMode_e) -> compressFunc_t {
    if compressMode == LZ4B_UNCOMPRESSED {
        return LZ4F_doNotCompressBlock;
    }
    if level < LZ4HC_CLEVEL_MIN {
        if blockMode == LZ4F_blockIndependent {
            return LZ4F_compressBlock;
        }
        return LZ4F_compressBlock_continue;
    }
    if blockMode == LZ4F_blockIndependent {
        return LZ4F_compressBlockHC;
    }
    LZ4F_compressBlockHC_continue
}

unsafe fn LZ4F_localSaveDict(c: &mut LZ4F_cctx) -> c_int {
    if c.prefs.compressionLevel < LZ4HC_CLEVEL_MIN {
        LZ4_saveDict(c.lz4CtxPtr as *mut LZ4_stream_t, c.tmpBuff as *mut c_char, 64 * 1024)
    } else {
        LZ4_saveDictHC(c.lz4CtxPtr as *mut LZ4_streamHC_t, c.tmpBuff as *mut c_char, 64 * 1024)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum LZ4F_lastBlockStatus {
    notDone,
    fromTmpBuffer,
    fromSrcBuffer,
}
use LZ4F_lastBlockStatus::*;

static k_cOptionsNull: LZ4F_compressOptions_t = LZ4F_compressOptions_t { stableSrc: 0, reserved: [0, 0, 0] };

unsafe fn LZ4F_compressUpdateImpl(
    cctxPtr: *mut LZ4F_cctx,
    dstBuffer: *mut c_void,
    dstCapacity: usize,
    srcBuffer: *const c_void,
    srcSize: usize,
    mut compressOptionsPtr: *const LZ4F_compressOptions_t,
    blockCompression: LZ4F_BlockCompressMode_e,
) -> usize {
    let c = &mut *cctxPtr;
    let blockSize = c.maxBlockSize;
    let mut srcPtr = srcBuffer as *const u8;
    let srcEnd = srcPtr.wrapping_add(srcSize);
    let dstStart = dstBuffer as *mut u8;
    let mut dstPtr = dstStart;
    let mut lastBlockCompressed = notDone;
    let compress = LZ4F_selectCompression(c.prefs.frameInfo.blockMode, c.prefs.compressionLevel, blockCompression);

    RETURN_ERROR_IF!(c.cStage != 1, ERROR_compressionState_uninitialized);
    if dstCapacity < LZ4F_compressBound_internal(srcSize, &c.prefs, c.tmpInSize) {
        RETURN_ERROR!(ERROR_dstMaxSize_tooSmall);
    }
    if blockCompression == LZ4B_UNCOMPRESSED && dstCapacity < srcSize {
        RETURN_ERROR!(ERROR_dstMaxSize_tooSmall);
    }

    if c.blockCompressMode != blockCompression {
        let bytesWritten = LZ4F_flush(cctxPtr, dstBuffer, dstCapacity, compressOptionsPtr);
        FORWARD_IF_ERROR!(bytesWritten);
        dstPtr = dstPtr.add(bytesWritten);
        c.blockCompressMode = blockCompression;
    }

    if compressOptionsPtr.is_null() {
        compressOptionsPtr = &k_cOptionsNull;
    }

    if c.tmpInSize > 0 {
        let sizeToCopy = blockSize - c.tmpInSize;
        if sizeToCopy > srcSize {
            ptr::copy_nonoverlapping(srcBuffer as *const u8, c.tmpIn.add(c.tmpInSize), srcSize);
            srcPtr = srcEnd;
            c.tmpInSize += srcSize;
        } else {
            lastBlockCompressed = fromTmpBuffer;
            ptr::copy_nonoverlapping(srcBuffer as *const u8, c.tmpIn.add(c.tmpInSize), sizeToCopy);
            srcPtr = srcPtr.add(sizeToCopy);
            dstPtr = dstPtr.add(LZ4F_makeBlock(
                dstPtr as *mut c_void, c.tmpIn as *const c_void, blockSize, compress, c.lz4CtxPtr,
                c.prefs.compressionLevel, c.cdict, c.prefs.frameInfo.blockChecksumFlag,
            ));
            if c.prefs.frameInfo.blockMode == LZ4F_blockLinked {
                c.tmpIn = c.tmpIn.add(blockSize);
            }
            c.tmpInSize = 0;
        }
    }

    while (srcEnd as usize - srcPtr as usize) >= blockSize {
        lastBlockCompressed = fromSrcBuffer;
        dstPtr = dstPtr.add(LZ4F_makeBlock(
            dstPtr as *mut c_void, srcPtr as *const c_void, blockSize, compress, c.lz4CtxPtr,
            c.prefs.compressionLevel, c.cdict, c.prefs.frameInfo.blockChecksumFlag,
        ));
        srcPtr = srcPtr.add(blockSize);
    }

    if (c.prefs.autoFlush != 0) && (srcPtr < srcEnd) {
        lastBlockCompressed = fromSrcBuffer;
        dstPtr = dstPtr.add(LZ4F_makeBlock(
            dstPtr as *mut c_void, srcPtr as *const c_void, srcEnd as usize - srcPtr as usize, compress,
            c.lz4CtxPtr, c.prefs.compressionLevel, c.cdict, c.prefs.frameInfo.blockChecksumFlag,
        ));
        srcPtr = srcEnd;
    }

    if (c.prefs.frameInfo.blockMode == LZ4F_blockLinked) && (lastBlockCompressed == fromSrcBuffer) {
        if (*compressOptionsPtr).stableSrc != 0 {
            c.tmpIn = c.tmpBuff;
        } else {
            let realDictSize = LZ4F_localSaveDict(c);
            c.tmpIn = c.tmpBuff.add(realDictSize as usize);
        }
    }

    if (c.prefs.autoFlush == 0) && (c.tmpIn.wrapping_add(blockSize) > c.tmpBuff.wrapping_add(c.maxBufferSize)) {
        let realDictSize = LZ4F_localSaveDict(c);
        c.tmpIn = c.tmpBuff.add(realDictSize as usize);
    }

    if srcPtr < srcEnd {
        let sizeToCopy = srcEnd as usize - srcPtr as usize;
        ptr::copy_nonoverlapping(srcPtr, c.tmpIn, sizeToCopy);
        c.tmpInSize = sizeToCopy;
    }

    if c.prefs.frameInfo.contentChecksumFlag == LZ4F_contentChecksumEnabled {
        xxh32_update(&mut c.xxh, srcBuffer as *const u8, srcSize);
    }

    c.totalInSize += srcSize as u64;
    dstPtr as usize - dstStart as usize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_compressUpdate(
    cctxPtr: *mut LZ4F_cctx,
    dstBuffer: *mut c_void,
    dstCapacity: usize,
    srcBuffer: *const c_void,
    srcSize: usize,
    compressOptionsPtr: *const LZ4F_compressOptions_t,
) -> usize {
    LZ4F_compressUpdateImpl(cctxPtr, dstBuffer, dstCapacity, srcBuffer, srcSize, compressOptionsPtr, LZ4B_COMPRESSED)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_uncompressedUpdate(
    cctxPtr: *mut LZ4F_cctx,
    dstBuffer: *mut c_void,
    dstCapacity: usize,
    srcBuffer: *const c_void,
    srcSize: usize,
    compressOptionsPtr: *const LZ4F_compressOptions_t,
) -> usize {
    LZ4F_compressUpdateImpl(cctxPtr, dstBuffer, dstCapacity, srcBuffer, srcSize, compressOptionsPtr, LZ4B_UNCOMPRESSED)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_flush(
    cctxPtr: *mut LZ4F_cctx,
    dstBuffer: *mut c_void,
    dstCapacity: usize,
    _compressOptionsPtr: *const LZ4F_compressOptions_t,
) -> usize {
    let c = &mut *cctxPtr;
    let dstStart = dstBuffer as *mut u8;
    let mut dstPtr = dstStart;

    if c.tmpInSize == 0 {
        return 0;
    }
    RETURN_ERROR_IF!(c.cStage != 1, ERROR_compressionState_uninitialized);
    RETURN_ERROR_IF!(dstCapacity < (c.tmpInSize + BHSize + BFSize), ERROR_dstMaxSize_tooSmall);

    let compress = LZ4F_selectCompression(c.prefs.frameInfo.blockMode, c.prefs.compressionLevel, c.blockCompressMode);
    dstPtr = dstPtr.add(LZ4F_makeBlock(
        dstPtr as *mut c_void, c.tmpIn as *const c_void, c.tmpInSize, compress, c.lz4CtxPtr,
        c.prefs.compressionLevel, c.cdict, c.prefs.frameInfo.blockChecksumFlag,
    ));

    if c.prefs.frameInfo.blockMode == LZ4F_blockLinked {
        c.tmpIn = c.tmpIn.add(c.tmpInSize);
    }
    c.tmpInSize = 0;

    if c.tmpIn.wrapping_add(c.maxBlockSize) > c.tmpBuff.wrapping_add(c.maxBufferSize) {
        let realDictSize = LZ4F_localSaveDict(c);
        c.tmpIn = c.tmpBuff.add(realDictSize as usize);
    }

    dstPtr as usize - dstStart as usize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_compressEnd(
    cctxPtr: *mut LZ4F_cctx,
    dstBuffer: *mut c_void,
    mut dstCapacity: usize,
    compressOptionsPtr: *const LZ4F_compressOptions_t,
) -> usize {
    let c = &mut *cctxPtr;
    let dstStart = dstBuffer as *mut u8;
    let mut dstPtr = dstStart;

    let flushSize = LZ4F_flush(cctxPtr, dstBuffer, dstCapacity, compressOptionsPtr);
    FORWARD_IF_ERROR!(flushSize);
    dstPtr = dstPtr.add(flushSize);
    dstCapacity -= flushSize;

    RETURN_ERROR_IF!(dstCapacity < 4, ERROR_dstMaxSize_tooSmall);
    LZ4F_writeLE32(dstPtr, 0);
    dstPtr = dstPtr.add(4);

    if c.prefs.frameInfo.contentChecksumFlag == LZ4F_contentChecksumEnabled {
        let xxh = xxh32_digest(&c.xxh);
        RETURN_ERROR_IF!(dstCapacity < 8, ERROR_dstMaxSize_tooSmall);
        LZ4F_writeLE32(dstPtr, xxh);
        dstPtr = dstPtr.add(4);
    }

    c.cStage = 0;

    if c.prefs.frameInfo.contentSize != 0 {
        if c.prefs.frameInfo.contentSize != c.totalInSize {
            RETURN_ERROR!(ERROR_frameSize_wrong);
        }
    }
    dstPtr as usize - dstStart as usize
}

/* ===== Decompression ===== */
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u32)]
enum dStage_t {
    dstage_getFrameHeader = 0,
    dstage_storeFrameHeader,
    dstage_init,
    dstage_getBlockHeader,
    dstage_storeBlockHeader,
    dstage_copyDirect,
    dstage_getBlockChecksum,
    dstage_getCBlock,
    dstage_storeCBlock,
    dstage_flushOut,
    dstage_getSuffix,
    dstage_storeSuffix,
    dstage_getSFrameSize,
    dstage_storeSFrameSize,
    dstage_skipSkippable,
}
use dStage_t::*;

#[repr(C)]
pub struct LZ4F_dctx {
    cmem: LZ4F_CustomMem,
    frameInfo: LZ4F_frameInfo_t,
    version: u32,
    dStage: dStage_t,
    frameRemainingSize: u64,
    maxBlockSize: usize,
    maxBufferSize: usize,
    tmpIn: *mut u8,
    tmpInSize: usize,
    tmpInTarget: usize,
    tmpOutBuffer: *mut u8,
    dict: *const u8,
    dictSize: usize,
    tmpOut: *mut u8,
    tmpOutSize: usize,
    tmpOutStart: usize,
    xxh: XXH32_state_t,
    blockChecksum: XXH32_state_t,
    skipChecksum: c_int,
    header: [u8; LZ4F_HEADER_SIZE_MAX],
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_createDecompressionContext_advanced(
    customMem: LZ4F_CustomMem,
    version: c_uint,
) -> *mut LZ4F_dctx {
    let dctx = LZ4F_calloc(core::mem::size_of::<LZ4F_dctx>(), customMem) as *mut LZ4F_dctx;
    if dctx.is_null() {
        return ptr::null_mut();
    }
    (*dctx).cmem = customMem;
    (*dctx).version = version;
    dctx
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_createDecompressionContext(
    LZ4F_decompressionContextPtr: *mut *mut LZ4F_dctx,
    versionNumber: c_uint,
) -> LZ4F_errorCode_t {
    RETURN_ERROR_IF!(LZ4F_decompressionContextPtr.is_null(), ERROR_parameter_null);
    *LZ4F_decompressionContextPtr = LZ4F_createDecompressionContext_advanced(LZ4F_defaultCMem, versionNumber);
    if (*LZ4F_decompressionContextPtr).is_null() {
        RETURN_ERROR!(ERROR_allocation_failed);
    }
    err::OK_NoError
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_freeDecompressionContext(dctx: *mut LZ4F_dctx) -> LZ4F_errorCode_t {
    let mut result: LZ4F_errorCode_t = err::OK_NoError;
    if !dctx.is_null() {
        result = (*dctx).dStage as usize;
        let cmem = (*dctx).cmem;
        LZ4F_free((*dctx).tmpIn as *mut c_void, cmem);
        LZ4F_free((*dctx).tmpOutBuffer as *mut c_void, cmem);
        LZ4F_free(dctx as *mut c_void, cmem);
    }
    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_resetDecompressionContext(dctx: *mut LZ4F_dctx) {
    (*dctx).dStage = dstage_getFrameHeader;
    (*dctx).dict = ptr::null();
    (*dctx).dictSize = 0;
    (*dctx).skipChecksum = 0;
    (*dctx).frameRemainingSize = 0;
}

unsafe fn LZ4F_decodeHeader(dctx: *mut LZ4F_dctx, src: *const c_void, srcSize: usize) -> usize {
    let d = &mut *dctx;
    let (blockMode, blockChecksumFlag, contentSizeFlag, contentChecksumFlag, dictIDFlag, blockSizeID);
    let frameHeaderSize;
    let srcPtr = src as *const u8;

    RETURN_ERROR_IF!(srcSize < minFHSize, ERROR_frameHeader_incomplete);
    ptr::write_bytes(&mut d.frameInfo as *mut LZ4F_frameInfo_t as *mut u8, 0, core::mem::size_of::<LZ4F_frameInfo_t>());

    if (LZ4F_readLE32(srcPtr) & 0xFFFFFFF0) == LZ4F_MAGIC_SKIPPABLE_START {
        d.frameInfo.frameType = LZ4F_skippableFrame;
        if src == d.header.as_ptr() as *const c_void {
            d.tmpInSize = srcSize;
            d.tmpInTarget = 8;
            d.dStage = dstage_storeSFrameSize;
            return srcSize;
        } else {
            d.dStage = dstage_getSFrameSize;
            return 4;
        }
    }

    if LZ4F_readLE32(srcPtr) != LZ4F_MAGICNUMBER {
        RETURN_ERROR!(ERROR_frameType_unknown);
    }
    d.frameInfo.frameType = LZ4F_frame;

    {
        let FLG = *srcPtr.add(4) as u32;
        let version = (FLG >> 6) & _2BITS;
        blockChecksumFlag = (FLG >> 4) & _1BIT;
        blockMode = (FLG >> 5) & _1BIT;
        contentSizeFlag = (FLG >> 3) & _1BIT;
        contentChecksumFlag = (FLG >> 2) & _1BIT;
        dictIDFlag = FLG & _1BIT;
        if ((FLG >> 1) & _1BIT) != 0 {
            RETURN_ERROR!(ERROR_reservedFlag_set);
        }
        if version != 1 {
            RETURN_ERROR!(ERROR_headerVersion_wrong);
        }
    }

    frameHeaderSize = minFHSize + (if contentSizeFlag != 0 { 8 } else { 0 }) + (if dictIDFlag != 0 { 4 } else { 0 });

    if srcSize < frameHeaderSize {
        if srcPtr != d.header.as_ptr() {
            ptr::copy_nonoverlapping(srcPtr, d.header.as_mut_ptr(), srcSize);
        }
        d.tmpInSize = srcSize;
        d.tmpInTarget = frameHeaderSize;
        d.dStage = dstage_storeFrameHeader;
        return srcSize;
    }

    {
        let BD = *srcPtr.add(5) as u32;
        blockSizeID = (BD >> 4) & _3BITS;
        if ((BD >> 7) & _1BIT) != 0 {
            RETURN_ERROR!(ERROR_reservedFlag_set);
        }
        if (blockSizeID as c_int) < 4 {
            RETURN_ERROR!(ERROR_maxBlockSize_invalid);
        }
        if ((BD >> 0) & _4BITS) != 0 {
            RETURN_ERROR!(ERROR_reservedFlag_set);
        }
    }

    {
        let HC = LZ4F_headerChecksum(srcPtr.add(4), frameHeaderSize - 5);
        RETURN_ERROR_IF!(HC != *srcPtr.add(frameHeaderSize - 1), ERROR_headerChecksum_invalid);
    }

    d.frameInfo.blockMode = blockMode as c_int;
    d.frameInfo.blockChecksumFlag = blockChecksumFlag as c_int;
    d.frameInfo.contentChecksumFlag = contentChecksumFlag as c_int;
    d.frameInfo.blockSizeID = blockSizeID as LZ4F_blockSizeID_t;
    d.maxBlockSize = LZ4F_getBlockSize(blockSizeID as LZ4F_blockSizeID_t);
    if contentSizeFlag != 0 {
        d.frameInfo.contentSize = LZ4F_readLE64(srcPtr.add(6));
        d.frameRemainingSize = d.frameInfo.contentSize;
    }
    if dictIDFlag != 0 {
        d.frameInfo.dictID = LZ4F_readLE32(srcPtr.add(frameHeaderSize - 5));
    }

    d.dStage = dstage_init;
    frameHeaderSize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_headerSize(src: *const c_void, srcSize: usize) -> usize {
    RETURN_ERROR_IF!(src.is_null(), ERROR_srcPtr_wrong);
    if srcSize < LZ4F_MIN_SIZE_TO_KNOW_HEADER_LENGTH {
        RETURN_ERROR!(ERROR_frameHeader_incomplete);
    }
    if (LZ4F_readLE32(src as *const u8) & 0xFFFFFFF0) == LZ4F_MAGIC_SKIPPABLE_START {
        return 8;
    }
    if LZ4F_readLE32(src as *const u8) != LZ4F_MAGICNUMBER {
        RETURN_ERROR!(ERROR_frameType_unknown);
    }
    {
        let FLG = *(src as *const u8).add(4) as u32;
        let contentSizeFlag = (FLG >> 3) & _1BIT;
        let dictIDFlag = FLG & _1BIT;
        minFHSize + (if contentSizeFlag != 0 { 8 } else { 0 }) + (if dictIDFlag != 0 { 4 } else { 0 })
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_getFrameInfo(
    dctx: *mut LZ4F_dctx,
    frameInfoPtr: *mut LZ4F_frameInfo_t,
    srcBuffer: *const c_void,
    srcSizePtr: *mut usize,
) -> LZ4F_errorCode_t {
    let d = &mut *dctx;
    if d.dStage > dstage_storeFrameHeader {
        let mut o: usize = 0;
        let mut i: usize = 0;
        *srcSizePtr = 0;
        *frameInfoPtr = d.frameInfo;
        return LZ4F_decompress(dctx, ptr::null_mut(), &mut o, ptr::null(), &mut i, ptr::null());
    } else {
        if d.dStage == dstage_storeFrameHeader {
            *srcSizePtr = 0;
            RETURN_ERROR!(ERROR_frameDecoding_alreadyStarted);
        } else {
            let hSize = LZ4F_headerSize(srcBuffer, *srcSizePtr);
            if LZ4F_isError_internal(hSize) {
                *srcSizePtr = 0;
                return hSize;
            }
            if *srcSizePtr < hSize {
                *srcSizePtr = 0;
                RETURN_ERROR!(ERROR_frameHeader_incomplete);
            }
            let mut decodeResult = LZ4F_decodeHeader(dctx, srcBuffer, hSize);
            if LZ4F_isError_internal(decodeResult) {
                *srcSizePtr = 0;
            } else {
                *srcSizePtr = decodeResult;
                decodeResult = BHSize;
            }
            *frameInfoPtr = d.frameInfo;
            return decodeResult;
        }
    }
}

unsafe fn LZ4F_updateDict(
    dctx: *mut LZ4F_dctx,
    dstPtr: *const u8,
    dstSize: usize,
    dstBufferStart: *const u8,
    withinTmp: c_uint,
) {
    let d = &mut *dctx;
    if d.dictSize == 0 {
        d.dict = dstPtr;
    }
    if d.dict.wrapping_add(d.dictSize) == dstPtr {
        d.dictSize += dstSize;
        return;
    }
    if (dstPtr as usize - dstBufferStart as usize) + dstSize >= 64 * 1024 {
        d.dict = dstBufferStart;
        d.dictSize = (dstPtr as usize - dstBufferStart as usize) + dstSize;
        return;
    }
    if withinTmp != 0 && (d.dict == d.tmpOutBuffer) {
        d.dictSize += dstSize;
        return;
    }
    if withinTmp != 0 {
        let preserveSize = d.tmpOut as usize - d.tmpOutBuffer as usize;
        // C: `size_t copySize = 64 KB - dctx->tmpOutSize;` — deliberately allowed
        // to wrap around; the `tmpOutSize > 64 KB` test below then zeroes it.
        let mut copySize = (64 * 1024usize).wrapping_sub(d.tmpOutSize);
        let oldDictEnd = d.dict.wrapping_add(d.dictSize).wrapping_sub(d.tmpOutStart);
        if d.tmpOutSize > 64 * 1024 {
            copySize = 0;
        }
        if copySize > preserveSize {
            copySize = preserveSize;
        }
        ptr::copy_nonoverlapping(oldDictEnd.wrapping_sub(copySize), d.tmpOutBuffer.add(preserveSize - copySize), copySize);
        d.dict = d.tmpOutBuffer;
        d.dictSize = preserveSize + d.tmpOutStart + dstSize;
        return;
    }
    if d.dict == d.tmpOutBuffer {
        if d.dictSize + dstSize > d.maxBufferSize {
            let preserveSize = 64 * 1024 - dstSize;
            ptr::copy_nonoverlapping(d.dict.wrapping_add(d.dictSize).wrapping_sub(preserveSize), d.tmpOutBuffer, preserveSize);
            d.dictSize = preserveSize;
        }
        ptr::copy_nonoverlapping(dstPtr, d.tmpOutBuffer.add(d.dictSize), dstSize);
        d.dictSize += dstSize;
        return;
    }
    {
        let mut preserveSize = 64 * 1024 - dstSize;
        if preserveSize > d.dictSize {
            preserveSize = d.dictSize;
        }
        ptr::copy_nonoverlapping(d.dict.wrapping_add(d.dictSize).wrapping_sub(preserveSize), d.tmpOutBuffer, preserveSize);
        ptr::copy_nonoverlapping(dstPtr, d.tmpOutBuffer.add(preserveSize), dstSize);
        d.dict = d.tmpOutBuffer;
        d.dictSize = preserveSize + dstSize;
    }
}

enum SmRes {
    Continue,
    Return(usize),
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_decompress(
    dctx: *mut LZ4F_dctx,
    dstBuffer: *mut c_void,
    dstSizePtr: *mut usize,
    srcBuffer: *const c_void,
    srcSizePtr: *mut usize,
    decompressOptionsPtr: *const LZ4F_decompressOptions_t,
) -> usize {
    let d = &mut *dctx;
    let optionsNull: LZ4F_decompressOptions_t = core::mem::zeroed();
    let srcStart = srcBuffer as *const u8;
    let srcEnd = srcStart.wrapping_add(*srcSizePtr);
    let mut srcPtr = srcStart;
    let dstStart = dstBuffer as *mut u8;
    let dstEnd = if dstStart.is_null() {
        ptr::null_mut()
    } else {
        dstStart.wrapping_add(*dstSizePtr)
    };
    let mut dstPtr = dstStart;
    let mut selectedIn: *const u8 = ptr::null();
    let mut doAnotherStage = 1i32;
    let mut nextSrcSizeHint: usize = 1;

    let mut decompressOptionsPtr = decompressOptionsPtr;
    if decompressOptionsPtr.is_null() {
        decompressOptionsPtr = &optionsNull;
    }
    *srcSizePtr = 0;
    *dstSizePtr = 0;
    d.skipChecksum |= ((*decompressOptionsPtr).skipChecksums != 0) as c_int;

    'sm: loop {
        if doAnotherStage == 0 {
            break 'sm;
        }
        let res: SmRes = match d.dStage {
            dstage_getFrameHeader => {
                if (srcEnd as usize - srcPtr as usize) >= maxFHSize {
                    let hSize = LZ4F_decodeHeader(dctx, srcPtr as *const c_void, srcEnd as usize - srcPtr as usize);
                    if LZ4F_isError_internal(hSize) {
                        return hSize;
                    }
                    srcPtr = srcPtr.add(hSize);
                    SmRes::Continue
                } else {
                    d.tmpInSize = 0;
                    if (srcEnd as usize - srcPtr as usize) == 0 {
                        return minFHSize;
                    }
                    d.tmpInTarget = minFHSize;
                    d.dStage = dstage_storeFrameHeader;
                    sm_store_frame_header(dctx, &mut srcPtr, srcEnd, &mut nextSrcSizeHint, &mut doAnotherStage)
                }
            }
            dstage_storeFrameHeader => sm_store_frame_header(dctx, &mut srcPtr, srcEnd, &mut nextSrcSizeHint, &mut doAnotherStage),
            dstage_init => {
                if d.frameInfo.contentChecksumFlag != 0 {
                    xxh32_reset(&mut d.xxh, 0);
                }
                {
                    let bufferNeeded = d.maxBlockSize + if d.frameInfo.blockMode == LZ4F_blockLinked { 128 * 1024 } else { 0 };
                    if bufferNeeded > d.maxBufferSize {
                        d.maxBufferSize = 0;
                        LZ4F_free(d.tmpIn as *mut c_void, d.cmem);
                        d.tmpIn = LZ4F_malloc(d.maxBlockSize + BFSize, d.cmem) as *mut u8;
                        RETURN_ERROR_IF!(d.tmpIn.is_null(), ERROR_allocation_failed);
                        LZ4F_free(d.tmpOutBuffer as *mut c_void, d.cmem);
                        d.tmpOutBuffer = LZ4F_malloc(bufferNeeded, d.cmem) as *mut u8;
                        RETURN_ERROR_IF!(d.tmpOutBuffer.is_null(), ERROR_allocation_failed);
                        d.maxBufferSize = bufferNeeded;
                    }
                }
                d.tmpInSize = 0;
                d.tmpInTarget = 0;
                d.tmpOut = d.tmpOutBuffer;
                d.tmpOutStart = 0;
                d.tmpOutSize = 0;
                d.dStage = dstage_getBlockHeader;
                sm_block_header(dctx, &mut srcPtr, srcEnd, &mut selectedIn, dstPtr, dstEnd, &mut nextSrcSizeHint, &mut doAnotherStage)
            }
            dstage_getBlockHeader | dstage_storeBlockHeader => {
                sm_block_header(dctx, &mut srcPtr, srcEnd, &mut selectedIn, dstPtr, dstEnd, &mut nextSrcSizeHint, &mut doAnotherStage)
            }
            dstage_copyDirect => {
                sm_copy_direct(dctx, &mut srcPtr, srcEnd, &mut dstPtr, dstEnd, dstStart, &mut nextSrcSizeHint, &mut doAnotherStage)
            }
            dstage_getBlockChecksum => sm_block_checksum(dctx, &mut srcPtr, srcEnd, &mut doAnotherStage),
            dstage_getCBlock | dstage_storeCBlock => {
                sm_cblock(dctx, &mut srcPtr, srcEnd, &mut selectedIn, &mut dstPtr, dstEnd, dstStart, &mut nextSrcSizeHint, &mut doAnotherStage)
            }
            dstage_flushOut => sm_flush_out(dctx, &mut dstPtr, dstEnd, dstStart, &mut nextSrcSizeHint, &mut doAnotherStage),
            dstage_getSuffix | dstage_storeSuffix => sm_suffix(dctx, &mut srcPtr, srcEnd, &mut selectedIn, &mut nextSrcSizeHint, &mut doAnotherStage),
            dstage_getSFrameSize | dstage_storeSFrameSize => sm_sframesize(dctx, &mut srcPtr, srcEnd, &mut selectedIn, &mut nextSrcSizeHint, &mut doAnotherStage),
            dstage_skipSkippable => sm_skip_skippable(dctx, &mut srcPtr, srcEnd, &mut nextSrcSizeHint, &mut doAnotherStage),
        };
        match res {
            SmRes::Continue => continue 'sm,
            SmRes::Return(e) => return e,
        }
    }

    if (d.frameInfo.blockMode == LZ4F_blockLinked)
        && (d.dict != d.tmpOutBuffer)
        && (!d.dict.is_null())
        && ((*decompressOptionsPtr).stableDst == 0)
        && (((d.dStage as u32).wrapping_sub(2)) < ((dstage_getSuffix as u32) - 2))
    {
        if d.dStage == dstage_flushOut {
            let preserveSize = d.tmpOut as usize - d.tmpOutBuffer as usize;
            // C: `size_t copySize = 64 KB - dctx->tmpOutSize;` — wraps on purpose,
            // the `tmpOutSize > 64 KB` test below then zeroes it.
            let mut copySize = (64 * 1024usize).wrapping_sub(d.tmpOutSize);
            let oldDictEnd = d.dict.wrapping_add(d.dictSize).wrapping_sub(d.tmpOutStart);
            if d.tmpOutSize > 64 * 1024 {
                copySize = 0;
            }
            if copySize > preserveSize {
                copySize = preserveSize;
            }
            ptr::copy_nonoverlapping(oldDictEnd.wrapping_sub(copySize), d.tmpOutBuffer.add(preserveSize - copySize), copySize);
            d.dict = d.tmpOutBuffer;
            d.dictSize = preserveSize + d.tmpOutStart;
        } else {
            let oldDictEnd = d.dict.wrapping_add(d.dictSize);
            let newDictSize = MIN(d.dictSize, 64 * 1024);
            ptr::copy_nonoverlapping(oldDictEnd.wrapping_sub(newDictSize), d.tmpOutBuffer, newDictSize);
            d.dict = d.tmpOutBuffer;
            d.dictSize = newDictSize;
            d.tmpOut = d.tmpOutBuffer.add(newDictSize);
        }
    }

    *srcSizePtr = srcPtr as usize - srcStart as usize;
    *dstSizePtr = dstPtr as usize - dstStart as usize;
    nextSrcSizeHint
}

// storeFrameHeader: returns SmRes. On "break" (decode success) -> Continue; on stall -> Continue (doAnother=0); on error -> Return.
unsafe fn sm_store_frame_header(
    dctx: *mut LZ4F_dctx,
    srcPtr: &mut *const u8,
    srcEnd: *const u8,
    nextSrcSizeHint: &mut usize,
    doAnotherStage: &mut i32,
) -> SmRes {
    let d = &mut *dctx;
    let sizeToCopy = MIN(d.tmpInTarget - d.tmpInSize, srcEnd as usize - *srcPtr as usize);
    ptr::copy_nonoverlapping(*srcPtr, d.header.as_mut_ptr().add(d.tmpInSize), sizeToCopy);
    d.tmpInSize += sizeToCopy;
    *srcPtr = (*srcPtr).add(sizeToCopy);
    if d.tmpInSize < d.tmpInTarget {
        *nextSrcSizeHint = (d.tmpInTarget - d.tmpInSize) + BHSize;
        *doAnotherStage = 0;
        return SmRes::Continue;
    }
    let r = LZ4F_decodeHeader(dctx, d.header.as_ptr() as *const c_void, d.tmpInTarget);
    if LZ4F_isError_internal(r) {
        return SmRes::Return(r);
    }
    SmRes::Continue
}

unsafe fn sm_block_header(
    dctx: *mut LZ4F_dctx,
    srcPtr: &mut *const u8,
    srcEnd: *const u8,
    selectedIn: &mut *const u8,
    dstPtr: *mut u8,
    dstEnd: *mut u8,
    nextSrcSizeHint: &mut usize,
    doAnotherStage: &mut i32,
) -> SmRes {
    let d = &mut *dctx;
    if d.dStage == dstage_getBlockHeader {
        if (srcEnd as usize - *srcPtr as usize) >= BHSize {
            *selectedIn = *srcPtr;
            *srcPtr = (*srcPtr).add(BHSize);
        } else {
            d.tmpInSize = 0;
            d.dStage = dstage_storeBlockHeader;
        }
    }
    if d.dStage == dstage_storeBlockHeader {
        let remainingInput = srcEnd as usize - *srcPtr as usize;
        let wantedData = BHSize - d.tmpInSize;
        let sizeToCopy = MIN(wantedData, remainingInput);
        ptr::copy_nonoverlapping(*srcPtr, d.tmpIn.add(d.tmpInSize), sizeToCopy);
        *srcPtr = (*srcPtr).add(sizeToCopy);
        d.tmpInSize += sizeToCopy;
        if d.tmpInSize < BHSize {
            *nextSrcSizeHint = BHSize - d.tmpInSize;
            *doAnotherStage = 0;
            return SmRes::Continue;
        }
        *selectedIn = d.tmpIn;
    }

    let blockHeader = LZ4F_readLE32(*selectedIn);
    let nextCBlockSize = (blockHeader & 0x7FFFFFFF) as usize;
    let crcSize = (d.frameInfo.blockChecksumFlag as usize) * BFSize;
    if blockHeader == 0 {
        d.dStage = dstage_getSuffix;
        return SmRes::Continue;
    }
    if nextCBlockSize > d.maxBlockSize {
        return SmRes::Return(returnErrorCode(err::ERROR_maxBlockSize_invalid));
    }
    if (blockHeader & LZ4F_BLOCKUNCOMPRESSED_FLAG) != 0 {
        d.tmpInTarget = nextCBlockSize;
        if d.frameInfo.blockChecksumFlag != 0 {
            xxh32_reset(&mut d.blockChecksum, 0);
        }
        d.dStage = dstage_copyDirect;
        return SmRes::Continue;
    }
    d.tmpInTarget = nextCBlockSize + crcSize;
    d.dStage = dstage_getCBlock;
    if dstPtr == dstEnd || *srcPtr == srcEnd {
        *nextSrcSizeHint = BHSize + nextCBlockSize + crcSize;
        *doAnotherStage = 0;
    }
    SmRes::Continue
}

unsafe fn sm_copy_direct(
    dctx: *mut LZ4F_dctx,
    srcPtr: &mut *const u8,
    srcEnd: *const u8,
    dstPtr: &mut *mut u8,
    dstEnd: *mut u8,
    dstStart: *mut u8,
    nextSrcSizeHint: &mut usize,
    doAnotherStage: &mut i32,
) -> SmRes {
    let d = &mut *dctx;
    let sizeToCopy;
    if (*dstPtr).is_null() {
        sizeToCopy = 0;
    } else {
        let minBuffSize = MIN(srcEnd as usize - *srcPtr as usize, dstEnd as usize - *dstPtr as usize);
        sizeToCopy = MIN(d.tmpInTarget, minBuffSize);
        ptr::copy_nonoverlapping(*srcPtr, *dstPtr, sizeToCopy);
        if d.skipChecksum == 0 {
            if d.frameInfo.blockChecksumFlag != 0 {
                xxh32_update(&mut d.blockChecksum, *srcPtr, sizeToCopy);
            }
            if d.frameInfo.contentChecksumFlag != 0 {
                xxh32_update(&mut d.xxh, *srcPtr, sizeToCopy);
            }
        }
        if d.frameInfo.contentSize != 0 {
            d.frameRemainingSize -= sizeToCopy as u64;
        }
        if d.frameInfo.blockMode == LZ4F_blockLinked {
            LZ4F_updateDict(dctx, *dstPtr, sizeToCopy, dstStart, 0);
        }
        *srcPtr = (*srcPtr).add(sizeToCopy);
        *dstPtr = (*dstPtr).add(sizeToCopy);
    }
    if sizeToCopy == d.tmpInTarget {
        if d.frameInfo.blockChecksumFlag != 0 {
            d.tmpInSize = 0;
            d.dStage = dstage_getBlockChecksum;
        } else {
            d.dStage = dstage_getBlockHeader;
        }
        return SmRes::Continue;
    }
    d.tmpInTarget -= sizeToCopy;
    *nextSrcSizeHint = d.tmpInTarget + (if d.frameInfo.blockChecksumFlag != 0 { BFSize } else { 0 }) + BHSize;
    *doAnotherStage = 0;
    SmRes::Continue
}

unsafe fn sm_block_checksum(
    dctx: *mut LZ4F_dctx,
    srcPtr: &mut *const u8,
    srcEnd: *const u8,
    doAnotherStage: &mut i32,
) -> SmRes {
    let d = &mut *dctx;
    let crcSrc: *const u8;
    if (srcEnd as usize - *srcPtr as usize) >= 4 && d.tmpInSize == 0 {
        crcSrc = *srcPtr;
        *srcPtr = (*srcPtr).add(4);
    } else {
        let stillToCopy = 4 - d.tmpInSize;
        let sizeToCopy = MIN(stillToCopy, srcEnd as usize - *srcPtr as usize);
        ptr::copy_nonoverlapping(*srcPtr, d.header.as_mut_ptr().add(d.tmpInSize), sizeToCopy);
        d.tmpInSize += sizeToCopy;
        *srcPtr = (*srcPtr).add(sizeToCopy);
        if d.tmpInSize < 4 {
            *doAnotherStage = 0;
            return SmRes::Continue;
        }
        crcSrc = d.header.as_ptr();
    }
    if d.skipChecksum == 0 {
        let readCRC = LZ4F_readLE32(crcSrc);
        let calcCRC = xxh32_digest(&d.blockChecksum);
        if readCRC != calcCRC {
            return SmRes::Return(returnErrorCode(err::ERROR_blockChecksum_invalid));
        }
    }
    d.dStage = dstage_getBlockHeader;
    SmRes::Continue
}

unsafe fn sm_cblock(
    dctx: *mut LZ4F_dctx,
    srcPtr: &mut *const u8,
    srcEnd: *const u8,
    selectedIn: &mut *const u8,
    dstPtr: &mut *mut u8,
    dstEnd: *mut u8,
    dstStart: *mut u8,
    nextSrcSizeHint: &mut usize,
    doAnotherStage: &mut i32,
) -> SmRes {
    let d = &mut *dctx;

    if d.dStage == dstage_getCBlock {
        if (srcEnd as usize - *srcPtr as usize) < d.tmpInTarget {
            d.tmpInSize = 0;
            d.dStage = dstage_storeCBlock;
        } else {
            *selectedIn = *srcPtr;
            *srcPtr = (*srcPtr).add(d.tmpInTarget);
        }
    }

    if d.dStage == dstage_storeCBlock {
        let wantedData = d.tmpInTarget - d.tmpInSize;
        let inputLeft = srcEnd as usize - *srcPtr as usize;
        let sizeToCopy = MIN(wantedData, inputLeft);
        ptr::copy_nonoverlapping(*srcPtr, d.tmpIn.add(d.tmpInSize), sizeToCopy);
        d.tmpInSize += sizeToCopy;
        *srcPtr = (*srcPtr).add(sizeToCopy);
        if d.tmpInSize < d.tmpInTarget {
            *nextSrcSizeHint = (d.tmpInTarget - d.tmpInSize)
                + (if d.frameInfo.blockChecksumFlag != 0 { BFSize } else { 0 })
                + BHSize;
            *doAnotherStage = 0;
            return SmRes::Continue;
        }
        *selectedIn = d.tmpIn;
    }

    if d.frameInfo.blockChecksumFlag != 0 {
        d.tmpInTarget -= 4;
        let readBlockCrc = LZ4F_readLE32((*selectedIn).add(d.tmpInTarget));
        let calcBlockCrc = xxh32(*selectedIn, d.tmpInTarget, 0);
        if readBlockCrc != calcBlockCrc {
            return SmRes::Return(returnErrorCode(err::ERROR_blockChecksum_invalid));
        }
    }

    if (dstEnd as usize - *dstPtr as usize) >= d.maxBlockSize
        && !(!d.dict.is_null() && (d.dict.wrapping_add(d.dictSize) == d.tmpOut))
    {
        let mut dict = d.dict as *const c_char;
        let mut dictSize = d.dictSize;
        if !dict.is_null() && dictSize > (1usize << 30) {
            dict = dict.add(dictSize - 64 * 1024);
            dictSize = 64 * 1024;
        }
        let decodedSize = LZ4_decompress_safe_usingDict(
            *selectedIn as *const c_char, *dstPtr as *mut c_char, d.tmpInTarget as c_int,
            d.maxBlockSize as c_int, dict, dictSize as c_int,
        );
        if decodedSize < 0 {
            return SmRes::Return(returnErrorCode(err::ERROR_decompressionFailed));
        }
        if d.frameInfo.contentChecksumFlag != 0 && d.skipChecksum == 0 {
            xxh32_update(&mut d.xxh, *dstPtr, decodedSize as usize);
        }
        if d.frameInfo.contentSize != 0 {
            d.frameRemainingSize -= decodedSize as u64;
        }
        if d.frameInfo.blockMode == LZ4F_blockLinked {
            LZ4F_updateDict(dctx, *dstPtr, decodedSize as usize, dstStart, 0);
        }
        *dstPtr = (*dstPtr).add(decodedSize as usize);
        d.dStage = dstage_getBlockHeader;
        return SmRes::Continue;
    }

    if d.frameInfo.blockMode == LZ4F_blockLinked {
        if d.dict == d.tmpOutBuffer {
            if d.dictSize > 128 * 1024 {
                ptr::copy_nonoverlapping(d.dict.wrapping_add(d.dictSize).wrapping_sub(64 * 1024), d.tmpOutBuffer, 64 * 1024);
                d.dictSize = 64 * 1024;
            }
            d.tmpOut = d.tmpOutBuffer.add(d.dictSize);
        } else {
            let reservedDictSpace = MIN(d.dictSize, 64 * 1024);
            d.tmpOut = d.tmpOutBuffer.add(reservedDictSpace);
        }
    }

    {
        let mut dict = d.dict as *const c_char;
        let mut dictSize = d.dictSize;
        if !dict.is_null() && dictSize > (1usize << 30) {
            dict = dict.add(dictSize - 64 * 1024);
            dictSize = 64 * 1024;
        }
        let decodedSize = LZ4_decompress_safe_usingDict(
            *selectedIn as *const c_char, d.tmpOut as *mut c_char, d.tmpInTarget as c_int,
            d.maxBlockSize as c_int, dict, dictSize as c_int,
        );
        if decodedSize < 0 {
            return SmRes::Return(returnErrorCode(err::ERROR_decompressionFailed));
        }
        if d.frameInfo.contentChecksumFlag != 0 && d.skipChecksum == 0 {
            xxh32_update(&mut d.xxh, d.tmpOut, decodedSize as usize);
        }
        if d.frameInfo.contentSize != 0 {
            d.frameRemainingSize -= decodedSize as u64;
        }
        d.tmpOutSize = decodedSize as usize;
        d.tmpOutStart = 0;
        d.dStage = dstage_flushOut;
    }
    sm_flush_out(dctx, dstPtr, dstEnd, dstStart, nextSrcSizeHint, doAnotherStage)
}

unsafe fn sm_flush_out(
    dctx: *mut LZ4F_dctx,
    dstPtr: &mut *mut u8,
    dstEnd: *mut u8,
    dstStart: *mut u8,
    nextSrcSizeHint: &mut usize,
    doAnotherStage: &mut i32,
) -> SmRes {
    let d = &mut *dctx;
    if !(*dstPtr).is_null() {
        let sizeToCopy = MIN(d.tmpOutSize - d.tmpOutStart, dstEnd as usize - *dstPtr as usize);
        ptr::copy_nonoverlapping(d.tmpOut.add(d.tmpOutStart), *dstPtr, sizeToCopy);
        if d.frameInfo.blockMode == LZ4F_blockLinked {
            LZ4F_updateDict(dctx, *dstPtr, sizeToCopy, dstStart, 1);
        }
        d.tmpOutStart += sizeToCopy;
        *dstPtr = (*dstPtr).add(sizeToCopy);
    }
    if d.tmpOutStart == d.tmpOutSize {
        d.dStage = dstage_getBlockHeader;
        return SmRes::Continue;
    }
    *doAnotherStage = 0;
    *nextSrcSizeHint = BHSize;
    SmRes::Continue
}

unsafe fn sm_suffix(
    dctx: *mut LZ4F_dctx,
    srcPtr: &mut *const u8,
    srcEnd: *const u8,
    selectedIn: &mut *const u8,
    nextSrcSizeHint: &mut usize,
    doAnotherStage: &mut i32,
) -> SmRes {
    let d = &mut *dctx;

    if d.dStage == dstage_getSuffix {
        if d.frameRemainingSize != 0 {
            return SmRes::Return(returnErrorCode(err::ERROR_frameSize_wrong));
        }
        if d.frameInfo.contentChecksumFlag == 0 {
            *nextSrcSizeHint = 0;
            LZ4F_resetDecompressionContext(dctx);
            *doAnotherStage = 0;
            return SmRes::Continue;
        }
        if (srcEnd as usize - *srcPtr as usize) < 4 {
            d.tmpInSize = 0;
            d.dStage = dstage_storeSuffix;
        } else {
            *selectedIn = *srcPtr;
            *srcPtr = (*srcPtr).add(4);
        }
    }

    if d.dStage == dstage_storeSuffix {
        let remainingInput = srcEnd as usize - *srcPtr as usize;
        let wantedData = 4 - d.tmpInSize;
        let sizeToCopy = MIN(wantedData, remainingInput);
        ptr::copy_nonoverlapping(*srcPtr, d.tmpIn.add(d.tmpInSize), sizeToCopy);
        *srcPtr = (*srcPtr).add(sizeToCopy);
        d.tmpInSize += sizeToCopy;
        if d.tmpInSize < 4 {
            *nextSrcSizeHint = 4 - d.tmpInSize;
            *doAnotherStage = 0;
            return SmRes::Continue;
        }
        *selectedIn = d.tmpIn;
    }

    if d.skipChecksum == 0 {
        let readCRC = LZ4F_readLE32(*selectedIn);
        let resultCRC = xxh32_digest(&d.xxh);
        if readCRC != resultCRC {
            return SmRes::Return(returnErrorCode(err::ERROR_contentChecksum_invalid));
        }
    }
    *nextSrcSizeHint = 0;
    LZ4F_resetDecompressionContext(dctx);
    *doAnotherStage = 0;
    SmRes::Continue
}

unsafe fn sm_sframesize(
    dctx: *mut LZ4F_dctx,
    srcPtr: &mut *const u8,
    srcEnd: *const u8,
    selectedIn: &mut *const u8,
    nextSrcSizeHint: &mut usize,
    doAnotherStage: &mut i32,
) -> SmRes {
    let d = &mut *dctx;

    if d.dStage == dstage_getSFrameSize {
        if (srcEnd as usize - *srcPtr as usize) >= 4 {
            *selectedIn = *srcPtr;
            *srcPtr = (*srcPtr).add(4);
        } else {
            d.tmpInSize = 4;
            d.tmpInTarget = 8;
            d.dStage = dstage_storeSFrameSize;
        }
    }

    if d.dStage == dstage_storeSFrameSize {
        let sizeToCopy = MIN(d.tmpInTarget - d.tmpInSize, srcEnd as usize - *srcPtr as usize);
        ptr::copy_nonoverlapping(*srcPtr, d.header.as_mut_ptr().add(d.tmpInSize), sizeToCopy);
        *srcPtr = (*srcPtr).add(sizeToCopy);
        d.tmpInSize += sizeToCopy;
        if d.tmpInSize < d.tmpInTarget {
            *nextSrcSizeHint = d.tmpInTarget - d.tmpInSize;
            *doAnotherStage = 0;
            return SmRes::Continue;
        }
        *selectedIn = d.header.as_ptr().add(4);
    }

    let SFrameSize = LZ4F_readLE32(*selectedIn) as usize;
    d.frameInfo.contentSize = SFrameSize as u64;
    d.tmpInTarget = SFrameSize;
    d.dStage = dstage_skipSkippable;
    SmRes::Continue
}

unsafe fn sm_skip_skippable(
    dctx: *mut LZ4F_dctx,
    srcPtr: &mut *const u8,
    srcEnd: *const u8,
    nextSrcSizeHint: &mut usize,
    doAnotherStage: &mut i32,
) -> SmRes {
    let d = &mut *dctx;
    let skipSize = MIN(d.tmpInTarget, srcEnd as usize - *srcPtr as usize);
    *srcPtr = (*srcPtr).add(skipSize);
    d.tmpInTarget -= skipSize;
    *doAnotherStage = 0;
    *nextSrcSizeHint = d.tmpInTarget;
    if *nextSrcSizeHint != 0 {
        return SmRes::Continue;
    }
    LZ4F_resetDecompressionContext(dctx);
    SmRes::Continue
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_decompress_usingDict(
    dctx: *mut LZ4F_dctx,
    dstBuffer: *mut c_void,
    dstSizePtr: *mut usize,
    srcBuffer: *const c_void,
    srcSizePtr: *mut usize,
    dict: *const c_void,
    dictSize: usize,
    decompressOptionsPtr: *const LZ4F_decompressOptions_t,
) -> usize {
    let d = &mut *dctx;
    if d.dStage <= dstage_init {
        d.dict = dict as *const u8;
        d.dictSize = dictSize;
    }
    LZ4F_decompress(dctx, dstBuffer, dstSizePtr, srcBuffer, srcSizePtr, decompressOptionsPtr)
}
