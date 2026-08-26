//! Translation of `crypto_shorthash/siphash24/ref/shorthash_siphashx24_ref.c`
//! and `crypto_shorthash/siphash24/shorthash_siphashx24.c`.

use core::ffi::c_int;

use crate::common::{load64_le, rotl64, store64_le};

pub const crypto_shorthash_siphashx24_BYTES: usize = 16;
pub const crypto_shorthash_siphashx24_KEYBYTES: usize = 16;

/// `SIPROUND` from `crypto_shorthash/siphash24/ref/shorthash_siphash_ref.h`.
macro_rules! sipround {
    ($v0:ident, $v1:ident, $v2:ident, $v3:ident) => {{
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
pub unsafe extern "C" fn crypto_shorthash_siphashx24(
    out: *mut u8,
    in_: *const u8,
    inlen: u64,
    k: *const u8,
) -> c_int {
    let mut v0: u64 = 0x736f6d6570736575;
    let mut v1: u64 = 0x646f72616e646f83;
    let mut v2: u64 = 0x6c7967656e657261;
    let mut v3: u64 = 0x7465646279746573;
    let mut b: u64;
    let k0: u64 = unsafe { load64_le(k) };
    let k1: u64 = unsafe { load64_le(k.add(8)) };
    let mut m: u64;
    let mut in_ = in_;
    let end: *const u8 = if inlen != 0 {
        unsafe { in_.add((inlen - (inlen % 8)) as usize) }
    } else {
        in_
    };
    let left: c_int = (inlen & 7) as c_int;

    b = inlen << 56;
    v3 ^= k1;
    v2 ^= k0;
    v1 ^= k1;
    v0 ^= k0;
    while in_ != end {
        m = unsafe { load64_le(in_) };
        v3 ^= m;
        sipround!(v0, v1, v2, v3);
        sipround!(v0, v1, v2, v3);
        v0 ^= m;
        in_ = unsafe { in_.add(8) };
    }
    match left {
        7 => {
            b |= (unsafe { *in_.add(6) } as u64) << 48;
            b |= (unsafe { *in_.add(5) } as u64) << 40;
            b |= (unsafe { *in_.add(4) } as u64) << 32;
            b |= (unsafe { *in_.add(3) } as u64) << 24;
            b |= (unsafe { *in_.add(2) } as u64) << 16;
            b |= (unsafe { *in_.add(1) } as u64) << 8;
            b |= unsafe { *in_ } as u64;
        }
        6 => {
            b |= (unsafe { *in_.add(5) } as u64) << 40;
            b |= (unsafe { *in_.add(4) } as u64) << 32;
            b |= (unsafe { *in_.add(3) } as u64) << 24;
            b |= (unsafe { *in_.add(2) } as u64) << 16;
            b |= (unsafe { *in_.add(1) } as u64) << 8;
            b |= unsafe { *in_ } as u64;
        }
        5 => {
            b |= (unsafe { *in_.add(4) } as u64) << 32;
            b |= (unsafe { *in_.add(3) } as u64) << 24;
            b |= (unsafe { *in_.add(2) } as u64) << 16;
            b |= (unsafe { *in_.add(1) } as u64) << 8;
            b |= unsafe { *in_ } as u64;
        }
        4 => {
            b |= (unsafe { *in_.add(3) } as u64) << 24;
            b |= (unsafe { *in_.add(2) } as u64) << 16;
            b |= (unsafe { *in_.add(1) } as u64) << 8;
            b |= unsafe { *in_ } as u64;
        }
        3 => {
            b |= (unsafe { *in_.add(2) } as u64) << 16;
            b |= (unsafe { *in_.add(1) } as u64) << 8;
            b |= unsafe { *in_ } as u64;
        }
        2 => {
            b |= (unsafe { *in_.add(1) } as u64) << 8;
            b |= unsafe { *in_ } as u64;
        }
        1 => {
            b |= unsafe { *in_ } as u64;
        }
        0 => {}
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
    unsafe { store64_le(out, b) };
    v1 ^= 0xdd;
    sipround!(v0, v1, v2, v3);
    sipround!(v0, v1, v2, v3);
    sipround!(v0, v1, v2, v3);
    sipround!(v0, v1, v2, v3);
    b = v0 ^ v1 ^ v2 ^ v3;
    unsafe { store64_le(out.add(8), b) };

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_shorthash_siphashx24_bytes() -> usize {
    crypto_shorthash_siphashx24_BYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_shorthash_siphashx24_keybytes() -> usize {
    crypto_shorthash_siphashx24_KEYBYTES
}
