//! Translation of `common/zstd_internal.h`, `common/allocations.h`,
//! and the public constants of `include/zstd.h`.
#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]

use super::mem::*;
use core::ffi::{c_int, c_uint, c_void};

/* ================= version ================= */

pub const ZSTD_VERSION_MAJOR: u32 = 1;
pub const ZSTD_VERSION_MINOR: u32 = 5;
pub const ZSTD_VERSION_RELEASE: u32 = 7;
pub const ZSTD_VERSION_NUMBER: u32 =
    ZSTD_VERSION_MAJOR * 100 * 100 + ZSTD_VERSION_MINOR * 100 + ZSTD_VERSION_RELEASE;
pub const ZSTD_VERSION_STRING: &str = "1.5.7\0";

/* ================= shared macros ================= */

#[inline(always)]
pub fn MIN<T: PartialOrd>(a: T, b: T) -> T {
    if a < b {
        a
    } else {
        b
    }
}

#[inline(always)]
pub fn MAX<T: PartialOrd>(a: T, b: T) -> T {
    if a > b {
        a
    } else {
        b
    }
}

#[inline(always)]
pub fn BOUNDED<T: PartialOrd>(min: T, val: T, max: T) -> T {
    MAX(min, MIN(val, max))
}

/* ================= common constants ================= */

pub const ZSTD_OPT_NUM: u32 = 1 << 12;
pub const ZSTD_REP_NUM: usize = 3;
pub static repStartValue: [U32; ZSTD_REP_NUM] = [1, 4, 8];

pub const BIT7: u32 = 128;
pub const BIT6: u32 = 64;
pub const BIT5: u32 = 32;
pub const BIT4: u32 = 16;
pub const BIT1: u32 = 2;
pub const BIT0: u32 = 1;

pub const ZSTD_WINDOWLOG_ABSOLUTEMIN: u32 = 10;
pub static ZSTD_fcs_fieldSize: [size_t; 4] = [0, 2, 4, 8];
pub static ZSTD_did_fieldSize: [size_t; 4] = [0, 1, 2, 4];

pub const ZSTD_FRAMEIDSIZE: size_t = 4;
pub const ZSTD_BLOCKHEADERSIZE: size_t = 3;
pub const ZSTD_blockHeaderSize: size_t = ZSTD_BLOCKHEADERSIZE;

pub type blockType_e = c_uint;
pub const bt_raw: blockType_e = 0;
pub const bt_rle: blockType_e = 1;
pub const bt_compressed: blockType_e = 2;
pub const bt_reserved: blockType_e = 3;

pub const ZSTD_FRAMECHECKSUMSIZE: size_t = 4;

pub const MIN_SEQUENCES_SIZE: size_t = 1;
pub const MIN_CBLOCK_SIZE: size_t = 1 + 1;
pub const MIN_LITERALS_FOR_4_STREAMS: size_t = 6;

pub type SymbolEncodingType_e = c_uint;
pub const set_basic: SymbolEncodingType_e = 0;
pub const set_rle: SymbolEncodingType_e = 1;
pub const set_compressed: SymbolEncodingType_e = 2;
pub const set_repeat: SymbolEncodingType_e = 3;

pub const LONGNBSEQ: u32 = 0x7F00;

pub const MINMATCH: u32 = 3;

pub const Litbits: u32 = 8;
pub const LitHufLog: u32 = 11;
pub const MaxLit: u32 = (1 << Litbits) - 1;
pub const MaxML: u32 = 52;
pub const MaxLL: u32 = 35;
pub const DefaultMaxOff: u32 = 28;
pub const MaxOff: u32 = 31;
pub const MaxSeq: u32 = if MaxLL > MaxML { MaxLL } else { MaxML };
pub const MLFSELog: u32 = 9;
pub const LLFSELog: u32 = 9;
pub const OffFSELog: u32 = 8;
pub const MaxFSELog: u32 = 9;
pub const MaxMLBits: u32 = 16;
pub const MaxLLBits: u32 = 16;

