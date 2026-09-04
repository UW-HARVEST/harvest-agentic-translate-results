//! Translation of `lz4frame.c` (LZ4 v1.10.0), built with `LZ4F_HEAPMODE=0`.
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![allow(unused_assignments)]

use crate::common::*;
use crate::lz4::*;
use crate::lz4hc::*;
use crate::xxhash::*;
use core::ffi::{c_char, c_int, c_uint, c_void};

pub const LZ4F_VERSION: c_uint = 100;
pub const LZ4F_HEADER_SIZE_MIN: usize = 7;
pub const LZ4F_HEADER_SIZE_MAX: usize = 19;
pub const LZ4F_BLOCK_HEADER_SIZE: usize = 4;
pub const LZ4F_BLOCK_CHECKSUM_SIZE: usize = 4;
pub const LZ4F_CONTENT_CHECKSUM_SIZE: usize = 4;
pub const LZ4F_MAGICNUMBER: u32 = 0x184D2204;
pub const LZ4F_MAGIC_SKIPPABLE_START: u32 = 0x184D2A50;
pub const LZ4F_MIN_SIZE_TO_KNOW_HEADER_LENGTH: usize = 5;

pub const _1BIT: u32 = 0x01;
pub const _2BITS: u32 = 0x03;
pub const _3BITS: u32 = 0x07;
pub const _4BITS: u32 = 0x0F;

pub const LZ4F_BLOCKUNCOMPRESSED_FLAG: u32 = 0x80000000;

/* LZ4F_blockSizeID_t */
pub const LZ4F_default: c_uint = 0;
pub const LZ4F_max64KB: c_uint = 4;
pub const LZ4F_max256KB: c_uint = 5;
pub const LZ4F_max1MB: c_uint = 6;
pub const LZ4F_max4MB: c_uint = 7;
pub const LZ4F_BLOCKSIZEID_DEFAULT: c_uint = LZ4F_max64KB;

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

/* LZ4F_BlockCompressMode_e */
const LZ4B_COMPRESSED: c_int = 0;
const LZ4B_UNCOMPRESSED: c_int = 1;

/* LZ4F_CtxType_e */
const ctxNone: u16 = 0;
const ctxFast: u16 = 1;
const ctxHC: u16 = 2;

pub const minFHSize: usize = LZ4F_HEADER_SIZE_MIN;
pub const maxFHSize: usize = LZ4F_HEADER_SIZE_MAX;
pub const BHSize: usize = LZ4F_BLOCK_HEADER_SIZE;
pub const BFSize: usize = LZ4F_BLOCK_CHECKSUM_SIZE;

pub type LZ4F_errorCode_t = usize;

/* ===== error codes ===== */
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
pub fn LZ4F_returnErrorCode(code: c_int) -> LZ4F_errorCode_t {
    (0isize).wrapping_sub(code as isize) as LZ4F_errorCode_t
}

#[unsafe(no_mangle)]
pub extern "C" fn LZ4F_isError(code: LZ4F_errorCode_t) -> c_uint {
    (code > LZ4F_returnErrorCode(LZ4F_ERROR_maxCode)) as c_uint
}

#[unsafe(no_mangle)]
pub extern "C" fn LZ4F_getErrorName(code: LZ4F_errorCode_t) -> *const c_char {
    if LZ4F_isError(code) != 0 {
        let idx = (0isize).wrapping_sub(code as isize) as usize;
        return LZ4F_errorStrings[idx].as_ptr() as *const c_char;
    }
    codeError.as_ptr() as *const c_char
}

#[unsafe(no_mangle)]
pub extern "C" fn LZ4F_getErrorCode(functionResult: usize) -> c_int {
    if LZ4F_isError(functionResult) == 0 {
        return LZ4F_OK_NoError;
    }
    (0isize).wrapping_sub(functionResult as isize) as c_int
}

/* ===== public structures ===== */

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

pub const LZ4F_INIT_FRAMEINFO: LZ4F_frameInfo_t = LZ4F_frameInfo_t {
    blockSizeID: LZ4F_max64KB,
    blockMode: LZ4F_blockLinked,
    contentChecksumFlag: LZ4F_noContentChecksum,
    frameType: LZ4F_frame,
    contentSize: 0,
    dictID: 0,
    blockChecksumFlag: LZ4F_noBlockChecksum,
};

pub const LZ4F_INIT_PREFERENCES: LZ4F_preferences_t = LZ4F_preferences_t {
    frameInfo: LZ4F_INIT_FRAMEINFO,
    compressionLevel: 0,
    autoFlush: 0,
    favorDecSpeed: 0,
    reserved: [0, 0, 0],
};

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

const _: () = {
    assert!(core::mem::size_of::<LZ4F_frameInfo_t>() == 32);
    assert!(core::mem::size_of::<LZ4F_preferences_t>() == 56);
};

/* ===== memory helpers ===== */

unsafe fn LZ4F_calloc(s: usize, cmem: LZ4F_CustomMem) -> *mut c_void {
    unsafe {
        if let Some(cc) = cmem.customCalloc {
            return cc(cmem.opaqueState, s);
        }
        if cmem.customAlloc.is_none() {
            return ALLOC_AND_ZERO(s);
        }
        let p = (cmem.customAlloc.unwrap())(cmem.opaqueState, s);
        if !p.is_null() {
            MEM_INIT(p as *mut u8, 0, s);
        }
        p
    }
}

unsafe fn LZ4F_malloc(s: usize, cmem: LZ4F_CustomMem) -> *mut c_void {
    unsafe {
        if let Some(ca) = cmem.customAlloc {
            return ca(cmem.opaqueState, s);
        }
        ALLOC(s)
    }
}

unsafe fn LZ4F_free(p: *mut c_void, cmem: LZ4F_CustomMem) {
    unsafe {
        if p.is_null() {
            return;
        }
        if let Some(cf) = cmem.customFree {
            cf(cmem.opaqueState, p);
            return;
        }
        FREEMEM(p);
    }
}

/* ===== LE read/write ===== */
#[inline(always)]
unsafe fn LZ4F_readLE32(src: *const c_void) -> u32 {
    unsafe {
        let p = src as *const u8;
        (*p as u32) | ((*p.wrapping_add(1) as u32) << 8) | ((*p.wrapping_add(2) as u32) << 16)
            | ((*p.wrapping_add(3) as u32) << 24)
    }
}
#[inline(always)]
unsafe fn LZ4F_writeLE32(dst: *mut c_void, v: u32) {
    unsafe {
        let p = dst as *mut u8;
        *p = v as u8;
        *p.wrapping_add(1) = (v >> 8) as u8;
        *p.wrapping_add(2) = (v >> 16) as u8;
        *p.wrapping_add(3) = (v >> 24) as u8;
    }
}
#[inline(always)]
unsafe fn LZ4F_readLE64(src: *const c_void) -> u64 {
    unsafe {
        let p = src as *const u8;
        let mut v = *p as u64;
        v |= (*p.wrapping_add(1) as u64) << 8;
        v |= (*p.wrapping_add(2) as u64) << 16;
        v |= (*p.wrapping_add(3) as u64) << 24;
        v |= (*p.wrapping_add(4) as u64) << 32;
        v |= (*p.wrapping_add(5) as u64) << 40;
        v |= (*p.wrapping_add(6) as u64) << 48;
        v |= (*p.wrapping_add(7) as u64) << 56;
        v
    }
}
#[inline(always)]
unsafe fn LZ4F_writeLE64(dst: *mut c_void, v: u64) {
    unsafe {
        let p = dst as *mut u8;
        *p = v as u8;
        *p.wrapping_add(1) = (v >> 8) as u8;
        *p.wrapping_add(2) = (v >> 16) as u8;
        *p.wrapping_add(3) = (v >> 24) as u8;
        *p.wrapping_add(4) = (v >> 32) as u8;
        *p.wrapping_add(5) = (v >> 40) as u8;
        *p.wrapping_add(6) = (v >> 48) as u8;
        *p.wrapping_add(7) = (v >> 56) as u8;
    }
}

/* ===== contexts ===== */

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

