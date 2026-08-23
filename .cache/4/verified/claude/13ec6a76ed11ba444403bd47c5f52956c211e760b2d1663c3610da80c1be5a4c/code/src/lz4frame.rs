//! Translation of lz4frame.c (LZ4F_HEAPMODE == 0)

use crate::common::*;
use crate::lz4::{
    LZ4_attach_dictionary, LZ4_compress_fast_continue, LZ4_compress_fast_extState_fastReset,
    LZ4_decompress_safe_usingDict, LZ4_initStream, LZ4_loadDict, LZ4_loadDictSlow,
    LZ4_resetStream_fast, LZ4_saveDict,
};
use crate::lz4hc::{
    LZ4_attach_HC_dictionary, LZ4_compress_HC_continue, LZ4_compress_HC_extStateHC_fastReset,
    LZ4_favorDecompressionSpeed, LZ4_initStreamHC, LZ4_loadDictHC, LZ4_resetStreamHC_fast,
    LZ4_saveDictHC, LZ4_setCompressionLevel,
};
use crate::xxhash::{xxh32, xxh32_digest, xxh32_reset, xxh32_update, XXH32_state_t};
use core::ffi::{c_char, c_int, c_void};

pub const LZ4F_VERSION: u32 = 100;

pub const LZ4F_HEADER_SIZE_MIN: usize = 7;
pub const LZ4F_HEADER_SIZE_MAX: usize = 19;
pub const LZ4F_BLOCK_HEADER_SIZE: usize = 4;
pub const LZ4F_BLOCK_CHECKSUM_SIZE: usize = 4;
pub const LZ4F_CONTENT_CHECKSUM_SIZE: usize = 4;

pub const LZ4F_MAGICNUMBER: u32 = 0x184D2204;
pub const LZ4F_MAGIC_SKIPPABLE_START: u32 = 0x184D2A50;
pub const LZ4F_MIN_SIZE_TO_KNOW_HEADER_LENGTH: usize = 5;

const _1BIT: u32 = 0x01;
const _2BITS: u32 = 0x03;
const _3BITS: u32 = 0x07;
const _4BITS: u32 = 0x0F;
const _8BITS: u32 = 0xFF;

const LZ4F_BLOCKUNCOMPRESSED_FLAG: u32 = 0x80000000;

/* LZ4F_blockSizeID_t */
pub const LZ4F_default: c_int = 0;
pub const LZ4F_max64KB: c_int = 4;
pub const LZ4F_max256KB: c_int = 5;
pub const LZ4F_max1MB: c_int = 6;
pub const LZ4F_max4MB: c_int = 7;
const LZ4F_BLOCKSIZEID_DEFAULT: c_int = LZ4F_max64KB;

/* LZ4F_blockMode_t */
pub const LZ4F_blockLinked: c_int = 0;
pub const LZ4F_blockIndependent: c_int = 1;

/* LZ4F_contentChecksum_t */
pub const LZ4F_noContentChecksum: c_int = 0;
pub const LZ4F_contentChecksumEnabled: c_int = 1;

/* LZ4F_blockChecksum_t */
pub const LZ4F_noBlockChecksum: c_int = 0;
pub const LZ4F_blockChecksumEnabled: c_int = 1;

/* LZ4F_frameType_t */
pub const LZ4F_frame: c_int = 0;
pub const LZ4F_skippableFrame: c_int = 1;

/* LZ4F_BlockCompressMode_e */
const LZ4B_COMPRESSED: c_int = 0;
const LZ4B_UNCOMPRESSED: c_int = 1;

/* LZ4F_CtxType_e */
const ctxNone: u16 = 0;
const ctxFast: u16 = 1;
const ctxHC: u16 = 2;

const minFHSize: usize = LZ4F_HEADER_SIZE_MIN;
const maxFHSize: usize = LZ4F_HEADER_SIZE_MAX;
const BHSize: usize = LZ4F_BLOCK_HEADER_SIZE;
const BFSize: usize = LZ4F_BLOCK_CHECKSUM_SIZE;

/* ================================================================ *
 *  Public types
 * ================================================================ */

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

#[repr(C)]
#[derive(Clone, Copy)]
pub struct LZ4F_frameInfo_t {
    pub blockSizeID: c_int,
    pub blockMode: c_int,
    pub contentChecksumFlag: c_int,
    pub frameType: c_int,
    pub contentSize: u64,
    pub dictID: u32,
    pub blockChecksumFlag: c_int,
}

