//! Shared definitions extracted from lz4.c (the parts included by lz4hc.c via
//! `LZ4_COMMONDEFS_ONLY`), plus the shared public structure layouts.

use core::ptr;

/* ---------------------------------------------------------------- *
 *  libc bindings (the C sources use malloc/calloc/free/mem*)
 * ---------------------------------------------------------------- */
extern "C" {
    pub fn malloc(size: usize) -> *mut u8;
    pub fn calloc(n: usize, size: usize) -> *mut u8;
    pub fn free(p: *mut u8);
    pub fn memcpy(dst: *mut u8, src: *const u8, n: usize) -> *mut u8;
    pub fn memmove(dst: *mut u8, src: *const u8, n: usize) -> *mut u8;
    pub fn memset(dst: *mut u8, c: i32, n: usize) -> *mut u8;
}

/* ---------------------------------------------------------------- *
 *  Common constants
 * ---------------------------------------------------------------- */
pub const MINMATCH: usize = 4;
pub const WILDCOPYLENGTH: usize = 8;
pub const LASTLITERALS: usize = 5;
pub const MFLIMIT: usize = 12;
pub const MATCH_SAFEGUARD_DISTANCE: usize = (2 * WILDCOPYLENGTH) - MINMATCH; /* 12 */
pub const FASTLOOP_SAFE_DISTANCE: usize = 64;
pub const LZ4_minLength: i32 = (MFLIMIT + 1) as i32; /* 13 */

pub const LZ4_DISTANCE_ABSOLUTE_MAX: u32 = 65535;
pub const LZ4_DISTANCE_MAX: u32 = 65535;

pub const ML_BITS: u32 = 4;
pub const ML_MASK: u32 = (1u32 << ML_BITS) - 1;
pub const RUN_BITS: u32 = 8 - ML_BITS;
pub const RUN_MASK: u32 = (1u32 << RUN_BITS) - 1;

pub const STEPSIZE: usize = 8; /* sizeof(reg_t) on x86_64 */

/* from lz4.h */
pub const LZ4_MEMORY_USAGE: u32 = 14;
pub const LZ4_HASHLOG: u32 = LZ4_MEMORY_USAGE - 2; /* 12 */
pub const LZ4_HASHTABLESIZE: usize = 1usize << LZ4_MEMORY_USAGE; /* bytes */
pub const LZ4_HASH_SIZE_U32: usize = 1usize << LZ4_HASHLOG; /* 4096 */
pub const LZ4_MAX_INPUT_SIZE: i32 = 0x7E00_0000;
pub const LZ4_STREAM_MINSIZE: usize = (1usize << LZ4_MEMORY_USAGE) + 32; /* 16416 */
pub const LZ4_STREAMDECODE_MINSIZE: usize = 32;

pub const LZ4_VERSION_MAJOR: i32 = 1;
pub const LZ4_VERSION_MINOR: i32 = 10;
pub const LZ4_VERSION_RELEASE: i32 = 0;
pub const LZ4_VERSION_NUMBER: i32 =
    LZ4_VERSION_MAJOR * 100 * 100 + LZ4_VERSION_MINOR * 100 + LZ4_VERSION_RELEASE;
pub const LZ4_VERSION_STRING: &[u8] = b"1.10.0\0";

/* from lz4hc.h */
pub const LZ4HC_CLEVEL_MIN: i32 = 2;
pub const LZ4HC_CLEVEL_DEFAULT: i32 = 9;
pub const LZ4HC_CLEVEL_OPT_MIN: i32 = 10;
pub const LZ4HC_CLEVEL_MAX: i32 = 12;

pub const LZ4HC_DICTIONARY_LOGSIZE: u32 = 16;
pub const LZ4HC_MAXD: usize = 1usize << LZ4HC_DICTIONARY_LOGSIZE;
pub const LZ4HC_MAXD_MASK: usize = LZ4HC_MAXD - 1;
pub const LZ4HC_HASH_LOG: u32 = 15;
pub const LZ4HC_HASHTABLESIZE: usize = 1usize << LZ4HC_HASH_LOG;
pub const LZ4HC_HASH_MASK: usize = LZ4HC_HASHTABLESIZE - 1;
pub const LZ4_STREAMHC_MINSIZE: usize = 262200;

/* limitedOutput_directive */
pub const notLimited: i32 = 0;
pub const limitedOutput: i32 = 1;
pub const fillOutput: i32 = 2;

