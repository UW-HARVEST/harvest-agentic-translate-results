//! Shared low-level helpers, mirroring the common part of `lz4.c`
//! (the section guarded by `LZ4_COMMONDEFS_ONLY`).
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]

use core::ptr;

/* ===== Common constants (lz4.c) ===== */
pub const MINMATCH: usize = 4;
pub const WILDCOPYLENGTH: usize = 8;
pub const LASTLITERALS: usize = 5;
pub const MFLIMIT: usize = 12;
pub const MATCH_SAFEGUARD_DISTANCE: usize = (2 * WILDCOPYLENGTH) - MINMATCH;
pub const FASTLOOP_SAFE_DISTANCE: isize = 64;
pub const LZ4_minLength: i32 = (MFLIMIT + 1) as i32;

pub const ML_BITS: u32 = 4;
pub const ML_MASK: u32 = (1u32 << ML_BITS) - 1;
pub const RUN_BITS: u32 = 8 - ML_BITS;
pub const RUN_MASK: u32 = (1u32 << RUN_BITS) - 1;

pub const LZ4_DISTANCE_MAX: u32 = 65535;
pub const LZ4_DISTANCE_ABSOLUTE_MAX: u32 = 65535;

pub const LZ4_MAX_INPUT_SIZE: u32 = 0x7E00_0000;

pub const LZ4_MEMORY_USAGE: u32 = 14;
pub const LZ4_HASHLOG: u32 = LZ4_MEMORY_USAGE - 2;
pub const LZ4_HASHTABLESIZE: usize = 1usize << LZ4_MEMORY_USAGE;
pub const LZ4_HASH_SIZE_U32: usize = 1usize << LZ4_HASHLOG;

pub const KB: usize = 1024;

/// `reg_t` on x86_64
pub type RegT = u64;
pub const STEPSIZE: usize = 8;

/* ===== limitedOutput_directive ===== */
pub const notLimited: i32 = 0;
pub const limitedOutput: i32 = 1;
pub const fillOutput: i32 = 2;

/* ===== tableType_t ===== */
pub const clearedTable: u32 = 0;
pub const byPtr: u32 = 1;
pub const byU32: u32 = 2;
pub const byU16: u32 = 3;

/* ===== dict_directive ===== */
pub const noDict: i32 = 0;
pub const withPrefix64k: i32 = 1;
pub const usingExtDict: i32 = 2;
pub const usingDictCtx: i32 = 3;

/* ===== dictIssue_directive ===== */
pub const noDictIssue: i32 = 0;
pub const dictSmall: i32 = 1;

/* ===== earlyEnd_directive ===== */
pub const decode_full_block: i32 = 0;
pub const partial_decode: i32 = 1;

/* ===== pointer helpers (wrapping, to match C's raw address arithmetic) ===== */
#[inline(always)]
pub fn cadd(p: *const u8, n: usize) -> *const u8 {
    p.wrapping_add(n)
}
#[inline(always)]
pub fn csub(p: *const u8, n: usize) -> *const u8 {
    p.wrapping_sub(n)
}
#[inline(always)]
pub fn coff(p: *const u8, n: isize) -> *const u8 {
    p.wrapping_offset(n)
}
#[inline(always)]
pub fn madd(p: *mut u8, n: usize) -> *mut u8 {
    p.wrapping_add(n)
}
#[inline(always)]
pub fn msub(p: *mut u8, n: usize) -> *mut u8 {
    p.wrapping_sub(n)
}
#[inline(always)]
pub fn moff(p: *mut u8, n: isize) -> *mut u8 {
    p.wrapping_offset(n)
}
/// `a - b` in bytes
#[inline(always)]
pub fn pdiff(a: *const u8, b: *const u8) -> isize {
    (a as isize).wrapping_sub(b as isize)
}

/* ===== unaligned memory access (little endian host) ===== */
#[inline(always)]
pub unsafe fn LZ4_read16(p: *const u8) -> u16 {
    unsafe { (p as *const u16).read_unaligned() }
}
#[inline(always)]
pub unsafe fn LZ4_read32(p: *const u8) -> u32 {
    unsafe { (p as *const u32).read_unaligned() }
}
#[inline(always)]
pub unsafe fn LZ4_read64(p: *const u8) -> u64 {
    unsafe { (p as *const u64).read_unaligned() }
}
#[inline(always)]
pub unsafe fn LZ4_read_ARCH(p: *const u8) -> RegT {
    unsafe { LZ4_read64(p) }
}
#[inline(always)]
pub unsafe fn LZ4_write16(p: *mut u8, v: u16) {
    unsafe { (p as *mut u16).write_unaligned(v) }
}
#[inline(always)]
pub unsafe fn LZ4_write32(p: *mut u8, v: u32) {
    unsafe { (p as *mut u32).write_unaligned(v) }
}
#[inline(always)]
pub unsafe fn LZ4_readLE16(p: *const u8) -> u16 {
    unsafe { LZ4_read16(p) }
}
#[inline(always)]
pub unsafe fn LZ4_readLE64(p: *const u8) -> u64 {
    unsafe { LZ4_read64(p) }
}
#[inline(always)]
pub unsafe fn LZ4_writeLE16(p: *mut u8, v: u16) {
    unsafe { LZ4_write16(p, v) }
}

