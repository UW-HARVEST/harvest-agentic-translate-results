//! Translation of c_src/libsodium/crypto_scalarmult/ed25519/ref10/scalarmult_ed25519_ref10.c

use core::ffi::c_int;

use crate::fe25519::ge25519_p3;

// crypto_scalarmult_ed25519_BYTES / SCALARBYTES (32U each).
const crypto_scalarmult_ed25519_BYTES: usize = 32;
const crypto_scalarmult_ed25519_SCALARBYTES: usize = 32;

extern "C" {
    fn _sodium_ge25519_is_canonical(s: *const u8) -> c_int;
    fn _sodium_ge25519_frombytes(h: *mut ge25519_p3, s: *const u8) -> c_int;
    fn _sodium_ge25519_has_small_order(p: *const ge25519_p3) -> c_int;
    fn _sodium_ge25519_is_on_main_subgroup(p: *const ge25519_p3) -> c_int;
    fn _sodium_ge25519_scalarmult(h: *mut ge25519_p3, a: *const u8, p: *const ge25519_p3);
    fn _sodium_ge25519_scalarmult_base(h: *mut ge25519_p3, a: *const u8);
    fn _sodium_ge25519_p3_tobytes(s: *mut u8, h: *const ge25519_p3);
    fn sodium_is_zero(n: *const u8, nlen: usize) -> c_int;
}

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

    ((((c as u32).wrapping_sub(1)) >> 8) & 1) as c_int
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
    let mut q_ge = core::mem::MaybeUninit::<ge25519_p3>::uninit();
    let mut p_ge = core::mem::MaybeUninit::<ge25519_p3>::uninit();
    let mut i: u32;

    if _sodium_ge25519_is_canonical(p) == 0
        || _sodium_ge25519_frombytes(p_ge.as_mut_ptr(), p) != 0
        || _sodium_ge25519_has_small_order(p_ge.as_ptr()) != 0
        || _sodium_ge25519_is_on_main_subgroup(p_ge.as_ptr()) == 0
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

    _sodium_ge25519_scalarmult(q_ge.as_mut_ptr(), t, p_ge.as_ptr());
    _sodium_ge25519_p3_tobytes(q, q_ge.as_ptr());
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

unsafe fn _crypto_scalarmult_ed25519_base(
    q: *mut u8,
    n: *const u8,
    clamp: c_int,
) -> c_int {
    let t: *mut u8 = q;
    let mut q_ge = core::mem::MaybeUninit::<ge25519_p3>::uninit();
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

    _sodium_ge25519_scalarmult_base(q_ge.as_mut_ptr(), t);
    _sodium_ge25519_p3_tobytes(q, q_ge.as_ptr());
    if _crypto_scalarmult_ed25519_is_inf(q) != 0 || sodium_is_zero(n, 32) != 0 {
        return -1;
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_scalarmult_ed25519_base(
    q: *mut u8,
    n: *const u8,
) -> c_int {
    _crypto_scalarmult_ed25519_base(q, n, 1)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_scalarmult_ed25519_base_noclamp(
    q: *mut u8,
    n: *const u8,
) -> c_int {
    _crypto_scalarmult_ed25519_base(q, n, 0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_scalarmult_ed25519_bytes() -> usize {
    crypto_scalarmult_ed25519_BYTES
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crypto_scalarmult_ed25519_scalarbytes() -> usize {
    crypto_scalarmult_ed25519_SCALARBYTES
}
