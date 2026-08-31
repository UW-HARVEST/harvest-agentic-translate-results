//! Translation of `lz4frame.c` (LZ4 1.10.0), built with `LZ4F_HEAPMODE=0`.

use core::ffi::{c_char, c_int, c_uint, c_void};

use crate::lz4;
use crate::lz4hc;
use crate::util::*;
use crate::xxhash::{self, XXH32State};

/* ===== error codes ===== */

pub const LZ4F_OK_NoError: usize = 0;
pub const LZ4F_ERROR_GENERIC: usize = 1;
pub const LZ4F_ERROR_maxBlockSize_invalid: usize = 2;
pub const LZ4F_ERROR_blockMode_invalid: usize = 3;
pub const LZ4F_ERROR_parameter_invalid: usize = 4;
pub const LZ4F_ERROR_compressionLevel_invalid: usize = 5;
pub const LZ4F_ERROR_headerVersion_wrong: usize = 6;
pub const LZ4F_ERROR_blockChecksum_invalid: usize = 7;
pub const LZ4F_ERROR_reservedFlag_set: usize = 8;
pub const LZ4F_ERROR_allocation_failed: usize = 9;
pub const LZ4F_ERROR_srcSize_tooLarge: usize = 10;
pub const LZ4F_ERROR_dstMaxSize_tooSmall: usize = 11;
pub const LZ4F_ERROR_frameHeader_incomplete: usize = 12;
pub const LZ4F_ERROR_frameType_unknown: usize = 13;
pub const LZ4F_ERROR_frameSize_wrong: usize = 14;
pub const LZ4F_ERROR_srcPtr_wrong: usize = 15;
pub const LZ4F_ERROR_decompressionFailed: usize = 16;
pub const LZ4F_ERROR_headerChecksum_invalid: usize = 17;
pub const LZ4F_ERROR_contentChecksum_invalid: usize = 18;
pub const LZ4F_ERROR_frameDecoding_alreadyStarted: usize = 19;
pub const LZ4F_ERROR_compressionState_uninitialized: usize = 20;
pub const LZ4F_ERROR_parameter_null: usize = 21;
pub const LZ4F_ERROR_io_write: usize = 22;
pub const LZ4F_ERROR_io_read: usize = 23;
pub const LZ4F_ERROR_maxCode: usize = 24;

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
fn ret_err(code: usize) -> usize {
    (0usize).wrapping_sub(code)
}

#[inline]
fn is_error(code: usize) -> bool {
    code > ret_err(LZ4F_ERROR_maxCode)
}

/* ===== public types ===== */

pub const LZ4F_VERSION: c_uint = 100;

pub const LZ4F_HEADER_SIZE_MIN: usize = 7;
pub const LZ4F_HEADER_SIZE_MAX: usize = 19;
pub const LZ4F_BLOCK_HEADER_SIZE: usize = 4;
pub const LZ4F_BLOCK_CHECKSUM_SIZE: usize = 4;
pub const LZ4F_MAGICNUMBER: u32 = 0x184D_2204;
pub const LZ4F_MAGIC_SKIPPABLE_START: u32 = 0x184D_2A50;
pub const LZ4F_MIN_SIZE_TO_KNOW_HEADER_LENGTH: usize = 5;

const MIN_FH_SIZE: usize = LZ4F_HEADER_SIZE_MIN;
const MAX_FH_SIZE: usize = LZ4F_HEADER_SIZE_MAX;
const BH_SIZE: usize = LZ4F_BLOCK_HEADER_SIZE;
const BF_SIZE: usize = LZ4F_BLOCK_CHECKSUM_SIZE;

const LZ4F_BLOCKUNCOMPRESSED_FLAG: u32 = 0x8000_0000;
const LZ4F_BLOCKSIZEID_DEFAULT: c_int = 4; /* LZ4F_max64KB */

