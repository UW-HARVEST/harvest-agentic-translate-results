use crate::common::{load32_le, rotl32, store32_le};

const crypto_core_salsa20_OUTPUTBYTES: usize = 64;
const crypto_core_salsa20_INPUTBYTES: usize = 16;
const crypto_core_salsa20_KEYBYTES: usize = 32;
const crypto_core_salsa20_CONSTBYTES: usize = 16;

const crypto_core_salsa2012_OUTPUTBYTES: usize = 64;
const crypto_core_salsa2012_INPUTBYTES: usize = 16;
const crypto_core_salsa2012_KEYBYTES: usize = 32;
const crypto_core_salsa2012_CONSTBYTES: usize = 16;

const crypto_core_salsa208_OUTPUTBYTES: usize = 64;
const crypto_core_salsa208_INPUTBYTES: usize = 16;
const crypto_core_salsa208_KEYBYTES: usize = 32;
const crypto_core_salsa208_CONSTBYTES: usize = 16;

unsafe fn crypto_core_salsa(out: *mut u8, in_: *const u8, k: *const u8, c: *const u8, rounds: i32) {
    let mut x0: u32;
    let mut x1: u32;
    let mut x2: u32;
    let mut x3: u32;
    let mut x4: u32;
    let mut x5: u32;
    let mut x6: u32;
    let mut x7: u32;
    let mut x8: u32;
    let mut x9: u32;
    let mut x10: u32;
    let mut x11: u32;
    let mut x12: u32;
    let mut x13: u32;
    let mut x14: u32;
    let mut x15: u32;
    let mut j0: u32;
    let j1: u32;
    let j2: u32;
    let j3: u32;
    let j4: u32;
    let mut j5: u32;
    let j6: u32;
    let j7: u32;
    let j8: u32;
    let j9: u32;
    let mut j10: u32;
    let j11: u32;
    let j12: u32;
    let j13: u32;
    let j14: u32;
    let mut j15: u32;
    let mut i: i32;

    j0 = 0x61707865;
    x0 = j0;
    j5 = 0x3320646e;
    x5 = j5;
    j10 = 0x79622d32;
    x10 = j10;
    j15 = 0x6b206574;
    x15 = j15;
    if !c.is_null() {
        x0 = load32_le(c.add(0));
        j0 = x0;
        x5 = load32_le(c.add(4));
        j5 = x5;
        x10 = load32_le(c.add(8));
        j10 = x10;
        x15 = load32_le(c.add(12));
        j15 = x15;
    }
    x1 = load32_le(k.add(0));
    j1 = x1;
    x2 = load32_le(k.add(4));
    j2 = x2;
    x3 = load32_le(k.add(8));
    j3 = x3;
    x4 = load32_le(k.add(12));
    j4 = x4;
    x11 = load32_le(k.add(16));
    j11 = x11;
    x12 = load32_le(k.add(20));
    j12 = x12;
    x13 = load32_le(k.add(24));
    j13 = x13;
    x14 = load32_le(k.add(28));
    j14 = x14;

    x6 = load32_le(in_.add(0));
    j6 = x6;
    x7 = load32_le(in_.add(4));
    j7 = x7;
    x8 = load32_le(in_.add(8));
    j8 = x8;
    x9 = load32_le(in_.add(12));
    j9 = x9;

    i = 0;
    while i < rounds {
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
        i += 2;
    }
    store32_le(out.add(0), x0.wrapping_add(j0));
    store32_le(out.add(4), x1.wrapping_add(j1));
    store32_le(out.add(8), x2.wrapping_add(j2));
    store32_le(out.add(12), x3.wrapping_add(j3));
    store32_le(out.add(16), x4.wrapping_add(j4));
    store32_le(out.add(20), x5.wrapping_add(j5));
    store32_le(out.add(24), x6.wrapping_add(j6));
    store32_le(out.add(28), x7.wrapping_add(j7));
    store32_le(out.add(32), x8.wrapping_add(j8));
    store32_le(out.add(36), x9.wrapping_add(j9));
    store32_le(out.add(40), x10.wrapping_add(j10));
    store32_le(out.add(44), x11.wrapping_add(j11));
    store32_le(out.add(48), x12.wrapping_add(j12));
    store32_le(out.add(52), x13.wrapping_add(j13));
    store32_le(out.add(56), x14.wrapping_add(j14));
    store32_le(out.add(60), x15.wrapping_add(j15));
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_salsa20(
    out: *mut u8,
    in_: *const u8,
    k: *const u8,
    c: *const u8,
) -> i32 {
    crypto_core_salsa(out, in_, k, c, 20);
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_core_salsa20_outputbytes() -> usize {
    crypto_core_salsa20_OUTPUTBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_core_salsa20_inputbytes() -> usize {
    crypto_core_salsa20_INPUTBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_core_salsa20_keybytes() -> usize {
    crypto_core_salsa20_KEYBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_core_salsa20_constbytes() -> usize {
    crypto_core_salsa20_CONSTBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_salsa2012(
    out: *mut u8,
    in_: *const u8,
    k: *const u8,
    c: *const u8,
) -> i32 {
    crypto_core_salsa(out, in_, k, c, 12);
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_core_salsa2012_outputbytes() -> usize {
    crypto_core_salsa2012_OUTPUTBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_core_salsa2012_inputbytes() -> usize {
    crypto_core_salsa2012_INPUTBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_core_salsa2012_keybytes() -> usize {
    crypto_core_salsa2012_KEYBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_core_salsa2012_constbytes() -> usize {
    crypto_core_salsa2012_CONSTBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_salsa208(
    out: *mut u8,
    in_: *const u8,
    k: *const u8,
    c: *const u8,
) -> i32 {
    crypto_core_salsa(out, in_, k, c, 8);
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_core_salsa208_outputbytes() -> usize {
    crypto_core_salsa208_OUTPUTBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_core_salsa208_inputbytes() -> usize {
    crypto_core_salsa208_INPUTBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_core_salsa208_keybytes() -> usize {
    crypto_core_salsa208_KEYBYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_core_salsa208_constbytes() -> usize {
    crypto_core_salsa208_CONSTBYTES
}
