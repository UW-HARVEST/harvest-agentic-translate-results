//! Translation of c_src/src/lz4frame.c (LZ4 frame format, v1.10.0)
//! Built with LZ4F_HEAPMODE == 0 (matching the CMake definition).

use core::ffi::{c_char, c_int, c_uint, c_void};
use core::ptr;

use crate::lz4::{
    self, memcpy_n, memset_n, pdiff, LZ4_attach_dictionary, LZ4_compress_fast_continue,
    LZ4_compress_fast_extState_fastReset, LZ4_decompress_safe_usingDict, LZ4_initStream,
    LZ4_loadDict, LZ4_loadDictSlow, LZ4_resetStream_fast, LZ4_saveDict, LZ4_sizeofState,
    LZ4_stream_t,
};
use crate::lz4hc::{
    LZ4HC_CLEVEL_DEFAULT, LZ4HC_CLEVEL_MAX, LZ4HC_CLEVEL_MIN, LZ4_attach_HC_dictionary,
    LZ4_compress_HC_continue, LZ4_compress_HC_extStateHC_fastReset, LZ4_favorDecompressionSpeed,
    LZ4_initStreamHC, LZ4_loadDictHC, LZ4_resetStreamHC_fast, LZ4_saveDictHC,
    LZ4_setCompressionLevel, LZ4_sizeofStateHC, LZ4_streamHC_t,
};
use crate::xxhash::{
    XXH32_state_t, LZ4_XXH32 as XXH32, LZ4_XXH32_digest as XXH32_digest,
    LZ4_XXH32_reset as XXH32_reset, LZ4_XXH32_update as XXH32_update,
};

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn calloc(n: usize, size: usize) -> *mut c_void;
    fn free(p: *mut c_void);
}

/* ---------------- public types ---------------- */