/* tableType_t */
pub const clearedTable: u32 = 0;
pub const byPtr: u32 = 1;
pub const byU32: u32 = 2;
pub const byU16: u32 = 3;

/* dict_directive */
pub const noDict: i32 = 0;
pub const withPrefix64k: i32 = 1;
pub const usingExtDict: i32 = 2;
pub const usingDictCtx: i32 = 3;

/* dictIssue_directive */
pub const noDictIssue: i32 = 0;
pub const dictSmall: i32 = 1;

/* earlyEnd_directive */
pub const decode_full_block: i32 = 0;
pub const partial_decode: i32 = 1;

/* ---------------------------------------------------------------- *
 *  Public structure layouts
 * ---------------------------------------------------------------- */

#[repr(C)]
#[derive(Clone, Copy)]
pub struct LZ4_stream_t_internal {
    pub hashTable: [u32; LZ4_HASH_SIZE_U32],
    pub dictionary: *const u8,
    pub dictCtx: *const LZ4_stream_t_internal,
    pub currentOffset: u32,
    pub tableType: u32,
    pub dictSize: u32,
    /* implicit padding to ensure structure is aligned */
}

pub type LZ4_stream_t = LZ4_stream_t_internal;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct LZ4_streamDecode_t_internal {
    pub externalDict: *const u8,
    pub prefixEnd: *const u8,
    pub extDictSize: usize,
    pub prefixSize: usize,
}

pub type LZ4_streamDecode_t = LZ4_streamDecode_t_internal;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct LZ4HC_CCtx_internal {
    pub hashTable: [u32; LZ4HC_HASHTABLESIZE],
    pub chainTable: [u16; LZ4HC_MAXD],
    pub end: *const u8,
    pub prefixStart: *const u8,
    pub dictStart: *const u8,
    pub dictLimit: u32,
    pub lowLimit: u32,
    pub nextToUpdate: u32,
    pub compressionLevel: i16,
    pub favorDecSpeed: i8,
    pub dirty: i8,
    pub dictCtx: *const LZ4HC_CCtx_internal,
}

/// `union LZ4_streamHC_u` : sized by `minStateSize` (262200), aligned like the
/// internal context (8 bytes).
#[repr(C, align(8))]
pub struct LZ4_streamHC_t {
    pub minStateSize: [u8; LZ4_STREAMHC_MINSIZE],
}

/// `union LZ4_stream_u` : sized by `minStateSize` (16416).
#[repr(C, align(8))]
pub struct LZ4_stream_u {
    pub minStateSize: [u8; LZ4_STREAM_MINSIZE],
}

pub const SIZEOF_LZ4_STREAM_T: usize = LZ4_STREAM_MINSIZE;
pub const SIZEOF_LZ4_STREAMHC_T: usize = LZ4_STREAMHC_MINSIZE;
pub const SIZEOF_LZ4_STREAMDECODE_T: usize = LZ4_STREAMDECODE_MINSIZE;

/* ---------------------------------------------------------------- *
 *  Memory helpers
 * ---------------------------------------------------------------- */

#[inline(always)]
pub fn LZ4_isAligned(p: *const u8, alignment: usize) -> i32 {
    (((p as usize) & (alignment - 1)) == 0) as i32
}

#[inline(always)]
pub unsafe fn MEM_INIT(p: *mut u8, v: i32, s: usize) {
    memset(p, v, s);
}

#[inline(always)]
pub unsafe fn LZ4_memcpy(dst: *mut u8, src: *const u8, n: usize) {
    if n != 0 {
        memcpy(dst, src, n);
    }
}

#[inline(always)]
pub unsafe fn LZ4_memmove(dst: *mut u8, src: *const u8, n: usize) {
    if n != 0 {
        memmove(dst, src, n);
    }
}