pub const ZSTD_MAX_HUF_HEADER_SIZE: size_t = 128;
pub const ZSTD_MAX_FSE_HEADERS_SIZE: size_t = (((MaxML + 1) * MLFSELog
    + (MaxLL + 1) * LLFSELog
    + (MaxOff + 1) * OffFSELog
    + 7)
    / 8) as size_t;

pub static LL_bits: [U8; (MaxLL + 1) as usize] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 3, 3, 4, 6, 7, 8, 9, 10, 11,
    12, 13, 14, 15, 16,
];
pub static LL_defaultNorm: [S16; (MaxLL + 1) as usize] = [
    4, 3, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 1, 1, 1, 2, 2, 2, 2, 2, 2, 2, 2, 2, 3, 2, 1, 1, 1, 1, 1,
    -1, -1, -1, -1,
];
pub const LL_DEFAULTNORMLOG: u32 = 6;
pub const LL_defaultNormLog: U32 = LL_DEFAULTNORMLOG;

pub static ML_bits: [U8; (MaxML + 1) as usize] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    1, 1, 1, 1, 2, 2, 3, 3, 4, 4, 5, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16,
];
pub static ML_defaultNorm: [S16; (MaxML + 1) as usize] = [
    1, 4, 3, 2, 2, 2, 2, 2, 2, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, -1, -1, -1, -1, -1, -1, -1,
];
pub const ML_DEFAULTNORMLOG: u32 = 6;
pub const ML_defaultNormLog: U32 = ML_DEFAULTNORMLOG;

pub static OF_defaultNorm: [S16; (DefaultMaxOff + 1) as usize] = [
    1, 1, 1, 1, 1, 1, 2, 2, 2, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, -1, -1, -1, -1, -1,
];
pub const OF_DEFAULTNORMLOG: u32 = 5;
pub const OF_defaultNormLog: U32 = OF_DEFAULTNORMLOG;

/* ================= shared inline functions ================= */

#[inline(always)]
pub unsafe fn ZSTD_copy8(dst: *mut u8, src: *const u8) {
    core::ptr::copy_nonoverlapping(src, dst, 8);
}

#[inline(always)]
pub unsafe fn ZSTD_copy16(dst: *mut u8, src: *const u8) {
    let mut buf = [0u8; 16];
    core::ptr::copy_nonoverlapping(src, buf.as_mut_ptr(), 16);
    core::ptr::copy_nonoverlapping(buf.as_ptr(), dst, 16);
}

pub const WILDCOPY_OVERLENGTH: isize = 32;
pub const WILDCOPY_VECLEN: isize = 16;

pub type ZSTD_overlap_e = c_uint;
pub const ZSTD_no_overlap: ZSTD_overlap_e = 0;
pub const ZSTD_overlap_src_before_dst: ZSTD_overlap_e = 1;

#[inline(always)]
pub unsafe fn ZSTD_wildcopy(
    dst: *mut u8,
    src: *const u8,
    length: isize,
    ovtype: ZSTD_overlap_e,
) {
    let diff = (dst as isize) - (src as isize);
    let mut ip = src;
    let mut op = dst;
    let oend = op.offset(length);

    if ovtype == ZSTD_overlap_src_before_dst && diff < WILDCOPY_VECLEN {
        loop {
            ZSTD_copy8(op, ip);
            op = op.add(8);
            ip = ip.add(8);
            if op >= oend {
                break;
            }
        }
    } else {
        ZSTD_copy16(op, ip);
        if 16 >= length {
            return;
        }
        op = op.add(16);
        ip = ip.add(16);
        loop {
            ZSTD_copy16(op, ip);
            op = op.add(16);
            ip = ip.add(16);
            ZSTD_copy16(op, ip);
            op = op.add(16);
            ip = ip.add(16);
            if op >= oend {
                break;
            }
        }
    }
}