pub type LZ4F_errorCode_t = usize;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct LZ4F_CustomMem {
    pub customAlloc: Option<unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void>,
    pub customCalloc: Option<unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void>,
    pub customFree: Option<unsafe extern "C" fn(*mut c_void, *mut c_void)>,
    pub opaqueState: *mut c_void,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct LZ4F_frameInfo_t {
    pub blockSizeID: u32,
    pub blockMode: u32,
    pub contentChecksumFlag: u32,
    pub frameType: u32,
    pub contentSize: u64,
    pub dictID: c_uint,
    pub blockChecksumFlag: u32,
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

/* ---------------- constants ---------------- */

pub const LZ4F_VERSION: c_uint = 100;

pub const LZ4F_HEADER_SIZE_MIN: usize = 7;
pub const LZ4F_HEADER_SIZE_MAX: usize = 19;
pub const MIN_FH_SIZE: usize = LZ4F_HEADER_SIZE_MIN;
pub const MAX_FH_SIZE: usize = LZ4F_HEADER_SIZE_MAX;
pub const BH_SIZE: usize = 4;
pub const BF_SIZE: usize = 4;

pub const LZ4F_MAGICNUMBER: u32 = 0x184D2204;
pub const LZ4F_MAGIC_SKIPPABLE_START: u32 = 0x184D2A50;
pub const LZ4F_MIN_SIZE_TO_KNOW_HEADER_LENGTH: usize = 5;

pub const LZ4F_BLOCKUNCOMPRESSED_FLAG: u32 = 0x8000_0000;

/* blockSizeID */
const LZ4F_DEFAULT_BSID: u32 = 0;
const LZ4F_MAX64KB: u32 = 4;
const LZ4F_MAX256KB: u32 = 5;
const LZ4F_MAX1MB: u32 = 6;
const LZ4F_MAX4MB: u32 = 7;
const LZ4F_BLOCKSIZEID_DEFAULT: u32 = LZ4F_MAX64KB;

/* blockMode */
const LZ4F_BLOCK_LINKED: u32 = 0;
const LZ4F_BLOCK_INDEPENDENT: u32 = 1;

/* contentChecksum */
const LZ4F_NO_CONTENT_CHECKSUM: u32 = 0;
const LZ4F_CONTENT_CHECKSUM_ENABLED: u32 = 1;

/* frameType */
const LZ4F_FRAME: u32 = 0;
const LZ4F_SKIPPABLE_FRAME: u32 = 1;

/* LZ4F_BlockCompressMode_e */
const LZ4B_COMPRESSED: u32 = 0;
const LZ4B_UNCOMPRESSED: u32 = 1;

/* LZ4F_CtxType_e */
const CTX_NONE: u16 = 0;
const CTX_FAST: u16 = 1;
const CTX_HC: u16 = 2;

/* error codes */
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

static ERROR_STRINGS: [&[u8]; 25] = [
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
fn err_code(code: c_int) -> LZ4F_errorCode_t {
    (0isize).wrapping_sub(code as isize) as usize
}

macro_rules! ret_err {
    ($e:expr) => {
        return err_code($e)
    };
}

macro_rules! ret_err_if {
    ($c:expr, $e:expr) => {
        if $c {
            return err_code($e);
        }
    };
}

macro_rules! fwd_if_error {
    ($r:expr) => {
        if LZ4F_isError($r) != 0 {
            return $r;
        }
    };
}

/* ---------------- allocation helpers ---------------- */

unsafe fn LZ4F_calloc(s: usize, cmem: LZ4F_CustomMem) -> *mut c_void {
    if let Some(cc) = cmem.customCalloc {
        return cc(cmem.opaqueState, s);
    }
    if cmem.customAlloc.is_none() {
        return calloc(1, s);
    }
    let p = (cmem.customAlloc.unwrap())(cmem.opaqueState, s);
    if !p.is_null() {
        memset_n(p as *mut u8, 0, s);
    }
    p
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

const DEFAULT_CMEM: LZ4F_CustomMem = LZ4F_CustomMem {
    customAlloc: None,
    customCalloc: None,
    customFree: None,
    opaqueState: ptr::null_mut(),
};

/* ---------------- LE helpers ---------------- */

#[inline]
unsafe fn readLE32(src: *const u8) -> u32 {
    let p = src;
    (*p as u32)
        | ((*p.add(1) as u32) << 8)
        | ((*p.add(2) as u32) << 16)
        | ((*p.add(3) as u32) << 24)
}

#[inline]
unsafe fn writeLE32(dst: *mut u8, v: u32) {
    *dst = v as u8;
    *dst.add(1) = (v >> 8) as u8;
    *dst.add(2) = (v >> 16) as u8;
    *dst.add(3) = (v >> 24) as u8;
}

#[inline]
unsafe fn readLE64(src: *const u8) -> u64 {
    let p = src;
    (*p as u64)
        | ((*p.add(1) as u64) << 8)
        | ((*p.add(2) as u64) << 16)
        | ((*p.add(3) as u64) << 24)
        | ((*p.add(4) as u64) << 32)
        | ((*p.add(5) as u64) << 40)
        | ((*p.add(6) as u64) << 48)
        | ((*p.add(7) as u64) << 56)
}

#[inline]
unsafe fn writeLE64(dst: *mut u8, v: u64) {
    let mut i = 0;
    while i < 8 {
        *dst.add(i) = (v >> (8 * i)) as u8;
        i += 1;
    }
}

/* ---------------- error API ---------------- */

#[unsafe(no_mangle)]
pub extern "C" fn LZ4F_isError(code: LZ4F_errorCode_t) -> c_uint {
    (code > err_code(LZ4F_ERROR_maxCode)) as c_uint
}

#[unsafe(no_mangle)]
pub extern "C" fn LZ4F_getErrorName(code: LZ4F_errorCode_t) -> *const c_char {
    if LZ4F_isError(code) != 0 {
        let idx = (0i32).wrapping_sub(code as u32 as i32) as usize;
        if idx < ERROR_STRINGS.len() {
            return ERROR_STRINGS[idx].as_ptr() as *const c_char;
        }
        return ptr::null();
    }
    CODE_ERROR.as_ptr() as *const c_char
}

#[unsafe(no_mangle)]
pub extern "C" fn LZ4F_getErrorCode(function_result: usize) -> c_int {
    if LZ4F_isError(function_result) == 0 {
        return LZ4F_OK_NoError;
    }
    (0isize).wrapping_sub(function_result as isize) as c_int
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
pub extern "C" fn LZ4F_getBlockSize(mut block_size_id: u32) -> usize {
    const BLOCK_SIZES: [usize; 4] = [64 * 1024, 256 * 1024, 1024 * 1024, 4 * 1024 * 1024];
    if block_size_id == 0 {
        block_size_id = LZ4F_BLOCKSIZEID_DEFAULT;
    }
    if block_size_id < LZ4F_MAX64KB || block_size_id > LZ4F_MAX4MB {
        return err_code(LZ4F_ERROR_maxBlockSize_invalid);
    }
    BLOCK_SIZES[(block_size_id - LZ4F_MAX64KB) as usize]
}

/* ---------------- private helpers ---------------- */

#[inline]
unsafe fn LZ4F_headerChecksum(header: *const c_void, length: usize) -> u8 {
    let xxh = XXH32(header, length, 0);
    (xxh >> 8) as u8
}

fn LZ4F_optimalBSID(requested_bsid: u32, src_size: usize) -> u32 {
    let mut proposed_bsid: u32 = LZ4F_MAX64KB;
    let mut max_block_size: usize = 64 * 1024;
    while requested_bsid > proposed_bsid {
        if src_size <= max_block_size {
            return proposed_bsid;
        }
        proposed_bsid += 1;
        max_block_size <<= 2;
    }
    requested_bsid
}

const fn init_frameinfo() -> LZ4F_frameInfo_t {
    LZ4F_frameInfo_t {
        blockSizeID: LZ4F_MAX64KB,
        blockMode: LZ4F_BLOCK_LINKED,
        contentChecksumFlag: LZ4F_NO_CONTENT_CHECKSUM,
        frameType: LZ4F_FRAME,
        contentSize: 0,
        dictID: 0,
        blockChecksumFlag: 0,
    }
}

const fn init_preferences() -> LZ4F_preferences_t {
    LZ4F_preferences_t {
        frameInfo: init_frameinfo(),
        compressionLevel: 0,
        autoFlush: 0,
        favorDecSpeed: 0,
        reserved: [0, 0, 0],
    }
}

fn LZ4F_compressBound_internal(
    src_size: usize,
    preferences_ptr: *const LZ4F_preferences_t,
    already_buffered: usize,
) -> usize {
    let mut prefs_null = init_preferences();
    prefs_null.frameInfo.contentChecksumFlag = LZ4F_CONTENT_CHECKSUM_ENABLED;
    prefs_null.frameInfo.blockChecksumFlag = 1;
    let prefs: LZ4F_preferences_t = if preferences_ptr.is_null() {
        prefs_null
    } else {
        unsafe { *preferences_ptr }
    };
    let flush: u32 = prefs.autoFlush | ((src_size == 0) as u32);
    let block_id = prefs.frameInfo.blockSizeID;
    let block_size = LZ4F_getBlockSize(block_id);
    let max_buffered = block_size.wrapping_sub(1);
    let buffered_size = if already_buffered < max_buffered {
        already_buffered
    } else {
        max_buffered
    };
    let max_src_size = src_size.wrapping_add(buffered_size);
    let nb_full_blocks = (max_src_size / block_size) as u32;
    let partial_block_size = max_src_size & block_size.wrapping_sub(1);
    let last_block_size = if flush != 0 { partial_block_size } else { 0 };
    let nb_blocks = nb_full_blocks.wrapping_add((last_block_size > 0) as u32);

    let block_crc_size = BF_SIZE.wrapping_mul(prefs.frameInfo.blockChecksumFlag as usize);
    let frame_end =
        BH_SIZE.wrapping_add((prefs.frameInfo.contentChecksumFlag as usize).wrapping_mul(BF_SIZE));

    (BH_SIZE.wrapping_add(block_crc_size))
        .wrapping_mul(nb_blocks as usize)
        .wrapping_add(block_size.wrapping_mul(nb_full_blocks as usize))
        .wrapping_add(last_block_size)
        .wrapping_add(frame_end)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_compressFrameBound(
    src_size: usize,
    preferences_ptr: *const LZ4F_preferences_t,
) -> usize {
    let mut prefs: LZ4F_preferences_t;
    let header_size = MAX_FH_SIZE;

    if !preferences_ptr.is_null() {
        prefs = *preferences_ptr;
    } else {
        prefs = core::mem::zeroed();
    }
    prefs.autoFlush = 1;

    header_size.wrapping_add(LZ4F_compressBound_internal(src_size, &prefs, 0))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_compressBound(
    src_size: usize,
    preferences_ptr: *const LZ4F_preferences_t,
) -> usize {
    if !preferences_ptr.is_null() && (*preferences_ptr).autoFlush != 0 {
        return LZ4F_compressBound_internal(src_size, preferences_ptr, 0);
    }
    LZ4F_compressBound_internal(src_size, preferences_ptr, usize::MAX)
}

/* ---------------- CDict ---------------- */

#[repr(C)]
pub struct LZ4F_CDict {
    pub cmem: LZ4F_CustomMem,
    pub dictContent: *mut c_void,
    pub fastCtx: *mut LZ4_stream_t,
    pub HCCtx: *mut LZ4_streamHC_t,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_createCDict_advanced(
    cmem: LZ4F_CustomMem,
    dict_buffer: *const c_void,
    mut dict_size: usize,
) -> *mut LZ4F_CDict {
    let mut dict_start = dict_buffer as *const c_char;
    let cdict = LZ4F_malloc(core::mem::size_of::<LZ4F_CDict>(), cmem) as *mut LZ4F_CDict;
    if cdict.is_null() {
        return ptr::null_mut();
    }
    (*cdict).cmem = cmem;
    if dict_size > 64 * 1024 {
        dict_start = dict_start.wrapping_add(dict_size - 64 * 1024);
        dict_size = 64 * 1024;
    }
    (*cdict).dictContent = LZ4F_malloc(dict_size, cmem);
    (*cdict).fastCtx = LZ4F_malloc(core::mem::size_of::<LZ4_stream_t>(), cmem) as *mut LZ4_stream_t;
    (*cdict).HCCtx =
        LZ4F_malloc(core::mem::size_of::<LZ4_streamHC_t>(), cmem) as *mut LZ4_streamHC_t;
    if (*cdict).dictContent.is_null() || (*cdict).fastCtx.is_null() || (*cdict).HCCtx.is_null() {
        LZ4F_freeCDict(cdict);
        return ptr::null_mut();
    }
    memcpy_n(
        (*cdict).dictContent as *mut u8,
        dict_start as *const u8,
        dict_size,
    );
    LZ4_initStream(
        (*cdict).fastCtx as *mut c_void,
        core::mem::size_of::<LZ4_stream_t>(),
    );
    LZ4_loadDictSlow(
        (*cdict).fastCtx,
        (*cdict).dictContent as *const c_char,
        dict_size as c_int,
    );
    LZ4_initStreamHC(
        (*cdict).HCCtx as *mut c_void,
        core::mem::size_of::<LZ4_streamHC_t>(),
    );
    LZ4_setCompressionLevel((*cdict).HCCtx, LZ4HC_CLEVEL_DEFAULT);
    LZ4_loadDictHC(
        (*cdict).HCCtx,
        (*cdict).dictContent as *const c_char,
        dict_size as c_int,
    );
    cdict
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_createCDict(
    dict_buffer: *const c_void,
    dict_size: usize,
) -> *mut LZ4F_CDict {
    LZ4F_createCDict_advanced(DEFAULT_CMEM, dict_buffer, dict_size)
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

/* ---------------- compression context ---------------- */

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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_createCompressionContext_advanced(
    custom_mem: LZ4F_CustomMem,
    version: c_uint,
) -> *mut LZ4F_cctx {
    let cctx = LZ4F_calloc(core::mem::size_of::<LZ4F_cctx>(), custom_mem) as *mut LZ4F_cctx;
    if cctx.is_null() {
        return ptr::null_mut();
    }
    (*cctx).cmem = custom_mem;
    (*cctx).version = version;
    (*cctx).cStage = 0;
    cctx
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_createCompressionContext(
    cctx_ptr: *mut *mut LZ4F_cctx,
    version: c_uint,
) -> LZ4F_errorCode_t {
    ret_err_if!(cctx_ptr.is_null(), LZ4F_ERROR_parameter_null);
    *cctx_ptr = LZ4F_createCompressionContext_advanced(DEFAULT_CMEM, version);
    ret_err_if!((*cctx_ptr).is_null(), LZ4F_ERROR_allocation_failed);
    LZ4F_OK_NoError as usize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_freeCompressionContext(cctx: *mut LZ4F_cctx) -> LZ4F_errorCode_t {
    if !cctx.is_null() {
        let cmem = (*cctx).cmem;
        LZ4F_free((*cctx).lz4CtxPtr, cmem);
        LZ4F_free((*cctx).tmpBuff as *mut c_void, cmem);
        LZ4F_free(cctx as *mut c_void, cmem);
    }
    LZ4F_OK_NoError as usize
}

unsafe fn LZ4F_initStream_internal(
    ctx: *mut c_void,
    cdict: *const LZ4F_CDict,
    level: c_int,
    block_mode: u32,
) {
    if level < LZ4HC_CLEVEL_MIN {
        if !cdict.is_null() || block_mode == LZ4F_BLOCK_LINKED {
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

fn ctxTypeID_to_size(ctx_type_id: c_int) -> c_int {
    match ctx_type_id {
        1 => LZ4_sizeofState(),
        2 => LZ4_sizeofStateHC(),
        _ => 0,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_compressBegin_internal(
    cctx: *mut LZ4F_cctx,
    dst_buffer: *mut c_void,
    dst_capacity: usize,
    dict_buffer: *const c_void,
    dict_size: usize,
    cdict: *const LZ4F_CDict,
    mut preferences_ptr: *const LZ4F_preferences_t,
) -> usize {
    let pref_null = init_preferences();
    let dst_start = dst_buffer as *mut u8;
    let mut dst_ptr = dst_start;

    ret_err_if!(dst_capacity < MAX_FH_SIZE, LZ4F_ERROR_dstMaxSize_tooSmall);
    if preferences_ptr.is_null() {
        preferences_ptr = &pref_null;
    }
    (*cctx).prefs = *preferences_ptr;

    /* cctx Management */
    {
        let ctx_type_id: u16 = if (*cctx).prefs.compressionLevel < LZ4HC_CLEVEL_MIN {
            1
        } else {
            2
        };
        let required_size = ctxTypeID_to_size(ctx_type_id as c_int);
        let allocated_size = ctxTypeID_to_size((*cctx).lz4CtxAlloc as c_int);
        if allocated_size < required_size {
            LZ4F_free((*cctx).lz4CtxPtr, (*cctx).cmem);
            if (*cctx).prefs.compressionLevel < LZ4HC_CLEVEL_MIN {
                (*cctx).lz4CtxPtr =
                    LZ4F_malloc(core::mem::size_of::<LZ4_stream_t>(), (*cctx).cmem);
                if !(*cctx).lz4CtxPtr.is_null() {
                    LZ4_initStream((*cctx).lz4CtxPtr, core::mem::size_of::<LZ4_stream_t>());
                }
            } else {
                (*cctx).lz4CtxPtr =
                    LZ4F_malloc(core::mem::size_of::<LZ4_streamHC_t>(), (*cctx).cmem);
                if !(*cctx).lz4CtxPtr.is_null() {
                    LZ4_initStreamHC((*cctx).lz4CtxPtr, core::mem::size_of::<LZ4_streamHC_t>());
                }
            }
            ret_err_if!((*cctx).lz4CtxPtr.is_null(), LZ4F_ERROR_allocation_failed);
            (*cctx).lz4CtxAlloc = ctx_type_id;
            (*cctx).lz4CtxType = ctx_type_id;
        } else if (*cctx).lz4CtxType != ctx_type_id {
            if (*cctx).prefs.compressionLevel < LZ4HC_CLEVEL_MIN {
                LZ4_initStream(
                    (*cctx).lz4CtxPtr as *mut c_void,
                    core::mem::size_of::<LZ4_stream_t>(),
                );
            } else {
                LZ4_initStreamHC(
                    (*cctx).lz4CtxPtr as *mut c_void,
                    core::mem::size_of::<LZ4_streamHC_t>(),
                );
                LZ4_setCompressionLevel(
                    (*cctx).lz4CtxPtr as *mut LZ4_streamHC_t,
                    (*cctx).prefs.compressionLevel,
                );
            }
            (*cctx).lz4CtxType = ctx_type_id;
        }
    }

    /* Buffer Management */
    if (*cctx).prefs.frameInfo.blockSizeID == 0 {
        (*cctx).prefs.frameInfo.blockSizeID = LZ4F_BLOCKSIZEID_DEFAULT;
    }
    (*cctx).maxBlockSize = LZ4F_getBlockSize((*cctx).prefs.frameInfo.blockSizeID);

    {
        let required_buff_size: usize = if (*preferences_ptr).autoFlush != 0 {
            if (*cctx).prefs.frameInfo.blockMode == LZ4F_BLOCK_LINKED {
                64 * 1024
            } else {
                0
            }
        } else {
            (*cctx).maxBlockSize.wrapping_add(
                if (*cctx).prefs.frameInfo.blockMode == LZ4F_BLOCK_LINKED {
                    128 * 1024
                } else {
                    0
                },
            )
        };

        if (*cctx).maxBufferSize < required_buff_size {
            (*cctx).maxBufferSize = 0;
            LZ4F_free((*cctx).tmpBuff as *mut c_void, (*cctx).cmem);
            (*cctx).tmpBuff = LZ4F_malloc(required_buff_size, (*cctx).cmem) as *mut u8;
            ret_err_if!((*cctx).tmpBuff.is_null(), LZ4F_ERROR_allocation_failed);
            (*cctx).maxBufferSize = required_buff_size;
        }
    }
    (*cctx).tmpIn = (*cctx).tmpBuff;
    (*cctx).tmpInSize = 0;
    XXH32_reset(&mut (*cctx).xxh, 0);

    /* context init */
    (*cctx).cdict = cdict;
    if (*cctx).prefs.frameInfo.blockMode == LZ4F_BLOCK_LINKED {
        LZ4F_initStream_internal(
            (*cctx).lz4CtxPtr,
            cdict,
            (*cctx).prefs.compressionLevel,
            LZ4F_BLOCK_LINKED,
        );
    }
    if (*preferences_ptr).compressionLevel >= LZ4HC_CLEVEL_MIN {
        LZ4_favorDecompressionSpeed(
            (*cctx).lz4CtxPtr as *mut LZ4_streamHC_t,
            (*preferences_ptr).favorDecSpeed as c_int,
        );
    }
    if !dict_buffer.is_null() {
        ret_err_if!(dict_size > c_int::MAX as usize, LZ4F_ERROR_parameter_invalid);
        if (*cctx).lz4CtxType == CTX_FAST {
            LZ4_loadDict(
                (*cctx).lz4CtxPtr as *mut LZ4_stream_t,
                dict_buffer as *const c_char,
                dict_size as c_int,
            );
        } else {
            LZ4_loadDictHC(
                (*cctx).lz4CtxPtr as *mut LZ4_streamHC_t,
                dict_buffer as *const c_char,
                dict_size as c_int,
            );
        }
    }

    /* Stage 2 : Write Frame Header */
    writeLE32(dst_ptr, LZ4F_MAGICNUMBER);
    dst_ptr = dst_ptr.add(4);
    {
        let header_start = dst_ptr;

        /* FLG Byte */
        *dst_ptr = (((1u32 & 0x03) << 6)
            + (((*cctx).prefs.frameInfo.blockMode & 0x01) << 5)
            + (((*cctx).prefs.frameInfo.blockChecksumFlag & 0x01) << 4)
            + ((((*cctx).prefs.frameInfo.contentSize > 0) as u32) << 3)
            + (((*cctx).prefs.frameInfo.contentChecksumFlag & 0x01) << 2)
            + (((*cctx).prefs.frameInfo.dictID > 0) as u32)) as u8;
        dst_ptr = dst_ptr.add(1);
        /* BD Byte */
        *dst_ptr = (((*cctx).prefs.frameInfo.blockSizeID & 0x07) << 4) as u8;
        dst_ptr = dst_ptr.add(1);
        /* Optional content size */
        if (*cctx).prefs.frameInfo.contentSize != 0 {
            writeLE64(dst_ptr, (*cctx).prefs.frameInfo.contentSize);
            dst_ptr = dst_ptr.add(8);
            (*cctx).totalInSize = 0;
        }
        /* Optional dictionary ID */
        if (*cctx).prefs.frameInfo.dictID != 0 {
            writeLE32(dst_ptr, (*cctx).prefs.frameInfo.dictID);
            dst_ptr = dst_ptr.add(4);
        }
        /* Header CRC Byte */
        *dst_ptr = LZ4F_headerChecksum(
            header_start as *const c_void,
            pdiff(dst_ptr, header_start) as usize,
        );
        dst_ptr = dst_ptr.add(1);
    }

    (*cctx).cStage = 1;
    pdiff(dst_ptr, dst_start) as usize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_compressBegin(
    cctx: *mut LZ4F_cctx,
    dst_buffer: *mut c_void,
    dst_capacity: usize,
    preferences_ptr: *const LZ4F_preferences_t,
) -> usize {
    LZ4F_compressBegin_internal(
        cctx,
        dst_buffer,
        dst_capacity,
        ptr::null(),
        0,
        ptr::null(),
        preferences_ptr,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_compressBegin_usingDictOnce(
    cctx: *mut LZ4F_cctx,
    dst_buffer: *mut c_void,
    dst_capacity: usize,
    dict: *const c_void,
    dict_size: usize,
    preferences_ptr: *const LZ4F_preferences_t,
) -> usize {
    LZ4F_compressBegin_internal(
        cctx,
        dst_buffer,
        dst_capacity,
        dict,
        dict_size,
        ptr::null(),
        preferences_ptr,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_compressBegin_usingDict(
    cctx: *mut LZ4F_cctx,
    dst_buffer: *mut c_void,
    dst_capacity: usize,
    dict: *const c_void,
    dict_size: usize,
    preferences_ptr: *const LZ4F_preferences_t,
) -> usize {
    LZ4F_compressBegin_usingDictOnce(
        cctx,
        dst_buffer,
        dst_capacity,
        dict,
        dict_size,
        preferences_ptr,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_compressBegin_usingCDict(
    cctx: *mut LZ4F_cctx,
    dst_buffer: *mut c_void,
    dst_capacity: usize,
    cdict: *const LZ4F_CDict,
    preferences_ptr: *const LZ4F_preferences_t,
) -> usize {
    LZ4F_compressBegin_internal(
        cctx,
        dst_buffer,
        dst_capacity,
        ptr::null(),
        0,
        cdict,
        preferences_ptr,
    )
}

/* ---------------- block compression ---------------- */

/* compress function selector */
const CF_DO_NOT_COMPRESS: c_int = 0;
const CF_BLOCK: c_int = 1;
const CF_BLOCK_CONTINUE: c_int = 2;
const CF_BLOCK_HC: c_int = 3;
const CF_BLOCK_HC_CONTINUE: c_int = 4;

unsafe fn call_compress(
    which: c_int,
    ctx: *mut c_void,
    src: *const c_char,
    dst: *mut c_char,
    src_size: c_int,
    dst_capacity: c_int,
    level: c_int,
    cdict: *const LZ4F_CDict,
) -> c_int {
    match which {
        CF_DO_NOT_COMPRESS => 0,
        CF_BLOCK => {
            let acceleration = if level < 0 { -level + 1 } else { 1 };
            LZ4F_initStream_internal(ctx, cdict, level, LZ4F_BLOCK_INDEPENDENT);
            if !cdict.is_null() {
                LZ4_compress_fast_continue(
                    ctx as *mut LZ4_stream_t,
                    src,
                    dst,
                    src_size,
                    dst_capacity,
                    acceleration,
                )
            } else {
                LZ4_compress_fast_extState_fastReset(
                    ctx,
                    src,
                    dst,
                    src_size,
                    dst_capacity,
                    acceleration,
                )
            }
        }
        CF_BLOCK_CONTINUE => {
            let acceleration = if level < 0 { -level + 1 } else { 1 };
            LZ4_compress_fast_continue(
                ctx as *mut LZ4_stream_t,
                src,
                dst,
                src_size,
                dst_capacity,
                acceleration,
            )
        }
        CF_BLOCK_HC => {
            LZ4F_initStream_internal(ctx, cdict, level, LZ4F_BLOCK_INDEPENDENT);
            if !cdict.is_null() {
                LZ4_compress_HC_continue(
                    ctx as *mut LZ4_streamHC_t,
                    src,
                    dst,
                    src_size,
                    dst_capacity,
                )
            } else {
                LZ4_compress_HC_extStateHC_fastReset(ctx, src, dst, src_size, dst_capacity, level)
            }
        }
        _ => LZ4_compress_HC_continue(ctx as *mut LZ4_streamHC_t, src, dst, src_size, dst_capacity),
    }
}

fn LZ4F_selectCompression(block_mode: u32, level: c_int, compress_mode: u32) -> c_int {
    if compress_mode == LZ4B_UNCOMPRESSED {
        return CF_DO_NOT_COMPRESS;
    }
    if level < LZ4HC_CLEVEL_MIN {
        if block_mode == LZ4F_BLOCK_INDEPENDENT {
            return CF_BLOCK;
        }
        return CF_BLOCK_CONTINUE;
    }
    if block_mode == LZ4F_BLOCK_INDEPENDENT {
        return CF_BLOCK_HC;
    }
    CF_BLOCK_HC_CONTINUE
}

unsafe fn LZ4F_makeBlock(
    dst: *mut u8,
    src: *const u8,
    src_size: usize,
    compress: c_int,
    lz4ctx: *mut c_void,
    level: c_int,
    cdict: *const LZ4F_CDict,
    crc_flag: u32,
) -> usize {
    let c_size_ptr = dst;
    let mut c_size: u32;
    c_size = call_compress(
        compress,
        lz4ctx,
        src as *const c_char,
        c_size_ptr.add(BH_SIZE) as *mut c_char,
        src_size as c_int,
        (src_size as c_int).wrapping_sub(1),
        level,
        cdict,
    ) as u32;

    if c_size == 0 || (c_size as usize) >= src_size {
        c_size = src_size as u32;
        writeLE32(c_size_ptr, c_size | LZ4F_BLOCKUNCOMPRESSED_FLAG);
        memcpy_n(c_size_ptr.add(BH_SIZE), src, src_size);
    } else {
        writeLE32(c_size_ptr, c_size);
    }
    if crc_flag != 0 {
        let crc32 = XXH32(
            c_size_ptr.add(BH_SIZE) as *const c_void,
            c_size as usize,
            0,
        );
        writeLE32(c_size_ptr.add(BH_SIZE + c_size as usize), crc32);
    }
    BH_SIZE + c_size as usize + (crc_flag as usize) * BF_SIZE
}

unsafe fn LZ4F_localSaveDict(cctx: *mut LZ4F_cctx) -> c_int {
    if (*cctx).prefs.compressionLevel < LZ4HC_CLEVEL_MIN {
        return LZ4_saveDict(
            (*cctx).lz4CtxPtr as *mut LZ4_stream_t,
            (*cctx).tmpBuff as *mut c_char,
            64 * 1024,
        );
    }
    LZ4_saveDictHC(
        (*cctx).lz4CtxPtr as *mut LZ4_streamHC_t,
        (*cctx).tmpBuff as *mut c_char,
        64 * 1024,
    )
}

const NOT_DONE: c_int = 0;
const FROM_TMP_BUFFER: c_int = 1;
const FROM_SRC_BUFFER: c_int = 2;

const K_C_OPTIONS_NULL: LZ4F_compressOptions_t = LZ4F_compressOptions_t {
    stableSrc: 0,
    reserved: [0, 0, 0],
};

unsafe fn LZ4F_compressUpdateImpl(
    cctx: *mut LZ4F_cctx,
    dst_buffer: *mut c_void,
    dst_capacity: usize,
    src_buffer: *const c_void,
    src_size: usize,
    mut compress_options_ptr: *const LZ4F_compressOptions_t,
    block_compression: u32,
) -> usize {
    let block_size = (*cctx).maxBlockSize;
    let mut src_ptr = src_buffer as *const u8;
    let src_end = src_ptr.wrapping_add(src_size);
    let dst_start = dst_buffer as *mut u8;
    let mut dst_ptr = dst_start;
    let mut last_block_compressed = NOT_DONE;
    let compress = LZ4F_selectCompression(
        (*cctx).prefs.frameInfo.blockMode,
        (*cctx).prefs.compressionLevel,
        block_compression,
    );
    let bytes_written: usize;

    ret_err_if!(
        (*cctx).cStage != 1,
        LZ4F_ERROR_compressionState_uninitialized
    );
    if dst_capacity < LZ4F_compressBound_internal(src_size, &(*cctx).prefs, (*cctx).tmpInSize) {
        ret_err!(LZ4F_ERROR_dstMaxSize_tooSmall);
    }

    if block_compression == LZ4B_UNCOMPRESSED && dst_capacity < src_size {
        ret_err!(LZ4F_ERROR_dstMaxSize_tooSmall);
    }

    /* flush currently written block, to continue with new block compression */
    if (*cctx).blockCompressMode != block_compression {
        bytes_written = LZ4F_flush(cctx, dst_buffer, dst_capacity, compress_options_ptr);
        dst_ptr = dst_ptr.wrapping_add(bytes_written);
        (*cctx).blockCompressMode = block_compression;
    }

    if compress_options_ptr.is_null() {
        compress_options_ptr = &K_C_OPTIONS_NULL;
    }

    /* complete tmp buffer */
    if (*cctx).tmpInSize > 0 {
        let size_to_copy = block_size.wrapping_sub((*cctx).tmpInSize);
        if size_to_copy > src_size {
            memcpy_n(
                (*cctx).tmpIn.add((*cctx).tmpInSize),
                src_buffer as *const u8,
                src_size,
            );
            src_ptr = src_end;
            (*cctx).tmpInSize += src_size;
        } else {
            last_block_compressed = FROM_TMP_BUFFER;
            memcpy_n(
                (*cctx).tmpIn.add((*cctx).tmpInSize),
                src_buffer as *const u8,
                size_to_copy,
            );
            src_ptr = src_ptr.wrapping_add(size_to_copy);

            dst_ptr = dst_ptr.wrapping_add(LZ4F_makeBlock(
                dst_ptr,
                (*cctx).tmpIn,
                block_size,
                compress,
                (*cctx).lz4CtxPtr,
                (*cctx).prefs.compressionLevel,
                (*cctx).cdict,
                (*cctx).prefs.frameInfo.blockChecksumFlag,
            ));
            if (*cctx).prefs.frameInfo.blockMode == LZ4F_BLOCK_LINKED {
                (*cctx).tmpIn = (*cctx).tmpIn.add(block_size);
            }
            (*cctx).tmpInSize = 0;
        }
    }

    while (pdiff(src_end, src_ptr) as usize) >= block_size {
        last_block_compressed = FROM_SRC_BUFFER;
        dst_ptr = dst_ptr.wrapping_add(LZ4F_makeBlock(
            dst_ptr,
            src_ptr,
            block_size,
            compress,
            (*cctx).lz4CtxPtr,
            (*cctx).prefs.compressionLevel,
            (*cctx).cdict,
            (*cctx).prefs.frameInfo.blockChecksumFlag,
        ));
        src_ptr = src_ptr.wrapping_add(block_size);
    }

    if (*cctx).prefs.autoFlush != 0 && src_ptr < src_end {
        last_block_compressed = FROM_SRC_BUFFER;
        dst_ptr = dst_ptr.wrapping_add(LZ4F_makeBlock(
            dst_ptr,
            src_ptr,
            pdiff(src_end, src_ptr) as usize,
            compress,
            (*cctx).lz4CtxPtr,
            (*cctx).prefs.compressionLevel,
            (*cctx).cdict,
            (*cctx).prefs.frameInfo.blockChecksumFlag,
        ));
        src_ptr = src_end;
    }

    /* preserve dictionary within tmpBuff whenever necessary */
    if (*cctx).prefs.frameInfo.blockMode == LZ4F_BLOCK_LINKED
        && last_block_compressed == FROM_SRC_BUFFER
    {
        if (*compress_options_ptr).stableSrc != 0 {
            (*cctx).tmpIn = (*cctx).tmpBuff;
        } else {
            let real_dict_size = LZ4F_localSaveDict(cctx);
            (*cctx).tmpIn = (*cctx).tmpBuff.wrapping_add(real_dict_size as usize);
        }
    }

    /* keep tmpIn within limits */
    if (*cctx).prefs.autoFlush == 0
        && (*cctx).tmpIn.wrapping_add(block_size)
            > (*cctx).tmpBuff.wrapping_add((*cctx).maxBufferSize)
    {
        let real_dict_size = LZ4F_localSaveDict(cctx);
        (*cctx).tmpIn = (*cctx).tmpBuff.wrapping_add(real_dict_size as usize);
    }

    /* some input data left, necessarily < blockSize */
    if src_ptr < src_end {
        let size_to_copy = pdiff(src_end, src_ptr) as usize;
        memcpy_n((*cctx).tmpIn, src_ptr, size_to_copy);
        (*cctx).tmpInSize = size_to_copy;
    }

    if (*cctx).prefs.frameInfo.contentChecksumFlag == LZ4F_CONTENT_CHECKSUM_ENABLED {
        XXH32_update(&mut (*cctx).xxh, src_buffer, src_size);
    }

    (*cctx).totalInSize = (*cctx).totalInSize.wrapping_add(src_size as u64);
    pdiff(dst_ptr, dst_start) as usize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_compressUpdate(
    cctx: *mut LZ4F_cctx,
    dst_buffer: *mut c_void,
    dst_capacity: usize,
    src_buffer: *const c_void,
    src_size: usize,
    compress_options_ptr: *const LZ4F_compressOptions_t,
) -> usize {
    LZ4F_compressUpdateImpl(
        cctx,
        dst_buffer,
        dst_capacity,
        src_buffer,
        src_size,
        compress_options_ptr,
        LZ4B_COMPRESSED,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_uncompressedUpdate(
    cctx: *mut LZ4F_cctx,
    dst_buffer: *mut c_void,
    dst_capacity: usize,
    src_buffer: *const c_void,
    src_size: usize,
    compress_options_ptr: *const LZ4F_compressOptions_t,
) -> usize {
    LZ4F_compressUpdateImpl(
        cctx,
        dst_buffer,
        dst_capacity,
        src_buffer,
        src_size,
        compress_options_ptr,
        LZ4B_UNCOMPRESSED,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_flush(
    cctx: *mut LZ4F_cctx,
    dst_buffer: *mut c_void,
    dst_capacity: usize,
    _compress_options_ptr: *const LZ4F_compressOptions_t,
) -> usize {
    let dst_start = dst_buffer as *mut u8;
    let mut dst_ptr = dst_start;

    if (*cctx).tmpInSize == 0 {
        return 0;
    }
    ret_err_if!(
        (*cctx).cStage != 1,
        LZ4F_ERROR_compressionState_uninitialized
    );
    ret_err_if!(
        dst_capacity < ((*cctx).tmpInSize + BH_SIZE + BF_SIZE),
        LZ4F_ERROR_dstMaxSize_tooSmall
    );

    let compress = LZ4F_selectCompression(
        (*cctx).prefs.frameInfo.blockMode,
        (*cctx).prefs.compressionLevel,
        (*cctx).blockCompressMode,
    );

    dst_ptr = dst_ptr.wrapping_add(LZ4F_makeBlock(
        dst_ptr,
        (*cctx).tmpIn,
        (*cctx).tmpInSize,
        compress,
        (*cctx).lz4CtxPtr,
        (*cctx).prefs.compressionLevel,
        (*cctx).cdict,
        (*cctx).prefs.frameInfo.blockChecksumFlag,
    ));

    if (*cctx).prefs.frameInfo.blockMode == LZ4F_BLOCK_LINKED {
        (*cctx).tmpIn = (*cctx).tmpIn.wrapping_add((*cctx).tmpInSize);
    }
    (*cctx).tmpInSize = 0;

    /* keep tmpIn within limits */
    if (*cctx).tmpIn.wrapping_add((*cctx).maxBlockSize)
        > (*cctx).tmpBuff.wrapping_add((*cctx).maxBufferSize)
    {
        let real_dict_size = LZ4F_localSaveDict(cctx);
        (*cctx).tmpIn = (*cctx).tmpBuff.wrapping_add(real_dict_size as usize);
    }

    pdiff(dst_ptr, dst_start) as usize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_compressEnd(
    cctx: *mut LZ4F_cctx,
    dst_buffer: *mut c_void,
    mut dst_capacity: usize,
    compress_options_ptr: *const LZ4F_compressOptions_t,
) -> usize {
    let dst_start = dst_buffer as *mut u8;
    let mut dst_ptr = dst_start;

    let flush_size = LZ4F_flush(cctx, dst_buffer, dst_capacity, compress_options_ptr);
    fwd_if_error!(flush_size);
    dst_ptr = dst_ptr.wrapping_add(flush_size);

    dst_capacity -= flush_size;

    ret_err_if!(dst_capacity < 4, LZ4F_ERROR_dstMaxSize_tooSmall);
    writeLE32(dst_ptr, 0);
    dst_ptr = dst_ptr.add(4);

    if (*cctx).prefs.frameInfo.contentChecksumFlag == LZ4F_CONTENT_CHECKSUM_ENABLED {
        let xxh = XXH32_digest(&(*cctx).xxh);
        ret_err_if!(dst_capacity < 8, LZ4F_ERROR_dstMaxSize_tooSmall);
        writeLE32(dst_ptr, xxh);
        dst_ptr = dst_ptr.add(4);
    }

    (*cctx).cStage = 0;

    if (*cctx).prefs.frameInfo.contentSize != 0 {
        if (*cctx).prefs.frameInfo.contentSize != (*cctx).totalInSize {
            ret_err!(LZ4F_ERROR_frameSize_wrong);
        }
    }

    pdiff(dst_ptr, dst_start) as usize
}

/* ---------------- single-pass frame compression ---------------- */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_compressFrame_usingCDict(
    cctx: *mut LZ4F_cctx,
    dst_buffer: *mut c_void,
    dst_capacity: usize,
    src_buffer: *const c_void,
    src_size: usize,
    cdict: *const LZ4F_CDict,
    preferences_ptr: *const LZ4F_preferences_t,
) -> usize {
    let mut prefs: LZ4F_preferences_t;
    let mut options: LZ4F_compressOptions_t;
    let dst_start = dst_buffer as *mut u8;
    let mut dst_ptr = dst_start;
    let dst_end = dst_start.wrapping_add(dst_capacity);

    if !preferences_ptr.is_null() {
        prefs = *preferences_ptr;
    } else {
        prefs = core::mem::zeroed();
    }
    if prefs.frameInfo.contentSize != 0 {
        prefs.frameInfo.contentSize = src_size as u64;
    }

    prefs.frameInfo.blockSizeID = LZ4F_optimalBSID(prefs.frameInfo.blockSizeID, src_size);
    prefs.autoFlush = 1;
    if src_size <= LZ4F_getBlockSize(prefs.frameInfo.blockSizeID) {
        prefs.frameInfo.blockMode = LZ4F_BLOCK_INDEPENDENT;
    }

    options = core::mem::zeroed();
    options.stableSrc = 1;

    ret_err_if!(
        dst_capacity < LZ4F_compressFrameBound(src_size, &prefs),
        LZ4F_ERROR_dstMaxSize_tooSmall
    );

    {
        let header_size =
            LZ4F_compressBegin_usingCDict(cctx, dst_buffer, dst_capacity, cdict, &prefs);
        fwd_if_error!(header_size);
        dst_ptr = dst_ptr.wrapping_add(header_size);
    }

    {
        let c_size = LZ4F_compressUpdate(
            cctx,
            dst_ptr as *mut c_void,
            pdiff(dst_end, dst_ptr) as usize,
            src_buffer,
            src_size,
            &options,
        );
        fwd_if_error!(c_size);
        dst_ptr = dst_ptr.wrapping_add(c_size);
    }

    {
        let tail_size = LZ4F_compressEnd(
            cctx,
            dst_ptr as *mut c_void,
            pdiff(dst_end, dst_ptr) as usize,
            &options,
        );
        fwd_if_error!(tail_size);
        dst_ptr = dst_ptr.wrapping_add(tail_size);
    }

    pdiff(dst_ptr, dst_start) as usize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_compressFrame(
    dst_buffer: *mut c_void,
    dst_capacity: usize,
    src_buffer: *const c_void,
    src_size: usize,
    preferences_ptr: *const LZ4F_preferences_t,
) -> usize {
    let result: usize;
    /* LZ4F_HEAPMODE == 0 : state on stack */
    let mut cctx: LZ4F_cctx = core::mem::zeroed();
    let mut lz4ctx: LZ4_stream_t = core::mem::zeroed();
    let cctx_ptr: *mut LZ4F_cctx = &mut cctx;

    cctx.version = LZ4F_VERSION;
    cctx.maxBufferSize = 5 * 1024 * 1024;
    if preferences_ptr.is_null() || (*preferences_ptr).compressionLevel < LZ4HC_CLEVEL_MIN {
        LZ4_initStream(
            &mut lz4ctx as *mut LZ4_stream_t as *mut c_void,
            core::mem::size_of::<LZ4_stream_t>(),
        );
        (*cctx_ptr).lz4CtxPtr = &mut lz4ctx as *mut LZ4_stream_t as *mut c_void;
        (*cctx_ptr).lz4CtxAlloc = 1;
        (*cctx_ptr).lz4CtxType = CTX_FAST;
    }

    result = LZ4F_compressFrame_usingCDict(
        cctx_ptr,
        dst_buffer,
        dst_capacity,
        src_buffer,
        src_size,
        ptr::null(),
        preferences_ptr,
    );

    if !preferences_ptr.is_null() && (*preferences_ptr).compressionLevel >= LZ4HC_CLEVEL_MIN {
        LZ4F_free((*cctx_ptr).lz4CtxPtr, (*cctx_ptr).cmem);
    }
    result
}

/* ---------------- decompression ---------------- */

const DSTAGE_GET_FRAME_HEADER: u32 = 0;
const DSTAGE_STORE_FRAME_HEADER: u32 = 1;
const DSTAGE_INIT: u32 = 2;
const DSTAGE_GET_BLOCK_HEADER: u32 = 3;
const DSTAGE_STORE_BLOCK_HEADER: u32 = 4;
const DSTAGE_COPY_DIRECT: u32 = 5;
const DSTAGE_GET_BLOCK_CHECKSUM: u32 = 6;
const DSTAGE_GET_CBLOCK: u32 = 7;
const DSTAGE_STORE_CBLOCK: u32 = 8;
const DSTAGE_FLUSH_OUT: u32 = 9;
const DSTAGE_GET_SUFFIX: u32 = 10;
const DSTAGE_STORE_SUFFIX: u32 = 11;
const DSTAGE_GET_SFRAME_SIZE: u32 = 12;
const DSTAGE_STORE_SFRAME_SIZE: u32 = 13;
const DSTAGE_SKIP_SKIPPABLE: u32 = 14;

/* additional internal labels (not real dStage values) */
const L_DECODE_BLOCK_HEADER: u32 = 100;
const L_AFTER_CBLOCK: u32 = 101;
const L_CHECK_SUFFIX: u32 = 102;
const L_DECODE_SFRAME_SIZE: u32 = 103;

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
    custom_mem: LZ4F_CustomMem,
    version: c_uint,
) -> *mut LZ4F_dctx {
    let dctx = LZ4F_calloc(core::mem::size_of::<LZ4F_dctx>(), custom_mem) as *mut LZ4F_dctx;
    if dctx.is_null() {
        return ptr::null_mut();
    }
    (*dctx).cmem = custom_mem;
    (*dctx).version = version;
    dctx
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_createDecompressionContext(
    dctx_ptr: *mut *mut LZ4F_dctx,
    version_number: c_uint,
) -> LZ4F_errorCode_t {
    ret_err_if!(dctx_ptr.is_null(), LZ4F_ERROR_parameter_null);
    *dctx_ptr = LZ4F_createDecompressionContext_advanced(DEFAULT_CMEM, version_number);
    if (*dctx_ptr).is_null() {
        ret_err!(LZ4F_ERROR_allocation_failed);
    }
    LZ4F_OK_NoError as usize
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_freeDecompressionContext(dctx: *mut LZ4F_dctx) -> LZ4F_errorCode_t {
    let mut result: LZ4F_errorCode_t = LZ4F_OK_NoError as usize;
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
    (*dctx).dStage = DSTAGE_GET_FRAME_HEADER;
    (*dctx).dict = ptr::null();
    (*dctx).dictSize = 0;
    (*dctx).skipChecksum = 0;
    (*dctx).frameRemainingSize = 0;
}

unsafe fn LZ4F_decodeHeader(dctx: *mut LZ4F_dctx, src: *const c_void, src_size: usize) -> usize {
    let block_mode: u32;
    let block_checksum_flag: u32;
    let content_size_flag: u32;
    let content_checksum_flag: u32;
    let dict_id_flag: u32;
    let block_size_id: u32;
    let frame_header_size: usize;
    let src_ptr = src as *const u8;

    ret_err_if!(src_size < MIN_FH_SIZE, LZ4F_ERROR_frameHeader_incomplete);
    memset_n(
        &mut (*dctx).frameInfo as *mut LZ4F_frameInfo_t as *mut u8,
        0,
        core::mem::size_of::<LZ4F_frameInfo_t>(),
    );

    /* special case : skippable frames */
    if (readLE32(src_ptr) & 0xFFFFFFF0) == LZ4F_MAGIC_SKIPPABLE_START {
        (*dctx).frameInfo.frameType = LZ4F_SKIPPABLE_FRAME;
        if src == (*dctx).header.as_ptr() as *const c_void {
            (*dctx).tmpInSize = src_size;
            (*dctx).tmpInTarget = 8;
            (*dctx).dStage = DSTAGE_STORE_SFRAME_SIZE;
            return src_size;
        } else {
            (*dctx).dStage = DSTAGE_GET_SFRAME_SIZE;
            return 4;
        }
    }

    if readLE32(src_ptr) != LZ4F_MAGICNUMBER {
        ret_err!(LZ4F_ERROR_frameType_unknown);
    }
    (*dctx).frameInfo.frameType = LZ4F_FRAME;

    /* Flags */
    {
        let flg = *src_ptr.add(4) as u32;
        let version = (flg >> 6) & 0x03;
        block_checksum_flag = (flg >> 4) & 0x01;
        block_mode = (flg >> 5) & 0x01;
        content_size_flag = (flg >> 3) & 0x01;
        content_checksum_flag = (flg >> 2) & 0x01;
        dict_id_flag = flg & 0x01;
        if ((flg >> 1) & 0x01) != 0 {
            ret_err!(LZ4F_ERROR_reservedFlag_set);
        }
        if version != 1 {
            ret_err!(LZ4F_ERROR_headerVersion_wrong);
        }
    }

    frame_header_size = MIN_FH_SIZE
        + (if content_size_flag != 0 { 8 } else { 0 })
        + (if dict_id_flag != 0 { 4 } else { 0 });

    if src_size < frame_header_size {
        if src_ptr != (*dctx).header.as_ptr() {
            memcpy_n((*dctx).header.as_mut_ptr(), src_ptr, src_size);
        }
        (*dctx).tmpInSize = src_size;
        (*dctx).tmpInTarget = frame_header_size;
        (*dctx).dStage = DSTAGE_STORE_FRAME_HEADER;
        return src_size;
    }

    {
        let bd = *src_ptr.add(5) as u32;
        block_size_id = (bd >> 4) & 0x07;
        if ((bd >> 7) & 0x01) != 0 {
            ret_err!(LZ4F_ERROR_reservedFlag_set);
        }
        if block_size_id < 4 {
            ret_err!(LZ4F_ERROR_maxBlockSize_invalid);
        }
        if (bd & 0x0F) != 0 {
            ret_err!(LZ4F_ERROR_reservedFlag_set);
        }
    }

    /* check header */
    {
        let hc = LZ4F_headerChecksum(src_ptr.add(4) as *const c_void, frame_header_size - 5);
        ret_err_if!(
            hc != *src_ptr.add(frame_header_size - 1),
            LZ4F_ERROR_headerChecksum_invalid
        );
    }

    (*dctx).frameInfo.blockMode = block_mode;
    (*dctx).frameInfo.blockChecksumFlag = block_checksum_flag;
    (*dctx).frameInfo.contentChecksumFlag = content_checksum_flag;
    (*dctx).frameInfo.blockSizeID = block_size_id;
    (*dctx).maxBlockSize = LZ4F_getBlockSize(block_size_id);
    if content_size_flag != 0 {
        (*dctx).frameInfo.contentSize = readLE64(src_ptr.add(6));
        (*dctx).frameRemainingSize = (*dctx).frameInfo.contentSize;
    }
    if dict_id_flag != 0 {
        (*dctx).frameInfo.dictID = readLE32(src_ptr.add(frame_header_size - 5));
    }

    (*dctx).dStage = DSTAGE_INIT;

    frame_header_size
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_headerSize(src: *const c_void, src_size: usize) -> usize {
    ret_err_if!(src.is_null(), LZ4F_ERROR_srcPtr_wrong);

    if src_size < LZ4F_MIN_SIZE_TO_KNOW_HEADER_LENGTH {
        ret_err!(LZ4F_ERROR_frameHeader_incomplete);
    }

    if (readLE32(src as *const u8) & 0xFFFFFFF0) == LZ4F_MAGIC_SKIPPABLE_START {
        return 8;
    }

    if readLE32(src as *const u8) != LZ4F_MAGICNUMBER {
        ret_err!(LZ4F_ERROR_frameType_unknown);
    }

    {
        let flg = *(src as *const u8).add(4);
        let content_size_flag = ((flg as u32) >> 3) & 0x01;
        let dict_id_flag = (flg as u32) & 0x01;
        MIN_FH_SIZE
            + (if content_size_flag != 0 { 8 } else { 0 })
            + (if dict_id_flag != 0 { 4 } else { 0 })
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_getFrameInfo(
    dctx: *mut LZ4F_dctx,
    frame_info_ptr: *mut LZ4F_frameInfo_t,
    src_buffer: *const c_void,
    src_size_ptr: *mut usize,
) -> LZ4F_errorCode_t {
    if (*dctx).dStage > DSTAGE_STORE_FRAME_HEADER {
        let mut o: usize = 0;
        let mut i: usize = 0;
        *src_size_ptr = 0;
        *frame_info_ptr = (*dctx).frameInfo;
        return LZ4F_decompress(
            dctx,
            ptr::null_mut(),
            &mut o,
            ptr::null(),
            &mut i,
            ptr::null(),
        );
    } else {
        if (*dctx).dStage == DSTAGE_STORE_FRAME_HEADER {
            *src_size_ptr = 0;
            ret_err!(LZ4F_ERROR_frameDecoding_alreadyStarted);
        } else {
            let h_size = LZ4F_headerSize(src_buffer, *src_size_ptr);
            if LZ4F_isError(h_size) != 0 {
                *src_size_ptr = 0;
                return h_size;
            }
            if *src_size_ptr < h_size {
                *src_size_ptr = 0;
                ret_err!(LZ4F_ERROR_frameHeader_incomplete);
            }

            {
                let mut decode_result = LZ4F_decodeHeader(dctx, src_buffer, h_size);
                if LZ4F_isError(decode_result) != 0 {
                    *src_size_ptr = 0;
                } else {
                    *src_size_ptr = decode_result;
                    decode_result = BH_SIZE;
                }
                *frame_info_ptr = (*dctx).frameInfo;
                return decode_result;
            }
        }
    }
}

unsafe fn LZ4F_updateDict(
    dctx: *mut LZ4F_dctx,
    dst_ptr: *const u8,
    dst_size: usize,
    dst_buffer_start: *const u8,
    within_tmp: u32,
) {
    if (*dctx).dictSize == 0 {
        (*dctx).dict = dst_ptr;
    }

    if (*dctx).dict.wrapping_add((*dctx).dictSize) == dst_ptr {
        (*dctx).dictSize += dst_size;
        return;
    }

    if (pdiff(dst_ptr, dst_buffer_start) as usize) + dst_size >= 64 * 1024 {
        (*dctx).dict = dst_buffer_start;
        (*dctx).dictSize = (pdiff(dst_ptr, dst_buffer_start) as usize) + dst_size;
        return;
    }

    if within_tmp != 0 && (*dctx).dict == (*dctx).tmpOutBuffer as *const u8 {
        (*dctx).dictSize += dst_size;
        return;
    }

    if within_tmp != 0 {
        let preserve_size = pdiff((*dctx).tmpOut, (*dctx).tmpOutBuffer) as usize;
        let mut copy_size = (64 * 1024usize).wrapping_sub((*dctx).tmpOutSize);
        let old_dict_end = (*dctx)
            .dict
            .wrapping_add((*dctx).dictSize)
            .wrapping_sub((*dctx).tmpOutStart);
        if (*dctx).tmpOutSize > 64 * 1024 {
            copy_size = 0;
        }
        if copy_size > preserve_size {
            copy_size = preserve_size;
        }

        memcpy_n(
            (*dctx).tmpOutBuffer.wrapping_add(preserve_size - copy_size),
            old_dict_end.wrapping_sub(copy_size),
            copy_size,
        );

        (*dctx).dict = (*dctx).tmpOutBuffer as *const u8;
        (*dctx).dictSize = preserve_size + (*dctx).tmpOutStart + dst_size;
        return;
    }

    if (*dctx).dict == (*dctx).tmpOutBuffer as *const u8 {
        if (*dctx).dictSize + dst_size > (*dctx).maxBufferSize {
            let preserve_size = 64 * 1024 - dst_size;
            memcpy_n(
                (*dctx).tmpOutBuffer,
                (*dctx)
                    .dict
                    .wrapping_add((*dctx).dictSize)
                    .wrapping_sub(preserve_size),
                preserve_size,
            );
            (*dctx).dictSize = preserve_size;
        }
        memcpy_n(
            (*dctx).tmpOutBuffer.wrapping_add((*dctx).dictSize),
            dst_ptr,
            dst_size,
        );
        (*dctx).dictSize += dst_size;
        return;
    }

    /* join dict & dest into tmp */
    {
        let mut preserve_size = 64 * 1024 - dst_size;
        if preserve_size > (*dctx).dictSize {
            preserve_size = (*dctx).dictSize;
        }
        memcpy_n(
            (*dctx).tmpOutBuffer,
            (*dctx)
                .dict
                .wrapping_add((*dctx).dictSize)
                .wrapping_sub(preserve_size),
            preserve_size,
        );
        memcpy_n(
            (*dctx).tmpOutBuffer.wrapping_add(preserve_size),
            dst_ptr,
            dst_size,
        );
        (*dctx).dict = (*dctx).tmpOutBuffer as *const u8;
        (*dctx).dictSize = preserve_size + dst_size;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_decompress(
    dctx: *mut LZ4F_dctx,
    dst_buffer: *mut c_void,
    dst_size_ptr: *mut usize,
    src_buffer: *const c_void,
    src_size_ptr: *mut usize,
    mut decompress_options_ptr: *const LZ4F_decompressOptions_t,
) -> usize {
    let options_null: LZ4F_decompressOptions_t = core::mem::zeroed();
    let src_start = src_buffer as *const u8;
    let src_end = src_start.wrapping_add(*src_size_ptr);
    let mut src_ptr = src_start;
    let dst_start = dst_buffer as *mut u8;
    let dst_end: *mut u8 = if !dst_start.is_null() {
        dst_start.wrapping_add(*dst_size_ptr)
    } else {
        ptr::null_mut()
    };
    let mut dst_ptr = dst_start;
    let mut selected_in: *const u8 = ptr::null();
    let mut do_another_stage = 1u32;
    let mut next_src_size_hint: usize = 1;

    if decompress_options_ptr.is_null() {
        decompress_options_ptr = &options_null;
    }
    *src_size_ptr = 0;
    *dst_size_ptr = 0;
    (*dctx).skipChecksum |= ((*decompress_options_ptr).skipChecksums != 0) as c_int;

    while do_another_stage != 0 {
        let mut label = (*dctx).dStage;
        'sw: loop {
            match label {
                DSTAGE_GET_FRAME_HEADER => {
                    if (pdiff(src_end, src_ptr) as usize) >= MAX_FH_SIZE {
                        let h_size = LZ4F_decodeHeader(
                            dctx,
                            src_ptr as *const c_void,
                            pdiff(src_end, src_ptr) as usize,
                        );
                        fwd_if_error!(h_size);
                        src_ptr = src_ptr.wrapping_add(h_size);
                        break 'sw;
                    }
                    (*dctx).tmpInSize = 0;
                    if pdiff(src_end, src_ptr) == 0 {
                        return MIN_FH_SIZE;
                    }
                    (*dctx).tmpInTarget = MIN_FH_SIZE;
                    (*dctx).dStage = DSTAGE_STORE_FRAME_HEADER;
                    label = DSTAGE_STORE_FRAME_HEADER;
                    continue 'sw;
                }

                DSTAGE_STORE_FRAME_HEADER => {
                    {
                        let a = (*dctx).tmpInTarget - (*dctx).tmpInSize;
                        let b = pdiff(src_end, src_ptr) as usize;
                        let size_to_copy = if a < b { a } else { b };
                        memcpy_n(
                            (*dctx).header.as_mut_ptr().add((*dctx).tmpInSize),
                            src_ptr,
                            size_to_copy,
                        );
                        (*dctx).tmpInSize += size_to_copy;
                        src_ptr = src_ptr.wrapping_add(size_to_copy);
                    }
                    if (*dctx).tmpInSize < (*dctx).tmpInTarget {
                        next_src_size_hint = ((*dctx).tmpInTarget - (*dctx).tmpInSize) + BH_SIZE;
                        do_another_stage = 0;
                        break 'sw;
                    }
                    let r = LZ4F_decodeHeader(
                        dctx,
                        (*dctx).header.as_ptr() as *const c_void,
                        (*dctx).tmpInTarget,
                    );
                    fwd_if_error!(r);
                    break 'sw;
                }

                DSTAGE_INIT => {
                    if (*dctx).frameInfo.contentChecksumFlag != 0 {
                        XXH32_reset(&mut (*dctx).xxh, 0);
                    }
                    {
                        let buffer_needed = (*dctx).maxBlockSize.wrapping_add(
                            if (*dctx).frameInfo.blockMode == LZ4F_BLOCK_LINKED {
                                128 * 1024
                            } else {
                                0
                            },
                        );
                        if buffer_needed > (*dctx).maxBufferSize {
                            (*dctx).maxBufferSize = 0;
                            LZ4F_free((*dctx).tmpIn as *mut c_void, (*dctx).cmem);
                            (*dctx).tmpIn = LZ4F_malloc(
                                (*dctx).maxBlockSize.wrapping_add(BF_SIZE),
                                (*dctx).cmem,
                            ) as *mut u8;
                            ret_err_if!((*dctx).tmpIn.is_null(), LZ4F_ERROR_allocation_failed);
                            LZ4F_free((*dctx).tmpOutBuffer as *mut c_void, (*dctx).cmem);
                            (*dctx).tmpOutBuffer =
                                LZ4F_malloc(buffer_needed, (*dctx).cmem) as *mut u8;
                            ret_err_if!(
                                (*dctx).tmpOutBuffer.is_null(),
                                LZ4F_ERROR_allocation_failed
                            );
                            (*dctx).maxBufferSize = buffer_needed;
                        }
                    }
                    (*dctx).tmpInSize = 0;
                    (*dctx).tmpInTarget = 0;
                    (*dctx).tmpOut = (*dctx).tmpOutBuffer;
                    (*dctx).tmpOutStart = 0;
                    (*dctx).tmpOutSize = 0;

                    (*dctx).dStage = DSTAGE_GET_BLOCK_HEADER;
                    label = DSTAGE_GET_BLOCK_HEADER;
                    continue 'sw;
                }

                DSTAGE_GET_BLOCK_HEADER => {
                    if (pdiff(src_end, src_ptr) as usize) >= BH_SIZE {
                        selected_in = src_ptr;
                        src_ptr = src_ptr.wrapping_add(BH_SIZE);
                    } else {
                        (*dctx).tmpInSize = 0;
                        (*dctx).dStage = DSTAGE_STORE_BLOCK_HEADER;
                    }
                    if (*dctx).dStage == DSTAGE_STORE_BLOCK_HEADER {
                        label = DSTAGE_STORE_BLOCK_HEADER;
                    } else {
                        label = L_DECODE_BLOCK_HEADER;
                    }
                    continue 'sw;
                }

                DSTAGE_STORE_BLOCK_HEADER => {
                    {
                        let remaining_input = pdiff(src_end, src_ptr) as usize;
                        let wanted_data = BH_SIZE - (*dctx).tmpInSize;
                        let size_to_copy = if wanted_data < remaining_input {
                            wanted_data
                        } else {
                            remaining_input
                        };
                        memcpy_n((*dctx).tmpIn.add((*dctx).tmpInSize), src_ptr, size_to_copy);
                        src_ptr = src_ptr.wrapping_add(size_to_copy);
                        (*dctx).tmpInSize += size_to_copy;

                        if (*dctx).tmpInSize < BH_SIZE {
                            next_src_size_hint = BH_SIZE - (*dctx).tmpInSize;
                            do_another_stage = 0;
                            break 'sw;
                        }
                        selected_in = (*dctx).tmpIn;
                    }
                    label = L_DECODE_BLOCK_HEADER;
                    continue 'sw;
                }

                L_DECODE_BLOCK_HEADER => {
                    let block_header = readLE32(selected_in);
                    let next_c_block_size = (block_header & 0x7FFFFFFF) as usize;
                    let crc_size = (*dctx).frameInfo.blockChecksumFlag as usize * BF_SIZE;
                    if block_header == 0 {
                        (*dctx).dStage = DSTAGE_GET_SUFFIX;
                        break 'sw;
                    }
                    if next_c_block_size > (*dctx).maxBlockSize {
                        ret_err!(LZ4F_ERROR_maxBlockSize_invalid);
                    }
                    if (block_header & LZ4F_BLOCKUNCOMPRESSED_FLAG) != 0 {
                        (*dctx).tmpInTarget = next_c_block_size;
                        if (*dctx).frameInfo.blockChecksumFlag != 0 {
                            XXH32_reset(&mut (*dctx).blockChecksum, 0);
                        }
                        (*dctx).dStage = DSTAGE_COPY_DIRECT;
                        break 'sw;
                    }
                    (*dctx).tmpInTarget = next_c_block_size + crc_size;
                    (*dctx).dStage = DSTAGE_GET_CBLOCK;
                    if dst_ptr == dst_end || src_ptr == src_end {
                        next_src_size_hint = BH_SIZE + next_c_block_size + crc_size;
                        do_another_stage = 0;
                    }
                    break 'sw;
                }

                DSTAGE_COPY_DIRECT => {
                    {
                        let size_to_copy: usize;
                        if dst_ptr.is_null() {
                            size_to_copy = 0;
                        } else {
                            let a = pdiff(src_end, src_ptr) as usize;
                            let b = pdiff(dst_end, dst_ptr) as usize;
                            let min_buff_size = if a < b { a } else { b };
                            size_to_copy = if (*dctx).tmpInTarget < min_buff_size {
                                (*dctx).tmpInTarget
                            } else {
                                min_buff_size
                            };
                            memcpy_n(dst_ptr, src_ptr, size_to_copy);
                            if (*dctx).skipChecksum == 0 {
                                if (*dctx).frameInfo.blockChecksumFlag != 0 {
                                    XXH32_update(
                                        &mut (*dctx).blockChecksum,
                                        src_ptr as *const c_void,
                                        size_to_copy,
                                    );
                                }
                                if (*dctx).frameInfo.contentChecksumFlag != 0 {
                                    XXH32_update(
                                        &mut (*dctx).xxh,
                                        src_ptr as *const c_void,
                                        size_to_copy,
                                    );
                                }
                            }
                            if (*dctx).frameInfo.contentSize != 0 {
                                (*dctx).frameRemainingSize = (*dctx)
                                    .frameRemainingSize
                                    .wrapping_sub(size_to_copy as u64);
                            }

                            if (*dctx).frameInfo.blockMode == LZ4F_BLOCK_LINKED {
                                LZ4F_updateDict(dctx, dst_ptr, size_to_copy, dst_start, 0);
                            }
                            src_ptr = src_ptr.wrapping_add(size_to_copy);
                            dst_ptr = dst_ptr.wrapping_add(size_to_copy);
                        }
                        if size_to_copy == (*dctx).tmpInTarget {
                            if (*dctx).frameInfo.blockChecksumFlag != 0 {
                                (*dctx).tmpInSize = 0;
                                (*dctx).dStage = DSTAGE_GET_BLOCK_CHECKSUM;
                            } else {
                                (*dctx).dStage = DSTAGE_GET_BLOCK_HEADER;
                            }
                            break 'sw;
                        }
                        (*dctx).tmpInTarget -= size_to_copy;
                    }
                    next_src_size_hint = (*dctx).tmpInTarget
                        + (if (*dctx).frameInfo.blockChecksumFlag != 0 {
                            BF_SIZE
                        } else {
                            0
                        })
                        + BH_SIZE;
                    do_another_stage = 0;
                    break 'sw;
                }

                DSTAGE_GET_BLOCK_CHECKSUM => {
                    {
                        let crc_src: *const u8;
                        if pdiff(src_end, src_ptr) >= 4 && (*dctx).tmpInSize == 0 {
                            crc_src = src_ptr;
                            src_ptr = src_ptr.wrapping_add(4);
                        } else {
                            let still_to_copy = 4 - (*dctx).tmpInSize;
                            let avail = pdiff(src_end, src_ptr) as usize;
                            let size_to_copy = if still_to_copy < avail {
                                still_to_copy
                            } else {
                                avail
                            };
                            memcpy_n(
                                (*dctx).header.as_mut_ptr().add((*dctx).tmpInSize),
                                src_ptr,
                                size_to_copy,
                            );
                            (*dctx).tmpInSize += size_to_copy;
                            src_ptr = src_ptr.wrapping_add(size_to_copy);
                            if (*dctx).tmpInSize < 4 {
                                do_another_stage = 0;
                                break 'sw;
                            }
                            crc_src = (*dctx).header.as_ptr();
                        }
                        if (*dctx).skipChecksum == 0 {
                            let read_crc = readLE32(crc_src);
                            let calc_crc = XXH32_digest(&(*dctx).blockChecksum);
                            if read_crc != calc_crc {
                                ret_err!(LZ4F_ERROR_blockChecksum_invalid);
                            }
                        }
                    }
                    (*dctx).dStage = DSTAGE_GET_BLOCK_HEADER;
                    break 'sw;
                }

                DSTAGE_GET_CBLOCK => {
                    if (pdiff(src_end, src_ptr) as usize) < (*dctx).tmpInTarget {
                        (*dctx).tmpInSize = 0;
                        (*dctx).dStage = DSTAGE_STORE_CBLOCK;
                        break 'sw;
                    }
                    selected_in = src_ptr;
                    src_ptr = src_ptr.wrapping_add((*dctx).tmpInTarget);
                    label = L_AFTER_CBLOCK;
                    continue 'sw;
                }

                DSTAGE_STORE_CBLOCK => {
                    {
                        let wanted_data = (*dctx).tmpInTarget - (*dctx).tmpInSize;
                        let input_left = pdiff(src_end, src_ptr) as usize;
                        let size_to_copy = if wanted_data < input_left {
                            wanted_data
                        } else {
                            input_left
                        };
                        memcpy_n((*dctx).tmpIn.add((*dctx).tmpInSize), src_ptr, size_to_copy);
                        (*dctx).tmpInSize += size_to_copy;
                        src_ptr = src_ptr.wrapping_add(size_to_copy);
                        if (*dctx).tmpInSize < (*dctx).tmpInTarget {
                            next_src_size_hint = ((*dctx).tmpInTarget - (*dctx).tmpInSize)
                                + (if (*dctx).frameInfo.blockChecksumFlag != 0 {
                                    BF_SIZE
                                } else {
                                    0
                                })
                                + BH_SIZE;
                            do_another_stage = 0;
                            break 'sw;
                        }
                        selected_in = (*dctx).tmpIn;
                    }
                    label = L_AFTER_CBLOCK;
                    continue 'sw;
                }

                L_AFTER_CBLOCK => {
                    /* First, decode and control block checksum if it exists */
                    if (*dctx).frameInfo.blockChecksumFlag != 0 {
                        (*dctx).tmpInTarget -= 4;
                        let read_block_crc = readLE32(selected_in.add((*dctx).tmpInTarget));
                        let calc_block_crc =
                            XXH32(selected_in as *const c_void, (*dctx).tmpInTarget, 0);
                        ret_err_if!(
                            read_block_crc != calc_block_crc,
                            LZ4F_ERROR_blockChecksum_invalid
                        );
                    }

                    /* decode directly into destination buffer if there is enough room */
                    if (pdiff(dst_end, dst_ptr) as usize) >= (*dctx).maxBlockSize
                        && !(!(*dctx).dict.is_null()
                            && (*dctx).dict.wrapping_add((*dctx).dictSize)
                                == (*dctx).tmpOut as *const u8)
                    {
                        let mut dict = (*dctx).dict as *const c_char;
                        let mut dict_size = (*dctx).dictSize;
                        let decoded_size: c_int;
                        if !dict.is_null() && dict_size > (1usize << 30) {
                            dict = dict.wrapping_add(dict_size - 64 * 1024);
                            dict_size = 64 * 1024;
                        }
                        decoded_size = LZ4_decompress_safe_usingDict(
                            selected_in as *const c_char,
                            dst_ptr as *mut c_char,
                            (*dctx).tmpInTarget as c_int,
                            (*dctx).maxBlockSize as c_int,
                            dict,
                            dict_size as c_int,
                        );
                        ret_err_if!(decoded_size < 0, LZ4F_ERROR_decompressionFailed);
                        if (*dctx).frameInfo.contentChecksumFlag != 0 && (*dctx).skipChecksum == 0 {
                            XXH32_update(
                                &mut (*dctx).xxh,
                                dst_ptr as *const c_void,
                                decoded_size as usize,
                            );
                        }
                        if (*dctx).frameInfo.contentSize != 0 {
                            (*dctx).frameRemainingSize = (*dctx)
                                .frameRemainingSize
                                .wrapping_sub(decoded_size as u64);
                        }

                        if (*dctx).frameInfo.blockMode == LZ4F_BLOCK_LINKED {
                            LZ4F_updateDict(dctx, dst_ptr, decoded_size as usize, dst_start, 0);
                        }

                        dst_ptr = dst_ptr.wrapping_add(decoded_size as usize);
                        (*dctx).dStage = DSTAGE_GET_BLOCK_HEADER;
                        break 'sw;
                    }

                    /* not enough place into dst : decode into tmpOut */
                    if (*dctx).frameInfo.blockMode == LZ4F_BLOCK_LINKED {
                        if (*dctx).dict == (*dctx).tmpOutBuffer as *const u8 {
                            if (*dctx).dictSize > 128 * 1024 {
                                memcpy_n(
                                    (*dctx).tmpOutBuffer,
                                    (*dctx)
                                        .dict
                                        .wrapping_add((*dctx).dictSize)
                                        .wrapping_sub(64 * 1024),
                                    64 * 1024,
                                );
                                (*dctx).dictSize = 64 * 1024;
                            }
                            (*dctx).tmpOut = (*dctx).tmpOutBuffer.add((*dctx).dictSize);
                        } else {
                            let reserved_dict_space = if (*dctx).dictSize < 64 * 1024 {
                                (*dctx).dictSize
                            } else {
                                64 * 1024
                            };
                            (*dctx).tmpOut = (*dctx).tmpOutBuffer.add(reserved_dict_space);
                        }
                    }

                    {
                        let mut dict = (*dctx).dict as *const c_char;
                        let mut dict_size = (*dctx).dictSize;
                        let decoded_size: c_int;
                        if !dict.is_null() && dict_size > (1usize << 30) {
                            dict = dict.wrapping_add(dict_size - 64 * 1024);
                            dict_size = 64 * 1024;
                        }
                        decoded_size = LZ4_decompress_safe_usingDict(
                            selected_in as *const c_char,
                            (*dctx).tmpOut as *mut c_char,
                            (*dctx).tmpInTarget as c_int,
                            (*dctx).maxBlockSize as c_int,
                            dict,
                            dict_size as c_int,
                        );
                        ret_err_if!(decoded_size < 0, LZ4F_ERROR_decompressionFailed);
                        if (*dctx).frameInfo.contentChecksumFlag != 0 && (*dctx).skipChecksum == 0 {
                            XXH32_update(
                                &mut (*dctx).xxh,
                                (*dctx).tmpOut as *const c_void,
                                decoded_size as usize,
                            );
                        }
                        if (*dctx).frameInfo.contentSize != 0 {
                            (*dctx).frameRemainingSize = (*dctx)
                                .frameRemainingSize
                                .wrapping_sub(decoded_size as u64);
                        }
                        (*dctx).tmpOutSize = decoded_size as usize;
                        (*dctx).tmpOutStart = 0;
                        (*dctx).dStage = DSTAGE_FLUSH_OUT;
                    }
                    label = DSTAGE_FLUSH_OUT;
                    continue 'sw;
                }

                DSTAGE_FLUSH_OUT => {
                    if !dst_ptr.is_null() {
                        let a = (*dctx).tmpOutSize - (*dctx).tmpOutStart;
                        let b = pdiff(dst_end, dst_ptr) as usize;
                        let size_to_copy = if a < b { a } else { b };
                        memcpy_n(
                            dst_ptr,
                            (*dctx).tmpOut.add((*dctx).tmpOutStart),
                            size_to_copy,
                        );

                        if (*dctx).frameInfo.blockMode == LZ4F_BLOCK_LINKED {
                            LZ4F_updateDict(dctx, dst_ptr, size_to_copy, dst_start, 1);
                        }

                        (*dctx).tmpOutStart += size_to_copy;
                        dst_ptr = dst_ptr.wrapping_add(size_to_copy);
                    }
                    if (*dctx).tmpOutStart == (*dctx).tmpOutSize {
                        (*dctx).dStage = DSTAGE_GET_BLOCK_HEADER;
                        break 'sw;
                    }
                    do_another_stage = 0;
                    next_src_size_hint = BH_SIZE;
                    break 'sw;
                }

                DSTAGE_GET_SUFFIX => {
                    ret_err_if!((*dctx).frameRemainingSize != 0, LZ4F_ERROR_frameSize_wrong);
                    if (*dctx).frameInfo.contentChecksumFlag == 0 {
                        next_src_size_hint = 0;
                        LZ4F_resetDecompressionContext(dctx);
                        do_another_stage = 0;
                        break 'sw;
                    }
                    if pdiff(src_end, src_ptr) < 4 {
                        (*dctx).tmpInSize = 0;
                        (*dctx).dStage = DSTAGE_STORE_SUFFIX;
                    } else {
                        selected_in = src_ptr;
                        src_ptr = src_ptr.wrapping_add(4);
                    }
                    if (*dctx).dStage == DSTAGE_STORE_SUFFIX {
                        label = DSTAGE_STORE_SUFFIX;
                    } else {
                        label = L_CHECK_SUFFIX;
                    }
                    continue 'sw;
                }

                DSTAGE_STORE_SUFFIX => {
                    {
                        let remaining_input = pdiff(src_end, src_ptr) as usize;
                        let wanted_data = 4 - (*dctx).tmpInSize;
                        let size_to_copy = if wanted_data < remaining_input {
                            wanted_data
                        } else {
                            remaining_input
                        };
                        memcpy_n((*dctx).tmpIn.add((*dctx).tmpInSize), src_ptr, size_to_copy);
                        src_ptr = src_ptr.wrapping_add(size_to_copy);
                        (*dctx).tmpInSize += size_to_copy;
                        if (*dctx).tmpInSize < 4 {
                            next_src_size_hint = 4 - (*dctx).tmpInSize;
                            do_another_stage = 0;
                            break 'sw;
                        }
                        selected_in = (*dctx).tmpIn;
                    }
                    label = L_CHECK_SUFFIX;
                    continue 'sw;
                }

                L_CHECK_SUFFIX => {
                    if (*dctx).skipChecksum == 0 {
                        let read_crc = readLE32(selected_in);
                        let result_crc = XXH32_digest(&(*dctx).xxh);
                        ret_err_if!(read_crc != result_crc, LZ4F_ERROR_contentChecksum_invalid);
                    }
                    next_src_size_hint = 0;
                    LZ4F_resetDecompressionContext(dctx);
                    do_another_stage = 0;
                    break 'sw;
                }

                DSTAGE_GET_SFRAME_SIZE => {
                    if pdiff(src_end, src_ptr) >= 4 {
                        selected_in = src_ptr;
                        src_ptr = src_ptr.wrapping_add(4);
                    } else {
                        (*dctx).tmpInSize = 4;
                        (*dctx).tmpInTarget = 8;
                        (*dctx).dStage = DSTAGE_STORE_SFRAME_SIZE;
                    }
                    if (*dctx).dStage == DSTAGE_STORE_SFRAME_SIZE {
                        label = DSTAGE_STORE_SFRAME_SIZE;
                    } else {
                        label = L_DECODE_SFRAME_SIZE;
                    }
                    continue 'sw;
                }

                DSTAGE_STORE_SFRAME_SIZE => {
                    {
                        let a = (*dctx).tmpInTarget - (*dctx).tmpInSize;
                        let b = pdiff(src_end, src_ptr) as usize;
                        let size_to_copy = if a < b { a } else { b };
                        memcpy_n(
                            (*dctx).header.as_mut_ptr().add((*dctx).tmpInSize),
                            src_ptr,
                            size_to_copy,
                        );
                        src_ptr = src_ptr.wrapping_add(size_to_copy);
                        (*dctx).tmpInSize += size_to_copy;
                        if (*dctx).tmpInSize < (*dctx).tmpInTarget {
                            next_src_size_hint = (*dctx).tmpInTarget - (*dctx).tmpInSize;
                            do_another_stage = 0;
                            break 'sw;
                        }
                        selected_in = (*dctx).header.as_ptr().add(4);
                    }
                    label = L_DECODE_SFRAME_SIZE;
                    continue 'sw;
                }

                L_DECODE_SFRAME_SIZE => {
                    let s_frame_size = readLE32(selected_in) as usize;
                    (*dctx).frameInfo.contentSize = s_frame_size as u64;
                    (*dctx).tmpInTarget = s_frame_size;
                    (*dctx).dStage = DSTAGE_SKIP_SKIPPABLE;
                    break 'sw;
                }

                DSTAGE_SKIP_SKIPPABLE => {
                    let avail = pdiff(src_end, src_ptr) as usize;
                    let skip_size = if (*dctx).tmpInTarget < avail {
                        (*dctx).tmpInTarget
                    } else {
                        avail
                    };
                    src_ptr = src_ptr.wrapping_add(skip_size);
                    (*dctx).tmpInTarget -= skip_size;
                    do_another_stage = 0;
                    next_src_size_hint = (*dctx).tmpInTarget;
                    if next_src_size_hint != 0 {
                        break 'sw;
                    }
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
    if (*dctx).frameInfo.blockMode == LZ4F_BLOCK_LINKED
        && (*dctx).dict != (*dctx).tmpOutBuffer as *const u8
        && !(*dctx).dict.is_null()
        && (*decompress_options_ptr).stableDst == 0
        && ((*dctx).dStage.wrapping_sub(2) < DSTAGE_GET_SUFFIX - 2)
    {
        if (*dctx).dStage == DSTAGE_FLUSH_OUT {
            let preserve_size = pdiff((*dctx).tmpOut, (*dctx).tmpOutBuffer) as usize;
            let mut copy_size = (64 * 1024usize).wrapping_sub((*dctx).tmpOutSize);
            let old_dict_end = (*dctx)
                .dict
                .wrapping_add((*dctx).dictSize)
                .wrapping_sub((*dctx).tmpOutStart);
            if (*dctx).tmpOutSize > 64 * 1024 {
                copy_size = 0;
            }
            if copy_size > preserve_size {
                copy_size = preserve_size;
            }

            memcpy_n(
                (*dctx).tmpOutBuffer.wrapping_add(preserve_size - copy_size),
                old_dict_end.wrapping_sub(copy_size),
                copy_size,
            );

            (*dctx).dict = (*dctx).tmpOutBuffer as *const u8;
            (*dctx).dictSize = preserve_size + (*dctx).tmpOutStart;
        } else {
            let old_dict_end = (*dctx).dict.wrapping_add((*dctx).dictSize);
            let new_dict_size = if (*dctx).dictSize < 64 * 1024 {
                (*dctx).dictSize
            } else {
                64 * 1024
            };

            memcpy_n(
                (*dctx).tmpOutBuffer,
                old_dict_end.wrapping_sub(new_dict_size),
                new_dict_size,
            );

            (*dctx).dict = (*dctx).tmpOutBuffer as *const u8;
            (*dctx).dictSize = new_dict_size;
            (*dctx).tmpOut = (*dctx).tmpOutBuffer.wrapping_add(new_dict_size);
        }
    }

    *src_size_ptr = pdiff(src_ptr, src_start) as usize;
    *dst_size_ptr = pdiff(dst_ptr, dst_start) as usize;
    next_src_size_hint
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_decompress_usingDict(
    dctx: *mut LZ4F_dctx,
    dst_buffer: *mut c_void,
    dst_size_ptr: *mut usize,
    src_buffer: *const c_void,
    src_size_ptr: *mut usize,
    dict: *const c_void,
    dict_size: usize,
    decompress_options_ptr: *const LZ4F_decompressOptions_t,
) -> usize {
    if (*dctx).dStage <= DSTAGE_INIT {
        (*dctx).dict = dict as *const u8;
        (*dctx).dictSize = dict_size;
    }
    LZ4F_decompress(
        dctx,
        dst_buffer,
        dst_size_ptr,
        src_buffer,
        src_size_ptr,
        decompress_options_ptr,
    )
}
