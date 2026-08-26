//! Translation of `c_src/src/lz4frame.c`

use crate::common::*;
use crate::lz4::*;
use crate::lz4hc::*;
use crate::xxhash::*;
use core::ffi::{c_char, c_int, c_uint, c_void};

/*-************************************
*  Public types
**************************************/
pub type LZ4F_errorCode_t = usize;

pub type LZ4F_AllocFunction = Option<unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void>;
pub type LZ4F_CallocFunction = Option<unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void>;
pub type LZ4F_FreeFunction = Option<unsafe extern "C" fn(*mut c_void, *mut c_void)>;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct LZ4F_CustomMem {
    pub customAlloc: LZ4F_AllocFunction,
    pub customCalloc: LZ4F_CallocFunction,
    pub customFree: LZ4F_FreeFunction,
    pub opaqueState: *mut c_void,
}

pub const LZ4F_defaultCMem: LZ4F_CustomMem = LZ4F_CustomMem {
    customAlloc: None,
    customCalloc: None,
    customFree: None,
    opaqueState: core::ptr::null_mut(),
};

/* LZ4F_blockSizeID_t */
pub const LZ4F_default: c_uint = 0;
pub const LZ4F_max64KB: c_uint = 4;
pub const LZ4F_max256KB: c_uint = 5;
pub const LZ4F_max1MB: c_uint = 6;
pub const LZ4F_max4MB: c_uint = 7;

/* LZ4F_blockMode_t */
pub const LZ4F_blockLinked: c_uint = 0;
pub const LZ4F_blockIndependent: c_uint = 1;

/* LZ4F_contentChecksum_t */
pub const LZ4F_noContentChecksum: c_uint = 0;
pub const LZ4F_contentChecksumEnabled: c_uint = 1;

/* LZ4F_blockChecksum_t */
pub const LZ4F_noBlockChecksum: c_uint = 0;
pub const LZ4F_blockChecksumEnabled: c_uint = 1;

