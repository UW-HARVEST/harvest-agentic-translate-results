//! Translation of:
//! * `crypto_shorthash/crypto_shorthash.c`
//! * `crypto_shorthash/siphash24/shorthash_siphash24.c`
//! * `crypto_shorthash/siphash24/shorthash_siphashx24.c`
//! * `crypto_shorthash/siphash24/ref/shorthash_siphash24_ref.c`
//! * `crypto_shorthash/siphash24/ref/shorthash_siphashx24_ref.c`
#![allow(dead_code)]

use core::ffi::c_int;

use crate::common::{load64_le, rotl64, store64_le};

extern "C" {
    fn randombytes_buf(buf: *mut u8, size: usize);
}

pub const CRYPTO_SHORTHASH_BYTES: usize = 8;
pub const CRYPTO_SHORTHASH_KEYBYTES: usize = 16;

pub const CRYPTO_SHORTHASH_SIPHASH24_BYTES: usize = 8;
pub const CRYPTO_SHORTHASH_SIPHASH24_KEYBYTES: usize = 16;

pub const CRYPTO_SHORTHASH_SIPHASHX24_BYTES: usize = 16;
pub const CRYPTO_SHORTHASH_SIPHASHX24_KEYBYTES: usize = 16;

#[inline(always)]
unsafe fn siphash_round(v0: &mut u64, v1: &mut u64, v2: &mut u64, v3: &mut u64) {
    *v0 = v0.wrapping_add(*v1);
    *v2 = v2.wrapping_add(*v3);
    *v1 = rotl64(*v1, 13);
    *v3 = rotl64(*v3, 16);
    *v1 ^= *v0;
    *v3 ^= *v2;
    *v0 = rotl64(*v0, 32);
    *v0 = v0.wrapping_add(*v3);
    *v2 = v2.wrapping_add(*v1);
    *v3 = rotl64(*v3, 21);
    *v1 = rotl64(*v1, 17);
    *v3 ^= *v0;
    *v1 ^= *v2;
    *v2 = rotl64(*v2, 32);
}

// ---- crypto_shorthash/crypto_shorthash.c ----

#[no_mangle]
pub unsafe extern "C" fn crypto_shorthash_bytes() -> usize {
    CRYPTO_SHORTHASH_BYTES
}

#[no_mangle]
pub unsafe extern "C" fn crypto_shorthash_keybytes() -> usize {
    CRYPTO_SHORTHASH_KEYBYTES
}

#[no_mangle]
pub unsafe extern "C" fn crypto_shorthash_primitive() -> *const core::ffi::c_char {
    static PRIMITIVE: &[u8] = b"siphash24\0";
    PRIMITIVE.as_ptr() as *const core::ffi::c_char
}

#[no_mangle]
pub unsafe extern "C" fn crypto_shorthash(
    out: *mut u8,
    inp: *const u8,
    inlen: u64,
    k: *const u8,
) -> c_int {
    crypto_shorthash_siphash24(out, inp, inlen, k)
}

#[no_mangle]
pub unsafe extern "C" fn crypto_shorthash_keygen(k: *mut u8) {
    randombytes_buf(k, CRYPTO_SHORTHASH_KEYBYTES);
}

// ---- crypto_shorthash/siphash24/shorthash_siphash24.c ----

#[no_mangle]
pub unsafe extern "C" fn crypto_shorthash_siphash24_bytes() -> usize {
    CRYPTO_SHORTHASH_SIPHASH24_BYTES
}

#[no_mangle]
pub unsafe extern "C" fn crypto_shorthash_siphash24_keybytes() -> usize {
    CRYPTO_SHORTHASH_SIPHASH24_KEYBYTES
}

// ---- crypto_shorthash/siphash24/shorthash_siphashx24.c ----

#[no_mangle]
pub unsafe extern "C" fn crypto_shorthash_siphashx24_bytes() -> usize {
    CRYPTO_SHORTHASH_SIPHASHX24_BYTES
}

