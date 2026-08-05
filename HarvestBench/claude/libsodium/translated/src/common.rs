//! Shared inline helpers translated from private/common.h
#![allow(dead_code)]

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
pub fn load64_le(src: &[u8]) -> u64 {
    let mut w = src[0] as u64;
    w |= (src[1] as u64) << 8;
    w |= (src[2] as u64) << 16;
    w |= (src[3] as u64) << 24;
    w |= (src[4] as u64) << 32;
    w |= (src[5] as u64) << 40;
    w |= (src[6] as u64) << 48;
    w |= (src[7] as u64) << 56;
    w
}

#[inline(always)]
pub fn store64_le(dst: &mut [u8], mut w: u64) {
    dst[0] = w as u8;
    w >>= 8;
    dst[1] = w as u8;
    w >>= 8;
    dst[2] = w as u8;
    w >>= 8;
    dst[3] = w as u8;
    w >>= 8;
    dst[4] = w as u8;
    w >>= 8;
    dst[5] = w as u8;
    w >>= 8;
    dst[6] = w as u8;
    w >>= 8;
    dst[7] = w as u8;
}

#[inline(always)]
pub fn load32_le(src: &[u8]) -> u32 {
    let mut w = src[0] as u32;
    w |= (src[1] as u32) << 8;
    w |= (src[2] as u32) << 16;
    w |= (src[3] as u32) << 24;
    w
}

#[inline(always)]
pub fn store32_le(dst: &mut [u8], mut w: u32) {
    dst[0] = w as u8;
    w >>= 8;
    dst[1] = w as u8;
    w >>= 8;
    dst[2] = w as u8;
    w >>= 8;
    dst[3] = w as u8;
}

#[inline(always)]
pub fn load64_be(src: &[u8]) -> u64 {
    let mut w = src[7] as u64;
    w |= (src[6] as u64) << 8;
    w |= (src[5] as u64) << 16;
    w |= (src[4] as u64) << 24;
    w |= (src[3] as u64) << 32;
    w |= (src[2] as u64) << 40;
    w |= (src[1] as u64) << 48;
    w |= (src[0] as u64) << 56;
    w
}

#[inline(always)]
pub fn store64_be(dst: &mut [u8], mut w: u64) {
    dst[7] = w as u8;
    w >>= 8;
    dst[6] = w as u8;
    w >>= 8;
    dst[5] = w as u8;
    w >>= 8;
    dst[4] = w as u8;
    w >>= 8;
    dst[3] = w as u8;
    w >>= 8;
    dst[2] = w as u8;
    w >>= 8;
    dst[1] = w as u8;
    w >>= 8;
    dst[0] = w as u8;
}

#[inline(always)]
pub fn load32_be(src: &[u8]) -> u32 {
    let mut w = src[3] as u32;
    w |= (src[2] as u32) << 8;
    w |= (src[1] as u32) << 16;
    w |= (src[0] as u32) << 24;
    w
}

#[inline(always)]
pub fn store32_be(dst: &mut [u8], mut w: u32) {
    dst[3] = w as u8;
    w >>= 8;
    dst[2] = w as u8;
    w >>= 8;
    dst[1] = w as u8;
    w >>= 8;
    dst[0] = w as u8;
}

#[inline(always)]
pub fn xor_buf(out: &mut [u8], inp: &[u8], n: usize) {
    for i in 0..n {
        out[i] ^= inp[i];
    }
}
