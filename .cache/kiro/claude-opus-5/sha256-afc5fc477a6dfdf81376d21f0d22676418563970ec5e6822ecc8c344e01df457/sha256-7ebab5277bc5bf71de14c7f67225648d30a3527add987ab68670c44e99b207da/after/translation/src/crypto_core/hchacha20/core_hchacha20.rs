//! Translation of c_src/libsodium/crypto_core/hchacha20/core_hchacha20.c

use core::ffi::c_int;

use crate::common::{load32_le, rotl32, store32_le};

// crypto_core_hchacha20_* constants from crypto_core_hchacha20.h
const CRYPTO_CORE_HCHACHA20_OUTPUTBYTES: usize = 32;
const CRYPTO_CORE_HCHACHA20_INPUTBYTES: usize = 16;
const CRYPTO_CORE_HCHACHA20_KEYBYTES: usize = 32;
const CRYPTO_CORE_HCHACHA20_CONSTBYTES: usize = 16;

// QUARTERROUND(A, B, C, D):
//   A += B; D = ROTL32(D ^ A, 16);
//   C += D; B = ROTL32(B ^ C, 12);
//   A += B; D = ROTL32(D ^ A,  8);
//   C += D; B = ROTL32(B ^ C,  7);
macro_rules! quarterround {
    ($a:ident, $b:ident, $c:ident, $d:ident) => {{
        $a = $a.wrapping_add($b);
        $d = rotl32($d ^ $a, 16);
        $c = $c.wrapping_add($d);
        $b = rotl32($b ^ $c, 12);
        $a = $a.wrapping_add($b);
        $d = rotl32($d ^ $a, 8);
        $c = $c.wrapping_add($d);
        $b = rotl32($b ^ $c, 7);
    }};
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_hchacha20(
    out: *mut u8,
    in_: *const u8,
    k: *const u8,
    c: *const u8,
) -> c_int {
    let mut i: c_int;
    let (mut x0, mut x1, mut x2, mut x3, mut x4, mut x5, mut x6, mut x7);
    let (mut x8, mut x9, mut x10, mut x11, mut x12, mut x13, mut x14, mut x15);

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
    x4 = load32_le(k.add(0));
    x5 = load32_le(k.add(4));
    x6 = load32_le(k.add(8));
    x7 = load32_le(k.add(12));
    x8 = load32_le(k.add(16));
    x9 = load32_le(k.add(20));
    x10 = load32_le(k.add(24));
    x11 = load32_le(k.add(28));
    x12 = load32_le(in_.add(0));
    x13 = load32_le(in_.add(4));
    x14 = load32_le(in_.add(8));
    x15 = load32_le(in_.add(12));

    i = 0;
    while i < 10 {
        quarterround!(x0, x4, x8, x12);
        quarterround!(x1, x5, x9, x13);
        quarterround!(x2, x6, x10, x14);
        quarterround!(x3, x7, x11, x15);
        quarterround!(x0, x5, x10, x15);
        quarterround!(x1, x6, x11, x12);
        quarterround!(x2, x7, x8, x13);
        quarterround!(x3, x4, x9, x14);
        i += 1;
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
pub unsafe extern "C" fn crypto_core_hchacha20_outputbytes() -> usize {
    CRYPTO_CORE_HCHACHA20_OUTPUTBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_hchacha20_inputbytes() -> usize {
    CRYPTO_CORE_HCHACHA20_INPUTBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_hchacha20_keybytes() -> usize {
    CRYPTO_CORE_HCHACHA20_KEYBYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_core_hchacha20_constbytes() -> usize {
    CRYPTO_CORE_HCHACHA20_CONSTBYTES
}