#[no_mangle]
pub unsafe extern "C" fn crypto_shorthash_siphashx24_keybytes() -> usize {
    CRYPTO_SHORTHASH_SIPHASHX24_KEYBYTES
}

// ---- crypto_shorthash/siphash24/ref/shorthash_siphash24_ref.c ----

#[no_mangle]
pub unsafe extern "C" fn crypto_shorthash_siphash24(
    out: *mut u8,
    inp: *const u8,
    inlen: u64,
    k: *const u8,
) -> c_int {
    /* "somepseudorandomlygeneratedbytes" */
    let mut v0: u64 = 0x736f_6d65_7073_6575;
    let mut v1: u64 = 0x646f_7261_6e64_6f6d;
    let mut v2: u64 = 0x6c79_6765_6e65_7261;
    let mut v3: u64 = 0x7465_6462_7974_6573;
    let mut b: u64;
    let k0 = load64_le(k);
    let k1 = load64_le(k.add(8));
    let mut m: u64;

    let mut in_ptr = inp;
    let end: *const u8 = if inlen != 0 {
        inp.add((inlen - (inlen % 8)) as usize)
    } else {
        inp
    };
    let left: i32 = (inlen & 7) as i32;

    b = inlen << 56;
    v3 ^= k1;
    v2 ^= k0;
    v1 ^= k1;
    v0 ^= k0;

    while in_ptr != end {
        m = load64_le(in_ptr);
        v3 ^= m;
        siphash_round(&mut v0, &mut v1, &mut v2, &mut v3);
        siphash_round(&mut v0, &mut v1, &mut v2, &mut v3);
        v0 ^= m;
        in_ptr = in_ptr.add(8);
    }

    match left {
        7 => {
            b |= (*end.add(6) as u64) << 48;
            b |= (*end.add(5) as u64) << 40;
            b |= (*end.add(4) as u64) << 32;
            b |= (*end.add(3) as u64) << 24;
            b |= (*end.add(2) as u64) << 16;
            b |= (*end.add(1) as u64) << 8;
            b |= *end.add(0) as u64;
        }
        6 => {
            b |= (*end.add(5) as u64) << 40;
            b |= (*end.add(4) as u64) << 32;
            b |= (*end.add(3) as u64) << 24;
            b |= (*end.add(2) as u64) << 16;
            b |= (*end.add(1) as u64) << 8;
            b |= *end.add(0) as u64;
        }
        5 => {
            b |= (*end.add(4) as u64) << 32;
            b |= (*end.add(3) as u64) << 24;
            b |= (*end.add(2) as u64) << 16;
            b |= (*end.add(1) as u64) << 8;
            b |= *end.add(0) as u64;
        }
        4 => {
            b |= (*end.add(3) as u64) << 24;
            b |= (*end.add(2) as u64) << 16;
            b |= (*end.add(1) as u64) << 8;
            b |= *end.add(0) as u64;
        }
        3 => {
            b |= (*end.add(2) as u64) << 16;
            b |= (*end.add(1) as u64) << 8;
            b |= *end.add(0) as u64;
        }
        2 => {
            b |= (*end.add(1) as u64) << 8;
            b |= *end.add(0) as u64;
        }
        1 => {
            b |= *end.add(0) as u64;
        }
        _ => {}
    }

    v3 ^= b;
    siphash_round(&mut v0, &mut v1, &mut v2, &mut v3);
    siphash_round(&mut v0, &mut v1, &mut v2, &mut v3);
    v0 ^= b;
    v2 ^= 0xff;
    siphash_round(&mut v0, &mut v1, &mut v2, &mut v3);
    siphash_round(&mut v0, &mut v1, &mut v2, &mut v3);
    siphash_round(&mut v0, &mut v1, &mut v2, &mut v3);
    siphash_round(&mut v0, &mut v1, &mut v2, &mut v3);
    b = v0 ^ v1 ^ v2 ^ v3;
    store64_le(out, b);

    0
}

// ---- crypto_shorthash/siphash24/ref/shorthash_siphashx24_ref.c ----

