//! Pointer-based LE/BE load/store helpers matching private/common.h byte-by-byte paths.
#![allow(dead_code)]

use core::ffi::c_int;

extern "C" {
    pub fn sodium_misuse() -> !;
    pub fn sodium_memzero(pnt: *mut core::ffi::c_void, len: usize);
    pub fn randombytes_buf(buf: *mut core::ffi::c_void, size: usize);
    pub fn crypto_verify_16(x: *const u8, y: *const u8) -> c_int;
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
pub fn rotl32(x: u32, b: u32) -> u32 {
    (x << b) | (x >> (32 - b))
}

#[inline(always)]
pub fn rotl64(x: u64, b: u32) -> u64 {
    (x << b) | (x >> (64 - b))
}

#[inline(always)]
pub fn rotr32(x: u32, b: u32) -> u32 {
    (x >> b) | (x << (32 - b))
}

#[inline(always)]
pub fn rotr64(x: u64, b: u32) -> u64 {
    (x >> b) | (x << (64 - b))
}
