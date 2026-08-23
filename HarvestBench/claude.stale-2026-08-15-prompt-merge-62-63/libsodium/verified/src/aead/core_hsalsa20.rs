//! Translated from crypto_core/hsalsa20/ref2/core_hsalsa20_ref2.c
use crate::primitives::cutil::{load32_le, rotl32, store32_le};

const ROUNDS: i32 = 20;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_hsalsa20(
    out: *mut u8,
    input: *const u8,
    k: *const u8,
    c: *const u8,
) -> i32 {
    let (mut x0, x5, x10, x15);
    if c.is_null() {
        x0 = 0x61707865u32;
        x5 = 0x3320646eu32;
        x10 = 0x79622d32u32;
        x15 = 0x6b206574u32;
    } else {
        x0 = load32_le(c.add(0));
        x5 = load32_le(c.add(4));
        x10 = load32_le(c.add(8));
        x15 = load32_le(c.add(12));
    }
    let mut x5 = x5;
    let mut x10 = x10;
    let mut x15 = x15;
    let mut x1 = load32_le(k.add(0));
    let mut x2 = load32_le(k.add(4));
    let mut x3 = load32_le(k.add(8));
    let mut x4 = load32_le(k.add(12));
    let mut x11 = load32_le(k.add(16));
    let mut x12 = load32_le(k.add(20));
    let mut x13 = load32_le(k.add(24));
    let mut x14 = load32_le(k.add(28));
    let mut x6 = load32_le(input.add(0));
    let mut x7 = load32_le(input.add(4));
    let mut x8 = load32_le(input.add(8));
    let mut x9 = load32_le(input.add(12));

    let mut i = ROUNDS;
    while i > 0 {
        x4 ^= rotl32(x0.wrapping_add(x12), 7);
        x8 ^= rotl32(x4.wrapping_add(x0), 9);
        x12 ^= rotl32(x8.wrapping_add(x4), 13);
        x0 ^= rotl32(x12.wrapping_add(x8), 18);
        x9 ^= rotl32(x5.wrapping_add(x1), 7);
        x13 ^= rotl32(x9.wrapping_add(x5), 9);
        x1 ^= rotl32(x13.wrapping_add(x9), 13);
        x5 ^= rotl32(x1.wrapping_add(x13), 18);
        x14 ^= rotl32(x10.wrapping_add(x6), 7);
        x2 ^= rotl32(x14.wrapping_add(x10), 9);
        x6 ^= rotl32(x2.wrapping_add(x14), 13);
        x10 ^= rotl32(x6.wrapping_add(x2), 18);
        x3 ^= rotl32(x15.wrapping_add(x11), 7);
        x7 ^= rotl32(x3.wrapping_add(x15), 9);
        x11 ^= rotl32(x7.wrapping_add(x3), 13);
        x15 ^= rotl32(x11.wrapping_add(x7), 18);
        x1 ^= rotl32(x0.wrapping_add(x3), 7);
        x2 ^= rotl32(x1.wrapping_add(x0), 9);
        x3 ^= rotl32(x2.wrapping_add(x1), 13);
        x0 ^= rotl32(x3.wrapping_add(x2), 18);
        x6 ^= rotl32(x5.wrapping_add(x4), 7);
        x7 ^= rotl32(x6.wrapping_add(x5), 9);
        x4 ^= rotl32(x7.wrapping_add(x6), 13);
        x5 ^= rotl32(x4.wrapping_add(x7), 18);
        x11 ^= rotl32(x10.wrapping_add(x9), 7);
        x8 ^= rotl32(x11.wrapping_add(x10), 9);
        x9 ^= rotl32(x8.wrapping_add(x11), 13);
        x10 ^= rotl32(x9.wrapping_add(x8), 18);
        x12 ^= rotl32(x15.wrapping_add(x14), 7);
        x13 ^= rotl32(x12.wrapping_add(x15), 9);
        x14 ^= rotl32(x13.wrapping_add(x12), 13);
        x15 ^= rotl32(x14.wrapping_add(x13), 18);
        i -= 2;
    }

    store32_le(out.add(0), x0);
    store32_le(out.add(4), x5);
    store32_le(out.add(8), x10);
    store32_le(out.add(12), x15);
    store32_le(out.add(16), x6);
    store32_le(out.add(20), x7);
    store32_le(out.add(24), x8);
    store32_le(out.add(28), x9);

    0
}
