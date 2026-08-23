//! SoftAesBlock type and helpers matching private/softaes.h.
//! softaes_block_encrypt is implemented in package P5 and linked as
//! `_sodium_softaes_block_encrypt` (per quirks.h). The load/store/xor/and
//! helpers are `static inline` in the C header, so they are reproduced here.
#![allow(dead_code)]
use crate::primitives::cutil::{load32_le, store32_le};

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SoftAesBlock {
    pub w0: u32,
    pub w1: u32,
    pub w2: u32,
    pub w3: u32,
}

extern "C" {
    // quirks.h: #define softaes_block_encrypt _sodium_softaes_block_encrypt
    #[link_name = "_sodium_softaes_block_encrypt"]
    pub fn softaes_block_encrypt(block: SoftAesBlock, rk: SoftAesBlock) -> SoftAesBlock;
}

#[inline(always)]
pub unsafe fn softaes_block_load(input: *const u8) -> SoftAesBlock {
    SoftAesBlock {
        w0: load32_le(input.add(0)),
        w1: load32_le(input.add(4)),
        w2: load32_le(input.add(8)),
        w3: load32_le(input.add(12)),
    }
}

#[inline(always)]
pub fn softaes_block_load64x2(a: u64, b: u64) -> SoftAesBlock {
    SoftAesBlock {
        w0: b as u32,
        w1: (b >> 32) as u32,
        w2: a as u32,
        w3: (a >> 32) as u32,
    }
}

#[inline(always)]
pub unsafe fn softaes_block_store(out: *mut u8, input: SoftAesBlock) {
    store32_le(out.add(0), input.w0);
    store32_le(out.add(4), input.w1);
    store32_le(out.add(8), input.w2);
    store32_le(out.add(12), input.w3);
}

#[inline(always)]
pub fn softaes_block_xor(a: SoftAesBlock, b: SoftAesBlock) -> SoftAesBlock {
    SoftAesBlock {
        w0: a.w0 ^ b.w0,
        w1: a.w1 ^ b.w1,
        w2: a.w2 ^ b.w2,
        w3: a.w3 ^ b.w3,
    }
}

#[inline(always)]
pub fn softaes_block_and(a: SoftAesBlock, b: SoftAesBlock) -> SoftAesBlock {
    SoftAesBlock {
        w0: a.w0 & b.w0,
        w1: a.w1 & b.w1,
        w2: a.w2 & b.w2,
        w3: a.w3 & b.w3,
    }
}
