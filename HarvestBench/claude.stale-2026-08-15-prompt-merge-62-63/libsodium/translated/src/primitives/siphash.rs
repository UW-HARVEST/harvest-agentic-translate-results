//! Translated from crypto_shorthash/siphash24/ref/{shorthash_siphash24_ref.c, shorthash_siphashx24_ref.c}
//! and shorthash_siphash24.c / shorthash_siphashx24.c
use crate::primitives::cutil::*;

macro_rules! sipround {
    ($v0:expr,$v1:expr,$v2:expr,$v3:expr) => {{
        $v0 = $v0.wrapping_add($v1);
        $v2 = $v2.wrapping_add($v3);
        $v1 = rotl64($v1, 13);
        $v3 = rotl64($v3, 16);
        $v1 ^= $v0;
        $v3 ^= $v2;
        $v0 = rotl64($v0, 32);
        $v0 = $v0.wrapping_add($v3);
        $v2 = $v2.wrapping_add($v1);
        $v3 = rotl64($v3, 21);
        $v1 = rotl64($v1, 17);
        $v3 ^= $v0;
        $v1 ^= $v2;
        $v2 = rotl64($v2, 32);
    }};
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_shorthash_siphash24(
    out: *mut u8,
    mut input: *const u8,
    inlen: u64,
    k: *const u8,
) -> i32 {
    let mut v0: u64 = 0x736f6d6570736575;
    let mut v1: u64 = 0x646f72616e646f6d;
    let mut v2: u64 = 0x6c7967656e657261;
    let mut v3: u64 = 0x7465646279746573;
    let mut b: u64;
    let k0 = load64_le(k);
    let k1 = load64_le(k.add(8));
    let mut m: u64;
    let end = if inlen != 0 {
        input.add((inlen - (inlen % 8)) as usize)
    } else {
        input
    };
    let left = (inlen & 7) as i32;

    b = (inlen as u64) << 56;
    v3 ^= k1;
    v2 ^= k0;
    v1 ^= k1;
    v0 ^= k0;
    while input != end {
        m = load64_le(input);
        v3 ^= m;
        sipround!(v0, v1, v2, v3);
        sipround!(v0, v1, v2, v3);
        v0 ^= m;
        input = input.add(8);
    }
    match left {
        7 => {
            b |= (*input.add(6) as u64) << 48;
            b |= (*input.add(5) as u64) << 40;
            b |= (*input.add(4) as u64) << 32;
            b |= (*input.add(3) as u64) << 24;
            b |= (*input.add(2) as u64) << 16;
            b |= (*input.add(1) as u64) << 8;
            b |= *input.add(0) as u64;
        }
        6 => {
            b |= (*input.add(5) as u64) << 40;
            b |= (*input.add(4) as u64) << 32;
            b |= (*input.add(3) as u64) << 24;
            b |= (*input.add(2) as u64) << 16;
            b |= (*input.add(1) as u64) << 8;
            b |= *input.add(0) as u64;
        }
        5 => {
            b |= (*input.add(4) as u64) << 32;
            b |= (*input.add(3) as u64) << 24;
            b |= (*input.add(2) as u64) << 16;
            b |= (*input.add(1) as u64) << 8;
            b |= *input.add(0) as u64;
        }
        4 => {
            b |= (*input.add(3) as u64) << 24;
            b |= (*input.add(2) as u64) << 16;
            b |= (*input.add(1) as u64) << 8;
            b |= *input.add(0) as u64;
        }
        3 => {
            b |= (*input.add(2) as u64) << 16;
            b |= (*input.add(1) as u64) << 8;
            b |= *input.add(0) as u64;
        }
        2 => {
            b |= (*input.add(1) as u64) << 8;
            b |= *input.add(0) as u64;
        }
        1 => {
            b |= *input.add(0) as u64;
        }
        _ => {}
    }
    v3 ^= b;
    sipround!(v0, v1, v2, v3);
    sipround!(v0, v1, v2, v3);
    v0 ^= b;
    v2 ^= 0xff;
    sipround!(v0, v1, v2, v3);
    sipround!(v0, v1, v2, v3);
    sipround!(v0, v1, v2, v3);
    sipround!(v0, v1, v2, v3);
    b = v0 ^ v1 ^ v2 ^ v3;
    store64_le(out, b);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_shorthash_siphashx24(
    out: *mut u8,
    mut input: *const u8,
    inlen: u64,
    k: *const u8,
) -> i32 {
    let mut v0: u64 = 0x736f6d6570736575;
    let mut v1: u64 = 0x646f72616e646f83;
    let mut v2: u64 = 0x6c7967656e657261;
    let mut v3: u64 = 0x7465646279746573;
    let mut b: u64;
    let k0 = load64_le(k);
    let k1 = load64_le(k.add(8));
    let mut m: u64;
    let end = if inlen != 0 {
        input.add((inlen - (inlen % 8)) as usize)
    } else {
        input
    };
    let left = (inlen & 7) as i32;

    b = (inlen as u64) << 56;
    v3 ^= k1;
    v2 ^= k0;
    v1 ^= k1;
    v0 ^= k0;
    while input != end {
        m = load64_le(input);
        v3 ^= m;
        sipround!(v0, v1, v2, v3);
        sipround!(v0, v1, v2, v3);
        v0 ^= m;
        input = input.add(8);
    }
    match left {
        7 => {
            b |= (*input.add(6) as u64) << 48;
            b |= (*input.add(5) as u64) << 40;
            b |= (*input.add(4) as u64) << 32;
            b |= (*input.add(3) as u64) << 24;
            b |= (*input.add(2) as u64) << 16;
            b |= (*input.add(1) as u64) << 8;
            b |= *input.add(0) as u64;
        }
        6 => {
            b |= (*input.add(5) as u64) << 40;
            b |= (*input.add(4) as u64) << 32;
            b |= (*input.add(3) as u64) << 24;
            b |= (*input.add(2) as u64) << 16;
            b |= (*input.add(1) as u64) << 8;
            b |= *input.add(0) as u64;
        }
        5 => {
            b |= (*input.add(4) as u64) << 32;
            b |= (*input.add(3) as u64) << 24;
            b |= (*input.add(2) as u64) << 16;
            b |= (*input.add(1) as u64) << 8;
            b |= *input.add(0) as u64;
        }
        4 => {
            b |= (*input.add(3) as u64) << 24;
            b |= (*input.add(2) as u64) << 16;
            b |= (*input.add(1) as u64) << 8;
            b |= *input.add(0) as u64;
        }
        3 => {
            b |= (*input.add(2) as u64) << 16;
            b |= (*input.add(1) as u64) << 8;
            b |= *input.add(0) as u64;
        }
        2 => {
            b |= (*input.add(1) as u64) << 8;
            b |= *input.add(0) as u64;
        }
        1 => {
            b |= *input.add(0) as u64;
        }
        _ => {}
    }
    v3 ^= b;
    sipround!(v0, v1, v2, v3);
    sipround!(v0, v1, v2, v3);
    v0 ^= b;
    v2 ^= 0xee;
    sipround!(v0, v1, v2, v3);
    sipround!(v0, v1, v2, v3);
    sipround!(v0, v1, v2, v3);
    sipround!(v0, v1, v2, v3);
    b = v0 ^ v1 ^ v2 ^ v3;
    store64_le(out, b);
    v1 ^= 0xdd;
    sipround!(v0, v1, v2, v3);
    sipround!(v0, v1, v2, v3);
    sipround!(v0, v1, v2, v3);
    sipround!(v0, v1, v2, v3);
    b = v0 ^ v1 ^ v2 ^ v3;
    store64_le(out.add(8), b);
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_shorthash_siphash24_bytes() -> usize {
    8
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_shorthash_siphash24_keybytes() -> usize {
    16
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_shorthash_siphashx24_bytes() -> usize {
    16
}
#[unsafe(no_mangle)]
pub extern "C" fn crypto_shorthash_siphashx24_keybytes() -> usize {
    16
}