#[repr(C)]
pub struct LZ4F_CDict {
    pub cmem: LZ4F_CustomMem,
    pub dictContent: *mut c_void,
    pub fastCtx: *mut LZ4_stream_t,
    pub HCCtx: *mut LZ4_streamHC_t,
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
pub extern "C" fn LZ4F_getBlockSize(mut blockSizeID: c_uint) -> usize {
    static blockSizes: [usize; 4] = [64 * KB, 256 * KB, 1024 * KB, 4096 * KB];

    if blockSizeID == 0 {
        blockSizeID = LZ4F_BLOCKSIZEID_DEFAULT;
    }
    if blockSizeID < LZ4F_max64KB || blockSizeID > LZ4F_max4MB {
        return LZ4F_returnErrorCode(LZ4F_ERROR_maxBlockSize_invalid);
    }
    let blockSizeIdx = (blockSizeID - LZ4F_max64KB) as usize;
    blockSizes[blockSizeIdx]
}

#[inline(always)]
fn MIN(a: usize, b: usize) -> usize {
    if a < b { a } else { b }
}

unsafe fn LZ4F_headerChecksum(header: *const c_void, length: usize) -> u8 {
    unsafe {
        let xxh = LZ4_XXH32(header, length, 0);
        (xxh >> 8) as u8
    }
}

/* ===== Simple-pass compression ===== */

fn LZ4F_optimalBSID(requestedBSID: c_uint, srcSize: usize) -> c_uint {
    let mut proposedBSID = LZ4F_max64KB;
    let mut maxBlockSize: usize = 64 * KB;
    while requestedBSID > proposedBSID {
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
    unsafe {
        let mut prefsNull = LZ4F_INIT_PREFERENCES;
        prefsNull.frameInfo.contentChecksumFlag = LZ4F_contentChecksumEnabled;
        prefsNull.frameInfo.blockChecksumFlag = LZ4F_blockChecksumEnabled;
        let prefsPtr: *const LZ4F_preferences_t = if preferencesPtr.is_null() {
            &prefsNull
        } else {
            preferencesPtr
        };
        let flush: u32 = (*prefsPtr).autoFlush | ((srcSize == 0) as u32);
        let blockID = (*prefsPtr).frameInfo.blockSizeID;
        let blockSize = LZ4F_getBlockSize(blockID);
        let maxBuffered = blockSize - 1;
        let bufferedSize = MIN(alreadyBuffered, maxBuffered);
        let maxSrcSize = srcSize + bufferedSize;
        let nbFullBlocks = (maxSrcSize / blockSize) as u32;
        let partialBlockSize = maxSrcSize & (blockSize - 1);
        let lastBlockSize = if flush != 0 { partialBlockSize } else { 0 };
        let nbBlocks = nbFullBlocks + ((lastBlockSize > 0) as u32);

        let blockCRCSize = BFSize * (*prefsPtr).frameInfo.blockChecksumFlag as usize;
        let frameEnd = BHSize + ((*prefsPtr).frameInfo.contentChecksumFlag as usize * BFSize);

        ((BHSize + blockCRCSize) * nbBlocks as usize)
            + (blockSize * nbFullBlocks as usize)
            + lastBlockSize
            + frameEnd
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_compressFrameBound(
    srcSize: usize,
    preferencesPtr: *const LZ4F_preferences_t,
) -> usize {
    unsafe {
        let mut prefs: LZ4F_preferences_t;
        let headerSize = maxFHSize;

        if !preferencesPtr.is_null() {
            prefs = *preferencesPtr;
        } else {
            prefs = core::mem::zeroed();
        }
        prefs.autoFlush = 1;

        headerSize + LZ4F_compressBound_internal(srcSize, &prefs, 0)
    }
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
    unsafe {
        let mut prefs: LZ4F_preferences_t;
        let mut options: LZ4F_compressOptions_t;
        let dstStart = dstBuffer as *mut u8;
        let mut dstPtr = dstStart;
        let dstEnd = madd(dstStart, dstCapacity);

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

        if dstCapacity < LZ4F_compressFrameBound(srcSize, &prefs) {
            return LZ4F_returnErrorCode(LZ4F_ERROR_dstMaxSize_tooSmall);
        }

        {
            let headerSize =
                LZ4F_compressBegin_usingCDict(cctx, dstBuffer, dstCapacity, cdict, &prefs);
            if LZ4F_isError(headerSize) != 0 {
                return headerSize;
            }
            dstPtr = madd(dstPtr, headerSize);
        }

        {
            let cSize = LZ4F_compressUpdate(
                cctx,
                dstPtr as *mut c_void,
                pdiff(dstEnd, dstPtr) as usize,
                srcBuffer,
                srcSize,
                &options,
            );
            if LZ4F_isError(cSize) != 0 {
                return cSize;
            }
            dstPtr = madd(dstPtr, cSize);
        }

        {
            let tailSize = LZ4F_compressEnd(
                cctx,
                dstPtr as *mut c_void,
                pdiff(dstEnd, dstPtr) as usize,
                &options,
            );
            if LZ4F_isError(tailSize) != 0 {
                return tailSize;
            }
            dstPtr = madd(dstPtr, tailSize);
        }

        pdiff(dstPtr, dstStart) as usize
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_compressFrame(
    dstBuffer: *mut c_void,
    dstCapacity: usize,
    srcBuffer: *const c_void,
    srcSize: usize,
    preferencesPtr: *const LZ4F_preferences_t,
) -> usize {
    unsafe {
        let result: usize;
        let mut cctx: LZ4F_cctx = core::mem::zeroed();
        let mut lz4ctx = core::mem::MaybeUninit::<LZ4_stream_t>::uninit();
        let cctxPtr: *mut LZ4F_cctx = &mut cctx;

        cctx.version = LZ4F_VERSION;
        cctx.maxBufferSize = 5 * 1024 * KB;
        if preferencesPtr.is_null() || (*preferencesPtr).compressionLevel < LZ4HC_CLEVEL_MIN {
            LZ4_initStream(
                lz4ctx.as_mut_ptr() as *mut c_void,
                core::mem::size_of::<LZ4_stream_t>(),
            );
            (*cctxPtr).lz4CtxPtr = lz4ctx.as_mut_ptr() as *mut c_void;
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
}

/* ===== Dictionary compression ===== */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_createCDict_advanced(
    cmem: LZ4F_CustomMem,
    dictBuffer: *const c_void,
    mut dictSize: usize,
) -> *mut LZ4F_CDict {
    unsafe {
        let mut dictStart = dictBuffer as *const c_char;
        let cdict = LZ4F_malloc(core::mem::size_of::<LZ4F_CDict>(), cmem) as *mut LZ4F_CDict;
        if cdict.is_null() {
            return core::ptr::null_mut();
        }
        (*cdict).cmem = cmem;
        if dictSize > 64 * KB {
            dictStart = cadd(dictStart as *const u8, dictSize - 64 * KB) as *const c_char;
            dictSize = 64 * KB;
        }
        (*cdict).dictContent = LZ4F_malloc(dictSize, cmem);
        (*cdict).fastCtx =
            LZ4F_malloc(core::mem::size_of::<LZ4_stream_t>(), cmem) as *mut LZ4_stream_t;
        (*cdict).HCCtx =
            LZ4F_malloc(core::mem::size_of::<LZ4_streamHC_t>(), cmem) as *mut LZ4_streamHC_t;
        if (*cdict).dictContent.is_null()
            || (*cdict).fastCtx.is_null()
            || (*cdict).HCCtx.is_null()
        {
            LZ4F_freeCDict(cdict);
            return core::ptr::null_mut();
        }
        LZ4_memcpy(
            (*cdict).dictContent as *mut u8,
            dictStart as *const u8,
            dictSize,
        );
        LZ4_initStream(
            (*cdict).fastCtx as *mut c_void,
            core::mem::size_of::<LZ4_stream_t>(),
        );
        LZ4_loadDictSlow(
            (*cdict).fastCtx,
            (*cdict).dictContent as *const c_char,
            dictSize as c_int,
        );
        LZ4_initStreamHC(
            (*cdict).HCCtx as *mut c_void,
            core::mem::size_of::<LZ4_streamHC_t>(),
        );
        LZ4_setCompressionLevel((*cdict).HCCtx, LZ4HC_CLEVEL_DEFAULT);
        LZ4_loadDictHC(
            (*cdict).HCCtx,
            (*cdict).dictContent as *const c_char,
            dictSize as c_int,
        );
        cdict
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_createCDict(
    dictBuffer: *const c_void,
    dictSize: usize,
) -> *mut LZ4F_CDict {
    unsafe { LZ4F_createCDict_advanced(LZ4F_defaultCMem, dictBuffer, dictSize) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_freeCDict(cdict: *mut LZ4F_CDict) {
    unsafe {
        if cdict.is_null() {
            return;
        }
        LZ4F_free((*cdict).dictContent, (*cdict).cmem);
        LZ4F_free((*cdict).fastCtx as *mut c_void, (*cdict).cmem);
        LZ4F_free((*cdict).HCCtx as *mut c_void, (*cdict).cmem);
        LZ4F_free(cdict as *mut c_void, (*cdict).cmem);
    }
}

/* ===== Advanced compression functions ===== */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_createCompressionContext_advanced(
    customMem: LZ4F_CustomMem,
    version: c_uint,
) -> *mut LZ4F_cctx {
    unsafe {
        let cctxPtr = LZ4F_calloc(core::mem::size_of::<LZ4F_cctx>(), customMem) as *mut LZ4F_cctx;
        if cctxPtr.is_null() {
            return core::ptr::null_mut();
        }
        (*cctxPtr).cmem = customMem;
        (*cctxPtr).version = version;
        (*cctxPtr).cStage = 0;
        cctxPtr
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_createCompressionContext(
    LZ4F_compressionContextPtr: *mut *mut LZ4F_cctx,
    version: c_uint,
) -> LZ4F_errorCode_t {
    unsafe {
        if LZ4F_compressionContextPtr.is_null() {
            return LZ4F_returnErrorCode(LZ4F_ERROR_parameter_null);
        }
        *LZ4F_compressionContextPtr =
            LZ4F_createCompressionContext_advanced(LZ4F_defaultCMem, version);
        if (*LZ4F_compressionContextPtr).is_null() {
            return LZ4F_returnErrorCode(LZ4F_ERROR_allocation_failed);
        }
        LZ4F_OK_NoError as LZ4F_errorCode_t
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_freeCompressionContext(cctxPtr: *mut LZ4F_cctx) -> LZ4F_errorCode_t {
    unsafe {
        if !cctxPtr.is_null() {
            LZ4F_free((*cctxPtr).lz4CtxPtr, (*cctxPtr).cmem);
            LZ4F_free((*cctxPtr).tmpBuff as *mut c_void, (*cctxPtr).cmem);
            LZ4F_free(cctxPtr as *mut c_void, (*cctxPtr).cmem);
        }
        LZ4F_OK_NoError as LZ4F_errorCode_t
    }
}

unsafe fn LZ4F_initStream_internal(
    ctx: *mut c_void,
    cdict: *const LZ4F_CDict,
    level: c_int,
    blockMode: c_uint,
) {
    unsafe {
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
    unsafe {
        let prefNull = LZ4F_INIT_PREFERENCES;
        let dstStart = dstBuffer as *mut u8;
        let mut dstPtr = dstStart;

        if dstCapacity < maxFHSize {
            return LZ4F_returnErrorCode(LZ4F_ERROR_dstMaxSize_tooSmall);
        }
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
                LZ4F_free((*cctx).lz4CtxPtr, (*cctx).cmem);
                if (*cctx).prefs.compressionLevel < LZ4HC_CLEVEL_MIN {
                    (*cctx).lz4CtxPtr =
                        LZ4F_malloc(core::mem::size_of::<LZ4_stream_t>(), (*cctx).cmem);
                    if !(*cctx).lz4CtxPtr.is_null() {
                        LZ4_initStream(
                            (*cctx).lz4CtxPtr,
                            core::mem::size_of::<LZ4_stream_t>(),
                        );
                    }
                } else {
                    (*cctx).lz4CtxPtr =
                        LZ4F_malloc(core::mem::size_of::<LZ4_streamHC_t>(), (*cctx).cmem);
                    if !(*cctx).lz4CtxPtr.is_null() {
                        LZ4_initStreamHC(
                            (*cctx).lz4CtxPtr,
                            core::mem::size_of::<LZ4_streamHC_t>(),
                        );
                    }
                }
                if (*cctx).lz4CtxPtr.is_null() {
                    return LZ4F_returnErrorCode(LZ4F_ERROR_allocation_failed);
                }
                (*cctx).lz4CtxAlloc = ctxTypeID;
                (*cctx).lz4CtxType = ctxTypeID;
            } else if (*cctx).lz4CtxType != ctxTypeID {
                if (*cctx).prefs.compressionLevel < LZ4HC_CLEVEL_MIN {
                    LZ4_initStream((*cctx).lz4CtxPtr, core::mem::size_of::<LZ4_stream_t>());
                } else {
                    LZ4_initStreamHC(
                        (*cctx).lz4CtxPtr,
                        core::mem::size_of::<LZ4_streamHC_t>(),
                    );
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
                    64 * KB
                } else {
                    0
                }
            } else {
                (*cctx).maxBlockSize
                    + (if (*cctx).prefs.frameInfo.blockMode == LZ4F_blockLinked {
                        128 * KB
                    } else {
                        0
                    })
            };

            if (*cctx).maxBufferSize < requiredBuffSize {
                (*cctx).maxBufferSize = 0;
                LZ4F_free((*cctx).tmpBuff as *mut c_void, (*cctx).cmem);
                (*cctx).tmpBuff = LZ4F_malloc(requiredBuffSize, (*cctx).cmem) as *mut u8;
                if (*cctx).tmpBuff.is_null() {
                    return LZ4F_returnErrorCode(LZ4F_ERROR_allocation_failed);
                }
                (*cctx).maxBufferSize = requiredBuffSize;
            }
        }
        (*cctx).tmpIn = (*cctx).tmpBuff;
        (*cctx).tmpInSize = 0;
        LZ4_XXH32_reset(&mut (*cctx).xxh, 0);

        /* context init */
        (*cctx).cdict = cdict;
        if (*cctx).prefs.frameInfo.blockMode == LZ4F_blockLinked {
            LZ4F_initStream_internal(
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
        LZ4F_writeLE32(dstPtr as *mut c_void, LZ4F_MAGICNUMBER);
        dstPtr = madd(dstPtr, 4);
        {
            let headerStart = dstPtr;

            /* FLG Byte */
            *dstPtr = (((1u32 & _2BITS) << 6)
                + (((*cctx).prefs.frameInfo.blockMode & _1BIT) << 5)
                + (((*cctx).prefs.frameInfo.blockChecksumFlag & _1BIT) << 4)
                + ((((*cctx).prefs.frameInfo.contentSize > 0) as u32) << 3)
                + (((*cctx).prefs.frameInfo.contentChecksumFlag & _1BIT) << 2)
                + (((*cctx).prefs.frameInfo.dictID > 0) as u32)) as u8;
            dstPtr = madd(dstPtr, 1);
            /* BD Byte */
            *dstPtr = (((*cctx).prefs.frameInfo.blockSizeID & _3BITS) << 4) as u8;
            dstPtr = madd(dstPtr, 1);
            /* Optional Frame content size field */
            if (*cctx).prefs.frameInfo.contentSize != 0 {
                LZ4F_writeLE64(dstPtr as *mut c_void, (*cctx).prefs.frameInfo.contentSize);
                dstPtr = madd(dstPtr, 8);
                (*cctx).totalInSize = 0;
            }
            /* Optional dictionary ID field */
            if (*cctx).prefs.frameInfo.dictID != 0 {
                LZ4F_writeLE32(dstPtr as *mut c_void, (*cctx).prefs.frameInfo.dictID);
                dstPtr = madd(dstPtr, 4);
            }
            /* Header CRC Byte */
            *dstPtr = LZ4F_headerChecksum(
                headerStart as *const c_void,
                pdiff(dstPtr, headerStart) as usize,
            );
            dstPtr = madd(dstPtr, 1);
        }

        (*cctx).cStage = 1;
        pdiff(dstPtr, dstStart) as usize
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_compressBegin(
    cctx: *mut LZ4F_cctx,
    dstBuffer: *mut c_void,
    dstCapacity: usize,
    preferencesPtr: *const LZ4F_preferences_t,
) -> usize {
    unsafe {
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
    unsafe {
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
    unsafe {
        LZ4F_compressBegin_usingDictOnce(
            cctx,
            dstBuffer,
            dstCapacity,
            dict,
            dictSize,
            preferencesPtr,
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_compressBegin_usingCDict(
    cctx: *mut LZ4F_cctx,
    dstBuffer: *mut c_void,
    dstCapacity: usize,
    cdict: *const LZ4F_CDict,
    preferencesPtr: *const LZ4F_preferences_t,
) -> usize {
    unsafe {
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
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_compressBound(
    srcSize: usize,
    preferencesPtr: *const LZ4F_preferences_t,
) -> usize {
    unsafe {
        if !preferencesPtr.is_null() && (*preferencesPtr).autoFlush != 0 {
            return LZ4F_compressBound_internal(srcSize, preferencesPtr, 0);
        }
        LZ4F_compressBound_internal(srcSize, preferencesPtr, usize::MAX)
    }
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
    unsafe {
        let cSizePtr = dst as *mut u8;
        let mut cSize: u32;
        cSize = compress(
            lz4ctx,
            src as *const c_char,
            madd(cSizePtr, BHSize) as *mut c_char,
            srcSize as c_int,
            (srcSize - 1) as c_int,
            level,
            cdict,
        ) as u32;

        if cSize == 0 || cSize as usize >= srcSize {
            cSize = srcSize as u32;
            LZ4F_writeLE32(cSizePtr as *mut c_void, cSize | LZ4F_BLOCKUNCOMPRESSED_FLAG);
            LZ4_memcpy(madd(cSizePtr, BHSize), src as *const u8, srcSize);
        } else {
            LZ4F_writeLE32(cSizePtr as *mut c_void, cSize);
        }
        if crcFlag != 0 {
            let crc32 = LZ4_XXH32(madd(cSizePtr, BHSize) as *const c_void, cSize as usize, 0);
            LZ4F_writeLE32(
                madd(cSizePtr, BHSize + cSize as usize) as *mut c_void,
                crc32,
            );
        }
        BHSize + cSize as usize + (crcFlag as usize) * BFSize
    }
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
    unsafe {
        let acceleration = if level < 0 { -level + 1 } else { 1 };
        LZ4F_initStream_internal(ctx, cdict, level, LZ4F_blockIndependent);
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
    unsafe {
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
    unsafe {
        LZ4F_initStream_internal(ctx, cdict, level, LZ4F_blockIndependent);
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
    unsafe {
        LZ4_compress_HC_continue(ctx as *mut LZ4_streamHC_t, src, dst, srcSize, dstCapacity)
    }
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

fn LZ4F_selectCompression(blockMode: c_uint, level: c_int, compressMode: c_int) -> compressFunc_t {
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

unsafe fn LZ4F_localSaveDict(cctxPtr: *mut LZ4F_cctx) -> c_int {
    unsafe {
        if (*cctxPtr).prefs.compressionLevel < LZ4HC_CLEVEL_MIN {
            return LZ4_saveDict(
                (*cctxPtr).lz4CtxPtr as *mut LZ4_stream_t,
                (*cctxPtr).tmpBuff as *mut c_char,
                (64 * KB) as c_int,
            );
        }
        LZ4_saveDictHC(
            (*cctxPtr).lz4CtxPtr as *mut LZ4_streamHC_t,
            (*cctxPtr).tmpBuff as *mut c_char,
            (64 * KB) as c_int,
        )
    }
}

const notDone: c_int = 0;
const fromTmpBuffer: c_int = 1;
const fromSrcBuffer: c_int = 2;

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
    unsafe {
        let blockSize = (*cctxPtr).maxBlockSize;
        let mut srcPtr = srcBuffer as *const u8;
        let srcEnd = cadd(srcPtr, srcSize);
        let dstStart = dstBuffer as *mut u8;
        let mut dstPtr = dstStart;
        let mut lastBlockCompressed = notDone;
        let compress = LZ4F_selectCompression(
            (*cctxPtr).prefs.frameInfo.blockMode,
            (*cctxPtr).prefs.compressionLevel,
            blockCompression,
        );
        let bytesWritten: usize;

        if (*cctxPtr).cStage != 1 {
            return LZ4F_returnErrorCode(LZ4F_ERROR_compressionState_uninitialized);
        }
        if dstCapacity
            < LZ4F_compressBound_internal(srcSize, &(*cctxPtr).prefs, (*cctxPtr).tmpInSize)
        {
            return LZ4F_returnErrorCode(LZ4F_ERROR_dstMaxSize_tooSmall);
        }

        if blockCompression == LZ4B_UNCOMPRESSED && dstCapacity < srcSize {
            return LZ4F_returnErrorCode(LZ4F_ERROR_dstMaxSize_tooSmall);
        }

        /* flush currently written block, to continue with new block compression */
        if (*cctxPtr).blockCompressMode != blockCompression {
            bytesWritten = LZ4F_flush(cctxPtr, dstBuffer, dstCapacity, compressOptionsPtr);
            dstPtr = madd(dstPtr, bytesWritten);
            (*cctxPtr).blockCompressMode = blockCompression;
        }

        if compressOptionsPtr.is_null() {
            compressOptionsPtr = &k_cOptionsNull;
        }

        /* complete tmp buffer */
        if (*cctxPtr).tmpInSize > 0 {
            let sizeToCopy = blockSize - (*cctxPtr).tmpInSize;
            if sizeToCopy > srcSize {
                LZ4_memcpy(
                    madd((*cctxPtr).tmpIn, (*cctxPtr).tmpInSize),
                    srcBuffer as *const u8,
                    srcSize,
                );
                srcPtr = srcEnd;
                (*cctxPtr).tmpInSize += srcSize;
            } else {
                lastBlockCompressed = fromTmpBuffer;
                LZ4_memcpy(
                    madd((*cctxPtr).tmpIn, (*cctxPtr).tmpInSize),
                    srcBuffer as *const u8,
                    sizeToCopy,
                );
                srcPtr = cadd(srcPtr, sizeToCopy);

                dstPtr = madd(
                    dstPtr,
                    LZ4F_makeBlock(
                        dstPtr as *mut c_void,
                        (*cctxPtr).tmpIn as *const c_void,
                        blockSize,
                        compress,
                        (*cctxPtr).lz4CtxPtr,
                        (*cctxPtr).prefs.compressionLevel,
                        (*cctxPtr).cdict,
                        (*cctxPtr).prefs.frameInfo.blockChecksumFlag,
                    ),
                );
                if (*cctxPtr).prefs.frameInfo.blockMode == LZ4F_blockLinked {
                    (*cctxPtr).tmpIn = madd((*cctxPtr).tmpIn, blockSize);
                }
                (*cctxPtr).tmpInSize = 0;
            }
        }

        while (pdiff(srcEnd, srcPtr) as usize) >= blockSize {
            lastBlockCompressed = fromSrcBuffer;
            dstPtr = madd(
                dstPtr,
                LZ4F_makeBlock(
                    dstPtr as *mut c_void,
                    srcPtr as *const c_void,
                    blockSize,
                    compress,
                    (*cctxPtr).lz4CtxPtr,
                    (*cctxPtr).prefs.compressionLevel,
                    (*cctxPtr).cdict,
                    (*cctxPtr).prefs.frameInfo.blockChecksumFlag,
                ),
            );
            srcPtr = cadd(srcPtr, blockSize);
        }

        if ((*cctxPtr).prefs.autoFlush != 0) && (srcPtr < srcEnd) {
            lastBlockCompressed = fromSrcBuffer;
            dstPtr = madd(
                dstPtr,
                LZ4F_makeBlock(
                    dstPtr as *mut c_void,
                    srcPtr as *const c_void,
                    pdiff(srcEnd, srcPtr) as usize,
                    compress,
                    (*cctxPtr).lz4CtxPtr,
                    (*cctxPtr).prefs.compressionLevel,
                    (*cctxPtr).cdict,
                    (*cctxPtr).prefs.frameInfo.blockChecksumFlag,
                ),
            );
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
                (*cctxPtr).tmpIn = madd((*cctxPtr).tmpBuff, realDictSize as usize);
            }
        }

        /* keep tmpIn within limits */
        if ((*cctxPtr).prefs.autoFlush == 0)
            && (madd((*cctxPtr).tmpIn, blockSize)
                > madd((*cctxPtr).tmpBuff, (*cctxPtr).maxBufferSize))
        {
            let realDictSize = LZ4F_localSaveDict(cctxPtr);
            (*cctxPtr).tmpIn = madd((*cctxPtr).tmpBuff, realDictSize as usize);
        }

        /* some input data left, necessarily < blockSize */
        if srcPtr < srcEnd {
            let sizeToCopy = pdiff(srcEnd, srcPtr) as usize;
            LZ4_memcpy((*cctxPtr).tmpIn, srcPtr, sizeToCopy);
            (*cctxPtr).tmpInSize = sizeToCopy;
        }

        if (*cctxPtr).prefs.frameInfo.contentChecksumFlag == LZ4F_contentChecksumEnabled {
            LZ4_XXH32_update(&mut (*cctxPtr).xxh, srcBuffer, srcSize);
        }

        (*cctxPtr).totalInSize += srcSize as u64;
        pdiff(dstPtr, dstStart) as usize
    }
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
    unsafe {
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
    unsafe {
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
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_flush(
    cctxPtr: *mut LZ4F_cctx,
    dstBuffer: *mut c_void,
    dstCapacity: usize,
    _compressOptionsPtr: *const LZ4F_compressOptions_t,
) -> usize {
    unsafe {
        let dstStart = dstBuffer as *mut u8;
        let mut dstPtr = dstStart;
        let compress: compressFunc_t;

        if (*cctxPtr).tmpInSize == 0 {
            return 0;
        }
        if (*cctxPtr).cStage != 1 {
            return LZ4F_returnErrorCode(LZ4F_ERROR_compressionState_uninitialized);
        }
        if dstCapacity < ((*cctxPtr).tmpInSize + BHSize + BFSize) {
            return LZ4F_returnErrorCode(LZ4F_ERROR_dstMaxSize_tooSmall);
        }

        compress = LZ4F_selectCompression(
            (*cctxPtr).prefs.frameInfo.blockMode,
            (*cctxPtr).prefs.compressionLevel,
            (*cctxPtr).blockCompressMode,
        );

        dstPtr = madd(
            dstPtr,
            LZ4F_makeBlock(
                dstPtr as *mut c_void,
                (*cctxPtr).tmpIn as *const c_void,
                (*cctxPtr).tmpInSize,
                compress,
                (*cctxPtr).lz4CtxPtr,
                (*cctxPtr).prefs.compressionLevel,
                (*cctxPtr).cdict,
                (*cctxPtr).prefs.frameInfo.blockChecksumFlag,
            ),
        );

        if (*cctxPtr).prefs.frameInfo.blockMode == LZ4F_blockLinked {
            (*cctxPtr).tmpIn = madd((*cctxPtr).tmpIn, (*cctxPtr).tmpInSize);
        }
        (*cctxPtr).tmpInSize = 0;

        /* keep tmpIn within limits */
        if madd((*cctxPtr).tmpIn, (*cctxPtr).maxBlockSize)
            > madd((*cctxPtr).tmpBuff, (*cctxPtr).maxBufferSize)
        {
            let realDictSize = LZ4F_localSaveDict(cctxPtr);
            (*cctxPtr).tmpIn = madd((*cctxPtr).tmpBuff, realDictSize as usize);
        }

        pdiff(dstPtr, dstStart) as usize
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_compressEnd(
    cctxPtr: *mut LZ4F_cctx,
    dstBuffer: *mut c_void,
    mut dstCapacity: usize,
    compressOptionsPtr: *const LZ4F_compressOptions_t,
) -> usize {
    unsafe {
        let dstStart = dstBuffer as *mut u8;
        let mut dstPtr = dstStart;

        let flushSize = LZ4F_flush(cctxPtr, dstBuffer, dstCapacity, compressOptionsPtr);
        if LZ4F_isError(flushSize) != 0 {
            return flushSize;
        }
        dstPtr = madd(dstPtr, flushSize);

        dstCapacity -= flushSize;

        if dstCapacity < 4 {
            return LZ4F_returnErrorCode(LZ4F_ERROR_dstMaxSize_tooSmall);
        }
        LZ4F_writeLE32(dstPtr as *mut c_void, 0);
        dstPtr = madd(dstPtr, 4);

        if (*cctxPtr).prefs.frameInfo.contentChecksumFlag == LZ4F_contentChecksumEnabled {
            let xxh = LZ4_XXH32_digest(&(*cctxPtr).xxh);
            if dstCapacity < 8 {
                return LZ4F_returnErrorCode(LZ4F_ERROR_dstMaxSize_tooSmall);
            }
            LZ4F_writeLE32(dstPtr as *mut c_void, xxh);
            dstPtr = madd(dstPtr, 4);
        }

        (*cctxPtr).cStage = 0;

        if (*cctxPtr).prefs.frameInfo.contentSize != 0
            && (*cctxPtr).prefs.frameInfo.contentSize != (*cctxPtr).totalInSize
        {
            return LZ4F_returnErrorCode(LZ4F_ERROR_frameSize_wrong);
        }

        pdiff(dstPtr, dstStart) as usize
    }
}

/* ===== Frame Decompression ===== */

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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_createDecompressionContext_advanced(
    customMem: LZ4F_CustomMem,
    version: c_uint,
) -> *mut LZ4F_dctx {
    unsafe {
        let dctx = LZ4F_calloc(core::mem::size_of::<LZ4F_dctx>(), customMem) as *mut LZ4F_dctx;
        if dctx.is_null() {
            return core::ptr::null_mut();
        }
        (*dctx).cmem = customMem;
        (*dctx).version = version;
        dctx
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_createDecompressionContext(
    LZ4F_decompressionContextPtr: *mut *mut LZ4F_dctx,
    versionNumber: c_uint,
) -> LZ4F_errorCode_t {
    unsafe {
        if LZ4F_decompressionContextPtr.is_null() {
            return LZ4F_returnErrorCode(LZ4F_ERROR_parameter_null);
        }
        *LZ4F_decompressionContextPtr =
            LZ4F_createDecompressionContext_advanced(LZ4F_defaultCMem, versionNumber);
        if (*LZ4F_decompressionContextPtr).is_null() {
            return LZ4F_returnErrorCode(LZ4F_ERROR_allocation_failed);
        }
        LZ4F_OK_NoError as LZ4F_errorCode_t
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_freeDecompressionContext(dctx: *mut LZ4F_dctx) -> LZ4F_errorCode_t {
    unsafe {
        let mut result: LZ4F_errorCode_t = LZ4F_OK_NoError as LZ4F_errorCode_t;
        if !dctx.is_null() {
            result = (*dctx).dStage as LZ4F_errorCode_t;
            LZ4F_free((*dctx).tmpIn as *mut c_void, (*dctx).cmem);
            LZ4F_free((*dctx).tmpOutBuffer as *mut c_void, (*dctx).cmem);
            LZ4F_free(dctx as *mut c_void, (*dctx).cmem);
        }
        result
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_resetDecompressionContext(dctx: *mut LZ4F_dctx) {
    unsafe {
        (*dctx).dStage = dstage_getFrameHeader;
        (*dctx).dict = core::ptr::null();
        (*dctx).dictSize = 0;
        (*dctx).skipChecksum = 0;
        (*dctx).frameRemainingSize = 0;
    }
}

unsafe fn LZ4F_decodeHeader(dctx: *mut LZ4F_dctx, src: *const c_void, srcSize: usize) -> usize {
    unsafe {
        let blockMode: c_uint;
        let blockChecksumFlag: c_uint;
        let contentSizeFlag: c_uint;
        let contentChecksumFlag: c_uint;
        let dictIDFlag: c_uint;
        let blockSizeID: c_uint;
        let frameHeaderSize: usize;
        let srcPtr = src as *const u8;

        if srcSize < minFHSize {
            return LZ4F_returnErrorCode(LZ4F_ERROR_frameHeader_incomplete);
        }
        MEM_INIT(
            &mut (*dctx).frameInfo as *mut LZ4F_frameInfo_t as *mut u8,
            0,
            core::mem::size_of::<LZ4F_frameInfo_t>(),
        );

        /* special case : skippable frames */
        if (LZ4F_readLE32(srcPtr as *const c_void) & 0xFFFFFFF0u32) == LZ4F_MAGIC_SKIPPABLE_START {
            (*dctx).frameInfo.frameType = LZ4F_skippableFrame;
            if src == (*dctx).header.as_ptr() as *const c_void {
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
            return LZ4F_returnErrorCode(LZ4F_ERROR_frameType_unknown);
        }
        (*dctx).frameInfo.frameType = LZ4F_frame;

        /* Flags */
        {
            let FLG = *srcPtr.wrapping_add(4) as u32;
            let version = (FLG >> 6) & _2BITS;
            blockChecksumFlag = (FLG >> 4) & _1BIT;
            blockMode = (FLG >> 5) & _1BIT;
            contentSizeFlag = (FLG >> 3) & _1BIT;
            contentChecksumFlag = (FLG >> 2) & _1BIT;
            dictIDFlag = FLG & _1BIT;
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
            if srcPtr != (*dctx).header.as_ptr() {
                LZ4_memcpy((*dctx).header.as_mut_ptr(), srcPtr, srcSize);
            }
            (*dctx).tmpInSize = srcSize;
            (*dctx).tmpInTarget = frameHeaderSize;
            (*dctx).dStage = dstage_storeFrameHeader;
            return srcSize;
        }

        {
            let BD = *srcPtr.wrapping_add(5) as u32;
            blockSizeID = (BD >> 4) & _3BITS;
            if ((BD >> 7) & _1BIT) != 0 {
                return LZ4F_returnErrorCode(LZ4F_ERROR_reservedFlag_set);
            }
            if blockSizeID < 4 {
                return LZ4F_returnErrorCode(LZ4F_ERROR_maxBlockSize_invalid);
            }
            if (BD & _4BITS) != 0 {
                return LZ4F_returnErrorCode(LZ4F_ERROR_reservedFlag_set);
            }
        }

        /* check header */
        {
            let HC = LZ4F_headerChecksum(
                srcPtr.wrapping_add(4) as *const c_void,
                frameHeaderSize - 5,
            );
            if HC != *srcPtr.wrapping_add(frameHeaderSize - 1) {
                return LZ4F_returnErrorCode(LZ4F_ERROR_headerChecksum_invalid);
            }
        }

        /* save */
        (*dctx).frameInfo.blockMode = blockMode;
        (*dctx).frameInfo.blockChecksumFlag = blockChecksumFlag;
        (*dctx).frameInfo.contentChecksumFlag = contentChecksumFlag;
        (*dctx).frameInfo.blockSizeID = blockSizeID;
        (*dctx).maxBlockSize = LZ4F_getBlockSize(blockSizeID);
        if contentSizeFlag != 0 {
            (*dctx).frameInfo.contentSize =
                LZ4F_readLE64(srcPtr.wrapping_add(6) as *const c_void);
            (*dctx).frameRemainingSize = (*dctx).frameInfo.contentSize;
        }
        if dictIDFlag != 0 {
            (*dctx).frameInfo.dictID =
                LZ4F_readLE32(srcPtr.wrapping_add(frameHeaderSize - 5) as *const c_void);
        }

        (*dctx).dStage = dstage_init;

        frameHeaderSize
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_headerSize(src: *const c_void, srcSize: usize) -> usize {
    unsafe {
        if src.is_null() {
            return LZ4F_returnErrorCode(LZ4F_ERROR_srcPtr_wrong);
        }

        if srcSize < LZ4F_MIN_SIZE_TO_KNOW_HEADER_LENGTH {
            return LZ4F_returnErrorCode(LZ4F_ERROR_frameHeader_incomplete);
        }

        if (LZ4F_readLE32(src) & 0xFFFFFFF0u32) == LZ4F_MAGIC_SKIPPABLE_START {
            return 8;
        }

        if LZ4F_readLE32(src) != LZ4F_MAGICNUMBER {
            return LZ4F_returnErrorCode(LZ4F_ERROR_frameType_unknown);
        }

        {
            let FLG = *(src as *const u8).wrapping_add(4);
            let contentSizeFlag = ((FLG as u32) >> 3) & _1BIT;
            let dictIDFlag = (FLG as u32) & _1BIT;
            minFHSize
                + (if contentSizeFlag != 0 { 8 } else { 0 })
                + (if dictIDFlag != 0 { 4 } else { 0 })
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_getFrameInfo(
    dctx: *mut LZ4F_dctx,
    frameInfoPtr: *mut LZ4F_frameInfo_t,
    srcBuffer: *const c_void,
    srcSizePtr: *mut usize,
) -> LZ4F_errorCode_t {
    unsafe {
        if (*dctx).dStage > dstage_storeFrameHeader {
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
        } else if (*dctx).dStage == dstage_storeFrameHeader {
            *srcSizePtr = 0;
            return LZ4F_returnErrorCode(LZ4F_ERROR_frameDecoding_alreadyStarted);
        } else {
            let hSize = LZ4F_headerSize(srcBuffer, *srcSizePtr);
            if LZ4F_isError(hSize) != 0 {
                *srcSizePtr = 0;
                return hSize;
            }
            if *srcSizePtr < hSize {
                *srcSizePtr = 0;
                return LZ4F_returnErrorCode(LZ4F_ERROR_frameHeader_incomplete);
            }

            let mut decodeResult = LZ4F_decodeHeader(dctx, srcBuffer, hSize);
            if LZ4F_isError(decodeResult) != 0 {
                *srcSizePtr = 0;
            } else {
                *srcSizePtr = decodeResult;
                decodeResult = BHSize;
            }
            *frameInfoPtr = (*dctx).frameInfo;
            decodeResult
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
    unsafe {
        if (*dctx).dictSize == 0 {
            (*dctx).dict = dstPtr;
        }

        if cadd((*dctx).dict, (*dctx).dictSize) == dstPtr {
            (*dctx).dictSize += dstSize;
            return;
        }

        if (pdiff(dstPtr, dstBufferStart) as usize) + dstSize >= 64 * KB {
            (*dctx).dict = dstBufferStart;
            (*dctx).dictSize = (pdiff(dstPtr, dstBufferStart) as usize) + dstSize;
            return;
        }

        if withinTmp != 0 && ((*dctx).dict == (*dctx).tmpOutBuffer) {
            (*dctx).dictSize += dstSize;
            return;
        }

        if withinTmp != 0 {
            let preserveSize = pdiff((*dctx).tmpOut, (*dctx).tmpOutBuffer) as usize;
            let mut copySize = 64 * KB - (*dctx).tmpOutSize;
            let oldDictEnd = csub(cadd((*dctx).dict, (*dctx).dictSize), (*dctx).tmpOutStart);
            if (*dctx).tmpOutSize > 64 * KB {
                copySize = 0;
            }
            if copySize > preserveSize {
                copySize = preserveSize;
            }

            LZ4_memcpy(
                madd(msub((*dctx).tmpOutBuffer, copySize), preserveSize),
                csub(oldDictEnd, copySize),
                copySize,
            );

            (*dctx).dict = (*dctx).tmpOutBuffer;
            (*dctx).dictSize = preserveSize + (*dctx).tmpOutStart + dstSize;
            return;
        }

        if (*dctx).dict == (*dctx).tmpOutBuffer {
            if (*dctx).dictSize + dstSize > (*dctx).maxBufferSize {
                let preserveSize = 64 * KB - dstSize;
                LZ4_memcpy(
                    (*dctx).tmpOutBuffer,
                    csub(cadd((*dctx).dict, (*dctx).dictSize), preserveSize),
                    preserveSize,
                );
                (*dctx).dictSize = preserveSize;
            }
            LZ4_memcpy(
                madd((*dctx).tmpOutBuffer, (*dctx).dictSize),
                dstPtr,
                dstSize,
            );
            (*dctx).dictSize += dstSize;
            return;
        }

        /* join dict & dest into tmp */
        {
            let mut preserveSize = 64 * KB - dstSize;
            if preserveSize > (*dctx).dictSize {
                preserveSize = (*dctx).dictSize;
            }
            LZ4_memcpy(
                (*dctx).tmpOutBuffer,
                csub(cadd((*dctx).dict, (*dctx).dictSize), preserveSize),
                preserveSize,
            );
            LZ4_memcpy(
                madd((*dctx).tmpOutBuffer, preserveSize),
                dstPtr,
                dstSize,
            );
            (*dctx).dict = (*dctx).tmpOutBuffer;
            (*dctx).dictSize = preserveSize + dstSize;
        }
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
    unsafe {
        let optionsNull: LZ4F_decompressOptions_t;
        let srcStart = srcBuffer as *const u8;
        let srcEnd = cadd(srcStart, *srcSizePtr);
        let mut srcPtr = srcStart;
        let dstStart = dstBuffer as *mut u8;
        let dstEnd: *mut u8 = if !dstStart.is_null() {
            madd(dstStart, *dstSizePtr)
        } else {
            core::ptr::null_mut()
        };
        let mut dstPtr = dstStart;
        let mut selectedIn: *const u8 = core::ptr::null();
        let mut doAnotherStage: c_uint = 1;
        let mut nextSrcSizeHint: usize = 1;

        optionsNull = core::mem::zeroed();
        if decompressOptionsPtr.is_null() {
            decompressOptionsPtr = &optionsNull;
        }
        *srcSizePtr = 0;
        *dstSizePtr = 0;
        (*dctx).skipChecksum |= ((*decompressOptionsPtr).skipChecksums != 0) as c_int;

        while doAnotherStage != 0 {
            let st = (*dctx).dStage;

            'sw: {
                /* ---- getFrameHeader / storeFrameHeader ---- */
                if st == dstage_getFrameHeader || st == dstage_storeFrameHeader {
                    if st == dstage_getFrameHeader {
                        if (pdiff(srcEnd, srcPtr) as usize) >= maxFHSize {
                            let hSize =
                                LZ4F_decodeHeader(dctx, srcPtr as *const c_void, pdiff(srcEnd, srcPtr) as usize);
                            if LZ4F_isError(hSize) != 0 {
                                return hSize;
                            }
                            srcPtr = cadd(srcPtr, hSize);
                            break 'sw;
                        }
                        (*dctx).tmpInSize = 0;
                        if pdiff(srcEnd, srcPtr) == 0 {
                            return minFHSize;
                        }
                        (*dctx).tmpInTarget = minFHSize;
                        (*dctx).dStage = dstage_storeFrameHeader;
                    }
                    /* dstage_storeFrameHeader */
                    {
                        let sizeToCopy = MIN(
                            (*dctx).tmpInTarget - (*dctx).tmpInSize,
                            pdiff(srcEnd, srcPtr) as usize,
                        );
                        LZ4_memcpy(
                            (*dctx).header.as_mut_ptr().wrapping_add((*dctx).tmpInSize),
                            srcPtr,
                            sizeToCopy,
                        );
                        (*dctx).tmpInSize += sizeToCopy;
                        srcPtr = cadd(srcPtr, sizeToCopy);
                    }
                    if (*dctx).tmpInSize < (*dctx).tmpInTarget {
                        nextSrcSizeHint = ((*dctx).tmpInTarget - (*dctx).tmpInSize) + BHSize;
                        doAnotherStage = 0;
                        break 'sw;
                    }
                    let r = LZ4F_decodeHeader(
                        dctx,
                        (*dctx).header.as_ptr() as *const c_void,
                        (*dctx).tmpInTarget,
                    );
                    if LZ4F_isError(r) != 0 {
                        return r;
                    }
                    break 'sw;
                }

                /* ---- init / getBlockHeader / storeBlockHeader ---- */
                if st == dstage_init || st == dstage_getBlockHeader || st == dstage_storeBlockHeader
                {
                    let mut cur = st;
                    if cur == dstage_init {
                        if (*dctx).frameInfo.contentChecksumFlag != 0 {
                            LZ4_XXH32_reset(&mut (*dctx).xxh, 0);
                        }
                        {
                            let bufferNeeded = (*dctx).maxBlockSize
                                + (if (*dctx).frameInfo.blockMode == LZ4F_blockLinked {
                                    128 * KB
                                } else {
                                    0
                                });
                            if bufferNeeded > (*dctx).maxBufferSize {
                                (*dctx).maxBufferSize = 0;
                                LZ4F_free((*dctx).tmpIn as *mut c_void, (*dctx).cmem);
                                (*dctx).tmpIn =
                                    LZ4F_malloc((*dctx).maxBlockSize + BFSize, (*dctx).cmem)
                                        as *mut u8;
                                if (*dctx).tmpIn.is_null() {
                                    return LZ4F_returnErrorCode(LZ4F_ERROR_allocation_failed);
                                }
                                LZ4F_free((*dctx).tmpOutBuffer as *mut c_void, (*dctx).cmem);
                                (*dctx).tmpOutBuffer =
                                    LZ4F_malloc(bufferNeeded, (*dctx).cmem) as *mut u8;
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
                        cur = dstage_getBlockHeader;
                    }

                    let mut run_store = false;
                    if cur == dstage_getBlockHeader {
                        if (pdiff(srcEnd, srcPtr) as usize) >= BHSize {
                            selectedIn = srcPtr;
                            srcPtr = cadd(srcPtr, BHSize);
                        } else {
                            (*dctx).tmpInSize = 0;
                            (*dctx).dStage = dstage_storeBlockHeader;
                        }
                        run_store = (*dctx).dStage == dstage_storeBlockHeader;
                    } else {
                        run_store = true;
                    }

                    if run_store {
                        let remainingInput = pdiff(srcEnd, srcPtr) as usize;
                        let wantedData = BHSize - (*dctx).tmpInSize;
                        let sizeToCopy = MIN(wantedData, remainingInput);
                        LZ4_memcpy(
                            madd((*dctx).tmpIn, (*dctx).tmpInSize),
                            srcPtr,
                            sizeToCopy,
                        );
                        srcPtr = cadd(srcPtr, sizeToCopy);
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
                        let blockHeader = LZ4F_readLE32(selectedIn as *const c_void);
                        let nextCBlockSize = (blockHeader & 0x7FFFFFFFu32) as usize;
                        let crcSize = (*dctx).frameInfo.blockChecksumFlag as usize * BFSize;
                        if blockHeader == 0 {
                            (*dctx).dStage = dstage_getSuffix;
                            break 'sw;
                        }
                        if nextCBlockSize > (*dctx).maxBlockSize {
                            return LZ4F_returnErrorCode(LZ4F_ERROR_maxBlockSize_invalid);
                        }
                        if (blockHeader & LZ4F_BLOCKUNCOMPRESSED_FLAG) != 0 {
                            (*dctx).tmpInTarget = nextCBlockSize;
                            if (*dctx).frameInfo.blockChecksumFlag != 0 {
                                LZ4_XXH32_reset(&mut (*dctx).blockChecksum, 0);
                            }
                            (*dctx).dStage = dstage_copyDirect;
                            break 'sw;
                        }
                        (*dctx).tmpInTarget = nextCBlockSize + crcSize;
                        (*dctx).dStage = dstage_getCBlock;
                        if dstPtr == dstEnd || srcPtr == srcEnd {
                            nextSrcSizeHint = BHSize + nextCBlockSize + crcSize;
                            doAnotherStage = 0;
                        }
                        break 'sw;
                    }
                }

                /* ---- copyDirect ---- */
                if st == dstage_copyDirect {
                    {
                        let sizeToCopy: usize;
                        if dstPtr.is_null() {
                            sizeToCopy = 0;
                        } else {
                            let minBuffSize = MIN(
                                pdiff(srcEnd, srcPtr) as usize,
                                pdiff(dstEnd, dstPtr) as usize,
                            );
                            sizeToCopy = MIN((*dctx).tmpInTarget, minBuffSize);
                            LZ4_memcpy(dstPtr, srcPtr, sizeToCopy);
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

                            if (*dctx).frameInfo.blockMode == LZ4F_blockLinked {
                                LZ4F_updateDict(dctx, dstPtr, sizeToCopy, dstStart, 0);
                            }
                            srcPtr = cadd(srcPtr, sizeToCopy);
                            dstPtr = madd(dstPtr, sizeToCopy);
                        }
                        if sizeToCopy == (*dctx).tmpInTarget {
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

                /* ---- getBlockChecksum ---- */
                if st == dstage_getBlockChecksum {
                    {
                        let crcSrc: *const c_void;
                        if (pdiff(srcEnd, srcPtr) >= 4) && ((*dctx).tmpInSize == 0) {
                            crcSrc = srcPtr as *const c_void;
                            srcPtr = cadd(srcPtr, 4);
                        } else {
                            let stillToCopy = 4 - (*dctx).tmpInSize;
                            let sizeToCopy = MIN(stillToCopy, pdiff(srcEnd, srcPtr) as usize);
                            LZ4_memcpy(
                                (*dctx).header.as_mut_ptr().wrapping_add((*dctx).tmpInSize),
                                srcPtr,
                                sizeToCopy,
                            );
                            (*dctx).tmpInSize += sizeToCopy;
                            srcPtr = cadd(srcPtr, sizeToCopy);
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
                                return LZ4F_returnErrorCode(LZ4F_ERROR_blockChecksum_invalid);
                            }
                        }
                    }
                    (*dctx).dStage = dstage_getBlockHeader;
                    break 'sw;
                }

                /* ---- getCBlock / storeCBlock / flushOut ---- */
                if st == dstage_getCBlock || st == dstage_storeCBlock || st == dstage_flushOut {
                    let mut goto_flushOut = st == dstage_flushOut;

                    if !goto_flushOut {
                        if st == dstage_getCBlock {
                            if (pdiff(srcEnd, srcPtr) as usize) < (*dctx).tmpInTarget {
                                (*dctx).tmpInSize = 0;
                                (*dctx).dStage = dstage_storeCBlock;
                                break 'sw;
                            }
                            selectedIn = srcPtr;
                            srcPtr = cadd(srcPtr, (*dctx).tmpInTarget);
                        } else {
                            /* dstage_storeCBlock */
                            let wantedData = (*dctx).tmpInTarget - (*dctx).tmpInSize;
                            let inputLeft = pdiff(srcEnd, srcPtr) as usize;
                            let sizeToCopy = MIN(wantedData, inputLeft);
                            LZ4_memcpy(
                                madd((*dctx).tmpIn, (*dctx).tmpInSize),
                                srcPtr,
                                sizeToCopy,
                            );
                            (*dctx).tmpInSize += sizeToCopy;
                            srcPtr = cadd(srcPtr, sizeToCopy);
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

                        /* control block checksum if it exists */
                        if (*dctx).frameInfo.blockChecksumFlag != 0 {
                            (*dctx).tmpInTarget -= 4;
                            let readBlockCrc = LZ4F_readLE32(
                                cadd(selectedIn, (*dctx).tmpInTarget) as *const c_void,
                            );
                            let calcBlockCrc =
                                LZ4_XXH32(selectedIn as *const c_void, (*dctx).tmpInTarget, 0);
                            if readBlockCrc != calcBlockCrc {
                                return LZ4F_returnErrorCode(LZ4F_ERROR_blockChecksum_invalid);
                            }
                        }

                        /* decode directly into destination buffer if there is enough room */
                        if ((pdiff(dstEnd, dstPtr) as usize) >= (*dctx).maxBlockSize)
                            && !(!(*dctx).dict.is_null()
                                && cadd((*dctx).dict, (*dctx).dictSize) == (*dctx).tmpOut)
                        {
                            let mut dict = (*dctx).dict as *const c_char;
                            let mut dictSize = (*dctx).dictSize;
                            let decodedSize: c_int;
                            if !dict.is_null() && dictSize > (1usize << 30) {
                                dict = cadd(dict as *const u8, dictSize - 64 * KB) as *const c_char;
                                dictSize = 64 * KB;
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

                            if (*dctx).frameInfo.blockMode == LZ4F_blockLinked {
                                LZ4F_updateDict(dctx, dstPtr, decodedSize as usize, dstStart, 0);
                            }

                            dstPtr = madd(dstPtr, decodedSize as usize);
                            (*dctx).dStage = dstage_getBlockHeader;
                            break 'sw;
                        }

                        /* not enough place into dst : decode into tmpOut */
                        if (*dctx).frameInfo.blockMode == LZ4F_blockLinked {
                            if (*dctx).dict == (*dctx).tmpOutBuffer {
                                if (*dctx).dictSize > 128 * KB {
                                    LZ4_memcpy(
                                        (*dctx).tmpOutBuffer,
                                        csub(
                                            cadd((*dctx).dict, (*dctx).dictSize),
                                            64 * KB,
                                        ),
                                        64 * KB,
                                    );
                                    (*dctx).dictSize = 64 * KB;
                                }
                                (*dctx).tmpOut = madd((*dctx).tmpOutBuffer, (*dctx).dictSize);
                            } else {
                                let reservedDictSpace = MIN((*dctx).dictSize, 64 * KB);
                                (*dctx).tmpOut = madd((*dctx).tmpOutBuffer, reservedDictSpace);
                            }
                        }

                        /* Decode block into tmpOut */
                        {
                            let mut dict = (*dctx).dict as *const c_char;
                            let mut dictSize = (*dctx).dictSize;
                            let decodedSize: c_int;
                            if !dict.is_null() && dictSize > (1usize << 30) {
                                dict = cadd(dict as *const u8, dictSize - 64 * KB) as *const c_char;
                                dictSize = 64 * KB;
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
                            if (*dctx).frameInfo.contentChecksumFlag != 0
                                && (*dctx).skipChecksum == 0
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
                        goto_flushOut = true;
                    }

                    /* ---- flushOut ---- */
                    if goto_flushOut {
                        if !dstPtr.is_null() {
                            let sizeToCopy = MIN(
                                (*dctx).tmpOutSize - (*dctx).tmpOutStart,
                                pdiff(dstEnd, dstPtr) as usize,
                            );
                            LZ4_memcpy(
                                dstPtr,
                                cadd((*dctx).tmpOut, (*dctx).tmpOutStart),
                                sizeToCopy,
                            );

                            if (*dctx).frameInfo.blockMode == LZ4F_blockLinked {
                                LZ4F_updateDict(dctx, dstPtr, sizeToCopy, dstStart, 1);
                            }

                            (*dctx).tmpOutStart += sizeToCopy;
                            dstPtr = madd(dstPtr, sizeToCopy);
                        }
                        if (*dctx).tmpOutStart == (*dctx).tmpOutSize {
                            (*dctx).dStage = dstage_getBlockHeader;
                            break 'sw;
                        }
                        doAnotherStage = 0;
                        nextSrcSizeHint = BHSize;
                        break 'sw;
                    }
                }

                /* ---- getSuffix / storeSuffix ---- */
                if st == dstage_getSuffix || st == dstage_storeSuffix {
                    let mut run_store = st == dstage_storeSuffix;
                    if st == dstage_getSuffix {
                        if (*dctx).frameRemainingSize != 0 {
                            return LZ4F_returnErrorCode(LZ4F_ERROR_frameSize_wrong);
                        }
                        if (*dctx).frameInfo.contentChecksumFlag == 0 {
                            nextSrcSizeHint = 0;
                            LZ4F_resetDecompressionContext(dctx);
                            doAnotherStage = 0;
                            break 'sw;
                        }
                        if pdiff(srcEnd, srcPtr) < 4 {
                            (*dctx).tmpInSize = 0;
                            (*dctx).dStage = dstage_storeSuffix;
                        } else {
                            selectedIn = srcPtr;
                            srcPtr = cadd(srcPtr, 4);
                        }
                        run_store = (*dctx).dStage == dstage_storeSuffix;
                    }

                    if run_store {
                        let remainingInput = pdiff(srcEnd, srcPtr) as usize;
                        let wantedData = 4 - (*dctx).tmpInSize;
                        let sizeToCopy = MIN(wantedData, remainingInput);
                        LZ4_memcpy(
                            madd((*dctx).tmpIn, (*dctx).tmpInSize),
                            srcPtr,
                            sizeToCopy,
                        );
                        srcPtr = cadd(srcPtr, sizeToCopy);
                        (*dctx).tmpInSize += sizeToCopy;
                        if (*dctx).tmpInSize < 4 {
                            nextSrcSizeHint = 4 - (*dctx).tmpInSize;
                            doAnotherStage = 0;
                            break 'sw;
                        }
                        selectedIn = (*dctx).tmpIn;
                    }

                    if (*dctx).skipChecksum == 0 {
                        let readCRC = LZ4F_readLE32(selectedIn as *const c_void);
                        let resultCRC = LZ4_XXH32_digest(&(*dctx).xxh);
                        if readCRC != resultCRC {
                            return LZ4F_returnErrorCode(LZ4F_ERROR_contentChecksum_invalid);
                        }
                    }
                    nextSrcSizeHint = 0;
                    LZ4F_resetDecompressionContext(dctx);
                    doAnotherStage = 0;
                    break 'sw;
                }

                /* ---- getSFrameSize / storeSFrameSize ---- */
                if st == dstage_getSFrameSize || st == dstage_storeSFrameSize {
                    let mut run_store = st == dstage_storeSFrameSize;
                    if st == dstage_getSFrameSize {
                        if pdiff(srcEnd, srcPtr) >= 4 {
                            selectedIn = srcPtr;
                            srcPtr = cadd(srcPtr, 4);
                        } else {
                            (*dctx).tmpInSize = 4;
                            (*dctx).tmpInTarget = 8;
                            (*dctx).dStage = dstage_storeSFrameSize;
                        }
                        run_store = (*dctx).dStage == dstage_storeSFrameSize;
                    }

                    if run_store {
                        let sizeToCopy = MIN(
                            (*dctx).tmpInTarget - (*dctx).tmpInSize,
                            pdiff(srcEnd, srcPtr) as usize,
                        );
                        LZ4_memcpy(
                            (*dctx).header.as_mut_ptr().wrapping_add((*dctx).tmpInSize),
                            srcPtr,
                            sizeToCopy,
                        );
                        srcPtr = cadd(srcPtr, sizeToCopy);
                        (*dctx).tmpInSize += sizeToCopy;
                        if (*dctx).tmpInSize < (*dctx).tmpInTarget {
                            nextSrcSizeHint = (*dctx).tmpInTarget - (*dctx).tmpInSize;
                            doAnotherStage = 0;
                            break 'sw;
                        }
                        selectedIn = (*dctx).header.as_ptr().wrapping_add(4);
                    }

                    {
                        let SFrameSize = LZ4F_readLE32(selectedIn as *const c_void) as usize;
                        (*dctx).frameInfo.contentSize = SFrameSize as u64;
                        (*dctx).tmpInTarget = SFrameSize;
                        (*dctx).dStage = dstage_skipSkippable;
                        break 'sw;
                    }
                }

                /* ---- skipSkippable ---- */
                if st == dstage_skipSkippable {
                    let skipSize = MIN((*dctx).tmpInTarget, pdiff(srcEnd, srcPtr) as usize);
                    srcPtr = cadd(srcPtr, skipSize);
                    (*dctx).tmpInTarget -= skipSize;
                    doAnotherStage = 0;
                    nextSrcSizeHint = (*dctx).tmpInTarget;
                    if nextSrcSizeHint != 0 {
                        break 'sw;
                    }
                    LZ4F_resetDecompressionContext(dctx);
                    break 'sw;
                }
            }
        }

        /* preserve history within tmpOut whenever necessary */
        if ((*dctx).frameInfo.blockMode == LZ4F_blockLinked)
            && ((*dctx).dict != (*dctx).tmpOutBuffer)
            && (!(*dctx).dict.is_null())
            && ((*decompressOptionsPtr).stableDst == 0)
            && (((*dctx).dStage as u32).wrapping_sub(2) < (dstage_getSuffix as u32) - 2)
        {
            if (*dctx).dStage == dstage_flushOut {
                let preserveSize = pdiff((*dctx).tmpOut, (*dctx).tmpOutBuffer) as usize;
                let mut copySize = 64 * KB - (*dctx).tmpOutSize;
                let oldDictEnd = csub(cadd((*dctx).dict, (*dctx).dictSize), (*dctx).tmpOutStart);
                if (*dctx).tmpOutSize > 64 * KB {
                    copySize = 0;
                }
                if copySize > preserveSize {
                    copySize = preserveSize;
                }

                LZ4_memcpy(
                    madd(msub((*dctx).tmpOutBuffer, copySize), preserveSize),
                    csub(oldDictEnd, copySize),
                    copySize,
                );

                (*dctx).dict = (*dctx).tmpOutBuffer;
                (*dctx).dictSize = preserveSize + (*dctx).tmpOutStart;
            } else {
                let oldDictEnd = cadd((*dctx).dict, (*dctx).dictSize);
                let newDictSize = MIN((*dctx).dictSize, 64 * KB);

                LZ4_memcpy((*dctx).tmpOutBuffer, csub(oldDictEnd, newDictSize), newDictSize);

                (*dctx).dict = (*dctx).tmpOutBuffer;
                (*dctx).dictSize = newDictSize;
                (*dctx).tmpOut = madd((*dctx).tmpOutBuffer, newDictSize);
            }
        }

        *srcSizePtr = pdiff(srcPtr, srcStart) as usize;
        *dstSizePtr = pdiff(dstPtr, dstStart) as usize;
        nextSrcSizeHint
    }
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
    unsafe {
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
}
