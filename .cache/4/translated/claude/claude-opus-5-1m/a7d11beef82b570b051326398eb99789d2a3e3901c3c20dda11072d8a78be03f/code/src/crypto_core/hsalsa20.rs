//! Translation of `crypto_core/hsalsa20/core_hsalsa20.c` and
//! `crypto_core/hsalsa20/ref2/core_hsalsa20_ref2.c`.

use core::ffi::c_int;

use crate::common::{load32_le, rotl32, store32_le};

pub const CRYPTO_CORE_HSALSA20_OUTPUTBYTES: usize = 32;
pub const CRYPTO_CORE_HSALSA20_INPUTBYTES: usize = 16;
pub const CRYPTO_CORE_HSALSA20_KEYBYTES: usize = 32;
pub const CRYPTO_CORE_HSALSA20_CONSTBYTES: usize = 16;

// ---------------------------------------------------------------------------
// core_hsalsa20.c
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_hsalsa20_outputbytes() -> usize {
    CRYPTO_CORE_HSALSA20_OUTPUTBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_hsalsa20_inputbytes() -> usize {
    CRYPTO_CORE_HSALSA20_INPUTBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_hsalsa20_keybytes() -> usize {
    CRYPTO_CORE_HSALSA20_KEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_hsalsa20_constbytes() -> usize {
    CRYPTO_CORE_HSALSA20_CONSTBYTES
}

// ---------------------------------------------------------------------------
// ref2/core_hsalsa20_ref2.c
// ---------------------------------------------------------------------------

const ROUNDS: c_int = 20;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_hsalsa20(
    out: *mut u8,
    in_: *const u8,
    k: *const u8,
    c: *const u8,
) -> c_int {
    let (mut x0, mut x5, mut x10, mut x15);

    if c.is_null() {
        x0 = 0x6170_7865u32;
        x5 = 0x3320_646eu32;
        x10 = 0x7962_2d32u32;
        x15 = 0x6b20_6574u32;
    } else {
        x0 = unsafe { load32_le(c.add(0)) };
        x5 = unsafe { load32_le(c.add(4)) };
        x10 = unsafe { load32_le(c.add(8)) };
        x15 = unsafe { load32_le(c.add(12)) };
    }
    let mut x1 = unsafe { load32_le(k.add(0)) };
    let mut x2 = unsafe { load32_le(k.add(4)) };
    let mut x3 = unsafe { load32_le(k.add(8)) };
    let mut x4 = unsafe { load32_le(k.add(12)) };
    let mut x11 = unsafe { load32_le(k.add(16)) };
    let mut x12 = unsafe { load32_le(k.add(20)) };
    let mut x13 = unsafe { load32_le(k.add(24)) };
    let mut x14 = unsafe { load32_le(k.add(28)) };
    let mut x6 = unsafe { load32_le(in_.add(0)) };
    let mut x7 = unsafe { load32_le(in_.add(4)) };
    let mut x8 = unsafe { load32_le(in_.add(8)) };
    let mut x9 = unsafe { load32_le(in_.add(12)) };

    let mut i: c_int = ROUNDS;
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

    unsafe {
        store32_le(out.add(0), x0);
        store32_le(out.add(4), x5);
        store32_le(out.add(8), x10);
        store32_le(out.add(12), x15);
        store32_le(out.add(16), x6);
        store32_le(out.add(20), x7);
        store32_le(out.add(24), x8);
        store32_le(out.add(28), x9);
    }

    0
}