/* LZ4F_frameType_t */
pub const LZ4F_frame: c_uint = 0;
pub const LZ4F_skippableFrame: c_uint = 1;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct LZ4F_frameInfo_t {
    pub blockSizeID: c_uint,
    pub blockMode: c_uint,
    pub contentChecksumFlag: c_uint,
    pub frameType: c_uint,
    pub contentSize: u64,
    pub dictID: c_uint,
    pub blockChecksumFlag: c_uint,
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

pub const LZ4F_VERSION: c_uint = 100;
pub const LZ4F_HEADER_SIZE_MIN: usize = 7;
pub const LZ4F_HEADER_SIZE_MAX: usize = 19;
pub const LZ4F_BLOCK_HEADER_SIZE: usize = 4;
pub const LZ4F_BLOCK_CHECKSUM_SIZE: usize = 4;
pub const LZ4F_MAGICNUMBER: u32 = 0x184D2204;
pub const LZ4F_MAGIC_SKIPPABLE_START: u32 = 0x184D2A50;
pub const LZ4F_MIN_SIZE_TO_KNOW_HEADER_LENGTH: usize = 5;

/*-************************************
*  Error codes
**************************************/
pub const LZ4F_OK_NoError: i32 = 0;
pub const LZ4F_ERROR_GENERIC: i32 = 1;
pub const LZ4F_ERROR_maxBlockSize_invalid: i32 = 2;
pub const LZ4F_ERROR_blockMode_invalid: i32 = 3;
pub const LZ4F_ERROR_parameter_invalid: i32 = 4;
pub const LZ4F_ERROR_compressionLevel_invalid: i32 = 5;
pub const LZ4F_ERROR_headerVersion_wrong: i32 = 6;
pub const LZ4F_ERROR_blockChecksum_invalid: i32 = 7;
pub const LZ4F_ERROR_reservedFlag_set: i32 = 8;
pub const LZ4F_ERROR_allocation_failed: i32 = 9;
pub const LZ4F_ERROR_srcSize_tooLarge: i32 = 10;
pub const LZ4F_ERROR_dstMaxSize_tooSmall: i32 = 11;
pub const LZ4F_ERROR_frameHeader_incomplete: i32 = 12;
pub const LZ4F_ERROR_frameType_unknown: i32 = 13;
pub const LZ4F_ERROR_frameSize_wrong: i32 = 14;
pub const LZ4F_ERROR_srcPtr_wrong: i32 = 15;
pub const LZ4F_ERROR_decompressionFailed: i32 = 16;
pub const LZ4F_ERROR_headerChecksum_invalid: i32 = 17;
pub const LZ4F_ERROR_contentChecksum_invalid: i32 = 18;
pub const LZ4F_ERROR_frameDecoding_alreadyStarted: i32 = 19;
pub const LZ4F_ERROR_compressionState_uninitialized: i32 = 20;
pub const LZ4F_ERROR_parameter_null: i32 = 21;
pub const LZ4F_ERROR_io_write: i32 = 22;
pub const LZ4F_ERROR_io_read: i32 = 23;
pub const LZ4F_ERROR_maxCode: i32 = 24;

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

static CODE_ERROR: &[u8] = b"Unspecified error code\0";

#[inline]
pub fn LZ4F_returnErrorCode(code: i32) -> LZ4F_errorCode_t {
    (0isize.wrapping_sub(code as isize)) as usize
}

macro_rules! RETURN_ERROR {
    ($e:expr) => {
        return LZ4F_returnErrorCode($e)
    };
}

macro_rules! RETURN_ERROR_IF {
    ($c:expr, $e:expr) => {
        if $c {
            return LZ4F_returnErrorCode($e);
        }
    };
}

macro_rules! FORWARD_IF_ERROR {
    ($r:expr) => {{
        let r = $r;
        if LZ4F_isError_(r) {
            return r;
        }
    }};
}

#[inline]
pub fn LZ4F_isError_(code: LZ4F_errorCode_t) -> bool {
    code > LZ4F_returnErrorCode(LZ4F_ERROR_maxCode)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_isError(code: LZ4F_errorCode_t) -> c_uint {
    (code > LZ4F_returnErrorCode(LZ4F_ERROR_maxCode)) as c_uint
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_getErrorName(code: LZ4F_errorCode_t) -> *const c_char {
    if LZ4F_isError_(code) {
        let idx = (0i32.wrapping_sub(code as i32)) as usize;
        return LZ4F_errorStrings[idx].as_ptr() as *const c_char;
    }
    CODE_ERROR.as_ptr() as *const c_char
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_getErrorCode(functionResult: usize) -> c_int {
    if !LZ4F_isError_(functionResult) {
        return LZ4F_OK_NoError;
    }
    (0isize.wrapping_sub(functionResult as isize)) as c_int
}

/*-************************************
*  Memory routines
**************************************/
unsafe fn LZ4F_calloc(s: usize, cmem: LZ4F_CustomMem) -> *mut c_void {
    if let Some(cc) = cmem.customCalloc {
        return cc(cmem.opaqueState, s);
    }
    if cmem.customAlloc.is_none() {
        return calloc(1, s);
    }
    {
        let p = (cmem.customAlloc.unwrap())(cmem.opaqueState, s);
        if !p.is_null() {
            mem_init(p as *mut u8, 0, s);
        }
        p
    }
}

unsafe fn LZ4F_malloc(s: usize, cmem: LZ4F_CustomMem) -> *mut c_void {
    if let Some(ca) = cmem.customAlloc {
        return ca(cmem.opaqueState, s);
    }
    malloc(s)
}

unsafe fn LZ4F_free(p: *mut c_void, cmem: LZ4F_CustomMem) {
    if p.is_null() {
        return;
    }
    if let Some(cf) = cmem.customFree {
        cf(cmem.opaqueState, p);
        return;
    }
    free(p);
}

/*-************************************
*  Little endian I/O
**************************************/
unsafe fn LZ4F_readLE32(src: *const c_void) -> u32 {
    let srcPtr = src as *const u8;
    let mut value32: u32 = *srcPtr as u32;
    value32 |= (*srcPtr.wrapping_add(1) as u32) << 8;
    value32 |= (*srcPtr.wrapping_add(2) as u32) << 16;
    value32 |= (*srcPtr.wrapping_add(3) as u32) << 24;
    value32
}

unsafe fn LZ4F_writeLE32(dst: *mut c_void, value32: u32) {
    let dstPtr = dst as *mut u8;
    *dstPtr = value32 as u8;
    *dstPtr.wrapping_add(1) = (value32 >> 8) as u8;
    *dstPtr.wrapping_add(2) = (value32 >> 16) as u8;
    *dstPtr.wrapping_add(3) = (value32 >> 24) as u8;
}

unsafe fn LZ4F_readLE64(src: *const c_void) -> u64 {
    let srcPtr = src as *const u8;
    let mut value64: u64 = *srcPtr as u64;
    value64 |= (*srcPtr.wrapping_add(1) as u64) << 8;
    value64 |= (*srcPtr.wrapping_add(2) as u64) << 16;
    value64 |= (*srcPtr.wrapping_add(3) as u64) << 24;
    value64 |= (*srcPtr.wrapping_add(4) as u64) << 32;
    value64 |= (*srcPtr.wrapping_add(5) as u64) << 40;
    value64 |= (*srcPtr.wrapping_add(6) as u64) << 48;
    value64 |= (*srcPtr.wrapping_add(7) as u64) << 56;
    value64
}

unsafe fn LZ4F_writeLE64(dst: *mut c_void, value64: u64) {
    let dstPtr = dst as *mut u8;
    *dstPtr = value64 as u8;
    *dstPtr.wrapping_add(1) = (value64 >> 8) as u8;
    *dstPtr.wrapping_add(2) = (value64 >> 16) as u8;
    *dstPtr.wrapping_add(3) = (value64 >> 24) as u8;
    *dstPtr.wrapping_add(4) = (value64 >> 32) as u8;
    *dstPtr.wrapping_add(5) = (value64 >> 40) as u8;
    *dstPtr.wrapping_add(6) = (value64 >> 48) as u8;
    *dstPtr.wrapping_add(7) = (value64 >> 56) as u8;
}

/*-************************************
*  Constants
**************************************/
const _1BIT: u32 = 0x01;
const _2BITS: u32 = 0x03;
const _3BITS: u32 = 0x07;
const _4BITS: u32 = 0x0F;
const _8BITS: u32 = 0xFF;

const LZ4F_BLOCKUNCOMPRESSED_FLAG: u32 = 0x80000000;
const LZ4F_BLOCKSIZEID_DEFAULT: c_uint = LZ4F_max64KB;

const minFHSize: usize = LZ4F_HEADER_SIZE_MIN;
const maxFHSize: usize = LZ4F_HEADER_SIZE_MAX;
const BHSize: usize = LZ4F_BLOCK_HEADER_SIZE;
const BFSize: usize = LZ4F_BLOCK_CHECKSUM_SIZE;

/* LZ4F_BlockCompressMode_e */
const LZ4B_COMPRESSED: u32 = 0;
const LZ4B_UNCOMPRESSED: u32 = 1;

/* LZ4F_CtxType_e */
const ctxNone: u16 = 0;
const ctxFast: u16 = 1;
const ctxHC: u16 = 2;

const LZ4F_INIT_FRAMEINFO: LZ4F_frameInfo_t = LZ4F_frameInfo_t {
    blockSizeID: LZ4F_max64KB,
    blockMode: LZ4F_blockLinked,
    contentChecksumFlag: LZ4F_noContentChecksum,
    frameType: LZ4F_frame,
    contentSize: 0,
    dictID: 0,
    blockChecksumFlag: LZ4F_noBlockChecksum,
};

const LZ4F_INIT_PREFERENCES: LZ4F_preferences_t = LZ4F_preferences_t {
    frameInfo: LZ4F_INIT_FRAMEINFO,
    compressionLevel: 0,
    autoFlush: 0,
    favorDecSpeed: 0,
    reserved: [0, 0, 0],
};

/*-************************************
*  Compression context
**************************************/
#[repr(C)]
pub struct LZ4F_cctx {
    pub cmem: LZ4F_CustomMem,
    pub prefs: LZ4F_preferences_t,
    pub version: u32,
    pub cStage: u32,
    pub cdict: *const LZ4F_CDict,
    pub maxBlockSize: usize,
    pub maxBufferSize: usize,
    pub tmpBuff: *mut u8,
    pub tmpIn: *mut u8,
    pub tmpInSize: usize,
    pub totalInSize: u64,
    pub xxh: XXH32_state_t,
    pub lz4CtxPtr: *mut c_void,
    pub lz4CtxAlloc: u16,
    pub lz4CtxType: u16,
    pub blockCompressMode: u32,
}

#[repr(C)]
pub struct LZ4F_CDict {
    pub cmem: LZ4F_CustomMem,
    pub dictContent: *mut c_void,
    pub fastCtx: *mut LZ4_stream_t,
    pub HCCtx: *mut LZ4_streamHC_t,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_getVersion() -> c_uint {
    LZ4F_VERSION
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_compressionLevel_max() -> c_int {
    LZ4HC_CLEVEL_MAX
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_getBlockSize(blockSizeID: c_uint) -> usize {
    static blockSizes: [usize; 4] = [64 * 1024, 256 * 1024, 1024 * 1024, 4 * 1024 * 1024];
    let mut blockSizeID = blockSizeID;

    if blockSizeID == 0 {
        blockSizeID = LZ4F_BLOCKSIZEID_DEFAULT;
    }
    if blockSizeID < LZ4F_max64KB || blockSizeID > LZ4F_max4MB {
        RETURN_ERROR!(LZ4F_ERROR_maxBlockSize_invalid);
    }
    {
        let blockSizeIdx = (blockSizeID as i32) - (LZ4F_max64KB as i32);
        blockSizes[blockSizeIdx as usize]
    }
}

/*-************************************
*  Private functions
**************************************/
#[inline(always)]
fn MINuz(a: usize, b: usize) -> usize {
    if a < b {
        a
    } else {
        b
    }
}

unsafe fn LZ4F_headerChecksum(header: *const c_void, length: usize) -> u8 {
    let xxh = LZ4_XXH32(header, length, 0);
    (xxh >> 8) as u8
}

/*-************************************
*  Simple-pass compression functions
**************************************/
fn LZ4F_optimalBSID(requestedBSID: c_uint, srcSize: usize) -> c_uint {
    let mut proposedBSID: c_uint = LZ4F_max64KB;
    let mut maxBlockSize: usize = 64 * 1024;
    while requestedBSID > proposedBSID {
        if srcSize <= maxBlockSize {
            return proposedBSID;
        }
        proposedBSID = ((proposedBSID as i32) + 1) as c_uint;
        maxBlockSize <<= 2;
    }
    requestedBSID
}

unsafe fn LZ4F_compressBound_internal(
    srcSize: usize,
    preferencesPtr: *const LZ4F_preferences_t,
    alreadyBuffered: usize,
) -> usize {
    let mut prefsNull: LZ4F_preferences_t = LZ4F_INIT_PREFERENCES;
    prefsNull.frameInfo.contentChecksumFlag = LZ4F_contentChecksumEnabled;
    prefsNull.frameInfo.blockChecksumFlag = LZ4F_blockChecksumEnabled;
    {
        let prefsPtr: *const LZ4F_preferences_t = if preferencesPtr.is_null() {
            &prefsNull as *const LZ4F_preferences_t
        } else {
            preferencesPtr
        };
        let flush: u32 = (*prefsPtr).autoFlush | ((srcSize == 0) as u32);
        let blockID: c_uint = (*prefsPtr).frameInfo.blockSizeID;
        let blockSize: usize = LZ4F_getBlockSize(blockID);
        let maxBuffered: usize = blockSize.wrapping_sub(1);
        let bufferedSize: usize = MINuz(alreadyBuffered, maxBuffered);
        let maxSrcSize: usize = srcSize.wrapping_add(bufferedSize);
        let nbFullBlocks: u32 = (maxSrcSize / blockSize) as u32;
        let partialBlockSize: usize = maxSrcSize & (blockSize.wrapping_sub(1));
        let lastBlockSize: usize = if flush != 0 { partialBlockSize } else { 0 };
        let nbBlocks: u32 = nbFullBlocks.wrapping_add((lastBlockSize > 0) as u32);

        let blockCRCSize: usize = BFSize * (*prefsPtr).frameInfo.blockChecksumFlag as usize;
        let frameEnd: usize =
            BHSize + ((*prefsPtr).frameInfo.contentChecksumFlag as usize) * BFSize;

        ((BHSize + blockCRCSize).wrapping_mul(nbBlocks as usize))
            .wrapping_add(blockSize.wrapping_mul(nbFullBlocks as usize))
            .wrapping_add(lastBlockSize)
            .wrapping_add(frameEnd)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_compressFrameBound(
    srcSize: usize,
    preferencesPtr: *const LZ4F_preferences_t,
) -> usize {
    let mut prefs: LZ4F_preferences_t;
    let headerSize: usize = maxFHSize;

    if !preferencesPtr.is_null() {
        prefs = *preferencesPtr;
    } else {
        prefs = core::mem::zeroed();
    }
    prefs.autoFlush = 1;

    headerSize + LZ4F_compressBound_internal(srcSize, &prefs, 0)
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
    let mut prefs: LZ4F_preferences_t;
    let mut options: LZ4F_compressOptions_t;
    let dstStart = dstBuffer as *mut u8;
    let mut dstPtr = dstStart;
    let dstEnd = dstStart.wrapping_add(dstCapacity);

    if !preferencesPtr.is_null() {
        prefs = *preferencesPtr;
    } else {
        prefs = core::mem::zeroed();
    }
    if prefs.frameInfo.contentSize != 0 {
        prefs.frameInfo.contentSize = srcSize as u64;
    }

    prefs.frameInfo.blockSizeID = LZ4F_optimalBSID(prefs.frameInfo.blockSizeID, srcSize);
    prefs.autoFlush = 1;
    if srcSize <= LZ4F_getBlockSize(prefs.frameInfo.blockSizeID) {
        prefs.frameInfo.blockMode = LZ4F_blockIndependent;
    }

    options = core::mem::zeroed();
    options.stableSrc = 1;

    RETURN_ERROR_IF!(
        dstCapacity < LZ4F_compressFrameBound(srcSize, &prefs),
        LZ4F_ERROR_dstMaxSize_tooSmall
    );

    {
        let headerSize = LZ4F_compressBegin_usingCDict(cctx, dstBuffer, dstCapacity, cdict, &prefs);
        FORWARD_IF_ERROR!(headerSize);
        dstPtr = dstPtr.wrapping_add(headerSize);
    }

    {
        let cSize = LZ4F_compressUpdate(
            cctx,
            dstPtr as *mut c_void,
            dstEnd as usize - dstPtr as usize,
            srcBuffer,
            srcSize,
            &options,
        );
        FORWARD_IF_ERROR!(cSize);
        dstPtr = dstPtr.wrapping_add(cSize);
    }

    {
        let tailSize = LZ4F_compressEnd(
            cctx,
            dstPtr as *mut c_void,
            dstEnd as usize - dstPtr as usize,
            &options,
        );
        FORWARD_IF_ERROR!(tailSize);
        dstPtr = dstPtr.wrapping_add(tailSize);
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
    let result: usize;
    /* LZ4F_HEAPMODE == 0 */
    let mut cctx_storage = core::mem::MaybeUninit::<LZ4F_cctx>::zeroed();
    let mut lz4ctx_storage = core::mem::MaybeUninit::<LZ4_stream_t>::uninit();
    let cctxPtr: *mut LZ4F_cctx = cctx_storage.as_mut_ptr();

    mem_init(cctxPtr as *mut u8, 0, core::mem::size_of::<LZ4F_cctx>());
    (*cctxPtr).version = LZ4F_VERSION;
    (*cctxPtr).maxBufferSize = 5 * 1024 * 1024;
    if preferencesPtr.is_null() || (*preferencesPtr).compressionLevel < LZ4HC_CLEVEL_MIN {
        let lz4ctx = lz4ctx_storage.as_mut_ptr();
        LZ4_initStream(lz4ctx as *mut c_void, core::mem::size_of::<LZ4_stream_t>());
        (*cctxPtr).lz4CtxPtr = lz4ctx as *mut c_void;
        (*cctxPtr).lz4CtxAlloc = 1;
        (*cctxPtr).lz4CtxType = ctxFast;
    }

    result = LZ4F_compressFrame_usingCDict(
        cctxPtr,
        dstBuffer,
        dstCapacity,
        srcBuffer,
        srcSize,
        core::ptr::null(),
        preferencesPtr,
    );

    if !preferencesPtr.is_null() && (*preferencesPtr).compressionLevel >= LZ4HC_CLEVEL_MIN {
        LZ4F_free((*cctxPtr).lz4CtxPtr, (*cctxPtr).cmem);
    }
    result
}

/*-***************************************************
*   Dictionary compression
*****************************************************/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_createCDict_advanced(
    cmem: LZ4F_CustomMem,
    dictBuffer: *const c_void,
    dictSize: usize,
) -> *mut LZ4F_CDict {
    let mut dictStart = dictBuffer as *const c_char;
    let mut dictSize = dictSize;
    let cdict = LZ4F_malloc(core::mem::size_of::<LZ4F_CDict>(), cmem) as *mut LZ4F_CDict;
    if cdict.is_null() {
        return core::ptr::null_mut();
    }
    (*cdict).cmem = cmem;
    if dictSize > 64 * 1024 {
        dictStart = dictStart.wrapping_add(dictSize - 64 * 1024);
        dictSize = 64 * 1024;
    }
    (*cdict).dictContent = LZ4F_malloc(dictSize, cmem);
    (*cdict).fastCtx = LZ4F_malloc(SIZEOF_LZ4_STREAM_T, cmem) as *mut LZ4_stream_t;
    (*cdict).HCCtx = LZ4F_malloc(SIZEOF_LZ4_STREAMHC_T, cmem) as *mut LZ4_streamHC_t;
    if (*cdict).dictContent.is_null() || (*cdict).fastCtx.is_null() || (*cdict).HCCtx.is_null() {
        LZ4F_freeCDict(cdict);
        return core::ptr::null_mut();
    }
    mem_copy(
        (*cdict).dictContent as *mut u8,
        dictStart as *const u8,
        dictSize,
    );
    LZ4_initStream((*cdict).fastCtx as *mut c_void, SIZEOF_LZ4_STREAM_T);
    LZ4_loadDictSlow(
        (*cdict).fastCtx,
        (*cdict).dictContent as *const c_char,
        dictSize as c_int,
    );
    LZ4_initStreamHC((*cdict).HCCtx as *mut c_void, SIZEOF_LZ4_STREAMHC_T);
    LZ4_setCompressionLevel((*cdict).HCCtx, LZ4HC_CLEVEL_DEFAULT);
    LZ4_loadDictHC(
        (*cdict).HCCtx,
        (*cdict).dictContent as *const c_char,
        dictSize as c_int,
    );
    cdict
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_createCDict(
    dictBuffer: *const c_void,
    dictSize: usize,
) -> *mut LZ4F_CDict {
    LZ4F_createCDict_advanced(LZ4F_defaultCMem, dictBuffer, dictSize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_freeCDict(cdict: *mut LZ4F_CDict) {
    if cdict.is_null() {
        return;
    }
    LZ4F_free((*cdict).dictContent, (*cdict).cmem);
    LZ4F_free((*cdict).fastCtx as *mut c_void, (*cdict).cmem);
    LZ4F_free((*cdict).HCCtx as *mut c_void, (*cdict).cmem);
    LZ4F_free(cdict as *mut c_void, (*cdict).cmem);
}

/*-*********************************
*  Advanced compression functions
***********************************/

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_createCompressionContext_advanced(
    customMem: LZ4F_CustomMem,
    version: c_uint,
) -> *mut LZ4F_cctx {
    let cctxPtr = LZ4F_calloc(core::mem::size_of::<LZ4F_cctx>(), customMem) as *mut LZ4F_cctx;
    if cctxPtr.is_null() {
        return core::ptr::null_mut();
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
    RETURN_ERROR_IF!(
        LZ4F_compressionContextPtr.is_null(),
        LZ4F_ERROR_parameter_null
    );

    *LZ4F_compressionContextPtr =
        LZ4F_createCompressionContext_advanced(LZ4F_defaultCMem, version);
    RETURN_ERROR_IF!(
        (*LZ4F_compressionContextPtr).is_null(),
        LZ4F_ERROR_allocation_failed
    );
    LZ4F_OK_NoError as LZ4F_errorCode_t
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_freeCompressionContext(cctxPtr: *mut LZ4F_cctx) -> LZ4F_errorCode_t {
    if !cctxPtr.is_null() {
        LZ4F_free((*cctxPtr).lz4CtxPtr, (*cctxPtr).cmem);
        LZ4F_free((*cctxPtr).tmpBuff as *mut c_void, (*cctxPtr).cmem);
        LZ4F_free(cctxPtr as *mut c_void, (*cctxPtr).cmem);
    }
    LZ4F_OK_NoError as LZ4F_errorCode_t
}

unsafe fn LZ4F_initStream(
    ctx: *mut c_void,
    cdict: *const LZ4F_CDict,
    level: c_int,
    blockMode: c_uint,
) {
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

unsafe fn ctxTypeID_to_size(ctxTypeID: c_int) -> c_int {
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
    preferencesPtr: *const LZ4F_preferences_t,
) -> usize {
    let prefNull: LZ4F_preferences_t = LZ4F_INIT_PREFERENCES;
    let dstStart = dstBuffer as *mut u8;
    let mut dstPtr = dstStart;
    let mut preferencesPtr = preferencesPtr;

    RETURN_ERROR_IF!(dstCapacity < maxFHSize, LZ4F_ERROR_dstMaxSize_tooSmall);
    if preferencesPtr.is_null() {
        preferencesPtr = &prefNull;
    }
    (*cctx).prefs = *preferencesPtr;

    /* cctx Management */
    {
        let ctxTypeID: u16 = if (*cctx).prefs.compressionLevel < LZ4HC_CLEVEL_MIN {
            1
        } else {
            2
        };
        let requiredSize = ctxTypeID_to_size(ctxTypeID as c_int);
        let allocatedSize = ctxTypeID_to_size((*cctx).lz4CtxAlloc as c_int);
        if allocatedSize < requiredSize {
            /* not enough space allocated */
            LZ4F_free((*cctx).lz4CtxPtr, (*cctx).cmem);
            if (*cctx).prefs.compressionLevel < LZ4HC_CLEVEL_MIN {
                (*cctx).lz4CtxPtr = LZ4F_malloc(SIZEOF_LZ4_STREAM_T, (*cctx).cmem);
                if !(*cctx).lz4CtxPtr.is_null() {
                    LZ4_initStream((*cctx).lz4CtxPtr, SIZEOF_LZ4_STREAM_T);
                }
            } else {
                (*cctx).lz4CtxPtr = LZ4F_malloc(SIZEOF_LZ4_STREAMHC_T, (*cctx).cmem);
                if !(*cctx).lz4CtxPtr.is_null() {
                    LZ4_initStreamHC((*cctx).lz4CtxPtr, SIZEOF_LZ4_STREAMHC_T);
                }
            }
            RETURN_ERROR_IF!((*cctx).lz4CtxPtr.is_null(), LZ4F_ERROR_allocation_failed);
            (*cctx).lz4CtxAlloc = ctxTypeID;
            (*cctx).lz4CtxType = ctxTypeID;
        } else if (*cctx).lz4CtxType != ctxTypeID {
            if (*cctx).prefs.compressionLevel < LZ4HC_CLEVEL_MIN {
                LZ4_initStream((*cctx).lz4CtxPtr, SIZEOF_LZ4_STREAM_T);
            } else {
                LZ4_initStreamHC((*cctx).lz4CtxPtr, SIZEOF_LZ4_STREAMHC_T);
                LZ4_setCompressionLevel(
                    (*cctx).lz4CtxPtr as *mut LZ4_streamHC_t,
                    (*cctx).prefs.compressionLevel,
                );
            }
            (*cctx).lz4CtxType = ctxTypeID;
        }
    }

    /* Buffer Management */
    if (*cctx).prefs.frameInfo.blockSizeID == 0 {
        (*cctx).prefs.frameInfo.blockSizeID = LZ4F_BLOCKSIZEID_DEFAULT;
    }
    (*cctx).maxBlockSize = LZ4F_getBlockSize((*cctx).prefs.frameInfo.blockSizeID);

    {
        let requiredBuffSize: usize = if (*preferencesPtr).autoFlush != 0 {
            if (*cctx).prefs.frameInfo.blockMode == LZ4F_blockLinked {
                64 * 1024
            } else {
                0
            }
        } else {
            (*cctx).maxBlockSize
                + (if (*cctx).prefs.frameInfo.blockMode == LZ4F_blockLinked {
                    128 * 1024
                } else {
                    0
                })
        };

        if (*cctx).maxBufferSize < requiredBuffSize {
            (*cctx).maxBufferSize = 0;
            LZ4F_free((*cctx).tmpBuff as *mut c_void, (*cctx).cmem);
            (*cctx).tmpBuff = LZ4F_malloc(requiredBuffSize, (*cctx).cmem) as *mut u8;
            RETURN_ERROR_IF!((*cctx).tmpBuff.is_null(), LZ4F_ERROR_allocation_failed);
            (*cctx).maxBufferSize = requiredBuffSize;
        }
    }
    (*cctx).tmpIn = (*cctx).tmpBuff;
    (*cctx).tmpInSize = 0;
    LZ4_XXH32_reset(&mut (*cctx).xxh, 0);

    /* context init */
    (*cctx).cdict = cdict;
    if (*cctx).prefs.frameInfo.blockMode == LZ4F_blockLinked {
        LZ4F_initStream(
            (*cctx).lz4CtxPtr,
            cdict,
            (*cctx).prefs.compressionLevel,
            LZ4F_blockLinked,
        );
    }
    if (*preferencesPtr).compressionLevel >= LZ4HC_CLEVEL_MIN {
        LZ4_favorDecompressionSpeed(
            (*cctx).lz4CtxPtr as *mut LZ4_streamHC_t,
            (*preferencesPtr).favorDecSpeed as c_int,
        );
    }
    if !dictBuffer.is_null() {
        RETURN_ERROR_IF!(dictSize > (c_int::MAX as usize), LZ4F_ERROR_parameter_invalid);
        if (*cctx).lz4CtxType == ctxFast {
            LZ4_loadDict(
                (*cctx).lz4CtxPtr as *mut LZ4_stream_t,
                dictBuffer as *const c_char,
                dictSize as c_int,
            );
        } else {
            LZ4_loadDictHC(
                (*cctx).lz4CtxPtr as *mut LZ4_streamHC_t,
                dictBuffer as *const c_char,
                dictSize as c_int,
            );
        }
    }

    /* Stage 2 : Write Frame Header */

    /* Magic Number */
    LZ4F_writeLE32(dstPtr as *mut c_void, LZ4F_MAGICNUMBER);
    dstPtr = dstPtr.wrapping_add(4);
    {
        let headerStart = dstPtr;

        /* FLG Byte */
        *dstPtr = (((1u32 & _2BITS) << 6)
            + (((*cctx).prefs.frameInfo.blockMode & _1BIT) << 5)
            + (((*cctx).prefs.frameInfo.blockChecksumFlag & _1BIT) << 4)
            + ((((*cctx).prefs.frameInfo.contentSize > 0) as u32) << 3)
            + (((*cctx).prefs.frameInfo.contentChecksumFlag & _1BIT) << 2)
            + (((*cctx).prefs.frameInfo.dictID > 0) as u32)) as u8;
        dstPtr = dstPtr.wrapping_add(1);
        /* BD Byte */
        *dstPtr = (((*cctx).prefs.frameInfo.blockSizeID & _3BITS) << 4) as u8;
        dstPtr = dstPtr.wrapping_add(1);
        /* Optional Frame content size field */
        if (*cctx).prefs.frameInfo.contentSize != 0 {
            LZ4F_writeLE64(dstPtr as *mut c_void, (*cctx).prefs.frameInfo.contentSize);
            dstPtr = dstPtr.wrapping_add(8);
            (*cctx).totalInSize = 0;
        }
        /* Optional dictionary ID field */
        if (*cctx).prefs.frameInfo.dictID != 0 {
            LZ4F_writeLE32(dstPtr as *mut c_void, (*cctx).prefs.frameInfo.dictID);
            dstPtr = dstPtr.wrapping_add(4);
        }
        /* Header CRC Byte */
        *dstPtr = LZ4F_headerChecksum(
            headerStart as *const c_void,
            dstPtr as usize - headerStart as usize,
        );
        dstPtr = dstPtr.wrapping_add(1);
    }

    (*cctx).cStage = 1;
    dstPtr as usize - dstStart as usize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_compressBegin(
    cctx: *mut LZ4F_cctx,
    dstBuffer: *mut c_void,
    dstCapacity: usize,
    preferencesPtr: *const LZ4F_preferences_t,
) -> usize {
    LZ4F_compressBegin_internal(
        cctx,
        dstBuffer,
        dstCapacity,
        core::ptr::null(),
        0,
        core::ptr::null(),
        preferencesPtr,
    )
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
    LZ4F_compressBegin_internal(
        cctx,
        dstBuffer,
        dstCapacity,
        dict,
        dictSize,
        core::ptr::null(),
        preferencesPtr,
    )
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
    LZ4F_compressBegin_usingDictOnce(
        cctx,
        dstBuffer,
        dstCapacity,
        dict,
        dictSize,
        preferencesPtr,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_compressBegin_usingCDict(
    cctx: *mut LZ4F_cctx,
    dstBuffer: *mut c_void,
    dstCapacity: usize,
    cdict: *const LZ4F_CDict,
    preferencesPtr: *const LZ4F_preferences_t,
) -> usize {
    LZ4F_compressBegin_internal(
        cctx,
        dstBuffer,
        dstCapacity,
        core::ptr::null(),
        0,
        cdict,
        preferencesPtr,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_compressBound(
    srcSize: usize,
    preferencesPtr: *const LZ4F_preferences_t,
) -> usize {
    if !preferencesPtr.is_null() && (*preferencesPtr).autoFlush != 0 {
        return LZ4F_compressBound_internal(srcSize, preferencesPtr, 0);
    }
    LZ4F_compressBound_internal(srcSize, preferencesPtr, usize::MAX)
}

type compressFunc_t = unsafe fn(
    ctx: *mut c_void,
    src: *const c_char,
    dst: *mut c_char,
    srcSize: c_int,
    dstSize: c_int,
    level: c_int,
    cdict: *const LZ4F_CDict,
) -> c_int;

unsafe fn LZ4F_makeBlock(
    dst: *mut c_void,
    src: *const c_void,
    srcSize: usize,
    compress: compressFunc_t,
    lz4ctx: *mut c_void,
    level: c_int,
    cdict: *const LZ4F_CDict,
    crcFlag: c_uint,
) -> usize {
    let cSizePtr = dst as *mut u8;
    let mut cSize: u32;
    cSize = compress(
        lz4ctx,
        src as *const c_char,
        cSizePtr.wrapping_add(BHSize) as *mut c_char,
        srcSize as c_int,
        (srcSize as c_int).wrapping_sub(1),
        level,
        cdict,
    ) as u32;

    if cSize == 0 || (cSize as usize) >= srcSize {
        cSize = srcSize as u32;
        LZ4F_writeLE32(cSizePtr as *mut c_void, cSize | LZ4F_BLOCKUNCOMPRESSED_FLAG);
        mem_copy(cSizePtr.wrapping_add(BHSize), src as *const u8, srcSize);
    } else {
        LZ4F_writeLE32(cSizePtr as *mut c_void, cSize);
    }
    if crcFlag != 0 {
        let crc32 = LZ4_XXH32(
            cSizePtr.wrapping_add(BHSize) as *const c_void,
            cSize as usize,
            0,
        );
        LZ4F_writeLE32(
            cSizePtr.wrapping_add(BHSize + cSize as usize) as *mut c_void,
            crc32,
        );
    }
    BHSize + cSize as usize + (crcFlag as usize) * BFSize
}

unsafe fn LZ4F_compressBlock(
    ctx: *mut c_void,
    src: *const c_char,
    dst: *mut c_char,
    srcSize: c_int,
    dstCapacity: c_int,
    level: c_int,
    cdict: *const LZ4F_CDict,
) -> c_int {
    let acceleration = if level < 0 { -level + 1 } else { 1 };
    LZ4F_initStream(ctx, cdict, level, LZ4F_blockIndependent);
    if !cdict.is_null() {
        LZ4_compress_fast_continue(
            ctx as *mut LZ4_stream_t,
            src,
            dst,
            srcSize,
            dstCapacity,
            acceleration,
        )
    } else {
        LZ4_compress_fast_extState_fastReset(ctx, src, dst, srcSize, dstCapacity, acceleration)
    }
}

unsafe fn LZ4F_compressBlock_continue(
    ctx: *mut c_void,
    src: *const c_char,
    dst: *mut c_char,
    srcSize: c_int,
    dstCapacity: c_int,
    level: c_int,
    _cdict: *const LZ4F_CDict,
) -> c_int {
    let acceleration = if level < 0 { -level + 1 } else { 1 };
    LZ4_compress_fast_continue(
        ctx as *mut LZ4_stream_t,
        src,
        dst,
        srcSize,
        dstCapacity,
        acceleration,
    )
}

unsafe fn LZ4F_compressBlockHC(
    ctx: *mut c_void,
    src: *const c_char,
    dst: *mut c_char,
    srcSize: c_int,
    dstCapacity: c_int,
    level: c_int,
    cdict: *const LZ4F_CDict,
) -> c_int {
    LZ4F_initStream(ctx, cdict, level, LZ4F_blockIndependent);
    if !cdict.is_null() {
        return LZ4_compress_HC_continue(
            ctx as *mut LZ4_streamHC_t,
            src,
            dst,
            srcSize,
            dstCapacity,
        );
    }
    LZ4_compress_HC_extStateHC_fastReset(ctx, src, dst, srcSize, dstCapacity, level)
}

unsafe fn LZ4F_compressBlockHC_continue(
    ctx: *mut c_void,
    src: *const c_char,
    dst: *mut c_char,
    srcSize: c_int,
    dstCapacity: c_int,
    _level: c_int,
    _cdict: *const LZ4F_CDict,
) -> c_int {
    LZ4_compress_HC_continue(ctx as *mut LZ4_streamHC_t, src, dst, srcSize, dstCapacity)
}

unsafe fn LZ4F_doNotCompressBlock(
    _ctx: *mut c_void,
    _src: *const c_char,
    _dst: *mut c_char,
    _srcSize: c_int,
    _dstCapacity: c_int,
    _level: c_int,
    _cdict: *const LZ4F_CDict,
) -> c_int {
    0
}

fn LZ4F_selectCompression(blockMode: c_uint, level: c_int, compressMode: u32) -> compressFunc_t {
    if compressMode == LZ4B_UNCOMPRESSED {
        return LZ4F_doNotCompressBlock as compressFunc_t;
    }
    if level < LZ4HC_CLEVEL_MIN {
        if blockMode == LZ4F_blockIndependent {
            return LZ4F_compressBlock as compressFunc_t;
        }
        return LZ4F_compressBlock_continue as compressFunc_t;
    }
    if blockMode == LZ4F_blockIndependent {
        return LZ4F_compressBlockHC as compressFunc_t;
    }
    LZ4F_compressBlockHC_continue as compressFunc_t
}

/// Save history (up to 64KB) into @tmpBuff
unsafe fn LZ4F_localSaveDict(cctxPtr: *mut LZ4F_cctx) -> c_int {
    if (*cctxPtr).prefs.compressionLevel < LZ4HC_CLEVEL_MIN {
        return LZ4_saveDict(
            (*cctxPtr).lz4CtxPtr as *mut LZ4_stream_t,
            (*cctxPtr).tmpBuff as *mut c_char,
            64 * 1024,
        );
    }
    LZ4_saveDictHC(
        (*cctxPtr).lz4CtxPtr as *mut LZ4_streamHC_t,
        (*cctxPtr).tmpBuff as *mut c_char,
        64 * 1024,
    )
}

/* LZ4F_lastBlockStatus */
const notDone: i32 = 0;
const fromTmpBuffer: i32 = 1;
const fromSrcBuffer: i32 = 2;

const k_cOptionsNull: LZ4F_compressOptions_t = LZ4F_compressOptions_t {
    stableSrc: 0,
    reserved: [0, 0, 0],
};

unsafe fn LZ4F_compressUpdateImpl(
    cctxPtr: *mut LZ4F_cctx,
    dstBuffer: *mut c_void,
    dstCapacity: usize,
    srcBuffer: *const c_void,
    srcSize: usize,
    compressOptionsPtr: *const LZ4F_compressOptions_t,
    blockCompression: u32,
) -> usize {
    let blockSize: usize = (*cctxPtr).maxBlockSize;
    let mut srcPtr = srcBuffer as *const u8;
    let srcEnd = srcPtr.wrapping_add(srcSize);
    let dstStart = dstBuffer as *mut u8;
    let mut dstPtr = dstStart;
    let mut lastBlockCompressed: i32 = notDone;
    let compress: compressFunc_t = LZ4F_selectCompression(
        (*cctxPtr).prefs.frameInfo.blockMode,
        (*cctxPtr).prefs.compressionLevel,
        blockCompression,
    );
    let mut compressOptionsPtr = compressOptionsPtr;
    let bytesWritten: usize;

    RETURN_ERROR_IF!(
        (*cctxPtr).cStage != 1,
        LZ4F_ERROR_compressionState_uninitialized
    );
    if dstCapacity
        < LZ4F_compressBound_internal(srcSize, &(*cctxPtr).prefs, (*cctxPtr).tmpInSize)
    {
        RETURN_ERROR!(LZ4F_ERROR_dstMaxSize_tooSmall);
    }

    if blockCompression == LZ4B_UNCOMPRESSED && dstCapacity < srcSize {
        RETURN_ERROR!(LZ4F_ERROR_dstMaxSize_tooSmall);
    }

    /* flush currently written block, to continue with new block compression */
    if (*cctxPtr).blockCompressMode != blockCompression {
        bytesWritten = LZ4F_flush(cctxPtr, dstBuffer, dstCapacity, compressOptionsPtr);
        dstPtr = dstPtr.wrapping_add(bytesWritten);
        (*cctxPtr).blockCompressMode = blockCompression;
    }

    if compressOptionsPtr.is_null() {
        compressOptionsPtr = &k_cOptionsNull;
    }

    /* complete tmp buffer */
    if (*cctxPtr).tmpInSize > 0 {
        let sizeToCopy: usize = blockSize - (*cctxPtr).tmpInSize;
        if sizeToCopy > srcSize {
            /* add src to tmpIn buffer */
            mem_copy(
                (*cctxPtr).tmpIn.wrapping_add((*cctxPtr).tmpInSize),
                srcBuffer as *const u8,
                srcSize,
            );
            srcPtr = srcEnd;
            (*cctxPtr).tmpInSize += srcSize;
        } else {
            /* complete tmpIn block and then compress it */
            lastBlockCompressed = fromTmpBuffer;
            mem_copy(
                (*cctxPtr).tmpIn.wrapping_add((*cctxPtr).tmpInSize),
                srcBuffer as *const u8,
                sizeToCopy,
            );
            srcPtr = srcPtr.wrapping_add(sizeToCopy);

            dstPtr = dstPtr.wrapping_add(LZ4F_makeBlock(
                dstPtr as *mut c_void,
                (*cctxPtr).tmpIn as *const c_void,
                blockSize,
                compress,
                (*cctxPtr).lz4CtxPtr,
                (*cctxPtr).prefs.compressionLevel,
                (*cctxPtr).cdict,
                (*cctxPtr).prefs.frameInfo.blockChecksumFlag,
            ));
            if (*cctxPtr).prefs.frameInfo.blockMode == LZ4F_blockLinked {
                (*cctxPtr).tmpIn = (*cctxPtr).tmpIn.wrapping_add(blockSize);
            }
            (*cctxPtr).tmpInSize = 0;
        }
    }

    while (srcEnd as usize - srcPtr as usize) >= blockSize {
        /* compress full blocks */
        lastBlockCompressed = fromSrcBuffer;
        dstPtr = dstPtr.wrapping_add(LZ4F_makeBlock(
            dstPtr as *mut c_void,
            srcPtr as *const c_void,
            blockSize,
            compress,
            (*cctxPtr).lz4CtxPtr,
            (*cctxPtr).prefs.compressionLevel,
            (*cctxPtr).cdict,
            (*cctxPtr).prefs.frameInfo.blockChecksumFlag,
        ));
        srcPtr = srcPtr.wrapping_add(blockSize);
    }

    if ((*cctxPtr).prefs.autoFlush != 0) && (srcPtr < srcEnd) {
        /* autoFlush : remaining input (< blockSize) is compressed */
        lastBlockCompressed = fromSrcBuffer;
        dstPtr = dstPtr.wrapping_add(LZ4F_makeBlock(
            dstPtr as *mut c_void,
            srcPtr as *const c_void,
            srcEnd as usize - srcPtr as usize,
            compress,
            (*cctxPtr).lz4CtxPtr,
            (*cctxPtr).prefs.compressionLevel,
            (*cctxPtr).cdict,
            (*cctxPtr).prefs.frameInfo.blockChecksumFlag,
        ));
        srcPtr = srcEnd;
    }

    /* preserve dictionary within @tmpBuff whenever necessary */
    if ((*cctxPtr).prefs.frameInfo.blockMode == LZ4F_blockLinked)
        && (lastBlockCompressed == fromSrcBuffer)
    {
        if (*compressOptionsPtr).stableSrc != 0 {
            (*cctxPtr).tmpIn = (*cctxPtr).tmpBuff;
        } else {
            let realDictSize = LZ4F_localSaveDict(cctxPtr);
            (*cctxPtr).tmpIn = (*cctxPtr).tmpBuff.wrapping_add(realDictSize as usize);
        }
    }

    /* keep tmpIn within limits */
    if ((*cctxPtr).prefs.autoFlush == 0)
        && ((*cctxPtr).tmpIn.wrapping_add(blockSize)
            > (*cctxPtr).tmpBuff.wrapping_add((*cctxPtr).maxBufferSize))
    {
        let realDictSize = LZ4F_localSaveDict(cctxPtr);
        (*cctxPtr).tmpIn = (*cctxPtr).tmpBuff.wrapping_add(realDictSize as usize);
    }

    /* some input data left, necessarily < blockSize */
    if srcPtr < srcEnd {
        let sizeToCopy: usize = srcEnd as usize - srcPtr as usize;
        mem_copy((*cctxPtr).tmpIn, srcPtr, sizeToCopy);
        (*cctxPtr).tmpInSize = sizeToCopy;
    }

    if (*cctxPtr).prefs.frameInfo.contentChecksumFlag == LZ4F_contentChecksumEnabled {
        LZ4_XXH32_update(&mut (*cctxPtr).xxh, srcBuffer, srcSize);
    }

    (*cctxPtr).totalInSize += srcSize as u64;
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
    LZ4F_compressUpdateImpl(
        cctxPtr,
        dstBuffer,
        dstCapacity,
        srcBuffer,
        srcSize,
        compressOptionsPtr,
        LZ4B_COMPRESSED,
    )
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
    LZ4F_compressUpdateImpl(
        cctxPtr,
        dstBuffer,
        dstCapacity,
        srcBuffer,
        srcSize,
        compressOptionsPtr,
        LZ4B_UNCOMPRESSED,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_flush(
    cctxPtr: *mut LZ4F_cctx,
    dstBuffer: *mut c_void,
    dstCapacity: usize,
    _compressOptionsPtr: *const LZ4F_compressOptions_t,
) -> usize {
    let dstStart = dstBuffer as *mut u8;
    let mut dstPtr = dstStart;
    let compress: compressFunc_t;

    if (*cctxPtr).tmpInSize == 0 {
        return 0;
    }
    RETURN_ERROR_IF!(
        (*cctxPtr).cStage != 1,
        LZ4F_ERROR_compressionState_uninitialized
    );
    RETURN_ERROR_IF!(
        dstCapacity < ((*cctxPtr).tmpInSize + BHSize + BFSize),
        LZ4F_ERROR_dstMaxSize_tooSmall
    );

    /* select compression function */
    compress = LZ4F_selectCompression(
        (*cctxPtr).prefs.frameInfo.blockMode,
        (*cctxPtr).prefs.compressionLevel,
        (*cctxPtr).blockCompressMode,
    );

    /* compress tmp buffer */
    dstPtr = dstPtr.wrapping_add(LZ4F_makeBlock(
        dstPtr as *mut c_void,
        (*cctxPtr).tmpIn as *const c_void,
        (*cctxPtr).tmpInSize,
        compress,
        (*cctxPtr).lz4CtxPtr,
        (*cctxPtr).prefs.compressionLevel,
        (*cctxPtr).cdict,
        (*cctxPtr).prefs.frameInfo.blockChecksumFlag,
    ));

    if (*cctxPtr).prefs.frameInfo.blockMode == LZ4F_blockLinked {
        (*cctxPtr).tmpIn = (*cctxPtr).tmpIn.wrapping_add((*cctxPtr).tmpInSize);
    }
    (*cctxPtr).tmpInSize = 0;

    /* keep tmpIn within limits */
    if (*cctxPtr).tmpIn.wrapping_add((*cctxPtr).maxBlockSize)
        > (*cctxPtr).tmpBuff.wrapping_add((*cctxPtr).maxBufferSize)
    {
        let realDictSize = LZ4F_localSaveDict(cctxPtr);
        (*cctxPtr).tmpIn = (*cctxPtr).tmpBuff.wrapping_add(realDictSize as usize);
    }

    dstPtr as usize - dstStart as usize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_compressEnd(
    cctxPtr: *mut LZ4F_cctx,
    dstBuffer: *mut c_void,
    dstCapacity: usize,
    compressOptionsPtr: *const LZ4F_compressOptions_t,
) -> usize {
    let dstStart = dstBuffer as *mut u8;
    let mut dstPtr = dstStart;
    let mut dstCapacity = dstCapacity;

    let flushSize = LZ4F_flush(cctxPtr, dstBuffer, dstCapacity, compressOptionsPtr);
    FORWARD_IF_ERROR!(flushSize);
    dstPtr = dstPtr.wrapping_add(flushSize);

    dstCapacity -= flushSize;

    RETURN_ERROR_IF!(dstCapacity < 4, LZ4F_ERROR_dstMaxSize_tooSmall);
    LZ4F_writeLE32(dstPtr as *mut c_void, 0);
    dstPtr = dstPtr.wrapping_add(4);

    if (*cctxPtr).prefs.frameInfo.contentChecksumFlag == LZ4F_contentChecksumEnabled {
        let xxh = LZ4_XXH32_digest(&(*cctxPtr).xxh);
        RETURN_ERROR_IF!(dstCapacity < 8, LZ4F_ERROR_dstMaxSize_tooSmall);
        LZ4F_writeLE32(dstPtr as *mut c_void, xxh);
        dstPtr = dstPtr.wrapping_add(4);
    }

    (*cctxPtr).cStage = 0;

    if (*cctxPtr).prefs.frameInfo.contentSize != 0 {
        if (*cctxPtr).prefs.frameInfo.contentSize != (*cctxPtr).totalInSize {
            RETURN_ERROR!(LZ4F_ERROR_frameSize_wrong);
        }
    }

    dstPtr as usize - dstStart as usize
}

/*-***************************************************
*   Frame Decompression
*****************************************************/

/* dStage_t */
const dstage_getFrameHeader: u32 = 0;
const dstage_storeFrameHeader: u32 = 1;
const dstage_init: u32 = 2;
const dstage_getBlockHeader: u32 = 3;
const dstage_storeBlockHeader: u32 = 4;
const dstage_copyDirect: u32 = 5;
const dstage_getBlockChecksum: u32 = 6;
const dstage_getCBlock: u32 = 7;
const dstage_storeCBlock: u32 = 8;
const dstage_flushOut: u32 = 9;
const dstage_getSuffix: u32 = 10;
const dstage_storeSuffix: u32 = 11;
const dstage_getSFrameSize: u32 = 12;
const dstage_storeSFrameSize: u32 = 13;
const dstage_skipSkippable: u32 = 14;

#[repr(C)]
pub struct LZ4F_dctx {
    pub cmem: LZ4F_CustomMem,
    pub frameInfo: LZ4F_frameInfo_t,
    pub version: u32,
    pub dStage: u32,
    pub frameRemainingSize: u64,
    pub maxBlockSize: usize,
    pub maxBufferSize: usize,
    pub tmpIn: *mut u8,
    pub tmpInSize: usize,
    pub tmpInTarget: usize,
    pub tmpOutBuffer: *mut u8,
    pub dict: *const u8,
    pub dictSize: usize,
    pub tmpOut: *mut u8,
    pub tmpOutSize: usize,
    pub tmpOutStart: usize,
    pub xxh: XXH32_state_t,
    pub blockChecksum: XXH32_state_t,
    pub skipChecksum: c_int,
    pub header: [u8; LZ4F_HEADER_SIZE_MAX],
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_createDecompressionContext_advanced(
    customMem: LZ4F_CustomMem,
    version: c_uint,
) -> *mut LZ4F_dctx {
    let dctx = LZ4F_calloc(core::mem::size_of::<LZ4F_dctx>(), customMem) as *mut LZ4F_dctx;
    if dctx.is_null() {
        return core::ptr::null_mut();
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
    RETURN_ERROR_IF!(
        LZ4F_decompressionContextPtr.is_null(),
        LZ4F_ERROR_parameter_null
    );

    *LZ4F_decompressionContextPtr =
        LZ4F_createDecompressionContext_advanced(LZ4F_defaultCMem, versionNumber);
    if (*LZ4F_decompressionContextPtr).is_null() {
        RETURN_ERROR!(LZ4F_ERROR_allocation_failed);
    }
    LZ4F_OK_NoError as LZ4F_errorCode_t
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_freeDecompressionContext(dctx: *mut LZ4F_dctx) -> LZ4F_errorCode_t {
    let mut result: LZ4F_errorCode_t = LZ4F_OK_NoError as LZ4F_errorCode_t;
    if !dctx.is_null() {
        result = (*dctx).dStage as LZ4F_errorCode_t;
        LZ4F_free((*dctx).tmpIn as *mut c_void, (*dctx).cmem);
        LZ4F_free((*dctx).tmpOutBuffer as *mut c_void, (*dctx).cmem);
        LZ4F_free(dctx as *mut c_void, (*dctx).cmem);
    }
    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_resetDecompressionContext(dctx: *mut LZ4F_dctx) {
    (*dctx).dStage = dstage_getFrameHeader;
    (*dctx).dict = core::ptr::null();
    (*dctx).dictSize = 0;
    (*dctx).skipChecksum = 0;
    (*dctx).frameRemainingSize = 0;
}

unsafe fn LZ4F_decodeHeader(dctx: *mut LZ4F_dctx, src: *const c_void, srcSize: usize) -> usize {
    let blockMode: u32;
    let blockChecksumFlag: u32;
    let contentSizeFlag: u32;
    let contentChecksumFlag: u32;
    let dictIDFlag: u32;
    let blockSizeID: u32;
    let frameHeaderSize: usize;
    let srcPtr = src as *const u8;

    /* need to decode header to get frameInfo */
    RETURN_ERROR_IF!(srcSize < minFHSize, LZ4F_ERROR_frameHeader_incomplete);
    mem_init(
        &mut (*dctx).frameInfo as *mut LZ4F_frameInfo_t as *mut u8,
        0,
        core::mem::size_of::<LZ4F_frameInfo_t>(),
    );

    /* special case : skippable frames */
    if (LZ4F_readLE32(srcPtr as *const c_void) & 0xFFFFFFF0u32) == LZ4F_MAGIC_SKIPPABLE_START {
        (*dctx).frameInfo.frameType = LZ4F_skippableFrame;
        if src == ((*dctx).header.as_ptr() as *const c_void) {
            (*dctx).tmpInSize = srcSize;
            (*dctx).tmpInTarget = 8;
            (*dctx).dStage = dstage_storeSFrameSize;
            return srcSize;
        } else {
            (*dctx).dStage = dstage_getSFrameSize;
            return 4;
        }
    }

    /* control magic number */
    if LZ4F_readLE32(srcPtr as *const c_void) != LZ4F_MAGICNUMBER {
        RETURN_ERROR!(LZ4F_ERROR_frameType_unknown);
    }
    (*dctx).frameInfo.frameType = LZ4F_frame;

    /* Flags */
    {
        let FLG: u32 = *srcPtr.wrapping_add(4) as u32;
        let version: u32 = (FLG >> 6) & _2BITS;
        blockChecksumFlag = (FLG >> 4) & _1BIT;
        blockMode = (FLG >> 5) & _1BIT;
        contentSizeFlag = (FLG >> 3) & _1BIT;
        contentChecksumFlag = (FLG >> 2) & _1BIT;
        dictIDFlag = FLG & _1BIT;
        /* validate */
        if ((FLG >> 1) & _1BIT) != 0 {
            RETURN_ERROR!(LZ4F_ERROR_reservedFlag_set);
        }
        if version != 1 {
            RETURN_ERROR!(LZ4F_ERROR_headerVersion_wrong);
        }
    }

    /* Frame Header Size */
    frameHeaderSize = minFHSize
        + (if contentSizeFlag != 0 { 8 } else { 0 })
        + (if dictIDFlag != 0 { 4 } else { 0 });

    if srcSize < frameHeaderSize {
        /* not enough input to fully decode frame header */
        if srcPtr != (*dctx).header.as_ptr() {
            mem_copy((*dctx).header.as_mut_ptr(), srcPtr, srcSize);
        }
        (*dctx).tmpInSize = srcSize;
        (*dctx).tmpInTarget = frameHeaderSize;
        (*dctx).dStage = dstage_storeFrameHeader;
        return srcSize;
    }

    {
        let BD: u32 = *srcPtr.wrapping_add(5) as u32;
        blockSizeID = (BD >> 4) & _3BITS;
        /* validate */
        if ((BD >> 7) & _1BIT) != 0 {
            RETURN_ERROR!(LZ4F_ERROR_reservedFlag_set);
        }
        if blockSizeID < 4 {
            RETURN_ERROR!(LZ4F_ERROR_maxBlockSize_invalid);
        }
        if ((BD >> 0) & _4BITS) != 0 {
            RETURN_ERROR!(LZ4F_ERROR_reservedFlag_set);
        }
    }

    /* check header */
    {
        let HC = LZ4F_headerChecksum(
            srcPtr.wrapping_add(4) as *const c_void,
            frameHeaderSize - 5,
        );
        RETURN_ERROR_IF!(
            HC != *srcPtr.wrapping_add(frameHeaderSize - 1),
            LZ4F_ERROR_headerChecksum_invalid
        );
    }

    /* save */
    (*dctx).frameInfo.blockMode = blockMode;
    (*dctx).frameInfo.blockChecksumFlag = blockChecksumFlag;
    (*dctx).frameInfo.contentChecksumFlag = contentChecksumFlag;
    (*dctx).frameInfo.blockSizeID = blockSizeID;
    (*dctx).maxBlockSize = LZ4F_getBlockSize(blockSizeID);
    if contentSizeFlag != 0 {
        (*dctx).frameInfo.contentSize = LZ4F_readLE64(srcPtr.wrapping_add(6) as *const c_void);
        (*dctx).frameRemainingSize = (*dctx).frameInfo.contentSize;
    }
    if dictIDFlag != 0 {
        (*dctx).frameInfo.dictID =
            LZ4F_readLE32(srcPtr.wrapping_add(frameHeaderSize - 5) as *const c_void);
    }

    (*dctx).dStage = dstage_init;

    frameHeaderSize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_headerSize(src: *const c_void, srcSize: usize) -> usize {
    RETURN_ERROR_IF!(src.is_null(), LZ4F_ERROR_srcPtr_wrong);

    /* minimal srcSize to determine header size */
    if srcSize < LZ4F_MIN_SIZE_TO_KNOW_HEADER_LENGTH {
        RETURN_ERROR!(LZ4F_ERROR_frameHeader_incomplete);
    }

    /* special case : skippable frames */
    if (LZ4F_readLE32(src) & 0xFFFFFFF0u32) == LZ4F_MAGIC_SKIPPABLE_START {
        return 8;
    }

    /* control magic number */
    if LZ4F_readLE32(src) != LZ4F_MAGICNUMBER {
        RETURN_ERROR!(LZ4F_ERROR_frameType_unknown);
    }

    /* Frame Header Size */
    {
        let FLG: u8 = *(src as *const u8).wrapping_add(4);
        let contentSizeFlag: u32 = ((FLG as u32) >> 3) & _1BIT;
        let dictIDFlag: u32 = (FLG as u32) & _1BIT;
        minFHSize
            + (if contentSizeFlag != 0 { 8 } else { 0 })
            + (if dictIDFlag != 0 { 4 } else { 0 })
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_getFrameInfo(
    dctx: *mut LZ4F_dctx,
    frameInfoPtr: *mut LZ4F_frameInfo_t,
    srcBuffer: *const c_void,
    srcSizePtr: *mut usize,
) -> LZ4F_errorCode_t {
    if (*dctx).dStage > dstage_storeFrameHeader {
        /* frameInfo already decoded */
        let mut o: usize = 0;
        let mut i: usize = 0;
        *srcSizePtr = 0;
        *frameInfoPtr = (*dctx).frameInfo;
        return LZ4F_decompress(
            dctx,
            core::ptr::null_mut(),
            &mut o,
            core::ptr::null(),
            &mut i,
            core::ptr::null(),
        );
    } else {
        if (*dctx).dStage == dstage_storeFrameHeader {
            *srcSizePtr = 0;
            RETURN_ERROR!(LZ4F_ERROR_frameDecoding_alreadyStarted);
        } else {
            let hSize = LZ4F_headerSize(srcBuffer, *srcSizePtr);
            if LZ4F_isError_(hSize) {
                *srcSizePtr = 0;
                return hSize;
            }
            if *srcSizePtr < hSize {
                *srcSizePtr = 0;
                RETURN_ERROR!(LZ4F_ERROR_frameHeader_incomplete);
            }

            {
                let mut decodeResult = LZ4F_decodeHeader(dctx, srcBuffer, hSize);
                if LZ4F_isError_(decodeResult) {
                    *srcSizePtr = 0;
                } else {
                    *srcSizePtr = decodeResult;
                    decodeResult = BHSize;
                }
                *frameInfoPtr = (*dctx).frameInfo;
                return decodeResult;
            }
        }
    }
}

/// only used for LZ4F_blockLinked mode
unsafe fn LZ4F_updateDict(
    dctx: *mut LZ4F_dctx,
    dstPtr: *const u8,
    dstSize: usize,
    dstBufferStart: *const u8,
    withinTmp: c_uint,
) {
    if (*dctx).dictSize == 0 {
        (*dctx).dict = dstPtr;
    }

    if (*dctx).dict.wrapping_add((*dctx).dictSize) == dstPtr {
        /* prefix mode, everything within dstBuffer */
        (*dctx).dictSize += dstSize;
        return;
    }

    if (dstPtr as usize - dstBufferStart as usize) + dstSize >= 64 * 1024 {
        /* history in dstBuffer becomes large enough to become dictionary */
        (*dctx).dict = dstBufferStart;
        (*dctx).dictSize = (dstPtr as usize - dstBufferStart as usize) + dstSize;
        return;
    }

    if withinTmp != 0 && ((*dctx).dict == ((*dctx).tmpOutBuffer as *const u8)) {
        /* continue history within tmpOutBuffer */
        (*dctx).dictSize += dstSize;
        return;
    }

    if withinTmp != 0 {
        /* copy relevant dict portion in front of tmpOut within tmpOutBuffer */
        let preserveSize: usize = (*dctx).tmpOut as usize - (*dctx).tmpOutBuffer as usize;
        let mut copySize: usize = (64 * 1024usize).wrapping_sub((*dctx).tmpOutSize);
        let oldDictEnd: *const u8 = (*dctx)
            .dict
            .wrapping_add((*dctx).dictSize)
            .wrapping_sub((*dctx).tmpOutStart);
        if (*dctx).tmpOutSize > 64 * 1024 {
            copySize = 0;
        }
        if copySize > preserveSize {
            copySize = preserveSize;
        }

        mem_copy(
            (*dctx)
                .tmpOutBuffer
                .wrapping_add(preserveSize)
                .wrapping_sub(copySize),
            oldDictEnd.wrapping_sub(copySize),
            copySize,
        );

        (*dctx).dict = (*dctx).tmpOutBuffer as *const u8;
        (*dctx).dictSize = preserveSize + (*dctx).tmpOutStart + dstSize;
        return;
    }

    if (*dctx).dict == ((*dctx).tmpOutBuffer as *const u8) {
        /* copy dst into tmp to complete dict */
        if (*dctx).dictSize + dstSize > (*dctx).maxBufferSize {
            /* tmp buffer not large enough */
            let preserveSize: usize = (64 * 1024usize).wrapping_sub(dstSize);
            mem_copy(
                (*dctx).tmpOutBuffer,
                (*dctx)
                    .dict
                    .wrapping_add((*dctx).dictSize)
                    .wrapping_sub(preserveSize),
                preserveSize,
            );
            (*dctx).dictSize = preserveSize;
        }
        mem_copy(
            (*dctx).tmpOutBuffer.wrapping_add((*dctx).dictSize),
            dstPtr,
            dstSize,
        );
        (*dctx).dictSize += dstSize;
        return;
    }

    /* join dict & dest into tmp */
    {
        let mut preserveSize: usize = (64 * 1024usize).wrapping_sub(dstSize);
        if preserveSize > (*dctx).dictSize {
            preserveSize = (*dctx).dictSize;
        }
        mem_copy(
            (*dctx).tmpOutBuffer,
            (*dctx)
                .dict
                .wrapping_add((*dctx).dictSize)
                .wrapping_sub(preserveSize),
            preserveSize,
        );
        mem_copy(
            (*dctx).tmpOutBuffer.wrapping_add(preserveSize),
            dstPtr,
            dstSize,
        );
        (*dctx).dict = (*dctx).tmpOutBuffer as *const u8;
        (*dctx).dictSize = preserveSize + dstSize;
    }
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
    let optionsNull: LZ4F_decompressOptions_t;
    let srcStart = srcBuffer as *const u8;
    let srcEnd = srcStart.wrapping_add(*srcSizePtr);
    let mut srcPtr = srcStart;
    let dstStart = dstBuffer as *mut u8;
    let dstEnd: *mut u8 = if !dstStart.is_null() {
        dstStart.wrapping_add(*dstSizePtr)
    } else {
        core::ptr::null_mut()
    };
    let mut dstPtr = dstStart;
    let mut selectedIn: *const u8 = core::ptr::null();
    let mut doAnotherStage: c_uint = 1;
    let mut nextSrcSizeHint: usize = 1;

    optionsNull = core::mem::zeroed();
    let decompressOptionsPtr: *const LZ4F_decompressOptions_t = if decompressOptionsPtr.is_null() {
        &optionsNull
    } else {
        decompressOptionsPtr
    };
    *srcSizePtr = 0;
    *dstSizePtr = 0;
    (*dctx).skipChecksum |= ((*decompressOptionsPtr).skipChecksums != 0) as c_int;

    /* behaves as a state machine */
    while doAnotherStage != 0 {
        let mut st: u32 = (*dctx).dStage;
        'sw: loop {
            match st {
                x if x == dstage_getFrameHeader => {
                    if (srcEnd as usize - srcPtr as usize) >= maxFHSize {
                        /* enough to decode - shortcut */
                        let hSize =
                            LZ4F_decodeHeader(dctx, srcPtr as *const c_void, srcEnd as usize - srcPtr as usize);
                        FORWARD_IF_ERROR!(hSize);
                        srcPtr = srcPtr.wrapping_add(hSize);
                        break 'sw;
                    }
                    (*dctx).tmpInSize = 0;
                    if srcEnd as usize - srcPtr as usize == 0 {
                        return minFHSize;
                    }
                    (*dctx).tmpInTarget = minFHSize;
                    (*dctx).dStage = dstage_storeFrameHeader;
                    st = dstage_storeFrameHeader;
                    continue 'sw;
                }

                x if x == dstage_storeFrameHeader => {
                    {
                        let sizeToCopy = MINuz(
                            (*dctx).tmpInTarget - (*dctx).tmpInSize,
                            srcEnd as usize - srcPtr as usize,
                        );
                        mem_copy(
                            (*dctx).header.as_mut_ptr().wrapping_add((*dctx).tmpInSize),
                            srcPtr,
                            sizeToCopy,
                        );
                        (*dctx).tmpInSize += sizeToCopy;
                        srcPtr = srcPtr.wrapping_add(sizeToCopy);
                    }
                    if (*dctx).tmpInSize < (*dctx).tmpInTarget {
                        nextSrcSizeHint = ((*dctx).tmpInTarget - (*dctx).tmpInSize) + BHSize;
                        doAnotherStage = 0;
                        break 'sw;
                    }
                    FORWARD_IF_ERROR!(LZ4F_decodeHeader(
                        dctx,
                        (*dctx).header.as_ptr() as *const c_void,
                        (*dctx).tmpInTarget
                    ));
                    break 'sw;
                }

                x if x == dstage_init => {
                    if (*dctx).frameInfo.contentChecksumFlag != 0 {
                        LZ4_XXH32_reset(&mut (*dctx).xxh, 0);
                    }
                    /* internal buffers allocation */
                    {
                        let bufferNeeded: usize = (*dctx).maxBlockSize
                            + (if (*dctx).frameInfo.blockMode == LZ4F_blockLinked {
                                128 * 1024
                            } else {
                                0
                            });
                        if bufferNeeded > (*dctx).maxBufferSize {
                            (*dctx).maxBufferSize = 0;
                            LZ4F_free((*dctx).tmpIn as *mut c_void, (*dctx).cmem);
                            (*dctx).tmpIn =
                                LZ4F_malloc((*dctx).maxBlockSize + BFSize, (*dctx).cmem)
                                    as *mut u8;
                            RETURN_ERROR_IF!((*dctx).tmpIn.is_null(), LZ4F_ERROR_allocation_failed);
                            LZ4F_free((*dctx).tmpOutBuffer as *mut c_void, (*dctx).cmem);
                            (*dctx).tmpOutBuffer =
                                LZ4F_malloc(bufferNeeded, (*dctx).cmem) as *mut u8;
                            RETURN_ERROR_IF!(
                                (*dctx).tmpOutBuffer.is_null(),
                                LZ4F_ERROR_allocation_failed
                            );
                            (*dctx).maxBufferSize = bufferNeeded;
                        }
                    }
                    (*dctx).tmpInSize = 0;
                    (*dctx).tmpInTarget = 0;
                    (*dctx).tmpOut = (*dctx).tmpOutBuffer;
                    (*dctx).tmpOutStart = 0;
                    (*dctx).tmpOutSize = 0;

                    (*dctx).dStage = dstage_getBlockHeader;
                    st = dstage_getBlockHeader;
                    continue 'sw;
                }

                x if x == dstage_getBlockHeader || x == dstage_storeBlockHeader => {
                    let mut do_store = true;
                    if st == dstage_getBlockHeader {
                        if (srcEnd as usize - srcPtr as usize) >= BHSize {
                            selectedIn = srcPtr;
                            srcPtr = srcPtr.wrapping_add(BHSize);
                        } else {
                            /* not enough input to read cBlockSize field */
                            (*dctx).tmpInSize = 0;
                            (*dctx).dStage = dstage_storeBlockHeader;
                        }
                        do_store = (*dctx).dStage == dstage_storeBlockHeader;
                    }

                    if do_store {
                        let remainingInput: usize = srcEnd as usize - srcPtr as usize;
                        let wantedData: usize = BHSize - (*dctx).tmpInSize;
                        let sizeToCopy: usize = MINuz(wantedData, remainingInput);
                        mem_copy(
                            (*dctx).tmpIn.wrapping_add((*dctx).tmpInSize),
                            srcPtr,
                            sizeToCopy,
                        );
                        srcPtr = srcPtr.wrapping_add(sizeToCopy);
                        (*dctx).tmpInSize += sizeToCopy;

                        if (*dctx).tmpInSize < BHSize {
                            nextSrcSizeHint = BHSize - (*dctx).tmpInSize;
                            doAnotherStage = 0;
                            break 'sw;
                        }
                        selectedIn = (*dctx).tmpIn;
                    }

                    /* decode block header */
                    {
                        let blockHeader: u32 = LZ4F_readLE32(selectedIn as *const c_void);
                        let nextCBlockSize: usize = (blockHeader & 0x7FFFFFFFu32) as usize;
                        let crcSize: usize =
                            (*dctx).frameInfo.blockChecksumFlag as usize * BFSize;
                        if blockHeader == 0 {
                            /* frameEnd signal, no more block */
                            (*dctx).dStage = dstage_getSuffix;
                            break 'sw;
                        }
                        if nextCBlockSize > (*dctx).maxBlockSize {
                            RETURN_ERROR!(LZ4F_ERROR_maxBlockSize_invalid);
                        }
                        if (blockHeader & LZ4F_BLOCKUNCOMPRESSED_FLAG) != 0 {
                            /* next block is uncompressed */
                            (*dctx).tmpInTarget = nextCBlockSize;
                            if (*dctx).frameInfo.blockChecksumFlag != 0 {
                                LZ4_XXH32_reset(&mut (*dctx).blockChecksum, 0);
                            }
                            (*dctx).dStage = dstage_copyDirect;
                            break 'sw;
                        }
                        /* next block is a compressed block */
                        (*dctx).tmpInTarget = nextCBlockSize + crcSize;
                        (*dctx).dStage = dstage_getCBlock;
                        if dstPtr == dstEnd || srcPtr == srcEnd {
                            nextSrcSizeHint = BHSize + nextCBlockSize + crcSize;
                            doAnotherStage = 0;
                        }
                        break 'sw;
                    }
                }

                x if x == dstage_copyDirect => {
                    {
                        let sizeToCopy: usize;
                        if dstPtr.is_null() {
                            sizeToCopy = 0;
                        } else {
                            let minBuffSize = MINuz(
                                srcEnd as usize - srcPtr as usize,
                                dstEnd as usize - dstPtr as usize,
                            );
                            sizeToCopy = MINuz((*dctx).tmpInTarget, minBuffSize);
                            mem_copy(dstPtr, srcPtr, sizeToCopy);
                            if (*dctx).skipChecksum == 0 {
                                if (*dctx).frameInfo.blockChecksumFlag != 0 {
                                    LZ4_XXH32_update(
                                        &mut (*dctx).blockChecksum,
                                        srcPtr as *const c_void,
                                        sizeToCopy,
                                    );
                                }
                                if (*dctx).frameInfo.contentChecksumFlag != 0 {
                                    LZ4_XXH32_update(
                                        &mut (*dctx).xxh,
                                        srcPtr as *const c_void,
                                        sizeToCopy,
                                    );
                                }
                            }
                            if (*dctx).frameInfo.contentSize != 0 {
                                (*dctx).frameRemainingSize -= sizeToCopy as u64;
                            }

                            /* history management (linked blocks only) */
                            if (*dctx).frameInfo.blockMode == LZ4F_blockLinked {
                                LZ4F_updateDict(dctx, dstPtr, sizeToCopy, dstStart, 0);
                            }
                            srcPtr = srcPtr.wrapping_add(sizeToCopy);
                            dstPtr = dstPtr.wrapping_add(sizeToCopy);
                        }
                        if sizeToCopy == (*dctx).tmpInTarget {
                            /* all done */
                            if (*dctx).frameInfo.blockChecksumFlag != 0 {
                                (*dctx).tmpInSize = 0;
                                (*dctx).dStage = dstage_getBlockChecksum;
                            } else {
                                (*dctx).dStage = dstage_getBlockHeader;
                            }
                            break 'sw;
                        }
                        (*dctx).tmpInTarget -= sizeToCopy;
                    }
                    nextSrcSizeHint = (*dctx).tmpInTarget
                        + (if (*dctx).frameInfo.blockChecksumFlag != 0 {
                            BFSize
                        } else {
                            0
                        })
                        + BHSize;
                    doAnotherStage = 0;
                    break 'sw;
                }

                x if x == dstage_getBlockChecksum => {
                    {
                        let crcSrc: *const c_void;
                        if ((srcEnd as usize - srcPtr as usize) >= 4) && ((*dctx).tmpInSize == 0) {
                            crcSrc = srcPtr as *const c_void;
                            srcPtr = srcPtr.wrapping_add(4);
                        } else {
                            let stillToCopy: usize = 4 - (*dctx).tmpInSize;
                            let sizeToCopy: usize =
                                MINuz(stillToCopy, srcEnd as usize - srcPtr as usize);
                            mem_copy(
                                (*dctx).header.as_mut_ptr().wrapping_add((*dctx).tmpInSize),
                                srcPtr,
                                sizeToCopy,
                            );
                            (*dctx).tmpInSize += sizeToCopy;
                            srcPtr = srcPtr.wrapping_add(sizeToCopy);
                            if (*dctx).tmpInSize < 4 {
                                doAnotherStage = 0;
                                break 'sw;
                            }
                            crcSrc = (*dctx).header.as_ptr() as *const c_void;
                        }
                        if (*dctx).skipChecksum == 0 {
                            let readCRC = LZ4F_readLE32(crcSrc);
                            let calcCRC = LZ4_XXH32_digest(&(*dctx).blockChecksum);
                            if readCRC != calcCRC {
                                RETURN_ERROR!(LZ4F_ERROR_blockChecksum_invalid);
                            }
                        }
                    }
                    (*dctx).dStage = dstage_getBlockHeader;
                    break 'sw;
                }

                x if x == dstage_getCBlock || x == dstage_storeCBlock => {
                    let do_store = st == dstage_storeCBlock;
                    if st == dstage_getCBlock {
                        if (srcEnd as usize - srcPtr as usize) < (*dctx).tmpInTarget {
                            (*dctx).tmpInSize = 0;
                            (*dctx).dStage = dstage_storeCBlock;
                            break 'sw;
                        }
                        /* input large enough to read full block directly */
                        selectedIn = srcPtr;
                        srcPtr = srcPtr.wrapping_add((*dctx).tmpInTarget);
                    }

                    if do_store {
                        let wantedData: usize = (*dctx).tmpInTarget - (*dctx).tmpInSize;
                        let inputLeft: usize = srcEnd as usize - srcPtr as usize;
                        let sizeToCopy: usize = MINuz(wantedData, inputLeft);
                        mem_copy(
                            (*dctx).tmpIn.wrapping_add((*dctx).tmpInSize),
                            srcPtr,
                            sizeToCopy,
                        );
                        (*dctx).tmpInSize += sizeToCopy;
                        srcPtr = srcPtr.wrapping_add(sizeToCopy);
                        if (*dctx).tmpInSize < (*dctx).tmpInTarget {
                            nextSrcSizeHint = ((*dctx).tmpInTarget - (*dctx).tmpInSize)
                                + (if (*dctx).frameInfo.blockChecksumFlag != 0 {
                                    BFSize
                                } else {
                                    0
                                })
                                + BHSize;
                            doAnotherStage = 0;
                            break 'sw;
                        }
                        selectedIn = (*dctx).tmpIn;
                    }

                    /* At this stage, input is large enough to decode a block */

                    /* First, decode and control block checksum if it exists */
                    if (*dctx).frameInfo.blockChecksumFlag != 0 {
                        (*dctx).tmpInTarget -= 4;
                        {
                            let readBlockCrc = LZ4F_readLE32(
                                selectedIn.wrapping_add((*dctx).tmpInTarget) as *const c_void,
                            );
                            let calcBlockCrc = LZ4_XXH32(
                                selectedIn as *const c_void,
                                (*dctx).tmpInTarget,
                                0,
                            );
                            RETURN_ERROR_IF!(
                                readBlockCrc != calcBlockCrc,
                                LZ4F_ERROR_blockChecksum_invalid
                            );
                        }
                    }

                    /* decode directly into destination buffer if there is enough room */
                    if ((dstEnd as usize).wrapping_sub(dstPtr as usize) >= (*dctx).maxBlockSize)
                        && !(!(*dctx).dict.is_null()
                            && (*dctx).dict.wrapping_add((*dctx).dictSize)
                                == ((*dctx).tmpOut as *const u8))
                    {
                        let mut dict = (*dctx).dict as *const c_char;
                        let mut dictSize = (*dctx).dictSize;
                        let decodedSize: c_int;
                        if !dict.is_null() && dictSize > (1usize << 30) {
                            dict = dict.wrapping_add(dictSize - 64 * 1024);
                            dictSize = 64 * 1024;
                        }
                        decodedSize = LZ4_decompress_safe_usingDict(
                            selectedIn as *const c_char,
                            dstPtr as *mut c_char,
                            (*dctx).tmpInTarget as c_int,
                            (*dctx).maxBlockSize as c_int,
                            dict,
                            dictSize as c_int,
                        );
                        RETURN_ERROR_IF!(decodedSize < 0, LZ4F_ERROR_decompressionFailed);
                        if ((*dctx).frameInfo.contentChecksumFlag != 0)
                            && ((*dctx).skipChecksum == 0)
                        {
                            LZ4_XXH32_update(
                                &mut (*dctx).xxh,
                                dstPtr as *const c_void,
                                decodedSize as usize,
                            );
                        }
                        if (*dctx).frameInfo.contentSize != 0 {
                            (*dctx).frameRemainingSize -= decodedSize as u64;
                        }

                        /* dictionary management */
                        if (*dctx).frameInfo.blockMode == LZ4F_blockLinked {
                            LZ4F_updateDict(dctx, dstPtr, decodedSize as usize, dstStart, 0);
                        }

                        dstPtr = dstPtr.wrapping_add(decodedSize as usize);
                        (*dctx).dStage = dstage_getBlockHeader;
                        break 'sw;
                    }

                    /* not enough place into dst : decode into tmpOut */

                    /* manage dictionary */
                    if (*dctx).frameInfo.blockMode == LZ4F_blockLinked {
                        if (*dctx).dict == ((*dctx).tmpOutBuffer as *const u8) {
                            /* truncate dictionary to 64 KB if too big */
                            if (*dctx).dictSize > 128 * 1024 {
                                mem_copy(
                                    (*dctx).tmpOutBuffer,
                                    (*dctx)
                                        .dict
                                        .wrapping_add((*dctx).dictSize)
                                        .wrapping_sub(64 * 1024),
                                    64 * 1024,
                                );
                                (*dctx).dictSize = 64 * 1024;
                            }
                            (*dctx).tmpOut = (*dctx).tmpOutBuffer.wrapping_add((*dctx).dictSize);
                        } else {
                            let reservedDictSpace = MINuz((*dctx).dictSize, 64 * 1024);
                            (*dctx).tmpOut = (*dctx).tmpOutBuffer.wrapping_add(reservedDictSpace);
                        }
                    }

                    /* Decode block into tmpOut */
                    {
                        let mut dict = (*dctx).dict as *const c_char;
                        let mut dictSize = (*dctx).dictSize;
                        let decodedSize: c_int;
                        if !dict.is_null() && dictSize > (1usize << 30) {
                            dict = dict.wrapping_add(dictSize - 64 * 1024);
                            dictSize = 64 * 1024;
                        }
                        decodedSize = LZ4_decompress_safe_usingDict(
                            selectedIn as *const c_char,
                            (*dctx).tmpOut as *mut c_char,
                            (*dctx).tmpInTarget as c_int,
                            (*dctx).maxBlockSize as c_int,
                            dict,
                            dictSize as c_int,
                        );
                        RETURN_ERROR_IF!(decodedSize < 0, LZ4F_ERROR_decompressionFailed);
                        if (*dctx).frameInfo.contentChecksumFlag != 0 && (*dctx).skipChecksum == 0
                        {
                            LZ4_XXH32_update(
                                &mut (*dctx).xxh,
                                (*dctx).tmpOut as *const c_void,
                                decodedSize as usize,
                            );
                        }
                        if (*dctx).frameInfo.contentSize != 0 {
                            (*dctx).frameRemainingSize -= decodedSize as u64;
                        }
                        (*dctx).tmpOutSize = decodedSize as usize;
                        (*dctx).tmpOutStart = 0;
                        (*dctx).dStage = dstage_flushOut;
                    }
                    st = dstage_flushOut;
                    continue 'sw;
                }

                x if x == dstage_flushOut => {
                    if !dstPtr.is_null() {
                        let sizeToCopy = MINuz(
                            (*dctx).tmpOutSize - (*dctx).tmpOutStart,
                            dstEnd as usize - dstPtr as usize,
                        );
                        mem_copy(
                            dstPtr,
                            (*dctx).tmpOut.wrapping_add((*dctx).tmpOutStart),
                            sizeToCopy,
                        );

                        /* dictionary management */
                        if (*dctx).frameInfo.blockMode == LZ4F_blockLinked {
                            LZ4F_updateDict(dctx, dstPtr, sizeToCopy, dstStart, 1);
                        }

                        (*dctx).tmpOutStart += sizeToCopy;
                        dstPtr = dstPtr.wrapping_add(sizeToCopy);
                    }
                    if (*dctx).tmpOutStart == (*dctx).tmpOutSize {
                        /* all flushed */
                        (*dctx).dStage = dstage_getBlockHeader;
                        break 'sw;
                    }
                    /* could not flush everything */
                    doAnotherStage = 0;
                    nextSrcSizeHint = BHSize;
                    break 'sw;
                }

                x if x == dstage_getSuffix || x == dstage_storeSuffix => {
                    let mut do_store = true;
                    if st == dstage_getSuffix {
                        RETURN_ERROR_IF!(
                            (*dctx).frameRemainingSize != 0,
                            LZ4F_ERROR_frameSize_wrong
                        );
                        if (*dctx).frameInfo.contentChecksumFlag == 0 {
                            /* no checksum, frame is completed */
                            nextSrcSizeHint = 0;
                            LZ4F_resetDecompressionContext(dctx);
                            doAnotherStage = 0;
                            break 'sw;
                        }
                        if (srcEnd as usize - srcPtr as usize) < 4 {
                            /* not enough size for entire CRC */
                            (*dctx).tmpInSize = 0;
                            (*dctx).dStage = dstage_storeSuffix;
                        } else {
                            selectedIn = srcPtr;
                            srcPtr = srcPtr.wrapping_add(4);
                        }
                        do_store = (*dctx).dStage == dstage_storeSuffix;
                    }

                    if do_store {
                        let remainingInput: usize = srcEnd as usize - srcPtr as usize;
                        let wantedData: usize = 4 - (*dctx).tmpInSize;
                        let sizeToCopy: usize = MINuz(wantedData, remainingInput);
                        mem_copy(
                            (*dctx).tmpIn.wrapping_add((*dctx).tmpInSize),
                            srcPtr,
                            sizeToCopy,
                        );
                        srcPtr = srcPtr.wrapping_add(sizeToCopy);
                        (*dctx).tmpInSize += sizeToCopy;
                        if (*dctx).tmpInSize < 4 {
                            nextSrcSizeHint = 4 - (*dctx).tmpInSize;
                            doAnotherStage = 0;
                            break 'sw;
                        }
                        selectedIn = (*dctx).tmpIn;
                    }

                    /* case dstage_checkSuffix */
                    if (*dctx).skipChecksum == 0 {
                        let readCRC = LZ4F_readLE32(selectedIn as *const c_void);
                        let resultCRC = LZ4_XXH32_digest(&(*dctx).xxh);
                        RETURN_ERROR_IF!(readCRC != resultCRC, LZ4F_ERROR_contentChecksum_invalid);
                    }
                    nextSrcSizeHint = 0;
                    LZ4F_resetDecompressionContext(dctx);
                    doAnotherStage = 0;
                    break 'sw;
                }

                x if x == dstage_getSFrameSize || x == dstage_storeSFrameSize => {
                    let mut do_store = true;
                    if st == dstage_getSFrameSize {
                        if (srcEnd as usize - srcPtr as usize) >= 4 {
                            selectedIn = srcPtr;
                            srcPtr = srcPtr.wrapping_add(4);
                        } else {
                            /* not enough input to read cBlockSize field */
                            (*dctx).tmpInSize = 4;
                            (*dctx).tmpInTarget = 8;
                            (*dctx).dStage = dstage_storeSFrameSize;
                        }
                        do_store = (*dctx).dStage == dstage_storeSFrameSize;
                    }

                    if do_store {
                        let sizeToCopy = MINuz(
                            (*dctx).tmpInTarget - (*dctx).tmpInSize,
                            srcEnd as usize - srcPtr as usize,
                        );
                        mem_copy(
                            (*dctx).header.as_mut_ptr().wrapping_add((*dctx).tmpInSize),
                            srcPtr,
                            sizeToCopy,
                        );
                        srcPtr = srcPtr.wrapping_add(sizeToCopy);
                        (*dctx).tmpInSize += sizeToCopy;
                        if (*dctx).tmpInSize < (*dctx).tmpInTarget {
                            /* not enough input to get full sBlockSize; wait for more */
                            nextSrcSizeHint = (*dctx).tmpInTarget - (*dctx).tmpInSize;
                            doAnotherStage = 0;
                            break 'sw;
                        }
                        selectedIn = (*dctx).header.as_ptr().wrapping_add(4);
                    }

                    /* case dstage_decodeSFrameSize */
                    {
                        let SFrameSize = LZ4F_readLE32(selectedIn as *const c_void) as usize;
                        (*dctx).frameInfo.contentSize = SFrameSize as u64;
                        (*dctx).tmpInTarget = SFrameSize;
                        (*dctx).dStage = dstage_skipSkippable;
                        break 'sw;
                    }
                }

                x if x == dstage_skipSkippable => {
                    let skipSize = MINuz((*dctx).tmpInTarget, srcEnd as usize - srcPtr as usize);
                    srcPtr = srcPtr.wrapping_add(skipSize);
                    (*dctx).tmpInTarget -= skipSize;
                    doAnotherStage = 0;
                    nextSrcSizeHint = (*dctx).tmpInTarget;
                    if nextSrcSizeHint != 0 {
                        break 'sw;
                    }
                    /* frame fully skipped : prepare context for a new frame */
                    LZ4F_resetDecompressionContext(dctx);
                    break 'sw;
                }

                _ => {
                    break 'sw;
                }
            }
        }
    }

    /* preserve history within tmpOut whenever necessary */
    if ((*dctx).frameInfo.blockMode == LZ4F_blockLinked)
        && ((*dctx).dict != ((*dctx).tmpOutBuffer as *const u8))
        && (!(*dctx).dict.is_null())
        && ((*decompressOptionsPtr).stableDst == 0)
        && (((*dctx).dStage.wrapping_sub(2)) < (dstage_getSuffix.wrapping_sub(2)))
    {
        if (*dctx).dStage == dstage_flushOut {
            let preserveSize: usize = (*dctx).tmpOut as usize - (*dctx).tmpOutBuffer as usize;
            let mut copySize: usize = (64 * 1024usize).wrapping_sub((*dctx).tmpOutSize);
            let oldDictEnd: *const u8 = (*dctx)
                .dict
                .wrapping_add((*dctx).dictSize)
                .wrapping_sub((*dctx).tmpOutStart);
            if (*dctx).tmpOutSize > 64 * 1024 {
                copySize = 0;
            }
            if copySize > preserveSize {
                copySize = preserveSize;
            }

            mem_copy(
                (*dctx)
                    .tmpOutBuffer
                    .wrapping_add(preserveSize)
                    .wrapping_sub(copySize),
                oldDictEnd.wrapping_sub(copySize),
                copySize,
            );

            (*dctx).dict = (*dctx).tmpOutBuffer as *const u8;
            (*dctx).dictSize = preserveSize + (*dctx).tmpOutStart;
        } else {
            let oldDictEnd: *const u8 = (*dctx).dict.wrapping_add((*dctx).dictSize);
            let newDictSize: usize = MINuz((*dctx).dictSize, 64 * 1024);

            mem_copy(
                (*dctx).tmpOutBuffer,
                oldDictEnd.wrapping_sub(newDictSize),
                newDictSize,
            );

            (*dctx).dict = (*dctx).tmpOutBuffer as *const u8;
            (*dctx).dictSize = newDictSize;
            (*dctx).tmpOut = (*dctx).tmpOutBuffer.wrapping_add(newDictSize);
        }
    }

    *srcSizePtr = srcPtr as usize - srcStart as usize;
    *dstSizePtr = dstPtr as usize - dstStart as usize;
    nextSrcSizeHint
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
    if (*dctx).dStage <= dstage_init {
        (*dctx).dict = dict as *const u8;
        (*dctx).dictSize = dictSize;
    }
    LZ4F_decompress(
        dctx,
        dstBuffer,
        dstSizePtr,
        srcBuffer,
        srcSizePtr,
        decompressOptionsPtr,
    )
}
