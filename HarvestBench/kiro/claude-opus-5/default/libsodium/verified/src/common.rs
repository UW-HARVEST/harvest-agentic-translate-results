//! Shared helpers corresponding to `libsodium/include/sodium/private/common.h`.
//!
//! `NATIVE_LITTLE_ENDIAN` / `NATIVE_BIG_ENDIAN` are *not* defined by the
//! reference build, so the portable byte-shuffling variants are reproduced.

#[inline(always)]
pub fn rotl32(x: u32, b: i32) -> u32 {
    x.rotate_left(b as u32)
}

#[inline(always)]
pub fn rotl64(x: u64, b: i32) -> u64 {
    x.rotate_left(b as u32)
}

#[inline(always)]
pub fn rotr32(x: u32, b: i32) -> u32 {
    x.rotate_right(b as u32)
}

#[inline(always)]
pub fn rotr64(x: u64, b: i32) -> u64 {
    x.rotate_right(b as u32)
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
pub unsafe fn xor_buf(out: *mut u8, inp: *const u8, n: usize) {
    for i in 0..n {
        *out.add(i) ^= *inp.add(i);
    }
}

/// `SODIUM_SIZE_MAX` == min(UINT64_MAX, SIZE_MAX) == usize::MAX on 64-bit.
pub const SODIUM_SIZE_MAX: usize = usize::MAX;
