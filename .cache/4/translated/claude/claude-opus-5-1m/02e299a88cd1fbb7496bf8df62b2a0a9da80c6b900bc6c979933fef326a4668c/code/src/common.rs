//! Shared constants and low-level memory helpers, mirroring the common section of `lz4.c`.

use core::ptr;

/*-************************************
*  Common Constants
**************************************/
pub const MINMATCH: usize = 4;

pub const WILDCOPYLENGTH: usize = 8;
pub const LASTLITERALS: usize = 5;
pub const MFLIMIT: usize = 12;
/// ensure it's possible to write 2 x wildcopyLength without overflowing output buffer
pub const MATCH_SAFEGUARD_DISTANCE: usize = (2 * WILDCOPYLENGTH) - MINMATCH;
pub const FASTLOOP_SAFE_DISTANCE: usize = 64;
pub const LZ4_minLength: i32 = (MFLIMIT + 1) as i32;

pub const LZ4_DISTANCE_ABSOLUTE_MAX: u32 = 65535;
pub const LZ4_DISTANCE_MAX: u32 = 65535;

pub const ML_BITS: u32 = 4;
pub const ML_MASK: u32 = (1u32 << ML_BITS) - 1;
pub const RUN_BITS: u32 = 8 - ML_BITS;
pub const RUN_MASK: u32 = (1u32 << RUN_BITS) - 1;

pub const LZ4_MEMORY_USAGE: u32 = 14;
pub const LZ4_HASHLOG: u32 = LZ4_MEMORY_USAGE - 2;
/// size in *bytes* of the hash table
pub const LZ4_HASHTABLESIZE: usize = 1usize << LZ4_MEMORY_USAGE;
/// number of U32 entries in the hash table
pub const LZ4_HASH_SIZE_U32: usize = 1usize << LZ4_HASHLOG;

pub const LZ4_MAX_INPUT_SIZE: i32 = 0x7E000000;

pub const LZ4_ACCELERATION_DEFAULT: i32 = 1;
pub const LZ4_ACCELERATION_MAX: i32 = 65537;

pub const LZ4_64Klimit: i32 = (64 * 1024) + (MFLIMIT as i32 - 1);
pub const LZ4_skipTrigger: u32 = 6;

pub const LZ4_VERSION_MAJOR: i32 = 1;
pub const LZ4_VERSION_MINOR: i32 = 10;
pub const LZ4_VERSION_RELEASE: i32 = 0;
pub const LZ4_VERSION_NUMBER: i32 =
    LZ4_VERSION_MAJOR * 100 * 100 + LZ4_VERSION_MINOR * 100 + LZ4_VERSION_RELEASE;
pub const LZ4_VERSION_STRING: &[u8] = b"1.10.0\0";

/// `LZ4_COMPRESSBOUND(isize)`
#[inline]
pub fn LZ4_COMPRESSBOUND(isize_: i32) -> i32 {
    if (isize_ as u32) > (LZ4_MAX_INPUT_SIZE as u32) {
        0
    } else {
        isize_
            .wrapping_add(isize_ / 255)
            .wrapping_add(16)
    }
}

#[inline]
pub fn LZ4_DECODER_RING_BUFFER_SIZE(maxBlockSize: i32) -> i32 {
    65536i32
        .wrapping_add(14)
        .wrapping_add(maxBlockSize)
}

/*-************************************
*  Types
**************************************/
#[cfg(target_arch = "x86_64")]
pub type RegT = u64;
#[cfg(not(target_arch = "x86_64"))]
pub type RegT = usize;

pub const STEPSIZE: usize = core::mem::size_of::<RegT>();

/// `limitedOutput_directive`
pub const notLimited: i32 = 0;
pub const limitedOutput: i32 = 1;
pub const fillOutput: i32 = 2;

/// `tableType_t`
pub const clearedTable: i32 = 0;
pub const byPtr: i32 = 1;
pub const byU32: i32 = 2;
pub const byU16: i32 = 3;

/// `dict_directive`
pub const noDict: i32 = 0;
pub const withPrefix64k: i32 = 1;
pub const usingExtDict: i32 = 2;
pub const usingDictCtx: i32 = 3;

/// `dictIssue_directive`
pub const noDictIssue: i32 = 0;
pub const dictSmall: i32 = 1;

/// `earlyEnd_directive`
pub const decode_full_block: i32 = 0;
pub const partial_decode: i32 = 1;

#[inline(always)]
pub fn LZ4_isAligned(p: *const u8, alignment: usize) -> bool {
    ((p as usize) & (alignment - 1)) == 0
}

#[inline(always)]
pub fn LZ4_isLittleEndian() -> bool {
    cfg!(target_endian = "little")
}

/*-************************************
*  Reading and writing into memory
**************************************/
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
pub unsafe fn LZ4_read_ARCH(p: *const u8) -> RegT {
    ptr::read_unaligned(p as *const RegT)
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
    if LZ4_isLittleEndian() {
        LZ4_read16(p)
    } else {
        (*p as u16) | ((*p.wrapping_add(1) as u16) << 8)
    }
}

#[inline(always)]
pub unsafe fn LZ4_readLE64(p: *const u8) -> u64 {
    if LZ4_isLittleEndian() {
        LZ4_read64(p)
    } else {
        let mut v: u64 = 0;
        let mut i = 0usize;
        while i < 8 {
            v |= (*p.wrapping_add(i) as u64) << (8 * i);
            i += 1;
        }
        v
    }
}

