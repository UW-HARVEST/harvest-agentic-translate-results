//! Translation of
//! `crypto_scalarmult/ed25519/ref10/scalarmult_ed25519_ref10.c`.

use core::ffi::c_int;

use crate::crypto_core::ed25519::types::Ge25519P3;
use crate::sodium::utils::sodium_is_zero;

pub const crypto_scalarmult_ed25519_BYTES: usize = 32;
pub const crypto_scalarmult_ed25519_SCALARBYTES: usize = 32;

unsafe extern "C" {
    fn _sodium_ge25519_is_canonical(s: *const u8) -> c_int;
    fn _sodium_ge25519_frombytes(h: *mut Ge25519P3, s: *const u8) -> c_int;
    fn _sodium_ge25519_has_small_order(p: *const Ge25519P3) -> c_int;
    fn _sodium_ge25519_is_on_main_subgroup(p: *const Ge25519P3) -> c_int;
    fn _sodium_ge25519_scalarmult(h: *mut Ge25519P3, a: *const u8, p: *const Ge25519P3);
    fn _sodium_ge25519_scalarmult_base(h: *mut Ge25519P3, a: *const u8);
    fn _sodium_ge25519_p3_tobytes(s: *mut u8, h: *const Ge25519P3);
}

unsafe fn _crypto_scalarmult_ed25519_is_inf(s: *const u8) -> c_int {
    let mut c: u8;
    let mut i: u32;

    c = unsafe { *s.add(0) } ^ 0x01;
    i = 1;
    while i < 31 {
        c |= unsafe { *s.add(i as usize) };
        i += 1;
    }
    c |= unsafe { *s.add(31) } & 0x7f;

    ((((c as u32).wrapping_sub(1u32)) >> 8) & 1) as c_int
}

#[inline]
unsafe fn _crypto_scalarmult_ed25519_clamp(k: *mut u8) {
    unsafe {
        *k.add(0) &= 248;
        *k.add(31) |= 64;
    }
}

unsafe fn _crypto_scalarmult_ed25519(
    q: *mut u8,
    n: *const u8,
    p: *const u8,
    clamp: c_int,
) -> c_int {
    /* `unsigned char *t = q;` - `t` aliases the output buffer. */
    let t: *mut u8 = q;
    let mut Q = Ge25519P3::default();
    let mut P = Ge25519P3::default();
    let mut i: u32;

    if unsafe { _sodium_ge25519_is_canonical(p) } == 0
        || unsafe { _sodium_ge25519_frombytes(&raw mut P, p) } != 0
        || unsafe { _sodium_ge25519_has_small_order(&raw const P) } != 0
        || unsafe { _sodium_ge25519_is_on_main_subgroup(&raw const P) } == 0
    {
        return -1;
    }
    i = 0;
    while i < 32 {
        unsafe { *t.add(i as usize) = *n.add(i as usize) };
        i += 1;
    }
    if clamp != 0 {
        unsafe { _crypto_scalarmult_ed25519_clamp(t) };
    }
    unsafe { *t.add(31) &= 127 };

    unsafe {
        _sodium_ge25519_scalarmult(&raw mut Q, t, &raw const P);
        _sodium_ge25519_p3_tobytes(q, &raw const Q);
    }
    if unsafe { _crypto_scalarmult_ed25519_is_inf(q) } != 0
        || unsafe { sodium_is_zero(n, 32) } != 0
    {
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
    unsafe { _crypto_scalarmult_ed25519(q, n, p, 1) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_scalarmult_ed25519_noclamp(
    q: *mut u8,
    n: *const u8,
    p: *const u8,
) -> c_int {
    unsafe { _crypto_scalarmult_ed25519(q, n, p, 0) }
}

unsafe fn _crypto_scalarmult_ed25519_base(q: *mut u8, n: *const u8, clamp: c_int) -> c_int {
    /* `unsigned char *t = q;` - `t` aliases the output buffer. */
    let t: *mut u8 = q;
    let mut Q = Ge25519P3::default();
    let mut i: u32;

    i = 0;
    while i < 32 {
        unsafe { *t.add(i as usize) = *n.add(i as usize) };
        i += 1;
    }
    if clamp != 0 {
        unsafe { _crypto_scalarmult_ed25519_clamp(t) };
    }
    unsafe { *t.add(31) &= 127 };

    unsafe {
        _sodium_ge25519_scalarmult_base(&raw mut Q, t);
        _sodium_ge25519_p3_tobytes(q, &raw const Q);
    }
    if unsafe { _crypto_scalarmult_ed25519_is_inf(q) } != 0
        || unsafe { sodium_is_zero(n, 32) } != 0
    {
        return -1;
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_scalarmult_ed25519_base(q: *mut u8, n: *const u8) -> c_int {
    unsafe { _crypto_scalarmult_ed25519_base(q, n, 1) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_scalarmult_ed25519_base_noclamp(
    q: *mut u8,
    n: *const u8,
) -> c_int {
    unsafe { _crypto_scalarmult_ed25519_base(q, n, 0) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_scalarmult_ed25519_bytes() -> usize {
    crypto_scalarmult_ed25519_BYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_scalarmult_ed25519_scalarbytes() -> usize {
    crypto_scalarmult_ed25519_SCALARBYTES
}
