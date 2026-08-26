//! Translation of `include/sodium/private/common.h` helpers plus small shared
//! utilities used across the crate.
//!
//! The reference build defines no `NATIVE_LITTLE_ENDIAN` / `NATIVE_BIG_ENDIAN`
//! macros, so the byte-shuffling fallbacks are used.  On any target these
//! produce identical results to the memcpy paths on a little-endian machine.

#[inline(always)]
pub fn rotl32(x: u32, b: i32) -> u32 {
    (x << b) | (x >> (32 - b))
}

#[inline(always)]
pub fn rotl64(x: u64, b: i32) -> u64 {
    (x << b) | (x >> (64 - b))
}

#[inline(always)]
pub fn rotr32(x: u32, b: i32) -> u32 {
    (x >> b) | (x << (32 - b))
}

#[inline(always)]
pub fn rotr64(x: u64, b: i32) -> u64 {
    (x >> b) | (x << (64 - b))
}

#[inline(always)]
pub unsafe fn load64_le(src: *const u8) -> u64 {
    let mut w = *src.add(0) as u64;
    w |= (*src.add(1) as u64) << 8;
    w |= (*src.add(2) as u64) << 16;
    w |= (*src.add(3) as u64) << 24;
    w |= (*src.add(4) as u64) << 32;
    w |= (*src.add(5) as u64) << 40;
    w |= (*src.add(6) as u64) << 48;
    w |= (*src.add(7) as u64) << 56;
    w
}

#[inline(always)]
pub unsafe fn store64_le(dst: *mut u8, mut w: u64) {
    *dst.add(0) = w as u8;
    w >>= 8;
    *dst.add(1) = w as u8;
    w >>= 8;
    *dst.add(2) = w as u8;
    w >>= 8;
    *dst.add(3) = w as u8;
    w >>= 8;
    *dst.add(4) = w as u8;
    w >>= 8;
    *dst.add(5) = w as u8;
    w >>= 8;
    *dst.add(6) = w as u8;
    w >>= 8;
    *dst.add(7) = w as u8;
}

#[inline(always)]
pub unsafe fn load32_le(src: *const u8) -> u32 {
    let mut w = *src.add(0) as u32;
    w |= (*src.add(1) as u32) << 8;
    w |= (*src.add(2) as u32) << 16;
    w |= (*src.add(3) as u32) << 24;
    w
}

#[inline(always)]
pub unsafe fn store32_le(dst: *mut u8, mut w: u32) {
    *dst.add(0) = w as u8;
    w >>= 8;
    *dst.add(1) = w as u8;
    w >>= 8;
    *dst.add(2) = w as u8;
    w >>= 8;
    *dst.add(3) = w as u8;
}

#[inline(always)]
pub unsafe fn load64_be(src: *const u8) -> u64 {
    let mut w = *src.add(7) as u64;
    w |= (*src.add(6) as u64) << 8;
    w |= (*src.add(5) as u64) << 16;
    w |= (*src.add(4) as u64) << 24;
    w |= (*src.add(3) as u64) << 32;
    w |= (*src.add(2) as u64) << 40;
    w |= (*src.add(1) as u64) << 48;
    w |= (*src.add(0) as u64) << 56;
    w
}

#[inline(always)]
pub unsafe fn store64_be(dst: *mut u8, mut w: u64) {
    *dst.add(7) = w as u8;
    w >>= 8;
    *dst.add(6) = w as u8;
    w >>= 8;
    *dst.add(5) = w as u8;
    w >>= 8;
    *dst.add(4) = w as u8;
    w >>= 8;
    *dst.add(3) = w as u8;
    w >>= 8;
    *dst.add(2) = w as u8;
    w >>= 8;
    *dst.add(1) = w as u8;
    w >>= 8;
    *dst.add(0) = w as u8;
}

#[inline(always)]
pub unsafe fn load32_be(src: *const u8) -> u32 {
    let mut w = *src.add(3) as u32;
    w |= (*src.add(2) as u32) << 8;
    w |= (*src.add(1) as u32) << 16;
    w |= (*src.add(0) as u32) << 24;
    w
}

#[inline(always)]
pub unsafe fn store32_be(dst: *mut u8, mut w: u32) {
    *dst.add(3) = w as u8;
    w >>= 8;
    *dst.add(2) = w as u8;
    w >>= 8;
    *dst.add(1) = w as u8;
    w >>= 8;
    *dst.add(0) = w as u8;
}

#[inline(always)]
pub unsafe fn xor_buf(out: *mut u8, in_: *const u8, n: usize) {
    for i in 0..n {
        *out.add(i) ^= *in_.add(i);
    }
}

/// `SODIUM_SIZE_MAX` == min(UINT64_MAX, SIZE_MAX)
pub const SODIUM_SIZE_MAX: u64 = if (u64::MAX as u128) < (usize::MAX as u128) {
    u64::MAX
} else {
    usize::MAX as u64
};

/// Small helpers mirroring libc functions used throughout the C sources.
#[inline(always)]
pub unsafe fn memcpy(dst: *mut u8, src: *const u8, n: usize) {
    if n != 0 {
        core::ptr::copy_nonoverlapping(src, dst, n);
    }
}

#[inline(always)]
pub unsafe fn memmove(dst: *mut u8, src: *const u8, n: usize) {
    if n != 0 {
        core::ptr::copy(src, dst, n);
    }
}

#[inline(always)]
pub unsafe fn memset(dst: *mut u8, c: u8, n: usize) {
    if n != 0 {
        core::ptr::write_bytes(dst, c, n);
    }
}