#[inline(always)]
pub unsafe fn LZ4_read16(p: *const u8) -> u16 {
    ptr::read_unaligned(p as *const u16)
}
#[inline(always)]
pub unsafe fn LZ4_read32(p: *const u8) -> u32 {
    ptr::read_unaligned(p as *const u32)
}
#[inline(always)]
pub unsafe fn LZ4_read64(p: *const u8) -> u64 {
    ptr::read_unaligned(p as *const u64)
}
#[inline(always)]
pub unsafe fn LZ4_read_ARCH(p: *const u8) -> u64 {
    LZ4_read64(p)
}
#[inline(always)]
pub unsafe fn LZ4_write16(p: *mut u8, v: u16) {
    ptr::write_unaligned(p as *mut u16, v)
}
#[inline(always)]
pub unsafe fn LZ4_write32(p: *mut u8, v: u32) {
    ptr::write_unaligned(p as *mut u32, v)
}
#[inline(always)]
pub unsafe fn LZ4_readLE16(p: *const u8) -> u16 {
    LZ4_read16(p)
}
#[inline(always)]
pub unsafe fn LZ4_readLE64(p: *const u8) -> u64 {
    LZ4_read64(p)
}
#[inline(always)]
pub unsafe fn LZ4_writeLE16(p: *mut u8, v: u16) {
    LZ4_write16(p, v)
}

/// A single 8-byte "load then store", exactly like `__builtin_memcpy(d,s,8)`.
#[inline(always)]
pub unsafe fn copy8(d: *mut u8, s: *const u8) {
    let v = ptr::read_unaligned(s as *const [u8; 8]);
    ptr::write_unaligned(d as *mut [u8; 8], v);
}

/// A single 16-byte "load then store", exactly like `__builtin_memcpy(d,s,16)`.
#[inline(always)]
pub unsafe fn copy16(d: *mut u8, s: *const u8) {
    let v = ptr::read_unaligned(s as *const [u8; 16]);
    ptr::write_unaligned(d as *mut [u8; 16], v);
}

#[inline(always)]
pub unsafe fn copy4(d: *mut u8, s: *const u8) {
    let v = ptr::read_unaligned(s as *const [u8; 4]);
    ptr::write_unaligned(d as *mut [u8; 4], v);
}

#[inline(always)]
pub unsafe fn copy2(d: *mut u8, s: *const u8) {
    let v = ptr::read_unaligned(s as *const [u8; 2]);
    ptr::write_unaligned(d as *mut [u8; 2], v);
}

/// customized variant of memcpy, which can overwrite up to 8 bytes beyond dstEnd
#[inline(always)]
pub unsafe fn LZ4_wildCopy8(dstPtr: *mut u8, srcPtr: *const u8, dstEnd: *mut u8) {
    let mut d = dstPtr;
    let mut s = srcPtr;
    loop {
        copy8(d, s);
        d = d.wrapping_add(8);
        s = s.wrapping_add(8);
        if !(d < dstEnd) {
            break;
        }
    }
}

/// customized variant of memcpy, which can overwrite up to 32 bytes beyond dstEnd
#[inline(always)]
pub unsafe fn LZ4_wildCopy32(dstPtr: *mut u8, srcPtr: *const u8, dstEnd: *mut u8) {
    let mut d = dstPtr;
    let mut s = srcPtr;
    loop {
        copy16(d, s);
        copy16(d.wrapping_add(16), s.wrapping_add(16));
        d = d.wrapping_add(32);
        s = s.wrapping_add(32);
        if !(d < dstEnd) {
            break;
        }
    }
}

pub static inc32table: [u32; 8] = [0, 1, 2, 1, 0, 4, 4, 4];
pub static dec64table: [i32; 8] = [0, 0, 0, -1, -4, 1, 2, 3];

#[inline(always)]
pub unsafe fn LZ4_memcpy_using_offset_base(
    dstPtr: *mut u8,
    srcPtr: *const u8,
    dstEnd: *mut u8,
    offset: usize,
) {
    let mut dstPtr = dstPtr;
    let mut srcPtr = srcPtr;
    if offset < 8 {
        LZ4_write32(dstPtr, 0); /* silence an msan warning when offset==0 */
        *dstPtr.wrapping_add(0) = *srcPtr.wrapping_add(0);
        *dstPtr.wrapping_add(1) = *srcPtr.wrapping_add(1);
        *dstPtr.wrapping_add(2) = *srcPtr.wrapping_add(2);
        *dstPtr.wrapping_add(3) = *srcPtr.wrapping_add(3);
        srcPtr = srcPtr.wrapping_add(inc32table[offset] as usize);
        copy4(dstPtr.wrapping_add(4), srcPtr);
        srcPtr = srcPtr.wrapping_offset(-(dec64table[offset] as isize));
        dstPtr = dstPtr.wrapping_add(8);
    } else {
        copy8(dstPtr, srcPtr);
        dstPtr = dstPtr.wrapping_add(8);
        srcPtr = srcPtr.wrapping_add(8);
    }
    LZ4_wildCopy8(dstPtr, srcPtr, dstEnd);
}

