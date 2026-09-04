//! Translation of
//! `crypto_scalarmult/ristretto255/ref10/scalarmult_ristretto255_ref10.c`
//! and `include/sodium/crypto_scalarmult_ristretto255.h`.

use core::ffi::c_int;

use crate::crypto_core::ed25519_ref10::ge::{
    _sodium_ge25519_scalarmult, _sodium_ge25519_scalarmult_base, _sodium_ristretto255_frombytes,
    _sodium_ristretto255_p3_tobytes,
};
use crate::crypto_core::ed25519_ref10::ge25519_p3;
use crate::sodium_utils::sodium_is_zero;

/* ---- constants from crypto_scalarmult_ristretto255.h ---- */

pub const crypto_scalarmult_ristretto255_BYTES: usize = 32;
pub const crypto_scalarmult_ristretto255_SCALARBYTES: usize = 32;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_scalarmult_ristretto255(
    q: *mut u8,
    n: *const u8,
    p: *const u8,
) -> c_int {
    let t: *mut u8 = q;
    let mut Q: ge25519_p3 = core::mem::zeroed();
    let mut P: ge25519_p3 = core::mem::zeroed();
    let mut i: u32;

    if _sodium_ristretto255_frombytes(&mut P, p) != 0 {
        return -1;
    }
    i = 0;
    while i < 32 {
        *t.add(i as usize) = *n.add(i as usize);
        i += 1;
    }
    *t.add(31) &= 127;
    _sodium_ge25519_scalarmult(&mut Q, t, &P);
    _sodium_ristretto255_p3_tobytes(q, &Q);
    if sodium_is_zero(q, 32) != 0 {
        return -1;
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_scalarmult_ristretto255_base(q: *mut u8, n: *const u8) -> c_int {
    let t: *mut u8 = q;
    let mut Q: ge25519_p3 = core::mem::zeroed();
    let mut i: u32;

    i = 0;
    while i < 32 {
        *t.add(i as usize) = *n.add(i as usize);
        i += 1;
    }
    *t.add(31) &= 127;
    _sodium_ge25519_scalarmult_base(&mut Q, t);
    _sodium_ristretto255_p3_tobytes(q, &Q);
    if sodium_is_zero(q, 32) != 0 {
        return -1;
    }
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_scalarmult_ristretto255_bytes() -> usize {
    crypto_scalarmult_ristretto255_BYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_scalarmult_ristretto255_scalarbytes() -> usize {
    crypto_scalarmult_ristretto255_SCALARBYTES
}