#[inline(always)]
pub unsafe fn LZ4_memcpy(dst: *mut u8, src: *const u8, size: usize) {
    unsafe { ptr::copy_nonoverlapping(src, dst, size) }
}
#[inline(always)]
pub unsafe fn LZ4_memmove(dst: *mut u8, src: *const u8, size: usize) {
    unsafe { ptr::copy(src, dst, size) }
}
#[inline(always)]
pub unsafe fn MEM_INIT(p: *mut u8, v: u8, size: usize) {
    unsafe { ptr::write_bytes(p, v, size) }
}

#[inline(always)]
pub fn LZ4_isAligned(p: *const u8, alignment: usize) -> i32 {
    (((p as usize) & (alignment - 1)) == 0) as i32
}

/// customized variant of memcpy, which can overwrite up to 8 bytes beyond dstEnd
#[inline(always)]
pub unsafe fn LZ4_wildCopy8(dstPtr: *mut u8, srcPtr: *const u8, dstEnd: *mut u8) {
    unsafe {
        let mut d = dstPtr;
        let mut s = srcPtr;
        let e = dstEnd;
        loop {
            LZ4_memcpy(d, s, 8);
            d = madd(d, 8);
            s = cadd(s, 8);
            if !(d < e) {
                break;
            }
        }
    }
}