pub const LZ4F_max64KB: c_int = 4;
pub const LZ4F_max4MB: c_int = 7;
pub const LZ4F_blockLinked: c_int = 0;
pub const LZ4F_blockIndependent: c_int = 1;
pub const LZ4F_noContentChecksum: c_int = 0;
pub const LZ4F_contentChecksumEnabled: c_int = 1;
pub const LZ4F_noBlockChecksum: c_int = 0;
pub const LZ4F_blockChecksumEnabled: c_int = 1;
pub const LZ4F_frame: c_int = 0;
pub const LZ4F_skippableFrame: c_int = 1;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct LZ4F_frameInfo_t {
    pub blockSizeID: c_int,
    pub blockMode: c_int,
    pub contentChecksumFlag: c_int,
    pub frameType: c_int,
    pub contentSize: u64,
    pub dictID: c_uint,
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
    /// `LZ4F_INIT_FRAMEINFO`
    const fn init() -> Self {
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
    const fn zeroed() -> Self {
        LZ4F_preferences_t {
            frameInfo: LZ4F_frameInfo_t::zeroed(),
            compressionLevel: 0,
            autoFlush: 0,
            favorDecSpeed: 0,
            reserved: [0; 3],
        }
    }
    /// `LZ4F_INIT_PREFERENCES`
    const fn init() -> Self {
        LZ4F_preferences_t {
            frameInfo: LZ4F_frameInfo_t::init(),
            compressionLevel: 0,
            autoFlush: 0,
            favorDecSpeed: 0,
            reserved: [0; 3],
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

const DEFAULT_CMEM: LZ4F_CustomMem = LZ4F_CustomMem {
    customAlloc: None,
    customCalloc: None,
    customFree: None,
    opaqueState: core::ptr::null_mut(),
};

const _: () = assert!(core::mem::size_of::<LZ4F_frameInfo_t>() == 32);
const _: () = assert!(core::mem::size_of::<LZ4F_preferences_t>() == 56);
const _: () = assert!(core::mem::size_of::<LZ4F_CustomMem>() == 32);

/* ===== memory helpers ===== */

unsafe fn f_calloc(s: usize, cmem: LZ4F_CustomMem) -> *mut c_void {
    unsafe {
        if let Some(cc) = cmem.customCalloc {
            return cc(cmem.opaqueState, s);
        }
        if cmem.customAlloc.is_none() {
            return alloc_and_zero(s);
        }
        let p = (cmem.customAlloc.unwrap())(cmem.opaqueState, s);
        if !p.is_null() {
            mem_init(p as *mut u8, 0, s);
        }
        p
    }
}

unsafe fn f_malloc(s: usize, cmem: LZ4F_CustomMem) -> *mut c_void {
    unsafe {
        if let Some(ca) = cmem.customAlloc {
            return ca(cmem.opaqueState, s);
        }
        malloc(s)
    }
}

unsafe fn f_free(p: *mut c_void, cmem: LZ4F_CustomMem) {
    unsafe {
        if p.is_null() {
            return;
        }
        if let Some(cf) = cmem.customFree {
            cf(cmem.opaqueState, p);
            return;
        }
        free(p);
    }
}

/* ===== little-endian helpers (byte-at-a-time, as in the C) ===== */

#[inline]
unsafe fn read_le32(src: *const u8) -> u32 {
    unsafe {
        (*src as u32)
            | ((*src.add(1) as u32) << 8)
            | ((*src.add(2) as u32) << 16)
            | ((*src.add(3) as u32) << 24)
    }
}

#[inline]
unsafe fn write_le32(dst: *mut u8, v: u32) {
    unsafe {
        *dst = v as u8;
        *dst.add(1) = (v >> 8) as u8;
        *dst.add(2) = (v >> 16) as u8;
        *dst.add(3) = (v >> 24) as u8;
    }
}

#[inline]
unsafe fn read_le64(src: *const u8) -> u64 {
    unsafe {
        let mut v = *src as u64;
        for i in 1..8 {
            v |= (*src.add(i) as u64) << (8 * i);
        }
        v
    }
}

#[inline]
unsafe fn write_le64(dst: *mut u8, v: u64) {
    unsafe {
        for i in 0..8 {
            *dst.add(i) = (v >> (8 * i)) as u8;
        }
    }
}

/* ===== compression context ===== */

const CTX_NONE: u16 = 0;
const CTX_FAST: u16 = 1;
const CTX_HC: u16 = 2;

const LZ4B_COMPRESSED: u32 = 0;
const LZ4B_UNCOMPRESSED: u32 = 1;

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
    pub xxh: XXH32State,
    pub lz4CtxPtr: *mut c_void,
    pub lz4CtxAlloc: u16,
    pub lz4CtxType: u16,
    pub blockCompressMode: u32,
}

#[repr(C)]
pub struct LZ4F_CDict {
    pub cmem: LZ4F_CustomMem,
    pub dictContent: *mut c_void,
    pub fastCtx: *mut lz4::LZ4Stream,
    pub HCCtx: *mut lz4hc::LZ4_streamHC_t,
}

/* ===== basic API ===== */

#[unsafe(no_mangle)]
pub extern "C" fn LZ4F_isError(code: usize) -> c_uint {
    is_error(code) as c_uint
}

#[unsafe(no_mangle)]
pub extern "C" fn LZ4F_getErrorName(code: usize) -> *const c_char {
    if is_error(code) {
        let idx = (0usize).wrapping_sub(code);
        return ERROR_STRINGS[idx].as_ptr() as *const c_char;
    }
    CODE_ERROR.as_ptr() as *const c_char
}

#[unsafe(no_mangle)]
pub extern "C" fn LZ4F_getErrorCode(function_result: usize) -> c_int {
    if !is_error(function_result) {
        return LZ4F_OK_NoError as c_int;
    }
    (0isize - function_result as isize) as c_int
}

#[unsafe(no_mangle)]
pub extern "C" fn LZ4F_getVersion() -> c_uint {
    LZ4F_VERSION
}

#[unsafe(no_mangle)]
pub extern "C" fn LZ4F_compressionLevel_max() -> c_int {
    lz4hc::LZ4HC_CLEVEL_MAX
}

#[unsafe(no_mangle)]
pub extern "C" fn LZ4F_getBlockSize(block_size_id: c_int) -> usize {
    static BLOCK_SIZES: [usize; 4] = [64 * 1024, 256 * 1024, 1024 * 1024, 4 * 1024 * 1024];
    let mut block_size_id = block_size_id;
    if block_size_id == 0 {
        block_size_id = LZ4F_BLOCKSIZEID_DEFAULT;
    }
    if block_size_id < LZ4F_max64KB || block_size_id > LZ4F_max4MB {
        return ret_err(LZ4F_ERROR_maxBlockSize_invalid);
    }
    BLOCK_SIZES[(block_size_id - LZ4F_max64KB) as usize]
}

unsafe fn header_checksum(header: *const u8, length: usize) -> u8 {
    unsafe { (xxhash::xxh32(header, length, 0) >> 8) as u8 }
}

/* ===== bounds ===== */

fn optimal_bsid(requested_bsid: c_int, src_size: usize) -> c_int {
    let mut proposed_bsid = LZ4F_max64KB;
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

unsafe fn compress_bound_internal(
    src_size: usize,
    preferences_ptr: *const LZ4F_preferences_t,
    already_buffered: usize,
) -> usize {
    unsafe {
        let mut prefs_null = LZ4F_preferences_t::init();
        prefs_null.frameInfo.contentChecksumFlag = LZ4F_contentChecksumEnabled;
        prefs_null.frameInfo.blockChecksumFlag = LZ4F_blockChecksumEnabled;
        let prefs = if preferences_ptr.is_null() {
            &prefs_null as *const LZ4F_preferences_t
        } else {
            preferences_ptr
        };
        let flush = (*prefs).autoFlush | ((src_size == 0) as c_uint);
        let block_id = (*prefs).frameInfo.blockSizeID;
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
        let nb_blocks = nb_full_blocks + ((last_block_size > 0) as u32);

        let block_crc_size = BF_SIZE * (*prefs).frameInfo.blockChecksumFlag as usize;
        let frame_end = BH_SIZE + ((*prefs).frameInfo.contentChecksumFlag as usize * BF_SIZE);

        ((BH_SIZE + block_crc_size) * nb_blocks as usize)
            + (block_size * nb_full_blocks as usize)
            + last_block_size
            + frame_end
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_compressFrameBound(
    src_size: usize,
    preferences_ptr: *const LZ4F_preferences_t,
) -> usize {
    unsafe {
        let mut prefs = if !preferences_ptr.is_null() {
            *preferences_ptr
        } else {
            LZ4F_preferences_t::zeroed()
        };
        prefs.autoFlush = 1;
        MAX_FH_SIZE + compress_bound_internal(src_size, &prefs, 0)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_compressBound(
    src_size: usize,
    preferences_ptr: *const LZ4F_preferences_t,
) -> usize {
    unsafe {
        if !preferences_ptr.is_null() && (*preferences_ptr).autoFlush != 0 {
            return compress_bound_internal(src_size, preferences_ptr, 0);
        }
        compress_bound_internal(src_size, preferences_ptr, usize::MAX)
    }
}

/* ===== dictionary compression ===== */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_createCDict_advanced(
    cmem: LZ4F_CustomMem,
    dict_buffer: *const c_void,
    dict_size: usize,
) -> *mut LZ4F_CDict {
    unsafe {
        let mut dict_start = dict_buffer as *const c_char;
        let mut dict_size = dict_size;
        let cdict = f_malloc(core::mem::size_of::<LZ4F_CDict>(), cmem) as *mut LZ4F_CDict;
        if cdict.is_null() {
            return core::ptr::null_mut();
        }
        (*cdict).cmem = cmem;
        if dict_size > 64 * 1024 {
            dict_start = dict_start.add(dict_size - 64 * 1024);
            dict_size = 64 * 1024;
        }
        (*cdict).dictContent = f_malloc(dict_size, cmem);
        (*cdict).fastCtx =
            f_malloc(core::mem::size_of::<lz4::LZ4Stream>(), cmem) as *mut lz4::LZ4Stream;
        (*cdict).HCCtx = f_malloc(core::mem::size_of::<lz4hc::LZ4_streamHC_t>(), cmem)
            as *mut lz4hc::LZ4_streamHC_t;
        if (*cdict).dictContent.is_null()
            || (*cdict).fastCtx.is_null()
            || (*cdict).HCCtx.is_null()
        {
            LZ4F_freeCDict(cdict);
            return core::ptr::null_mut();
        }
        mem_copy(
            (*cdict).dictContent as *mut u8,
            dict_start as *const u8,
            dict_size,
        );
        lz4::LZ4_initStream(
            (*cdict).fastCtx as *mut c_void,
            core::mem::size_of::<lz4::LZ4Stream>(),
        );
        lz4::LZ4_loadDictSlow(
            (*cdict).fastCtx,
            (*cdict).dictContent as *const c_char,
            dict_size as c_int,
        );
        lz4hc::LZ4_initStreamHC(
            (*cdict).HCCtx as *mut c_void,
            core::mem::size_of::<lz4hc::LZ4_streamHC_t>(),
        );
        lz4hc::LZ4_setCompressionLevel((*cdict).HCCtx, lz4hc::LZ4HC_CLEVEL_DEFAULT);
        lz4hc::LZ4_loadDictHC(
            (*cdict).HCCtx,
            (*cdict).dictContent as *const c_char,
            dict_size as c_int,
        );
        cdict
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_createCDict(
    dict_buffer: *const c_void,
    dict_size: usize,
) -> *mut LZ4F_CDict {
    unsafe { LZ4F_createCDict_advanced(DEFAULT_CMEM, dict_buffer, dict_size) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_freeCDict(cdict: *mut LZ4F_CDict) {
    unsafe {
        if cdict.is_null() {
            return;
        }
        let cmem = (*cdict).cmem;
        f_free((*cdict).dictContent, cmem);
        f_free((*cdict).fastCtx as *mut c_void, cmem);
        f_free((*cdict).HCCtx as *mut c_void, cmem);
        f_free(cdict as *mut c_void, cmem);
    }
}

/* ===== compression context management ===== */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_createCompressionContext_advanced(
    custom_mem: LZ4F_CustomMem,
    version: c_uint,
) -> *mut LZ4F_cctx {
    unsafe {
        let cctx = f_calloc(core::mem::size_of::<LZ4F_cctx>(), custom_mem) as *mut LZ4F_cctx;
        if cctx.is_null() {
            return core::ptr::null_mut();
        }
        (*cctx).cmem = custom_mem;
        (*cctx).version = version;
        (*cctx).cStage = 0;
        cctx
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_createCompressionContext(
    cctx_ptr: *mut *mut LZ4F_cctx,
    version: c_uint,
) -> usize {
    unsafe {
        if cctx_ptr.is_null() {
            return ret_err(LZ4F_ERROR_parameter_null);
        }
        *cctx_ptr = LZ4F_createCompressionContext_advanced(DEFAULT_CMEM, version);
        if (*cctx_ptr).is_null() {
            return ret_err(LZ4F_ERROR_allocation_failed);
        }
        LZ4F_OK_NoError
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_freeCompressionContext(cctx: *mut LZ4F_cctx) -> usize {
    unsafe {
        if !cctx.is_null() {
            let cmem = (*cctx).cmem;
            f_free((*cctx).lz4CtxPtr, cmem);
            f_free((*cctx).tmpBuff as *mut c_void, cmem);
            f_free(cctx as *mut c_void, cmem);
        }
        LZ4F_OK_NoError
    }
}

unsafe fn f_init_stream(
    ctx: *mut c_void,
    cdict: *const LZ4F_CDict,
    level: c_int,
    block_mode: c_int,
) {
    unsafe {
        if level < lz4hc::LZ4HC_CLEVEL_MIN {
            if !cdict.is_null() || block_mode == LZ4F_blockLinked {
                lz4::LZ4_resetStream_fast(ctx as *mut lz4::LZ4Stream);
                if !cdict.is_null() {
                    lz4::LZ4_attach_dictionary(
                        ctx as *mut lz4::LZ4Stream,
                        (*cdict).fastCtx as *const lz4::LZ4Stream,
                    );
                }
            }
        } else {
            lz4hc::LZ4_resetStreamHC_fast(ctx as *mut lz4hc::LZ4_streamHC_t, level);
            if !cdict.is_null() {
                lz4hc::LZ4_attach_HC_dictionary(
                    ctx as *mut lz4hc::LZ4_streamHC_t,
                    (*cdict).HCCtx as *const lz4hc::LZ4_streamHC_t,
                );
            }
        }
    }
}

fn ctx_type_id_to_size(ctx_type_id: c_int) -> c_int {
    match ctx_type_id {
        1 => lz4::LZ4_sizeofState(),
        2 => lz4hc::LZ4_sizeofStateHC(),
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
    preferences_ptr: *const LZ4F_preferences_t,
) -> usize {
    unsafe {
        let pref_null = LZ4F_preferences_t::init();
        let dst_start = dst_buffer as *mut u8;
        let mut dst_ptr = dst_start;

        if dst_capacity < MAX_FH_SIZE {
            return ret_err(LZ4F_ERROR_dstMaxSize_tooSmall);
        }
        let preferences_ptr = if preferences_ptr.is_null() {
            &pref_null as *const LZ4F_preferences_t
        } else {
            preferences_ptr
        };
        (*cctx).prefs = *preferences_ptr;

        /* cctx Management */
        {
            let ctx_type_id: u16 = if (*cctx).prefs.compressionLevel < lz4hc::LZ4HC_CLEVEL_MIN {
                1
            } else {
                2
            };
            let required_size = ctx_type_id_to_size(ctx_type_id as c_int);
            let allocated_size = ctx_type_id_to_size((*cctx).lz4CtxAlloc as c_int);
            if allocated_size < required_size {
                f_free((*cctx).lz4CtxPtr, (*cctx).cmem);
                if (*cctx).prefs.compressionLevel < lz4hc::LZ4HC_CLEVEL_MIN {
                    (*cctx).lz4CtxPtr =
                        f_malloc(core::mem::size_of::<lz4::LZ4Stream>(), (*cctx).cmem);
                    if !(*cctx).lz4CtxPtr.is_null() {
                        lz4::LZ4_initStream(
                            (*cctx).lz4CtxPtr,
                            core::mem::size_of::<lz4::LZ4Stream>(),
                        );
                    }
                } else {
                    (*cctx).lz4CtxPtr =
                        f_malloc(core::mem::size_of::<lz4hc::LZ4_streamHC_t>(), (*cctx).cmem);
                    if !(*cctx).lz4CtxPtr.is_null() {
                        lz4hc::LZ4_initStreamHC(
                            (*cctx).lz4CtxPtr,
                            core::mem::size_of::<lz4hc::LZ4_streamHC_t>(),
                        );
                    }
                }
                if (*cctx).lz4CtxPtr.is_null() {
                    return ret_err(LZ4F_ERROR_allocation_failed);
                }
                (*cctx).lz4CtxAlloc = ctx_type_id;
                (*cctx).lz4CtxType = ctx_type_id;
            } else if (*cctx).lz4CtxType != ctx_type_id {
                if (*cctx).prefs.compressionLevel < lz4hc::LZ4HC_CLEVEL_MIN {
                    lz4::LZ4_initStream(
                        (*cctx).lz4CtxPtr,
                        core::mem::size_of::<lz4::LZ4Stream>(),
                    );
                } else {
                    lz4hc::LZ4_initStreamHC(
                        (*cctx).lz4CtxPtr,
                        core::mem::size_of::<lz4hc::LZ4_streamHC_t>(),
                    );
                    lz4hc::LZ4_setCompressionLevel(
                        (*cctx).lz4CtxPtr as *mut lz4hc::LZ4_streamHC_t,
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
            let required_buff_size = if (*preferences_ptr).autoFlush != 0 {
                if (*cctx).prefs.frameInfo.blockMode == LZ4F_blockLinked {
                    64 * 1024
                } else {
                    0
                }
            } else {
                (*cctx).maxBlockSize
                    + if (*cctx).prefs.frameInfo.blockMode == LZ4F_blockLinked {
                        128 * 1024
                    } else {
                        0
                    }
            };

            if (*cctx).maxBufferSize < required_buff_size {
                (*cctx).maxBufferSize = 0;
                f_free((*cctx).tmpBuff as *mut c_void, (*cctx).cmem);
                (*cctx).tmpBuff = f_malloc(required_buff_size, (*cctx).cmem) as *mut u8;
                if (*cctx).tmpBuff.is_null() {
                    return ret_err(LZ4F_ERROR_allocation_failed);
                }
                (*cctx).maxBufferSize = required_buff_size;
            }
        }
        (*cctx).tmpIn = (*cctx).tmpBuff;
        (*cctx).tmpInSize = 0;
        xxhash::xxh32_reset(&mut (*cctx).xxh, 0);

        /* context init */
        (*cctx).cdict = cdict;
        if (*cctx).prefs.frameInfo.blockMode == LZ4F_blockLinked {
            f_init_stream(
                (*cctx).lz4CtxPtr,
                cdict,
                (*cctx).prefs.compressionLevel,
                LZ4F_blockLinked,
            );
        }
        if (*preferences_ptr).compressionLevel >= lz4hc::LZ4HC_CLEVEL_MIN {
            lz4hc::LZ4_favorDecompressionSpeed(
                (*cctx).lz4CtxPtr as *mut lz4hc::LZ4_streamHC_t,
                (*preferences_ptr).favorDecSpeed as c_int,
            );
        }
        if !dict_buffer.is_null() {
            if dict_size > c_int::MAX as usize {
                return ret_err(LZ4F_ERROR_parameter_invalid);
            }
            if (*cctx).lz4CtxType == CTX_FAST {
                lz4::LZ4_loadDict(
                    (*cctx).lz4CtxPtr as *mut lz4::LZ4Stream,
                    dict_buffer as *const c_char,
                    dict_size as c_int,
                );
            } else {
                lz4hc::LZ4_loadDictHC(
                    (*cctx).lz4CtxPtr as *mut lz4hc::LZ4_streamHC_t,
                    dict_buffer as *const c_char,
                    dict_size as c_int,
                );
            }
        }

        /* Stage 2 : Write Frame Header */
        write_le32(dst_ptr, LZ4F_MAGICNUMBER);
        dst_ptr = dst_ptr.add(4);
        {
            let header_start = dst_ptr;

            /* FLG Byte */
            *dst_ptr = (((1 & 0x03) << 6)
                + (((*cctx).prefs.frameInfo.blockMode & 0x01) << 5)
                + (((*cctx).prefs.frameInfo.blockChecksumFlag & 0x01) << 4)
                + ((((*cctx).prefs.frameInfo.contentSize > 0) as c_int) << 3)
                + (((*cctx).prefs.frameInfo.contentChecksumFlag & 0x01) << 2)
                + (((*cctx).prefs.frameInfo.dictID > 0) as c_int)) as u8;
            dst_ptr = dst_ptr.add(1);
            /* BD Byte */
            *dst_ptr = (((*cctx).prefs.frameInfo.blockSizeID & 0x07) << 4) as u8;
            dst_ptr = dst_ptr.add(1);
            /* Optional Frame content size field */
            if (*cctx).prefs.frameInfo.contentSize != 0 {
                write_le64(dst_ptr, (*cctx).prefs.frameInfo.contentSize);
                dst_ptr = dst_ptr.add(8);
                (*cctx).totalInSize = 0;
            }
            /* Optional dictionary ID field */
            if (*cctx).prefs.frameInfo.dictID != 0 {
                write_le32(dst_ptr, (*cctx).prefs.frameInfo.dictID);
                dst_ptr = dst_ptr.add(4);
            }
            /* Header CRC Byte */
            *dst_ptr = header_checksum(header_start, dst_ptr as usize - header_start as usize);
            dst_ptr = dst_ptr.add(1);
        }

        (*cctx).cStage = 1;
        dst_ptr as usize - dst_start as usize
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_compressBegin(
    cctx: *mut LZ4F_cctx,
    dst_buffer: *mut c_void,
    dst_capacity: usize,
    preferences_ptr: *const LZ4F_preferences_t,
) -> usize {
    unsafe {
        LZ4F_compressBegin_internal(
            cctx,
            dst_buffer,
            dst_capacity,
            core::ptr::null(),
            0,
            core::ptr::null(),
            preferences_ptr,
        )
    }
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
    unsafe {
        LZ4F_compressBegin_internal(
            cctx,
            dst_buffer,
            dst_capacity,
            dict,
            dict_size,
            core::ptr::null(),
            preferences_ptr,
        )
    }
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
    unsafe {
        LZ4F_compressBegin_usingDictOnce(
            cctx,
            dst_buffer,
            dst_capacity,
            dict,
            dict_size,
            preferences_ptr,
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_compressBegin_usingCDict(
    cctx: *mut LZ4F_cctx,
    dst_buffer: *mut c_void,
    dst_capacity: usize,
    cdict: *const LZ4F_CDict,
    preferences_ptr: *const LZ4F_preferences_t,
) -> usize {
    unsafe {
        LZ4F_compressBegin_internal(
            cctx,
            dst_buffer,
            dst_capacity,
            core::ptr::null(),
            0,
            cdict,
            preferences_ptr,
        )
    }
}

/* ===== block compression ===== */

#[derive(Clone, Copy, PartialEq, Eq)]
enum CompressFn {
    Block,
    BlockContinue,
    BlockHC,
    BlockHCContinue,
    DoNotCompress,
}

unsafe fn call_compress(
    f: CompressFn,
    ctx: *mut c_void,
    src: *const c_char,
    dst: *mut c_char,
    src_size: c_int,
    dst_capacity: c_int,
    level: c_int,
    cdict: *const LZ4F_CDict,
) -> c_int {
    unsafe {
        match f {
            CompressFn::Block => {
                let acceleration = if level < 0 { -level + 1 } else { 1 };
                f_init_stream(ctx, cdict, level, LZ4F_blockIndependent);
                if !cdict.is_null() {
                    lz4::LZ4_compress_fast_continue(
                        ctx as *mut lz4::LZ4Stream,
                        src,
                        dst,
                        src_size,
                        dst_capacity,
                        acceleration,
                    )
                } else {
                    lz4::LZ4_compress_fast_extState_fastReset(
                        ctx,
                        src,
                        dst,
                        src_size,
                        dst_capacity,
                        acceleration,
                    )
                }
            }
            CompressFn::BlockContinue => {
                let acceleration = if level < 0 { -level + 1 } else { 1 };
                lz4::LZ4_compress_fast_continue(
                    ctx as *mut lz4::LZ4Stream,
                    src,
                    dst,
                    src_size,
                    dst_capacity,
                    acceleration,
                )
            }
            CompressFn::BlockHC => {
                f_init_stream(ctx, cdict, level, LZ4F_blockIndependent);
                if !cdict.is_null() {
                    lz4hc::LZ4_compress_HC_continue(
                        ctx as *mut lz4hc::LZ4_streamHC_t,
                        src,
                        dst,
                        src_size,
                        dst_capacity,
                    )
                } else {
                    lz4hc::LZ4_compress_HC_extStateHC_fastReset(
                        ctx,
                        src,
                        dst,
                        src_size,
                        dst_capacity,
                        level,
                    )
                }
            }
            CompressFn::BlockHCContinue => lz4hc::LZ4_compress_HC_continue(
                ctx as *mut lz4hc::LZ4_streamHC_t,
                src,
                dst,
                src_size,
                dst_capacity,
            ),
            CompressFn::DoNotCompress => 0,
        }
    }
}

fn select_compression(block_mode: c_int, level: c_int, compress_mode: u32) -> CompressFn {
    if compress_mode == LZ4B_UNCOMPRESSED {
        return CompressFn::DoNotCompress;
    }
    if level < lz4hc::LZ4HC_CLEVEL_MIN {
        if block_mode == LZ4F_blockIndependent {
            return CompressFn::Block;
        }
        return CompressFn::BlockContinue;
    }
    if block_mode == LZ4F_blockIndependent {
        return CompressFn::BlockHC;
    }
    CompressFn::BlockHCContinue
}

/// `LZ4F_makeBlock()`
unsafe fn make_block(
    dst: *mut u8,
    src: *const u8,
    src_size: usize,
    compress: CompressFn,
    lz4ctx: *mut c_void,
    level: c_int,
    cdict: *const LZ4F_CDict,
    crc_flag: c_int,
) -> usize {
    unsafe {
        let c_size_ptr = dst;
        let mut c_size = call_compress(
            compress,
            lz4ctx,
            src as *const c_char,
            c_size_ptr.add(BH_SIZE) as *mut c_char,
            src_size as c_int,
            (src_size as c_int) - 1,
            level,
            cdict,
        ) as u32;

        if c_size == 0 || c_size as usize >= src_size {
            c_size = src_size as u32;
            write_le32(c_size_ptr, c_size | LZ4F_BLOCKUNCOMPRESSED_FLAG);
            mem_copy(c_size_ptr.add(BH_SIZE), src, src_size);
        } else {
            write_le32(c_size_ptr, c_size);
        }
        if crc_flag != 0 {
            let crc32 = xxhash::xxh32(c_size_ptr.add(BH_SIZE), c_size as usize, 0);
            write_le32(c_size_ptr.add(BH_SIZE + c_size as usize), crc32);
        }
        BH_SIZE + c_size as usize + (crc_flag as usize) * BF_SIZE
    }
}

/// Save history (up to 64KB) into `tmpBuff`.
unsafe fn local_save_dict(cctx: *mut LZ4F_cctx) -> c_int {
    unsafe {
        if (*cctx).prefs.compressionLevel < lz4hc::LZ4HC_CLEVEL_MIN {
            lz4::LZ4_saveDict(
                (*cctx).lz4CtxPtr as *mut lz4::LZ4Stream,
                (*cctx).tmpBuff as *mut c_char,
                64 * 1024,
            )
        } else {
            lz4hc::LZ4_saveDictHC(
                (*cctx).lz4CtxPtr as *mut lz4hc::LZ4_streamHC_t,
                (*cctx).tmpBuff as *mut c_char,
                64 * 1024,
            )
        }
    }
}

const K_C_OPTIONS_NULL: LZ4F_compressOptions_t =
    LZ4F_compressOptions_t { stableSrc: 0, reserved: [0; 3] };

const NOT_DONE: u32 = 0;
const FROM_TMP_BUFFER: u32 = 1;
const FROM_SRC_BUFFER: u32 = 2;

unsafe fn compress_update_impl(
    cctx: *mut LZ4F_cctx,
    dst_buffer: *mut c_void,
    dst_capacity: usize,
    src_buffer: *const c_void,
    src_size: usize,
    compress_options_ptr: *const LZ4F_compressOptions_t,
    block_compression: u32,
) -> usize {
    unsafe {
        let block_size = (*cctx).maxBlockSize;
        let mut src_ptr = src_buffer as *const u8;
        let src_end = src_ptr.wrapping_add(src_size);
        let dst_start = dst_buffer as *mut u8;
        let mut dst_ptr = dst_start;
        let mut last_block_compressed = NOT_DONE;
        let compress = select_compression(
            (*cctx).prefs.frameInfo.blockMode,
            (*cctx).prefs.compressionLevel,
            block_compression,
        );

        if (*cctx).cStage != 1 {
            return ret_err(LZ4F_ERROR_compressionState_uninitialized);
        }
        if dst_capacity < compress_bound_internal(src_size, &(*cctx).prefs, (*cctx).tmpInSize) {
            return ret_err(LZ4F_ERROR_dstMaxSize_tooSmall);
        }

        if block_compression == LZ4B_UNCOMPRESSED && dst_capacity < src_size {
            return ret_err(LZ4F_ERROR_dstMaxSize_tooSmall);
        }

        /* flush currently written block, to continue with a new block compression */
        if (*cctx).blockCompressMode != block_compression {
            let bytes_written = LZ4F_flush(cctx, dst_buffer, dst_capacity, compress_options_ptr);
            dst_ptr = dst_ptr.wrapping_add(bytes_written);
            (*cctx).blockCompressMode = block_compression;
        }

        let compress_options_ptr = if compress_options_ptr.is_null() {
            &K_C_OPTIONS_NULL as *const LZ4F_compressOptions_t
        } else {
            compress_options_ptr
        };

        /* complete tmp buffer */
        if (*cctx).tmpInSize > 0 {
            let size_to_copy = block_size - (*cctx).tmpInSize;
            if size_to_copy > src_size {
                mem_copy(
                    (*cctx).tmpIn.add((*cctx).tmpInSize),
                    src_buffer as *const u8,
                    src_size,
                );
                src_ptr = src_end;
                (*cctx).tmpInSize += src_size;
            } else {
                last_block_compressed = FROM_TMP_BUFFER;
                mem_copy(
                    (*cctx).tmpIn.add((*cctx).tmpInSize),
                    src_buffer as *const u8,
                    size_to_copy,
                );
                src_ptr = src_ptr.wrapping_add(size_to_copy);

                dst_ptr = dst_ptr.wrapping_add(make_block(
                    dst_ptr,
                    (*cctx).tmpIn,
                    block_size,
                    compress,
                    (*cctx).lz4CtxPtr,
                    (*cctx).prefs.compressionLevel,
                    (*cctx).cdict,
                    (*cctx).prefs.frameInfo.blockChecksumFlag,
                ));
                if (*cctx).prefs.frameInfo.blockMode == LZ4F_blockLinked {
                    (*cctx).tmpIn = (*cctx).tmpIn.add(block_size);
                }
                (*cctx).tmpInSize = 0;
            }
        }

        while (src_end as usize - src_ptr as usize) >= block_size {
            last_block_compressed = FROM_SRC_BUFFER;
            dst_ptr = dst_ptr.wrapping_add(make_block(
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
            dst_ptr = dst_ptr.wrapping_add(make_block(
                dst_ptr,
                src_ptr,
                src_end as usize - src_ptr as usize,
                compress,
                (*cctx).lz4CtxPtr,
                (*cctx).prefs.compressionLevel,
                (*cctx).cdict,
                (*cctx).prefs.frameInfo.blockChecksumFlag,
            ));
            src_ptr = src_end;
        }

        /* preserve dictionary within tmpBuff whenever necessary */
        if (*cctx).prefs.frameInfo.blockMode == LZ4F_blockLinked
            && last_block_compressed == FROM_SRC_BUFFER
        {
            if (*compress_options_ptr).stableSrc != 0 {
                (*cctx).tmpIn = (*cctx).tmpBuff;
            } else {
                let real_dict_size = local_save_dict(cctx);
                (*cctx).tmpIn = (*cctx).tmpBuff.wrapping_add(real_dict_size as usize);
            }
        }

        /* keep tmpIn within limits */
        if (*cctx).prefs.autoFlush == 0
            && (*cctx).tmpIn.wrapping_add(block_size)
                > (*cctx).tmpBuff.wrapping_add((*cctx).maxBufferSize)
        {
            let real_dict_size = local_save_dict(cctx);
            (*cctx).tmpIn = (*cctx).tmpBuff.wrapping_add(real_dict_size as usize);
        }

        /* some input data left, necessarily < blockSize */
        if src_ptr < src_end {
            let size_to_copy = src_end as usize - src_ptr as usize;
            mem_copy((*cctx).tmpIn, src_ptr, size_to_copy);
            (*cctx).tmpInSize = size_to_copy;
        }

        if (*cctx).prefs.frameInfo.contentChecksumFlag == LZ4F_contentChecksumEnabled {
            xxhash::xxh32_update(&mut (*cctx).xxh, src_buffer as *const u8, src_size);
        }

        (*cctx).totalInSize = (*cctx).totalInSize.wrapping_add(src_size as u64);
        dst_ptr as usize - dst_start as usize
    }
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
    unsafe {
        compress_update_impl(
            cctx,
            dst_buffer,
            dst_capacity,
            src_buffer,
            src_size,
            compress_options_ptr,
            LZ4B_COMPRESSED,
        )
    }
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
    unsafe {
        compress_update_impl(
            cctx,
            dst_buffer,
            dst_capacity,
            src_buffer,
            src_size,
            compress_options_ptr,
            LZ4B_UNCOMPRESSED,
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_flush(
    cctx: *mut LZ4F_cctx,
    dst_buffer: *mut c_void,
    dst_capacity: usize,
    _compress_options_ptr: *const LZ4F_compressOptions_t,
) -> usize {
    unsafe {
        let dst_start = dst_buffer as *mut u8;
        let mut dst_ptr = dst_start;

        if (*cctx).tmpInSize == 0 {
            return 0;
        }
        if (*cctx).cStage != 1 {
            return ret_err(LZ4F_ERROR_compressionState_uninitialized);
        }
        if dst_capacity < ((*cctx).tmpInSize + BH_SIZE + BF_SIZE) {
            return ret_err(LZ4F_ERROR_dstMaxSize_tooSmall);
        }

        let compress = select_compression(
            (*cctx).prefs.frameInfo.blockMode,
            (*cctx).prefs.compressionLevel,
            (*cctx).blockCompressMode,
        );

        dst_ptr = dst_ptr.wrapping_add(make_block(
            dst_ptr,
            (*cctx).tmpIn,
            (*cctx).tmpInSize,
            compress,
            (*cctx).lz4CtxPtr,
            (*cctx).prefs.compressionLevel,
            (*cctx).cdict,
            (*cctx).prefs.frameInfo.blockChecksumFlag,
        ));

        if (*cctx).prefs.frameInfo.blockMode == LZ4F_blockLinked {
            (*cctx).tmpIn = (*cctx).tmpIn.wrapping_add((*cctx).tmpInSize);
        }
        (*cctx).tmpInSize = 0;

        /* keep tmpIn within limits */
        if (*cctx).tmpIn.wrapping_add((*cctx).maxBlockSize)
            > (*cctx).tmpBuff.wrapping_add((*cctx).maxBufferSize)
        {
            let real_dict_size = local_save_dict(cctx);
            (*cctx).tmpIn = (*cctx).tmpBuff.wrapping_add(real_dict_size as usize);
        }

        dst_ptr as usize - dst_start as usize
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_compressEnd(
    cctx: *mut LZ4F_cctx,
    dst_buffer: *mut c_void,
    dst_capacity: usize,
    compress_options_ptr: *const LZ4F_compressOptions_t,
) -> usize {
    unsafe {
        let dst_start = dst_buffer as *mut u8;
        let mut dst_ptr = dst_start;
        let mut dst_capacity = dst_capacity;

        let flush_size = LZ4F_flush(cctx, dst_buffer, dst_capacity, compress_options_ptr);
        if is_error(flush_size) {
            return flush_size;
        }
        dst_ptr = dst_ptr.wrapping_add(flush_size);

        dst_capacity -= flush_size;

        if dst_capacity < 4 {
            return ret_err(LZ4F_ERROR_dstMaxSize_tooSmall);
        }
        write_le32(dst_ptr, 0);
        dst_ptr = dst_ptr.add(4);

        if (*cctx).prefs.frameInfo.contentChecksumFlag == LZ4F_contentChecksumEnabled {
            let xxh = xxhash::xxh32_digest(&(*cctx).xxh);
            if dst_capacity < 8 {
                return ret_err(LZ4F_ERROR_dstMaxSize_tooSmall);
            }
            write_le32(dst_ptr, xxh);
            dst_ptr = dst_ptr.add(4);
        }

        (*cctx).cStage = 0;

        if (*cctx).prefs.frameInfo.contentSize != 0 {
            if (*cctx).prefs.frameInfo.contentSize != (*cctx).totalInSize {
                return ret_err(LZ4F_ERROR_frameSize_wrong);
            }
        }

        dst_ptr as usize - dst_start as usize
    }
}

/* ===== single-shot frame compression ===== */

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
    unsafe {
        let dst_start = dst_buffer as *mut u8;
        let mut dst_ptr = dst_start;
        let dst_end = dst_start.wrapping_add(dst_capacity);

        let mut prefs = if !preferences_ptr.is_null() {
            *preferences_ptr
        } else {
            LZ4F_preferences_t::zeroed()
        };
        if prefs.frameInfo.contentSize != 0 {
            prefs.frameInfo.contentSize = src_size as u64;
        }

        prefs.frameInfo.blockSizeID = optimal_bsid(prefs.frameInfo.blockSizeID, src_size);
        prefs.autoFlush = 1;
        if src_size <= LZ4F_getBlockSize(prefs.frameInfo.blockSizeID) {
            prefs.frameInfo.blockMode = LZ4F_blockIndependent;
        }

        let mut options = LZ4F_compressOptions_t { stableSrc: 0, reserved: [0; 3] };
        options.stableSrc = 1;

        if dst_capacity < LZ4F_compressFrameBound(src_size, &prefs) {
            return ret_err(LZ4F_ERROR_dstMaxSize_tooSmall);
        }

        {
            let header_size =
                LZ4F_compressBegin_usingCDict(cctx, dst_buffer, dst_capacity, cdict, &prefs);
            if is_error(header_size) {
                return header_size;
            }
            dst_ptr = dst_ptr.wrapping_add(header_size);
        }

        {
            let c_size = LZ4F_compressUpdate(
                cctx,
                dst_ptr as *mut c_void,
                dst_end as usize - dst_ptr as usize,
                src_buffer,
                src_size,
                &options,
            );
            if is_error(c_size) {
                return c_size;
            }
            dst_ptr = dst_ptr.wrapping_add(c_size);
        }

        {
            let tail_size = LZ4F_compressEnd(
                cctx,
                dst_ptr as *mut c_void,
                dst_end as usize - dst_ptr as usize,
                &options,
            );
            if is_error(tail_size) {
                return tail_size;
            }
            dst_ptr = dst_ptr.wrapping_add(tail_size);
        }

        dst_ptr as usize - dst_start as usize
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_compressFrame(
    dst_buffer: *mut c_void,
    dst_capacity: usize,
    src_buffer: *const c_void,
    src_size: usize,
    preferences_ptr: *const LZ4F_preferences_t,
) -> usize {
    unsafe {
        /* LZ4F_HEAPMODE == 0 : cctx and the fast context live on the stack */
        let mut cctx_storage = core::mem::MaybeUninit::<LZ4F_cctx>::zeroed();
        let cctx_ptr = cctx_storage.as_mut_ptr();
        let mut lz4ctx = core::mem::MaybeUninit::<lz4::LZ4Stream>::uninit();

        (*cctx_ptr).version = LZ4F_VERSION;
        (*cctx_ptr).maxBufferSize = 5 * 1024 * 1024;
        if preferences_ptr.is_null()
            || (*preferences_ptr).compressionLevel < lz4hc::LZ4HC_CLEVEL_MIN
        {
            lz4::LZ4_initStream(
                lz4ctx.as_mut_ptr() as *mut c_void,
                core::mem::size_of::<lz4::LZ4Stream>(),
            );
            (*cctx_ptr).lz4CtxPtr = lz4ctx.as_mut_ptr() as *mut c_void;
            (*cctx_ptr).lz4CtxAlloc = 1;
            (*cctx_ptr).lz4CtxType = CTX_FAST;
        }

        let result = LZ4F_compressFrame_usingCDict(
            cctx_ptr,
            dst_buffer,
            dst_capacity,
            src_buffer,
            src_size,
            core::ptr::null(),
            preferences_ptr,
        );

        if !preferences_ptr.is_null()
            && (*preferences_ptr).compressionLevel >= lz4hc::LZ4HC_CLEVEL_MIN
        {
            f_free((*cctx_ptr).lz4CtxPtr, (*cctx_ptr).cmem);
        }
        result
    }
}

/* ===== frame decompression ===== */

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
    pub xxh: XXH32State,
    pub blockChecksum: XXH32State,
    pub skipChecksum: c_int,
    pub header: [u8; LZ4F_HEADER_SIZE_MAX],
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_createDecompressionContext_advanced(
    custom_mem: LZ4F_CustomMem,
    version: c_uint,
) -> *mut LZ4F_dctx {
    unsafe {
        let dctx = f_calloc(core::mem::size_of::<LZ4F_dctx>(), custom_mem) as *mut LZ4F_dctx;
        if dctx.is_null() {
            return core::ptr::null_mut();
        }
        (*dctx).cmem = custom_mem;
        (*dctx).version = version;
        dctx
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_createDecompressionContext(
    dctx_ptr: *mut *mut LZ4F_dctx,
    version_number: c_uint,
) -> usize {
    unsafe {
        if dctx_ptr.is_null() {
            return ret_err(LZ4F_ERROR_parameter_null);
        }
        *dctx_ptr = LZ4F_createDecompressionContext_advanced(DEFAULT_CMEM, version_number);
        if (*dctx_ptr).is_null() {
            return ret_err(LZ4F_ERROR_allocation_failed);
        }
        LZ4F_OK_NoError
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_freeDecompressionContext(dctx: *mut LZ4F_dctx) -> usize {
    unsafe {
        let mut result = LZ4F_OK_NoError;
        if !dctx.is_null() {
            result = (*dctx).dStage as usize;
            let cmem = (*dctx).cmem;
            f_free((*dctx).tmpIn as *mut c_void, cmem);
            f_free((*dctx).tmpOutBuffer as *mut c_void, cmem);
            f_free(dctx as *mut c_void, cmem);
        }
        result
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_resetDecompressionContext(dctx: *mut LZ4F_dctx) {
    unsafe {
        (*dctx).dStage = DSTAGE_GET_FRAME_HEADER;
        (*dctx).dict = core::ptr::null();
        (*dctx).dictSize = 0;
        (*dctx).skipChecksum = 0;
        (*dctx).frameRemainingSize = 0;
    }
}

unsafe fn decode_header(dctx: *mut LZ4F_dctx, src: *const c_void, src_size: usize) -> usize {
    unsafe {
        let src_ptr = src as *const u8;

        if src_size < MIN_FH_SIZE {
            return ret_err(LZ4F_ERROR_frameHeader_incomplete);
        }
        (*dctx).frameInfo = LZ4F_frameInfo_t::zeroed();

        /* special case : skippable frames */
        if (read_le32(src_ptr) & 0xFFFF_FFF0) == LZ4F_MAGIC_SKIPPABLE_START {
            (*dctx).frameInfo.frameType = LZ4F_skippableFrame;
            if src_ptr == (*dctx).header.as_ptr() {
                (*dctx).tmpInSize = src_size;
                (*dctx).tmpInTarget = 8;
                (*dctx).dStage = DSTAGE_STORE_SFRAME_SIZE;
                return src_size;
            } else {
                (*dctx).dStage = DSTAGE_GET_SFRAME_SIZE;
                return 4;
            }
        }

        /* control magic number */
        if read_le32(src_ptr) != LZ4F_MAGICNUMBER {
            return ret_err(LZ4F_ERROR_frameType_unknown);
        }
        (*dctx).frameInfo.frameType = LZ4F_frame;

        let block_mode;
        let block_checksum_flag;
        let content_size_flag;
        let content_checksum_flag;
        let dict_id_flag;
        {
            let flg = *src_ptr.add(4) as u32;
            let version = (flg >> 6) & 0x03;
            block_checksum_flag = (flg >> 4) & 0x01;
            block_mode = (flg >> 5) & 0x01;
            content_size_flag = (flg >> 3) & 0x01;
            content_checksum_flag = (flg >> 2) & 0x01;
            dict_id_flag = flg & 0x01;
            if ((flg >> 1) & 0x01) != 0 {
                return ret_err(LZ4F_ERROR_reservedFlag_set);
            }
            if version != 1 {
                return ret_err(LZ4F_ERROR_headerVersion_wrong);
            }
        }

        let frame_header_size = MIN_FH_SIZE
            + if content_size_flag != 0 { 8 } else { 0 }
            + if dict_id_flag != 0 { 4 } else { 0 };

        if src_size < frame_header_size {
            if src_ptr != (*dctx).header.as_ptr() {
                mem_copy((*dctx).header.as_mut_ptr(), src_ptr, src_size);
            }
            (*dctx).tmpInSize = src_size;
            (*dctx).tmpInTarget = frame_header_size;
            (*dctx).dStage = DSTAGE_STORE_FRAME_HEADER;
            return src_size;
        }

        let block_size_id;
        {
            let bd = *src_ptr.add(5) as u32;
            block_size_id = (bd >> 4) & 0x07;
            if ((bd >> 7) & 0x01) != 0 {
                return ret_err(LZ4F_ERROR_reservedFlag_set);
            }
            if block_size_id < 4 {
                return ret_err(LZ4F_ERROR_maxBlockSize_invalid);
            }
            if (bd & 0x0F) != 0 {
                return ret_err(LZ4F_ERROR_reservedFlag_set);
            }
        }

        /* check header */
        {
            let hc = header_checksum(src_ptr.add(4), frame_header_size - 5);
            if hc != *src_ptr.add(frame_header_size - 1) {
                return ret_err(LZ4F_ERROR_headerChecksum_invalid);
            }
        }

        (*dctx).frameInfo.blockMode = block_mode as c_int;
        (*dctx).frameInfo.blockChecksumFlag = block_checksum_flag as c_int;
        (*dctx).frameInfo.contentChecksumFlag = content_checksum_flag as c_int;
        (*dctx).frameInfo.blockSizeID = block_size_id as c_int;
        (*dctx).maxBlockSize = LZ4F_getBlockSize(block_size_id as c_int);
        if content_size_flag != 0 {
            (*dctx).frameInfo.contentSize = read_le64(src_ptr.add(6));
            (*dctx).frameRemainingSize = (*dctx).frameInfo.contentSize;
        }
        if dict_id_flag != 0 {
            (*dctx).frameInfo.dictID = read_le32(src_ptr.add(frame_header_size - 5));
        }

        (*dctx).dStage = DSTAGE_INIT;

        frame_header_size
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_headerSize(src: *const c_void, src_size: usize) -> usize {
    unsafe {
        if src.is_null() {
            return ret_err(LZ4F_ERROR_srcPtr_wrong);
        }
        if src_size < LZ4F_MIN_SIZE_TO_KNOW_HEADER_LENGTH {
            return ret_err(LZ4F_ERROR_frameHeader_incomplete);
        }
        if (read_le32(src as *const u8) & 0xFFFF_FFF0) == LZ4F_MAGIC_SKIPPABLE_START {
            return 8;
        }
        if read_le32(src as *const u8) != LZ4F_MAGICNUMBER {
            return ret_err(LZ4F_ERROR_frameType_unknown);
        }
        {
            let flg = *(src as *const u8).add(4);
            let content_size_flag = ((flg >> 3) & 0x01) as usize;
            let dict_id_flag = (flg & 0x01) as usize;
            MIN_FH_SIZE
                + if content_size_flag != 0 { 8 } else { 0 }
                + if dict_id_flag != 0 { 4 } else { 0 }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_getFrameInfo(
    dctx: *mut LZ4F_dctx,
    frame_info_ptr: *mut LZ4F_frameInfo_t,
    src_buffer: *const c_void,
    src_size_ptr: *mut usize,
) -> usize {
    unsafe {
        if (*dctx).dStage > DSTAGE_STORE_FRAME_HEADER {
            /* frameInfo already decoded */
            let mut o: usize = 0;
            let mut i: usize = 0;
            *src_size_ptr = 0;
            *frame_info_ptr = (*dctx).frameInfo;
            return LZ4F_decompress(
                dctx,
                core::ptr::null_mut(),
                &mut o,
                core::ptr::null(),
                &mut i,
                core::ptr::null(),
            );
        }
        if (*dctx).dStage == DSTAGE_STORE_FRAME_HEADER {
            *src_size_ptr = 0;
            return ret_err(LZ4F_ERROR_frameDecoding_alreadyStarted);
        }
        let h_size = LZ4F_headerSize(src_buffer, *src_size_ptr);
        if is_error(h_size) {
            *src_size_ptr = 0;
            return h_size;
        }
        if *src_size_ptr < h_size {
            *src_size_ptr = 0;
            return ret_err(LZ4F_ERROR_frameHeader_incomplete);
        }

        let mut decode_result = decode_header(dctx, src_buffer, h_size);
        if is_error(decode_result) {
            *src_size_ptr = 0;
        } else {
            *src_size_ptr = decode_result;
            decode_result = BH_SIZE;
        }
        *frame_info_ptr = (*dctx).frameInfo;
        decode_result
    }
}

/// `LZ4F_updateDict()` : only used in `LZ4F_blockLinked` mode.
unsafe fn update_dict(
    dctx: *mut LZ4F_dctx,
    dst_ptr: *const u8,
    dst_size: usize,
    dst_buffer_start: *const u8,
    within_tmp: bool,
) {
    unsafe {
        if (*dctx).dictSize == 0 {
            (*dctx).dict = dst_ptr;
        }

        if (*dctx).dict.wrapping_add((*dctx).dictSize) == dst_ptr {
            (*dctx).dictSize += dst_size;
            return;
        }

        if (dst_ptr as usize - dst_buffer_start as usize) + dst_size >= 64 * 1024 {
            (*dctx).dict = dst_buffer_start;
            (*dctx).dictSize = (dst_ptr as usize - dst_buffer_start as usize) + dst_size;
            return;
        }

        if within_tmp && (*dctx).dict == (*dctx).tmpOutBuffer as *const u8 {
            (*dctx).dictSize += dst_size;
            return;
        }

        if within_tmp {
            let preserve_size = (*dctx).tmpOut as usize - (*dctx).tmpOutBuffer as usize;
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

            mem_copy(
                (*dctx).tmpOutBuffer.wrapping_add(preserve_size - copy_size),
                old_dict_end.wrapping_sub(copy_size),
                copy_size,
            );

            (*dctx).dict = (*dctx).tmpOutBuffer;
            (*dctx).dictSize = preserve_size + (*dctx).tmpOutStart + dst_size;
            return;
        }

        if (*dctx).dict == (*dctx).tmpOutBuffer as *const u8 {
            if (*dctx).dictSize + dst_size > (*dctx).maxBufferSize {
                let preserve_size = 64 * 1024 - dst_size;
                mem_copy(
                    (*dctx).tmpOutBuffer,
                    (*dctx)
                        .dict
                        .wrapping_add((*dctx).dictSize)
                        .wrapping_sub(preserve_size),
                    preserve_size,
                );
                (*dctx).dictSize = preserve_size;
            }
            mem_copy(
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
            mem_copy(
                (*dctx).tmpOutBuffer,
                (*dctx)
                    .dict
                    .wrapping_add((*dctx).dictSize)
                    .wrapping_sub(preserve_size),
                preserve_size,
            );
            mem_copy(
                (*dctx).tmpOutBuffer.wrapping_add(preserve_size),
                dst_ptr,
                dst_size,
            );
            (*dctx).dict = (*dctx).tmpOutBuffer;
            (*dctx).dictSize = preserve_size + dst_size;
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LZ4F_decompress(
    dctx: *mut LZ4F_dctx,
    dst_buffer: *mut c_void,
    dst_size_ptr: *mut usize,
    src_buffer: *const c_void,
    src_size_ptr: *mut usize,
    decompress_options_ptr: *const LZ4F_decompressOptions_t,
) -> usize {
    unsafe {
        let options_null = LZ4F_decompressOptions_t {
            stableDst: 0,
            skipChecksums: 0,
            reserved1: 0,
            reserved0: 0,
        };
        let src_start = src_buffer as *const u8;
        let src_end = src_start.wrapping_add(*src_size_ptr);
        let mut src_ptr = src_start;
        let dst_start = dst_buffer as *mut u8;
        let dst_end: *mut u8 = if !dst_start.is_null() {
            dst_start.wrapping_add(*dst_size_ptr)
        } else {
            core::ptr::null_mut()
        };
        let mut dst_ptr = dst_start;
        let mut selected_in: *const u8 = core::ptr::null();
        let mut do_another_stage = true;
        let mut next_src_size_hint: usize = 1;

        let decompress_options_ptr = if decompress_options_ptr.is_null() {
            &options_null as *const LZ4F_decompressOptions_t
        } else {
            decompress_options_ptr
        };
        *src_size_ptr = 0;
        *dst_size_ptr = 0;
        (*dctx).skipChecksum |= ((*decompress_options_ptr).skipChecksums != 0) as c_int;

        macro_rules! ret_e {
            ($e:expr) => {
                return ret_err($e)
            };
        }

        'state_machine: while do_another_stage {
            /* `stage` mirrors the C switch; fall-through is modelled by
             * re-assigning and looping. */
            let mut stage = (*dctx).dStage;

            /* --- dstage_getFrameHeader --- */
            if stage == DSTAGE_GET_FRAME_HEADER {
                if (src_end as usize - src_ptr as usize) >= MAX_FH_SIZE {
                    let h_size =
                        decode_header(dctx, src_ptr as *const c_void, src_end as usize - src_ptr as usize);
                    if is_error(h_size) {
                        return h_size;
                    }
                    src_ptr = src_ptr.wrapping_add(h_size);
                    continue 'state_machine;
                }
                (*dctx).tmpInSize = 0;
                if src_end as usize - src_ptr as usize == 0 {
                    return MIN_FH_SIZE;
                }
                (*dctx).tmpInTarget = MIN_FH_SIZE;
                (*dctx).dStage = DSTAGE_STORE_FRAME_HEADER;
                stage = DSTAGE_STORE_FRAME_HEADER;
            }

            /* --- dstage_storeFrameHeader --- */
            if stage == DSTAGE_STORE_FRAME_HEADER {
                {
                    let a = (*dctx).tmpInTarget - (*dctx).tmpInSize;
                    let b = src_end as usize - src_ptr as usize;
                    let size_to_copy = if a < b { a } else { b };
                    mem_copy(
                        (*dctx).header.as_mut_ptr().add((*dctx).tmpInSize),
                        src_ptr,
                        size_to_copy,
                    );
                    (*dctx).tmpInSize += size_to_copy;
                    src_ptr = src_ptr.wrapping_add(size_to_copy);
                }
                if (*dctx).tmpInSize < (*dctx).tmpInTarget {
                    next_src_size_hint = ((*dctx).tmpInTarget - (*dctx).tmpInSize) + BH_SIZE;
                    do_another_stage = false;
                    break 'state_machine;
                }
                let r = decode_header(
                    dctx,
                    (*dctx).header.as_ptr() as *const c_void,
                    (*dctx).tmpInTarget,
                );
                if is_error(r) {
                    return r;
                }
                continue 'state_machine;
            }

            /* --- dstage_init --- */
            if stage == DSTAGE_INIT {
                if (*dctx).frameInfo.contentChecksumFlag != 0 {
                    xxhash::xxh32_reset(&mut (*dctx).xxh, 0);
                }
                {
                    let buffer_needed = (*dctx).maxBlockSize
                        + if (*dctx).frameInfo.blockMode == LZ4F_blockLinked {
                            128 * 1024
                        } else {
                            0
                        };
                    if buffer_needed > (*dctx).maxBufferSize {
                        (*dctx).maxBufferSize = 0;
                        f_free((*dctx).tmpIn as *mut c_void, (*dctx).cmem);
                        (*dctx).tmpIn =
                            f_malloc((*dctx).maxBlockSize + BF_SIZE, (*dctx).cmem) as *mut u8;
                        if (*dctx).tmpIn.is_null() {
                            ret_e!(LZ4F_ERROR_allocation_failed);
                        }
                        f_free((*dctx).tmpOutBuffer as *mut c_void, (*dctx).cmem);
                        (*dctx).tmpOutBuffer = f_malloc(buffer_needed, (*dctx).cmem) as *mut u8;
                        if (*dctx).tmpOutBuffer.is_null() {
                            ret_e!(LZ4F_ERROR_allocation_failed);
                        }
                        (*dctx).maxBufferSize = buffer_needed;
                    }
                }
                (*dctx).tmpInSize = 0;
                (*dctx).tmpInTarget = 0;
                (*dctx).tmpOut = (*dctx).tmpOutBuffer;
                (*dctx).tmpOutStart = 0;
                (*dctx).tmpOutSize = 0;

                (*dctx).dStage = DSTAGE_GET_BLOCK_HEADER;
                stage = DSTAGE_GET_BLOCK_HEADER;
            }

            /* --- dstage_getBlockHeader / storeBlockHeader --- */
            if stage == DSTAGE_GET_BLOCK_HEADER || stage == DSTAGE_STORE_BLOCK_HEADER {
                if stage == DSTAGE_GET_BLOCK_HEADER {
                    if (src_end as usize - src_ptr as usize) >= BH_SIZE {
                        selected_in = src_ptr;
                        src_ptr = src_ptr.wrapping_add(BH_SIZE);
                    } else {
                        (*dctx).tmpInSize = 0;
                        (*dctx).dStage = DSTAGE_STORE_BLOCK_HEADER;
                    }
                }

                if (*dctx).dStage == DSTAGE_STORE_BLOCK_HEADER {
                    let remaining_input = src_end as usize - src_ptr as usize;
                    let wanted_data = BH_SIZE - (*dctx).tmpInSize;
                    let size_to_copy = if wanted_data < remaining_input {
                        wanted_data
                    } else {
                        remaining_input
                    };
                    mem_copy((*dctx).tmpIn.add((*dctx).tmpInSize), src_ptr, size_to_copy);
                    src_ptr = src_ptr.wrapping_add(size_to_copy);
                    (*dctx).tmpInSize += size_to_copy;

                    if (*dctx).tmpInSize < BH_SIZE {
                        next_src_size_hint = BH_SIZE - (*dctx).tmpInSize;
                        do_another_stage = false;
                        break 'state_machine;
                    }
                    selected_in = (*dctx).tmpIn;
                }

                /* decode block header */
                {
                    let block_header = read_le32(selected_in);
                    let next_cblock_size = (block_header & 0x7FFF_FFFF) as usize;
                    let crc_size = (*dctx).frameInfo.blockChecksumFlag as usize * BF_SIZE;
                    if block_header == 0 {
                        (*dctx).dStage = DSTAGE_GET_SUFFIX;
                        continue 'state_machine;
                    }
                    if next_cblock_size > (*dctx).maxBlockSize {
                        ret_e!(LZ4F_ERROR_maxBlockSize_invalid);
                    }
                    if (block_header & LZ4F_BLOCKUNCOMPRESSED_FLAG) != 0 {
                        (*dctx).tmpInTarget = next_cblock_size;
                        if (*dctx).frameInfo.blockChecksumFlag != 0 {
                            xxhash::xxh32_reset(&mut (*dctx).blockChecksum, 0);
                        }
                        (*dctx).dStage = DSTAGE_COPY_DIRECT;
                        continue 'state_machine;
                    }
                    /* next block is a compressed block */
                    (*dctx).tmpInTarget = next_cblock_size + crc_size;
                    (*dctx).dStage = DSTAGE_GET_CBLOCK;
                    if dst_ptr == dst_end || src_ptr == src_end {
                        next_src_size_hint = BH_SIZE + next_cblock_size + crc_size;
                        do_another_stage = false;
                    }
                    continue 'state_machine;
                }
            }

            /* --- dstage_copyDirect --- */
            if stage == DSTAGE_COPY_DIRECT {
                let size_to_copy;
                if dst_ptr.is_null() {
                    size_to_copy = 0;
                } else {
                    let a = src_end as usize - src_ptr as usize;
                    let b = dst_end as usize - dst_ptr as usize;
                    let min_buff_size = if a < b { a } else { b };
                    size_to_copy = if (*dctx).tmpInTarget < min_buff_size {
                        (*dctx).tmpInTarget
                    } else {
                        min_buff_size
                    };
                    mem_copy(dst_ptr, src_ptr, size_to_copy);
                    if (*dctx).skipChecksum == 0 {
                        if (*dctx).frameInfo.blockChecksumFlag != 0 {
                            xxhash::xxh32_update(&mut (*dctx).blockChecksum, src_ptr, size_to_copy);
                        }
                        if (*dctx).frameInfo.contentChecksumFlag != 0 {
                            xxhash::xxh32_update(&mut (*dctx).xxh, src_ptr, size_to_copy);
                        }
                    }
                    if (*dctx).frameInfo.contentSize != 0 {
                        (*dctx).frameRemainingSize =
                            (*dctx).frameRemainingSize.wrapping_sub(size_to_copy as u64);
                    }
                    if (*dctx).frameInfo.blockMode == LZ4F_blockLinked {
                        update_dict(dctx, dst_ptr, size_to_copy, dst_start, false);
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
                    continue 'state_machine;
                }
                (*dctx).tmpInTarget -= size_to_copy;
                next_src_size_hint = (*dctx).tmpInTarget
                    + if (*dctx).frameInfo.blockChecksumFlag != 0 {
                        BF_SIZE
                    } else {
                        0
                    }
                    + BH_SIZE;
                do_another_stage = false;
                break 'state_machine;
            }

            /* --- dstage_getBlockChecksum --- */
            if stage == DSTAGE_GET_BLOCK_CHECKSUM {
                let crc_src;
                if (src_end as usize - src_ptr as usize) >= 4 && (*dctx).tmpInSize == 0 {
                    crc_src = src_ptr;
                    src_ptr = src_ptr.wrapping_add(4);
                } else {
                    let still_to_copy = 4 - (*dctx).tmpInSize;
                    let remaining = src_end as usize - src_ptr as usize;
                    let size_to_copy = if still_to_copy < remaining {
                        still_to_copy
                    } else {
                        remaining
                    };
                    mem_copy(
                        (*dctx).header.as_mut_ptr().add((*dctx).tmpInSize),
                        src_ptr,
                        size_to_copy,
                    );
                    (*dctx).tmpInSize += size_to_copy;
                    src_ptr = src_ptr.wrapping_add(size_to_copy);
                    if (*dctx).tmpInSize < 4 {
                        do_another_stage = false;
                        break 'state_machine;
                    }
                    crc_src = (*dctx).header.as_ptr();
                }
                if (*dctx).skipChecksum == 0 {
                    let read_crc = read_le32(crc_src);
                    let calc_crc = xxhash::xxh32_digest(&(*dctx).blockChecksum);
                    if read_crc != calc_crc {
                        ret_e!(LZ4F_ERROR_blockChecksum_invalid);
                    }
                }
                (*dctx).dStage = DSTAGE_GET_BLOCK_HEADER;
                continue 'state_machine;
            }

            /* --- dstage_getCBlock / storeCBlock --- */
            if stage == DSTAGE_GET_CBLOCK || stage == DSTAGE_STORE_CBLOCK {
                let mut entered_store = stage == DSTAGE_STORE_CBLOCK;
                if stage == DSTAGE_GET_CBLOCK {
                    if (src_end as usize - src_ptr as usize) < (*dctx).tmpInTarget {
                        (*dctx).tmpInSize = 0;
                        (*dctx).dStage = DSTAGE_STORE_CBLOCK;
                        continue 'state_machine;
                    }
                    selected_in = src_ptr;
                    src_ptr = src_ptr.wrapping_add((*dctx).tmpInTarget);
                    entered_store = false;
                }

                if entered_store {
                    let wanted_data = (*dctx).tmpInTarget - (*dctx).tmpInSize;
                    let input_left = src_end as usize - src_ptr as usize;
                    let size_to_copy = if wanted_data < input_left {
                        wanted_data
                    } else {
                        input_left
                    };
                    mem_copy((*dctx).tmpIn.add((*dctx).tmpInSize), src_ptr, size_to_copy);
                    (*dctx).tmpInSize += size_to_copy;
                    src_ptr = src_ptr.wrapping_add(size_to_copy);
                    if (*dctx).tmpInSize < (*dctx).tmpInTarget {
                        next_src_size_hint = ((*dctx).tmpInTarget - (*dctx).tmpInSize)
                            + if (*dctx).frameInfo.blockChecksumFlag != 0 {
                                BF_SIZE
                            } else {
                                0
                            }
                            + BH_SIZE;
                        do_another_stage = false;
                        break 'state_machine;
                    }
                    selected_in = (*dctx).tmpIn;
                }

                /* decode and control block checksum if it exists */
                if (*dctx).frameInfo.blockChecksumFlag != 0 {
                    (*dctx).tmpInTarget -= 4;
                    let read_block_crc = read_le32(selected_in.add((*dctx).tmpInTarget));
                    let calc_block_crc = xxhash::xxh32(selected_in, (*dctx).tmpInTarget, 0);
                    if read_block_crc != calc_block_crc {
                        ret_e!(LZ4F_ERROR_blockChecksum_invalid);
                    }
                }

                /* decode directly into the destination buffer if there's room */
                if (dst_end as usize - dst_ptr as usize) >= (*dctx).maxBlockSize
                    && !(!(*dctx).dict.is_null()
                        && (*dctx).dict.wrapping_add((*dctx).dictSize)
                            == (*dctx).tmpOut as *const u8)
                {
                    let mut dict = (*dctx).dict as *const c_char;
                    let mut dict_size = (*dctx).dictSize;
                    if !dict.is_null() && dict_size > (1usize << 30) {
                        dict = dict.wrapping_add(dict_size - 64 * 1024);
                        dict_size = 64 * 1024;
                    }
                    let decoded_size = lz4::LZ4_decompress_safe_usingDict(
                        selected_in as *const c_char,
                        dst_ptr as *mut c_char,
                        (*dctx).tmpInTarget as c_int,
                        (*dctx).maxBlockSize as c_int,
                        dict,
                        dict_size as c_int,
                    );
                    if decoded_size < 0 {
                        ret_e!(LZ4F_ERROR_decompressionFailed);
                    }
                    if (*dctx).frameInfo.contentChecksumFlag != 0 && (*dctx).skipChecksum == 0 {
                        xxhash::xxh32_update(&mut (*dctx).xxh, dst_ptr, decoded_size as usize);
                    }
                    if (*dctx).frameInfo.contentSize != 0 {
                        (*dctx).frameRemainingSize =
                            (*dctx).frameRemainingSize.wrapping_sub(decoded_size as u64);
                    }

                    if (*dctx).frameInfo.blockMode == LZ4F_blockLinked {
                        update_dict(dctx, dst_ptr, decoded_size as usize, dst_start, false);
                    }

                    dst_ptr = dst_ptr.wrapping_add(decoded_size as usize);
                    (*dctx).dStage = DSTAGE_GET_BLOCK_HEADER;
                    continue 'state_machine;
                }

                /* not enough room in dst : decode into tmpOut */
                if (*dctx).frameInfo.blockMode == LZ4F_blockLinked {
                    if (*dctx).dict == (*dctx).tmpOutBuffer as *const u8 {
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
                        let reserved_dict_space = if (*dctx).dictSize < 64 * 1024 {
                            (*dctx).dictSize
                        } else {
                            64 * 1024
                        };
                        (*dctx).tmpOut = (*dctx).tmpOutBuffer.wrapping_add(reserved_dict_space);
                    }
                }

                {
                    let mut dict = (*dctx).dict as *const c_char;
                    let mut dict_size = (*dctx).dictSize;
                    if !dict.is_null() && dict_size > (1usize << 30) {
                        dict = dict.wrapping_add(dict_size - 64 * 1024);
                        dict_size = 64 * 1024;
                    }
                    let decoded_size = lz4::LZ4_decompress_safe_usingDict(
                        selected_in as *const c_char,
                        (*dctx).tmpOut as *mut c_char,
                        (*dctx).tmpInTarget as c_int,
                        (*dctx).maxBlockSize as c_int,
                        dict,
                        dict_size as c_int,
                    );
                    if decoded_size < 0 {
                        ret_e!(LZ4F_ERROR_decompressionFailed);
                    }
                    if (*dctx).frameInfo.contentChecksumFlag != 0 && (*dctx).skipChecksum == 0 {
                        xxhash::xxh32_update(
                            &mut (*dctx).xxh,
                            (*dctx).tmpOut,
                            decoded_size as usize,
                        );
                    }
                    if (*dctx).frameInfo.contentSize != 0 {
                        (*dctx).frameRemainingSize =
                            (*dctx).frameRemainingSize.wrapping_sub(decoded_size as u64);
                    }
                    (*dctx).tmpOutSize = decoded_size as usize;
                    (*dctx).tmpOutStart = 0;
                    (*dctx).dStage = DSTAGE_FLUSH_OUT;
                }
                stage = DSTAGE_FLUSH_OUT;
            }

            /* --- dstage_flushOut --- */
            if stage == DSTAGE_FLUSH_OUT {
                if !dst_ptr.is_null() {
                    let a = (*dctx).tmpOutSize - (*dctx).tmpOutStart;
                    let b = dst_end as usize - dst_ptr as usize;
                    let size_to_copy = if a < b { a } else { b };
                    mem_copy(
                        dst_ptr,
                        (*dctx).tmpOut.wrapping_add((*dctx).tmpOutStart),
                        size_to_copy,
                    );

                    if (*dctx).frameInfo.blockMode == LZ4F_blockLinked {
                        update_dict(dctx, dst_ptr, size_to_copy, dst_start, true);
                    }

                    (*dctx).tmpOutStart += size_to_copy;
                    dst_ptr = dst_ptr.wrapping_add(size_to_copy);
                }
                if (*dctx).tmpOutStart == (*dctx).tmpOutSize {
                    (*dctx).dStage = DSTAGE_GET_BLOCK_HEADER;
                    continue 'state_machine;
                }
                do_another_stage = false;
                next_src_size_hint = BH_SIZE;
                break 'state_machine;
            }

            /* --- dstage_getSuffix / storeSuffix --- */
            if stage == DSTAGE_GET_SUFFIX || stage == DSTAGE_STORE_SUFFIX {
                if stage == DSTAGE_GET_SUFFIX {
                    if (*dctx).frameRemainingSize != 0 {
                        ret_e!(LZ4F_ERROR_frameSize_wrong);
                    }
                    if (*dctx).frameInfo.contentChecksumFlag == 0 {
                        next_src_size_hint = 0;
                        LZ4F_resetDecompressionContext(dctx);
                        do_another_stage = false;
                        break 'state_machine;
                    }
                    if (src_end as isize - src_ptr as isize) < 4 {
                        (*dctx).tmpInSize = 0;
                        (*dctx).dStage = DSTAGE_STORE_SUFFIX;
                    } else {
                        selected_in = src_ptr;
                        src_ptr = src_ptr.wrapping_add(4);
                    }
                }

                if (*dctx).dStage == DSTAGE_STORE_SUFFIX {
                    let remaining_input = src_end as usize - src_ptr as usize;
                    let wanted_data = 4 - (*dctx).tmpInSize;
                    let size_to_copy = if wanted_data < remaining_input {
                        wanted_data
                    } else {
                        remaining_input
                    };
                    mem_copy((*dctx).tmpIn.add((*dctx).tmpInSize), src_ptr, size_to_copy);
                    src_ptr = src_ptr.wrapping_add(size_to_copy);
                    (*dctx).tmpInSize += size_to_copy;
                    if (*dctx).tmpInSize < 4 {
                        next_src_size_hint = 4 - (*dctx).tmpInSize;
                        do_another_stage = false;
                        break 'state_machine;
                    }
                    selected_in = (*dctx).tmpIn;
                }

                if (*dctx).skipChecksum == 0 {
                    let read_crc = read_le32(selected_in);
                    let result_crc = xxhash::xxh32_digest(&(*dctx).xxh);
                    if read_crc != result_crc {
                        ret_e!(LZ4F_ERROR_contentChecksum_invalid);
                    }
                }
                next_src_size_hint = 0;
                LZ4F_resetDecompressionContext(dctx);
                do_another_stage = false;
                break 'state_machine;
            }

            /* --- dstage_getSFrameSize / storeSFrameSize --- */
            if stage == DSTAGE_GET_SFRAME_SIZE || stage == DSTAGE_STORE_SFRAME_SIZE {
                if stage == DSTAGE_GET_SFRAME_SIZE {
                    if (src_end as isize - src_ptr as isize) >= 4 {
                        selected_in = src_ptr;
                        src_ptr = src_ptr.wrapping_add(4);
                    } else {
                        (*dctx).tmpInSize = 4;
                        (*dctx).tmpInTarget = 8;
                        (*dctx).dStage = DSTAGE_STORE_SFRAME_SIZE;
                    }
                }

                if (*dctx).dStage == DSTAGE_STORE_SFRAME_SIZE {
                    let a = (*dctx).tmpInTarget - (*dctx).tmpInSize;
                    let b = src_end as usize - src_ptr as usize;
                    let size_to_copy = if a < b { a } else { b };
                    mem_copy(
                        (*dctx).header.as_mut_ptr().add((*dctx).tmpInSize),
                        src_ptr,
                        size_to_copy,
                    );
                    src_ptr = src_ptr.wrapping_add(size_to_copy);
                    (*dctx).tmpInSize += size_to_copy;
                    if (*dctx).tmpInSize < (*dctx).tmpInTarget {
                        next_src_size_hint = (*dctx).tmpInTarget - (*dctx).tmpInSize;
                        do_another_stage = false;
                        break 'state_machine;
                    }
                    selected_in = (*dctx).header.as_ptr().add(4);
                }

                {
                    let s_frame_size = read_le32(selected_in) as usize;
                    (*dctx).frameInfo.contentSize = s_frame_size as u64;
                    (*dctx).tmpInTarget = s_frame_size;
                    (*dctx).dStage = DSTAGE_SKIP_SKIPPABLE;
                    continue 'state_machine;
                }
            }

            /* --- dstage_skipSkippable --- */
            if stage == DSTAGE_SKIP_SKIPPABLE {
                let remaining = src_end as usize - src_ptr as usize;
                let skip_size = if (*dctx).tmpInTarget < remaining {
                    (*dctx).tmpInTarget
                } else {
                    remaining
                };
                src_ptr = src_ptr.wrapping_add(skip_size);
                (*dctx).tmpInTarget -= skip_size;
                do_another_stage = false;
                next_src_size_hint = (*dctx).tmpInTarget;
                if next_src_size_hint != 0 {
                    break 'state_machine;
                }
                LZ4F_resetDecompressionContext(dctx);
                break 'state_machine;
            }

            /* unreachable in practice */
            break 'state_machine;
        }

        /* preserve history within tmpOut whenever necessary */
        if (*dctx).frameInfo.blockMode == LZ4F_blockLinked
            && (*dctx).dict != (*dctx).tmpOutBuffer as *const u8
            && !(*dctx).dict.is_null()
            && (*decompress_options_ptr).stableDst == 0
            && ((*dctx).dStage.wrapping_sub(2) < DSTAGE_GET_SUFFIX - 2)
        {
            if (*dctx).dStage == DSTAGE_FLUSH_OUT {
                let preserve_size = (*dctx).tmpOut as usize - (*dctx).tmpOutBuffer as usize;
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

                mem_copy(
                    (*dctx).tmpOutBuffer.wrapping_add(preserve_size - copy_size),
                    old_dict_end.wrapping_sub(copy_size),
                    copy_size,
                );

                (*dctx).dict = (*dctx).tmpOutBuffer;
                (*dctx).dictSize = preserve_size + (*dctx).tmpOutStart;
            } else {
                let old_dict_end = (*dctx).dict.wrapping_add((*dctx).dictSize);
                let new_dict_size = if (*dctx).dictSize < 64 * 1024 {
                    (*dctx).dictSize
                } else {
                    64 * 1024
                };

                mem_copy(
                    (*dctx).tmpOutBuffer,
                    old_dict_end.wrapping_sub(new_dict_size),
                    new_dict_size,
                );

                (*dctx).dict = (*dctx).tmpOutBuffer;
                (*dctx).dictSize = new_dict_size;
                (*dctx).tmpOut = (*dctx).tmpOutBuffer.wrapping_add(new_dict_size);
            }
        }

        *src_size_ptr = src_ptr as usize - src_start as usize;
        *dst_size_ptr = dst_ptr as usize - dst_start as usize;
        next_src_size_hint
    }
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
    unsafe {
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
}