#[inline(always)]
pub unsafe fn ZSTD_limitCopy(
    dst: *mut u8,
    dstCapacity: size_t,
    src: *const u8,
    srcSize: size_t,
) -> size_t {
    let length = MIN(dstCapacity, srcSize);
    if length > 0 {
        ZSTD_memcpy(dst, src, length);
    }
    length
}

pub const ZSTD_WORKSPACETOOLARGE_FACTOR: u32 = 3;
pub const ZSTD_WORKSPACETOOLARGE_MAXDURATION: u32 = 128;

pub type ZSTD_bufferMode_e = c_uint;
pub const ZSTD_bm_buffered: ZSTD_bufferMode_e = 0;
pub const ZSTD_bm_stable: ZSTD_bufferMode_e = 1;

/* ================= ZSTD_frameSizeInfo ================= */

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ZSTD_frameSizeInfo {
    pub nbBlocks: size_t,
    pub compressedSize: size_t,
    pub decompressedBound: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct blockProperties_t {
    pub blockType: blockType_e,
    pub lastBlock: U32,
    pub origSize: U32,
}

/* ================= custom allocation ================= */

pub type ZSTD_allocFunction = Option<unsafe extern "C" fn(*mut c_void, size_t) -> *mut c_void>;
pub type ZSTD_freeFunction = Option<unsafe extern "C" fn(*mut c_void, *mut c_void)>;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ZSTD_customMem {
    pub customAlloc: ZSTD_allocFunction,
    pub customFree: ZSTD_freeFunction,
    pub opaque: *mut c_void,
}

impl Default for ZSTD_customMem {
    fn default() -> Self {
        ZSTD_customMem {
            customAlloc: None,
            customFree: None,
            opaque: core::ptr::null_mut(),
        }
    }
}

pub const ZSTD_defaultCMem: ZSTD_customMem = ZSTD_customMem {
    customAlloc: None,
    customFree: None,
    opaque: core::ptr::null_mut(),
};

unsafe extern "C" {
    pub fn malloc(size: size_t) -> *mut c_void;
    pub fn calloc(n: size_t, size: size_t) -> *mut c_void;
    pub fn free(p: *mut c_void);
    pub fn memcpy(d: *mut c_void, s: *const c_void, n: size_t) -> *mut c_void;
    pub fn memmove(d: *mut c_void, s: *const c_void, n: size_t) -> *mut c_void;
    pub fn memset(d: *mut c_void, c: c_int, n: size_t) -> *mut c_void;
    pub fn qsort(
        base: *mut c_void,
        nmemb: size_t,
        size: size_t,
        compar: unsafe extern "C" fn(*const c_void, *const c_void) -> c_int,
    );
    pub fn clock() -> i64;
}

#[inline(always)]
pub unsafe fn ZSTD_customMalloc(size: size_t, customMem: ZSTD_customMem) -> *mut c_void {
    if let Some(f) = customMem.customAlloc {
        return f(customMem.opaque, size);
    }
    malloc(size)
}

#[inline(always)]
pub unsafe fn ZSTD_customCalloc(size: size_t, customMem: ZSTD_customMem) -> *mut c_void {
    if let Some(f) = customMem.customAlloc {
        let ptr = f(customMem.opaque, size);
        if !ptr.is_null() {
            ZSTD_memset(ptr as *mut u8, 0, size);
        }
        return ptr;
    }
    calloc(1, size)
}

#[inline(always)]
pub unsafe fn ZSTD_customFree(ptr: *mut c_void, customMem: ZSTD_customMem) {
    if !ptr.is_null() {
        if let Some(f) = customMem.customFree {
            f(customMem.opaque, ptr)
        } else {
            free(ptr)
        }
    }
}

/* ================= misc ================= */

#[inline(always)]
pub fn ZSTD_cpuSupportsBmi2() -> c_int {
    0
}

pub const ZSTD_TRACE: u32 = 0;