/// copies two times 16 bytes; can overwrite up to 32 bytes beyond dstEnd
#[inline(always)]
pub unsafe fn LZ4_wildCopy32(dstPtr: *mut u8, srcPtr: *const u8, dstEnd: *mut u8) {
    unsafe {
        let mut d = dstPtr;
        let mut s = srcPtr;
        let e = dstEnd;
        loop {
            LZ4_memcpy(d, s, 16);
            LZ4_memcpy(madd(d, 16), cadd(s, 16), 16);
            d = madd(d, 32);
            s = cadd(s, 32);
            if !(d < e) {
                break;
            }
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
    unsafe {
        let mut d = dstPtr;
        let mut s = srcPtr;
        if offset < 8 {
            LZ4_write32(d, 0);
            *d.wrapping_add(0) = *s.wrapping_add(0);
            *d.wrapping_add(1) = *s.wrapping_add(1);
            *d.wrapping_add(2) = *s.wrapping_add(2);
            *d.wrapping_add(3) = *s.wrapping_add(3);
            s = cadd(s, inc32table[offset] as usize);
            LZ4_memcpy(madd(d, 4), s, 4);
            s = coff(s, -(dec64table[offset] as isize));
            d = madd(d, 8);
        } else {
            LZ4_memcpy(d, s, 8);
            d = madd(d, 8);
            s = cadd(s, 8);
        }
        LZ4_wildCopy8(d, s, dstEnd);
    }
}

#[inline(always)]
pub unsafe fn LZ4_memcpy_using_offset(
    dstPtr: *mut u8,
    srcPtr: *const u8,
    dstEnd: *mut u8,
    offset: usize,
) {
    unsafe {
        let mut v: [u8; 8] = [0; 8];
        let mut d = dstPtr;
        match offset {
            1 => {
                MEM_INIT(v.as_mut_ptr(), *srcPtr, 8);
            }
            2 => {
                LZ4_memcpy(v.as_mut_ptr(), srcPtr, 2);
                LZ4_memcpy(v.as_mut_ptr().add(2), srcPtr, 2);
                let tmp = LZ4_read32(v.as_ptr());
                LZ4_write32(v.as_mut_ptr().add(4), tmp);
            }
            4 => {
                LZ4_memcpy(v.as_mut_ptr(), srcPtr, 4);
                LZ4_memcpy(v.as_mut_ptr().add(4), srcPtr, 4);
            }
            _ => {
                LZ4_memcpy_using_offset_base(dstPtr, srcPtr, dstEnd, offset);
                return;
            }
        }
        LZ4_memcpy(d, v.as_ptr(), 8);
        d = madd(d, 8);
        while d < dstEnd {
            LZ4_memcpy(d, v.as_ptr(), 8);
            d = madd(d, 8);
        }
    }
}

/// LE, 64-bit: `__builtin_ctzll(val) >> 3`
#[inline(always)]
pub fn LZ4_NbCommonBytes(val: RegT) -> u32 {
    val.trailing_zeros() >> 3
}

#[inline(always)]
pub unsafe fn LZ4_count(pIn0: *const u8, pMatch0: *const u8, pInLimit: *const u8) -> u32 {
    unsafe {
        let pStart = pIn0;
        let mut pIn = pIn0;
        let mut pMatch = pMatch0;

        if pIn < csub(pInLimit, STEPSIZE - 1) {
            let diff = LZ4_read_ARCH(pMatch) ^ LZ4_read_ARCH(pIn);
            if diff == 0 {
                pIn = cadd(pIn, STEPSIZE);
                pMatch = cadd(pMatch, STEPSIZE);
            } else {
                return LZ4_NbCommonBytes(diff);
            }
        }

        while pIn < csub(pInLimit, STEPSIZE - 1) {
            let diff = LZ4_read_ARCH(pMatch) ^ LZ4_read_ARCH(pIn);
            if diff == 0 {
                pIn = cadd(pIn, STEPSIZE);
                pMatch = cadd(pMatch, STEPSIZE);
                continue;
            }
            pIn = cadd(pIn, LZ4_NbCommonBytes(diff) as usize);
            return pdiff(pIn, pStart) as u32;
        }

        if (STEPSIZE == 8) && (pIn < csub(pInLimit, 3)) && (LZ4_read32(pMatch) == LZ4_read32(pIn)) {
            pIn = cadd(pIn, 4);
            pMatch = cadd(pMatch, 4);
        }
        if (pIn < csub(pInLimit, 1)) && (LZ4_read16(pMatch) == LZ4_read16(pIn)) {
            pIn = cadd(pIn, 2);
            pMatch = cadd(pMatch, 2);
        }
        if (pIn < pInLimit) && (*pMatch == *pIn) {
            pIn = cadd(pIn, 1);
        }
        pdiff(pIn, pStart) as u32
    }
}

/* ===== LZ4 public structures ===== */

#[repr(C)]
#[derive(Clone, Copy)]
pub struct LZ4_stream_t_internal {
    pub hashTable: [u32; LZ4_HASH_SIZE_U32],
    pub dictionary: *const u8,
    pub dictCtx: *const LZ4_stream_t_internal,
    pub currentOffset: u32,
    pub tableType: u32,
    pub dictSize: u32,
}

/// `LZ4_stream_t`: the union of `char[LZ4_STREAM_MINSIZE]` and the internal
/// struct. Both are exactly 16416 bytes on LP64.
pub type LZ4_stream_t = LZ4_stream_t_internal;

pub const LZ4_STREAM_MINSIZE: usize = (1usize << LZ4_MEMORY_USAGE) + 32;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct LZ4_streamDecode_t_internal {
    pub externalDict: *const u8,
    pub prefixEnd: *const u8,
    pub extDictSize: usize,
    pub prefixSize: usize,
}
pub type LZ4_streamDecode_t = LZ4_streamDecode_t_internal;
pub const LZ4_STREAMDECODE_MINSIZE: usize = 32;

/* ===== LZ4HC public structures ===== */
pub const LZ4HC_DICTIONARY_LOGSIZE: usize = 16;
pub const LZ4HC_MAXD: usize = 1usize << LZ4HC_DICTIONARY_LOGSIZE;
pub const LZ4HC_HASH_LOG: u32 = 15;
pub const LZ4HC_HASHTABLESIZE: usize = 1usize << LZ4HC_HASH_LOG;

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

pub const LZ4_STREAMHC_MINSIZE: usize = 262200;

/// `LZ4_streamHC_t`: union of `char[262200]` and `LZ4HC_CCtx_internal`
/// (262192 bytes) -- the union is therefore 262200 bytes.
#[repr(C)]
pub struct LZ4_streamHC_t {
    pub internal_donotuse: LZ4HC_CCtx_internal,
    pub _tail: [u8; LZ4_STREAMHC_MINSIZE - 262192],
}

const _: () = {
    assert!(core::mem::size_of::<LZ4_stream_t_internal>() == LZ4_STREAM_MINSIZE);
    assert!(core::mem::size_of::<LZ4_streamDecode_t_internal>() == LZ4_STREAMDECODE_MINSIZE);
    assert!(core::mem::size_of::<LZ4HC_CCtx_internal>() == 262192);
    assert!(core::mem::size_of::<LZ4_streamHC_t>() == LZ4_STREAMHC_MINSIZE);
};