#[no_mangle]
pub unsafe extern "C" fn crypto_shorthash_siphashx24(
    out: *mut u8,
    inp: *const u8,
    inlen: u64,
    k: *const u8,
) -> c_int {
    let mut v0: u64 = 0x736f_6d65_7073_6575;
    let mut v1: u64 = 0x646f_7261_6e64_6f83;
    let mut v2: u64 = 0x6c79_6765_6e65_7261;
    let mut v3: u64 = 0x7465_6462_7974_6573;
    let mut b: u64;
    let k0 = load64_le(k);
    let k1 = load64_le(k.add(8));
    let mut m: u64;

    let mut in_ptr = inp;
    let end: *const u8 = if inlen != 0 {
        inp.add((inlen - (inlen % 8)) as usize)
    } else {
        inp
    };
    let left: i32 = (inlen & 7) as i32;

    b = inlen << 56;
    v3 ^= k1;
    v2 ^= k0;
    v1 ^= k1;
    v0 ^= k0;

    while in_ptr != end {
        m = load64_le(in_ptr);
        v3 ^= m;
        siphash_round(&mut v0, &mut v1, &mut v2, &mut v3);
        siphash_round(&mut v0, &mut v1, &mut v2, &mut v3);
        v0 ^= m;
        in_ptr = in_ptr.add(8);
    }

    match left {
        7 => {
            b |= (*end.add(6) as u64) << 48;
            b |= (*end.add(5) as u64) << 40;
            b |= (*end.add(4) as u64) << 32;
            b |= (*end.add(3) as u64) << 24;
            b |= (*end.add(2) as u64) << 16;
            b |= (*end.add(1) as u64) << 8;
            b |= *end.add(0) as u64;
        }
        6 => {
            b |= (*end.add(5) as u64) << 40;
            b |= (*end.add(4) as u64) << 32;
            b |= (*end.add(3) as u64) << 24;
            b |= (*end.add(2) as u64) << 16;
            b |= (*end.add(1) as u64) << 8;
            b |= *end.add(0) as u64;
        }
        5 => {
            b |= (*end.add(4) as u64) << 32;
            b |= (*end.add(3) as u64) << 24;
            b |= (*end.add(2) as u64) << 16;
            b |= (*end.add(1) as u64) << 8;
            b |= *end.add(0) as u64;
        }
        4 => {
            b |= (*end.add(3) as u64) << 24;
            b |= (*end.add(2) as u64) << 16;
            b |= (*end.add(1) as u64) << 8;
            b |= *end.add(0) as u64;
        }
        3 => {
            b |= (*end.add(2) as u64) << 16;
            b |= (*end.add(1) as u64) << 8;
            b |= *end.add(0) as u64;
        }
        2 => {
            b |= (*end.add(1) as u64) << 8;
            b |= *end.add(0) as u64;
        }
        1 => {
            b |= *end.add(0) as u64;
        }
        _ => {}
    }

    v3 ^= b;
    siphash_round(&mut v0, &mut v1, &mut v2, &mut v3);
    siphash_round(&mut v0, &mut v1, &mut v2, &mut v3);
    v0 ^= b;
    v2 ^= 0xee;
    siphash_round(&mut v0, &mut v1, &mut v2, &mut v3);
    siphash_round(&mut v0, &mut v1, &mut v2, &mut v3);
    siphash_round(&mut v0, &mut v1, &mut v2, &mut v3);
    siphash_round(&mut v0, &mut v1, &mut v2, &mut v3);
    b = v0 ^ v1 ^ v2 ^ v3;
    store64_le(out, b);
    v1 ^= 0xdd;
    siphash_round(&mut v0, &mut v1, &mut v2, &mut v3);
    siphash_round(&mut v0, &mut v1, &mut v2, &mut v3);
    siphash_round(&mut v0, &mut v1, &mut v2, &mut v3);
    siphash_round(&mut v0, &mut v1, &mut v2, &mut v3);
    b = v0 ^ v1 ^ v2 ^ v3;
    store64_le(out.add(8), b);

    0
}