/// LZ4_memcpy_using_offset() presumes :
/// - dstEnd >= dstPtr + MINMATCH
/// - there is at least 12 bytes available to write after dstEnd
#[inline(always)]
pub unsafe fn LZ4_memcpy_using_offset(
    dstPtr: *mut u8,
    srcPtr: *const u8,
    dstEnd: *mut u8,
    offset: usize,
) {
    let mut v: [u8; 8] = [0; 8];
    let mut dstPtr = dstPtr;

    match offset {
        1 => {
            MEM_INIT(v.as_mut_ptr(), *srcPtr as i32, 8);
        }
        2 => {
            LZ4_memcpy(v.as_mut_ptr(), srcPtr, 2);
            LZ4_memcpy(v.as_mut_ptr().wrapping_add(2), srcPtr, 2);
            let tmp = ptr::read_unaligned(v.as_ptr() as *const [u8; 4]);
            ptr::write_unaligned(v.as_mut_ptr().wrapping_add(4) as *mut [u8; 4], tmp);
        }
        4 => {
            LZ4_memcpy(v.as_mut_ptr(), srcPtr, 4);
            LZ4_memcpy(v.as_mut_ptr().wrapping_add(4), srcPtr, 4);
        }
        _ => {
            LZ4_memcpy_using_offset_base(dstPtr, srcPtr, dstEnd, offset);
            return;
        }
    }

    copy8(dstPtr, v.as_ptr());
    dstPtr = dstPtr.wrapping_add(8);
    while dstPtr < dstEnd {
        copy8(dstPtr, v.as_ptr());
        dstPtr = dstPtr.wrapping_add(8);
    }
}

/* ---------------------------------------------------------------- *
 *  Common functions
 * ---------------------------------------------------------------- */

#[inline(always)]
pub fn LZ4_NbCommonBytes(val: u64) -> u32 {
    /* little endian, 64-bit */
    val.trailing_zeros() >> 3
}

#[inline(always)]
pub unsafe fn LZ4_count(pIn: *const u8, pMatch: *const u8, pInLimit: *const u8) -> u32 {
    let pStart = pIn;
    let mut pIn = pIn;
    let mut pMatch = pMatch;

    if pIn < pInLimit.wrapping_sub(STEPSIZE - 1) {
        let diff = LZ4_read_ARCH(pMatch) ^ LZ4_read_ARCH(pIn);
        if diff == 0 {
            pIn = pIn.wrapping_add(STEPSIZE);
            pMatch = pMatch.wrapping_add(STEPSIZE);
        } else {
            return LZ4_NbCommonBytes(diff);
        }
    }

    while pIn < pInLimit.wrapping_sub(STEPSIZE - 1) {
        let diff = LZ4_read_ARCH(pMatch) ^ LZ4_read_ARCH(pIn);
        if diff == 0 {
            pIn = pIn.wrapping_add(STEPSIZE);
            pMatch = pMatch.wrapping_add(STEPSIZE);
            continue;
        }
        pIn = pIn.wrapping_add(LZ4_NbCommonBytes(diff) as usize);
        return (pIn as usize).wrapping_sub(pStart as usize) as u32;
    }

    if (pIn < pInLimit.wrapping_sub(3)) && (LZ4_read32(pMatch) == LZ4_read32(pIn)) {
        pIn = pIn.wrapping_add(4);
        pMatch = pMatch.wrapping_add(4);
    }
    if (pIn < pInLimit.wrapping_sub(1)) && (LZ4_read16(pMatch) == LZ4_read16(pIn)) {
        pIn = pIn.wrapping_add(2);
        pMatch = pMatch.wrapping_add(2);
    }
    if (pIn < pInLimit) && (*pMatch == *pIn) {
        pIn = pIn.wrapping_add(1);
    }
    (pIn as usize).wrapping_sub(pStart as usize) as u32
}

/* pointer distance helpers -------------------------------------- */

#[inline(always)]
pub fn pdiff<T>(a: *const T, b: *const T) -> usize {
    (a as usize).wrapping_sub(b as usize)
}

#[inline(always)]
pub fn pdiff_i<T>(a: *const T, b: *const T) -> isize {
    (a as isize).wrapping_sub(b as isize)
}