#[inline(always)]
pub unsafe fn LZ4_writeLE16(p: *mut u8, v: u16) {
    if LZ4_isLittleEndian() {
        LZ4_write16(p, v)
    } else {
        *p = v as u8;
        *p.wrapping_add(1) = (v >> 8) as u8;
    }
}

/* fixed-size copies; implemented as read-then-write so that overlapping
 * regions behave exactly like the machine code generated for `memcpy` with a
 * constant size. */
#[inline(always)]
pub unsafe fn copy2(d: *mut u8, s: *const u8) {
    let v = ptr::read_unaligned(s as *const [u8; 2]);
    ptr::write_unaligned(d as *mut [u8; 2], v);
}

#[inline(always)]
pub unsafe fn copy4(d: *mut u8, s: *const u8) {
    let v = ptr::read_unaligned(s as *const [u8; 4]);
    ptr::write_unaligned(d as *mut [u8; 4], v);
}

#[inline(always)]
pub unsafe fn copy8(d: *mut u8, s: *const u8) {
    let v = ptr::read_unaligned(s as *const [u8; 8]);
    ptr::write_unaligned(d as *mut [u8; 8], v);
}

#[inline(always)]
pub unsafe fn copy16(d: *mut u8, s: *const u8) {
    let v = ptr::read_unaligned(s as *const [u8; 16]);
    ptr::write_unaligned(d as *mut [u8; 16], v);
}

/// memcpy for a runtime length (regions must not overlap, like C `memcpy`);
/// implemented with `ptr::copy` (memmove semantics) so it is always well defined.
#[inline(always)]
pub unsafe fn mem_copy(d: *mut u8, s: *const u8, n: usize) {
    ptr::copy(s, d, n);
}

/// memmove for a runtime length
#[inline(always)]
pub unsafe fn mem_move(d: *mut u8, s: *const u8, n: usize) {
    ptr::copy(s, d, n);
}

#[inline(always)]
pub unsafe fn mem_init(d: *mut u8, v: u8, n: usize) {
    ptr::write_bytes(d, v, n);
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
    let mut d = dstPtr;
    let mut s = srcPtr;
    if offset < 8 {
        LZ4_write32(d, 0); /* silence an msan warning when offset==0 */
        *d.wrapping_add(0) = *s.wrapping_add(0);
        *d.wrapping_add(1) = *s.wrapping_add(1);
        *d.wrapping_add(2) = *s.wrapping_add(2);
        *d.wrapping_add(3) = *s.wrapping_add(3);
        s = s.wrapping_add(inc32table[offset] as usize);
        copy4(d.wrapping_add(4), s);
        s = s.wrapping_offset(-(dec64table[offset] as isize));
        d = d.wrapping_add(8);
    } else {
        copy8(d, s);
        d = d.wrapping_add(8);
        s = s.wrapping_add(8);
    }

    LZ4_wildCopy8(d, s, dstEnd);
}

#[inline(always)]
pub unsafe fn LZ4_memcpy_using_offset(
    dstPtr: *mut u8,
    srcPtr: *const u8,
    dstEnd: *mut u8,
    offset: usize,
) {
    let mut v: [u8; 8] = [0; 8];
    let mut d = dstPtr;

    match offset {
        1 => {
            mem_init(v.as_mut_ptr(), *srcPtr, 8);
        }
        2 => {
            copy2(v.as_mut_ptr(), srcPtr);
            copy2(v.as_mut_ptr().wrapping_add(2), srcPtr);
            copy4(v.as_mut_ptr().wrapping_add(4), v.as_ptr());
        }
        4 => {
            copy4(v.as_mut_ptr(), srcPtr);
            copy4(v.as_mut_ptr().wrapping_add(4), srcPtr);
        }
        _ => {
            LZ4_memcpy_using_offset_base(dstPtr, srcPtr, dstEnd, offset);
            return;
        }
    }

    copy8(d, v.as_ptr());
    d = d.wrapping_add(8);
    while d < dstEnd {
        copy8(d, v.as_ptr());
        d = d.wrapping_add(8);
    }
}

/*-************************************
*  Common functions
**************************************/
#[inline(always)]
pub fn LZ4_NbCommonBytes(val: RegT) -> u32 {
    if LZ4_isLittleEndian() {
        if core::mem::size_of::<RegT>() == 8 {
            (val as u64).trailing_zeros() >> 3
        } else {
            (val as u32).trailing_zeros() >> 3
        }
    } else {
        if core::mem::size_of::<RegT>() == 8 {
            (val as u64).leading_zeros() >> 3
        } else {
            (val as u32).leading_zeros() >> 3
        }
    }
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
        return (pIn as usize - pStart as usize) as u32;
    }

    if (STEPSIZE == 8)
        && (pIn < pInLimit.wrapping_sub(3))
        && (LZ4_read32(pMatch) == LZ4_read32(pIn))
    {
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
    (pIn as usize - pStart as usize) as u32
}

/*-************************************
*  libc bindings (memory + stdio)
**************************************/
pub type c_void = core::ffi::c_void;

extern "C" {
    pub fn malloc(size: usize) -> *mut c_void;
    pub fn calloc(nmemb: usize, size: usize) -> *mut c_void;
    pub fn free(ptr: *mut c_void);
}
