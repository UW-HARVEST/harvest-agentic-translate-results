//! Translation of
//! `crypto_scalarmult/ristretto255/ref10/scalarmult_ristretto255_ref10.c`.

use core::ffi::c_int;

use crate::crypto_core::ed25519::types::Ge25519P3;
use crate::sodium::utils::sodium_is_zero;

pub const crypto_scalarmult_ristretto255_BYTES: usize = 32;
pub const crypto_scalarmult_ristretto255_SCALARBYTES: usize = 32;

unsafe extern "C" {
    fn _sodium_ristretto255_frombytes(h: *mut Ge25519P3, s: *const u8) -> c_int;
    fn _sodium_ristretto255_p3_tobytes(s: *mut u8, h: *const Ge25519P3);
    fn _sodium_ge25519_scalarmult(h: *mut Ge25519P3, a: *const u8, p: *const Ge25519P3);
    fn _sodium_ge25519_scalarmult_base(h: *mut Ge25519P3, a: *const u8);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_scalarmult_ristretto255(
    q: *mut u8,
    n: *const u8,
    p: *const u8,
) -> c_int {
    /* `unsigned char *t = q;` - `t` aliases the output buffer. */
    let t: *mut u8 = q;
    let mut Q = Ge25519P3::default();
    let mut P = Ge25519P3::default();
    let mut i: u32;

    if unsafe { _sodium_ristretto255_frombytes(&raw mut P, p) } != 0 {
        return -1;
    }
    i = 0;
    while i < 32 {
        unsafe { *t.add(i as usize) = *n.add(i as usize) };
        i += 1;
    }
    unsafe { *t.add(31) &= 127 };
    unsafe {
        _sodium_ge25519_scalarmult(&raw mut Q, t, &raw const P);
        _sodium_ristretto255_p3_tobytes(q, &raw const Q);
    }
    if unsafe { sodium_is_zero(q, 32) } != 0 {
        return -1;
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_scalarmult_ristretto255_base(q: *mut u8, n: *const u8) -> c_int {
    /* `unsigned char *t = q;` - `t` aliases the output buffer. */
    let t: *mut u8 = q;
    let mut Q = Ge25519P3::default();
    let mut i: u32;

    i = 0;
    while i < 32 {
        unsafe { *t.add(i as usize) = *n.add(i as usize) };
        i += 1;
    }
    unsafe { *t.add(31) &= 127 };
    unsafe {
        _sodium_ge25519_scalarmult_base(&raw mut Q, t);
        _sodium_ristretto255_p3_tobytes(q, &raw const Q);
    }
    if unsafe { sodium_is_zero(q, 32) } != 0 {
        return -1;
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_scalarmult_ristretto255_bytes() -> usize {
    crypto_scalarmult_ristretto255_BYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_scalarmult_ristretto255_scalarbytes() -> usize {
    crypto_scalarmult_ristretto255_SCALARBYTES
}
