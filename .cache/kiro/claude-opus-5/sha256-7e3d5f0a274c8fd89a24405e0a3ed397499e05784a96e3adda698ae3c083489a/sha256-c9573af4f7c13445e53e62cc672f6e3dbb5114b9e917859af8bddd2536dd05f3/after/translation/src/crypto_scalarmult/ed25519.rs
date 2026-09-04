//! Translation of `crypto_scalarmult/ed25519/ref10/scalarmult_ed25519_ref10.c`
//! and `include/sodium/crypto_scalarmult_ed25519.h`.

use core::ffi::c_int;

use crate::crypto_core::ed25519_ref10::ge::{
    _sodium_ge25519_frombytes, _sodium_ge25519_has_small_order, _sodium_ge25519_is_canonical,
    _sodium_ge25519_is_on_main_subgroup, _sodium_ge25519_p3_tobytes, _sodium_ge25519_scalarmult,
    _sodium_ge25519_scalarmult_base,
};
use crate::crypto_core::ed25519_ref10::ge25519_p3;
use crate::sodium_utils::sodium_is_zero;

/* ---- constants from crypto_scalarmult_ed25519.h ---- */

pub const crypto_scalarmult_ed25519_BYTES: usize = 32;
pub const crypto_scalarmult_ed25519_SCALARBYTES: usize = 32;

unsafe fn _crypto_scalarmult_ed25519_is_inf(s: *const u8) -> c_int {
    let mut c: u8;
    let mut i: u32;

    c = *s.add(0) ^ 0x01;
    i = 1;
    while i < 31 {
        c |= *s.add(i as usize);
        i += 1;
    }
    c |= *s.add(31) & 0x7f;

    ((((c as u32).wrapping_sub(1u32)) >> 8) & 1) as c_int
}

#[inline]
unsafe fn _crypto_scalarmult_ed25519_clamp(k: *mut u8) {
    *k.add(0) &= 248;
    *k.add(31) |= 64;
}

unsafe fn _crypto_scalarmult_ed25519(
    q: *mut u8,
    n: *const u8,
    p: *const u8,
    clamp: c_int,
) -> c_int {
    let t: *mut u8 = q;
    let mut Q: ge25519_p3 = core::mem::zeroed();
    let mut P: ge25519_p3 = core::mem::zeroed();
    let mut i: u32;

    if _sodium_ge25519_is_canonical(p) == 0
        || _sodium_ge25519_frombytes(&mut P, p) != 0
        || _sodium_ge25519_has_small_order(&P) != 0
        || _sodium_ge25519_is_on_main_subgroup(&P) == 0
    {
        return -1;
    }
    i = 0;
    while i < 32 {
        *t.add(i as usize) = *n.add(i as usize);
        i += 1;
    }
    if clamp != 0 {
        _crypto_scalarmult_ed25519_clamp(t);
    }
    *t.add(31) &= 127;

    _sodium_ge25519_scalarmult(&mut Q, t, &P);
    _sodium_ge25519_p3_tobytes(q, &Q);
    if _crypto_scalarmult_ed25519_is_inf(q) != 0 || sodium_is_zero(n, 32) != 0 {
        return -1;
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_scalarmult_ed25519(
    q: *mut u8,
    n: *const u8,
    p: *const u8,
) -> c_int {
    _crypto_scalarmult_ed25519(q, n, p, 1)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_scalarmult_ed25519_noclamp(
    q: *mut u8,
    n: *const u8,
    p: *const u8,
) -> c_int {
    _crypto_scalarmult_ed25519(q, n, p, 0)
}

unsafe fn _crypto_scalarmult_ed25519_base(q: *mut u8, n: *const u8, clamp: c_int) -> c_int {
    let t: *mut u8 = q;
    let mut Q: ge25519_p3 = core::mem::zeroed();
    let mut i: u32;

    i = 0;
    while i < 32 {
        *t.add(i as usize) = *n.add(i as usize);
        i += 1;
    }
    if clamp != 0 {
        _crypto_scalarmult_ed25519_clamp(t);
    }
    *t.add(31) &= 127;

    _sodium_ge25519_scalarmult_base(&mut Q, t);
    _sodium_ge25519_p3_tobytes(q, &Q);
    if _crypto_scalarmult_ed25519_is_inf(q) != 0 || sodium_is_zero(n, 32) != 0 {
        return -1;
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_scalarmult_ed25519_base(q: *mut u8, n: *const u8) -> c_int {
    _crypto_scalarmult_ed25519_base(q, n, 1)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_scalarmult_ed25519_base_noclamp(q: *mut u8, n: *const u8) -> c_int {
    _crypto_scalarmult_ed25519_base(q, n, 0)
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_scalarmult_ed25519_bytes() -> usize {
    crypto_scalarmult_ed25519_BYTES
}

#[unsafe(no_mangle)]
pub extern "C" fn crypto_scalarmult_ed25519_scalarbytes() -> usize {
    crypto_scalarmult_ed25519_SCALARBYTES
}
