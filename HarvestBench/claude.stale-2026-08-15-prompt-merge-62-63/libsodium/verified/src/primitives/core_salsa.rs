//! Translated from crypto_core/salsa/ref/core_salsa_ref.c, hsalsa20, hchacha20.
use crate::primitives::cutil::*;
use core::ffi::c_int;

#[inline(always)]
fn r(x: u32, b: u32) -> u32 {
    rotl32(x, b)
}

unsafe fn crypto_core_salsa_impl(
    out: *mut u8,
    inp: *const u8,
    k: *const u8,
    c: *const u8,
    rounds: c_int,
) {
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

    let j0;
    let j5;
    let j10;
    let j15;
    j0 = 0x61707865u32;
    x0 = j0;
    j5 = 0x3320646eu32;
    x5 = j5;
    j10 = 0x79622d32u32;
    x10 = j10;
    j15 = 0x6b206574u32;
    x15 = j15;
    let (j0, j5, j10, j15) = if !c.is_null() {
        x0 = load32_le(c.add(0));
        x5 = load32_le(c.add(4));
        x10 = load32_le(c.add(8));
        x15 = load32_le(c.add(12));
        (x0, x5, x10, x15)
    } else {
        (j0, j5, j10, j15)
    };
    let j1 = load32_le(k.add(0));
    x1 = j1;
    let j2 = load32_le(k.add(4));
    x2 = j2;
    let j3 = load32_le(k.add(8));
    x3 = j3;
    let j4 = load32_le(k.add(12));
    x4 = j4;
    let j11 = load32_le(k.add(16));
    x11 = j11;
    let j12 = load32_le(k.add(20));
    x12 = j12;
    let j13 = load32_le(k.add(24));
    x13 = j13;
    let j14 = load32_le(k.add(28));
    x14 = j14;

    let j6 = load32_le(inp.add(0));
    x6 = j6;
    let j7 = load32_le(inp.add(4));
    x7 = j7;
    let j8 = load32_le(inp.add(8));
    x8 = j8;
    let j9 = load32_le(inp.add(12));
    x9 = j9;

    let mut i = 0;
    while i < rounds {
        x4 ^= r(x0.wrapping_add(x12), 7);
        x8 ^= r(x4.wrapping_add(x0), 9);
        x12 ^= r(x8.wrapping_add(x4), 13);
        x0 ^= r(x12.wrapping_add(x8), 18);
        x9 ^= r(x5.wrapping_add(x1), 7);
        x13 ^= r(x9.wrapping_add(x5), 9);
        x1 ^= r(x13.wrapping_add(x9), 13);
        x5 ^= r(x1.wrapping_add(x13), 18);
        x14 ^= r(x10.wrapping_add(x6), 7);
        x2 ^= r(x14.wrapping_add(x10), 9);
        x6 ^= r(x2.wrapping_add(x14), 13);
        x10 ^= r(x6.wrapping_add(x2), 18);
        x3 ^= r(x15.wrapping_add(x11), 7);
        x7 ^= r(x3.wrapping_add(x15), 9);
        x11 ^= r(x7.wrapping_add(x3), 13);
        x15 ^= r(x11.wrapping_add(x7), 18);
        x1 ^= r(x0.wrapping_add(x3), 7);
        x2 ^= r(x1.wrapping_add(x0), 9);
        x3 ^= r(x2.wrapping_add(x1), 13);
        x0 ^= r(x3.wrapping_add(x2), 18);
        x6 ^= r(x5.wrapping_add(x4), 7);
        x7 ^= r(x6.wrapping_add(x5), 9);
        x4 ^= r(x7.wrapping_add(x6), 13);
        x5 ^= r(x4.wrapping_add(x7), 18);
        x11 ^= r(x10.wrapping_add(x9), 7);
        x8 ^= r(x11.wrapping_add(x10), 9);
        x9 ^= r(x8.wrapping_add(x11), 13);
        x10 ^= r(x9.wrapping_add(x8), 18);
        x12 ^= r(x15.wrapping_add(x14), 7);
        x13 ^= r(x12.wrapping_add(x15), 9);
        x14 ^= r(x13.wrapping_add(x12), 13);
        x15 ^= r(x14.wrapping_add(x13), 18);
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
    inp: *const u8,
    k: *const u8,
    c: *const u8,
) -> c_int {
    crypto_core_salsa_impl(out, inp, k, c, 20);
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_core_salsa20_outputbytes() -> usize {
    64
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_core_salsa20_inputbytes() -> usize {
    16
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_core_salsa20_keybytes() -> usize {
    32
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_core_salsa20_constbytes() -> usize {
    16
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_salsa2012(
    out: *mut u8,
    inp: *const u8,
    k: *const u8,
    c: *const u8,
) -> c_int {
    crypto_core_salsa_impl(out, inp, k, c, 12);
    0
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_core_salsa2012_outputbytes() -> usize {
    64
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_core_salsa2012_inputbytes() -> usize {
    16
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_core_salsa2012_keybytes() -> usize {
    32
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_core_salsa2012_constbytes() -> usize {
    16
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_salsa208(
    out: *mut u8,
    inp: *const u8,
    k: *const u8,
    c: *const u8,
) -> c_int {
    crypto_core_salsa_impl(out, inp, k, c, 8);
    0
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_core_salsa208_outputbytes() -> usize {
    64
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_core_salsa208_inputbytes() -> usize {
    16
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_core_salsa208_keybytes() -> usize {
    32
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_core_salsa208_constbytes() -> usize {
    16
}

// ---- hsalsa20 ----

#[unsafe(no_mangle)]
pub extern "C" fn crypto_core_hsalsa20_outputbytes() -> usize {
    32
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_core_hsalsa20_inputbytes() -> usize {
    16
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_core_hsalsa20_keybytes() -> usize {
    32
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_core_hsalsa20_constbytes() -> usize {
    16
}

// ---- hchacha20 ----

#[inline(always)]
fn quarterround(a: &mut u32, b: &mut u32, c: &mut u32, d: &mut u32) {
    *a = a.wrapping_add(*b);
    *d = rotl32(*d ^ *a, 16);
    *c = c.wrapping_add(*d);
    *b = rotl32(*b ^ *c, 12);
    *a = a.wrapping_add(*b);
    *d = rotl32(*d ^ *a, 8);
    *c = c.wrapping_add(*d);
    *b = rotl32(*b ^ *c, 7);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_hchacha20(
    out: *mut u8,
    inp: *const u8,
    k: *const u8,
    c: *const u8,
) -> c_int {
    let mut x0;
    let mut x1;
    let mut x2;
    let mut x3;
    if c.is_null() {
        x0 = 0x61707865u32;
        x1 = 0x3320646eu32;
        x2 = 0x79622d32u32;
        x3 = 0x6b206574u32;
    } else {
        x0 = load32_le(c.add(0));
        x1 = load32_le(c.add(4));
        x2 = load32_le(c.add(8));
        x3 = load32_le(c.add(12));
    }
    let mut x4 = load32_le(k.add(0));
    let mut x5 = load32_le(k.add(4));
    let mut x6 = load32_le(k.add(8));
    let mut x7 = load32_le(k.add(12));
    let mut x8 = load32_le(k.add(16));
    let mut x9 = load32_le(k.add(20));
    let mut x10 = load32_le(k.add(24));
    let mut x11 = load32_le(k.add(28));
    let mut x12 = load32_le(inp.add(0));
    let mut x13 = load32_le(inp.add(4));
    let mut x14 = load32_le(inp.add(8));
    let mut x15 = load32_le(inp.add(12));

    for _ in 0..10 {
        quarterround(&mut x0, &mut x4, &mut x8, &mut x12);
        quarterround(&mut x1, &mut x5, &mut x9, &mut x13);
        quarterround(&mut x2, &mut x6, &mut x10, &mut x14);
        quarterround(&mut x3, &mut x7, &mut x11, &mut x15);
        quarterround(&mut x0, &mut x5, &mut x10, &mut x15);
        quarterround(&mut x1, &mut x6, &mut x11, &mut x12);
        quarterround(&mut x2, &mut x7, &mut x8, &mut x13);
        quarterround(&mut x3, &mut x4, &mut x9, &mut x14);
    }

    store32_le(out.add(0), x0);
    store32_le(out.add(4), x1);
    store32_le(out.add(8), x2);
    store32_le(out.add(12), x3);
    store32_le(out.add(16), x12);
    store32_le(out.add(20), x13);
    store32_le(out.add(24), x14);
    store32_le(out.add(28), x15);

    0
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_core_hchacha20_outputbytes() -> usize {
    32
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_core_hchacha20_inputbytes() -> usize {
    16
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_core_hchacha20_keybytes() -> usize {
    32
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_core_hchacha20_constbytes() -> usize {
    16
}