impl LZ4F_frameInfo_t {
    const fn zeroed() -> Self {
        LZ4F_frameInfo_t {
            blockSizeID: 0,
            blockMode: 0,
            contentChecksumFlag: 0,
            frameType: 0,
            contentSize: 0,
            dictID: 0,
            blockChecksumFlag: 0,
        }
    }
    const fn init() -> Self {
        /* LZ4F_INIT_FRAMEINFO */
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
    pub autoFlush: u32,
    pub favorDecSpeed: u32,
    pub reserved: [u32; 3],
}

impl LZ4F_preferences_t {
    const fn zeroed() -> Self {
        LZ4F_preferences_t {
            frameInfo: LZ4F_frameInfo_t::zeroed(),
            compressionLevel: 0,
            autoFlush: 0,
            favorDecSpeed: 0,
            reserved: [0, 0, 0],
        }
    }
    const fn init() -> Self {
        /* LZ4F_INIT_PREFERENCES */
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
    pub stableSrc: u32,
    pub reserved: [u32; 3],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct LZ4F_decompressOptions_t {
    pub stableDst: u32,
    pub skipChecksums: u32,
    pub reserved1: u32,
    pub reserved0: u32,
}

#[repr(C)]
pub struct LZ4F_CDict {
    pub cmem: LZ4F_CustomMem,
    pub dictContent: *mut u8,
    pub fastCtx: *mut LZ4_stream_t,
    pub HCCtx: *mut LZ4_streamHC_t,
}

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
    pub blockCompressMode: c_int,
}

/* dStage_t */
const dstage_getFrameHeader: c_int = 0;
const dstage_storeFrameHeader: c_int = 1;
const dstage_init: c_int = 2;
const dstage_getBlockHeader: c_int = 3;
const dstage_storeBlockHeader: c_int = 4;
const dstage_copyDirect: c_int = 5;
const dstage_getBlockChecksum: c_int = 6;
const dstage_getCBlock: c_int = 7;
const dstage_storeCBlock: c_int = 8;
const dstage_flushOut: c_int = 9;
const dstage_getSuffix: c_int = 10;
const dstage_storeSuffix: c_int = 11;
const dstage_getSFrameSize: c_int = 12;
const dstage_storeSFrameSize: c_int = 13;
const dstage_skipSkippable: c_int = 14;

#[repr(C)]
pub struct LZ4F_dctx {
    pub cmem: LZ4F_CustomMem,
    pub frameInfo: LZ4F_frameInfo_t,
    pub version: u32,
    pub dStage: c_int,
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

pub type LZ4F_errorCode_t = usize;

/* ================================================================ *
 *  Memory routines
 * ================================================================ */

unsafe fn LZ4F_calloc(s: usize, cmem: LZ4F_CustomMem) -> *mut u8 {
    if let Some(f) = cmem.customCalloc {
        return f(cmem.opaqueState, s) as *mut u8;
    }
    if cmem.customAlloc.is_none() {
        return calloc(1, s);
    }
    {
        let p = (cmem.customAlloc.unwrap())(cmem.opaqueState, s) as *mut u8;
        if !p.is_null() {
            MEM_INIT(p, 0, s);
        }
        p
    }
}

unsafe fn LZ4F_malloc(s: usize, cmem: LZ4F_CustomMem) -> *mut u8 {
    if let Some(f) = cmem.customAlloc {
        return f(cmem.opaqueState, s) as *mut u8;
    }
    malloc(s)
}

unsafe fn LZ4F_free(p: *mut u8, cmem: LZ4F_CustomMem) {
    if p.is_null() {
        return;
    }
    if let Some(f) = cmem.customFree {
        f(cmem.opaqueState, p as *mut c_void);
        return;
    }
    free(p);
}

/* ================================================================ *
 *  Little-endian helpers
 * ================================================================ */
#[inline(always)]
unsafe fn LZ4F_readLE32(src: *const u8) -> u32 {
    let mut value32: u32 = *src as u32;
    value32 |= (*src.wrapping_add(1) as u32) << 8;
    value32 |= (*src.wrapping_add(2) as u32) << 16;
    value32 |= (*src.wrapping_add(3) as u32) << 24;
    value32
}

#[inline(always)]
unsafe fn LZ4F_writeLE32(dst: *mut u8, value32: u32) {
    *dst.wrapping_add(0) = value32 as u8;
    *dst.wrapping_add(1) = (value32 >> 8) as u8;
    *dst.wrapping_add(2) = (value32 >> 16) as u8;
    *dst.wrapping_add(3) = (value32 >> 24) as u8;
}

#[inline(always)]
unsafe fn LZ4F_readLE64(src: *const u8) -> u64 {
    let mut value64: u64 = *src as u64;
    value64 |= (*src.wrapping_add(1) as u64) << 8;
    value64 |= (*src.wrapping_add(2) as u64) << 16;
    value64 |= (*src.wrapping_add(3) as u64) << 24;
    value64 |= (*src.wrapping_add(4) as u64) << 32;
    value64 |= (*src.wrapping_add(5) as u64) << 40;
    value64 |= (*src.wrapping_add(6) as u64) << 48;
    value64 |= (*src.wrapping_add(7) as u64) << 56;
    value64
}

#[inline(always)]
unsafe fn LZ4F_writeLE64(dst: *mut u8, value64: u64) {
    *dst.wrapping_add(0) = value64 as u8;
    *dst.wrapping_add(1) = (value64 >> 8) as u8;
    *dst.wrapping_add(2) = (value64 >> 16) as u8;
    *dst.wrapping_add(3) = (value64 >> 24) as u8;
    *dst.wrapping_add(4) = (value64 >> 32) as u8;
    *dst.wrapping_add(5) = (value64 >> 40) as u8;
    *dst.wrapping_add(6) = (value64 >> 48) as u8;
    *dst.wrapping_add(7) = (value64 >> 56) as u8;
}

/* ================================================================ *
 *  Error management
 * ================================================================ */

pub const LZ4F_OK_NoError: c_int = 0;
pub const LZ4F_ERROR_GENERIC: c_int = 1;
pub const LZ4F_ERROR_maxBlockSize_invalid: c_int = 2;
pub const LZ4F_ERROR_blockMode_invalid: c_int = 3;
pub const LZ4F_ERROR_parameter_invalid: c_int = 4;
pub const LZ4F_ERROR_compressionLevel_invalid: c_int = 5;
pub const LZ4F_ERROR_headerVersion_wrong: c_int = 6;
pub const LZ4F_ERROR_blockChecksum_invalid: c_int = 7;
pub const LZ4F_ERROR_reservedFlag_set: c_int = 8;
pub const LZ4F_ERROR_allocation_failed: c_int = 9;
pub const LZ4F_ERROR_srcSize_tooLarge: c_int = 10;
pub const LZ4F_ERROR_dstMaxSize_tooSmall: c_int = 11;
pub const LZ4F_ERROR_frameHeader_incomplete: c_int = 12;
pub const LZ4F_ERROR_frameType_unknown: c_int = 13;
pub const LZ4F_ERROR_frameSize_wrong: c_int = 14;
pub const LZ4F_ERROR_srcPtr_wrong: c_int = 15;
pub const LZ4F_ERROR_decompressionFailed: c_int = 16;
pub const LZ4F_ERROR_headerChecksum_invalid: c_int = 17;
pub const LZ4F_ERROR_contentChecksum_invalid: c_int = 18;
pub const LZ4F_ERROR_frameDecoding_alreadyStarted: c_int = 19;
pub const LZ4F_ERROR_compressionState_uninitialized: c_int = 20;
pub const LZ4F_ERROR_parameter_null: c_int = 21;
pub const LZ4F_ERROR_io_write: c_int = 22;
pub const LZ4F_ERROR_io_read: c_int = 23;
pub const LZ4F_ERROR_maxCode: c_int = 24;

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

static codeError: &[u8] = b"Unspecified error code\0";

#[inline(always)]
pub fn lz4f_isError(code: LZ4F_errorCode_t) -> bool {
    code > (0usize).wrapping_sub(LZ4F_ERROR_maxCode as usize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_isError(code: LZ4F_errorCode_t) -> u32 {
    lz4f_isError(code) as u32
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_getErrorName(code: LZ4F_errorCode_t) -> *const c_char {
    if lz4f_isError(code) {
        let idx = (0i64).wrapping_sub(code as i64) as usize;
        return LZ4F_errorStrings[idx].as_ptr() as *const c_char;
    }
    codeError.as_ptr() as *const c_char
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_getErrorCode(functionResult: usize) -> c_int {
    if !lz4f_isError(functionResult) {
        return LZ4F_OK_NoError;
    }
    (0isize).wrapping_sub(functionResult as isize) as c_int
}

#[inline(always)]
fn LZ4F_returnErrorCode(code: c_int) -> LZ4F_errorCode_t {
    (0isize).wrapping_sub(code as isize) as usize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_getVersion() -> u32 {
    LZ4F_VERSION
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_compressionLevel_max() -> c_int {
    LZ4HC_CLEVEL_MAX
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_getBlockSize(mut blockSizeID: c_int) -> usize {
    static blockSizes: [usize; 4] = [64 * 1024, 256 * 1024, 1024 * 1024, 4 * 1024 * 1024];

    if blockSizeID == 0 {
        blockSizeID = LZ4F_BLOCKSIZEID_DEFAULT;
    }
    if blockSizeID < LZ4F_max64KB || blockSizeID > LZ4F_max4MB {
        return LZ4F_returnErrorCode(LZ4F_ERROR_maxBlockSize_invalid);
    }
    let blockSizeIdx = (blockSizeID - LZ4F_max64KB) as usize;
    blockSizes[blockSizeIdx]
}

/* ================================================================ *
 *  Private functions
 * ================================================================ */

#[inline(always)]
fn MIN_usize(a: usize, b: usize) -> usize {
    if a < b {
        a
    } else {
        b
    }
}

unsafe fn LZ4F_headerChecksum(header: *const u8, length: usize) -> u8 {
    let xxh = xxh32(header, length, 0);
    (xxh >> 8) as u8
}

/* ================================================================ *
 *  Simple-pass compression functions
 * ================================================================ */

fn LZ4F_optimalBSID(requestedBSID: c_int, srcSize: usize) -> c_int {
    let mut proposedBSID: c_int = LZ4F_max64KB;
    let mut maxBlockSize: usize = 64 * 1024;
    /* Both operands have C type LZ4F_blockSizeID_t, an enum whose enumerators are
     * all non-negative, so its compatible type is `unsigned int`: this comparison
     * is UNSIGNED in the C. Out-of-range (negative when viewed as int) IDs
     * arriving through the ABI must therefore compare as large values. */
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
    {
        let prefsPtr: *const LZ4F_preferences_t = if preferencesPtr.is_null() {
            &prefsNull as *const LZ4F_preferences_t
        } else {
            preferencesPtr
        };
        let flush: u32 = (*prefsPtr).autoFlush | ((srcSize == 0) as u32);
        let blockID: c_int = (*prefsPtr).frameInfo.blockSizeID;
        let blockSize: usize = LZ4F_getBlockSize(blockID);
        let maxBuffered: usize = blockSize.wrapping_sub(1);
        let bufferedSize: usize = MIN_usize(alreadyBuffered, maxBuffered);
        let maxSrcSize: usize = srcSize.wrapping_add(bufferedSize);
        let nbFullBlocks: u32 = (maxSrcSize / blockSize) as u32;
        let partialBlockSize: usize = maxSrcSize & (blockSize.wrapping_sub(1));
        let lastBlockSize: usize = if flush != 0 { partialBlockSize } else { 0 };
        let nbBlocks: u32 = nbFullBlocks.wrapping_add((lastBlockSize > 0) as u32);

        /* blockChecksumFlag / contentChecksumFlag have C enum types whose
         * enumerators are all non-negative, hence a compatible type of
         * `unsigned int`: these products ZERO-extend the 32-bit field, they do not
         * sign-extend it. */
        let blockCRCSize: usize =
            BFSize.wrapping_mul((*prefsPtr).frameInfo.blockChecksumFlag as u32 as usize);
        let frameEnd: usize = BHSize.wrapping_add(
            ((*prefsPtr).frameInfo.contentChecksumFlag as u32 as usize).wrapping_mul(BFSize),
        );

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
        prefs = LZ4F_preferences_t::zeroed();
        MEM_INIT(
            &mut prefs as *mut LZ4F_preferences_t as *mut u8,
            0,
            core::mem::size_of::<LZ4F_preferences_t>(),
        );
    }
    prefs.autoFlush = 1;

    headerSize + LZ4F_compressBound_internal(srcSize, &prefs as *const LZ4F_preferences_t, 0)
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
    let dstStart: *mut u8 = dstBuffer as *mut u8;
    let mut dstPtr: *mut u8 = dstStart;
    let dstEnd: *mut u8 = dstStart.wrapping_add(dstCapacity);

    if !preferencesPtr.is_null() {
        prefs = *preferencesPtr;
    } else {
        prefs = LZ4F_preferences_t::zeroed();
        MEM_INIT(
            &mut prefs as *mut LZ4F_preferences_t as *mut u8,
            0,
            core::mem::size_of::<LZ4F_preferences_t>(),
        );
    }
    if prefs.frameInfo.contentSize != 0 {
        prefs.frameInfo.contentSize = srcSize as u64;
    }

    prefs.frameInfo.blockSizeID = LZ4F_optimalBSID(prefs.frameInfo.blockSizeID, srcSize);
    prefs.autoFlush = 1;
    if srcSize <= LZ4F_getBlockSize(prefs.frameInfo.blockSizeID) {
        prefs.frameInfo.blockMode = LZ4F_blockIndependent;
    }

    options = LZ4F_compressOptions_t {
        stableSrc: 0,
        reserved: [0, 0, 0],
    };
    MEM_INIT(
        &mut options as *mut LZ4F_compressOptions_t as *mut u8,
        0,
        core::mem::size_of::<LZ4F_compressOptions_t>(),
    );
    options.stableSrc = 1;

    if dstCapacity < LZ4F_compressFrameBound(srcSize, &prefs as *const LZ4F_preferences_t) {
        return LZ4F_returnErrorCode(LZ4F_ERROR_dstMaxSize_tooSmall);
    }

    {
        let headerSize = LZ4F_compressBegin_usingCDict(
            cctx,
            dstBuffer,
            dstCapacity,
            cdict,
            &prefs as *const LZ4F_preferences_t,
        );
        if lz4f_isError(headerSize) {
            return headerSize;
        }
        dstPtr = dstPtr.wrapping_add(headerSize);
    }

    {
        let cSize = LZ4F_compressUpdate(
            cctx,
            dstPtr as *mut c_void,
            pdiff(dstEnd, dstPtr),
            srcBuffer,
            srcSize,
            &options as *const LZ4F_compressOptions_t,
        );
        if lz4f_isError(cSize) {
            return cSize;
        }
        dstPtr = dstPtr.wrapping_add(cSize);
    }

    {
        let tailSize = LZ4F_compressEnd(
            cctx,
            dstPtr as *mut c_void,
            pdiff(dstEnd, dstPtr),
            &options as *const LZ4F_compressOptions_t,
        );
        if lz4f_isError(tailSize) {
            return tailSize;
        }
        dstPtr = dstPtr.wrapping_add(tailSize);
    }

    pdiff(dstPtr, dstStart)
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
    let mut cctx: LZ4F_cctx = core::mem::zeroed();
    let mut lz4ctx = LZ4_stream_u {
        minStateSize: [0u8; SIZEOF_LZ4_STREAM_T],
    };
    let cctxPtr: *mut LZ4F_cctx = &mut cctx;

    MEM_INIT(
        cctxPtr as *mut u8,
        0,
        core::mem::size_of::<LZ4F_cctx>(),
    );
    cctx.version = LZ4F_VERSION;
    cctx.maxBufferSize = 5 * 1024 * 1024; /* mess with real buffer size to prevent dynamic allocation */
    if preferencesPtr.is_null() || (*preferencesPtr).compressionLevel < LZ4HC_CLEVEL_MIN {
        LZ4_initStream(
            lz4ctx.minStateSize.as_mut_ptr() as *mut c_void,
            SIZEOF_LZ4_STREAM_T,
        );
        (*cctxPtr).lz4CtxPtr = lz4ctx.minStateSize.as_mut_ptr() as *mut c_void;
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
        LZ4F_free((*cctxPtr).lz4CtxPtr as *mut u8, (*cctxPtr).cmem);
    }
    result
}

/* ================================================================ *
 *  Dictionary compression
 * ================================================================ */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_createCDict_advanced(
    cmem: LZ4F_CustomMem,
    dictBuffer: *const c_void,
    mut dictSize: usize,
) -> *mut LZ4F_CDict {
    let mut dictStart: *const c_char = dictBuffer as *const c_char;
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
    memcpy((*cdict).dictContent, dictStart as *const u8, dictSize);
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
    LZ4F_free((*cdict).fastCtx as *mut u8, (*cdict).cmem);
    LZ4F_free((*cdict).HCCtx as *mut u8, (*cdict).cmem);
    LZ4F_free(cdict as *mut u8, (*cdict).cmem);
}

/* ================================================================ *
 *  Advanced compression functions
 * ================================================================ */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_createCompressionContext_advanced(
    customMem: LZ4F_CustomMem,
    version: u32,
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
    version: u32,
) -> LZ4F_errorCode_t {
    if LZ4F_compressionContextPtr.is_null() {
        return LZ4F_returnErrorCode(LZ4F_ERROR_parameter_null);
    }

    *LZ4F_compressionContextPtr =
        LZ4F_createCompressionContext_advanced(LZ4F_defaultCMem, version);
    if (*LZ4F_compressionContextPtr).is_null() {
        return LZ4F_returnErrorCode(LZ4F_ERROR_allocation_failed);
    }
    LZ4F_OK_NoError as usize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_freeCompressionContext(cctxPtr: *mut LZ4F_cctx) -> LZ4F_errorCode_t {
    if !cctxPtr.is_null() {
        LZ4F_free((*cctxPtr).lz4CtxPtr as *mut u8, (*cctxPtr).cmem);
        LZ4F_free((*cctxPtr).tmpBuff, (*cctxPtr).cmem);
        LZ4F_free(cctxPtr as *mut u8, (*cctxPtr).cmem);
    }
    LZ4F_OK_NoError as usize
}

unsafe fn LZ4F_initStream(
    ctx: *mut c_void,
    cdict: *const LZ4F_CDict,
    level: c_int,
    blockMode: c_int,
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

fn ctxTypeID_to_size(ctxTypeID: c_int) -> c_int {
    match ctxTypeID {
        1 => SIZEOF_LZ4_STREAM_T as c_int,
        2 => SIZEOF_LZ4_STREAMHC_T as c_int,
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
    let dstStart: *mut u8 = dstBuffer as *mut u8;
    let mut dstPtr: *mut u8 = dstStart;

    if dstCapacity < maxFHSize {
        return LZ4F_returnErrorCode(LZ4F_ERROR_dstMaxSize_tooSmall);
    }
    if preferencesPtr.is_null() {
        preferencesPtr = &prefNull as *const LZ4F_preferences_t;
    }
    (*cctx).prefs = *preferencesPtr;

    /* cctx Management */
    {
        let ctxTypeID: u16 = if (*cctx).prefs.compressionLevel < LZ4HC_CLEVEL_MIN {
            1
        } else {
            2
        };
        let requiredSize: c_int = ctxTypeID_to_size(ctxTypeID as c_int);
        let allocatedSize: c_int = ctxTypeID_to_size((*cctx).lz4CtxAlloc as c_int);
        if allocatedSize < requiredSize {
            /* not enough space allocated */
            LZ4F_free((*cctx).lz4CtxPtr as *mut u8, (*cctx).cmem);
            if (*cctx).prefs.compressionLevel < LZ4HC_CLEVEL_MIN {
                (*cctx).lz4CtxPtr =
                    LZ4F_malloc(SIZEOF_LZ4_STREAM_T, (*cctx).cmem) as *mut c_void;
                if !(*cctx).lz4CtxPtr.is_null() {
                    LZ4_initStream((*cctx).lz4CtxPtr, SIZEOF_LZ4_STREAM_T);
                }
            } else {
                (*cctx).lz4CtxPtr =
                    LZ4F_malloc(SIZEOF_LZ4_STREAMHC_T, (*cctx).cmem) as *mut c_void;
                if !(*cctx).lz4CtxPtr.is_null() {
                    LZ4_initStreamHC((*cctx).lz4CtxPtr, SIZEOF_LZ4_STREAMHC_T);
                }
            }
            if (*cctx).lz4CtxPtr.is_null() {
                return LZ4F_returnErrorCode(LZ4F_ERROR_allocation_failed);
            }
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
            LZ4F_free((*cctx).tmpBuff, (*cctx).cmem);
            (*cctx).tmpBuff = LZ4F_malloc(requiredBuffSize, (*cctx).cmem);
            if (*cctx).tmpBuff.is_null() {
                return LZ4F_returnErrorCode(LZ4F_ERROR_allocation_failed);
            }
            (*cctx).maxBufferSize = requiredBuffSize;
        }
    }
    (*cctx).tmpIn = (*cctx).tmpBuff;
    (*cctx).tmpInSize = 0;
    xxh32_reset(&mut (*cctx).xxh, 0);

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
        if dictSize > c_int::MAX as usize {
            return LZ4F_returnErrorCode(LZ4F_ERROR_parameter_invalid);
        }
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
    LZ4F_writeLE32(dstPtr, LZ4F_MAGICNUMBER);
    dstPtr = dstPtr.wrapping_add(4);
    {
        let headerStart: *mut u8 = dstPtr;

        /* FLG Byte */
        *dstPtr = (((1 & _2BITS) << 6)
            + ((((*cctx).prefs.frameInfo.blockMode as u32) & _1BIT) << 5)
            + ((((*cctx).prefs.frameInfo.blockChecksumFlag as u32) & _1BIT) << 4)
            + ((((*cctx).prefs.frameInfo.contentSize > 0) as u32) << 3)
            + ((((*cctx).prefs.frameInfo.contentChecksumFlag as u32) & _1BIT) << 2)
            + (((*cctx).prefs.frameInfo.dictID > 0) as u32)) as u8;
        dstPtr = dstPtr.wrapping_add(1);
        /* BD Byte */
        *dstPtr = ((((*cctx).prefs.frameInfo.blockSizeID as u32) & _3BITS) << 4) as u8;
        dstPtr = dstPtr.wrapping_add(1);
        /* Optional Frame content size field */
        if (*cctx).prefs.frameInfo.contentSize != 0 {
            LZ4F_writeLE64(dstPtr, (*cctx).prefs.frameInfo.contentSize);
            dstPtr = dstPtr.wrapping_add(8);
            (*cctx).totalInSize = 0;
        }
        /* Optional dictionary ID field */
        if (*cctx).prefs.frameInfo.dictID != 0 {
            LZ4F_writeLE32(dstPtr, (*cctx).prefs.frameInfo.dictID);
            dstPtr = dstPtr.wrapping_add(4);
        }
        /* Header CRC Byte */
        *dstPtr = LZ4F_headerChecksum(headerStart, pdiff(dstPtr, headerStart));
        dstPtr = dstPtr.wrapping_add(1);
    }

    (*cctx).cStage = 1;
    pdiff(dstPtr, dstStart)
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
    crcFlag: c_int,
) -> usize {
    let cSizePtr: *mut u8 = dst as *mut u8;
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
        LZ4F_writeLE32(cSizePtr, cSize | LZ4F_BLOCKUNCOMPRESSED_FLAG);
        memcpy(cSizePtr.wrapping_add(BHSize), src as *const u8, srcSize);
    } else {
        LZ4F_writeLE32(cSizePtr, cSize);
    }
    if crcFlag != 0 {
        let crc32 = xxh32(cSizePtr.wrapping_add(BHSize), cSize as usize, 0);
        LZ4F_writeLE32(
            cSizePtr.wrapping_add(BHSize).wrapping_add(cSize as usize),
            crc32,
        );
    }
    BHSize + cSize as usize + (crcFlag as u32 as usize) * BFSize
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
    let acceleration: c_int = if level < 0 { -level + 1 } else { 1 };
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
    let acceleration: c_int = if level < 0 { -level + 1 } else { 1 };
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

fn LZ4F_selectCompression(blockMode: c_int, level: c_int, compressMode: c_int) -> compressFunc_t {
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

/* Save history (up to 64KB) into @tmpBuff */
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

static k_cOptionsNull: LZ4F_compressOptions_t = LZ4F_compressOptions_t {
    stableSrc: 0,
    reserved: [0, 0, 0],
};

unsafe fn LZ4F_compressUpdateImpl(
    cctxPtr: *mut LZ4F_cctx,
    dstBuffer: *mut c_void,
    dstCapacity: usize,
    srcBuffer: *const c_void,
    srcSize: usize,
    mut compressOptionsPtr: *const LZ4F_compressOptions_t,
    blockCompression: c_int,
) -> usize {
    let blockSize: usize = (*cctxPtr).maxBlockSize;
    let mut srcPtr: *const u8 = srcBuffer as *const u8;
    let srcEnd: *const u8 = srcPtr.wrapping_add(srcSize);
    let dstStart: *mut u8 = dstBuffer as *mut u8;
    let mut dstPtr: *mut u8 = dstStart;
    let mut lastBlockCompressed: i32 = notDone;
    let compress: compressFunc_t = LZ4F_selectCompression(
        (*cctxPtr).prefs.frameInfo.blockMode,
        (*cctxPtr).prefs.compressionLevel,
        blockCompression,
    );
    let bytesWritten: usize;

    if (*cctxPtr).cStage != 1 {
        return LZ4F_returnErrorCode(LZ4F_ERROR_compressionState_uninitialized);
    }
    if dstCapacity
        < LZ4F_compressBound_internal(
            srcSize,
            &(*cctxPtr).prefs as *const LZ4F_preferences_t,
            (*cctxPtr).tmpInSize,
        )
    {
        return LZ4F_returnErrorCode(LZ4F_ERROR_dstMaxSize_tooSmall);
    }

    if blockCompression == LZ4B_UNCOMPRESSED && dstCapacity < srcSize {
        return LZ4F_returnErrorCode(LZ4F_ERROR_dstMaxSize_tooSmall);
    }

    /* flush currently written block, to continue with new block compression */
    if (*cctxPtr).blockCompressMode != blockCompression {
        bytesWritten = LZ4F_flush(cctxPtr, dstBuffer, dstCapacity, compressOptionsPtr);
        dstPtr = dstPtr.wrapping_add(bytesWritten);
        (*cctxPtr).blockCompressMode = blockCompression;
    }

    if compressOptionsPtr.is_null() {
        compressOptionsPtr = &k_cOptionsNull as *const LZ4F_compressOptions_t;
    }

    /* complete tmp buffer */
    if (*cctxPtr).tmpInSize > 0 {
        let sizeToCopy: usize = blockSize - (*cctxPtr).tmpInSize;
        if sizeToCopy > srcSize {
            /* add src to tmpIn buffer */
            memcpy(
                (*cctxPtr).tmpIn.wrapping_add((*cctxPtr).tmpInSize),
                srcBuffer as *const u8,
                srcSize,
            );
            srcPtr = srcEnd;
            (*cctxPtr).tmpInSize += srcSize;
        } else {
            /* complete tmpIn block and then compress it */
            lastBlockCompressed = fromTmpBuffer;
            memcpy(
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

    while pdiff(srcEnd, srcPtr) >= blockSize {
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
            pdiff(srcEnd, srcPtr),
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
        let sizeToCopy: usize = pdiff(srcEnd, srcPtr);
        memcpy((*cctxPtr).tmpIn, srcPtr, sizeToCopy);
        (*cctxPtr).tmpInSize = sizeToCopy;
    }

    if (*cctxPtr).prefs.frameInfo.contentChecksumFlag == LZ4F_contentChecksumEnabled {
        xxh32_update(&mut (*cctxPtr).xxh, srcBuffer as *const u8, srcSize);
    }

    (*cctxPtr).totalInSize = (*cctxPtr).totalInSize.wrapping_add(srcSize as u64);
    pdiff(dstPtr, dstStart)
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
    let dstStart: *mut u8 = dstBuffer as *mut u8;
    let mut dstPtr: *mut u8 = dstStart;
    let compress: compressFunc_t;

    if (*cctxPtr).tmpInSize == 0 {
        return 0; /* nothing to flush */
    }
    if (*cctxPtr).cStage != 1 {
        return LZ4F_returnErrorCode(LZ4F_ERROR_compressionState_uninitialized);
    }
    if dstCapacity < ((*cctxPtr).tmpInSize + BHSize + BFSize) {
        return LZ4F_returnErrorCode(LZ4F_ERROR_dstMaxSize_tooSmall);
    }

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

    pdiff(dstPtr, dstStart)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_compressEnd(
    cctxPtr: *mut LZ4F_cctx,
    dstBuffer: *mut c_void,
    mut dstCapacity: usize,
    compressOptionsPtr: *const LZ4F_compressOptions_t,
) -> usize {
    let dstStart: *mut u8 = dstBuffer as *mut u8;
    let mut dstPtr: *mut u8 = dstStart;

    let flushSize = LZ4F_flush(cctxPtr, dstBuffer, dstCapacity, compressOptionsPtr);
    if lz4f_isError(flushSize) {
        return flushSize;
    }
    dstPtr = dstPtr.wrapping_add(flushSize);

    dstCapacity -= flushSize;

    if dstCapacity < 4 {
        return LZ4F_returnErrorCode(LZ4F_ERROR_dstMaxSize_tooSmall);
    }
    LZ4F_writeLE32(dstPtr, 0);
    dstPtr = dstPtr.wrapping_add(4); /* endMark */

    if (*cctxPtr).prefs.frameInfo.contentChecksumFlag == LZ4F_contentChecksumEnabled {
        let xxh = xxh32_digest(&(*cctxPtr).xxh);
        if dstCapacity < 8 {
            return LZ4F_returnErrorCode(LZ4F_ERROR_dstMaxSize_tooSmall);
        }
        LZ4F_writeLE32(dstPtr, xxh);
        dstPtr = dstPtr.wrapping_add(4);
    }

    (*cctxPtr).cStage = 0; /* state is now re-usable */

    if (*cctxPtr).prefs.frameInfo.contentSize != 0 {
        if (*cctxPtr).prefs.frameInfo.contentSize != (*cctxPtr).totalInSize {
            return LZ4F_returnErrorCode(LZ4F_ERROR_frameSize_wrong);
        }
    }

    pdiff(dstPtr, dstStart)
}

/* ================================================================ *
 *  Frame Decompression
 * ================================================================ */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_createDecompressionContext_advanced(
    customMem: LZ4F_CustomMem,
    version: u32,
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
    versionNumber: u32,
) -> LZ4F_errorCode_t {
    if LZ4F_decompressionContextPtr.is_null() {
        return LZ4F_returnErrorCode(LZ4F_ERROR_parameter_null);
    }

    *LZ4F_decompressionContextPtr =
        LZ4F_createDecompressionContext_advanced(LZ4F_defaultCMem, versionNumber);
    if (*LZ4F_decompressionContextPtr).is_null() {
        return LZ4F_returnErrorCode(LZ4F_ERROR_allocation_failed);
    }
    LZ4F_OK_NoError as usize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_freeDecompressionContext(dctx: *mut LZ4F_dctx) -> LZ4F_errorCode_t {
    let mut result: LZ4F_errorCode_t = LZ4F_OK_NoError as usize;
    if !dctx.is_null() {
        result = (*dctx).dStage as LZ4F_errorCode_t;
        LZ4F_free((*dctx).tmpIn, (*dctx).cmem);
        LZ4F_free((*dctx).tmpOutBuffer, (*dctx).cmem);
        LZ4F_free(dctx as *mut u8, (*dctx).cmem);
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
    let srcPtr: *const u8 = src as *const u8;

    /* need to decode header to get frameInfo */
    if srcSize < minFHSize {
        return LZ4F_returnErrorCode(LZ4F_ERROR_frameHeader_incomplete);
    }
    MEM_INIT(
        &mut (*dctx).frameInfo as *mut LZ4F_frameInfo_t as *mut u8,
        0,
        core::mem::size_of::<LZ4F_frameInfo_t>(),
    );

    /* special case : skippable frames */
    if (LZ4F_readLE32(srcPtr) & 0xFFFFFFF0u32) == LZ4F_MAGIC_SKIPPABLE_START {
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
    if LZ4F_readLE32(srcPtr) != LZ4F_MAGICNUMBER {
        return LZ4F_returnErrorCode(LZ4F_ERROR_frameType_unknown);
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
            return LZ4F_returnErrorCode(LZ4F_ERROR_reservedFlag_set);
        }
        if version != 1 {
            return LZ4F_returnErrorCode(LZ4F_ERROR_headerVersion_wrong);
        }
    }

    /* Frame Header Size */
    frameHeaderSize = minFHSize
        + (if contentSizeFlag != 0 { 8 } else { 0 })
        + (if dictIDFlag != 0 { 4 } else { 0 });

    if srcSize < frameHeaderSize {
        /* not enough input to fully decode frame header */
        if srcPtr != (*dctx).header.as_ptr() {
            memcpy((*dctx).header.as_mut_ptr(), srcPtr, srcSize);
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
            return LZ4F_returnErrorCode(LZ4F_ERROR_reservedFlag_set);
        }
        if blockSizeID < 4 {
            return LZ4F_returnErrorCode(LZ4F_ERROR_maxBlockSize_invalid);
        }
        if ((BD >> 0) & _4BITS) != 0 {
            return LZ4F_returnErrorCode(LZ4F_ERROR_reservedFlag_set);
        }
    }

    /* check header */
    {
        let HC: u8 = LZ4F_headerChecksum(srcPtr.wrapping_add(4), frameHeaderSize - 5);
        if HC != *srcPtr.wrapping_add(frameHeaderSize - 1) {
            return LZ4F_returnErrorCode(LZ4F_ERROR_headerChecksum_invalid);
        }
    }

    /* save */
    (*dctx).frameInfo.blockMode = blockMode as c_int;
    (*dctx).frameInfo.blockChecksumFlag = blockChecksumFlag as c_int;
    (*dctx).frameInfo.contentChecksumFlag = contentChecksumFlag as c_int;
    (*dctx).frameInfo.blockSizeID = blockSizeID as c_int;
    (*dctx).maxBlockSize = LZ4F_getBlockSize(blockSizeID as c_int);
    if contentSizeFlag != 0 {
        (*dctx).frameInfo.contentSize = LZ4F_readLE64(srcPtr.wrapping_add(6));
        (*dctx).frameRemainingSize = (*dctx).frameInfo.contentSize;
    }
    if dictIDFlag != 0 {
        (*dctx).frameInfo.dictID = LZ4F_readLE32(srcPtr.wrapping_add(frameHeaderSize - 5));
    }

    (*dctx).dStage = dstage_init;

    frameHeaderSize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_headerSize(src: *const c_void, srcSize: usize) -> usize {
    if src.is_null() {
        return LZ4F_returnErrorCode(LZ4F_ERROR_srcPtr_wrong);
    }

    /* minimal srcSize to determine header size */
    if srcSize < LZ4F_MIN_SIZE_TO_KNOW_HEADER_LENGTH {
        return LZ4F_returnErrorCode(LZ4F_ERROR_frameHeader_incomplete);
    }

    /* special case : skippable frames */
    if (LZ4F_readLE32(src as *const u8) & 0xFFFFFFF0u32) == LZ4F_MAGIC_SKIPPABLE_START {
        return 8;
    }

    /* control magic number */
    if LZ4F_readLE32(src as *const u8) != LZ4F_MAGICNUMBER {
        return LZ4F_returnErrorCode(LZ4F_ERROR_frameType_unknown);
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
            /* frame decoding already started, in the middle of header => automatic fail */
            *srcSizePtr = 0;
            return LZ4F_returnErrorCode(LZ4F_ERROR_frameDecoding_alreadyStarted);
        } else {
            let hSize = LZ4F_headerSize(srcBuffer, *srcSizePtr);
            if lz4f_isError(hSize) {
                *srcSizePtr = 0;
                return hSize;
            }
            if *srcSizePtr < hSize {
                *srcSizePtr = 0;
                return LZ4F_returnErrorCode(LZ4F_ERROR_frameHeader_incomplete);
            }

            {
                let mut decodeResult = LZ4F_decodeHeader(dctx, srcBuffer, hSize);
                if lz4f_isError(decodeResult) {
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

/* LZ4F_updateDict() : only used for LZ4F_blockLinked mode */
unsafe fn LZ4F_updateDict(
    dctx: *mut LZ4F_dctx,
    dstPtr: *const u8,
    dstSize: usize,
    dstBufferStart: *const u8,
    withinTmp: u32,
) {
    if (*dctx).dictSize == 0 {
        (*dctx).dict = dstPtr;
    }

    if (*dctx).dict.wrapping_add((*dctx).dictSize) == dstPtr {
        /* prefix mode, everything within dstBuffer */
        (*dctx).dictSize += dstSize;
        return;
    }

    if pdiff(dstPtr, dstBufferStart) + dstSize >= 64 * 1024 {
        /* history in dstBuffer becomes large enough to become dictionary */
        (*dctx).dict = dstBufferStart;
        (*dctx).dictSize = pdiff(dstPtr, dstBufferStart) + dstSize;
        return;
    }

    if (withinTmp != 0) && ((*dctx).dict == (*dctx).tmpOutBuffer as *const u8) {
        /* continue history within tmpOutBuffer */
        (*dctx).dictSize += dstSize;
        return;
    }

    if withinTmp != 0 {
        /* copy relevant dict portion in front of tmpOut within tmpOutBuffer */
        let preserveSize: usize = pdiff((*dctx).tmpOut as *const u8, (*dctx).tmpOutBuffer as *const u8);
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

        memcpy(
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

    if (*dctx).dict == (*dctx).tmpOutBuffer as *const u8 {
        /* copy dst into tmp to complete dict */
        if (*dctx).dictSize + dstSize > (*dctx).maxBufferSize {
            /* tmp buffer not large enough */
            let preserveSize: usize = (64 * 1024usize).wrapping_sub(dstSize);
            memcpy(
                (*dctx).tmpOutBuffer,
                (*dctx)
                    .dict
                    .wrapping_add((*dctx).dictSize)
                    .wrapping_sub(preserveSize),
                preserveSize,
            );
            (*dctx).dictSize = preserveSize;
        }
        memcpy(
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
        memcpy(
            (*dctx).tmpOutBuffer,
            (*dctx)
                .dict
                .wrapping_add((*dctx).dictSize)
                .wrapping_sub(preserveSize),
            preserveSize,
        );
        memcpy(
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
    mut decompressOptionsPtr: *const LZ4F_decompressOptions_t,
) -> usize {
    let mut optionsNull: LZ4F_decompressOptions_t = LZ4F_decompressOptions_t {
        stableDst: 0,
        skipChecksums: 0,
        reserved1: 0,
        reserved0: 0,
    };
    let srcStart: *const u8 = srcBuffer as *const u8;
    let srcEnd: *const u8 = srcStart.wrapping_add(*srcSizePtr);
    let mut srcPtr: *const u8 = srcStart;
    let dstStart: *mut u8 = dstBuffer as *mut u8;
    let dstEnd: *mut u8 = if !dstStart.is_null() {
        dstStart.wrapping_add(*dstSizePtr)
    } else {
        core::ptr::null_mut()
    };
    let mut dstPtr: *mut u8 = dstStart;
    let mut selectedIn: *const u8 = core::ptr::null();
    let mut doAnotherStage: u32 = 1;
    let mut nextSrcSizeHint: usize = 1;

    MEM_INIT(
        &mut optionsNull as *mut LZ4F_decompressOptions_t as *mut u8,
        0,
        core::mem::size_of::<LZ4F_decompressOptions_t>(),
    );
    if decompressOptionsPtr.is_null() {
        decompressOptionsPtr = &optionsNull as *const LZ4F_decompressOptions_t;
    }
    *srcSizePtr = 0;
    *dstSizePtr = 0;
    (*dctx).skipChecksum |= ((*decompressOptionsPtr).skipChecksums != 0) as c_int;

    /* behaves as a state machine */
    while doAnotherStage != 0 {
        let mut pos: c_int = (*dctx).dStage;

        'sw: loop {
            if pos == dstage_getFrameHeader {
                if pdiff(srcEnd, srcPtr) >= maxFHSize {
                    /* enough to decode - shortcut */
                    let hSize = LZ4F_decodeHeader(
                        dctx,
                        srcPtr as *const c_void,
                        pdiff(srcEnd, srcPtr),
                    );
                    if lz4f_isError(hSize) {
                        return hSize;
                    }
                    srcPtr = srcPtr.wrapping_add(hSize);
                    break 'sw;
                }
                (*dctx).tmpInSize = 0;
                if pdiff(srcEnd, srcPtr) == 0 {
                    return minFHSize; /* 0-size input */
                }
                (*dctx).tmpInTarget = minFHSize;
                (*dctx).dStage = dstage_storeFrameHeader;
                pos = dstage_storeFrameHeader;
                /* fall-through */
            }

            if pos == dstage_storeFrameHeader {
                {
                    let sizeToCopy = MIN_usize(
                        (*dctx).tmpInTarget - (*dctx).tmpInSize,
                        pdiff(srcEnd, srcPtr),
                    );
                    memcpy(
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
                {
                    let r = LZ4F_decodeHeader(
                        dctx,
                        (*dctx).header.as_ptr() as *const c_void,
                        (*dctx).tmpInTarget,
                    );
                    if lz4f_isError(r) {
                        return r;
                    }
                }
                break 'sw;
            }

            if pos == dstage_init {
                if (*dctx).frameInfo.contentChecksumFlag != 0 {
                    xxh32_reset(&mut (*dctx).xxh, 0);
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
                        /* tmp buffers too small */
                        (*dctx).maxBufferSize = 0;
                        LZ4F_free((*dctx).tmpIn, (*dctx).cmem);
                        (*dctx).tmpIn = LZ4F_malloc((*dctx).maxBlockSize + BFSize, (*dctx).cmem);
                        if (*dctx).tmpIn.is_null() {
                            return LZ4F_returnErrorCode(LZ4F_ERROR_allocation_failed);
                        }
                        LZ4F_free((*dctx).tmpOutBuffer, (*dctx).cmem);
                        (*dctx).tmpOutBuffer = LZ4F_malloc(bufferNeeded, (*dctx).cmem);
                        if (*dctx).tmpOutBuffer.is_null() {
                            return LZ4F_returnErrorCode(LZ4F_ERROR_allocation_failed);
                        }
                        (*dctx).maxBufferSize = bufferNeeded;
                    }
                }
                (*dctx).tmpInSize = 0;
                (*dctx).tmpInTarget = 0;
                (*dctx).tmpOut = (*dctx).tmpOutBuffer;
                (*dctx).tmpOutStart = 0;
                (*dctx).tmpOutSize = 0;

                (*dctx).dStage = dstage_getBlockHeader;
                pos = dstage_getBlockHeader;
                /* fall-through */
            }

            if pos == dstage_getBlockHeader || pos == dstage_storeBlockHeader {
                let run_store: bool;
                if pos == dstage_getBlockHeader {
                    if pdiff(srcEnd, srcPtr) >= BHSize {
                        selectedIn = srcPtr;
                        srcPtr = srcPtr.wrapping_add(BHSize);
                    } else {
                        /* not enough input to read cBlockSize field */
                        (*dctx).tmpInSize = 0;
                        (*dctx).dStage = dstage_storeBlockHeader;
                    }
                    run_store = (*dctx).dStage == dstage_storeBlockHeader;
                } else {
                    run_store = true;
                }

                if run_store {
                    let remainingInput: usize = pdiff(srcEnd, srcPtr);
                    let wantedData: usize = BHSize - (*dctx).tmpInSize;
                    let sizeToCopy: usize = MIN_usize(wantedData, remainingInput);
                    memcpy(
                        (*dctx).tmpIn.wrapping_add((*dctx).tmpInSize),
                        srcPtr,
                        sizeToCopy,
                    );
                    srcPtr = srcPtr.wrapping_add(sizeToCopy);
                    (*dctx).tmpInSize += sizeToCopy;

                    if (*dctx).tmpInSize < BHSize {
                        /* not enough input for cBlockSize */
                        nextSrcSizeHint = BHSize - (*dctx).tmpInSize;
                        doAnotherStage = 0;
                        break 'sw;
                    }
                    selectedIn = (*dctx).tmpIn;
                }

                /* decode block header */
                {
                    let blockHeader: u32 = LZ4F_readLE32(selectedIn);
                    let nextCBlockSize: usize = (blockHeader & 0x7FFFFFFFu32) as usize;
                    let crcSize: usize = (*dctx).frameInfo.blockChecksumFlag as usize * BFSize;
                    if blockHeader == 0 {
                        /* frameEnd signal, no more block */
                        (*dctx).dStage = dstage_getSuffix;
                        break 'sw;
                    }
                    if nextCBlockSize > (*dctx).maxBlockSize {
                        return LZ4F_returnErrorCode(LZ4F_ERROR_maxBlockSize_invalid);
                    }
                    if (blockHeader & LZ4F_BLOCKUNCOMPRESSED_FLAG) != 0 {
                        /* next block is uncompressed */
                        (*dctx).tmpInTarget = nextCBlockSize;
                        if (*dctx).frameInfo.blockChecksumFlag != 0 {
                            xxh32_reset(&mut (*dctx).blockChecksum, 0);
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

            if pos == dstage_copyDirect {
                /* uncompressed block */
                {
                    let sizeToCopy: usize;
                    if dstPtr.is_null() {
                        sizeToCopy = 0;
                    } else {
                        let minBuffSize: usize =
                            MIN_usize(pdiff(srcEnd, srcPtr), pdiff(dstEnd, dstPtr));
                        sizeToCopy = MIN_usize((*dctx).tmpInTarget, minBuffSize);
                        memcpy(dstPtr, srcPtr, sizeToCopy);
                        if (*dctx).skipChecksum == 0 {
                            if (*dctx).frameInfo.blockChecksumFlag != 0 {
                                xxh32_update(&mut (*dctx).blockChecksum, srcPtr, sizeToCopy);
                            }
                            if (*dctx).frameInfo.contentChecksumFlag != 0 {
                                xxh32_update(&mut (*dctx).xxh, srcPtr, sizeToCopy);
                            }
                        }
                        if (*dctx).frameInfo.contentSize != 0 {
                            (*dctx).frameRemainingSize =
                                (*dctx).frameRemainingSize.wrapping_sub(sizeToCopy as u64);
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
                            (*dctx).dStage = dstage_getBlockHeader; /* new block */
                        }
                        break 'sw;
                    }
                    (*dctx).tmpInTarget -= sizeToCopy; /* need to copy more */
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

            if pos == dstage_getBlockChecksum {
                /* check block checksum for recently transferred uncompressed block */
                {
                    let crcSrc: *const u8;
                    if (pdiff_i(srcEnd, srcPtr) >= 4) && ((*dctx).tmpInSize == 0) {
                        crcSrc = srcPtr;
                        srcPtr = srcPtr.wrapping_add(4);
                    } else {
                        let stillToCopy: usize = 4 - (*dctx).tmpInSize;
                        let sizeToCopy: usize = MIN_usize(stillToCopy, pdiff(srcEnd, srcPtr));
                        memcpy(
                            (*dctx).header.as_mut_ptr().wrapping_add((*dctx).tmpInSize),
                            srcPtr,
                            sizeToCopy,
                        );
                        (*dctx).tmpInSize += sizeToCopy;
                        srcPtr = srcPtr.wrapping_add(sizeToCopy);
                        if (*dctx).tmpInSize < 4 {
                            /* all input consumed */
                            doAnotherStage = 0;
                            break 'sw;
                        }
                        crcSrc = (*dctx).header.as_ptr();
                    }
                    if (*dctx).skipChecksum == 0 {
                        let readCRC: u32 = LZ4F_readLE32(crcSrc);
                        let calcCRC: u32 = xxh32_digest(&(*dctx).blockChecksum);
                        if readCRC != calcCRC {
                            return LZ4F_returnErrorCode(LZ4F_ERROR_blockChecksum_invalid);
                        }
                    }
                }
                (*dctx).dStage = dstage_getBlockHeader; /* new block */
                break 'sw;
            }

            if pos == dstage_getCBlock || pos == dstage_storeCBlock {
                let run_store: bool;
                if pos == dstage_getCBlock {
                    if pdiff(srcEnd, srcPtr) < (*dctx).tmpInTarget {
                        (*dctx).tmpInSize = 0;
                        (*dctx).dStage = dstage_storeCBlock;
                        break 'sw;
                    }
                    /* input large enough to read full block directly */
                    selectedIn = srcPtr;
                    srcPtr = srcPtr.wrapping_add((*dctx).tmpInTarget);
                    run_store = false; /* if (0) */
                } else {
                    run_store = true;
                }

                if run_store {
                    let wantedData: usize = (*dctx).tmpInTarget - (*dctx).tmpInSize;
                    let inputLeft: usize = pdiff(srcEnd, srcPtr);
                    let sizeToCopy: usize = MIN_usize(wantedData, inputLeft);
                    memcpy(
                        (*dctx).tmpIn.wrapping_add((*dctx).tmpInSize),
                        srcPtr,
                        sizeToCopy,
                    );
                    (*dctx).tmpInSize += sizeToCopy;
                    srcPtr = srcPtr.wrapping_add(sizeToCopy);
                    if (*dctx).tmpInSize < (*dctx).tmpInTarget {
                        /* need more input */
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
                        let readBlockCrc: u32 =
                            LZ4F_readLE32(selectedIn.wrapping_add((*dctx).tmpInTarget));
                        let calcBlockCrc: u32 = xxh32(selectedIn, (*dctx).tmpInTarget, 0);
                        if readBlockCrc != calcBlockCrc {
                            return LZ4F_returnErrorCode(LZ4F_ERROR_blockChecksum_invalid);
                        }
                    }
                }

                /* decode directly into destination buffer if there is enough room */
                if (pdiff(dstEnd, dstPtr) >= (*dctx).maxBlockSize)
                    && !(!(*dctx).dict.is_null()
                        && (*dctx).dict.wrapping_add((*dctx).dictSize)
                            == (*dctx).tmpOut as *const u8)
                {
                    let mut dict: *const c_char = (*dctx).dict as *const c_char;
                    let mut dictSize: usize = (*dctx).dictSize;
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
                    if decodedSize < 0 {
                        return LZ4F_returnErrorCode(LZ4F_ERROR_decompressionFailed);
                    }
                    if ((*dctx).frameInfo.contentChecksumFlag != 0) && ((*dctx).skipChecksum == 0)
                    {
                        xxh32_update(&mut (*dctx).xxh, dstPtr, decodedSize as usize);
                    }
                    if (*dctx).frameInfo.contentSize != 0 {
                        (*dctx).frameRemainingSize =
                            (*dctx).frameRemainingSize.wrapping_sub(decodedSize as u64);
                    }

                    /* dictionary management */
                    if (*dctx).frameInfo.blockMode == LZ4F_blockLinked {
                        LZ4F_updateDict(dctx, dstPtr, decodedSize as usize, dstStart, 0);
                    }

                    dstPtr = dstPtr.wrapping_add(decodedSize as usize);
                    (*dctx).dStage = dstage_getBlockHeader; /* end of block */
                    break 'sw;
                }

                /* not enough place into dst : decode into tmpOut */

                /* manage dictionary */
                if (*dctx).frameInfo.blockMode == LZ4F_blockLinked {
                    if (*dctx).dict == (*dctx).tmpOutBuffer as *const u8 {
                        /* truncate dictionary to 64 KB if too big */
                        if (*dctx).dictSize > 128 * 1024 {
                            memcpy(
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
                        /* dict not within tmpOut */
                        let reservedDictSpace: usize = MIN_usize((*dctx).dictSize, 64 * 1024);
                        (*dctx).tmpOut = (*dctx).tmpOutBuffer.wrapping_add(reservedDictSpace);
                    }
                }

                /* Decode block into tmpOut */
                {
                    let mut dict: *const c_char = (*dctx).dict as *const c_char;
                    let mut dictSize: usize = (*dctx).dictSize;
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
                    if decodedSize < 0 {
                        return LZ4F_returnErrorCode(LZ4F_ERROR_decompressionFailed);
                    }
                    if (*dctx).frameInfo.contentChecksumFlag != 0 && (*dctx).skipChecksum == 0 {
                        xxh32_update(&mut (*dctx).xxh, (*dctx).tmpOut, decodedSize as usize);
                    }
                    if (*dctx).frameInfo.contentSize != 0 {
                        (*dctx).frameRemainingSize =
                            (*dctx).frameRemainingSize.wrapping_sub(decodedSize as u64);
                    }
                    (*dctx).tmpOutSize = decodedSize as usize;
                    (*dctx).tmpOutStart = 0;
                    (*dctx).dStage = dstage_flushOut;
                }
                pos = dstage_flushOut;
                /* fall-through */
            }

            if pos == dstage_flushOut {
                /* flush decoded data from tmpOut to dstBuffer */
                if !dstPtr.is_null() {
                    let sizeToCopy: usize = MIN_usize(
                        (*dctx).tmpOutSize - (*dctx).tmpOutStart,
                        pdiff(dstEnd, dstPtr),
                    );
                    memcpy(
                        dstPtr,
                        (*dctx).tmpOut.wrapping_add((*dctx).tmpOutStart),
                        sizeToCopy,
                    );

                    /* dictionary management */
                    if (*dctx).frameInfo.blockMode == LZ4F_blockLinked {
                        LZ4F_updateDict(dctx, dstPtr, sizeToCopy, dstStart, 1 /*withinTmp*/);
                    }

                    (*dctx).tmpOutStart += sizeToCopy;
                    dstPtr = dstPtr.wrapping_add(sizeToCopy);
                }
                if (*dctx).tmpOutStart == (*dctx).tmpOutSize {
                    /* all flushed */
                    (*dctx).dStage = dstage_getBlockHeader; /* get next block */
                    break 'sw;
                }
                /* could not flush everything : stop there, just request a block header */
                doAnotherStage = 0;
                nextSrcSizeHint = BHSize;
                break 'sw;
            }

            if pos == dstage_getSuffix || pos == dstage_storeSuffix {
                let run_store: bool;
                if pos == dstage_getSuffix {
                    if (*dctx).frameRemainingSize != 0 {
                        return LZ4F_returnErrorCode(LZ4F_ERROR_frameSize_wrong);
                    }
                    if (*dctx).frameInfo.contentChecksumFlag == 0 {
                        /* no checksum, frame is completed */
                        nextSrcSizeHint = 0;
                        LZ4F_resetDecompressionContext(dctx);
                        doAnotherStage = 0;
                        break 'sw;
                    }
                    if pdiff_i(srcEnd, srcPtr) < 4 {
                        /* not enough size for entire CRC */
                        (*dctx).tmpInSize = 0;
                        (*dctx).dStage = dstage_storeSuffix;
                    } else {
                        selectedIn = srcPtr;
                        srcPtr = srcPtr.wrapping_add(4);
                    }
                    run_store = (*dctx).dStage == dstage_storeSuffix;
                } else {
                    run_store = true;
                }

                if run_store {
                    let remainingInput: usize = pdiff(srcEnd, srcPtr);
                    let wantedData: usize = 4 - (*dctx).tmpInSize;
                    let sizeToCopy: usize = MIN_usize(wantedData, remainingInput);
                    memcpy(
                        (*dctx).tmpIn.wrapping_add((*dctx).tmpInSize),
                        srcPtr,
                        sizeToCopy,
                    );
                    srcPtr = srcPtr.wrapping_add(sizeToCopy);
                    (*dctx).tmpInSize += sizeToCopy;
                    if (*dctx).tmpInSize < 4 {
                        /* not enough input to read complete suffix */
                        nextSrcSizeHint = 4 - (*dctx).tmpInSize;
                        doAnotherStage = 0;
                        break 'sw;
                    }
                    selectedIn = (*dctx).tmpIn;
                }

                /* case dstage_checkSuffix: */
                if (*dctx).skipChecksum == 0 {
                    let readCRC: u32 = LZ4F_readLE32(selectedIn);
                    let resultCRC: u32 = xxh32_digest(&(*dctx).xxh);
                    if readCRC != resultCRC {
                        return LZ4F_returnErrorCode(LZ4F_ERROR_contentChecksum_invalid);
                    }
                }
                nextSrcSizeHint = 0;
                LZ4F_resetDecompressionContext(dctx);
                doAnotherStage = 0;
                break 'sw;
            }

            if pos == dstage_getSFrameSize || pos == dstage_storeSFrameSize {
                let run_store: bool;
                if pos == dstage_getSFrameSize {
                    if pdiff_i(srcEnd, srcPtr) >= 4 {
                        selectedIn = srcPtr;
                        srcPtr = srcPtr.wrapping_add(4);
                    } else {
                        /* not enough input to read cBlockSize field */
                        (*dctx).tmpInSize = 4;
                        (*dctx).tmpInTarget = 8;
                        (*dctx).dStage = dstage_storeSFrameSize;
                    }
                    run_store = (*dctx).dStage == dstage_storeSFrameSize;
                } else {
                    run_store = true;
                }

                if run_store {
                    let sizeToCopy: usize = MIN_usize(
                        (*dctx).tmpInTarget - (*dctx).tmpInSize,
                        pdiff(srcEnd, srcPtr),
                    );
                    memcpy(
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

                /* case dstage_decodeSFrameSize: */
                {
                    let SFrameSize: usize = LZ4F_readLE32(selectedIn) as usize;
                    (*dctx).frameInfo.contentSize = SFrameSize as u64;
                    (*dctx).tmpInTarget = SFrameSize;
                    (*dctx).dStage = dstage_skipSkippable;
                    break 'sw;
                }
            }

            if pos == dstage_skipSkippable {
                let skipSize: usize = MIN_usize((*dctx).tmpInTarget, pdiff(srcEnd, srcPtr));
                srcPtr = srcPtr.wrapping_add(skipSize);
                (*dctx).tmpInTarget -= skipSize;
                doAnotherStage = 0;
                nextSrcSizeHint = (*dctx).tmpInTarget;
                if nextSrcSizeHint != 0 {
                    break 'sw; /* still more to skip */
                }
                /* frame fully skipped : prepare context for a new frame */
                LZ4F_resetDecompressionContext(dctx);
                break 'sw;
            }

            /* unreachable */
            break 'sw;
        }
    }

    /* preserve history within tmpOut whenever necessary */
    if ((*dctx).frameInfo.blockMode == LZ4F_blockLinked)
        && ((*dctx).dict != (*dctx).tmpOutBuffer as *const u8)
        && (!(*dctx).dict.is_null())
        && ((*decompressOptionsPtr).stableDst == 0)
        && (((*dctx).dStage as u32).wrapping_sub(2) < (dstage_getSuffix as u32).wrapping_sub(2))
    {
        if (*dctx).dStage == dstage_flushOut {
            let preserveSize: usize =
                pdiff((*dctx).tmpOut as *const u8, (*dctx).tmpOutBuffer as *const u8);
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

            memcpy(
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
            let newDictSize: usize = MIN_usize((*dctx).dictSize, 64 * 1024);

            memcpy(
                (*dctx).tmpOutBuffer,
                oldDictEnd.wrapping_sub(newDictSize),
                newDictSize,
            );

            (*dctx).dict = (*dctx).tmpOutBuffer as *const u8;
            (*dctx).dictSize = newDictSize;
            (*dctx).tmpOut = (*dctx).tmpOutBuffer.wrapping_add(newDictSize);
        }
    }

    *srcSizePtr = pdiff(srcPtr, srcStart);
    *dstSizePtr = pdiff(dstPtr, dstStart);
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
